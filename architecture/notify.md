# Notify Module

## Purpose

Event-driven notification system delivering scan lifecycle events (started, completed, findings detected, errors) to generic webhooks and platform-specific channels (Slack, Discord, Microsoft Teams). Always compiled — no feature gate.

## Location & Feature Gating

| Path | Feature Gate |
|------|-------------|
| `crates/eggsec/src/notify/mod.rs` | None (always compiled) |
| `crates/eggsec/src/notify/webhook.rs` | None (always compiled) |
| `crates/eggsec/src/commands/handlers/notify.rs` | `cli` (CLI handler) |
| `crates/eggsec/src/commands/webhook.rs` | `cli` (test/send helpers) |

Email notifications (SMTP via `lettre`) live in `crates/eggsec/src/agent/alerts/routing.rs:194` (`EmailChannel` + `send_email`), gated behind the `email-notifications` feature. This is separate from the `notify` module and integrates through the agent alert routing pipeline.

## Key Types

| Type | Location | Description |
|------|----------|-------------|
| `NotifyManager` | `notify/mod.rs:10` | Central dispatcher; holds `WebhookNotifier` + three `Option<String>` platform URLs |
| `WebhookNotifier` | `notify/webhook.rs:53` | HTTP sender with HMAC-SHA256 signing, retry logic, platform payload builders |
| `NotificationPayload` | `notify/webhook.rs:12` | Serialized body: event, timestamp, scan_id, target, message, optional findings/stats |
| `FindingSummary` | `notify/webhook.rs:24` | Finding snapshot: severity, finding_type, description, location (all `String`) |
| `ScanStats` | `notify/webhook.rs:44` | Scan statistics: duration_ms, requests_total/success/failed, findings_total |
| `NotificationConfig` | `config/scan.rs:105` | Config: webhooks vec, platform URLs, event filter, notify_on_* flags |
| `WebhookConfig` | `config/scan.rs:147` | Per-webhook: url (`String`), name, secret (`Option<SensitiveString>`), headers, events |
| `WebhookEvent` | `config/scan.rs:165` | Enum (4 variants): `ScanStarted`, `ScanComplete`, `FindingDetected`, `ScanError` |

## Architecture

### Module structure

| File | Lines | Role |
|------|-------|------|
| `notify/mod.rs` | 377 | `NotifyManager` definition, dispatch orchestration, event filtering, tests |
| `notify/webhook.rs` | 783 | `WebhookNotifier`, HMAC signing, retry logic, Slack/Discord/Teams payload builders, tests |
| `commands/handlers/notify.rs` | 101 | CLI `handle_notify` (test + send subcommands) |
| `commands/webhook.rs` | 95 | `WebhookTestConfig`, `send_webhook_notifications`, `has_any_webhook` helpers |

### NotifyManager fields (`notify/mod.rs:10-16`)

```rust
pub struct NotifyManager {
    notifier: WebhookNotifier,          // generic webhooks
    slack_webhook: Option<String>,      // plain String — see Security Considerations
    discord_webhook: Option<String>,    // plain String
    teams_webhook: Option<String>,      // plain String
    config: NotificationConfig,         // event filter + notify_on_* flags
}
```

### NotificationConfig fields (`config/scan.rs:105-129`)

| Field | Type | Default |
|-------|------|---------|
| `webhooks` | `Vec<WebhookConfig>` | `vec![]` |
| `slack_webhook` | `Option<String>` | `None` |
| `discord_webhook` | `Option<String>` | `None` |
| `teams_webhook` | `Option<String>` | `None` |
| `platform_event_filter` | `Option<Vec<WebhookEvent>>` | `None` (all events) |
| `notify_on_complete` | `bool` | `true` |
| `notify_on_findings` | `bool` | `true` |
| `notify_on_error` | `bool` | `true` |

### WebhookConfig fields (`config/scan.rs:147-161`)

| Field | Type | Notes |
|-------|------|-------|
| `url` | `String` | Must start with `http://` or `https://` (`validate()` at line 177) |
| `name` | `Option<String>` | Optional label |
| `headers` | `FxHashMap<String, String>` | Custom headers; keys must be non-empty |
| `events` | `Vec<WebhookEvent>` | Filter: only matching events delivered |
| `secret` | `Option<SensitiveString>` | HMAC-SHA256 signing secret — properly redacted |

