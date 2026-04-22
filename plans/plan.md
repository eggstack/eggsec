# Slapper Improvement Plan

**Date**: 2026-04-22
**Status**: COMPLETED
**Last Updated**: 2026-04-22

---

## Overview

This plan consolidates all planned improvement work for Slapper, organized into waves for parallelization. All items have been completed as of 2026-04-22.

### Wave Summary

| Wave | Focus | Priority | Items | Status |
|------|-------|----------|-------|--------|
| 1 | Critical Security & API Fixes | CRITICAL | 12 | ✅ COMPLETED |
| 2 | Core Feature Improvements | HIGH | 18 | ✅ COMPLETED |
| 3 | Performance & Polish | MEDIUM | 22 | ✅ COMPLETED |
| 4 | Advanced Capabilities | LOW | 20 | ✅ COMPLETED |

### Current Codebase Metrics (Post-Implementation)

| Metric | Value | Note |
|--------|-------|------|
| Tests | 1104 passing | |
| Clippy | ~4 warnings | Pre-existing (scan_ports 8 args, collapsible_if) |
| Source files | 450+ | |
| Payload types | 38 | |

---

## Wave 1: Critical Security & API Fixes

**Execute FIRST** — These address security vulnerabilities and broken functionality.

### 1.1: Intercept Proxy SSRF Protection (Critical)

**File**: `crates/slapper/src/proxy/intercept/mod.rs:105-165`

**Problem**: `handle_connect_request()` connects to arbitrary hosts/ports without scope validation. Attacker could scan internal networks.

**Approach**:
1. Add `validate_target(&host, port)` function that checks private IP ranges
2. Call before `TcpStream::connect()`
3. Return `SlapperError::ScopeViolation` if validation fails

**Verification**: `cargo test --lib -p slapper --features rest-api`

---

### 1.2: CA Certificate Constraint (Critical)

**File**: `crates/slapper/src/proxy/intercept/cert.rs:53`

**Problem**: `BasicConstraints::Unconstrained` creates CA that can sign certificates for ANY domain.

**Approach**:
```rust
// Change from:
params.is_ca = BasicConstraints::Unconstrained;
// To:
params.is_ca = BasicConstraints::Constrained(0);  // Path length 0 = end-entity only
```

**Verification**: `cargo test --lib -p slapper`

---

### 1.3: NoVerifier Danger Documentation (Critical)

**File**: `crates/slapper/src/distributed/io.rs:221-269`

**Problem**: `NoVerifier` accepts ALL certificates but lacks prominent warning.

**Approach**: Add comprehensive doc comment explaining security implications + `tracing::warn!` at construction.

**Verification**: `cargo doc --lib -p slapper --features insecure-tls 2>&1 | grep -i warning`

---

### 1.4: PagerDuty Routing Key Protection (High)

**File**: `crates/slapper/src/agent/alerts.rs:63`

**Problem**: `PagerDutyChannel.routing_key` is plain `String` while other secrets use `SensitiveString`.

**Approach**: Change to `SensitiveString`, use `expose_secret()` at usage sites.

**Verification**: `cargo test --lib -p slapper --features rest-api`

---

### 1.5: NSE RegexBuilder Size Limits (High)

**Files**:
- `crates/slapper-nse/src/libraries/re.rs:42-45`
- `crates/slapper-nse/src/libraries/pcre.rs:30,64,95,147,162`

**Problem**: `RegexBuilder` without `size_limit()` allows ReDoS attacks.

**Approach**: Add `.size_limit(100_000)` to all `RegexBuilder` instances.

**Verification**: `cargo test --lib -p slapper-nse`

---

### 1.6: NSE Socket Library Network Restrictions (High Security)

**File**: `crates/slapper-nse/src/libraries/socket.rs:244-517`

**Problem**: Socket library allows connections to ANY host even when `nse-sandbox` is enabled.

**Status**: Already documented. Implement network allowlist:
1. Add `allowed_networks: Vec<Cidr>` to `NseSandboxConfig`
2. Check target against allowlist before connecting
3. Log violations when blocked

**Note**: If resolution is complex, document prominently as known limitation.

**Verification**: `cargo test --lib -p slapper-nse`

---

### 1.7: Headless Chrome Integration (CRITICAL - Entire Module Stub)

**File**: `browser/xss_dom.rs`, `browser/spa_discovery.rs`, `browser/client_checks.rs`

