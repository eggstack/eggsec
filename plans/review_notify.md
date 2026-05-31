# Notify Architecture Review
**Document:** architecture/notify.md
**Reviewed:** 2026-05-31
**Accuracy:** High
**Lines Reviewed:** 29

## Verified Claims
- `NotifyConfig` struct: Verified at `crates/slapper/src/notify/mod.rs:12` with fields `webhooks`, `slack_webhook`, `discord_webhook`, `teams_webhook`, `notify_on_start`, `notify_on_complete`, `notify_on_findings`, `notify_on_error`
- `NotifyManager` struct: Verified at `crates/slapper/src/notify/mod.rs:38`
- `WebhookNotifier` struct: Verified at `crates/slapper/src/notify/webhook.rs:51`
- `WebhookConfig` struct: Verified at `crates/slapper/src/notify/webhook.rs:57` with fields `name`, `url`, `secret`, `headers`, `events`
- `WebhookEvent` enum: Verified at `crates/slapper/src/notify/webhook.rs:43` with variants `ScanStarted`, `ScanComplete`, `ScanError`, `FindingDetected`, `RateLimited`
- `NotificationPayload` struct: Verified at `crates/slapper/src/notify/webhook.rs:12`
- `FindingSummary` struct: Verified at `crates/slapper/src/notify/webhook.rs:25`
- `ScanStats` struct: Verified at `crates/slapper/src/notify/webhook.rs:33`
- HMAC signing: Verified at `crates/slapper/src/notify/webhook.rs:98-107` using HMAC-SHA256
- Retry logic: Not explicitly present in `webhook.rs` - webhook sends once without retry
- Event filtering: Verified at `crates/slapper/src/notify/webhook.rs:76` (`webhook.events.contains(&payload.event)`)
- Multi-platform dispatch (webhooks, Slack, Discord, Teams): Verified in `NotifyManager` methods
- All files present: `mod.rs`, `webhook.rs` - verified

## Discrepancies
- **Retry logic claim**: Documented as "HMAC signing, retry logic, event filtering" in webhook.rs description. Actual: No retry logic exists in `webhook.rs`. The `send_webhook()` method (`webhook.rs:89`) sends a single request and returns success/failure. No retry loop or backoff mechanism.
- **`WebhookEvent` variants**: Documented as `Error`. Actual variant name is `ScanError` (`crates/slapper/src/notify/webhook.rs:46`)
- **`notify_findings()` missing Discord dispatch**: The `notify_findings()` method (`notify/mod.rs:199`) dispatches to webhooks, Slack, and Teams but does NOT dispatch to Discord, even though `NotifyManager` has a `discord_webhook` field and Discord dispatch is implemented in `notify_scan_complete()` and `notify_error()`

## Bugs Found
- **Incomplete Discord dispatch in `notify_findings()`**: The `notify_findings()` method at `crates/slapper/src/notify/mod.rs:199` sends to webhooks, Slack, and Teams but skips Discord. Compare with `notify_scan_complete()` (lines 118-196) and `notify_error()` (lines 261-315) which both dispatch to Discord. This appears to be a missing code path.

## Improvement Opportunities
- Add retry logic to webhook sends if reliability is a stated goal
- Add Discord dispatch to `notify_findings()` to match other notify methods
- The `WebhookEvent::RateLimited` variant is defined but never constructed anywhere in the codebase - consider removing or implementing rate-limit notification

## Stale Items
- "retry logic" claim is stale - no retry logic exists in the webhook implementation
