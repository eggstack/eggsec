# Slapper Consolidated Improvement Plan

**Date**: 2026-04-24
**Status**: COMPLETED - large file splits done (3.3, 3.4)
**Note**: Wave 1 items, 2.x, 3.3, 3.4 verified as completed as of 2026-04-24
**Original Sources**: 14 plan files (plan.md through plan14.md), now consolidated

---

## Overview

This document consolidates all improvement items into a single coherent plan with wave-based parallelization for sub-agent execution. Every item has been verified against the actual codebase for accuracy.

### Priority Summary

| Priority | Items | Wave |
|----------|-------|------|
| CRITICAL | 8 | Wave 1 |
| HIGH | 10 | Wave 2 |
| MEDIUM | 12 | Waves 3-4 |
| LOW | 24+ | Waves 5-9 |

---

## Wave 1: CRITICAL Security Fixes (Parallel within groups)

Items that are blocking/broken and must be fixed first. These are foundational — later waves depend on these.

### Sub-Agent Instructions for Wave 1

Run `cargo test --lib -p slapper` and `cargo clippy --lib -p slapper` before and after each change. Each fix should be independently testable. All paths are relative to `crates/slapper/src/` unless otherwise noted.

---

### 1.1: Failing Test - `git_secrets::test_scan_current_directory`

**File**: `crates/slapper/src/recon/git_secrets.rs:397-403`

**Verified**: Yes, issue present. Test calls `scan_directory(".")` which is fragile — depends on CWD being a valid git repo with readable history. Fails in CI with shallow clones or detached HEAD.

**Current code (line 400)**:
```rust
assert!(result.is_ok());
```

**Fix**: Improve assertion to provide diagnostic output and tolerate empty repos:
```rust
assert!(result.is_ok(), "Git secrets scan failed: {:?}", result.err());
let report = result.unwrap();
assert!(report.commits_scanned >= 0 && report.commits_scanned <= 100,
    "Expected 0-100 commits, got {}", report.commits_scanned);
```

**After fixing**: Run `cargo test --lib -p slapper -- git_secrets::test_scan_current_directory` to verify.

---

### 1.2: Plugin Timeout Not Enforced

**Files**:
- `crates/slapper-plugin/src/lib.rs:175-188` — `run_all()` uses `join_all` with no timeout
- `crates/slapper-plugin/src/python.rs:520-582` — `run()` trait impl, `_config` received but `timeout_secs` never used
- `crates/slapper-ruby/src/loader.rs:330-332` — `run()` trait impl, `_config` explicitly unused
- `crates/slapper-ruby/src/bridge.rs:144-155` — `run_plugin()` uses `rx.recv()` with no timeout

**Verified**: Yes, issue present at all four locations. `PluginConfig.timeout_secs` (default 300s) is defined at `lib.rs:39-40` but never enforced anywhere in the execution chain.

**Root Cause**: `timeout_secs` configuration is never passed to async task executing plugins.

**Fix - Python** (`python.rs`, in the `run()` method at line 520):
```rust
async fn run(&self, target: &str, config: &PluginConfig) -> Result<PluginResult> {
    let timeout = Duration::from_secs(config.timeout_secs);
    // Wrap the entire run_check_direct loop in tokio::time::timeout
    let result = tokio::time::timeout(timeout, async {
        // ... existing run logic ...
    }).await
    .map_err(|_| anyhow::anyhow!("Plugin execution timed out after {} seconds", config.timeout_secs))?;
    // ...
}
```

**Fix - Ruby** (`bridge.rs`, in `run_plugin()` at line 144):
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

**Note**: Plugin timeout cannot forcibly terminate threads — it only prevents waiting indefinitely. Document this limitation.

---

### 1.3: Race Condition in Port Scanner

**Files**:
- `crates/slapper/src/scanner/ports/mod.rs:507,543-572`
- `crates/slapper/src/scanner/fingerprint.rs:268-272`
- `crates/slapper/src/scanner/endpoints.rs:761-765`

**Verified**: Yes, classic TOCTOU race at all three locations. Two separate `Mutex` acquisitions: count is read in lock #1, released, then lock #2 is acquired to increment. Between the two locks, N concurrent tasks can all read the same count and all decide to increment, exceeding `max_results`.

**Current code pattern (all three files)**:
```rust
let count = *results_count.lock().await;     // Lock acquisition #1
if count >= limit {
    false
} else {
    *results_count.lock().await += 1;        // Lock acquisition #2 (separate!)
    true
}
```

**Fix**: Use `AtomicU64::fetch_add` for atomic check-and-increment:
```rust
// Change results_count from Mutex<usize> to AtomicU64
let results_count = Arc::new(AtomicU64::new(0));

// Then atomic check-and-increment (single atomic operation)
Some(limit) => {
    let old = results_count.fetch_add(1, Ordering::Relaxed);
    old < limit
}
```

**After fixing**: Run `cargo test --lib -p slapper -- scanner::ports` and `cargo test --lib -p slapper -- scanner::endpoints` to verify.

---

### 1.4: Path Traversal in Plugin Loading

**Files**:
- `crates/slapper-plugin/src/python.rs:155-224` — `load_plugins()` reads paths from `read_dir()` with no canonicalization
- `crates/slapper-plugin/src/lib.rs:267-289` — `discover_plugins()` same issue
- `crates/slapper-ruby/src/loader.rs:61-92` — `discover_plugins()` same issue

**Verified**: Yes, no `canonicalize()` call anywhere in any plugin loading path. Zero occurrences of "canonicalize" in `slapper-plugin/src/` and `slapper-ruby/src/`.

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

Then call `validate_plugin_path(plugin_dir, &file_path)` in all three locations before reading/processing plugin files.

---

### 1.5: TUI Plugin Tab Cannot Compile

**Files**:
- `crates/slapper/src/tui/workers/plugin.rs:11` — References missing `TaskResult::PluginsLoaded` variant
- `crates/slapper/src/tui/workers/runner.rs:168-222` — `TaskResult` enum (no `PluginsLoaded` variant exists)
- `crates/slapper/src/tui/app/task_management.rs:368-373` — `build_task_config()` always returns `None`
- `crates/slapper/src/tui/app/mod.rs` — `build_current_task()` doesn't handle `Tab::Plugin`