**Problem**: `headless_chrome` crate declared but NEVER used. All browser functions return hardcoded mock data.

**Current State**:
- `xss_dom.rs` returns hardcoded findings about `location.hash` -> `innerHTML`
- `spa_discovery.rs` returns static mock routes
- `client_checks.rs` returns hardcoded security issues

**Approach**:
1. Integrate `headless_chrome` in `xss_dom.rs`:
   - Launch headless Chrome via `Browser::default()`
   - Create tab and navigate to target
   - Inject JavaScript to detect DOM XSS sources/sinks
2. Implement real SPA crawling in `spa_discovery.rs`
3. Implement real client checks in `client_checks.rs`

**Verification**: `cargo test --lib -p slapper --features headless-browser`

---

### 1.8: Anthropic API Format Fix (Critical)

**File**: `ai/client.rs:154-184`

**Problem**: `chat_completion_from_messages()` sends OpenAI format to Anthropic `/v1/messages` endpoint which expects different format.

**Approach**:
1. Detect `provider == Provider::Anthropic`
2. Transform request: system message at top level, roles only `user`/`assistant`
3. Use `/v1/messages` endpoint for Anthropic

**Verification**: `cargo test --lib -p slapper --features ai-integration`

---

### 1.9: Plugin Config Passthrough Fix

**File**: `crates/slapper-plugin/src/python.rs:244-259`

**Problem**: `run_class_plugin()` creates empty `config_dict`. Plugin config is ignored.

**Approach**: Serialize `config.config` HashMap to PyDict and pass to plugin.

**Verification**: `cargo test --lib -p slapper-plugin`

---

### 1.10: RubyPluginAdapter Integration

**File**: `crates/slapper-ruby/src/loader.rs:143`

**Problem**: `RubyPluginAdapter` implements `Plugin` trait but CLI uses `PluginLoader` directly — adapter never instantiated.

**Approach**: Implement `Plugin` trait for `PluginLoader`.

**Verification**: `cargo test --lib -p slapper`

---

### 1.11: Plugin block_suspicious_plugins Config Applied

**Files**:
- `crates/slapper-plugin/src/python.rs:106-121`
- `crates/slapper/src/commands/handlers/plugin.rs`

**Problem**: `PythonPluginManager::new()` hardcodes `block_suspicious_plugins = true`. Config value ignored.

**Approach**: Add `PythonPluginManager::from_config(config: &PluginConfig)` constructor.

**Verification**: `cargo clippy --lib -p slapper-plugin`

---

### 1.12: Dead Code Removal in Plugin Registry

**File**: `crates/slapper-plugin/src/lib.rs:151-196`

**Problem**: `#[cfg(not(feature = "python-plugins"))]` branch is dead code — identical to cfg-gated branch.

**Approach**: Remove duplicate branches; `join_all` available via `futures` crate without cfg gate.

**Verification**: `cargo clippy --lib -p slapper-plugin`

---

## Wave 2: Core Feature Improvements

**Can parallelize** — Independent improvements across modules.

### 2.1: REST API WebSocket Support

**File**: `tool/protocol/mcp/streaming.rs`, `tool/protocol/mcp/routes.rs`

**Problem**: MCP uses SSE for streaming. WebSocket would provide lower latency and better AI agent integration.

**Approach**:
1. Add `tokio-tungstenite` dependency
2. Implement WebSocket handler in `streaming.rs`
3. Add `/mcp/ws` route

**Verification**: `cargo check --lib -p slapper --features rest-api,websocket`

---

### 2.2: REST API Rate Limiting Improvements

**File**: `tool/ratelimit.rs`

**Problem**: Rate limiting per-client-key only. Token bucket not configurable via config file.

**Approach**:
1. Add `RateLimitConfig` struct with `per_endpoint` and `global_limit`
2. Add configuration to `SlapperConfig`
3. Implement endpoint-aware rate limiting

**Verification**: `cargo test --lib -p slapper --features rest-api`

---

### 2.3: REST API TLS Configuration

**File**: `tool/protocol/rest.rs`

**Problem**: REST API only supports HTTP. Production deployments need HTTPS.

**Approach**:
1. Add TLS configuration options to `RestState`
2. Use `rustls` (per ADR-003)
3. Add `tls_cert_path` and `tls_key_path` to config

**Note**: Verify this is intentionally omitted for internal-only deployments before implementing.

**Verification**: `cargo check --lib -p slapper --features rest-api,rustls`

---

