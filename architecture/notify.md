# Notify Module

## Purpose

Notification system supporting webhooks, Slack, Discord, and Microsoft Teams. Sends event-driven notifications for scan start, completion, findings detected, and errors.

## Key Types

| Type | Location | Description |
|------|----------|-------------|
| `NotifyConfig` | `notify/mod.rs` | Notification configuration (webhook URLs, event filters) |
| `NotifyManager` | `notify/mod.rs` | Central notification dispatcher |
| `WebhookNotifier` | `notify/webhook.rs` | HTTP webhook sender with HMAC signing and retry logic |
| `WebhookConfig` | `notify/webhook.rs` | Individual webhook configuration (URL, secret, events) |
| `WebhookEvent` | `notify/webhook.rs` | Enum: ScanStarted, ScanComplete, ScanError, FindingDetected |
| `NotificationPayload` | `notify/webhook.rs` | Serialized notification body |
| `FindingSummary` | `notify/webhook.rs` | Finding summary for notifications |
| `ScanStats` | `notify/webhook.rs` | Scan statistics for notifications |

## Files

| File | Description |
|------|-------------|
| `mod.rs` | Module root: `NotifyConfig`, `NotifyManager`, multi-platform dispatch with deduplication |
| `webhook.rs` | `WebhookNotifier` with HMAC signing, retry logic, event filtering, platform payload builders |

## Implementation Status

Fully implemented. `NotifyManager` dispatches to webhooks, Slack, Discord, and Teams. Supports event filtering (scan start/complete/findings/error) and webhook HMAC signing. All platform notifiers (Slack, Discord, Teams) share retry logic with exponential backoff (3 retries).

## Wiring

`NotifyManager` is created in `CommandContext::new()` from `SlapperConfig` and is available on all CLI scan handlers. Each handler calls `notify_scan_started`, `notify_scan_complete`, and `notify_error` at appropriate lifecycle points.

## Retry Logic

All notification paths (generic webhooks, Slack, Discord, Teams) use shared retry logic:
- Max 3 retries with exponential backoff (1s, 2s base delays)
- HTTP response status is checked for all paths (4xx/5xx treated as failure)
- Generic webhooks also support HMAC-SHA256 signing via `X-Signature-256` header