**Verified**: Yes, issue present but currently masked by pyo3 API incompatibility errors in `slapper-plugin` (8 errors from deprecated `into_py`, `PyNone`). The `PluginsLoaded` missing variant error would surface after those are fixed.

**Issues**:
1. `TaskResult::PluginsLoaded` variant does not exist in the enum at `runner.rs:168-222`
2. `PluginTab::build_task_config()` always returns `None` (line 371)
3. `build_current_task()` doesn't handle `Tab::Plugin`
4. `DiscoveredPlugin` type mismatch with `PluginInfo`

**Fix (in order)**:
1. Add `PluginsLoaded(Vec<PluginInfo>)` variant to `TaskResult` enum in `runner.rs`
2. Implement actual task config building in `task_management.rs:368-373`
3. Add `Tab::Plugin` arm to `build_current_task()` in `mod.rs`
4. Add `From<DiscoveredPlugin> for PluginInfo` impl or conversion function

**Note**: This is currently blocked behind pyo3 API fix in `slapper-plugin`. Fix the pyo3 errors first, then this compile error will surface.

---

### 1.6: Auth Pattern - Replace `unwrap_u8()`

**Files**: 6 locations (all under `crates/slapper/src/`)
- `tool/protocol/rest.rs:137`
- `tool/protocol/ai_routes.rs:40`
- `tool/protocol/agent_routes.rs:279`
- `tool/protocol/openai/handlers.rs:26`
- `tool/protocol/mcp/auth.rs:11`
- `tool/protocol/grpc.rs:27`

**Verified**: Yes, all 6 locations confirmed using `.unwrap_u8() == 1` pattern.

**Current code** (all 6 locations):
```rust
Some(v) if key.as_bytes().ct_eq(v.as_bytes()).unwrap_u8() == 1 => Ok(()),
```

**Fix**:
```rust
Some(v) if bool::from(key.as_bytes().ct_eq(v.as_bytes())) => Ok(()),
```

**Why**: `ConstantTimeEq::ct_eq()` returns `Choice`. Calling `.unwrap_u8()` degrades it to `u8`, enabling `== 1` branching which could enable side-channel attacks. Use `bool::from()` instead.

**After fixing**: Run `cargo check --lib -p slapper --features rest-api,ai-integration` to verify all compile.

---

### 1.7: Silent Data Loss in Serialization

**Files**:
- `crates/slapper/src/tool/response.rs:260-262`
- `crates/slapper/src/distributed/worker.rs:172`

**Verified**: Yes, both locations confirmed.

**Current code** (`response.rs:260`):
```rust
pub fn to_json_line(&self) -> String {
    serde_json::to_string(self).unwrap_or_default() + "\n"
}
```

If serialization fails, returns `"\n"` (empty string + newline). Finding data is lost silently.

**Current code** (`worker.rs:172`):
```rust
.map(|o| serde_json::to_string(o).unwrap_or_default())
```

Same pattern — task appears to succeed with empty output.

**Fix** (`response.rs`):
```rust
pub fn to_json_line(&self) -> Result<String, serde_json::Error> {
    serde_json::to_string(self).map(|s| s + "\n")
}
```

Update all callers to handle `Result`. For `worker.rs`, propagate the error instead of defaulting to empty string.

---

### 1.8: TOCTOU Race Condition in Config Loading

**File**: `crates/slapper/src/config/loader.rs:19-27,46-63`

**Verified**: Yes, both `load_config()` and `load_scope()` use separate `exists()` check then `read()`.

**Current pattern** (`loader.rs:19-27`):
```rust
if !path.exists() {                                          // Check
    return Ok(SlapperConfig::default());
}
let content = fs::read_to_string(&path)                     // Use (separate operation)
    .with_context(|| format!("Failed to read config file: {:?}", path))?;
```

**Fix**: Eliminate the separate existence check — attempt read directly and handle "not found" as "use defaults":
```rust
let canonical_path = path.canonicalize().map_err(|e| {
    anyhow::anyhow!("Failed to canonicalize config path '{}': {}", path.display(), e)
})?;
let content = fs::read_to_string(&canonical_path)
    .with_context(|| format!("Failed to read config file: {:?}", canonical_path))?;
```

Apply same fix to `load_scope()` at lines 46-63.

---

## Wave 2: HIGH Priority Security (Parallel within groups)

### Sub-Agent Instructions for Wave 2

Wave 2 can start after Wave 1 is complete. Items within Wave 2 are independent and can be parallelized. All paths relative to `crates/slapper/src/` unless otherwise noted.

---

### 2.1: Ruby Sandbox Escape

**File**: `crates/slapper-ruby/src/api.rs:16-574`

**Verified**: Yes, dangerous APIs fully exposed. `register_api()` at lines 16-28 exposes `Slapper::HTTP`, `Slapper::Scanner`, `Slapper::Fuzzer`, `Slapper::Metasploit`, `Slapper::Encoder`, `Slapper::Session` modules. These are registered as Ruby module functions BEFORE any plugin validation runs.

**Issue**: Plugins have unrestricted network access via `Slapper::HTTP.get(...)`, `Slapper::Scanner.tcp_connect(...)`, `Slapper::Metasploit.execute_module(...)`, etc. Pattern detection in `bridge.rs:34` only checks source patterns, not what registered APIs provide.

**Recommended Fix**: Remove HTTP, Scanner, Fuzzer, Metasploit modules — keep only safe reporting methods. This is a design decision — consider if Ruby plugins should be restricted to read-only operations.

---

### 2.2: Python Suspicious Pattern Detection Gaps

**File**: `crates/slapper-plugin/src/python.rs:16-27`

**Verified**: Yes, 8 patterns currently checked. The following dangerous patterns are missing.

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

---

### 2.3: Ruby Pattern Detection Gaps

**File**: `crates/slapper-ruby/src/bridge.rs:13-31`

**Verified**: Yes, 15 patterns currently checked. Missing patterns:

```rust
("Kernel.exec", regex::Regex::new(r"Kernel\.exec").unwrap()),
("open alias", regex::Regex::new(r"\bopen\b").unwrap()),
("eval without parens", regex::Regex::new(r"\beval\b").unwrap()),
```

**Why these matter**:
- `Kernel.exec("cmd")` or `Kernel.exec "cmd"` — command execution not caught by `\bexec\(` pattern
- Ruby's `open("|cmd")` executes commands — backtick pattern catches `` ` `` but not `open("|...")`
- `eval "code"` without parens bypasses `\beval\(` pattern; also `instance_eval`, `class_eval`, `module_eval` not checked

