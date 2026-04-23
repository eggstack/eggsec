# Slapper Consolidated Improvement Plan

**Date**: 2026-04-23
**Status**: CONSOLIDATED - All plan files merged
**Review Source**: Comprehensive analysis of plan.md, plan2.md through plan14.md

---

## Overview

This document consolidates all improvement items from 14 plan files into a single coherent plan with logical ordering and wave-based parallelization for sub-agent execution.

### Original Plan Files

| File | Focus | Items |
|------|-------|-------|
| `plan.md` | Codebase review, critical issues | 16 |
| `plan2.md` | Code quality issues | 16 |
| `plan3.md` | Security hardening, dependencies | 21 |
| `plan4.md` | Security audit | 14 |
| `plan5.md` | Performance optimization | 13 |
| `plan6.md` | Security improvements | 14 |
| `plan7.md` | Deep dive findings | 6 |
| `plan8.md` | Performance deep dive | 13 |
| `plan9.md` | TUI improvements | 10 |
| `plan10.md` | Agent harness | 8 |
| `plan11.md` | Plugin architecture | 18 |
| `plan12.md` | Unified plugin architecture | 5 phases |
| `plan13.md` | Agent harness deep dive | 8 |
| `plan14.md` | TUI usability | 14 |

### Priority Summary

| Priority | Items | Wave |
|----------|-------|------|
| CRITICAL | 8 | Wave 1 |
| HIGH | 25+ | Wave 2 |
| MEDIUM | 30+ | Wave 3-4 |
| LOW | 20+ | Wave 5-6 |

---

## Wave 1: CRITICAL Security Fixes (Sequential)

Items that are blocking/broken and must be fixed first.

### 1.1: Failing Test - `git_secrets::test_scan_current_directory`

**File**: `crates/slapper/src/recon/git_secrets.rs:397-403`

**Issue**: Test assertion `assert!(result.is_ok())` fails intermittently depending on working directory state.

**Fix**:
```rust
// Current (line 400):
assert!(result.is_ok());

// Recommended fix:
assert!(result.is_ok(), "Git secrets scan failed: {:?}", result.err());
let report = result.unwrap();
assert!(report.commits_scanned >= 0 && report.commits_scanned <= 100,
    "Expected 0-100 commits, got {}", report.commits_scanned);
```

### 1.2: Feature Flag Bug - Missing `IntoResponse` Import

**File**: `crates/slapper/src/tool/protocol/ai_routes.rs:1`

**Issue**: Compilation failure when using `rest-api` feature without `ai-integration`.

**Fix**: Add to imports at line 1:
```rust
#[cfg(feature = "rest-api")]
use axum::{
    extract::State,
    response::IntoResponse,  // ADD THIS
    routing::{get, post},
    Json, Router,
};
```

### 1.3: Plugin Timeout Not Enforced

**Files**:
- `crates/slapper-plugin/src/lib.rs:175-188`
- `crates/slapper-plugin/src/python.rs:394-442`
- `crates/slapper-ruby/src/loader.rs:147-192`
- `crates/slapper-ruby/src/bridge.rs:144-155`

**Root Cause**: `timeout_secs` configuration (default 300s) is never passed to async task executing plugins.

**Fix - Python**:
```rust
async fn run_check(&self, check_name: &str, target: &str, timeout_secs: u64) -> Result<PluginResult> {
    let json_results = tokio::time::timeout(
        Duration::from_secs(timeout_secs),
        tokio::task::spawn_blocking({
            let check_name = check_name.to_string();
            let target = target.to_string();
            move || self.run_check_direct(&check_name, &target, &serde_json::Value::Object(serde_json::Map::new()))
        })
    )
    .await
    .map_err(|_| anyhow::anyhow!("Plugin execution timed out after {} seconds", timeout_secs))??;
}
```