## Behavior & Flow

### Dispatch flow (`notify/mod.rs:144-177`)

1. `NotifyManager::dispatch()` iterates `notifier.notify()` results (generic webhooks).
2. If `slack_webhook` is `Some`, calls `notifier.notify_slack()` with `platform_event_filter`.
3. If `discord_webhook` is `Some`, calls `notifier.notify_discord()` with `platform_event_filter`.
4. If `teams_webhook` is `Some`, calls `notifier.notify_teams()` with `platform_event_filter`.
5. All failures are fire-and-forget: logged via `tracing::warn!`, never propagated.

### Event filtering

- **Generic webhooks** (`webhook.rs:72-75`): Each `WebhookConfig.events` is checked; payload skipped if event not in list.
- **Platform notifiers** (`webhook.rs:133-137`, `149-153`, `169-173`): Filtered by `platform_event_filter` from `NotificationConfig`. `None` = all events delivered.
- **`notify_scan_complete` suppression** (`mod.rs:87-91`): Skipped when `notify_on_complete == false` AND (no findings OR `notify_on_findings == false`). Prevents duplicate scan-complete when findings are reported separately via `notify_findings`.

### Lifecycle hook points

| Method | When Called | Event |
|--------|-----------|-------|
| `notify_scan_started()` | Pre-scan | `WebhookEvent::ScanStarted` |
| `notify_scan_complete()` | Post-scan (with optional findings + stats) | `WebhookEvent::ScanComplete` |
| `notify_findings()` | After findings collected | `WebhookEvent::FindingDetected` |
| `notify_error()` | On scan failure | `WebhookEvent::ScanError` |

### Platform payload shapes

**Slack** (`webhook.rs:228-275`): Single-attachment format.
```json
{ "attachments": [{ "color": "#36a64f", "title": "Eggsec - ScanComplete",
    "fields": [{ "title": "Target", "value": "...", "short": true },
               { "title": "Scan ID", "value": "...", "short": true },
               { "title": "Findings", "value": "[high] xss - XSS", "short": false }],
    "footer": "Eggsec Security Scanner", "ts": <unix_ts> }] }
```
Color: `#36a64f` (green) for started/complete, `#dc3545` (red) for error, `#ffc107` (amber) for findings.

**Discord** (`webhook.rs:277-319`): Embed format.
```json
{ "embeds": [{ "title": "Eggsec - ScanComplete", "description": "...",
    "color": 0x36a64f, "fields": [{ "name": "Target", "value": "...", "inline": true },
    { "name": "Scan ID", "value": "...", "inline": true },
    { "name": "Statistics", "value": "Duration: 2000ms\nRequests: 48/50\nFindings: 1", "inline": false }],
    "footer": { "text": "Eggsec Security Scanner" }, "timestamp": "<rfc3339>" }] }
```
Color uses hex integer format. Statistics field included only when `stats` is present.

**Teams** (`webhook.rs:321-373`): MessageCard format.
```json
{ "@type": "MessageCard", "@context": "http://schema.org/extensions",
  "themeColor": "36a64f", "summary": "Eggsec - ScanComplete",
  "sections": [{ "activityTitle": "Eggsec - ScanComplete",
    "facts": [{ "name": "Target", "value": "..." }, { "name": "Scan ID", "value": "..." },
              { "name": "Event", "value": "ScanComplete" },
              { "name": "Statistics", "value": "Duration: 2000ms\nRequests: 48/50" },
              { "name": "Findings", "value": "[high] xss - XSS" }],
    "markdown": true }],
  "potentialAction": [{ "@type": "OpenUri", "name": "View Scan",
    "targets": [{ "os": "default", "uri": "<target>" }] }] }
```
Color uses hex string (no `0x` prefix). `potentialAction` links to target URL.

### HMAC-SHA256 signing (`webhook.rs:87-109`)

When `WebhookConfig.secret` is `Some`:
1. Canonical JSON of `NotificationPayload` is serialized.
2. HMAC-SHA256 computed with the secret.
3. Signature set as `X-Signature-256: sha256=<hex>` header.
4. Custom headers from `WebhookConfig.headers` are merged after the signature.