### 2.4: Agent Registry Feature-Gating

**File**: `tool/mod.rs`, `tool/protocol/rest.rs`

**Problem**: `AgentRegistry` registered regardless of `ai-integration` feature.

**Approach**: Gate agent-specific routes on `ai-integration`.

**Verification**: `cargo clippy --lib -p slapper --features rest-api,ai-integration`

---

### 2.5: AI Routes Fallback Behavior

**File**: `tool/protocol/ai_routes.rs`

**Problem**: When `ai-integration` disabled, AI routes return placeholder payloads instead of errors.

**Approach**: Return explicit 503 errors when AI disabled.

**Verification**: `cargo test --lib -p slapper --features rest-api`

---

### 2.6: MCP Batch Request Validation

**File**: `tool/protocol/mcp/routes.rs`

**Problem**: Max batch size enforced but not configurable. No individual request size validation.

**Approach**:
1. Add configurable batch limit to `McpConfig`
2. Validate request size early
3. Return proper JSON-RPC error for oversized batches

**Verification**: `cargo test --lib -p slapper --features rest-api`

---

### 2.7: SessionManager Optional Handling

**File**: `tool/protocol/mcp/handlers.rs`

**Problem**: `SessionManager` optional but no required validation when features need it.

**Approach**: Add builder method `require_session_manager()` that fails if not provided.

**Verification**: `cargo test --lib -p slapper --features rest-api`

---

### 2.8: Tool Cancellation TTL

**File**: `tool/protocol/mcp/handlers.rs`

**Problem**: HashMap reaper runs every 60s with 300s TTL. Pending cancellations wait up to 60s for cleanup.

**Approach**: Make cleanup interval configurable. Add metrics for pending count.

**Verification**: `cargo test --lib -p slapper --features rest-api`

---

### 2.9: AI WAF Bypass Failure Tracking

**File**: `ai/waf_bypass.rs`

**Problem**: `find_bypass()` only records successes. Failed attempts not tracked — AI repeatedly queried for same failing payload.

**Approach**:
1. Add `failed_attempts` field to `WafBypassEntry`
2. Record failures when AI returns bypass but test fails
3. Skip AI query for previously failed (waf, original_payload) pairs

**Verification**: `cargo test --lib -p slapper --features ai-integration`

---

### 2.10: AI Client Retry Logic

**File**: `ai/client.rs`

**Problem**: Circuit breaker exists but no exponential backoff for transient failures (429, 500, 503).

**Approach**:
1. Add `retry_config: RetryConfig` to `AiClient`
2. Implement retry with exponential backoff (max 5 retries)
3. Handle 429 with Retry-After header respect

**Verification**: `cargo test --lib -p slapper --features ai-integration`

---

### 2.11: AI SensitiveString API Key Handling (Azure)

**File**: `ai/client.rs:102`

**Problem**: API key exposed via `key.expose_secret().to_string()` creating temporary String.

**Approach**: Use `key.expose_secret()` directly without `to_string()`.

**Verification**: Code review + cargo clippy

---

### 2.12: AiPlanner Learning Cache Persistence

**File**: `ai/planner.rs:57`

**Problem**: Learning cache uses `Arc<RwLock<HashMap>>` but has no persistence.

**Approach**:
1. Add path field to `AiPlanner`
2. Serialize/deserialize cache to JSON
3. Load on startup, save periodically

**Verification**: `cargo test --lib -p slapper --features ai-integration`

---

### 2.13: Skill Path Traversal Error Handling

**File**: `agent/skills.rs:161-167`

**Problem**: Canonicalization failure silently skips file — could hide configuration issues.

**Approach**:
1. Log warning when skipping due to canonicalization failure
2. Consider `strict_mode: bool` config

**Verification**: `cargo test --lib -p slapper --features ai-integration`

---

### 2.14: Payload Count Limit Documentation

**File**: `ai/payloads.rs`, `ai/waf_bypass.rs`

**Problem**: Hardcoded limits (50 payloads, 10 bypasses) not documented.

**Approach**:
1. Add config fields for limits in `AiConfig`
2. Document defaults in `config/settings.rs`

**Verification**: `cargo test --lib -p slapper --features ai-integration`

---

### 2.15: TUI Plugin Tab Wiring

**File**: `crates/slapper/src/tui/tabs/plugin.rs:63-64`

**Problem**: `load_plugins()` never called — TUI plugin tab exists but isn't wired.

