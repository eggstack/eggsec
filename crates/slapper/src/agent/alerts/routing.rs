use parking_lot::Mutex;
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use chrono::Utc;
use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::agent::channels::{
    AggregatedAlert, Alert, AlertChannel, EmailChannel, EscalationLevel, PagerDutyChannel,
    ReportSummary, ScanReport, WebhookConfig,
};

type HmacSha256 = Hmac<Sha256>;

/// Registry mapping channel names to AlertChannel definitions
pub struct ChannelRegistry {
    channels: FxHashMap<String, AlertChannel>,
}

impl ChannelRegistry {
    pub fn new() -> Self {
        Self {
            channels: FxHashMap::default(),
        }
    }

    pub fn register(&mut self, name: String, channel: AlertChannel) {
        self.channels.insert(name, channel);
    }

    pub fn get(&self, name: &str) -> Option<&AlertChannel> {
        self.channels.get(name)
    }

    pub fn get_all(&self) -> Vec<(String, AlertChannel)> {
        self.channels
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    pub fn contains(&self, name: &str) -> bool {
        self.channels.contains_key(name)
    }
}

impl Default for ChannelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub struct AlertRouter {
    registry: Arc<Mutex<ChannelRegistry>>,
    recent_alerts: Arc<Mutex<FxHashMap<String, Instant>>>,
    dedup_window_secs: u64,
    client: reqwest::Client,
}

impl AlertRouter {
    fn create_pooled_client() -> Result<reqwest::Client> {
        reqwest::Client::builder()
            .pool_max_idle_per_host(20)
            .pool_idle_timeout(Duration::from_secs(30))
            .tcp_nodelay(true)
            .build()
            .context("Failed to create HTTP client")
    }

    pub fn new() -> Result<Self> {
        let client = Self::create_pooled_client().unwrap_or_else(|_| {
            reqwest::Client::builder()
                .pool_max_idle_per_host(5)
                .build()
                .context("Failed to create fallback HTTP client")?
        });

        Ok(Self {
            registry: Arc::new(Mutex::new(ChannelRegistry::new())),
            recent_alerts: Arc::new(Mutex::new(FxHashMap::default())),
            dedup_window_secs: 300,
            client,
        })
    }

    fn default() -> Self {
        Self {
            registry: Arc::new(Mutex::new(ChannelRegistry::new())),
            recent_alerts: Arc::new(Mutex::new(FxHashMap::default())),
            dedup_window_secs: 300,
            client: reqwest::Client::new(),
        }
    }

    /// Register a channel with a name for target-specific routing
    pub fn register_channel(&self, name: String, channel: AlertChannel) {
        self.registry.lock().register(name, channel);
    }

    /// Get a channel by name from the registry
    pub fn get_channel(&self, name: &str) -> Option<AlertChannel> {
        self.registry.lock().get(name).cloned()
    }

    /// Send alert to all channels, or filter by channel_names if provided
    pub async fn send(&self, alert: &Alert, channel_names: Option<&[String]>) -> Result<()> {
        {
            let mut recent_alerts = self.recent_alerts.lock();
            if recent_alerts.len() > 1000 {
                let cutoff = Duration::from_secs(self.dedup_window_secs * 2);
                recent_alerts.retain(|_, last_sent| last_sent.elapsed() < cutoff);
            }
        }

        let dedup_key = self.make_dedup_key(alert);
        {
            let recent_alerts = self.recent_alerts.lock();
            if let Some(last_sent) = recent_alerts.get(&dedup_key) {
                if last_sent.elapsed() < Duration::from_secs(self.dedup_window_secs) {
                    tracing::debug!("Duplicate alert suppressed: {}", dedup_key);
                    return Ok(());
                }
            }
        }

        let channels_to_send: Vec<AlertChannel> = if let Some(names) = channel_names {
            // Filter channels by name from registry
            let registry = self.registry.lock();
            names
                .iter()
                .filter_map(|name| registry.get(name).cloned())
                .collect()
        } else {
            // Send to all registered channels
            self.registry
                .lock()
                .get_all()
                .into_iter()
                .map(|(_, c)| c)
                .collect()
        };

        if channels_to_send.is_empty() {
            tracing::debug!(
                "No channels to send alert to (channel_names: {:?})",
                channel_names
            );
            return Ok(());
        }

        for channel in &channels_to_send {
            self.send_to_channel(channel, alert).await?;
        }

        {
            let mut recent_alerts = self.recent_alerts.lock();
            recent_alerts.insert(dedup_key, Instant::now());
        }
        Ok(())
    }