---

### 2.4: TLS Bypass Without Warning Logging

**Verified**: Yes, but needs refinement. Two categories:

**Category A — Hardcoded `true` (7 locations, MUST fix)**:
These always disable TLS verification with no user control:
- `scanner/cms/mod.rs:82`
- `scanner/cms/joomla.rs:31`
- `scanner/cms/drupal.rs:31`
- `scanner/templates/executor.rs:25`
- `waf/detector/compare.rs:17`
- `stress/http.rs:153`
- `proxy/health.rs:100`

**Additional location not in original plans**:
- `recon/ssl_audit.rs:146` — also hardcoded `true` without warning

**Category B — Flag-controlled (4 locations, LOWER priority)**:
These use user-controlled flags (`insecure`, `verify_tls`) — user opted in:
- `fuzzer/engine/advanced.rs:18` — uses `args.common.insecure`
- `fuzzer/engine/core.rs:178` — uses `args.common.insecure`
- `scanner/endpoints.rs:686` — uses `!config.verify_tls`
- `loadtest/runner.rs:232` — uses `self.insecure`

**Fix for Category A**: Use centralized `create_insecure_http_client()` from `utils/http.rs` which already logs warnings. Replace direct `danger_accept_invalid_certs(true)` calls.

**Fix for Category B**: Add `tracing::warn!()` before the flag check:
```rust
if insecure_flag {
    tracing::warn!(
        "TLS certificate verification disabled. This is insecure and should only \
         be used in isolated testing environments."
    );
    client = client.danger_accept_invalid_certs(true);
}
```

---

### 2.5: Scope Validation Missing in REST API

**File**: `crates/slapper/src/tool/protocol/rest.rs:301-339`

**Verified**: Yes, `execute_tool` handler performs auth check and rate limiting but has zero scope validation. Target from user payload is directly dispatched. No scope-related imports exist in the file.

**Fix**: Add scope check before dispatch:
```rust
let target_url = &payload.target;
if let Some(ref scope) = state.scope {
    if !scope.is_allowed(target_url) {
        return Err(AppError::ScopeViolation(target_url.clone()));
    }
}
```

**Context**: `Scope` and `ScopeRule` are defined in `config/scope.rs`. `AppError` needs a `ScopeViolation` variant added.

---

### 2.6: Scope Validation Missing in MCP Handlers

**File**: `crates/slapper/src/tool/protocol/mcp/handlers.rs:252-337`

**Verified**: Yes, `handle_tools_call` has zero scope validation. Target constructed from user arguments and dispatched without scope checks.

**Fix**: Add scope check before tool execution. Pattern is same as 2.5.

---

### 2.7: Scope Validation Missing in OpenAI Handlers

**File**: `crates/slapper/src/tool/protocol/openai/handlers.rs:121-207`

**Verified**: Yes, `non_streaming_response` extracts target from user query and dispatches without scope checks.

**Fix**: Add scope check on extracted targets before execution.

---

### 2.8: Credential Exposure in Proxy URL

**Files**:
- `crates/slapper/src/proxy/config.rs:132-147` — `to_url()` embeds `pass.expose_secret()` in URL
- `crates/slapper/src/proxy/pool.rs:173-188` — uses `to_url()` as DashMap key and in logging

**Verified**: Yes, `to_url()` returns credentials in plaintext (`socks5://user:secret@10.0.0.1:9050`). Despite `password` being `SensitiveString`, `expose_secret()` embeds the credential.

**Additional affected locations** (broader than original plan):
- `proxy/health.rs:48` — stored in `HealthCheckResult.proxy_url`
- `proxy/rotator.rs:99,129,166,386` — used for tracking
- `commands/handlers/stress.rs:93` — `println!("  - {}", proxy.to_url())` — **prints credentials to stdout**

**Fix**: Add `to_log_key()` method for safe logging/display:
```rust
pub fn to_log_key(&self) -> String {
    match (&self.username, &self.password) {
        (Some(user), Some(_)) => format!("{}://{}:***@{}:{}", scheme, user, self.address, self.port),
        _ => self.to_url(),
    }
}
```

Replace all non-connection uses of `to_url()` with `to_log_key()`. Keep `to_url()` only for actual proxy connections.

---

### 2.9: ai_client Field Never Used in Agent

**File**: `crates/slapper/src/agent/mod.rs:77-78` (declaration), 107-108 (init), 119-121 (setter)

**Verified**: Yes, `ai_client: Option<AiClient>` is declared, initialized to `None`, settable via `with_ai_client()`, but **never used in production code**. Only referenced in tests (line 366). The `run()`, `execute_scan()`, `handle_findings()`, `trigger_scan()` methods never reference `self.ai_client`.

**Fix**: Integrate `ai_client` into scan workflow in `handle_findings()`:
```rust
#[cfg(feature = "ai-integration")]
if let Some(ref client) = self.ai_client {
    let analysis = client.analyze_findings_typed(&finding_values).await?;
    // Use analysis to determine alert severity or triage findings
}
```

---

### 2.10: Formula Injection Unicode Bypass Fix

**File**: `crates/slapper/src/output/escape.rs:16-35`

**Verified**: Yes, with nuance. The `escape_csv` function checks for ASCII formula chars (`=`, `+`, `-`, `@`, `\t`, `\r`) but not fullwidth Unicode variants (U+FF1D, U+FF0B, U+FF0D, U+FF20). There IS an implicit partial defense: `first_char_is_control` checks `!c.is_ascii()` which is true for fullwidth chars, causing them to get quoted. However, this protection relies solely on CSV quoting, which is not universally safe across all spreadsheet implementations (especially East Asian locale versions). The variable name `first_char_is_control` is misleading — it actually checks "non-ASCII first character".

**Fix**: Normalize input to NFKC form before checking, making fullwidth-to-ASCII normalization explicit:
```rust
use unicode_normalization::UnicodeNormalization;
let normalized: String = s.nfkc().collect();
// Then check normalized string for formula chars
```

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

### Sub-Agent Instructions for Wave 3

Wave 3 depends on Waves 1-2 being complete. Large file splits are independent of each other and can be done in parallel by different agents. All paths relative to `crates/slapper/src/`.

---

