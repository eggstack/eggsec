# Notify Module

## Purpose

Notification system supporting webhooks, Slack, Discord, and Microsoft Teams. Sends event-driven notifications for scan start, completion, findings detected, and errors.

## Key Types

| Type | Location | Description |
|------|----------|-------------|
| `NotifyConfig` | `notify/mod.rs` | Notification configuration (webhook URLs, event filters) |
| `NotifyManager` | `notify/mod.rs` | Central notification dispatcher |
| `WebhookNotifier` | `notify/webhook.rs` | HTTP webhook sender with HMAC signing |
| `WebhookConfig` | `notify/webhook.rs` | Individual webhook configuration (URL, secret, events) |
| `WebhookEvent` | `notify/webhook.rs` | Enum: ScanStarted, ScanComplete, FindingDetected, Error |
| `NotificationPayload` | `notify/webhook.rs` | Serialized notification body |
| `FindingSummary` | `notify/webhook.rs` | Finding summary for notifications |
| `ScanStats` | `notify/webhook.rs` | Scan statistics for notifications |

## Files

| File | Description |
|------|-------------|
| `mod.rs` | Module root: `NotifyConfig`, `NotifyManager`, multi-platform dispatch |
| `webhook.rs` | `WebhookNotifier` with HMAC signing, retry logic, event filtering |

## Implementation Status

Fully implemented. `NotifyManager` dispatches to webhooks, Slack, Discord, and Teams. Supports event filtering (scan start/complete/findings/error) and webhook HMAC signing.