    async fn send_to_channel(&self, channel: &AlertChannel, alert: &Alert) -> Result<()> {
        match channel {
            AlertChannel::Webhook(config) => {
                self.send_webhook(config, alert).await?;
            }
            AlertChannel::Slack(config) => {
                let webhook_config = WebhookConfig {
                    url: config.webhook_url.clone(),
                    secret: None,
                    headers: FxHashMap::default(),
                };
                self.send_webhook(&webhook_config, alert).await?;
            }
            AlertChannel::Email(config) => {
                self.send_email(config, alert).await?;
            }
            AlertChannel::PagerDuty(config) => {
                self.send_pagerduty(config, alert).await?;
            }
        }
        Ok(())
    }

    async fn send_email(&self, config: &EmailChannel, alert: &Alert) -> Result<()> {
        use lettre::message::Mailbox;
        use lettre::transport::smtp::SmtpTransport;
        use lettre::transport::Transport;
        use lettre::Message;

        let mailer = SmtpTransport::relay(&config.smtp_host)?
            .port(config.smtp_port)
            .build();

        let subject = alert.title.clone();
        let body = format!(
            "Severity: {}\nTarget: {}\n\n{}\n\nFinding IDs: {:?}\nRecommended Actions: {:?}",
            alert.severity.as_str().to_uppercase(),
            alert.target,
            alert.message,
            alert.finding_ids,
            alert.recommended_actions
        );

        let mut email_builder = Message::builder()
            .from(config.from.parse()?)
            .subject(subject);

        for addr in &config.to {
            let mailbox: Mailbox = addr.parse()?;
            email_builder = email_builder.to(mailbox);
        }

        let email = email_builder.body(body)?;

        tokio::task::spawn_blocking(move || mailer.send(&email)).await??;

        tracing::info!(
            "Email alert sent via {}:{} from {} to {:?}",
            config.smtp_host,
            config.smtp_port,
            config.from,
            config.to
        );

        Ok(())
    }

    async fn send_webhook(&self, config: &WebhookConfig, alert: &Alert) -> Result<()> {
        let payload = serde_json::json!({
            "alert": {
                "severity": alert.severity.as_str(),
                "title": alert.title,
                "message": alert.message,
                "target": alert.target,
                "finding_ids": alert.finding_ids,
                "recommended_actions": alert.recommended_actions,
                "timestamp": Utc::now().to_rfc3339(),
            }
        });

        let mut request = self.client.post(&config.url).json(&payload);

        if let Some(ref secret) = config.secret {
            let mut mac = HmacSha256::new_from_slice(secret.expose_secret().as_bytes())
                .expect("HMAC can take key of any size");
            let canonical_json = serde_json::to_string(&payload).unwrap();
            mac.update(canonical_json.as_bytes());
            let result = mac.finalize();
            let signature = format!("sha256={}", hex::encode(result.into_bytes()));
            request = request.header("X-Signature-256", signature);
        }

        for (key, value) in &config.headers {
            request = request.header(key, value);
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::warn!("Webhook failed with status: {}, body: {}", status, body);
            return Err(anyhow::anyhow!("Webhook failed with status: {}", status));
        }

        Ok(())
    }

    async fn send_pagerduty(&self, config: &PagerDutyChannel, alert: &Alert) -> Result<()> {
        let pd_severity = match alert.severity {
            crate::types::Severity::Critical => "critical",
            crate::types::Severity::High => "error",
            crate::types::Severity::Medium => "warning",
            crate::types::Severity::Low => "info",
            crate::types::Severity::Info => "info",
        };

        let payload = serde_json::json!({
            "routing_key": config.routing_key.expose_secret(),
            "event_action": "trigger",
            "dedup_key": self.make_dedup_key(alert),
            "payload": {
                "summary": alert.title,
                "source": alert.target,
                "severity": pd_severity,
                "custom_details": {
                    "message": alert.message,
                    "finding_ids": alert.finding_ids,
                    "recommended_actions": alert.recommended_actions,
                }
            }
        });

        let response = self
            .client
            .post("https://events.pagerduty.com/v2/enqueue")
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::warn!(
                "PagerDuty API failed with status: {}, body: {}",
                status,
                body
            );
            return Err(anyhow::anyhow!(
                "PagerDuty API failed with status: {}",
                status
            ));
        }

        Ok(())
    }

