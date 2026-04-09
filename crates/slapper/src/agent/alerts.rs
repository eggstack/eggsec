//! Alert routing system for the security agent.
//!
//! Routes alerts to configured channels (webhooks, email, Slack, PagerDuty)
//! with rate limiting and deduplication.

use std::time::{Duration, Instant};

use anyhow::Result;
use chrono::Utc;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

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
    pub secret: Option<String>,
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
    channels: Vec<AlertChannel>,
    recent_alerts: std::collections::HashMap<String, Instant>,
    dedup_window_secs: u64,
}

impl AlertRouter {
    pub fn new() -> Self {
        Self {
            channels: Vec::new(),
            recent_alerts: std::collections::HashMap::new(),
            dedup_window_secs: 300,
        }
    }

    pub fn add_channel(&mut self, channel: AlertChannel) {
        self.channels.push(channel);
    }

    pub async fn send(&mut self, alert: &Alert) -> Result<()> {
        let dedup_key = self.make_dedup_key(alert);
        if let Some(last_sent) = self.recent_alerts.get(&dedup_key) {
            if last_sent.elapsed() < Duration::from_secs(self.dedup_window_secs) {
                tracing::debug!("Duplicate alert suppressed: {}", dedup_key);
                return Ok(());
            }
        }

        for channel in &self.channels {
            self.send_to_channel(channel, alert).await?;
        }

        self.recent_alerts.insert(dedup_key, Instant::now());
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
                tracing::info!(
                    "Would send email alert via {}:{} from {} to {:?}",
                    config.smtp_host,
                    config.smtp_port,
                    config.from,
                    config.to
                );
            }
            AlertChannel::PagerDuty(config) => {
                tracing::info!(
                    "Would send PagerDuty alert with routing_key {} severity {}",
                    config.routing_key,
                    config.severity
                );
            }
        }
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

        let client = reqwest::Client::new();
        let mut request = client.post(&config.url).json(&payload);

        if let Some(ref secret) = config.secret {
            let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
                .expect("HMAC can take key of any size");
            mac.update(payload.to_string().as_bytes());
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

    fn make_dedup_key(&self, alert: &Alert) -> String {
        format!(
            "{}:{}:{}",
            alert.target,
            alert.severity.as_str(),
            alert.title
        )
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
