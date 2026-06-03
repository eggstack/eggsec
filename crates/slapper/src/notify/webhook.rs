use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::config::{WebhookConfig, WebhookEvent};
use crate::utils::create_http_client;
use anyhow::Result;
use hmac::{Hmac, Mac};
use sha2::Sha256;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPayload {
    pub event: WebhookEvent,
    pub timestamp: DateTime<Utc>,
    pub scan_id: String,
    pub target: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub findings: Option<Vec<FindingSummary>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stats: Option<ScanStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingSummary {
    pub severity: String,
    pub finding_type: String,
    pub description: String,
    pub location: String,
}

#[cfg(any(feature = "tool-api", feature = "rest-api", feature = "grpc-api"))]
impl From<&crate::tool::finding::Finding> for FindingSummary {
    fn from(f: &crate::tool::finding::Finding) -> Self {
        Self {
            severity: f.severity.to_string(),
            finding_type: f.finding_type.to_string(),
            description: f.description.clone(),
            location: f.location.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanStats {
    pub duration_ms: u64,
    pub requests_total: u64,
    pub requests_success: u64,
    pub requests_failed: u64,
    pub findings_total: usize,
}

pub struct WebhookNotifier {
    client: reqwest::Client,
    webhooks: Vec<WebhookConfig>,
}

impl WebhookNotifier {
    pub fn new(webhooks: Vec<WebhookConfig>) -> Result<Self> {
        let client = create_http_client(10)?;

        Ok(Self { client, webhooks })
    }

    pub fn is_enabled(&self) -> bool {
        !self.webhooks.is_empty()
    }

    pub async fn notify(&self, payload: &NotificationPayload) -> Vec<Result<(), String>> {
        let mut results = Vec::new();

        for webhook in &self.webhooks {
            if !webhook.events.contains(&payload.event) {
                continue;
            }

            match self.send_webhook(webhook, payload).await {
                Ok(_) => results.push(Ok(())),
                Err(e) => results.push(Err(e)),
            }
        }

        results
    }

    async fn send_webhook(
        &self,
        webhook: &WebhookConfig,
        payload: &NotificationPayload,
    ) -> Result<(), String> {
        const MAX_RETRIES: u32 = 3;
        const BASE_DELAY_MS: u64 = 1000;

        let mut last_error = String::new();

        for attempt in 0..MAX_RETRIES {
            if attempt > 0 {
                let delay_ms = BASE_DELAY_MS * 2u64.pow(attempt - 1);
                tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
            }

            match self.try_send_webhook(webhook, payload).await {
                Ok(_) => return Ok(()),
                Err(e) => {
                    last_error = e;
                    if attempt < MAX_RETRIES - 1 {
                        tracing::warn!(
                            "Webhook delivery attempt {} failed, retrying: {}",
                            attempt + 1,
                            last_error
                        );
                    }
                }
            }
        }

        Err(last_error)
    }

    async fn try_send_webhook(
        &self,
        webhook: &WebhookConfig,
        payload: &NotificationPayload,
    ) -> Result<(), String> {
        let client = &self.client;

        let mut request = client.post(&webhook.url);

        if let Some(ref secret) = webhook.secret {
            type HmacSha256 = Hmac<Sha256>;
            let mut mac = HmacSha256::new_from_slice(secret.expose_secret().as_bytes())
                .map_err(|e| format!("HMAC error: {}", e))?;
            let canonical_json =
                serde_json::to_string(payload).map_err(|e| format!("JSON error: {}", e))?;
            mac.update(canonical_json.as_bytes());
            let result = mac.finalize();
            let signature = format!("sha256={}", hex::encode(result.into_bytes()));
            request = request.header("X-Signature-256", signature);
        }

        for (key, value) in &webhook.headers {
            request = request.header(key.as_str(), value.as_str());
        }

        let response = request
            .json(payload)
            .send()
            .await
            .map_err(|e| format!("Webhook request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Webhook returned status {}", response.status()));
        }

        Ok(())
    }

    pub async fn notify_slack(
        &self,
        webhook_url: &str,
        payload: &NotificationPayload,
    ) -> Result<(), String> {
        let slack_payload = self.build_slack_payload(payload);
        self.send_with_retry(webhook_url, &slack_payload, "Slack").await
    }

    pub async fn notify_discord(
        &self,
        webhook_url: &str,
        payload: &NotificationPayload,
    ) -> Result<(), String> {
        let discord_payload = self.build_discord_payload(payload);
        self.send_with_retry(webhook_url, &discord_payload, "Discord").await
    }

    pub async fn notify_teams(
        &self,
        webhook_url: &str,
        payload: &NotificationPayload,
    ) -> Result<(), String> {
        let teams_payload = self.build_teams_payload(payload);
        self.send_with_retry(webhook_url, &teams_payload, "Teams").await
    }

    async fn send_with_retry(
        &self,
        url: &str,
        payload: &serde_json::Value,
        platform: &str,
    ) -> Result<(), String> {
        const MAX_RETRIES: u32 = 3;
        const BASE_DELAY_MS: u64 = 1000;

        let mut last_error = String::new();

        for attempt in 0..MAX_RETRIES {
            if attempt > 0 {
                let delay_ms = BASE_DELAY_MS * 2u64.pow(attempt - 1);
                tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
            }

            match self
                .client
                .post(url)
                .json(payload)
                .send()
                .await
            {
                Ok(response) => {
                    if response.status().is_success() {
                        return Ok(());
                    }
                    last_error = format!("{} returned status {}", platform, response.status());
                }
                Err(e) => {
                    last_error = format!("{} webhook failed: {}", platform, e);
                }
            }

            if attempt < MAX_RETRIES - 1 {
                tracing::warn!(
                    "{} delivery attempt {} failed, retrying: {}",
                    platform,
                    attempt + 1,
                    last_error
                );
            }
        }

        Err(last_error)
    }

    fn build_slack_payload(&self, payload: &NotificationPayload) -> serde_json::Value {
        let color = match payload.event {
            WebhookEvent::ScanStarted => "#36a64f",
            WebhookEvent::ScanComplete => "#36a64f",
            WebhookEvent::ScanError => "#dc3545",
            WebhookEvent::FindingDetected => "#ffc107",
        };

        let mut attachment = serde_json::json!({
            "color": color,
            "title": format!("Slapper - {:?}", payload.event),
            "fields": [
                {
                    "title": "Target",
                    "value": payload.target,
                    "short": true
                },
                {
                    "title": "Scan ID",
                    "value": payload.scan_id,
                    "short": true
                }
            ],
            "footer": "Slapper Security Scanner",
            "ts": payload.timestamp.timestamp()
        });

        if let Some(ref findings) = payload.findings {
            let findings_text: Vec<String> = findings
                .iter()
                .map(|f| format!("[{}] {} - {}", f.severity, f.finding_type, f.description))
                .collect();

            if let Some(obj) = attachment.as_object_mut() {
                if let Some(fields) = obj.get_mut("fields").and_then(|v| v.as_array_mut()) {
                    fields.push(serde_json::json!({
                        "title": "Findings",
                        "value": findings_text.join("\n"),
                        "short": false
                    }));
                }
            }
        }

        serde_json::json!({
            "attachments": [attachment]
        })
    }

    fn build_discord_payload(&self, payload: &NotificationPayload) -> serde_json::Value {
        let color = match payload.event {
            WebhookEvent::ScanStarted => 0x36a64f,
            WebhookEvent::ScanComplete => 0x36a64f,
            WebhookEvent::ScanError => 0xdc3545,
            WebhookEvent::FindingDetected => 0xffc107,
        };

        let mut fields = vec![
            serde_json::json!({
                "name": "Target",
                "value": payload.target,
                "inline": true
            }),
            serde_json::json!({
                "name": "Scan ID",
                "value": payload.scan_id,
                "inline": true
            }),
        ];

        if let Some(ref stats) = payload.stats {
            fields.push(serde_json::json!({
                "name": "Statistics",
                "value": format!("Duration: {}ms\nRequests: {}/{}\nFindings: {}",
                    stats.duration_ms, stats.requests_success, stats.requests_total, stats.findings_total),
                "inline": false
            }));
        }

        serde_json::json!({
            "embeds": [{
                "title": format!("Slapper - {:?}", payload.event),
                "description": payload.message,
                "color": color,
                "fields": fields,
                "footer": {
                    "text": "Slapper Security Scanner"
                },
                "timestamp": payload.timestamp.to_rfc3339()
            }]
        })
    }

    fn build_teams_payload(&self, payload: &NotificationPayload) -> serde_json::Value {
        let color = match payload.event {
            WebhookEvent::ScanStarted => "36a64f",
            WebhookEvent::ScanComplete => "36a64f",
            WebhookEvent::ScanError => "dc3545",
            WebhookEvent::FindingDetected => "ffc107",
        };

        let mut facts = vec![
            serde_json::json!({ "name": "Target", "value": payload.target }),
            serde_json::json!({ "name": "Scan ID", "value": payload.scan_id }),
            serde_json::json!({ "name": "Event", "value": format!("{:?}", payload.event) }),
        ];

        if let Some(ref stats) = payload.stats {
            facts.push(serde_json::json!({
                "name": "Statistics",
                "value": format!("Duration: {}ms\nRequests: {}/{}",
                    stats.duration_ms, stats.requests_success, stats.requests_total)
            }));
        }

        if let Some(ref findings) = payload.findings {
            let findings_text: Vec<String> = findings
                .iter()
                .map(|f| format!("[{}] {} - {}", f.severity, f.finding_type, f.description))
                .collect();
            facts.push(serde_json::json!({
                "name": "Findings",
                "value": findings_text.join("\n")
            }));
        }

        serde_json::json!({
            "@type": "MessageCard",
            "@context": "http://schema.org/extensions",
            "themeColor": color,
            "summary": format!("Slapper - {:?}", payload.event),
            "sections": [{
                "activityTitle": format!("Slapper - {:?}", payload.event),
                "facts": facts,
                "markdown": true
            }],
            "potentialAction": [{
                "@type": "OpenUri",
                "name": "View Scan",
                "targets": [{
                    "os": "default",
                    "uri": payload.target
                }]
            }]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SensitiveString;

    #[test]
    fn test_webhook_event_serialization() {
        let events = vec![
            WebhookEvent::ScanStarted,
            WebhookEvent::ScanComplete,
            WebhookEvent::ScanError,
            WebhookEvent::FindingDetected,
        ];
        for event in &events {
            let json = serde_json::to_string(event).unwrap();
            let deserialized: WebhookEvent = serde_json::from_str(&json).unwrap();
            assert_eq!(*event, deserialized);
        }
    }

    #[test]
    fn test_notification_payload_serialization() {
        let payload = NotificationPayload {
            event: WebhookEvent::ScanComplete,
            timestamp: Utc::now(),
            scan_id: "test-scan".to_string(),
            target: "example.com".to_string(),
            message: "Test message".to_string(),
            findings: Some(vec![FindingSummary {
                severity: "high".to_string(),
                finding_type: "xss".to_string(),
                description: "Reflected XSS".to_string(),
                location: "/search".to_string(),
            }]),
            stats: Some(ScanStats {
                duration_ms: 1500,
                requests_total: 100,
                requests_success: 95,
                requests_failed: 5,
                findings_total: 3,
            }),
        };

        let json = serde_json::to_string(&payload).unwrap();
        let deserialized: NotificationPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(payload.event, deserialized.event);
        assert_eq!(payload.scan_id, deserialized.scan_id);
        assert_eq!(payload.target, deserialized.target);
        assert_eq!(payload.message, deserialized.message);
        assert!(deserialized.findings.is_some());
        assert!(deserialized.stats.is_some());
    }

    #[test]
    fn test_notification_payload_skips_none_fields() {
        let payload = NotificationPayload {
            event: WebhookEvent::ScanStarted,
            timestamp: Utc::now(),
            scan_id: "test".to_string(),
            target: "test.com".to_string(),
            message: "test".to_string(),
            findings: None,
            stats: None,
        };

        let json = serde_json::to_string(&payload).unwrap();
        assert!(!json.contains("findings"));
        assert!(!json.contains("stats"));
    }

    #[test]
    fn test_finding_summary_creation() {
        let finding = FindingSummary {
            severity: "critical".to_string(),
            finding_type: "sqli".to_string(),
            description: "SQL injection in login".to_string(),
            location: "/login".to_string(),
        };
        assert_eq!(finding.severity, "critical");
        assert_eq!(finding.location, "/login");
    }

    #[test]
    fn test_scan_stats_creation() {
        let stats = ScanStats {
            duration_ms: 5000,
            requests_total: 200,
            requests_success: 190,
            requests_failed: 10,
            findings_total: 5,
        };
        assert_eq!(stats.duration_ms, 5000);
        assert_eq!(stats.findings_total, 5);
    }

    #[test]
    fn test_webhook_config_creation() {
        let config = WebhookConfig {
            name: Some("test-webhook".to_string()),
            url: "https://example.com/hook".to_string(),
            secret: Some(SensitiveString::new("my-secret")),
            headers: rustc_hash::FxHashMap::default(),
            events: vec![WebhookEvent::ScanComplete, WebhookEvent::FindingDetected],
        };
        assert_eq!(config.name, Some("test-webhook".to_string()));
        assert_eq!(config.events.len(), 2);
        assert!(config.secret.is_some());
    }

    #[test]
    fn test_webhook_notifier_is_enabled() {
        let notifier_empty = WebhookNotifier {
            client: create_http_client(10).unwrap(),
            webhooks: vec![],
        };
        assert!(!notifier_empty.is_enabled());

        let notifier_with_hooks = WebhookNotifier {
            client: create_http_client(10).unwrap(),
            webhooks: vec![WebhookConfig {
                name: Some("test".to_string()),
                url: "https://example.com".to_string(),
                secret: None,
                headers: rustc_hash::FxHashMap::default(),
                events: vec![WebhookEvent::ScanComplete],
            }],
        };
        assert!(notifier_with_hooks.is_enabled());
    }

    #[test]
    fn test_build_slack_payload() {
        let notifier = WebhookNotifier {
            client: create_http_client(10).unwrap(),
            webhooks: vec![],
        };
        let payload = NotificationPayload {
            event: WebhookEvent::ScanComplete,
            timestamp: Utc::now(),
            scan_id: "s1".to_string(),
            target: "example.com".to_string(),
            message: "Done".to_string(),
            findings: None,
            stats: None,
        };
        let slack = notifier.build_slack_payload(&payload);
        assert!(slack.get("attachments").is_some());
        let attachments = slack["attachments"].as_array().unwrap();
        assert_eq!(attachments.len(), 1);
        assert_eq!(attachments[0]["color"], "#36a64f");
    }

    #[test]
    fn test_build_discord_payload() {
        let notifier = WebhookNotifier {
            client: create_http_client(10).unwrap(),
            webhooks: vec![],
        };
        let payload = NotificationPayload {
            event: WebhookEvent::ScanError,
            timestamp: Utc::now(),
            scan_id: "s2".to_string(),
            target: "example.com".to_string(),
            message: "Error occurred".to_string(),
            findings: None,
            stats: None,
        };
        let discord = notifier.build_discord_payload(&payload);
        assert!(discord.get("embeds").is_some());
        let embeds = discord["embeds"].as_array().unwrap();
        assert_eq!(embeds.len(), 1);
        assert_eq!(embeds[0]["color"], 0xdc3545);
    }

    #[test]
    fn test_build_teams_payload() {
        let notifier = WebhookNotifier {
            client: create_http_client(10).unwrap(),
            webhooks: vec![],
        };
        let payload = NotificationPayload {
            event: WebhookEvent::FindingDetected,
            timestamp: Utc::now(),
            scan_id: "s3".to_string(),
            target: "example.com".to_string(),
            message: "Findings found".to_string(),
            findings: None,
            stats: None,
        };
        let teams = notifier.build_teams_payload(&payload);
        assert_eq!(teams["@type"], "MessageCard");
        assert_eq!(teams["themeColor"], "ffc107");
    }

    #[test]
    fn test_slack_payload_with_findings() {
        let notifier = WebhookNotifier {
            client: create_http_client(10).unwrap(),
            webhooks: vec![],
        };
        let payload = NotificationPayload {
            event: WebhookEvent::FindingDetected,
            timestamp: Utc::now(),
            scan_id: "s4".to_string(),
            target: "example.com".to_string(),
            message: "1 finding".to_string(),
            findings: Some(vec![FindingSummary {
                severity: "high".to_string(),
                finding_type: "xss".to_string(),
                description: "XSS in search".to_string(),
                location: "/q".to_string(),
            }]),
            stats: None,
        };
        let slack = notifier.build_slack_payload(&payload);
        let fields = slack["attachments"][0]["fields"].as_array().unwrap();
        assert_eq!(fields.len(), 3);
        assert_eq!(fields[2]["title"], "Findings");
    }

    #[test]
    fn test_discord_payload_with_stats() {
        let notifier = WebhookNotifier {
            client: create_http_client(10).unwrap(),
            webhooks: vec![],
        };
        let payload = NotificationPayload {
            event: WebhookEvent::ScanComplete,
            timestamp: Utc::now(),
            scan_id: "s5".to_string(),
            target: "example.com".to_string(),
            message: "Done".to_string(),
            findings: None,
            stats: Some(ScanStats {
                duration_ms: 2000,
                requests_total: 50,
                requests_success: 48,
                requests_failed: 2,
                findings_total: 1,
            }),
        };
        let discord = notifier.build_discord_payload(&payload);
        let fields = discord["embeds"][0]["fields"].as_array().unwrap();
        assert_eq!(fields.len(), 3);
        assert_eq!(fields[2]["name"], "Statistics");
    }
}