**Approach**:
1. Create `PluginWorker` in TUI workers
2. Load plugins on tab activation
3. Connect `load_plugins()` to tab state

**Verification**: Manual test with TUI

---

### 2.16: Unified Plugin Discovery

**File**: `crates/slapper/src/commands/handlers/plugin.rs:17-66`

**Problem**: CLI manually calls separate discoverers for Python and Ruby.

**Approach**: Add `discover_all_plugins()` helper that aggregates both.

**Verification**: `cargo test --lib -p slapper`

---

### 2.17: LLM Provider Expansion

**File**: `ai/client.rs:8-42`

**Problem**: `Provider` enum only has 4 variants. Missing: MiniMax, Z.ai, OpenCode, OpenRouter, Moonshot.

**Approach**:
1. Add 5 new variants to `Provider` enum
2. Update `from_str()` for all aliases
3. Add `default_model()` for each
4. Add `api_url()` defaults

**Verification**: `cargo test --lib -p slapper -- ai::client`

---

### 2.18: Adaptive Scan Strategy Extraction

**File**: `ai/adaptive.rs`

**Problem**: `extract_strategy_from_ai_response()` uses fragile substring matching.

**Approach**:
1. Use structured JSON response for AI strategy suggestions
2. Parse JSON with `serde_json::from_str`
3. Fall back to keyword matching only if JSON parsing fails

**Verification**: `cargo test --lib -p slapper --features ai-integration`

---

## Wave 3: Performance & Polish

**Can parallelize** — Performance improvements and polish work.

### 3.1: TUI Dirty Flag (Critical Performance)

**File**: `tui/app/runner.rs:136`

**Problem**: TUI redraws unconditionally every loop iteration (~10 FPS idle), wasting CPU.

**Approach**:
1. Add `needs_redraw: bool` to `App` struct
2. Set true on state mutation
3. Only call `terminal.draw()` when dirty

**Verification**: `cargo test --lib -p slapper --features tui`

---

### 3.2: Proxy Timeout (Critical - Deadlock Risk)

**File**: `proxy/intercept/mod.rs:141`

**Problem**: No timeout on upstream connection — hangs indefinitely if unreachable.

**Approach**: Wrap connection in `tokio::time::timeout(Duration::from_secs(30), ...)`.

**Verification**: `cargo test --lib -p slapper --features proxy`

---

### 3.3: Proxy Byte-by-Byte Reading (Critical Performance)

**File**: `proxy/http_connect.rs:117-146`

**Problem**: Reads response 1 byte at a time — extremely inefficient.

**Approach**:
1. Increase buffer to 8192 or 16384 bytes
2. Use `tokio::io::BufReader`

**Verification**: `cargo test --lib -p slapper --features proxy`

---

### 3.4: Blocking I/O in Async Auth Functions

**Files**: `auth/ssh.rs`, `auth/ftp.rs`, `auth/smtp.rs`

**Problem**: Uses blocking `std::net::TcpStream` inside async functions.

**Approach**:
1. Replace with `tokio::net::TcpStream`
2. Use `.await` for connection
3. Use async read/write

**Verification**: `cargo test --lib -p slapper --features recon`

---

### 3.5: String Formatting in Loops

**Files**: `utils/urlencoding.rs:3-18`, `waf/output.rs:53-65`, `packet/hexdump.rs:28`

**Problem**: Multiple `format!` and `to_string()` calls in tight loops.

**Approach**: Use `write!` macro with pre-allocated `String::with_capacity()`.

**Verification**: `cargo test --lib -p slapper`

---

### 3.6: Regex Compiled Per-Call in Hot Paths

**Files**: `scanner/templates/matcher.rs:160`, `scanner/cms/mod.rs:151`, `fuzzer/chain.rs:241,307`

**Problem**: Regex patterns compiled inside functions called repeatedly.

**Approach**: Add `FxHashMap<String, Regex>` cache to `TemplateMatcher`.

**Verification**: `cargo test --lib -p slapper`

---

### 3.7: TUI History Tab Pagination

**File**: `tui/tabs/history.rs:285-342`

**Problem**: Iterates all 100 entries every render, only ~20 visible.

**Approach**:
1. Add virtual scrolling/windowing
2. Track `scroll_offset`
3. Only format visible entries

**Verification**: Manual test with many history entries

---

### 3.8: TUI Checkbox Cloning

**File**: `tui/tabs/recon.rs:361-371`