**Fix - Ruby**:
```rust
pub fn run_plugin(&self, plugin: &RubyPlugin, target: &str, timeout_secs: u64) -> Result<RubyPluginResult> {
    let (tx, rx) = mpsc::channel();
    self.tx.send(RubyRequest::RunPlugin { ... })?;
    match rx.recv_timeout(Duration::from_secs(timeout_secs)) {
        Ok(result) => result,
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
            anyhow::bail!("Plugin execution timed out after {} seconds", timeout_secs)
        }
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
            anyhow::bail!("Ruby VM thread has shut down")
        }
    }
}
```

### 1.4: Race Condition in Port Scanner

**Files**:
- `crates/slapper/src/scanner/ports/mod.rs:507,543-572`
- `crates/slapper/src/scanner/fingerprint.rs:268-272`
- `crates/slapper/src/scanner/endpoints.rs:761-765`

**Root Cause**: Check-then-act race - `count >= limit` and `+= 1` under separate mutex acquisitions.

**Fix**: Use `AtomicU64::fetch_add` for atomic check-and-increment:
```rust
// Change results_count from Mutex<usize> to AtomicU64
let results_count = Arc::new(AtomicU64::new(0));

// Then atomic check-and-increment
Some(limit) => {
    let old = results_count.fetch_add(1, Ordering::Relaxed);
    old < limit
}
```

### 1.5: Path Traversal in Plugin Loading

**Files**:
- `crates/slapper-plugin/src/python.rs:155-224`
- `crates/slapper-plugin/src/lib.rs:267-289`
- `crates/slapper-ruby/src/loader.rs:61-92`

**Fix**: Create shared validation in `slapper-plugin/src/validation.rs`:
```rust
pub fn validate_plugin_path(base: &Path, user_path: &Path) -> Result<PathBuf> {
    let canonical = user_path.canonicalize()
        .map_err(|e| anyhow!("Failed to canonicalize path: {}", e))?;
    let base_canonical = base.canonicalize()
        .map_err(|e| anyhow!("Failed to canonicalize base path: {}", e))?;
    if !canonical.starts_with(&base_canonical) {
        return Err(anyhow!("Path traversal detected"));
    }
    Ok(canonical)
}
```

### 1.6: TUI Plugin Tab Cannot Compile

**Files**:
- `crates/slapper/src/tui/workers/plugin.rs:11`
- `crates/slapper/src/tui/workers/runner.rs:168-222`
- `crates/slapper/src/tui/app/task_management.rs:368-373`
- `crates/slapper/src/tui/app/mod.rs:444-485`

**Issues**:
1. `TaskResult::PluginsLoaded` variant does not exist
2. `PluginTab::build_task_config()` always returns `None`
3. `build_current_task()` doesn't handle `Tab::Plugin`
4. `DiscoveredPlugin` type mismatch with `PluginInfo`

**Fix**: Add missing variant, implement task config, add tab handler, add type conversion.

### 1.7: Auth Pattern - Replace `unwrap_u8()`

**Files**: 6 locations
- `tool/protocol/rest.rs:137`
- `tool/protocol/ai_routes.rs:40`
- `tool/protocol/agent_routes.rs:279`
- `tool/protocol/openai/handlers.rs:26`
- `tool/protocol/mcp/auth.rs:11`
- `tool/protocol/grpc.rs:27`

**Fix**:
```rust
// Before:
Some(v) if key.as_bytes().ct_eq(v.as_bytes()).unwrap_u8() == 1 => Ok(()),

// After:
Some(v) if key.as_bytes().ct_eq(v.as_bytes()).into() => Ok(()),
```

### 1.8: Silent Data Loss in Serialization

**Files**:
- `tool/response.rs:260-262`
- `distributed/worker.rs:172`

**Fix**: Return `Result<String, SerError>`:
```rust
pub fn to_json_line(&self) -> Result<String, serde_json::Error> {
    serde_json::to_string(self).map(|s| s + "\n")
}
```

### 1.9: TOCTOU Race Condition in Config Loading

**File**: `config/loader.rs:19-27, 46-63`

**Fix**: Use `canonicalize()` before reading:
```rust
let canonical_path = path.canonicalize().map_err(|e| {
    anyhow::anyhow!("Failed to canonicalize config path '{}': {}", path.display(), e)
})?;
let content = fs::read_to_string(&canonical_path)?;
```

---

