# Notify Module

## Purpose

Notification system supporting webhooks, Slack, Discord, and Microsoft Teams. Sends event-driven notifications for scan start, completion, findings detected, and errors.

## Key Types

| Type | Location | Description |
|------|----------|-------------|
| `NotificationConfig` | `config/scan.rs` | Notification configuration (webhook URLs, event filters, platform event filter) |
| `NotifyManager` | `notify/mod.rs` | Central notification dispatcher |
| `WebhookNotifier` | `notify/webhook.rs` | HTTP webhook sender with HMAC signing and retry logic |
| `WebhookConfig` | `config/scan.rs` | Individual webhook configuration (URL, secret, events) |
| `WebhookEvent` | `config/scan.rs` | Enum: ScanStarted, ScanComplete, ScanError, FindingDetected |
| `NotificationPayload` | `notify/webhook.rs` | Serialized notification body |
| `FindingSummary` | `notify/webhook.rs` | Finding summary for notifications |
| `ScanStats` | `notify/webhook.rs` | Scan statistics for notifications |

## Files

| File | Description |
|------|-------------|
| `mod.rs` | Module root: `NotifyManager`, multi-platform dispatch |
| `webhook.rs` | `WebhookNotifier` with HMAC signing, retry logic, event filtering, platform payload builders |

## Implementation Status

Fully implemented. `NotifyManager` dispatches to webhooks, Slack, Discord, and Teams. Supports event filtering (scan start/complete/findings/error) for all notification paths. Generic webhooks filter by per-webhook `events` field; Slack/Discord/Teams filter via `platform_event_filter` in config. All paths share retry logic with exponential backoff (3 retries). Generic webhooks support HMAC-SHA256 signing via `X-Signature-256` header.

## Wiring

`NotifyManager` is created in `CommandContext::new()` from `SlapperConfig` and is available on all CLI scan handlers. Each handler calls `notify_scan_started`, `notify_scan_complete`, and `notify_error` at appropriate lifecycle points.

## Retry Logic

All notification paths (generic webhooks, Slack, Discord, Teams) use shared retry logic:
- Max 3 retries with exponential backoff (1s, 2s base delays)
- HTTP response status is checked for all paths (4xx/5xx treated as failure)
- Generic webhooks also support HMAC-SHA256 signing via `X-Signature-256` header

## Event Filtering

- **Generic webhooks**: Each `WebhookConfig` has an `events` field; only matching events are delivered.
- **Platform notifiers** (Slack/Discord/Teams): Filtered by `platform_event_filter` in `NotificationConfig`. When `None`, all events are delivered.
