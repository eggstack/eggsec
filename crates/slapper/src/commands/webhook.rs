use crate::error::{Result, SlapperError};
use crate::notify::{WebhookNotifier, NotificationPayload, WebhookConfig, WebhookEvent};
use crate::types::SensitiveString;

pub struct WebhookTestConfig {
    pub slack: Option<String>,
    pub discord: Option<String>,
    pub teams: Option<String>,
    pub webhook: Option<String>,
    pub secret: Option<String>,
}

pub async fn send_webhook_notifications(
    config: &WebhookTestConfig,
    payload: &NotificationPayload,
    custom_webhooks: Option<Vec<WebhookConfig>>,
) -> Result<()> {
    let notifier = WebhookNotifier::new(custom_webhooks.unwrap_or_default())?;

    let mut errors = Vec::new();

    if let Some(ref slack_url) = config.slack {
        println!("Sending test to Slack: {}", slack_url);
        if let Err(e) = notifier.notify_slack(slack_url, payload).await {
            println!("  ✗ Slack failed: {}", e);
            errors.push("Slack");
        } else {
            println!("  ✓ Slack notification sent successfully!");
        }
    }

    if let Some(ref discord_url) = config.discord {
        println!("Sending test to Discord: {}", discord_url);
        if let Err(e) = notifier.notify_discord(discord_url, payload).await {
            println!("  ✗ Discord failed: {}", e);
            errors.push("Discord");
        } else {
            println!("  ✓ Discord notification sent successfully!");
        }
    }

    if let Some(ref teams_url) = config.teams {
        println!("Sending test to Teams: {}", teams_url);
        if let Err(e) = notifier.notify_teams(teams_url, payload).await {
            println!("  ✗ Teams failed: {}", e);
            errors.push("Teams");
        } else {
            println!("  ✓ Teams notification sent successfully!");
        }
    }

    if let Some(ref webhook_url) = config.webhook {
        println!("Sending test to custom webhook: {}", webhook_url);
        let webhook_config = vec![WebhookConfig {
            name: "test".to_string(),
            url: webhook_url.clone(),
            secret: config.secret.clone().map(SensitiveString::new),
            headers: std::collections::HashMap::new(),
            events: vec![WebhookEvent::ScanComplete],
        }];
        let notifier = WebhookNotifier::new(webhook_config)?;
        match notifier.notify(payload).await.first() {
            Some(Ok(_)) => println!("  ✓ Test notification sent successfully!"),
            Some(Err(e)) => {
                println!("  ✗ Failed: {}", e);
                errors.push("Webhook");
            }
            None => println!("  No webhooks configured"),
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(SlapperError::Config(format!("Failed to send notifications to: {}", errors.join(", "))))
    }
}

pub fn has_any_webhook(config: &WebhookTestConfig) -> bool {
    config.slack.is_some() || config.discord.is_some() || config.teams.is_some() || config.webhook.is_some()
}

pub fn print_webhook_usage() {
    println!("No webhook URL provided.");
    println!("Usage:");
    println!("  slapper notify test --slack <url>");
    println!("  slapper notify test --discord <url>");
    println!("  slapper notify test --teams <url>");
    println!("  slapper notify test --webhook <url> [--secret <secret>]");
}