## Wave 2: HIGH Priority Security (Parallel)

### 2.1: Ruby Sandbox Escape

**File**: `crates/slapper-ruby/src/api.rs:81-962`

**Issue**: Dangerous APIs bypass pattern detection. Plugins have unrestricted network access via `Slapper::HTTP`, `Slapper::Scanner`, `Slapper::Metasploit`.

**Recommended Fix**: Remove HTTP, Scanner, Fuzzer, Metasploit modules - keep only safe reporting methods.

### 2.2: Python Suspicious Pattern Detection Gaps

**File**: `crates/slapper-plugin/src/python.rs:16-27`

**Missing Patterns** (add to `static SUSPICIOUS_PATTERNS`):
```rust
("pty.spawn", regex::Regex::new(r"pty\.spawn").unwrap()),
("os.popen", regex::Regex::new(r"os\.popen").unwrap()),
("multiprocessing.Process", regex::Regex::new(r"multiprocessing\.Process").unwrap()),
("ctypes", regex::Regex::new(r"ctypes").unwrap()),
("importlib", regex::Regex::new(r"importlib").unwrap()),
("getattr", regex::Regex::new(r"getattr\(").unwrap()),
("chr()", regex::Regex::new(r"chr\(").unwrap()),
("hex escape", regex::Regex::new(r"\\x[0-9a-fA-F]{2}").unwrap()),
("unicode escape", regex::Regex::new(r"\\u[0-9a-fA-F]{4}").unwrap()),
("octal escape", regex::Regex::new(r"\\[0-7]{3,}").unwrap()),
```

### 2.3: Ruby Pattern Detection Gaps

**File**: `crates/slapper-ruby/src/bridge.rs:13-31`

**Missing Patterns**:
```rust
("Kernel.exec", regex::Regex::new(r"Kernel\.exec").unwrap()),
("open alias", regex::Regex::new(r"\bopen\b").unwrap()),
("eval without parens", regex::Regex::new(r"\beval\b").unwrap()),
```

### 2.4: TLS Bypass Without Warning Logging

**Files** (11 locations need warning logs):
- `scanner/cms/mod.rs:82`
- `scanner/cms/joomla.rs:31`
- `scanner/cms/drupal.rs:31`
- `scanner/templates/executor.rs:25`
- `waf/detector/compare.rs:17`
- `stress/http.rs:153`
- `proxy/health.rs:100`
- `fuzzer/engine/advanced.rs:18`
- `fuzzer/engine/core.rs:178`
- `scanner/endpoints.rs:686`
- `loadtest/runner.rs:232`

**Fix**: Add warning log when using `danger_accept_invalid_certs(true)`:
```rust
if self.insecure {
    tracing::warn!(
        "TLS certificate verification disabled. This is insecure and should only \
         be used in isolated testing environments."
    );
    client = client.danger_accept_invalid_certs(true);
}
```

### 2.5: Scope Validation Missing in REST API

**File**: `tool/protocol/rest.rs:301-339`

**Fix**: Add scope check before dispatch:
```rust
let target_url = &payload.target;
if let Some(ref scope) = state.scope {
    if !scope.is_allowed(target_url) {
        return Err(AppError::ScopeViolation(target_url.clone()));
    }
}
```

### 2.6: Scope Validation Missing in MCP Handlers

**File**: `tool/protocol/mcp/handlers.rs:252-337`

**Fix**: Add scope check before tool execution.

### 2.7: Scope Validation Missing in OpenAI Handlers

**File**: `tool/protocol/openai/handlers.rs:121-207`

**Fix**: Add scope check on extracted targets before execution.

### 2.8: Credential Exposure in Proxy URL

**Files**:
- `crates/slapper/src/proxy/config.rs:132-147`
- `crates/slapper/src/proxy/pool.rs:173-188`

**Fix**: Add separate logging method:
```rust
pub fn to_log_key(&self) -> String {
    match (&self.username, &self.password) {
        (Some(user), Some(_)) => format!("{}://{}:***@{}:{}", scheme, user, self.address, self.port),
        _ => self.to_url(),
    }
}
```