### Retry logic (`webhook.rs:180-226`)

Shared across all paths (generic, Slack, Discord, Teams):
- **Max 3 attempts** (`MAX_RETRIES = 3`, line 187).
- **Exponential backoff**: delays of 0ms, 1000ms, 2000ms (`BASE_DELAY_MS = 1000`, line 188).
- **Success check**: `response.status().is_success()` (line 205).
- **Retried on**: non-success HTTP status, network errors.
- **Not retried on**: serialization errors, HMAC key errors (immediate failure).

## Credential Handling

| Credential | Type | Location | Redaction |
|-----------|------|----------|-----------|
| Generic webhook `url` | `String` | `WebhookConfig.url` (`config/scan.rs:148`) | **Not redacted** — plain `String` |
| Generic webhook `secret` | `SensitiveString` | `WebhookConfig.secret` (`config/scan.rs:160`) | Properly redacted |
| Slack webhook URL | `String` | `NotificationConfig.slack_webhook` (`config/scan.rs:110`) | **Not redacted** |
| Discord webhook URL | `String` | `NotificationConfig.discord_webhook` (`config/scan.rs:112`) | **Not redacted** |
| Teams webhook URL | `String` | `NotificationConfig.teams_webhook` (`config/scan.rs:116`) | **Not redacted** |

**Design note**: Webhook URLs (Slack/Discord/Teams) are plain `String` rather than `SensitiveString`. These are not API tokens but webhook endpoint URLs, which are semi-public by nature (Slack/Discord webhook URLs grant posting access). The `secret` field for HMAC signing is properly protected. This is a reasonable design choice but worth noting: if webhook URLs are treated as confidential, they should migrate to `SensitiveString`.

## Public API

| Method | Signature | Description |
|--------|-----------|-------------|
| `NotifyManager::new` | `(NotificationConfig) -> Self` | Construct from config |
| `NotifyManager::from_settings` | `(&EggsecConfig) -> Self` | Construct from full config (`mod.rs:37`) |
| `NotifyManager::notify_scan_started` | `async (&self, scan_id, target)` | Fire `ScanStarted` event |
| `NotifyManager::notify_scan_complete` | `async (&self, scan_id, target, message, findings?, stats?)` | Fire `ScanComplete` with optional data |
| `NotifyManager::notify_findings` | `async (&self, scan_id, target, Vec<FindingSummary>)` | Fire `FindingDetected` event |
| `NotifyManager::notify_error` | `async (&self, scan_id, target, error)` | Fire `ScanError` event |
| `NotifyManager::is_enabled` | `(&self) -> bool` | True if any webhook or platform URL is configured |
| `WebhookNotifier::new` | `(Vec<WebhookConfig>) -> Result<Self>` | Construct with reqwest client (10s timeout) |
| `WebhookNotifier::notify` | `async (&self, &NotificationPayload) -> Vec<Result<(), String>>` | Dispatch to all matching generic webhooks |
| `WebhookNotifier::notify_slack` | `async (&self, url, payload, filter?) -> Result<(), String>` | Send to Slack |
| `WebhookNotifier::notify_discord` | `async (&self, url, payload, filter?) -> Result<(), String>` | Send to Discord |
| `WebhookNotifier::notify_teams` | `async (&self, url, payload, filter?) -> Result<(), String>` | Send to Teams |

## Integration Points

### CLI

- `CommandContext::new()` creates `NotifyManager::from_settings()` (`commands/handlers/mod.rs:125`).
- `handle_notify` (`commands/handlers/notify.rs:4`) dispatches `NotifyCommand::Test` and `NotifyCommand::Send` subcommands.
- `commands/webhook.rs` provides `send_webhook_notifications()` for ad-hoc test sends.

### Command handlers

Every scan handler in `commands/handlers/` calls the notify lifecycle methods via `ctx.notify_manager`:
- `scan.rs` (lines 16-188): Port scan, endpoint scan, fingerprint scan, NSE scan, full scan, resume scan.
- `wireless.rs` (lines 44-176): Wireless scan, deauth attack.
- `stress.rs` (lines 53-79): Stress test.
- `mobile.rs` (lines 77-188): Mobile APK/IPA analysis, dynamic scan.
- `hunt.rs` (lines 24-67): Hunt scan.
- `browser.rs` (lines 32-120): Browser scan.
- `fuzz.rs` (lines 23-107): Fuzz scan, WAF stress test.

