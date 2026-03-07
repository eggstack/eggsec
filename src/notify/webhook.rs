use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::utils::create_http_client;
use anyhow::Result;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanStats {
    pub duration_ms: u64,
    pub requests_total: u64,
    pub requests_success: u64,
    pub requests_failed: u64,
    pub findings_total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WebhookEvent {
    ScanStarted,
    ScanComplete,
    ScanError,
    FindingDetected,
    RateLimited,
}

pub struct WebhookNotifier {
    client: reqwest::Client,
    webhooks: Vec<WebhookConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    pub name: String,
    pub url: String,
    pub secret: Option<String>,
    pub headers: HashMap<String, String>,
    pub events: Vec<WebhookEvent>,
}

impl WebhookNotifier {
    pub fn new(webhooks: Vec<WebhookConfig>) -> Result<Self> {
        let client = create_http_client(10)?;
        
        Ok(Self { client, webhooks })
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
    
    async fn send_webhook(&self, webhook: &WebhookConfig, payload: &NotificationPayload) -> Result<(), String> {
        let client = &self.client;
        
        let mut request = client.post(&webhook.url);
        
        if let Some(ref secret) = webhook.secret {
            request = request.header("X-Webhook-Secret", secret);
        }
        
        for (key, value) in &webhook.headers {
            request = request.header(key.as_str(), value.as_str());
        }
        
        request
            .json(payload)
            .send()
            .await
            .map_err(|e| format!("Webhook request failed: {}", e))?;
        
        Ok(())
    }
    
    pub async fn notify_slack(&self, webhook_url: &str, payload: &NotificationPayload) -> Result<(), String> {
        let slack_payload = self.build_slack_payload(payload);
        
        let response = self.client
            .post(webhook_url)
            .json(&slack_payload)
            .send()
            .await
            .map_err(|e| format!("Slack webhook failed: {}", e))?;
        
        if !response.status().is_success() {
            return Err(format!("Slack returned status {}", response.status()));
        }
        
        Ok(())
    }
    
    pub async fn notify_discord(&self, webhook_url: &str, payload: &NotificationPayload) -> Result<(), String> {
        let discord_payload = self.build_discord_payload(payload);
        
        let response = self.client
            .post(webhook_url)
            .json(&discord_payload)
            .send()
            .await
            .map_err(|e| format!("Discord webhook failed: {}", e))?;
        
        if !response.status().is_success() {
            return Err(format!("Discord returned status {}", response.status()));
        }
        
        Ok(())
    }
    
    pub async fn notify_teams(&self, webhook_url: &str, payload: &NotificationPayload) -> Result<(), String> {
        let teams_payload = self.build_teams_payload(payload);
        
        let response = self.client
            .post(webhook_url)
            .json(&teams_payload)
            .send()
            .await
            .map_err(|e| format!("Teams webhook failed: {}", e))?;
        
        if !response.status().is_success() {
            return Err(format!("Teams returned status {}", response.status()));
        }
        
        Ok(())
    }
    
    fn build_slack_payload(&self, payload: &NotificationPayload) -> serde_json::Value {
        let color = match payload.event {
            WebhookEvent::ScanStarted => "#36a64f",
            WebhookEvent::ScanComplete => "#36a64f",
            WebhookEvent::ScanError => "#dc3545",
            WebhookEvent::FindingDetected => "#ffc107",
            WebhookEvent::RateLimited => "#fd7e14",
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
            let findings_text: Vec<String> = findings.iter()
                .map(|f| format!("[{}] {} - {}", f.severity, f.finding_type, f.description))
                .collect();
            
            if let Some(obj) = attachment.as_object_mut() {
                obj.get_mut("fields").unwrap().as_array_mut().unwrap().push(serde_json::json!({
                    "title": "Findings",
                    "value": findings_text.join("\n"),
                    "short": false
                }));
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
            WebhookEvent::RateLimited => 0xfd7e14,
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
            WebhookEvent::RateLimited => "fd7e14",
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
            let findings_text: Vec<String> = findings.iter()
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