### 2.9: ai_client Field Never Used in Agent

**File**: `crates/slapper/src/agent/mod.rs:77-78`

**Fix**: Integrate `ai_client` into scan workflow:
```rust
// In handle_findings(), after collecting findings
#[cfg(feature = "ai-integration")]
if let Some(ref client) = self.ai_client {
    let analysis = client.analyze_findings_typed(&finding_values).await?;
    // Use analysis to determine alert severity
}
```

### 2.10: Formula Injection Unicode Bypass Fix

**File**: `crates/slapper/src/output/escape.rs:16-35`

**Issue**: Fullwidth Unicode variants bypass CSV formula detection.

**Fix**: Normalize input to NFKC form before checking.

**Test Cases**:
```rust
#[test]
fn test_fullwidth_equals_bypass() {
    assert!(escape_csv("\u{FF1D}1+1").starts_with('"'));
}
#[test]
fn test_fullwidth_plus_bypass() {
    assert!(escape_csv("\u{FF0B}2+2").starts_with('"'));
}
```

---

## Wave 3: Code Quality - TUI & Plugin Refactoring (Parallel, High Effort)

### 3.1: TUI Tab Dispatching Duplication

**Files**:
- `crates/slapper/src/tui/app/mod.rs` (~967 lines)
- `crates/slapper/src/tui/tabs/mod.rs` (859 lines)

**Issue**: ~1,500 lines of repetitive 29-arm match statements.

**Fix**: Introduce `enum_dispatch` crate pattern:
```toml
# Cargo.toml
enum_dispatch = "0.3"
```

### 3.2: TUI Architecture Refactoring

**Files** to split:
- `tui/app/mod.rs` (967 lines) → Extract task management, state fields
- `tui/tabs/mod.rs` (859 lines) → Extract traits to `traits.rs`
- `tui/tabs/settings.rs` (798 lines) → Split to `settings/main.rs`, `settings/http.rs`, `settings/proxy.rs`
- `tui/tabs/packet.rs` (743 lines) → Split to `packet/capture.rs`, `packet/send.rs`
- `tui/tabs/fuzz.rs` (698 lines) → Split to `fuzz/config.rs`, `fuzz/results.rs`

### 3.3: MCP Handlers Large File Refactoring

**File**: `tool/protocol/mcp/handlers.rs` (1081 lines)

**Split Plan**:
- `handlers_server.rs` (~250 lines) - McpServer struct, constructor
- `handlers_request.rs` (~700 lines) - Request handlers, tool handlers
- `handlers_helpers.rs` (~150 lines) - Helper functions

### 3.4: recon/dependency_scan Large File

**File**: `recon/dependency_scan.rs` (1051 lines)

**Split Plan**: Split by ecosystem into `recon/dependency/npm.rs`, `cargo.rs`, `go.rs`, `ruby.rs`, etc.

### 3.5: Plugin System Fixes (PLG-007 to PLG-018)

| Issue | File | Fix |
|-------|------|-----|
| PLG-007: Credential exposure | `lib.rs`, `api.rs` | Add filtered_config() |
| PLG-008: Missing Mutex | `python.rs:58-63` | Add `Mutex<Vec<LoadedPlugin>>` |
| PLG-009: TUI task config | `task_management.rs:368-373` | Implement actual task spawning |
| PLG-010: Type mismatch | `plugin.rs` vs `tabs/plugin.rs` | Add From impl |
| PLG-011: Missing lifecycle | `lib.rs` | Add `init()`, `shutdown()` |
| PLG-012: Missing health check | `lib.rs` | Add `health_check()` |
| PLG-013: O(n) registry | `lib.rs` | Change to `HashMap` |
| PLG-014: PyO3 API mixing | `python.rs` | Migrate to Bound API |
| PLG-015: Case-insensitive | `python.rs`, `bridge.rs` | Add `(?i)` flag |
| PLG-016: Hot reload | `lib.rs` | Add `reload_plugin()` |
| PLG-017: Plugin priority | `lib.rs` | Add `priority` field |
| PLG-018: Namespace isolation | `python.rs:226-258` | Prefix class names |