### 3.1: TUI Tab Dispatching Duplication

**Files**:
- `tui/app/mod.rs` (**899 lines** — was 967, reduced by prior submodule extraction)
- `tui/tabs/mod.rs` (**655 lines** — was 859, reduced)

**Verified**: Yes, 29 tab variants with repetitive match statements. `mod.rs` has 6 large `match self.current_tab` statements. `tabs/mod.rs` has extensive match arms for both immutable (lines 447-475) and mutable (lines 509-564) access patterns.

**Fix**: Introduce `enum_dispatch` crate pattern:
```toml
# Cargo.toml
enum_dispatch = "0.3"
```

Define a `TabDispatch` trait with common methods, derive `enum_dispatch` for the `Tab` enum. This eliminates manual match dispatching.

**Alternative** (less dependency): Use a macro to generate match arms.

---

### 3.2: TUI Architecture Refactoring

**Files to split** (all line counts verified):
- `tui/app/mod.rs` (**899 lines**) → Extract task management, state fields to existing submodules
- `tui/tabs/mod.rs` (**655 lines**) → Extract traits to `traits.rs`
- `tui/tabs/settings.rs` (**798 lines**) → Split to `settings/main.rs`, `settings/http.rs`, `settings/proxy.rs`
- `tui/tabs/packet.rs` (**743 lines**) → Split to `packet/capture.rs`, `packet/send.rs`
- `tui/tabs/fuzz.rs` (**698 lines**) → Split to `fuzz/config.rs`, `fuzz/results.rs`

**Guidelines for splitting**:
- Keep the parent module as `mod.rs` with re-exports
- Move private implementations to submodules
- Ensure all existing tests pass after split
- Run `cargo clippy --lib -p slapper` after each split

---

### 3.3: MCP Handlers Large File Refactoring

**File**: `tool/protocol/mcp/handlers.rs` (**1069 lines**)

**Verified**: Yes, still over 1000 lines.

**Split Plan**:
- `handlers_server.rs` (~250 lines) — McpServer struct, constructor, router setup
- `handlers_request.rs` (~700 lines) — Request handlers, tool handlers
- `handlers_helpers.rs` (~150 lines) — Helper functions, utility methods

**After splitting**: Update `tool/protocol/mcp/mod.rs` to declare new submodules.

---

### 3.4: recon/dependency_scan Large File

**File**: `recon/dependency_scan.rs` (**1051 lines**, verified exact match)

**Split Plan**: Split by ecosystem into `recon/dependency/` subdirectory:
- `npm.rs` — Node.js/NPM dependency scanning
- `cargo.rs` — Rust/Cargo dependency scanning
- `go.rs` — Go module scanning
- `ruby.rs` — Ruby gem scanning
- `mod.rs` — Shared types, main scan orchestration

---

### 3.5: Plugin System Fixes (PLG-007 to PLG-018)

**Verified partially**. Some items already addressed:

| Issue | File | Status | Fix |
|-------|------|--------|-----|
| PLG-007: Credential exposure | `lib.rs`, `api.rs` | Not fixed | Add `filtered_config()` |
| PLG-008: Missing Mutex | `python.rs:58-63` | Not fixed | `PythonPluginManager.plugins` is `Vec<LoadedPlugin>` with no Mutex — caller must handle sync |
| PLG-009: TUI task config | `task_management.rs:368-373` | Not fixed | Implement actual task spawning (linked to Wave 1 item 1.5) |
| PLG-010: Type mismatch | `plugin.rs` vs `tabs/plugin.rs` | Not fixed | Add `From` impl |
| PLG-011: Missing lifecycle | `lib.rs` | Not fixed | Add `init()`, `shutdown()` |
| PLG-012: Missing health check | `lib.rs` | Not fixed | Add `health_check()` |
| PLG-013: O(n) registry | `lib.rs:145-146` | Not fixed | `PluginRegistry::get()` uses `self.plugins.iter().find()` — change to `HashMap<String, Arc<dyn Plugin>>` |
| PLG-014: PyO3 API mixing | `python.rs` | **Partially addressed** | `Python::attach` migration already done. Remaining: standardize `Py<T>` vs `Bound<T>` usage |
| PLG-015: Case-insensitive | `python.rs`, `bridge.rs` | Not fixed | Add `(?i)` flag to regex patterns |
| PLG-016: Hot reload | `lib.rs` | Not fixed | Add `reload_plugin()` |
| PLG-017: Plugin priority | `lib.rs` | Not fixed | Add `priority` field |
| PLG-018: Namespace isolation | `python.rs:226-258` | Not fixed | Prefix class names |

---

### 3.6: CircuitBreakerRegistry Dead Code or Utilization

**File**: `utils/circuit_breaker.rs` (282 lines)

**Verified**: Yes, `CircuitBreakerRegistry` at line 125 is dead code. It is only referenced in its own definition and the `utils/mod.rs` re-export. No other file instantiates or uses it. Each `AiClient` creates its own `CircuitBreaker` directly.

**Fix**: Either:
1. Remove `CircuitBreakerRegistry` as dead code, OR
2. Utilize it for multi-provider AI support (register one breaker per provider)

---

## Wave 4: Performance Optimization (Parallel)

### Sub-Agent Instructions for Wave 4

Wave 4 can start after Wave 3 (refactoring). Performance changes should be benchmarked before and after. All paths relative to `crates/slapper/src/`.

---

### 4.1: HashMap -> FxHashMap Migration

**Files**: 55 files with 140 references to `std::collections::HashMap`

**Verified**: The four highest-priority hot paths all confirmed using `std::collections::HashMap`:
- `waf/detector/mod.rs:19-20` — signatures storage
- `waf/detector/detect.rs:159` — detection loop
- `scanner/templates/models.rs:8` — templates
- `fuzzer/chain.rs:5` — fuzz chains
- `proxy/intercept/rules.rs:7` — rules

**Note**: Some hot-path modules have already been migrated to FxHashMap (`fuzzer/state.rs`, `fuzzer/payloads/mod.rs`, `scanner/templates/matcher.rs`). Follow existing pattern.

**Fix**:
```rust
// Before:
use std::collections::HashMap;

// After:
use rustc_hash::FxHashMap as HashMap;
// OR (more explicit):
use rustc_hash::FxHashMap;
```

**Strategy**: Migrate hot paths first (listed above), then widen to other performance-sensitive areas.