### TUI

TUI Settings tab exposes notification configuration (`tabs/settings/main.rs`):
- `notify_inputs: InputGroup` (line 25): Webhook URL fields.
- `notify_on_complete: Checkbox` (line 30): Toggle scan-complete notifications.
- `notify_on_findings: Checkbox` (line 31): Toggle findings notifications.
- Config read/write at lines 411-412, 506-507.

### Output

`NotificationPayload` serializes to JSON for all webhook delivery. No direct integration with `eggsec-output` report formats.

## Testing

- **Unit tests** (`notify/mod.rs:193-377`): 9 tests covering defaults, `is_enabled`, serialization, scan-complete suppression logic.
- **Unit tests** (`notify/webhook.rs:376-783`): 17 tests covering event serialization, payload serialization, finding summary, scan stats, webhook config, notifier enablement, platform payload builders (Slack/Discord/Teams), HMAC signature generation, event filtering, platform event filtering.
- **Total**: 26 tests.

## Invariants & Gotchas

1. **Fire-and-forget**: Notification failures never abort scans. Errors are logged via `tracing::warn!` only (`mod.rs:148-149`).
2. **Scan-complete suppression**: `notify_scan_complete` is suppressed when `notify_on_complete == false` AND (no findings OR `notify_on_findings == false`) (`mod.rs:87-91`). This prevents duplicate notifications when findings are sent separately.
3. **Platform event filter**: When `platform_event_filter` is `None`, all events pass. When set, only matching events are delivered to Slack/Discord/Teams.
4. **Webhook event filter**: Per-webhook `events` field is checked before delivery. Empty events vec means no events are delivered.
5. **HTTP client**: Created via `create_http_client(10)` with 10-second timeout (`webhook.rs:60`).
6. **HMAC canonical form**: Signature is computed over `serde_json::to_string(&payload_value)` where `payload_value` is the `serde_json::Value` — not the original struct. This means field ordering follows serde's JSON serialization.
7. **Teams `potentialAction`**: The `uri` field in `potentialAction` is set to `payload.target` — if the target is not a valid URL (e.g., an IP address or hostname), the Teams card action link will be broken (`webhook.rs:369`).

## Security Considerations

- **Webhook URLs are plain `String`**: Slack/Discord/Teams webhook URLs and generic webhook URLs are not wrapped in `SensitiveString`. These URLs grant posting access to the respective channels. If webhook URLs are treated as confidential, they should be migrated to `SensitiveString` for consistency with `WebhookConfig.secret`.
- **SSRF consideration**: Webhook URLs are user-supplied via config. The `WebhookConfig::validate()` method (`config/scan.rs:177`) only checks URL scheme (`http://` or `https://`). It does not restrict targets to external addresses, so internal network addresses (localhost, private ranges) could be targeted. This is a design consideration for environments where outbound network access must be restricted.
- **HMAC secret exposure**: The `secret` field is `SensitiveString` and accessed only via `expose_secret()` at the point of use (`webhook.rs:89`). Properly protected.
- **Error messages include URL**: `tracing::warn!` in `send_with_retry` logs the platform name and error status but not the URL itself (`webhook.rs:216-222`). However, the `webhook.rs:208` error message includes the platform name and status — not the full URL. The `commands/webhook.rs:22-23` prints the URL to stdout (`println!("Sending test to Slack: {}", slack_url)`).

## Bug Sweep

| Finding | Location | Severity | Description |
|---------|----------|----------|-------------|
| Webhook URLs logged to stdout | `commands/webhook.rs:22,32,42,52` | Low | `println!("Sending test to Slack: {}", slack_url)` prints webhook URLs to terminal. In shared environments, this could expose webhook endpoints. |
| SSRF no address restriction | `config/scan.rs:177-187` | Design | `WebhookConfig::validate()` allows any `http://`/`https://` URL including internal addresses. Consider adding `classify_address()` check for strict environments. |
| Teams `potentialAction` URI | `webhook.rs:369` | Low | `payload.target` is used as the URI. If target is not a valid URL, the card action link is broken. |

*Last verified against source: 2026-08-25*