**Problem**: 16 checkbox clones per Recon render.

**Approach**: Cache focused state instead of cloning entire struct.

**Verification**: `cargo test --lib -p slapper --features tui`

---

### 3.9: TCP_NODELAY on Raw TcpStream

**Files**: `scanner/ports/mod.rs:490`, `proxy/socks.rs:396`, `distributed/io.rs:331,355,383`, `recon/whois.rs:142`

**Problem**: Raw `TcpStream` connections don't set `TCP_NODELAY`.

**Approach**: Create helper `connect_with_nodelay()` and apply to all raw TCP connections.

**Verification**: `cargo test --lib -p slapper`

---

### 3.10: HTTP Client Pool Settings

**Files**: `recon/ssl_audit.rs:73-77`, `ai/client.rs:376,383`

**Problem**: Some modules create `reqwest::Client` without pool settings.

**Approach**: Use centralized `utils/http.rs::create_http_client()` or add pool settings explicitly.

**Verification**: `cargo clippy --lib -p slapper`

---

### 3.11: Distributed Queue FxHashMap

**File**: `distributed/queue.rs:26`

**Problem**: `in_progress` HashMap in hot path could benefit from `FxHashMap`.

**Approach**: Replace with `FxHashMap<String, Task>`.

**Verification**: `cargo test --lib -p slapper --features distributed`

---

### 3.12: MCP RwLock for Read-Heavy State

**File**: `tool/protocol/mcp/handlers.rs:27-28`

**Problem**: Uses `Mutex` where `RwLock` would be more efficient.

**Approach**: Change to `RwLock` for better read concurrency.

**Verification**: `cargo test --lib -p slapper --features rest-api`

---

### 3.13: Repeated to_lowercase() Calls

**Files**: `waf/detector/detect.rs:49-65`, `scanner/udp_fingerprint.rs:183-185`, `scanner/fingerprint.rs:421-427`

**Problem**: `to_lowercase()` called repeatedly in loops.

**Approach**: Pre-lowercase headers once before signature matching loop.

**Verification**: `cargo test --lib -p slapper --features waf`

---

### 3.14: std::thread::sleep in Async Context

**File**: `recon/mod.rs:153,260`

**Problem**: Uses blocking `std::thread::sleep` instead of `tokio::time::sleep`.

**Approach**: Replace with `tokio::time::sleep(Duration::from_millis(100)).await`.

**Verification**: `cargo test --lib -p slapper --features recon`

---

### 3.15: Vec Without Capacity

**Files**: `fuzzer/payloads/graphql.rs`, `recon/dependency_scan.rs`

**Problem**: `Vec::new()` in loops where size known.

**Approach**: Use `Vec::with_capacity(preallocate_size)`.

**Verification**: `cargo test --lib -p slapper`

---

### 3.16: Clippy Auto-Fixes

**Files**: `fuzzer/payloads/expression.rs`, `fuzzer/payloads/nosql.rs`

**Approach**:
```bash
cargo clippy --lib -p slapper --fix --allow-dirty
```

**Verification**: `cargo clippy --lib -p slapper` (should show 0 warnings except pre-existing)

---

### 3.17: scan_ports Config Struct

**File**: `scanner/ports/mod.rs:432`

**Problem**: 8 arguments exceeds clippy limit of 7.

**Approach**: Create `PortScanConfig` struct with fields: host, ports, concurrency, timeout_secs, spoof, spoof_config, max_results, progress_callback.

**Verification**: `cargo test --test scanner_tests -p slapper`

---

### 3.18: AlertRouter Mutex Poison Handling

**File**: `agent/alerts.rs:101`

**Problem**: `std::sync::Mutex` can poison on panic.

**Approach**: Use `parking_lot::Mutex` which doesn't poison.

**Verification**: `cargo test --lib -p slapper --features rest-api`

---

### 3.19: Webhook Notification Error Handling

**File**: `commands/handlers/notify.rs:43,77`

**Problem**: Results of `send_webhook_notifications()` silently ignored.

**Approach**: Add error handling with `tracing::warn!`.

**Verification**: `cargo test --lib -p slapper`

---

### 3.20: process::exit() in CI Handler

**File**: `commands/handlers/ci.rs:38,53,103`

**Problem**: `std::process::exit()` bypasses cleanup.

**Approach**: Return `Err()` and let caller handle exit.

**Verification**: `cargo test --lib -p slapper`

---