---

### 4.2: to_lowercase() Optimization

**Verified locations**:
- `vuln/triage.rs:48,52` — 4+ `to_lowercase()` calls per iteration on same strings
- `recon/dependency_scan.rs:855-857` — 4 calls to `title.to_lowercase()` on same string
- `ai/planner.rs:397,411` — nested loop with repeated lowercasing

**Fix**: Cache lowercase once per string:
```rust
let title_lower = title.to_lowercase();
let description_lower = description.to_lowercase();
// Then use title_lower / description_lower in all comparisons
```

---

### 4.3: std::Mutex -> parking_lot::Mutex

**Verified locations**:
- `scanner/ports/spoofed.rs:48` — `std::sync::OnceLock<std::sync::Mutex<std::fs::File>>`
- `stress/metrics.rs:112` — `Arc<std::sync::Mutex<Instant>>`
- `tui/workers/recon.rs:112` — `Arc<std::sync::Mutex<String>>` (used in polling loop)

**Fix**: Replace with `parking_lot::Mutex`. Note: `parking_lot::Mutex::lock()` returns `MutexGuard` directly (NOT `Result`), so remove `.unwrap()` calls.

---

### 4.4: TimingAnalyzer Lock-Free Redesign

**Files**:
- `fuzzer/detection/analyzer.rs` — `TimingAnalyzer` struct with `&mut self` methods on `samples: Vec<f64>`
- `fuzzer/engine/utils.rs:198-199` — **Bottleneck site** where `Arc<Mutex<TimingAnalyzer>>` is locked for every fuzzer request

**Verified**: Yes, every fuzzer request acquires the tokio Mutex to call `record()`, serializing all concurrent fuzzing tasks. The `record()` method takes `&mut self` because it mutates `self.samples: Vec<f64>`.

**Fix**: Use `crossbeam::queue::SegQueue` + atomic stats:
```rust
pub struct TimingAnalyzer {
    sample_queue: SegQueue<f64>,  // Lock-free queue
    total_requests: AtomicU64,
    // ... atomics for other stats
}
```

**Note**: `crossbeam` is already in `Cargo.toml` — no new dependencies needed. This significantly changes statistics collection — test with concurrent workloads.

---

### 4.5: RateLimiter DashMap Conversion

**File**: `tool/ratelimit.rs:76-79`

**Verified**: Yes, uses `RwLock<HashMap<String, TokenBucket>>` at line 76. Already uses `parking_lot::RwLock` (imported at line 6), not `std::sync::RwLock`, which is an improvement. However, `DashMap` would still be more efficient for high-concurrency token bucket access.

**Current code**:
```rust
use parking_lot::RwLock;
// ...
tokens: RwLock<HashMap<String, TokenBucket>>,
```

**Fix**:
```rust
pub struct RateLimiter {
    config: RateLimitConfig,
    tokens: DashMap<String, TokenBucket>,  // Sharded locking
    // ...
}
```

---

### 4.6: Regex Caching in ChainExecutor

**File**: `fuzzer/chain.rs:241,307`

**Verified**: Yes, both locations confirmed creating fresh `RegexBuilder` on every call.

**Fix**: Add cache to `ChainExecutor`:
```rust
pub struct ChainExecutor {
    // ...
    regex_cache: FxHashMap<String, regex::Regex>,
}
```

Implement `get_or_compile()` helper that checks cache first.

---

### 4.7: tokio::sync::watch for Progress Updates

**File**: `tui/workers/recon.rs:112-141`

**Verified**: Yes, `tui/workers/recon.rs` uses `Arc<Mutex<String>>` with a polling loop (200ms sleep). Other workers use `mpsc` channels which is acceptable.

**Fix**: Replace the polling pattern in recon worker with `watch` channel:
```rust
let (tx, rx) = watch::channel::<String>("initial".to_string());
// In worker:
tx.send("Processing step 1".to_string())?;
// In UI:
while rx.changed().await.is_ok() {
    println!("Progress: {}", *rx.borrow());
}
```

---

### 4.8: String Allocation Optimizations

**Verified locations**:
- `scanner/fingerprint.rs:327` — `format!("{}:{}", host, port)` per port
- `scanner/udp_fingerprint.rs:139` — `format!("{}:{}", host, port).parse::<SocketAddr>()` — allocates string just to parse

**Fix for udp_fingerprint.rs**: Use `std::net::SocketAddr::new()` with `IpAddr::from_str()` directly, avoiding string allocation:
```rust
let ip: IpAddr = host.parse()?;
let addr = SocketAddr::new(ip, port);
```

---

## Wave 5: Agent System Improvements (Parallel)

### Sub-Agent Instructions for Wave 5

Wave 5 depends on Wave 1 being complete. All 9 items are independent and can be done in parallel. All paths relative to `crates/slapper/src/`.

---

### 5.1: Graceful Shutdown Not Implemented

**File**: `agent/mod.rs:129-159`

**Verified**: Yes, `run()` uses `self.running` (Arc<RwLock<bool>>) with polling loop. No SIGTERM/SIGINT signal handlers. No `CancellationToken` or `tokio::signal::ctrl_c()` usage.

**Fix**: Add signal handlers with `CancellationToken`:
```rust
use tokio_util::sync::CancellationToken;

pub async fn run(&mut self) -> Result<()> {
    let token = CancellationToken::new();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.unwrap();
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

---

### 5.2: Event Loop Swallows Errors

**File**: `agent/mod.rs:145-147`

**Verified**: Yes, and worse than described. Line 145:
```rust
if self.process_scheduled_scans().await.is_ok() {
    tracing::debug!("Processed scheduled scans");
}
```
If `is_ok()` returns false (error occurred), **nothing happens** — no logging, no error handling. Error is completely discarded.

**Fix**:
```rust
if let Err(e) = self.process_scheduled_scans().await {
    tracing::warn!(error = %e, "Scheduled scan failed");
} else {
    tracing::debug!("Processed scheduled scans");
}
```

---

### 5.3: Dedup Key Collision in AlertRouter

**File**: `agent/alerts/routing.rs:237-244`

**Verified**: Yes, `make_dedup_key()` reads:
```rust
fn make_dedup_key(&self, alert: &Alert) -> String {
    format!("{}:{}:{}", alert.target, alert.severity.as_str(), alert.title)
}
```
No `finding_ids` hash included. Alerts with same target/severity/title but different findings produce same key.

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

---

### 5.4: Lock Poisoning in TargetPortfolio

**File**: `agent/portfolio.rs:221,225,229,233` (and more — lines 237, 241, 248, 253, 254, 259, 260, 266, 270)

**Verified**: Yes, uses `std::sync::RwLock` (imported at line 9) with `.unwrap()` on lock acquisitions. Will panic on lock poisoning.

**Fix**: Migrate to `parking_lot::RwLock` which does not panic on poison:
```rust
use parking_lot::RwLock;
pub struct TargetPortfolio {
    data: Arc<RwLock<PortfolioData>>,
}
```

Note: `parking_lot::RwLock::read()` and `write()` return guards directly, not `Result`. Remove all `.unwrap()` calls on lock acquisitions.

---

### 5.5: Severity Filtering Only Handles Critical

**File**: `agent/mod.rs:268-296`

**Verified**: Yes, line 268-270:
```rust
let critical_findings: Vec<_> = findings.iter()
    .filter(|f| matches!(f.severity, crate::tool::response::ResponseSeverity::Critical))
    .collect();