### 3.6: CircuitBreakerRegistry Dead Code or Utilization

**File**: `utils/circuit_breaker.rs`

**Issue**: `CircuitBreakerRegistry` never instantiated. Each `AiClient` creates own `CircuitBreaker` directly.

**Fix**: Either remove as dead code OR utilize for multi-provider support.

---

## Wave 4: Performance Optimization (Parallel)

### 4.1: HashMap -> FxHashMap Migration

**Files**: 140 locations use `std::collections::HashMap`

**Highest Priority Hot Paths**:
- `waf/detector/mod.rs:19-20` - signatures storage
- `waf/detector/detect.rs:159` - detection loop
- `scanner/templates/models.rs:8` - templates
- `fuzzer/chain.rs:5` - fuzz chains
- `proxy/intercept/rules.rs:7` - rules

**Fix**:
```rust
// Before:
use std::collections::HashMap;

// After:
use rustc_hash::FxHashMap;
```

### 4.2: to_lowercase() Optimization

**Critical Locations**:
- `vuln/triage.rs:48,52` - 9 lowercalls on same strings
- `recon/dependency_scan.rs:855-857` - 5 lowercalls
- `ai/planner.rs:397,411` - nested loop

**Fix**: Cache lowercase once:
```rust
let title_lower = title.to_lowercase();
let description_lower = description.to_lowercase();
```

### 4.3: std::Mutex -> parking_lot::Mutex

**Locations**:
- `scanner/ports/spoofed.rs:48`
- `stress/metrics.rs:112`
- `tui/workers/recon.rs:112`

### 4.4: TimingAnalyzer Lock-Free Redesign

**File**: `fuzzer/detection/analyzer.rs:196-199`

**Issue**: Single mutex serializes 100+ concurrent tasks.

**Fix**: Use `crossbeam::queue::SegQueue` + atomic stats:
```rust
pub struct TimingAnalyzer {
    sample_queue: SegQueue<f64>,  // Lock-free queue
    total_requests: AtomicU64,
    // ... atomics for other stats
}
```

### 4.5: RateLimiter DashMap Conversion

**File**: `tool/ratelimit.rs:76-79`

**Issue**: `RwLock<HashMap>` forces serialized writes.

**Fix**:
```rust
pub struct RateLimiter {
    config: RateLimitConfig,
    tokens: DashMap<String, TokenBucket>,  // Sharded locking
    // ...
}
```

### 4.6: Regex Caching in ChainExecutor

**File**: `fuzzer/chain.rs:241,307`

**Issue**: Fresh `RegexBuilder` per call.

**Fix**: Add cache:
```rust
pub struct ChainExecutor {
    // ...
    regex_cache: FxHashMap<String, regex::Regex>,
}
```

### 4.7: tokio::sync::watch for Progress Updates

**Files**: `tui/workers/*.rs`

**Issue**: Polling-based progress updates.

**Fix**: Use `tokio::sync::watch` channel:
```rust
let (tx, rx) = watch::channel::<String>("initial".to_string());
```

### 4.8: String Allocation Optimizations

**Locations**:
- `scanner/fingerprint.rs:327` - `format!("{}:{}", host, port)` per port
- `scanner/udp_fingerprint.rs:139` - similar

**Fix**: Pre-resolve `SocketAddr` once.

---

## Wave 5: Agent System Improvements (Parallel)

### 5.1: Graceful Shutdown Not Implemented

**File**: `crates/slapper/src/agent/mod.rs:129-159`

**Fix**: Add SIGTERM/SIGINT handlers with `CancellationToken`:
```rust
use tokio_util::sync::CancellationToken;

pub async fn run(&mut self) -> Result<()> {
    let token = CancellationToken::new();
    // Spawn signal handlers
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await?;
        token.cancel();
    });

    loop {
        tokio::select! {
            _ = token.cancelled() => break,
            _ = poll_interval.tick() => { /* work */ }
        }
    }
}
```

### 5.2: Event Loop Swallows Errors

**File**: `agent/mod.rs:145-147`