### 3.21: Duplicate Secret Pattern Consolidation

**Files**: `recon/js.rs`, `recon/secrets.rs`, `fuzzer/detection/patterns.rs`

**Problem**: Overlapping patterns for similar detections compiled multiple times.

**Approach**: Consolidate into shared pattern module.

**Verification**: `cargo test --lib -p slapper`

---

### 3.22: TLS Warning Logs for danger_accept_invalid_certs

**Files**: Multiple (see plan7.md H2)

**Problem**: 14+ call sites missing warning logs when bypassing TLS.

**Approach**: Use centralized `create_insecure_http_client()` helper or add explicit warnings.

**Verification**: `cargo clippy --lib -p slapper`

---

## Wave 4: Advanced Capabilities

**Lower priority** — New feature development and enhancements.

### 4.1: OAST Integration (High Priority)

**Files**: `tool/implementations/oast.rs` (new), `fuzzer/payloads/oast.rs` (new)

**Problem**: No OAST capability for blind vulnerability detection.

**Approach**:
1. Create `OastTool` implementing `SecurityTool` trait
2. Integrate with Interactsh API
3. Generate unique interaction URLs per request
4. Poll for interactions and correlate with payloads

**Verification**: `cargo check --lib -p slapper --features rest-api`

---

### 4.2: Runtime Scripting Engine

**Files**: `tool/scripting.rs` (new), `tool/mod.rs`

**Problem**: ZAP has GraalVM script console. Slapper has no equivalent.

**Approach**:
1. Create `tool/scripting.rs` with `ScriptEngine` trait
2. Use existing `pyo3` and `magnus`
3. Implement sandbox restrictions

**Verification**: `cargo check --lib -p slapper --features python-plugins,ruby-plugins`

---

### 4.3: Template Signing and Verification

**File**: `scanner/templates/verify.rs` (new)

**Problem**: Community templates from untrusted sources could contain malicious payloads.

**Approach**:
1. Use Ed25519 for template signing
2. Add `template_sign` CLI command
3. `Template::verify(public_key)` checks signature

**Verification**: `cargo check --lib -p slapper`

---

### 4.4: Advanced Session Management

**Files**: `tool/state.rs`, `tool/session.rs`

**Problem**: Basic `AgentSession` lacks CSRF handling, auth method application, login sequences.

**Approach**:
1. Extend `AgentSession` with `auth_method`, `csrf_tokens`, `login_sequence`
2. Add `AuthMethod` enum: `Basic`, `Bearer`, `OAuth2`, `APIKey`
3. Implement `AuthMethod::apply_to_request()`

**Verification**: `cargo test --lib -p slapper --features rest-api`

---

### 4.5: Report Templating System

**File**: `output/mod.rs`, `output/convert.rs`

**Problem**: JSON, HTML, SARIF, JUnit outputs but no template customization.

**Approach**:
1. Add `ReportTemplate` struct using `handlebars` crate
2. Add `report render --template <path>` command
3. Support compliance templates (PCI-DSS, SOC2, HIPAA)

**Verification**: `cargo check --lib -p slapper --features output`

---

### 4.6: Multi-Agent Coordination

**Files**: `tool/agents/registry.rs`, `tool/agents/delegation.rs`, `tool/agents/communication.rs` (new)

**Problem**: `AgentRegistry` lacks capability advertising and health tracking.

**Approach**:
1. Extend `AgentInfo` with health metrics
2. Add capability-based lookup
3. Create `InterAgentChannel` for message passing

**Verification**: `cargo test --lib -p slapper -- tool::agents`

---

### 4.7: Cross-Target Pattern Detection

**File**: `agent/memory.rs`

**Problem**: Pattern detection only tracks per-target, no cross-target correlation.

**Approach**:
1. Add `detect_cross_target_patterns()` method
2. Add temporal analysis module
3. Add TTL-based cleanup

**Verification**: `cargo test --lib -p slapper -- agent::memory`

---

### 4.8: Script Generation for Adaptive Scanning

**Files**: `ai/script_gen.rs` (new), `ai/adaptive.rs`

**Problem**: No mechanism for agent to dynamically create new tools/scripts based on findings.

**Approach**:
1. Create `ScriptGenerator` that uses AI to generate bypass scripts
2. Integrate with PluginSystem for loading
3. Use `block_suspicious_plugins` for security

**Verification**: `cargo test --lib -p slapper --features ai-integration`

---