```
Only `Critical` findings trigger alerts. High, Medium, Low, and Info are completely ignored.

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

---

### 5.6: Path Validation in LongitudinalMemory

**File**: `agent/memory.rs:107-123`

**Verified**: Yes, lines 111-114 use string replacement:
```rust
let safe_name = target
    .replace("://", "_")
    .replace("/", "_")
    .replace(":", "_");
```

**Clarification**: The actual risk is **filename collision** (different target URLs could produce same filename), not path traversal. The module creates filenames FROM target strings, not from user-provided paths. The hashing at lines 116-118 helps somewhat with uniqueness.

**Fix**: Use `crate::utils::validation::validate_path()` for the constructed path, and consider using a full hash of the target URL as the filename to avoid collisions.

---

### 5.7: Timezone Parsing Only Numeric

**File**: `agent/portfolio.rs:78-96`

**Verified**: Yes, lines 79-85:
```rust
let local = match &self.timezone[..] {
    "UTC" => time.hour() as i32,
    _ => {
        let offset_hours: i64 = self.timezone.trim().parse().unwrap_or(0);
        ...
    }
};
```
Named timezones (`"America/New_York"`, `"Europe/London"`) silently fall back to offset 0 (UTC).

**Fix**: Add `chrono-tz` support:
```rust
use chrono_tz::Tz;
let tz: Tz = self.timezone.parse().unwrap_or(chrono_tz::UTC);
let local = time.with_timezone(&tz);
```

**Note**: Requires adding `chrono-tz` to Cargo.toml.

---

### 5.8: Missing Error Propagation

**File**: `agent/mod.rs:267-296`

**Verified**: Yes, `handle_findings()` returns `()` (unit), not `Result<()>`. Errors from `self.alert_router.send()` are logged but never propagated.

**Fix**: Change signature to return `Result<()>` and propagate errors to caller.

---

### 5.9: TOCTOU Race in AlertRouter Dedup

**File**: `agent/alerts/routing.rs:45-75`

**Verified**: Yes, three separate lock acquisitions:
1. Lock `recent_alerts` to check size, cleanup (lines 47-51)
2. Lock `recent_alerts` to check dedup key (lines 56-62)
3. Lock `recent_alerts` to insert dedup key (lines 71-72)

Between step 2 (check) and step 3 (insert), another concurrent call could proceed with same key.

**Fix**: Use `DashMap` for atomic check-and-insert, or combine steps 2 and 3 into a single lock scope.

---

## Wave 6: REST API & External Integrations (Parallel)

### Sub-Agent Instructions for Wave 6

Wave 6 depends on Wave 1 being complete. All items are independent. Waves 5 and 6 can run in parallel with each other. All paths relative to `crates/slapper/src/` unless otherwise noted.

---

### 6.1: REST API TLS Configuration

**File**: `tool/protocol/rest.rs:16-35`

**Verified**: Yes, `RestState` has fields `registry`, `dispatcher`, `api_key`, `rate_limiter` — no TLS fields. `create_router()` creates plain HTTP router. OpenAPI spec hardcodes `"url": "http://127.0.0.1:8080"`.

**Fix**: Import `TlsConfig` from `crate::distributed`, add TLS fields to `RestState`, update `create_router()` to support TLS.

---

### 6.2: REST API Rate Limiting Improvements

**File**: `tool/ratelimit.rs:11-16`

**Verified**: Basic per-client token bucket rate limiting **already exists** (keyed by target string). What is MISSING:
- Per-endpoint rate limiting (all endpoints share same config)
- Global rate limit (no total-request cap)
- IP-based limiting (limits keyed by target, not client IP)

**Fix**: Add `RateLimitConfig` fields for per-endpoint limits, global cap, and IP-based tracking.

---

### 6.3: REST API WebSocket Support

**File**: `tool/protocol/mcp/streaming.rs:1-19`

**Verified**: Yes, file only contains `StreamEvent` for SSE. No WebSocket code. The `websocket` feature exists in Cargo.toml with `tokio-tungstenite` but is not integrated into REST/MCP protocol layer.

**Fix**: Add WebSocket endpoint to REST router using existing `tokio-tungstenite` dependency.

---

### 6.4: UDP IP Spoofing Integration

**File**: `stress/udp.rs:19-117`

**Verified**: Yes, `raw_udp` module (lines 20-117) defines `build_udp_packet()`, checksum functions — complete raw UDP packet builder with IP spoofing. But `run_udp_flood()` (line 120) uses standard `UdpSocket` and never calls `raw_udp::*`. Zero references to `raw_udp::` from outside the module.

**Fix**: Integrate with `--spoof-ip` flag when `stress-testing` feature enabled. When spoof enabled, use `raw_udp::build_udp_packet()` + raw socket instead of `UdpSocket`.

---

### 6.5: Ruby API block_on Deadlock Risk

**File**: `crates/slapper-ruby/src/api.rs` (1073 lines)

**Verified**: Yes, **35 instances** of `rt.block_on()` confirmed. Classic deadlock risk — if Ruby plugin callback invoked from tokio async context, calling `block_on()` on same runtime handle panics.

**Fix**: Use dedicated blocking thread pool:
```rust
static ASYNC_POOL: std::sync::OnceLock<tokio::runtime::Runtime> =
    std::sync::OnceLock::new();