**Fix**:
```rust
if let Err(e) = self.process_scheduled_scans().await {
    tracing::warn!(error = %e, "Scheduled scan failed");
} else {
    tracing::debug!("Processed scheduled scans");
}
```

### 5.3: Dedup Key Collision in AlertRouter

**File**: `agent/alerts/routing.rs:237-244`

**Issue**: Different `finding_ids` produce same dedup key.

**Fix**: Include hash of finding_ids:
```rust
fn make_dedup_key(&self, alert: &Alert) -> String {
    let mut hasher = DefaultHasher::new();
    let mut sorted_ids = alert.finding_ids.clone();
    sorted_ids.sort();
    for id in &sorted_ids { id.hash(&mut hasher); }
    let finding_hash = hasher.finish();
    format!("{}:{}:{}:{:016x}", alert.target, alert.severity.as_str(), alert.title, finding_hash)
}
```

### 5.4: Lock Poisoning in TargetPortfolio

**File**: `agent/portfolio.rs:221,225,229,233`

**Issue**: Uses `std::sync::RwLock` with `.unwrap()`.

**Fix**: Migrate to `parking_lot::RwLock`:
```rust
use parking_lot::RwLock;
pub struct TargetPortfolio {
    data: Arc<RwLock<PortfolioData>>,
}
```

### 5.5: Severity Filtering Only Handles Critical

**File**: `agent/mod.rs:268-296`

**Issue**: Only `Critical` findings trigger alerts.

**Fix**: Implement multi-level alerting:
```rust
fn determine_alert_level(severity: &ResponseSeverity) -> AlertLevel {
    match severity {
        ResponseSeverity::Critical => AlertLevel::Immediate,
        ResponseSeverity::High => AlertLevel::Important,
        ResponseSeverity::Medium => AlertLevel::Notice,
        _ => AlertLevel::Info,
    }
}
```

### 5.6: Path Validation in LongitudinalMemory

**File**: `agent/memory.rs:107-123`

**Issue**: Uses string replacement instead of canonicalization.

**Fix**: Use `crate::utils::validation::validate_path()`.

### 5.7: Timezone Parsing Only Numeric

**File**: `agent/portfolio.rs:78-96`

**Issue**: Named timezones silently fail.

**Fix**: Add `chrono-tz` support:
```rust
use chrono_tz::Tz;
let tz: Tz = self.timezone.parse().unwrap_or(chrono_tz::UTC);
let local = time.with_timezone(&tz);
```

### 5.8: Missing Error Propagation

**File**: `agent/mod.rs:267-296`

**Fix**: `handle_findings()` should return `Result<()>` and propagate errors.

### 5.9: TOCTOU Race in AlertRouter Dedup

**File**: `agent/alerts/routing.rs:45-75`

**Issue**: Dedup check and insert not atomic.

**Fix**: Use `DashMap` for atomic check-and-insert.

---

## Wave 6: REST API & External Integrations (Parallel)

### 6.1: REST API TLS Configuration

**File**: `tool/protocol/rest.rs:16-35`

**Issue**: `RestState` lacks TLS configuration fields.

**Fix**: Import `TlsConfig` from `crate::distributed`, add TLS fields.

### 6.2: REST API Rate Limiting Improvements

**File**: `tool/ratelimit.rs:11-16`

**Missing**:
- Per-endpoint rate limiting
- Global rate limit
- IP-based limiting

### 6.3: REST API WebSocket Support

**File**: `tool/protocol/mcp/streaming.rs:1-19`

**Note**: `websocket` feature already exists with `tokio-tungstenite`.

### 6.4: UDP IP Spoofing Integration

**File**: `stress/udp.rs:19-117`

**Issue**: `raw_udp` module exists but unused.

**Fix**: Integrate with `--spoof-ip` flag when `stress-testing` enabled.

### 6.5: Ruby API block_on Deadlock Risk

**File**: `slapper-ruby/src/api.rs`

**Issue**: 35 instances of `get_runtime().block_on()`.

**Fix**: Use dedicated thread pool:
```rust
static ASYNC_POOL: std::sync::OnceLock<tokio::runtime::Runtime> =
    std::sync::OnceLock::new();
```