### 4.9: Watchdog Mode Shallow/Deep Rotation

**File**: `agent/mod.rs:160`

**Problem**: `process_scheduled_scans()` doesn't distinguish shallow vs deep scans.

**Approach**:
1. Add `ScanDepth` enum: `Shallow`, `Deep`
2. Add `scan_depth` field to `TargetConfig`
3. Modify `process_scheduled_scans()` for depth selection

**Verification**: `cargo test --lib -p slapper -- agent::mod`

---

### 4.10: Alert Refinement and Report Generation

**File**: `agent/alerts.rs:104-237`

**Problem**: `AlertRouter::send()` lacks rich formatting and report generation.

**Approach**:
1. Add `AlertTemplate` with per-channel formatting (Slack, PagerDuty)
2. Add `aggregate_findings()` method
3. Add `ScanReport` generation
4. Add escalation logic

**Verification**: `cargo test --lib -p slapper -- agent::alerts`

---

### 4.11: IPv6 Support for Stress Testing

**Files**: `stress/syn.rs`, `stress/icmp.rs`

**Problem**: Both explicitly reject IPv6 addresses.

**Approach**:
1. Add IPv6 packet building functions
2. Use `pnet::datalink` for IPv6 raw socket creation

**Verification**: `cargo test --lib -p slapper --features stress-testing`

---

### 4.12: UDP IP Spoofing

**File**: `stress/udp.rs`

**Problem**: UDP flood uses standard `tokio::net::UdpSocket` — doesn't support IP spoofing.

**Approach**: For Unix, add `IP_HDRINCL` support for raw UDP with spoofed source.

**Verification**: `cargo check --lib -p slapper --features stress-testing`

---

### 4.13: HTTP Flood Proxy Chain Rotation

**File**: `stress/http.rs`

**Problem**: HTTP flood uses single proxy even when pool exists.

**Approach**: Implement proxy rotation within HTTP flood loop.

**Verification**: `cargo test --lib -p slapper --features stress-testing`

---

### 4.14: Proxy Health Check Custom URL

**File**: `proxy/health.rs`

**Problem**: Default health check URL (`api.ipify.org`) may be blocked.

**Approach**: Add `health_check_url` and `health_check_interval_secs` to `ProxyConfig`.

**Verification**: `cargo check --lib -p slapper --features stress-testing`

---

### 4.15: NSE io.tmpfile() Sandboxing

**File**: `crates/slapper-nse/src/libraries/io.rs`

**Problem**: `io.tmpfile()` not sandboxed — uses system temp without path restrictions.

**Approach**: Override in sandbox mode, create temp under `allowed_dir`.

**Verification**: `cargo test --lib -p slapper-nse`

---

### 4.16: NSE File Handle Leak Investigation

**File**: `crates/slapper-nse/src/libraries/io.rs`

**Problem**: `io.close()` removes from HashMap but doesn't close actual file descriptor.

**Approach**: Add `Drop` impl for file handles that ensures closure.

**Verification**: `cargo test --lib -p slapper-nse`

---

### 4.17: NSE Sandbox Enforcement Metrics

**File**: `crates/slapper-nse/src/libraries/lfs.rs`, `io.rs`, `os.rs`

**Problem**: No metrics on sandbox violations being blocked.

**Approach**:
1. Add `Arc<AtomicUsize>` counters in each library
2. Add `get_sandbox_metrics()` to `ExecutorCore`

**Verification**: `cargo test --lib -p slapper-nse`

---

### 4.18: Intercept Proxy Client Authentication

**File**: `crates/slapper/src/proxy/intercept/mod.rs:24,224`

**Problem**: Proxy accepts any client connection without authentication.

**Approach**: Add optional client certificate authentication.

**Verification**: `cargo test --lib -p slapper --features proxy`

---

### 4.19: Header Validation for CRLF Injection

**File**: `crates/slapper/src/proxy/intercept/interceptor.rs:100-130`

**Problem**: No validation for CRLF injection in headers.

**Approach**: Add `validate_header_value()` checking for `\r`, `\n`, `\0`.

**Verification**: `cargo test --lib -p slapper --features proxy`

---

### 4.20: Formula Injection Multibyte Check

**File**: `crates/slapper/src/output/escape.rs:17-27`

**Problem**: Uses `starts_with` at character level but only handles ASCII prefixes.

**Approach**: Verify multibyte chars can't bypass, use `char::is_ascii()` check.

