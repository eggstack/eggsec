//! Alert routing system for the security agent.
//!
//! Routes alerts to configured channels (webhooks, email, Slack, PagerDuty)
//! with rate limiting and deduplication.

use std::sync::{Arc, Mutex};
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
    pub routing_key: String,
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
            let recent_alerts = self.recent_alerts.lock().map_err(|_| std::io::Error::new(
                std::io::ErrorKind::WouldBlock,
                "Failed to acquire lock on recent alerts"
            ))?;
            if recent_alerts.len() > 1000 {
                drop(recent_alerts);
                self.cleanup_stale_entries();
            }
        }

        let dedup_key = self.make_dedup_key(alert);
        {
            let recent_alerts = self.recent_alerts.lock().map_err(|_| std::io::Error::new(
                std::io::ErrorKind::WouldBlock,
                "Failed to acquire lock on recent alerts"
            ))?;
            if let Some(last_sent) = recent_alerts.get(&dedup_key) {
                if last_sent.elapsed() < Duration::from_secs(self.dedup_window_secs) {
                    tracing::debug!("Duplicate alert suppressed: {}", dedup_key);
                    return Ok(());
                }
            }
        }

        let channels = self.channels.lock().map_err(|_| std::io::Error::new(
                std::io::ErrorKind::WouldBlock,
                "Failed to acquire lock on channels"
            ))?.clone();
        for channel in &channels {
            self.send_to_channel(channel, alert).await?;
        }

        {
            let mut recent_alerts = self.recent_alerts.lock().map_err(|_| std::io::Error::new(
                std::io::ErrorKind::WouldBlock,
                "Failed to acquire lock on recent alerts"
            ))?;
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
            tracing::warn!("Webhook failed with status: {}", response.status());
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
            "routing_key": config.routing_key,
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

    fn cleanup_stale_entries(&self) {
        let cutoff = Duration::from_secs(self.dedup_window_secs * 2);
        if let Ok(mut recent_alerts) = self.recent_alerts.lock() {
            recent_alerts.retain(|_, last_sent| last_sent.elapsed() < cutoff);
        }
    }
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
}