    fn make_dedup_key(&self, alert: &Alert) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        let mut sorted_ids = alert.finding_ids.clone();
        sorted_ids.sort();
        for id in &sorted_ids {
            id.hash(&mut hasher);
        }
        let finding_hash = hasher.finish();
        format!(
            "{}:{}:{}:{:016x}",
            alert.target,
            alert.severity.as_str(),
            alert.title,
            finding_hash
        )
    }

    fn cleanup_stale_entries(&self) {
        let cutoff = Duration::from_secs(self.dedup_window_secs * 2);
        let mut recent_alerts = self.recent_alerts.lock();
        recent_alerts.retain(|_, last_sent| last_sent.elapsed() < cutoff);
    }

    pub async fn aggregate_findings(&self, alerts: &[Alert]) -> AggregatedAlert {
        let mut severity_counts: FxHashMap<String, usize> = FxHashMap::default();
        let mut all_finding_ids = Vec::new();
        let mut targets: FxHashSet<String> = FxHashSet::default();

        for alert in alerts {
            *severity_counts
                .entry(alert.severity.as_str().to_string())
                .or_insert(0) += 1;
            all_finding_ids.extend(alert.finding_ids.clone());
            targets.insert(alert.target.clone());
        }

        let max_severity = severity_counts
            .iter()
            .max_by_key(|(sev, _)| match sev.as_str() {
                "critical" => 5,
                "high" => 4,
                "medium" => 3,
                "low" => 2,
                _ => 1,
            })
            .map(|(sev, _)| sev.clone())
            .unwrap_or_else(|| "info".to_string());

        AggregatedAlert {
            total_count: alerts.len(),
            severity_counts,
            all_finding_ids,
            affected_targets: targets.into_iter().collect(),
            max_severity,
            timestamp: Utc::now(),
        }
    }

    pub fn generate_scan_report(
        &self,
        target: &str,
        alerts: &[Alert],
        findings: &[crate::tool::response::Finding],
    ) -> ScanReport {
        let critical_count = alerts
            .iter()
            .filter(|a| a.severity == crate::types::Severity::Critical)
            .count();
        let high_count = alerts
            .iter()
            .filter(|a| a.severity == crate::types::Severity::High)
            .count();
        let medium_count = alerts
            .iter()
            .filter(|a| a.severity == crate::types::Severity::Medium)
            .count();
        let low_count = alerts
            .iter()
            .filter(|a| a.severity == crate::types::Severity::Low)
            .count();
        let info_count = alerts
            .iter()
            .filter(|a| a.severity == crate::types::Severity::Info)
            .count();

        ScanReport {
            id: uuid::Uuid::new_v4().to_string(),
            target: target.to_string(),
            generated_at: Utc::now(),
            summary: ReportSummary {
                total_findings: findings.len(),
                critical_count,
                high_count,
                medium_count,
                low_count,
                info_count,
                risk_score: (critical_count * 10
                    + high_count * 7
                    + medium_count * 4
                    + low_count * 1) as f64,
            },
            findings: findings.to_vec(),
            alerts: alerts.to_vec(),
            recommendations: self.generate_recommendations(findings),
        }
    }

    fn generate_recommendations(&self, findings: &[crate::tool::response::Finding]) -> Vec<String> {
        let mut recommendations = Vec::new();

        let mut vuln_types: FxHashSet<String> = FxHashSet::default();
        for finding in findings {
            vuln_types.insert(format!("{:?}", finding.finding_type));
        }

        if vuln_types.contains("SqlInjection") {
            recommendations.push("Implement parameterized queries or use an ORM".to_string());
        }
        if vuln_types.contains("Xss") {
            recommendations
                .push("Implement Content Security Policy and output encoding".to_string());
        }
        if vuln_types.contains("Ssrf") {
            recommendations.push("Validate and sanitize all user-supplied URLs".to_string());
        }

        if recommendations.is_empty() {
            recommendations
                .push("Continue regular security scanning and patch management".to_string());
        }

        recommendations
    }

    pub async fn escalate_alert(
        &self,
        alert: &Alert,
        escalation_level: EscalationLevel,
    ) -> Result<()> {
        match escalation_level {
            EscalationLevel::Warning => {
                tracing::warn!("Alert escalated to Warning: {}", alert.title);
            }
            EscalationLevel::Urgent => {
                let channels = self.registry.lock().get_all();
                for (_, channel) in &channels {
                    if matches!(channel, AlertChannel::Slack(_)) {
                        self.send_to_channel(channel, alert).await?;
                    }
                }
            }
            EscalationLevel::Critical => {
                let channels = self.registry.lock().get_all();
                for (_, channel) in &channels {
                    self.send_to_channel(channel, alert).await?;
                }
            }
        }
        Ok(())
    }
}

impl Default for AlertRouter {
    fn default() -> Self {
        Self::default()
    }
}
