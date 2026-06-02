# Notify Module Architecture Review

**Document:** architecture/notify.md
**Reviewed:** 2026-06-02
**Accuracy:** High
**Lines Reviewed:** 29

## Verified Claims
- [NotifyConfig]: Verified at `crates/slapper/src/notify/mod.rs:12`
- [NotifyManager]: Verified at `crates/slapper/src/notify/mod.rs:38`
- [WebhookNotifier]: Verified at `crates/slapper/src/notify/webhook.rs:51`
- [WebhookConfig]: Verified at `crates/slapper/src/notify/webhook.rs:57`
- [WebhookEvent enum]: Verified at `crates/slapper/src/notify/webhook.rs:43-49` (ScanStarted, ScanComplete, ScanError, FindingDetected, RateLimited)
- [NotificationPayload]: Verified at `crates/slapper/src/notify/webhook.rs:12`
- [FindingSummary]: Verified at `crates/slapper/src/notify/webhook.rs:25`
- [ScanStats]: Verified at `crates/slapper/src/notify/webhook.rs:33`
- [HMAC signing for webhooks]: Verified at `crates/slapper/src/notify/webhook.rs:98-107`
- [Event filtering]: Verified at `crates/slapper/src/notify/webhook.rs:76-78`
- [Files: mod.rs and webhook.rs]: Verified

## Discrepancies
- None significant.

## Bugs Found
- [Silent error suppression with let _]: In `crates/slapper/src/notify/mod.rs:114`, `let _ = notifier.notify(&payload).await;` silently ignores notification failures. This pattern appears multiple times (lines 140-143, 219-222, 293-296). Should use `tracing::warn` or similar (priority: medium)

## Improvement Opportunities
- [Duplicate payload construction]: The `notify_scan_complete()` method (lines 118-196) constructs the same `NotificationPayload` multiple times for webhooks, Slack, Discord, and Teams. Each platform gets a clone of the payload. Consider constructing once and cloning (priority: low)
- [No retry logic for failed notifications]: The `WebhookNotifier::notify()` method (line 72) does not implement retry logic. If a webhook fails, it's not retried (priority: medium)
- [RateLimited event never dispatched]: The `WebhookEvent::RateLimited` variant exists but `NotifyManager` has no `notify_rate_limited()` method to actually send this event (priority: low)

## Stale Items
- None.

## Code Interrogation Findings
- [teams_webhook in NotifyManager but no Teams notifier method]: The `NotifyManager` stores `teams_webhook: Option<String>` but there's no `notify_teams()` method call in the dispatch methods, unlike Slack and Discord which have dedicated calls. The Teams functionality exists in `WebhookNotifier::notify_teams()` but may not be used through the manager (priority: low).
- [No timeout on webhook requests]: The `create_http_client(10)` in webhook.rs creates a client with 10 second timeout, but individual webhook requests don't have explicit timeouts and could hang.