fn get_blocking_runtime() -> &'static tokio::runtime::Runtime {
    ASYNC_POOL.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("ruby-async")
            .build()
            .unwrap()
    })
}
```

Then replace `get_runtime().block_on(...)` with `get_blocking_runtime().block_on(...)`.

---

## Wave 7: Dependency Updates (Sequential - Highest Risk)

### Sub-Agent Instructions for Wave 7

Wave 7 must be LAST — after all other waves. These are highest risk because they change fundamental dependencies. Test thoroughly after each update. Must be done sequentially.

---

### 7.1: Axum 0.7.x -> 0.8.x

**Current version**: `axum = "0.7"` (workspace Cargo.toml)

**Breaking Changes**:
- Path syntax `/:param` → `/{param}`
- `#[async_trait]` removed
- `Option<T>` extractor changes
- Update all route definitions and handler signatures

---

### 7.2: Tonic 0.12.x -> 0.14.x

**Current version**: `tonic = "0.12"` (workspace Cargo.toml)

**Breaking Changes**:
- prost extracted to separate crate
- `BoxBody` removed
- Update all gRPC service definitions

**Note**: Must be done WITH axum update (7.1) since they share some transitive dependencies.

---

## Wave 8: TUI Usability Improvements (Parallel)

### Sub-Agent Instructions for Wave 8

Wave 8 can start after Wave 1 (foundation fixes). Independent of Waves 3-7. All items are independent and can be done in parallel. All paths relative to `crates/slapper/src/`.

---

### 8.1: Global Search (Ctrl+F)

**Files**: `tui/search.rs` (new), `tui/app/runner.rs`, `tui/ui.rs`

**Verified**: Search infrastructure partially exists. `search_query` field in `App` struct, search input handled in `runner.rs:422`. But no dedicated global search module — search is tab-specific.

**Fix**: Create `tui/search.rs` with global search that works across all tabs. Add Ctrl+F keybinding.

---

### 8.2: Clipboard/Copy-Paste Support

**Files**: `tui/utils/clipboard.rs` (new), `tui/components/selection.rs` (new)

**Verified**: No clipboard functionality exists anywhere in TUI code.

**Recommendation**: Use `arboard` crate (pure Rust clipboard access).

---

### 8.3: Pause/Resume (Ctrl+Z/Ctrl+Y)

**Files**: `tui/workers/pause.rs` (new), `tui/app/mod.rs`, `tui/app/runner.rs`

**Verified**: Help text at `help.rs:180` shows `"Space" => "Pause/Resume"` but no implementation exists for active scan pause/resume. Note: `Tab::Resume` is for session file resume (unrelated).

**Fix**: Create `tui/workers/pause.rs` with pause/resume mechanism for running scan tasks. Wire into event loop.

---

### 8.4: Tab Overflow Display

**File**: `tui/ui.rs:255-272`

**Verified**: Yes, `draw_tabs()` renders ALL 29 tabs with no overflow handling. Will overflow on narrow terminals.

**Fix**: Add horizontal scroll or tab groups. Consider wrapping with `Tabs::new(titles).block(block)`.

---

### 8.5: Input Validation Visual Feedback

**File**: `tui/components/input.rs:8-12`

**Verified**: `ValidationResult` IS used (in `scan_ports.rs:153` and `scan_ports.rs:163`). Validation methods exist: `validate_url()`, `validate_ip()`, `validate_port()`, etc. What's missing is **visual feedback** — red border for invalid input is not implemented.

**Fix**: Implement visual rendering of validation state (red border for invalid, green for valid). Add validation state tracking to input component rendering.

---

### 8.6: Session Auto-Persistence

**Files**: `tui/session.rs` (new), state management

**Verified**: No session auto-persistence mechanism exists.

**Fix**: Create `tui/session.rs` with periodic state serialization. Save on exit, restore on startup.

---

### 8.7: Theme System (Dark/Light)

**File**: `tui/theme.rs` (new)

**Verified**: No theme system exists. Hardcoded `Color::*` literals used throughout TUI code.

**Fix**: Create theme struct with all color definitions. Replace inline `Color::*` with theme references. Add dark/light presets.

---

### 8.8: Keyboard Shortcuts Inline Display

**File**: `tui/ui.rs`

**Verified**: Help exists in `tui/help.rs` with full command listing. Status bar exists. Issue is discoverability.

**Fix**: Show contextual keyboard hints inline (e.g., "[Ctrl+F] Search" in status bar).

---

### 8.9: Tab Bookmarks/Favorites

**Verified**: No bookmark/favorite functionality exists. Grep for "bookmark" and "favorite" returns zero results.

**Fix**: Add bookmark persistence (save to config), keyboard shortcut to toggle bookmark, display bookmarked tabs prominently.

---

## Wave 9: Plugin Architecture Unification (Long Term)

### Sub-Agent Instructions for Wave 9

This is a long-term wave. Lower priority than Waves 1-8. All paths relative to `crates/`.

---

### 9.1: Enhanced PluginBackend Trait

**Current State**: A basic unified `Plugin` trait already exists at `slapper-plugin/src/lib.rs:98-113`:
```rust
pub trait Plugin: Send + Sync {
    fn info(&self) -> &PluginInfo;
    fn language(&self) -> PluginLanguage;
    fn list_checks(&self) -> Vec<PluginCheck>;
    async fn run_check(&self, check_name: &str, target: &str) -> Result<PluginResult>;
    async fn run(&self, target: &str, config: &PluginConfig) -> Result<PluginResult>;
}
```

**What's needed**: Add lifecycle methods, health checking, and priority support:
```rust
fn init(&self) -> Result<()>;
fn shutdown(&self) -> Result<()>;
fn health_check(&self) -> Result<HealthStatus>;
fn priority(&self) -> u32;
```

---

### 9.2: Shared Security Patterns Module

**Current State**: Not implemented. Python patterns in `slapper-plugin/src/python.rs`, Ruby patterns in `slapper-ruby/src/bridge.rs`.

**Fix**: Consolidate into `slapper-plugin/src/security.rs`.

---

### 9.3: Move Ruby Loader Into Plugin Crate

**Current State**: `slapper-ruby/src/loader.rs` exists in separate crate.

**Fix**: Consolidate into `slapper-plugin`.

---