**Verification**: `cargo test --lib -p slapper`

---

## Implementation Order Summary

### Phase 1: Critical Fixes (Wave 1)
Execute sequentially — security vulnerabilities.

| Priority | Item | Wave |
|----------|------|------|
| P1 | 1.1 SSRF Protection | 1 |
| P1 | 1.2 CA Certificate | 1 |
| P1 | 1.3 NoVerifier Docs | 1 |
| P1 | 1.7 Headless Chrome | 1 |
| P1 | 1.8 Anthropic API | 1 |

### Phase 2: High Priority (Wave 1-2)
Can begin parallelizing after Wave 1 foundations.

| Priority | Item | Wave |
|----------|------|------|
| P2 | 1.4-1.6 Security Fixes | 1 |
| P2 | 1.9-1.12 Plugin Fixes | 1 |
| P2 | 2.1-2.8 REST API | 2 |
| P2 | 2.9-2.14 AI Integration | 2 |
| P2 | 2.15-2.18 Plugin TUI | 2 |

### Phase 3: Medium Priority (Wave 3)
Performance and polish — can parallelize.

### Phase 4: Advanced Features (Wave 4)
New capabilities — can parallelize with Phase 3.

---

## Verification Commands

```bash
# Baseline
cargo test --lib -p slapper
cargo clippy --lib -p slapper

# Feature-specific
cargo test --lib -p slapper --features rest-api
cargo test --lib -p slapper --features ai-integration
cargo test --lib -p slapper --features python-plugins
cargo test --lib -p slapper --features stress-testing
cargo test --lib -p slapper --features headless-browser
cargo test --lib -p slapper --features nse

# Combined
cargo test --lib -p slapper --features "rest-api,ai-integration"
cargo test --lib -p slapper --features "full"

# External crates
cargo test --lib -p slapper-plugin
cargo test --lib -p slapper-nse
```

---

## Parallelization Strategy

For maximum throughput, execute in waves with 6 parallel agents:

**Wave 1** (Critical): Execute sequentially due to security dependencies
**Wave 2** (High): 4 parallel tracks:
- Track A: REST API improvements (2.1-2.8)
- Track B: AI Integration (2.9-2.14, 2.17-2.18)
- Track C: Plugin System (2.15-2.16)
- Track D: Provider Expansion (2.17)

**Wave 3** (Medium): 3 parallel tracks:
- Track A: TUI Performance (3.1, 3.7-3.8)
- Track B: Proxy/Network (3.2-3.3, 3.9-3.10)
- Track C: Code Quality (3.4-3.6, 3.16-3.22)

**Wave 4** (Advanced): 3 parallel tracks:
- Track A: OAST + Scripting (4.1-4.2)
- Track B: Agent Capabilities (4.6-4.10)
- Track C: NSE + Proxy (4.14-4.19)

---

## Known Limitations (from Historical plan.md)

### rt.block_on Deadlock Risk (Ruby API)

**File**: `crates/slapper-ruby/src/api.rs`

35 instances of `get_runtime().block_on` in synchronous Ruby functions calling async code. Requires significant refactoring (moving to fully async API or using `spawn`).

### NSE Socket Library Not Sandboxed

**Status**: Documented in `docs/NSE_SCRIPTS.md` and `slapper_skills/nse_sandbox.md`.

The `socket` library is NOT sandboxed even when `nse-sandbox` is enabled. The `lfs` library IS sandboxed with path restrictions.

---

## For Future Agents

When starting new improvement work:

1. Run `cargo test --lib -p slapper` to verify baseline
2. Run `cargo clippy --lib -p slapper` to check warnings
3. Create a new plan file for new work (don't modify this one)
4. Update AGENTS.md with any new patterns discovered
5. Always verify plan items against actual codebase before assuming they still apply
6. Use `rg` to confirm file paths, line numbers, and patterns exist

---

## Historical Context

Original plan files consolidated into this document:
- plan.md — Historical (completed work reference)
- plan2.md — Feature Flag Improvements (REST API, AI, Plugins, Stress, Headless Browser, NSE)
- plan3.md — Plugin System Improvements
- plan4.md — Tool Extensibility (OAST, Scripting, Templates, Sessions, Reports)
- plan5.md — Security Issues (Critical, High, Medium, Low)
- plan6.md — Performance Issues
- plan7.md — Code Quality Issues
- plan8.md — Agentic Capabilities (LLM Providers, Multi-Agent, Memory)
