
mod webhook;

pub use webhook::{
    FindingSummary, NotificationPayload, ScanStats, WebhookConfig, WebhookEvent, WebhookNotifier,
};

use crate::config::WebhookEvent as ConfigWebhookEvent;
use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotifyConfig {
    pub webhooks: Vec<webhook::WebhookConfig>,
    pub slack_webhook: Option<String>,
    pub discord_webhook: Option<String>,
    pub teams_webhook: Option<String>,
    pub notify_on_start: bool,
    pub notify_on_complete: bool,
    pub notify_on_findings: bool,
    pub notify_on_error: bool,
}

impl Default for NotifyConfig {
    fn default() -> Self {
        Self {
            webhooks: Vec::new(),
            slack_webhook: None,
            discord_webhook: None,
            teams_webhook: None,
            notify_on_start: false,
            notify_on_complete: true,
            notify_on_findings: true,
            notify_on_error: true,
        }
    }
}

pub struct NotifyManager {
    notifier: Option<WebhookNotifier>,
    slack_webhook: Option<String>,
    discord_webhook: Option<String>,
    teams_webhook: Option<String>,
    config: NotifyConfig,
}

impl NotifyManager {
    pub fn new(config: NotifyConfig) -> Self {
        let notifier = if config.webhooks.is_empty() {
            None
        } else {
            WebhookNotifier::new(config.webhooks.clone()).ok()
        };

        Self {
            notifier,
            slack_webhook: config.slack_webhook.clone(),
            discord_webhook: config.discord_webhook.clone(),
            teams_webhook: config.teams_webhook.clone(),
            config,
        }
    }

    pub fn from_settings(settings: &crate::config::SlapperConfig) -> Self {
        let notify_settings = &settings.notifications;

        let mut webhooks = Vec::new();

        for wh in &notify_settings.webhooks {
            webhooks.push(webhook::WebhookConfig {
                name: wh.name.clone(),
                url: wh.url.clone(),
                secret: wh.secret.as_ref().map(|s| s.expose_secret().to_string()),
                headers: wh.headers.clone(),
                events: wh
                    .events
                    .iter()
                    .map(|e| match e {
                        ConfigWebhookEvent::ScanStart => WebhookEvent::ScanStarted,
                        ConfigWebhookEvent::ScanComplete => WebhookEvent::ScanComplete,
                        ConfigWebhookEvent::Finding => WebhookEvent::FindingDetected,
                        ConfigWebhookEvent::Error => WebhookEvent::ScanError,
                    })
                    .collect(),
            });
        }

        Self::new(NotifyConfig {
            webhooks,
            slack_webhook: notify_settings.slack_webhook.clone(),
            discord_webhook: notify_settings.discord_webhook.clone(),
            teams_webhook: notify_settings.teams_webhook.clone(),
            notify_on_start: false,
            notify_on_complete: notify_settings.notify_on_complete,
            notify_on_findings: notify_settings.notify_on_findings,
            notify_on_error: true,
        })
    }

    pub async fn notify_scan_started(&self, scan_id: &str, target: &str) {
        if !self.config.notify_on_start {
            return;
        }

        if let Some(ref notifier) = self.notifier {
            let payload = NotificationPayload {
                event: WebhookEvent::ScanStarted,
                timestamp: Utc::now(),
                scan_id: scan_id.to_string(),
                target: target.to_string(),
                message: "Scan started".to_string(),
                findings: None,
                stats: None,
            };
            let _ = notifier.notify(&payload).await;
        }
    }

    pub async fn notify_scan_complete(
        &self,
        scan_id: &str,
        target: &str,
        message: &str,
        findings: Option<Vec<FindingSummary>>,
        stats: Option<ScanStats>,
    ) {
        if !self.config.notify_on_complete && findings.is_none() {
            return;
        }

        if let Some(ref notifier) = self.notifier {
            let payload = NotificationPayload {
                event: WebhookEvent::ScanComplete,
                timestamp: Utc::now(),
                scan_id: scan_id.to_string(),
                target: target.to_string(),
                message: message.to_string(),
                findings: findings.clone(),
                stats: stats.clone(),
            };
            for result in notifier.notify(&payload).await {
                if let Err(e) = result {
                    tracing::warn!("Webhook notification failed: {}", e);
                }
            }
        }

        if let Some(ref slack_url) = self.slack_webhook {
            if let Some(ref notifier) = self.notifier {
                let payload = NotificationPayload {
                    event: WebhookEvent::ScanComplete,
                    timestamp: Utc::now(),
                    scan_id: scan_id.to_string(),
                    target: target.to_string(),
                    message: message.to_string(),
                    findings: findings.clone(),
                    stats: stats.clone(),
                };
                if let Err(e) = notifier.notify_slack(slack_url, &payload).await {
                    tracing::warn!("Slack notification failed: {}", e);
                }
            }
        }

        if let Some(ref discord_url) = self.discord_webhook {
            if let Some(ref notifier) = self.notifier {
                let payload = NotificationPayload {
                    event: WebhookEvent::ScanComplete,
                    timestamp: Utc::now(),
                    scan_id: scan_id.to_string(),
                    target: target.to_string(),
                    message: message.to_string(),
                    findings: findings.clone(),
                    stats,
                };
                if let Err(e) = notifier.notify_discord(discord_url, &payload).await {
                    tracing::warn!("Discord notification failed: {}", e);
                }
            }
        }

        if let Some(ref teams_url) = self.teams_webhook {
            if let Some(ref notifier) = self.notifier {
                let payload = NotificationPayload {
                    event: WebhookEvent::ScanComplete,
                    timestamp: Utc::now(),
                    scan_id: scan_id.to_string(),
                    target: target.to_string(),
                    message: message.to_string(),
                    findings,
                    stats: None,
                };
                if let Err(e) = notifier.notify_teams(teams_url, &payload).await {
                    tracing::warn!("Teams notification failed: {}", e);
                }
            }
        }
    }