---

## Wave 7: Dependency Updates (Sequential - Highest Risk)

### 7.1: Axum 0.7.x -> 0.8.x

**Breaking Changes**:
- Path syntax `/:param` → `/{param}`
- `#[async_trait]` removed
- `Option<T>` extractor changes

### 7.2: Tonic 0.12.x -> 0.14.x

**Breaking Changes**:
- prost extracted to separate crate
- `BoxBody` removed

**Note**: Must be done WITH axum update.

### 7.3: Duplicate Dependencies Resolution

| Dependency | Old | New |
|------------|-----|-----|
| base64 | 0.21.7 | 0.22.x |
| darling | 0.20.11 | 0.23.0 |

---

## Wave 8: TUI Usability Improvements (Parallel)

### 8.1: Global Search (Ctrl+F)

**Files**: `tui/search.rs` (new), `tui/app/runner.rs`, `tui/ui.rs`

**Issue**: Search only works in History tab.

### 8.2: Clipboard/Copy-Paste Support

**Files**: `tui/utils/clipboard.rs` (new), `tui/components/selection.rs` (new)

**Recommendation**: Use `arboard` crate (pure Rust).

### 8.3: Pause/Resume (Ctrl+Z/Ctrl+Y)

**Files**: `tui/workers/pause.rs` (new), `tui/app/mod.rs`, `tui/app/runner.rs`

**Issue**: Help shows "Space for Pause/Resume" but no implementation.

### 8.4: Tab Overflow Display

**File**: `tui/ui.rs:255-272`

**Issue**: All 39 tabs rendered, overflow on narrow terminals.

**Fix**: Add horizontal scroll or tab groups.

### 8.5: Input Validation with Visual Feedback

**File**: `components/input.rs:8-12`

**Issue**: `ValidationResult` struct exists but completely unused.

**Fix**: Implement validate() method, show red border for invalid.

### 8.6: Session Auto-Persistence

**Files**: `tui/session.rs` (new), state management

**Issue**: No auto-save, session lost on crash.

### 8.7: Theme System (Dark/Light)

**File**: `tui/theme.rs` (new)

**Issue**: 463+ inline `Color::*` literals, no theme switching.

### 8.8: Keyboard Shortcuts Inline Display

**File**: `tui/ui.rs`

**Issue**: Some users miss status bar hints.

### 8.9: Tab Bookmarks/Favorites

**Issue**: No way to bookmark frequently-used tabs.

---

## Wave 9: Plugin Architecture Unification (Long Term)

### 9.1: Unified PluginBackend Trait

Create unified `PluginBackend` trait abstracting over language-specific implementations.

### 9.2: Shared Security Patterns Module

Consolidate Python and Ruby security patterns into `slapper-plugin/src/security.rs`.

### 9.3: Move Ruby Loader Into Plugin Crate

Consolidate `slapper-ruby/src/loader.rs` into `slapper-plugin`.

### 9.4: Plugin Lifecycle Features

Add `init()`, `activate()`, `deactivate()`, `health_check()` to Plugin trait.

### 9.5: Metasploit Integration Enhancement

Add auto-pivoting, session persistence/caching.

---

## Implementation Order by Parallelization

### Can Run in Parallel (Same Wave Items)

| Wave | Parallel Group | Items |
|------|---------------|-------|
| 1 | Quick fixes | 1.1, 1.2, 1.7, 1.8 |
| 1 | Plugin security | 1.3, 1.4, 1.5, 1.6, 1.9 |
| 2 | Security patterns | 2.2, 2.3, 2.4, 2.8 |
| 2 | Scope validation | 2.5, 2.6, 2.7 |
| 2 | Agent AI integration | 2.9 |
| 3 | Large file splits | 3.1, 3.2, 3.3, 3.4 |
| 3 | Plugin fixes | 3.5 (multiple items) |
| 4 | Performance | 4.1, 4.2, 4.3, 4.6, 4.8 |
| 4 | Async optimizations | 4.4, 4.5, 4.7 |
| 5 | Agent fixes | 5.1-5.9 (all independent) |
| 6 | API improvements | 6.1-6.5 (all independent) |
| 8 | TUI features | 8.1-8.9 (all independent) |

