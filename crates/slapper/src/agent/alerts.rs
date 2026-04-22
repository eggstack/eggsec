//! Alert routing system for the security agent.
//!
//! Routes alerts to configured channels (webhooks, email, Slack, PagerDuty)
//! with rate limiting and deduplication.

use std::sync::Arc;
use parking_lot::Mutex;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use chrono::Utc;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use lettre::transport::smtp::SmtpTransport;
use lettre::transport::Transport;
use lettre::Message;
use lettre::message::Mailbox;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Alert {
    pub severity: crate::types::Severity,
    pub title: String,
    pub message: String,
    pub target: String,
    pub finding_ids: Vec<String>,
    pub recommended_actions: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WebhookConfig {
    pub url: String,
    pub secret: Option<crate::types::SensitiveString>,
    pub headers: std::collections::HashMap<String, String>,
}

impl Default for WebhookConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            secret: None,
            headers: std::collections::HashMap::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmailChannel {
    pub smtp_host: String,
    pub smtp_port: u16,
    pub from: String,
    pub to: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SlackChannel {
    pub webhook_url: String,
    pub channel: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PagerDutyChannel {
    pub routing_key: crate::types::SensitiveString,
    pub severity: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AlertChannel {
    Webhook(WebhookConfig),
    Email(EmailChannel),
    Slack(SlackChannel),
    PagerDuty(PagerDutyChannel),
}

pub struct AlertRouter {
    channels: Arc<Mutex<Vec<AlertChannel>>>,
    recent_alerts: Arc<Mutex<std::collections::HashMap<String, Instant>>>,
    dedup_window_secs: u64,
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

    pub fn new() -> Self {
        Self {
            channels: Arc::new(Mutex::new(Vec::new())),
            recent_alerts: Arc::new(Mutex::new(std::collections::HashMap::new())),
            dedup_window_secs: 300,
        }
    }

    pub fn add_channel(&self, channel: AlertChannel) {
        self.channels.lock().unwrap().push(channel);
    }

    pub async fn send(&self, alert: &Alert) -> Result<()> {
        {
            let recent_alerts = self.recent_alerts.lock().await;
            if recent_alerts.len() > 1000 {
                drop(recent_alerts);
                self.cleanup_stale_entries();
            }
        }

        let dedup_key = self.make_dedup_key(alert);
        {
            let recent_alerts = self.recent_alerts.lock().await;
            if let Some(last_sent) = recent_alerts.get(&dedup_key) {
                if last_sent.elapsed() < Duration::from_secs(self.dedup_window_secs) {
                    tracing::debug!("Duplicate alert suppressed: {}", dedup_key);
                    return Ok(());
                }
            }
        }

        let channels = self.channels.lock().await.clone();
        for channel in &channels {
            self.send_to_channel(channel, alert).await?;
        }

        {
            let mut recent_alerts = self.recent_alerts.lock().await;
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
                    headers: std::collections::HashMap::new(),
                };
                self.send_webhook(&webhook_config, alert).await?;
            }
            AlertChannel::Email(config) => {
                if let Err(e) = self.send_email(config, alert).await {
                    tracing::warn!("Failed to send email alert: {}", e);
                }
            }
            AlertChannel::PagerDuty(config) => {
                if let Err(e) = self.send_pagerduty(config, alert).await {
                    tracing::warn!("Failed to send PagerDuty alert: {}", e);
                }
            }
        }
        Ok(())
    }

    async fn send_email(&self, config: &EmailChannel, alert: &Alert) -> Result<()> {
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

        tokio::task::spawn_blocking(move || {
            mailer.send(&email)
        })
        .await??;

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

        let client = Self::create_pooled_client()?;
        let mut request = client.post(&config.url).json(&payload);

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
            tracing::warn!(
                "Webhook failed with status: {}, body: {}",
                status,
                body
            );
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

        let client = Self::create_pooled_client()?;
        let response = client
            .post("https://events.pagerduty.com/v2/enqueue")
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            tracing::warn!("PagerDuty API failed with status: {}", response.status());
        }

        Ok(())
    }

    fn make_dedup_key(&self, alert: &Alert) -> String {
        format!(
            "{}:{}:{}",
            alert.target,
            alert.severity.as_str(),
            alert.title
        )
    }

    async fn cleanup_stale_entries(&self) {
        let cutoff = Duration::from_secs(self.dedup_window_secs * 2);
        let mut recent_alerts = self.recent_alerts.lock().await;
        recent_alerts.retain(|_, last_sent| last_sent.elapsed() < cutoff);
    }

    pub async fn aggregate_findings(&self, alerts: &[Alert]) -> AggregatedAlert {
        let mut severity_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        let mut all_finding_ids = Vec::new();
        let mut targets: std::collections::HashSet<String> = std::collections::HashSet::new();

        for alert in alerts {
            *severity_counts.entry(alert.severity.as_str().to_string()).or_insert(0) += 1;
            all_finding_ids.extend(alert.finding_ids.clone());
            targets.insert(alert.target.clone());
        }

        let max_severity = severity_counts
            .iter()
            .max_by_key(|(sev, _)| {
                match sev.as_str() {
                    "critical" => 5,
                    "high" => 4,
                    "medium" => 3,
                    "low" => 2,
                    _ => 1,
                }
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
        let critical_count = alerts.iter().filter(|a| a.severity == crate::types::Severity::Critical).count();
        let high_count = alerts.iter().filter(|a| a.severity == crate::types::Severity::High).count();
        let medium_count = alerts.iter().filter(|a| a.severity == crate::types::Severity::Medium).count();
        let low_count = alerts.iter().filter(|a| a.severity == crate::types::Severity::Low).count();
        let info_count = alerts.iter().filter(|a| a.severity == crate::types::Severity::Info).count();

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
                risk_score: (critical_count * 10 + high_count * 7 + medium_count * 4 + low_count * 1) as f64,
            },
            findings: findings.to_vec(),
            alerts: alerts.to_vec(),
            recommendations: self.generate_recommendations(findings),
        }
    }

    fn generate_recommendations(&self, findings: &[crate::tool::response::Finding]) -> Vec<String> {
        let mut recommendations = Vec::new();

        let mut vuln_types: std::collections::HashSet<String> = std::collections::HashSet::new();
        for finding in findings {
            vuln_types.insert(format!("{:?}", finding.finding_type));
        }

        if vuln_types.contains("SqlInjection") {
            recommendations.push("Implement parameterized queries or use an ORM".to_string());
        }
        if vuln_types.contains("Xss") {
            recommendations.push("Implement Content Security Policy and output encoding".to_string());
        }
        if vuln_types.contains("Ssrf") {
            recommendations.push("Validate and sanitize all user-supplied URLs".to_string());
        }

        if recommendations.is_empty() {
            recommendations.push("Continue regular security scanning and patch management".to_string());
        }

        recommendations
    }

    pub async fn escalate_alert(&self, alert: &Alert, escalation_level: EscalationLevel) -> Result<()> {
        match escalation_level {
            EscalationLevel::Warning => {
                tracing::warn!("Alert escalated to Warning: {}", alert.title);
            }
            EscalationLevel::Urgent => {
                let channels = self.channels.lock().await.clone();
                for channel in &channels {
                    if matches!(channel, AlertChannel::Slack(_)) {
                        self.send_to_channel(channel, alert).await?;
                    }
                }
            }
            EscalationLevel::Critical => {
                let channels = self.channels.lock().await.clone();
                for channel in &channels {
                    self.send_to_channel(channel, alert).await?;
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct AggregatedAlert {
    pub total_count: usize,
    pub severity_counts: std::collections::HashMap<String, usize>,
    pub all_finding_ids: Vec<String>,
    pub affected_targets: Vec<String>,
    pub max_severity: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct ScanReport {
    pub id: String,
    pub target: String,
    pub generated_at: chrono::DateTime<chrono::Utc>,
    pub summary: ReportSummary,
    pub findings: Vec<crate::tool::response::Finding>,
    pub alerts: Vec<Alert>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ReportSummary {
    pub total_findings: usize,
    pub critical_count: usize,
    pub high_count: usize,
    pub medium_count: usize,
    pub low_count: usize,
    pub info_count: usize,
    pub risk_score: f64,
}

#[derive(Debug, Clone, Copy)]
pub enum EscalationLevel {
    Warning,
    Urgent,
    Critical,
}

pub struct AlertTemplate {
    pub name: String,
    pub slack_template: SlackTemplate,
    pub pagerduty_template: PagerDutyTemplate,
    pub email_template: EmailTemplate,
}

#[derive(Debug, Clone)]
pub struct SlackTemplate {
    pub title_format: String,
    pub body_format: String,
    pub color_by_severity: std::collections::HashMap<String, String>,
}

impl SlackTemplate {
    pub fn default_templates() -> Self {
        let mut colors = std::collections::HashMap::new();
        colors.insert("critical".to_string(), "#dc3545".to_string());
        colors.insert("high".to_string(), "#fd7e14".to_string());
        colors.insert("medium".to_string(), "#ffc107".to_string());
        colors.insert("low".to_string(), "#0dcaf0".to_string());
        colors.insert("info".to_string(), "#6c757d".to_string());

        Self {
            title_format: "[{{severity}}] {{title}}".to_string(),
            body_format: "Target: {{target}}\n{{message}}\nFindings: {{finding_count}}".to_string(),
            color_by_severity: colors,
        }
    }

    pub fn format(&self, alert: &Alert, finding_count: usize) -> SlackFormattedAlert {
        let title = self.title_format
            .replace("{{severity}}", &alert.severity.as_str().to_uppercase())
            .replace("{{title}}", &alert.title);

        let body = self.body_format
            .replace("{{target}}", &alert.target)
            .replace("{{message}}", &alert.message)
            .replace("{{finding_count}}", &finding_count.to_string());

        let color = self.color_by_severity
            .get(alert.severity.as_str())
            .cloned()
            .unwrap_or_else(|| "#6c757d".to_string());

        SlackFormattedAlert {
            title,
            body,
            color,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SlackFormattedAlert {
    pub title: String,
    pub body: String,
    pub color: String,
}

#[derive(Debug, Clone)]
pub struct PagerDutyTemplate {
    pub severity_mapping: std::collections::HashMap<String, String>,
}

impl PagerDutyTemplate {
    pub fn default_template() -> Self {
        let mut mapping = std::collections::HashMap::new();
        mapping.insert("critical".to_string(), "critical".to_string());
        mapping.insert("high".to_string(), "error".to_string());
        mapping.insert("medium".to_string(), "warning".to_string());
        mapping.insert("low".to_string(), "info".to_string());
        mapping.insert("info".to_string(), "info".to_string());

        Self {
            severity_mapping: mapping,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EmailTemplate {
    pub subject_format: String,
    pub body_format: String,
}

impl EmailTemplate {
    pub fn default_template() -> Self {
        Self {
            subject_format: "[Slapper] {{severity}}: {{title}}".to_string(),
            body_format: "Security Alert: {{title}}\n\n\
                Target: {{target}}\n\
                Severity: {{severity}}\n\n\
                {{message}}\n\n\
                Recommended Actions:\n\
                {{actions}}".to_string(),
        }
    }

    pub fn format(&self, alert: &Alert) -> EmailFormattedAlert {
        let subject = self.subject_format
            .replace("{{severity}}", &alert.severity.as_str().to_uppercase())
            .replace("{{title}}", &alert.title);

        let body = self.body_format
            .replace("{{target}}", &alert.target)
            .replace("{{title}}", &alert.title)
            .replace("{{message}}", &alert.message)
            .replace("{{severity}}", &alert.severity.as_str().to_uppercase())
            .replace("{{actions}}", &alert.recommended_actions.join("\n"));

        EmailFormattedAlert {
            subject,
            body,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EmailFormattedAlert {
    pub subject: String,
    pub body: String,
}

impl Default for AlertRouter {
    fn default() -> Self {
        Self::new()
    }
}

type HmacSha256 = Hmac<Sha256>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alert_creation() {
        let alert = Alert {
            severity: crate::types::Severity::Critical,
            title: "Test Alert".to_string(),
            message: "This is a test".to_string(),
            target: "https://example.com".to_string(),
            finding_ids: vec!["finding-1".to_string()],
            recommended_actions: vec!["Review immediately".to_string()],
        };

        assert_eq!(alert.severity.as_str(), "critical");
    }

    #[test]
    fn test_webhook_config_default() {
        let config = WebhookConfig::default();
        assert!(config.url.is_empty());
        assert!(config.secret.is_none());
        assert!(config.headers.is_empty());
    }

    #[test]
    fn test_webhook_config_with_secret() {
        let config = WebhookConfig {
            url: "https://example.com/webhook".to_string(),
            secret: Some(crate::types::SensitiveString::from("secret-key".to_string())),
            headers: std::collections::HashMap::new(),
        };
        assert!(config.secret.is_some());
        assert_eq!(config.secret.as_ref().unwrap().expose_secret(), "secret-key");
    }

    #[test]
    fn test_webhook_config_with_headers() {
        let mut headers = std::collections::HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer token".to_string());
        headers.insert("X-Custom-Header".to_string(), "custom-value".to_string());

        let config = WebhookConfig {
            url: "https://example.com/webhook".to_string(),
            secret: None,
            headers,
        };
        assert_eq!(config.headers.len(), 2);
        assert_eq!(config.headers.get("Authorization"), Some(&"Bearer token".to_string()));
    }

    #[test]
    fn test_alert_router_new() {
        let router = AlertRouter::new();
        assert_eq!(router.dedup_window_secs, 300);
    }

    #[test]
    fn test_alert_router_add_webhook_channel() {
        let router = AlertRouter::new();
        let config = WebhookConfig {
            url: "https://example.com/webhook".to_string(),
            secret: None,
            headers: std::collections::HashMap::new(),
        };
        router.add_channel(AlertChannel::Webhook(config));
    }

    #[test]
    fn test_alert_router_add_slack_channel() {
        let router = AlertRouter::new();
        let config = SlackChannel {
            webhook_url: "https://hooks.slack.com/services/xxx".to_string(),
            channel: Some("#alerts".to_string()),
        };
        router.add_channel(AlertChannel::Slack(config));
    }

    #[test]
    fn test_alert_router_add_email_channel() {
        let router = AlertRouter::new();
        let config = EmailChannel {
            smtp_host: "smtp.example.com".to_string(),
            smtp_port: 587,
            from: "alerts@example.com".to_string(),
            to: vec!["admin@example.com".to_string()],
        };
        router.add_channel(AlertChannel::Email(config));
    }

    #[test]
    fn test_alert_router_add_pagerduty_channel() {
        let router = AlertRouter::new();
        let config = PagerDutyChannel {
            routing_key: crate::types::SensitiveString::from("routing-key-123".to_string()),
            severity: "critical".to_string(),
        };
        router.add_channel(AlertChannel::PagerDuty(config));
    }

    #[test]
    fn test_make_dedup_key() {
        let router = AlertRouter::new();
        let alert = Alert {
            severity: crate::types::Severity::High,
            title: "SQL Injection".to_string(),
            message: "Vulnerability found".to_string(),
            target: "https://example.com".to_string(),
            finding_ids: vec![],
            recommended_actions: vec![],
        };
        let key = router.make_dedup_key(&alert);
        assert!(key.contains("https://example.com"));
        assert!(key.contains("high"));
        assert!(key.contains("SQL Injection"));
    }

    #[test]
    fn test_make_dedup_key_different_severities() {
        let router = AlertRouter::new();
        let alert1 = Alert {
            severity: crate::types::Severity::Critical,
            title: "Test".to_string(),
            message: "".to_string(),
            target: "https://example.com".to_string(),
            finding_ids: vec![],
            recommended_actions: vec![],
        };
        let alert2 = Alert {
            severity: crate::types::Severity::Low,
            title: "Test".to_string(),
            message: "".to_string(),
            target: "https://example.com".to_string(),
            finding_ids: vec![],
            recommended_actions: vec![],
        };
        let key1 = router.make_dedup_key(&alert1);
        let key2 = router.make_dedup_key(&alert2);
        assert_ne!(key1, key2);
    }

    #[tokio::test]
    async fn test_alert_router_send_duplicate_suppression() {
        let router = AlertRouter::new();
        let config = WebhookConfig {
            url: "https://example.com/webhook".to_string(),
            secret: None,
            headers: std::collections::HashMap::new(),
        };
        router.add_channel(AlertChannel::Webhook(config));

        let alert = Alert {
            severity: crate::types::Severity::Critical,
            title: "Test Alert".to_string(),
            message: "This is a test".to_string(),
            target: "https://example.com".to_string(),
            finding_ids: vec!["finding-1".to_string()],
            recommended_actions: vec!["Review immediately".to_string()],
        };

        let result1 = router.send(&alert).await;
        let result2 = router.send(&alert).await;
        assert!(result1.is_ok());
        assert!(result2.is_ok());
    }

    #[test]
    fn test_hmac_signature_generation() {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        type HmacSha256 = Hmac<Sha256>;

        let secret = "test-secret-key";
        let payload = serde_json::json!({"alert": {"title": "Test"}});
        let canonical_json = serde_json::to_string(&payload).unwrap();

        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
        mac.update(canonical_json.as_bytes());
        let result = mac.finalize();
        let signature = format!("sha256={}", hex::encode(result.into_bytes()));

        assert!(signature.starts_with("sha256="));
        assert_eq!(signature.len(), 71);
    }

    #[test]
    fn test_hmac_signature_different_payloads_different_signatures() {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        type HmacSha256 = Hmac<Sha256>;

        let secret = "test-secret-key";

        let payload1 = serde_json::json!({"alert": {"title": "Test1"}});
        let payload2 = serde_json::json!({"alert": {"title": "Test2"}});

        let canonical_json1 = serde_json::to_string(&payload1).unwrap();
        let canonical_json2 = serde_json::to_string(&payload2).unwrap();

        let mut mac1 = HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
        mac1.update(canonical_json1.as_bytes());
        let sig1 = hex::encode(mac1.finalize().into_bytes());

        let mut mac2 = HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
        mac2.update(canonical_json2.as_bytes());
        let sig2 = hex::encode(mac2.finalize().into_bytes());

        assert_ne!(sig1, sig2);
    }

    #[test]
    fn test_hmac_signature_different_keys_different_signatures() {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        type HmacSha256 = Hmac<Sha256>;

        let payload = serde_json::json!({"alert": {"title": "Test"}});
        let canonical_json = serde_json::to_string(&payload).unwrap();

        let mut mac1 = HmacSha256::new_from_slice("key1".as_bytes()).expect("HMAC can take key of any size");
        mac1.update(canonical_json.as_bytes());
        let sig1 = hex::encode(mac1.finalize().into_bytes());

        let mut mac2 = HmacSha256::new_from_slice("key2".as_bytes()).expect("HMAC can take key of any size");
        mac2.update(canonical_json.as_bytes());
        let sig2 = hex::encode(mac2.finalize().into_bytes());

        assert_ne!(sig1, sig2);
    }

    #[test]
    fn test_sensitive_string_in_webhook_secret() {
        let sensitive = crate::types::SensitiveString::from("my-secret-key".to_string());
        let config = WebhookConfig {
            url: "https://example.com/webhook".to_string(),
            secret: Some(sensitive),
            headers: std::collections::HashMap::new(),
        };
        assert!(config.secret.is_some());
        let exposed = config.secret.unwrap().expose_secret();
        assert_eq!(exposed, "my-secret-key");
    }

    #[test]
    fn test_webhook_payload_structure() {
        let alert = Alert {
            severity: crate::types::Severity::Critical,
            title: "Critical Finding".to_string(),
            message: "SQL injection detected".to_string(),
            target: "https://example.com/login".to_string(),
            finding_ids: vec!["finding-1".to_string(), "finding-2".to_string()],
            recommended_actions: vec!["Patch immediately".to_string()],
        };

        let payload = serde_json::json!({
            "alert": {
                "severity": alert.severity.as_str(),
                "title": alert.title,
                "message": alert.message,
                "target": alert.target,
                "finding_ids": alert.finding_ids,
                "recommended_actions": alert.recommended_actions,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }
        });

        assert_eq!(payload["alert"]["severity"], "critical");
        assert_eq!(payload["alert"]["title"], "Critical Finding");
        assert!(payload["alert"]["finding_ids"].as_array().unwrap().len() == 2);
    }
}