    pub async fn notify_findings(
        &self,
        scan_id: &str,
        target: &str,
        findings: Vec<FindingSummary>,
    ) {
        if !self.config.notify_on_findings {
            return;
        }

        if let Some(ref notifier) = self.notifier {
            let payload = NotificationPayload {
                event: WebhookEvent::FindingDetected,
                timestamp: Utc::now(),
                scan_id: scan_id.to_string(),
                target: target.to_string(),
                message: format!("{} findings detected", findings.len()),
                findings: Some(findings.clone()),
                stats: None,
            };
            for result in notifier.notify(&payload).await {
                if let Err(e) = result {
                    tracing::warn!("Webhook notification failed: {}", e);
                }
            }
        }

        if let Some(ref slack_url) = self.slack_webhook {
            if let Some(ref notifier) = self.notifier {
                let payload = NotificationPayload {
                    event: WebhookEvent::FindingDetected,
                    timestamp: Utc::now(),
                    scan_id: scan_id.to_string(),
                    target: target.to_string(),
                    message: format!("{} findings detected", findings.len()),
                    findings: Some(findings.clone()),
                    stats: None,
                };
                if let Err(e) = notifier.notify_slack(slack_url, &payload).await {
                    tracing::warn!("Slack notification failed: {}", e);
                }
            }
        }

        if let Some(ref teams_url) = self.teams_webhook {
            if let Some(ref notifier) = self.notifier {
                let payload = NotificationPayload {
                    event: WebhookEvent::FindingDetected,
                    timestamp: Utc::now(),
                    scan_id: scan_id.to_string(),
                    target: target.to_string(),
                    message: format!("{} findings detected", findings.len()),
                    findings: Some(findings),
                    stats: None,
                };
                if let Err(e) = notifier.notify_teams(teams_url, &payload).await {
                    tracing::warn!("Teams notification failed: {}", e);
                }
            }
        }
    }

    pub async fn notify_error(&self, scan_id: &str, target: &str, error: &str) {
        if !self.config.notify_on_error {
            return;
        }

        if let Some(ref notifier) = self.notifier {
            let payload = NotificationPayload {
                event: WebhookEvent::ScanError,
                timestamp: Utc::now(),
                scan_id: scan_id.to_string(),
                target: target.to_string(),
                message: error.to_string(),
                findings: None,
                stats: None,
            };
            for result in notifier.notify(&payload).await {
                if let Err(e) = result {
                    tracing::warn!("Webhook notification failed: {}", e);
                }
            }
        }

        if let Some(ref discord_url) = self.discord_webhook {
            if let Some(ref notifier) = self.notifier {
                let payload = NotificationPayload {
                    event: WebhookEvent::ScanError,
                    timestamp: Utc::now(),
                    scan_id: scan_id.to_string(),
                    target: target.to_string(),
                    message: error.to_string(),
                    findings: None,
                    stats: None,
                };
                if let Err(e) = notifier.notify_discord(discord_url, &payload).await {
                    tracing::warn!("Discord notification failed: {}", e);
                }
            }
        }

        if let Some(ref teams_url) = self.teams_webhook {
            if let Some(ref notifier) = self.notifier {
                let payload = NotificationPayload {
                    event: WebhookEvent::ScanError,
                    timestamp: Utc::now(),
                    scan_id: scan_id.to_string(),
                    target: target.to_string(),
                    message: error.to_string(),
                    findings: None,
                    stats: None,
                };
                if let Err(e) = notifier.notify_teams(teams_url, &payload).await {
                    tracing::warn!("Teams notification failed: {}", e);
                }
            }
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.notifier.is_some()
            || self.slack_webhook.is_some()
            || self.discord_webhook.is_some()
            || self.teams_webhook.is_some()
    }
}

impl Default for NotifyManager {
    fn default() -> Self {
        Self::new(NotifyConfig::default())
    }
}