### 9.4: Plugin Lifecycle Features

**Current State**: Not implemented. `Plugin` trait has no lifecycle methods.

**Fix**: Add `init()`, `activate()`, `deactivate()`, `health_check()` to Plugin trait.

---

### 9.5: Metasploit Integration Enhancement

**Current State**: Basic MSF RPC integration in `slapper-ruby/src/api.rs` (connect, disconnect, list_modules, execute_module, session management). No auto-pivoting, session persistence/caching.

**Fix**: Add auto-pivoting, session persistence/caching.

---

## Implementation Order by Parallelization

### Parallel Execution Map

```
Wave 1 (CRITICAL) ────────────────────────────────────────── Foundation
    │
    ├── Wave 2 (HIGH Security) ──────────── After Wave 1
    │
    ├── Wave 5 (Agent) ──────────────────── After Wave 1
    ├── Wave 6 (API) ────────────────────── After Wave 1
    ├── Wave 8 (TUI Features) ───────────── After Wave 1
    │
    ├── Wave 3 (Refactoring) ────────────── After Waves 1-2
    │   └── Wave 4 (Performance) ────────── After Wave 3
    │
    └── Wave 7 (Dependencies) ───────────── After ALL other waves (highest risk)
    
    Wave 9 (Plugin Unification) ─────────── Long term, anytime
```

### Sub-Agent Assignment Strategy

| Agent | Wave | Items | Dependencies |
|-------|------|-------|-------------|
| Agent 1 | Wave 1 | 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8 | None (start first) |
| Agent 2 | Wave 2 | 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 2.8, 2.9, 2.10 | Wave 1 complete |
| Agent 3 | Wave 3 | 3.1, 3.2, 3.3, 3.4, 3.5, 3.6 | Waves 1-2 complete |
| Agent 4 | Wave 4 | 4.1, 4.2, 4.3, 4.4, 4.5, 4.6, 4.7, 4.8 | Wave 3 complete |
| Agent 5 | Wave 5 | 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 5.7, 5.8, 5.9 | Wave 1 complete |
| Agent 6 | Wave 6 | 6.1, 6.2, 6.3, 6.4, 6.5 | Wave 1 complete |
| Agent 7 | Wave 8 | 8.1, 8.2, 8.3, 8.4, 8.5, 8.6, 8.7, 8.8, 8.9 | Wave 1 complete |
| Agent 8 | Wave 7 | 7.1, 7.2 | ALL other waves complete |

### Within-Wave Parallelization

| Wave | Parallel Group A | Parallel Group B |
|------|-----------------|-----------------|
| 1 | 1.1, 1.7, 1.8 (quick fixes) | 1.2, 1.3, 1.4, 1.5, 1.6, 1.9 (security fixes) |
| 2 | 2.2, 2.3, 2.4, 2.8, 2.10 (patterns/TLS) | 2.5, 2.6, 2.7 (scope validation) |
| 3 | 3.1, 3.2, 3.3, 3.4 (file splits) | 3.5, 3.6 (plugin/registry) |
| 4 | 4.1, 4.2, 4.3, 4.6, 4.8 (data structures) | 4.4, 4.5, 4.7 (async/lock-free) |
| 5 | All 5.1-5.9 independent | - |
| 6 | All 6.1-6.5 independent | - |
| 8 | All 8.1-8.9 independent | - |

---

## Verification Commands

Before starting any work:
```bash
cargo test --lib -p slapper
cargo clippy --lib -p slapper
cargo test --lib -p slapper -- --list 2>/dev/null | wc -l  # Expected: 1148+
find crates/slapper/src -name '*.rs' | wc -l               # Expected: 470+
```

After each wave:
```bash
cargo test --lib -p slapper
cargo clippy --lib -p slapper
```

For feature-gated changes:
```bash
cargo check --lib -p slapper --features rest-api
cargo check --lib -p slapper --features rest-api,ai-integration
cargo check --lib -p slapper --features python-plugins
```

---

## Notes for Future Agents

1. **Wave 1 items are foundational** — many other items depend on these being fixed first.

2. **Wave 7 (Dependencies) is highest risk** — test thoroughly after axum/tonic migration. Do this LAST.

3. **Plugin timeout (1.2)** cannot forcibly terminate threads — timeout only prevents waiting indefinitely. Document this limitation.

4. **Ruby sandbox (2.1)** is a design decision — consider if Ruby plugins should be restricted to read-only operations.

5. **TimingAnalyzer redesign (4.4)** significantly changes statistics collection — test with concurrent workloads.

6. **crossbeam is already in Cargo.toml** — no new dependencies needed for SegQueue.

7. **Feature-gated tabs** need BOTH `#[cfg(feature = "...")]` AND `#[cfg(not(feature = "..."))]` arms — always.

8. **WAF detection patterns** — verify `_lower` field serialization compatibility after changes.

9. **TUI Plugin Tab (1.5)** is blocked behind pyo3 API fix in `slapper-plugin`. Fix pyo3 deprecated API errors first, then the `PluginsLoaded` variant error will surface.

10. **parking_lot vs std::sync Mutex**: `parking_lot::Mutex::lock()` returns `MutexGuard` directly (NOT `Result<MutexGuard, PoisonError>`). When migrating, remove all `.unwrap()` on lock calls.

11. **Proxy credential exposure (2.8)** is broader than just the two files listed — also affects `proxy/health.rs`, `proxy/rotator.rs`, and `commands/handlers/stress.rs:93` which prints credentials to stdout.

12. **Formula injection (2.10)** has an implicit partial defense via `first_char_is_control` (checks `!c.is_ascii()`) but this is not designed for fullwidth bypass and relies on CSV quoting.

---

## Historical Context

Original plan files consolidated (no longer exist):
- `plan.md` - Codebase review, critical issues
- `plan2.md` - Code quality issues
- `plan3.md` - Security hardening, dependencies
- `plan4.md` - Security audit
- `plan5.md` - Performance optimization
- `plan6.md` - Security improvements
- `plan7.md` - Deep dive findings
- `plan8.md` - Performance deep dive
- `plan9.md` - TUI improvements
- `plan10.md` - Agent harness
- `plan11.md` - Plugin architecture
- `plan12.md` - Unified plugin architecture
- `plan13.md` - Agent harness deep dive
- `plan14.md` - TUI usability

---

*End of Consolidated Plan*