### Sequential (Must Complete Before Next)

1. Wave 1 (all items - foundation)
2. Wave 2 (security - after Wave 1)
3. Wave 3 (refactoring - after Wave 1-2)
4. Wave 4 (performance - after Wave 3)
5. Wave 5 (agent - after Wave 1)
6. Wave 6 (API - after Wave 1)
7. Wave 7 (dependencies - after all, highest risk)
8. Wave 8 (TUI - anytime after Wave 1)
9. Wave 9 (plugin unification - long term)

---

## Sub-Agent Parallelization Strategy

### Agent 1: Security Fixes (Wave 1)
- 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8, 1.9

### Agent 2: High-Priority Security (Wave 2)
- 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 2.8, 2.9, 2.10

### Agent 3: TUI & Plugin Refactoring (Wave 3)
- 3.1, 3.2, 3.3, 3.4, 3.5, 3.6

### Agent 4: Performance Optimization (Wave 4)
- 4.1, 4.2, 4.3, 4.4, 4.5, 4.6, 4.7, 4.8

### Agent 5: Agent System (Wave 5)
- 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 5.7, 5.8, 5.9

### Agent 6: API & Integrations (Wave 6)
- 6.1, 6.2, 6.3, 6.4, 6.5

### Agent 7: Dependency Updates (Wave 7)
- 7.1, 7.2, 7.3

### Agent 8: TUI Improvements (Wave 8)
- 8.1, 8.2, 8.3, 8.4, 8.5, 8.6, 8.7, 8.8, 8.9

---

## Verification Commands

Before starting any work:
```bash
cargo test --lib -p slapper
cargo clippy --lib -p slapper
cargo test --lib -p slapper -- --list 2>/dev/null | wc -l  # Expected: 1148+
find crates/slapper/src -name '*.rs' | wc -l  # Expected: 470+
```

After each wave:
```bash
cargo test --lib -p slapper
cargo clippy --lib -p slapper
```

---

## Notes for Future Agents

1. **Wave 1 items are foundational** - many other items depend on these being fixed first.

2. **Wave 7 (Dependencies) is highest risk** - test thoroughly after axum/tonic migration.

3. **Plugin timeout (1.3)** cannot forcibly terminate threads - document this limitation.

4. **Ruby sandbox (2.1)** is a design decision - consider if Ruby plugins should be restricted to read-only operations.

5. **TimingAnalyzer redesign (4.4)** significantly changes statistics collection - test with concurrent workloads.

6. **crossbeam is already in Cargo.toml** - no new dependencies needed for SegQueue.

7. **Feature-gated tabs** need both `#[cfg]` and `#[cfg(not())]` arms - always.

8. **WAF detection patterns** - verify `_lower` field serialization compatibility.

9. **TUI Plugin Tab Compile Error**: `TaskResult::PluginsLoaded` variant is referenced in `tui/workers/plugin.rs:11` but may not exist in `TaskResult` enum. Verify and add missing variant.

10. **Large File Reference**:
    | File | Lines | Should Split? |
    |------|-------|---------------|
    | `tui/app/mod.rs` | 967 | Yes |
    | `tool/protocol/mcp/handlers.rs` | 1081 | Yes |
    | `recon/dependency_scan.rs` | 1051 | Yes |

---

## Historical Context

Original plan files consolidated:
- `plan.md` - Original consolidated plan
- `plan2.md` - Code Quality Issues
- `plan3.md` - Security, Dependencies
- `plan4.md` - Security Audit
- `plan5.md` - Performance Optimization
- `plan6.md` - Security Improvements
- `plan7.md` - Deep Dive Findings
- `plan8.md` - Performance Deep Dive
- `plan9.md` - TUI Improvements
- `plan10.md` - Agent Harness
- `plan11.md` - Plugin Architecture
- `plan12.md` - Unified Plugin Architecture
- `plan13.md` - Agent Harness Deep Dive
- `plan14.md` - TUI Usability

---

*End of Consolidated Plan*