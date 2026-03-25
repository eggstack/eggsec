use crate::notify::{WebhookNotifier, NotificationPayload, WebhookConfig, WebhookEvent};

pub struct WebhookTestConfig {
    pub slack: Option<String>,
    pub discord: Option<String>,
    pub teams: Option<String>,
    pub webhook: Option<String>,
    pub secret: Option<String>,
}

fn print_send_result(result: Result<(), String>, provider: &str) {
    match result {
        Ok(_) => println!("  ✓ {} notification sent successfully!", provider),
        Err(e) => println!("  ✗ {} failed: {}", provider, e),
    }
}

pub async fn send_webhook_notifications(
    config: &WebhookTestConfig,
    payload: &NotificationPayload,
    custom_webhooks: Option<Vec<WebhookConfig>>,
) {
    let notifier = WebhookNotifier::new(custom_webhooks.unwrap_or_default()).ok();

    if let Some(ref slack_url) = config.slack {
        println!("Sending test to Slack: {}", slack_url);
        if let Some(ref n) = notifier {
            let result = n.notify_slack(slack_url, payload).await;
            print_send_result(result, "Slack");
        } else {
            println!("  ✗ Failed to create notifier");
        }
    }

    if let Some(ref discord_url) = config.discord {
        println!("Sending test to Discord: {}", discord_url);
        if let Some(ref n) = notifier {
            let result = n.notify_discord(discord_url, payload).await;
            print_send_result(result, "Discord");
        } else {
            println!("  ✗ Failed to create notifier");
        }
    }

    if let Some(ref teams_url) = config.teams {
        println!("Sending test to Teams: {}", teams_url);
        if let Some(ref n) = notifier {
            let result = n.notify_teams(teams_url, payload).await;
            print_send_result(result, "Teams");
        } else {
            println!("  ✗ Failed to create notifier");
        }
    }

    if let Some(ref webhook_url) = config.webhook {
        println!("Sending test to custom webhook: {}", webhook_url);
        let webhook_config = vec![WebhookConfig {
            name: "test".to_string(),
            url: webhook_url.clone(),
            secret: config.secret.clone(),
            headers: std::collections::HashMap::new(),
            events: vec![WebhookEvent::ScanComplete],
        }];
        if let Ok(notifier) = WebhookNotifier::new(webhook_config) {
            match notifier.notify(payload).await.first() {
                Some(Ok(_)) => println!("  ✓ Test notification sent successfully!"),
                Some(Err(e)) => println!("  ✗ Failed: {}", e),
                None => println!("  No webhooks configured"),
            }
        }
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
