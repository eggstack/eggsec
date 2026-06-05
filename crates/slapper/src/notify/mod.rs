mod webhook;

pub use webhook::{FindingSummary, NotificationPayload, ScanStats, WebhookNotifier};

pub use crate::config::{WebhookConfig, WebhookEvent};

use crate::config::NotificationConfig;
use chrono::Utc;

pub struct NotifyManager {
    notifier: WebhookNotifier,
    slack_webhook: Option<String>,
    discord_webhook: Option<String>,
    teams_webhook: Option<String>,
    config: NotificationConfig,
}

impl NotifyManager {
    pub fn new(config: NotificationConfig) -> Self {
        let notifier = match WebhookNotifier::new(config.webhooks.clone()) {
            Ok(n) => n,
            Err(e) => {
                tracing::warn!("Failed to create webhook notifier, using empty list: {}", e);
                WebhookNotifier::new(vec![]).expect("empty webhook list")
            }
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

        let webhooks: Vec<WebhookConfig> = notify_settings
            .webhooks
            .iter()
            .map(|wh| WebhookConfig {
                name: wh.name.clone().or_else(|| Some("unnamed".to_string())),
                url: wh.url.clone(),
                secret: wh.secret.clone(),
                headers: wh.headers.clone(),
                events: wh.events.clone(),
            })
            .collect();

        Self::new(NotificationConfig {
            webhooks,
            slack_webhook: notify_settings.slack_webhook.clone(),
            discord_webhook: notify_settings.discord_webhook.clone(),
            teams_webhook: notify_settings.teams_webhook.clone(),
            platform_event_filter: notify_settings.platform_event_filter.clone(),
            notify_on_complete: notify_settings.notify_on_complete,
            notify_on_findings: notify_settings.notify_on_findings,
            notify_on_error: notify_settings.notify_on_error,
        })
    }

    pub async fn notify_scan_started(&self, scan_id: &str, target: &str) {
        let payload = NotificationPayload {
            event: WebhookEvent::ScanStarted,
            timestamp: Utc::now(),
            scan_id: scan_id.to_string(),
            target: target.to_string(),
            message: "Scan started".to_string(),
            findings: None,
            stats: None,
        };
        self.dispatch(&payload).await;
    }

    pub async fn notify_scan_complete(
        &self,
        scan_id: &str,
        target: &str,
        message: &str,
        findings: Option<Vec<FindingSummary>>,
        stats: Option<ScanStats>,
    ) {
        // Findings always trigger a notification even if notify_on_complete is disabled,
        // since finding delivery is controlled separately by notify_on_findings.
        if !self.config.notify_on_complete && findings.is_none() {
            return;
        }

        let payload = NotificationPayload {
            event: WebhookEvent::ScanComplete,
            timestamp: Utc::now(),
            scan_id: scan_id.to_string(),
            target: target.to_string(),
            message: message.to_string(),
            findings,
            stats,
        };
        self.dispatch(&payload).await;
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

        let payload = NotificationPayload {
            event: WebhookEvent::FindingDetected,
            timestamp: Utc::now(),
            scan_id: scan_id.to_string(),
            target: target.to_string(),
            message: format!("{} findings detected", findings.len()),
            findings: Some(findings),
            stats: None,
        };
        self.dispatch(&payload).await;
    }

    pub async fn notify_error(&self, scan_id: &str, target: &str, error: &str) {
        if !self.config.notify_on_error {
            return;
        }

        let payload = NotificationPayload {
            event: WebhookEvent::ScanError,
            timestamp: Utc::now(),
            scan_id: scan_id.to_string(),
            target: target.to_string(),
            message: error.to_string(),
            findings: None,
            stats: None,
        };
        self.dispatch(&payload).await;
    }

    async fn dispatch(&self, payload: &NotificationPayload) {
        // Notifications are fire-and-forget: failures are logged but never propagated
        // to avoid aborting scans over transient webhook issues.
        for result in self.notifier.notify(payload).await {
            if let Err(e) = result {
                tracing::warn!("Webhook notification failed: {}", e);
            }
        }

        if let Some(ref slack_url) = self.slack_webhook {
            let filter = self.config.platform_event_filter.as_deref();
            if let Err(e) = self
                .notifier
                .notify_slack(slack_url, payload, filter)
                .await
            {
                tracing::warn!("Slack notification failed: {}", e);
            }
        }

        if let Some(ref discord_url) = self.discord_webhook {
            let filter = self.config.platform_event_filter.as_deref();
            if let Err(e) = self
                .notifier
                .notify_discord(discord_url, payload, filter)
                .await
            {
                tracing::warn!("Discord notification failed: {}", e);
            }
        }

        if let Some(ref teams_url) = self.teams_webhook {
            let filter = self.config.platform_event_filter.as_deref();
            if let Err(e) = self
                .notifier
                .notify_teams(teams_url, payload, filter)
                .await
            {
                tracing::warn!("Teams notification failed: {}", e);
            }
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.notifier.is_enabled()
            || self.slack_webhook.is_some()
            || self.discord_webhook.is_some()
            || self.teams_webhook.is_some()
    }
}

impl Default for NotifyManager {
    fn default() -> Self {
        Self::new(NotificationConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notify_config_defaults() {
        let config = NotificationConfig::default();
        assert!(config.webhooks.is_empty());
        assert!(config.slack_webhook.is_none());
        assert!(config.discord_webhook.is_none());
        assert!(config.teams_webhook.is_none());
        assert!(config.notify_on_complete);
        assert!(config.notify_on_findings);
        assert!(config.notify_on_error);
    }

    #[test]
    fn test_notify_manager_default() {
        let manager = NotifyManager::default();
        assert!(!manager.is_enabled());
    }

    #[test]
    fn test_notify_manager_is_enabled_with_webhook() {
        let config = NotificationConfig {
            webhooks: vec![WebhookConfig {
                name: Some("test".to_string()),
                url: "https://example.com".to_string(),
                secret: None,
                headers: rustc_hash::FxHashMap::default(),
                events: vec![WebhookEvent::ScanComplete],
            }],
            ..Default::default()
        };
        let manager = NotifyManager::new(config);
        assert!(manager.is_enabled());
    }

    #[test]
    fn test_notify_manager_is_enabled_with_slack() {
        let config = NotificationConfig {
            slack_webhook: Some("https://hooks.slack.com/test".to_string()),
            ..Default::default()
        };
        let manager = NotifyManager::new(config);
        assert!(manager.is_enabled());
    }

    #[test]
    fn test_notify_manager_is_enabled_with_discord() {
        let config = NotificationConfig {
            discord_webhook: Some("https://discord.com/api/webhooks/test".to_string()),
            ..Default::default()
        };
        let manager = NotifyManager::new(config);
        assert!(manager.is_enabled());
    }

    #[test]
    fn test_notify_manager_is_enabled_with_teams() {
        let config = NotificationConfig {
            teams_webhook: Some("https://outlook.office.com/webhook/test".to_string()),
            ..Default::default()
        };
        let manager = NotifyManager::new(config);
        assert!(manager.is_enabled());
    }

    #[test]
    fn test_notify_manager_created_with_empty_webhooks() {
        let config = NotificationConfig {
            webhooks: vec![],
            slack_webhook: Some("https://hooks.slack.com/test".to_string()),
            ..Default::default()
        };
        let manager = NotifyManager::new(config);
        assert!(manager.is_enabled());
    }

    #[test]
    fn test_notify_config_serialization() {
        let config = NotificationConfig {
            webhooks: vec![WebhookConfig {
                name: Some("my-hook".to_string()),
                url: "https://example.com/hook".to_string(),
                secret: None,
                headers: rustc_hash::FxHashMap::default(),
                events: vec![WebhookEvent::ScanComplete],
            }],
            slack_webhook: Some("https://hooks.slack.com/test".to_string()),
            discord_webhook: None,
            teams_webhook: None,
            platform_event_filter: None,
            notify_on_complete: true,
            notify_on_findings: false,
            notify_on_error: true,
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: NotificationConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.webhooks.len(), 1);
        assert_eq!(deserialized.webhooks[0].name, Some("my-hook".to_string()));
        assert_eq!(
            deserialized.slack_webhook,
            Some("https://hooks.slack.com/test".to_string())
        );
        assert!(deserialized.notify_on_complete);
        assert!(!deserialized.notify_on_findings);
    }
}
