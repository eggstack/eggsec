# Slapper Consolidated Improvement Plan

**Date**: 2026-04-24
**Status**: IN PROGRESS - all waves actively being worked
**Note**: See Implementation Status at bottom for completed items. Individual plan files (plan2.md-plan10.md) are superseded by this consolidated plan.
**Source**: Consolidated from 10 plan files (plan.md through plan10.md)

---

## Overview

This document consolidates all improvement items into a single coherent plan with wave-based parallelization for sub-agent execution. Every item has been verified against the actual codebase.

### Priority Summary

| Priority | Items | Wave |
|----------|-------|------|
| CRITICAL | 10 | Wave 1 |
| HIGH | 15 | Wave 2 |
| MEDIUM | 20 | Waves 3-4 |
| LOW | 30+ | Waves 5-9 |

---

## Wave 1: CRITICAL Security Fixes (Parallel within groups)

Items that are blocking/broken and must be fixed first. These are foundational — later waves depend on these.

### Sub-Agent Instructions for Wave 1

Run `cargo test --lib -p slapper` and `cargo clippy --lib -p slapper` before and after each change. Each fix should be independently testable. All paths are relative to `crates/slapper/src/` unless otherwise noted.

---

### 1.1: Failing Test - `git_secrets::test_scan_current_directory`

**File**: `recon/git_secrets.rs:397-403`

**Issue**: Test calls `scan_directory(".")` which is fragile — depends on CWD being a valid git repo with readable history. Fails in CI with shallow clones or detached HEAD.

**Current code**:
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

**Verification**: `cargo test --lib -p slapper -- git_secrets::test_scan_current_directory`

---

### 1.2: Plugin Timeout Not Enforced

**Files**:
- `slapper-plugin/src/lib.rs:175-188` — `run_all()` uses `join_all` with no timeout
- `slapper-plugin/src/python.rs:520-582` — `run()` trait impl, `_config` received but `timeout_secs` never used
- `slapper-ruby/src/loader.rs:330-332` — `run()` trait impl, `_config` explicitly unused
- `slapper-ruby/src/bridge.rs:144-155` — `run_plugin()` uses `rx.recv()` with no timeout

**Issue**: `PluginConfig.timeout_secs` (default 300s) is defined but never enforced anywhere in the execution chain.

**Fix - Python** (`python.rs`, in the `run()` method):
```rust
async fn run(&self, target: &str, config: &PluginConfig) -> Result<PluginResult> {
    let timeout = Duration::from_secs(config.timeout_secs);
    let result = tokio::time::timeout(timeout, async {
        // ... existing run logic ...
    }).await
    .map_err(|_| anyhow::anyhow!("Plugin execution timed out after {} seconds", config.timeout_secs))?;
    // ...
}
```

**Fix - Ruby** (`bridge.rs`, in `run_plugin()`):
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

**Note**: Plugin timeout cannot forcibly terminate threads — it only prevents waiting indefinitely.

---

### 1.3: Race Condition in Port Scanner

**Files**:
- `scanner/ports/mod.rs:507,543-572`
- `scanner/fingerprint.rs:268-272`
- `scanner/endpoints.rs:761-765`

**Issue**: Classic TOCTOU race. Two separate `Mutex` acquisitions: count is read in lock #1, released, then lock #2 is acquired to increment. Between the two locks, N concurrent tasks can all read the same count and all decide to increment, exceeding `max_results`.

**Current code pattern**:
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

**Verification**: `cargo test --lib -p slapper -- scanner::ports` and `cargo test --lib -p slapper -- scanner::endpoints`

---

### 1.4: Path Traversal in Plugin Loading

**Files**:
- `slapper-plugin/src/python.rs:155-224` — `load_plugins()` reads paths from `read_dir()` with no canonicalization
- `slapper-plugin/src/lib.rs:267-289` — `discover_plugins()` same issue
- `slapper-ruby/src/loader.rs:61-92` — `discover_plugins()` same issue

**Issue**: No `canonicalize()` call anywhere in any plugin loading path.

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
- `tui/workers/plugin.rs:11` — References missing `TaskResult::PluginsLoaded` variant
- `tui/workers/runner.rs:168-222` — `TaskResult` enum (no `PluginsLoaded` variant exists)
- `tui/app/task_management.rs:368-373` — `build_task_config()` always returns `None`
- `tui/app/mod.rs` — `build_current_task()` doesn't handle `Tab::Plugin`

**Issues**:
1. `TaskResult::PluginsLoaded` variant does not exist in the enum
2. `PluginTab::build_task_config()` always returns `None`
3. `build_current_task()` doesn't handle `Tab::Plugin`
4. `DiscoveredPlugin` type mismatch with `PluginInfo`

**Fix (in order)**:
1. Add `PluginsLoaded(Vec<PluginInfo>)` variant to `TaskResult` enum in `runner.rs`
2. Implement actual task config building in `task_management.rs:368-373`
3. Add `Tab::Plugin` arm to `build_current_task()` in `mod.rs`
4. Add `From<DiscoveredPlugin> for PluginInfo` impl or conversion function

---

### 1.6: Auth Pattern - Replace `unwrap_u8()`

**Files**: 6 locations (all under `crates/slapper/src/`)
- `tool/protocol/rest.rs:137`
- `tool/protocol/ai_routes.rs:40`
- `tool/protocol/agent_routes.rs:279`
- `tool/protocol/openai/handlers.rs:26`
- `tool/protocol/mcp/auth.rs:11`
- `tool/protocol/grpc.rs:27`

**Issue**: All 6 locations using `.unwrap_u8() == 1` pattern on `ConstantTimeEq::ct_eq()`.

**Current code**:
```rust
Some(v) if key.as_bytes().ct_eq(v.as_bytes()).unwrap_u8() == 1 => Ok(()),
```

**Fix**:
```rust
Some(v) if bool::from(key.as_bytes().ct_eq(v.as_bytes())) => Ok(()),
```

**Why**: `ConstantTimeEq::ct_eq()` returns `Choice`. Calling `.unwrap_u8()` degrades it to `u8`, enabling side-channel attacks. Use `bool::from()` instead.

**Verification**: `cargo check --lib -p slapper --features rest-api,ai-integration`

---

### 1.7: Silent Data Loss in Serialization

**Files**:
- `tool/response.rs:260-262`
- `distributed/worker.rs:172`

**Issue**: Both locations use `unwrap_or_default()` on `serde_json::to_string()`. If serialization fails, returns empty string — finding data lost silently.

**Fix** (`response.rs`):
```rust
pub fn to_json_line(&self) -> Result<String, serde_json::Error> {
    serde_json::to_string(self).map(|s| s + "\n")
}
```

Update all callers to handle `Result`. For `worker.rs`, propagate the error instead of defaulting to empty string.

---

### 1.8: TOCTOU Race Condition in Config Loading

**File**: `config/loader.rs:19-27,46-63`

**Issue**: Both `load_config()` and `load_scope()` use separate `exists()` check then `read()`.

**Current pattern**:
```rust
if !path.exists() {                                          // Check
    return Ok(SlapperConfig::default());
}
let content = fs::read_to_string(&path)                     // Use (separate operation)
    .with_context(|| format!("Failed to read config file: {:?}", path))?;
```

**Fix**: Eliminate the separate existence check:
```rust
let canonical_path = path.canonicalize().map_err(|e| {
    anyhow::anyhow!("Failed to canonicalize config path '{}': {}", path.display(), e)
})?;
let content = fs::read_to_string(&canonical_path)
    .with_context(|| format!("Failed to read config file: {:?}", canonical_path))?;
```

---

### 1.9: IMAP Injection in slapper-nse (CRITICAL)

**Location**: `crates/slapper-nse/src/libraries/imap.rs`

**Issue**: User-controlled input concatenated directly into IMAP protocol commands without escaping. 12 injection points.

**Vulnerable Code Locations**:
| Line | Function | Vulnerable Pattern |
|------|----------|------------------|
| 60 | `login` | `format!("{} LOGIN {} {}", tag, user, password)` |
| 117 | `list_mailboxes` | `format!("{} LIST \"{}\" \"{}\"", tag, ref_name, mailbox_name)` |
| 162 | `select` | `format!("{} SELECT {}", tag, mailbox)` |
| 206 | `fetch` | `format!("{} FETCH {} {}", tag, sequence, fields)` |
| 236 | `store` | `format!("{} STORE {} +FLAGS ({})", tag, sequence, flags)` |
| 258 | `copy` | `format!("{} COPY {} {}", tag, sequence, mailbox)` |
| 279 | `search` | `format!("{} SEARCH {}", tag, criteria)` |
| 312 | `status` | `format!("{} STATUS {} ({})", tag, mailbox, items)` |
| 355 | `create` | `format!("{} CREATE {}", tag, mailbox)` |
| 375 | `delete` | `format!("{} DELETE {}", tag, mailbox)` |
| 396 | `rename` | `format!("{} RENAME {} {}", tag, old_name, new_name)` |
| 418 | `subscribe` | `format!("{} SUBSCRIBE {}", tag, mailbox)` |

**Fix**: Add IMAP string escaping function per RFC 3501:
```rust
fn escape_imap_quoted(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 2);
    for ch in s.chars() {
        match ch {
            '\\' => result.push_str("\\\\"),
            '"' => result.push_str("\\\""),
            '\r' => {},
            '\n' => {},
            c => result.push(c),
        }
    }
    result
}
```

Apply to all 12 locations.

**Priority**: CRITICAL — Direct arbitrary command injection possible.

---

### 1.10: resolve_host() Lacks Private IP Blocking

**Location**: `utils/parsing.rs:57-64`

**Issue**: `utils::resolve_host()` used by port scanner does NOT block private/loopback IPs unlike `TargetScope::resolve_host()`.

**Current (insecure)**:
```rust
pub fn resolve_host(host: &str) -> Result<IpAddr> {
    let addrs: Vec<IpAddr> = (host, 0).to_socket_addrs()?
        .map(|sa| sa.ip()).collect();
    addrs.into_iter().next()
        .ok_or_else(|| anyhow!("Could not resolve host: {}", host))
}
```

**Fix**: Add private IP blocking:
```rust
pub fn resolve_host(host: &str) -> Result<IpAddr> {
    let addrs: Vec<IpAddr> = (host, 0).to_socket_addrs()?
        .map(|sa| sa.ip()).collect();
    let ip = addrs.into_iter()
        .next()
        .ok_or_else(|| anyhow!("Could not resolve host: {}", host))?;

    if is_loopback(&ip) {
        anyhow::bail!("Resolved to loopback address blocked");
    }
    if is_private_ip(&ip) {
        anyhow::bail!("Resolved to private IP address blocked");
    }
    Ok(ip)
}
```

Where `is_private_ip()` checks 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16, 169.254.0.0/16, and IPv6 equivalents.

**Priority**: HIGH — SSRF to internal infrastructure possible.

---

## Wave 2: HIGH Priority Security (Parallel within groups)

Wave 2 can start after Wave 1 is complete. Items within Wave 2 are independent and can be parallelized.

---

### 2.1: Ruby Sandbox Escape

**File**: `slapper-ruby/src/api.rs:16-574`

**Issue**: Dangerous APIs fully exposed. `register_api()` exposes `Slapper::HTTP`, `Slapper::Scanner`, `Slapper::Fuzzer`, `Slapper::Metasploit` modules BEFORE any plugin validation runs.

**Fix**: Remove HTTP, Scanner, Fuzzer, Metasploit modules — keep only safe reporting methods.

---

### 2.2: Python Suspicious Pattern Detection Gaps

**File**: `slapper-plugin/src/python.rs:16-27`

**Issue**: 8 patterns currently checked. Missing dangerous patterns.

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

**File**: `slapper-ruby/src/bridge.rs:13-31`

**Issue**: 15 patterns currently checked. Missing:
```rust
("Kernel.exec", regex::Regex::new(r"Kernel\.exec").unwrap()),
("open alias", regex::Regex::new(r"\bopen\b").unwrap()),
("eval without parens", regex::Regex::new(r"\beval\b").unwrap()),
```

Also add `(?i)` flag for case-insensitive matching on method names.

---

### 2.4: TLS Bypass Without Warning Logging

**Category A — Hardcoded `true` (7 locations, MUST fix)**:
- `scanner/cms/mod.rs:82`
- `scanner/cms/joomla.rs:31`
- `scanner/cms/drupal.rs:31`
- `scanner/templates/executor.rs:25`
- `waf/detector/compare.rs:17`
- `stress/http.rs:153`
- `proxy/health.rs:100`
- `recon/ssl_audit.rs:146` — also hardcoded `true`

**Category B — Flag-controlled (4 locations, LOWER priority)**:
- `fuzzer/engine/advanced.rs:18`
- `fuzzer/engine/core.rs:178`
- `scanner/endpoints.rs:686`
- `loadtest/runner.rs:232`

**Fix for Category A**: Use centralized `create_insecure_http_client()` from `utils/http.rs` which already logs warnings.

**Fix for Category B**: Add `tracing::warn!()` before the flag check.

---

### 2.5: Scope Validation Missing in REST API

**File**: `tool/protocol/rest.rs:301-339`

**Issue**: `execute_tool` handler performs auth check and rate limiting but has zero scope validation. Target from user payload is directly dispatched.

**Fix**: Add scope check before dispatch:
```rust
let target_url = &payload.target;
if let Some(ref scope) = state.scope {
    if !scope.is_allowed(target_url) {
        return Err(AppError::ScopeViolation(target_url.clone()));
    }
}
```

---

### 2.6: Scope Validation Missing in MCP Handlers

**File**: `tool/protocol/mcp/handlers.rs:252-337`

**Issue**: `handle_tools_call` has zero scope validation. Target constructed from user arguments and dispatched without scope checks.

**Fix**: Add scope check before tool execution.

---

### 2.7: Scope Validation Missing in OpenAI Handlers

**File**: `tool/protocol/openai/handlers.rs:121-207`

**Issue**: `non_streaming_response` extracts target from user query and dispatches without scope checks.

**Fix**: Add scope check on extracted targets before execution.

---

### 2.8: Credential Exposure in Proxy URL

**Files**:
- `proxy/config.rs:132-147` — `to_url()` embeds `pass.expose_secret()` in URL
- `proxy/pool.rs:173-188` — uses `to_url()` as DashMap key and in logging

**Additional affected locations**:
- `proxy/health.rs:48` — stored in `HealthCheckResult.proxy_url`
- `proxy/rotator.rs:99,129,166,386` — used for tracking
- `commands/handlers/stress.rs:93` — **prints credentials to stdout**

**Fix**: Add `to_log_key()` method for safe logging/display:
```rust
pub fn to_log_key(&self) -> String {
    match (&self.username, &self.password) {
        (Some(user), Some(_)) => format!("{}://{}:***@{}:{}", scheme, user, self.address, self.port),
        _ => self.to_url(),
    }
}
```

**Status**: `to_log_key()` already implemented at `proxy/config.rs:149`.

---

### 2.9: ai_client Field Never Used in Agent

**File**: `agent/mod.rs:77-78` (declaration), 107-108 (init), 119-121 (setter)

**Issue**: `ai_client: Option<AiClient>` is declared, initialized to `None`, settable via `with_ai_client()`, but **never used in production code**.

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

**File**: `output/escape.rs:16-35`

**Issue**: `escape_csv` checks for ASCII formula chars (`=`, `+`, `-`, `@`) but not fullwidth Unicode variants. Implicit partial defense via `first_char_is_control` checking `!c.is_ascii()` causes fullwidth chars to get quoted, but this relies solely on CSV quoting.

**Fix**: Normalize input to NFKC form before checking:
```rust
use unicode_normalization::UnicodeNormalization;
let normalized: String = s.nfkc().collect();
// Then check normalized string for formula chars
```

---

### 2.11: HMAC Signing Inconsistency in Webhooks

**Location**: `notify/webhook.rs:96-97`

**Issue**: Webhook sends raw secret in header vs. alerts use HMAC-SHA256.

**Current (INSECURE)**:
```rust
if let Some(ref secret) = webhook.secret {
    request = request.header("X-Webhook-Secret", secret.expose_secret());
}
```

**Fix**: Use HMAC signing like `agent/alerts/routing.rs:167-174`:
```rust
if let Some(ref secret) = webhook.secret {
    let mut mac = HmacSha256::new_from_slice(secret.expose_secret().as_bytes())
        .expect("HMAC can take key of any size");
    let body = response.text().await?;
    mac.update(body.as_bytes());
    let result = mac.finalize();
    let signature = format!("sha256={}", hex::encode(result.into_bytes()));
    request = request.header("X-Signature-256", signature);
}
```

---

### 2.12: Stack Trace Regex Insufficient

**Location**: `utils/error.rs:9-23`

**Issue**: Current patterns miss Rust panics, Python tracebacks, Go stack traces.

**Current patterns** only catch Java/C# style and Unix paths.

**Missing Patterns**:
```rust
static RUST_PANIC: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"thread\s+'[^']+'\s+panicked\s+at").unwrap()
});

static PYTHON_TRACEBACK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"Traceback \(most recent call last\):").unwrap()
});

static GO_PANIC: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(panic:\s+runtime\s+error:|goroutine\s+\d+\s+\[)").unwrap()
});

static WINDOWS_PATH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[A-Za-z]:\\[\w\\]+").unwrap()
});
```

---

### 2.13: OpenAI Conditional Scope Validation

**Location**: `tool/protocol/openai/handlers.rs:61-72`

**Issue**: Scope validation only occurs when tools match. Queries that don't match any tool bypass scope.

**Fix**: Make scope validation unconditional:
```rust
let target = extract_target_from_query(&user_query);
if let Some(ref scope) = state.scope {  // Always check
    if !scope.is_target_allowed(&target.value).unwrap_or(false) {
        return Err(...);
    }
}
```

---

## Wave 3: Code Quality - TUI & Plugin Refactoring (Parallel, High Effort)

Wave 3 depends on Waves 1-2 being complete. Large file splits are independent of each other.

---

### 3.1: TUI Tab Dispatching Duplication

**Files**:
- `tui/app/mod.rs` (**899 lines**)
- `tui/tabs/mod.rs` (**655 lines**)

**Issue**: 29 tab variants with repetitive match statements. `mod.rs` has 6 large `match self.current_tab` statements. `tabs/mod.rs` has extensive match arms for both immutable and mutable access patterns.

**Fix**: Use `enum_dispatch` crate pattern or a macro to generate match arms.

---

### 3.2: TUI Architecture Refactoring

**Files to split**:
- `tui/app/mod.rs` (**899 lines**) → Extract task management, state fields to existing submodules
- `tui/tabs/mod.rs` (**655 lines**) → Extract traits to `traits.rs`
- `tui/tabs/settings.rs` (**798 lines**) → Split to `settings/main.rs`, `settings/http.rs`, `settings/proxy.rs`
- `tui/tabs/packet.rs` (**743 lines**) → Split to `packet/capture.rs`, `packet/send.rs`
- `tui/tabs/fuzz.rs` (**698 lines**) → Split to `fuzz/config.rs`, `fuzz/results.rs`

---

### 3.3: MCP Handlers Large File Refactoring

**File**: `tool/protocol/mcp/handlers.rs` (**1069 lines**)

**Split Plan**:
- `handlers_server.rs` (~250 lines) — McpServer struct, constructor, router setup
- `handlers_request.rs` (~700 lines) — Request handlers, tool handlers
- `handlers_helpers.rs` (~150 lines) — Helper functions, utility methods

---

### 3.4: recon/dependency_scan Large File

**File**: `recon/dependency_scan.rs` (**1051 lines**)

**Split Plan**: Split by ecosystem into `recon/dependency/` subdirectory:
- `npm.rs` — Node.js/NPM dependency scanning
- `cargo.rs` — Rust/Cargo dependency scanning
- `go.rs` — Go module scanning
- `ruby.rs` — Ruby gem scanning
- `mod.rs` — Shared types, main scan orchestration

---

### 3.5: Plugin System Fixes (PLG-007 to PLG-018)

| Issue | File | Status | Fix |
|-------|------|--------|-----|
| PLG-007: Credential exposure | `lib.rs`, `api.rs` | Partial | Add `filtered_config()` |
| PLG-008: Missing Mutex | `python.rs:58-63` | Not fixed | `PythonPluginManager.plugins` needs sync |
| PLG-009: TUI task config | `task_management.rs:368-373` | Not fixed | Implement actual task spawning |
| PLG-010: Type mismatch | `plugin.rs` vs `tabs/plugin.rs` | Not fixed | Add `From` impl |
| PLG-011: Missing lifecycle | `lib.rs` | Not fixed | Add `init()`, `shutdown()` |
| PLG-012: Missing health check | `lib.rs` | Not fixed | Add `health_check()` |
| PLG-013: O(n) registry | `lib.rs:145-146` | Not fixed | Use `HashMap` instead of `Vec` |
| PLG-014: PyO3 API mixing | `python.rs` | Partial | Standardize `Py<T>` vs `Bound<T>` |
| PLG-015: Case-insensitive | `python.rs`, `bridge.rs` | Not fixed | Add `(?i)` flag to patterns |
| PLG-016: Hot reload | `lib.rs` | Not fixed | Add `reload_plugin()` |
| PLG-017: Plugin priority | `lib.rs` | Not fixed | Add `priority` field |
| PLG-018: Namespace isolation | `python.rs:226-258` | Not fixed | Prefix class names |

---

### 3.6: CircuitBreakerRegistry Dead Code

**File**: `utils/circuit_breaker.rs` (282 lines)

**Issue**: `CircuitBreakerRegistry` at line 125 is dead code. Only referenced in its own definition and `utils/mod.rs` re-export.

**Fix**: Either remove as dead code, or utilize for multi-provider AI support.

---

### 3.7: UTF-8 Byte Slicing Crash in InputField

**Location**: `tui/components/input.rs:312-343`

**Issue**: `InputField` stores `cursor_pos` as byte offset but `render()` uses `self.value.len()` which returns character count. Mixing byte indices with character counts causes panic when user types multi-byte UTF-8 characters (CJK, emoji).

**Example panic**:
```
value = "日本語abc"  // 8 bytes
cursor_pos = 4       // byte offset
width = 5, available = 3
start = 4 - 1 = 3
end   = (3 + 3).min(8) = 6
byte slice [3..6] = [0xe8, 0xaa, 0x61] — byte 3 is MIDDLE of multi-byte char
Panic: "byte index 3 is not a valid char boundary"
```

**Fix**: Use character-based indexing:
```rust
let char_count = self.value.chars().count();
if char_count > available {
    let cursor_char_pos = self.value.chars().count().min(self.cursor_pos);
    let start = cursor_char_pos.saturating_sub(available / 2);
    let end = (start + available).min(char_count);
    let truncated: String = self.value.chars().skip(start).take(end - start).collect();
    format!("{}...", truncated)
}
```

---

### 3.8: Tab Match Duplication in ui.rs

**Location**: `tui/ui.rs:310-705`

**Issue**: 3 separate match statements (draw_breadcrumb, draw_content, draw_status_bar) each with 29 identical arms. ~240 lines of duplicated code.

**Existing infrastructure**: `tabs/mod.rs:446-505` has `Tab::as_tab_render(&app)` method that returns `&'a dyn TabRender`.

**Fix**: Replace each match with direct calls to `as_tab_render()` and `as_tab_input()`:
```rust
fn draw_content(f: &mut Frame, app: &App, area: Rect) {
    use crate::tui::tabs::TabRender;
    let insert_mode = app.mode == InputMode::Insert;
    app.current_tab.as_tab_render(app).render(f, area, insert_mode);
    app.current_tab.as_tab_render(app).render_overlays(f, area);
}
```

---

### 3.9: Reset Method Conflict in TabState/TabInput

**Location**: `tabs/mod.rs:590,654`

**Issue**: `TabState::reset()` (line 590) is `fn reset(&mut self);` (REQUIRED). `TabInput::reset()` (line 654) is `fn reset(&mut self) {}` (DEFAULT empty). The default shadows the required method, causing confusion.

**Fix**: Remove `TabInput::reset()` default, let it inherit from `TabState`.

---

### 3.10: Dead handle_search() Method

**Location**: `tabs/mod.rs:643`

**Issue**: `handle_search(&mut self, _query: &str) {}` default implementation is never called anywhere in codebase.

**Fix**: Remove the dead method.

---

## Wave 4: Performance Optimization (Parallel)

Wave 4 can start after Wave 3. Performance changes should be benchmarked before and after.

---

### 4.1: HashMap -> FxHashMap Migration

**Files**: 55 files with 140 references to `std::collections::HashMap`

**Priority hot paths** (already using `std::collections::HashMap`):
- `waf/detector/mod.rs:19-20` — signatures storage
- `waf/detector/detect.rs:159` — detection loop
- `scanner/templates/models.rs:8` — templates
- `fuzzer/chain.rs:5` — fuzz chains
- `proxy/intercept/rules.rs:7` — rules

**Note**: Some hot-path modules have already been migrated to FxHashMap (`fuzzer/state.rs`, `fuzzer/payloads/mod.rs`, `scanner/templates/matcher.rs`).

**Fix**:
```rust
// Before:
use std::collections::HashMap;

// After:
use rustc_hash::FxHashMap as HashMap;
// OR:
use rustc_hash::FxHashMap;
```

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
- `tui/workers/recon.rs:112` — `Arc<std::sync::Mutex<String>>`

**Note**: `parking_lot::Mutex::lock()` returns `MutexGuard` directly (NOT `Result`), so remove `.unwrap()` calls.

---

### 4.4: TimingAnalyzer Lock-Free Redesign

**Files**:
- `fuzzer/detection/analyzer.rs` — `TimingAnalyzer` struct with `&mut self` methods on `samples: Vec<f64>`
- `fuzzer/engine/utils.rs:198-199` — **Bottleneck** where `Arc<Mutex<TimingAnalyzer>>` is locked for every fuzzer request

**Issue**: Every fuzzer request acquires the tokio Mutex to call `record()`, serializing all concurrent fuzzing tasks.

**Fix**: Use `crossbeam::queue::SegQueue` + atomic stats:
```rust
pub struct TimingAnalyzer {
    sample_queue: SegQueue<f64>,  // Lock-free queue
    total_requests: AtomicU64,
    // ... atomics for other stats
}
```

**Note**: `crossbeam` is already in `Cargo.toml` — no new dependencies needed.

---

### 4.5: RateLimiter DashMap Conversion

**File**: `tool/ratelimit.rs:76-79`

**Issue**: Uses `RwLock<HashMap<String, TokenBucket>>`. Already uses `parking_lot::RwLock` (improvement over `std::sync`), but `DashMap` would be more efficient for high-concurrency token bucket access.

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

**Issue**: Both locations create fresh `RegexBuilder` on every call.

**Fix**: Add cache to `ChainExecutor`:
```rust
pub struct ChainExecutor {
    // ...
    regex_cache: FxHashMap<String, Regex>,
}
```

---

### 4.7: tokio::sync::watch for Progress Updates

**File**: `tui/workers/recon.rs:112-141`

**Issue**: Uses `Arc<Mutex<String>>` with a polling loop (200ms sleep).

**Fix**: Replace with `watch` channel:
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

**Fix for udp_fingerprint.rs**: Use `std::net::SocketAddr::new()` with `IpAddr::from_str()` directly, avoiding string allocation.

---

### 4.9: String Interpolation Optimization

**Location**: `fuzzer/chain.rs:327-335`

**Issue**: `interpolate_string()` does O(n × m) string operations per variable substitution:
```rust
fn interpolate_string(&self, input: &str) -> String {
    let mut result = input.to_string();
    for (key, value) in &self.variables {
        let placeholder = format!("${{{}}}", key);
        result = result.replace(&placeholder, value);
    }
    result
}
```

**Fix**: Single-pass regex with capture groups:
```rust
fn interpolate_string(&self, input: &str) -> String {
    static RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"\$\{(\w+)\}").unwrap()
    });

    RE.replace_all(input, |caps: &Captures| {
        let key = &caps[1];
        self.variables.get(key).unwrap_or(&format!("${{{}}}", key))
    }).into_owned()
}
```

---

### 4.10: Severity Comparison Optimization

**Location**: `tool/finding.rs:182`, `tool/response.rs:87`

**Issue**: `FromStr` impl uses `s.to_lowercase().as_str()` creating new String each time.

**Fix**: Use `eq_ignore_ascii_case()` instead:
```rust
fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
        _ if s.eq_ignore_ascii_case("critical") => Ok(ResponseSeverity::Critical),
        _ if s.eq_ignore_ascii_case("high") => Ok(ResponseSeverity::High),
        _ if s.eq_ignore_ascii_case("medium") || s.eq_ignore_ascii_case("moderate") => Ok(ResponseSeverity::Medium),
        _ if s.eq_ignore_ascii_case("low") => Ok(ResponseSeverity::Low),
        _ if s.eq_ignore_ascii_case("info") || s.eq_ignore_ascii_case("informational") => Ok(ResponseSeverity::Info),
        _ => Ok(ResponseSeverity::None),
    }
}
```

---

## Wave 5: Agent System Improvements (Parallel)

Wave 5 depends on Wave 1 being complete. All items are independent and can be done in parallel.

---

### 5.1: ResponseSeverity vs Severity Type Mismatch

**Files**:
- `tool/finding.rs:11` — `Finding.severity` uses `ResponseSeverity` (includes `None` variant)
- `agent/mod.rs:308` — `process_findings_by_severity()` imports `ResponseSeverity`
- `agent/memory.rs:51` — `ScanSummary` uses canonical `Severity`
- `types.rs:16-23` — canonical `Severity` (no `None` variant)

**Root Cause**: `Finding` predates canonical `Severity` migration. `ResponseSeverity` was kept for API compatibility.

**Fix**:
1. Add `None` variant to `Severity` enum
2. Change `Finding.severity` to use `crate::types::Severity`
3. Add `From<ResponseSeverity> for Severity` impl
4. Update all test files

---

### 5.2: AlertRouter Channel Persistence Missing

**File**: `agent/alerts/routing.rs:33-43`

**Issue**: `AlertRouter` and `AlertRoutingRules` exist but no config file format for channel persistence. Without channels, agent never sends alerts.

**Fix - Option A (CLI flags)** [RECOMMENDED]:
Add `--alert-webhook`, `--alert-slack`, `--alert-email` flags to `cli/agent.rs`.

**Fix - Option B (config file)**:
Define `AgentAlertsConfig` in `agent/alerts/config.rs`.

---

### 5.3: Silent Timezone Fallback in OffPeakWindow

**Location**: `agent/portfolio.rs:80-93`

**Issue**: `parse().unwrap_or(chrono_tz::UTC)` silently falls back if invalid timezone provided.

**Fix**:
```rust
let tz: Tz = match self.timezone.parse() {
    Ok(tz) => tz,
    Err(e) => {
        tracing::warn!(
            "Invalid timezone '{}' in off_peak_window: {}. Defaulting to UTC.",
            self.timezone, e
        );
        chrono_tz::UTC
    }
};
```

---

### 5.4: Priority Field Unused in Scan Scheduling

**File**: `agent/mod.rs:181-222` — `process_scheduled_scans()`

**Issue**: `config.priority` is stored but never read. Targets iterated in arbitrary HashMap order.

**Fix**:
```rust
let mut targets = self.portfolio.get_all_targets();
// Sort by priority descending (Critical first)
targets.sort_by(|(_, a), (_, b)| {
    b.priority.as_int().cmp(&a.priority.as_int())
});
```

---

### 5.5: AI Client Integration Partial

**File**: `agent/mod.rs:282-298` — `handle_findings()`

**Issue**: Agent only calls `analyze_findings_typed()`. Unused: `AiPayloadGenerator`, `SmartWafBypass`, `AdaptiveScanEngine`.

**Fix - Phase 1**: Before scan, call `ai_client.suggest_payloads()` to get context-aware payloads.
**Fix - Phase 2**: In scanner/fuzzer pipeline, call `waf_bypass.suggest()` for blocked payloads.
**Fix - Phase 3**: After `process_findings()`, call `adaptive.analyze_findings()` to adjust next scan parameters.

---

### 5.6: Silent JSON Serialization Failure in handle_findings

**File**: `agent/mod.rs:287`

**Issue**: `unwrap_or_default()` silently drops findings that fail serialization.

**Fix**:
```rust
let mut finding_values = Vec::with_capacity(findings.len());
for (i, f) in findings.iter().enumerate() {
    match serde_json::to_value(f) {
        Ok(v) => finding_values.push(v),
        Err(e) => {
            tracing::warn!(
                "Failed to serialize finding {} (id={}): {}",
                i, f.id, e
            );
        }
    }
}
```

---

### 5.7: ToolDispatcher Error Swallowing

**File**: `tool/dispatcher.rs:48-65`

**Issue**: When `tool.execute()` returns `Err(_)`, pattern ignores actual error and creates dummy `ToolResponse`. Original error never logged.

**Fix**: Log the error with context:
```rust
Err(e) => {
    tracing::warn!(
        target = %request.tool,
        target_url = %request.target.value,
        "Tool '{}' failed on '{}': {:?}",
        request.tool, request.target.value, e
    );
    // ... create response with error info
}
```

---

### 5.8: SkillRegistry Not Wired into Agent

**File**: `agent/mod.rs:74-86`

**Issue**: `Agent` has no `SkillRegistry` field. Skills only loaded via CLI.

**Fix**: Add `SkillRegistry` field to `Agent` and auto-load skills from default directories.

---

### 5.9: Graceful Shutdown Not Implemented

**File**: `agent/mod.rs:129-159`

**Issue**: `run()` uses `self.running` (Arc<RwLock<bool>>) with polling loop. No SIGTERM/SIGINT signal handlers.

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

### 5.10: Event Loop Swallows Errors

**File**: `agent/mod.rs:145-147`

**Issue**: If `is_ok()` returns false, nothing happens — error completely discarded.

**Fix**:
```rust
if let Err(e) = self.process_scheduled_scans().await {
    tracing::warn!(error = %e, "Scheduled scan failed");
} else {
    tracing::debug!("Processed scheduled scans");
}
```

---

### 5.11: Dedup Key Collision in AlertRouter

**File**: `agent/alerts/routing.rs:237-244`

**Issue**: `make_dedup_key()` doesn't include `finding_ids` hash. Alerts with same target/severity/title but different findings produce same key.

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

### 5.12: Lock Poisoning in TargetPortfolio

**File**: `agent/portfolio.rs` (multiple locations)

**Issue**: Uses `std::sync::RwLock` with `.unwrap()` on lock acquisitions. Will panic on lock poisoning.

**Fix**: Migrate to `parking_lot::RwLock` which does not panic on poison.

---

### 5.13: TOCTOU Race in AlertRouter Dedup

**File**: `agent/alerts/routing.rs:45-75`

**Issue**: Three separate lock acquisitions for check-size, check-dedup, insert-dedup. Between check and insert, another concurrent call could proceed with same key.

**Fix**: Use `DashMap` for atomic check-and-insert, or combine steps into a single lock scope.

---

### 5.14: LongitudinalMemory TTL Cleanup Never Invoked

**File**: `agent/memory.rs:397`

**Issue**: `cleanup_old_patterns()` exists but is never called.

**Fix**: Add scheduled cleanup to Agent in `process_scheduled_scans()`:
```rust
if should_cleanup() {
    let cleaned = self.memory.cleanup_old_patterns(90)?;  // 90 day TTL
    if cleaned > 0 {
        tracing::info!("Cleaned {} old pattern entries", cleaned);
    }
}
```

---

### 5.15: Portfolio save() Blocking I/O in Async Context

**File**: `agent/portfolio.rs:196-212`

**Issue**: `std::fs` (blocking) used inside async context.

**Fix**: Wrap in `tokio::task::spawn_blocking`:
```rust
pub async fn save_async(&self) -> Result<()> {
    let file_path = self.file_path.clone();
    let data = self.data.read().clone();
    tokio::task::spawn_blocking(move || {
        // ... write file ...
    }).await??;
    Ok(())
}
```

---

### 5.16: Memory Storage Failure Treated as Non-Critical

**File**: `agent/mod.rs:399-401`

**Issue**: `tracing::warn!` logs but continues as if success.

**Fix**: Escalate to error level and trigger alert:
```rust
if let Err(e) = self.memory.store_scan_results(target, &result) {
    tracing::error!(
        target = %target,
        error = %e,
        "CRITICAL: Failed to store scan results for {}. Historical tracking disabled.",
        target
    );
    // Optionally trigger alert
}
```

---

### 5.17: Scan Failure Escalation Missing

**File**: `agent/mod.rs:156-160`

**Issue**: If all scans start failing (network outage), no alert is triggered.

**Fix**: Add consecutive failure counter:
```rust
struct Agent {
    consecutive_failures: Arc<AtomicUsize>,
    failure_threshold: usize,  // configurable, default 3
}

impl Agent {
    async fn process_scheduled_scans(&mut self) -> Result<()> {
        let result = /* ... scan execution ... */;
        match result {
            Ok(_) => {
                self.consecutive_failures.store(0, Ordering::Relaxed);
            }
            Err(e) => {
                let failures = self.consecutive_failures.fetch_add(1, Ordering::Relaxed) + 1;
                if failures >= self.failure_threshold {
                    // Trigger alert
                }
            }
        }
        Ok(())
    }
}
```

---

### 5.18: Missing Integration Tests for Agent

**File**: `agent/mod.rs:434-545`

**Issue**: Only 11 unit tests exist. No integration tests exercising full event loop.

**Fix**: Add integration tests in `tests/agent_tests.rs`:
- Test: agent processes scheduled scan
- Test: agent handles Critical findings with alert
- Test: agent handles empty portfolio gracefully
- Test: agent respects off-peak window
- Test: agent escalates on consecutive failures

---

## Wave 6: REST API & External Integrations (Parallel)

Wave 6 depends on Wave 1 being complete. Waves 5 and 6 can run in parallel with each other.

---

### 6.1: REST API TLS Configuration

**File**: `tool/protocol/rest.rs:16-35`

**Issue**: `RestState` has no TLS fields. `create_router()` creates plain HTTP router.

**Fix**: Import `TlsConfig` from `crate::distributed`, add TLS fields to `RestState`, update `create_router()` to support TLS.

---

### 6.2: REST API Rate Limiting Improvements

**File**: `tool/ratelimit.rs:11-16`

**Issue**: Basic per-client token bucket exists. What's MISSING:
- Per-endpoint rate limiting (all endpoints share same config)
- Global rate limit (no total-request cap)
- IP-based limiting (limits keyed by target, not client IP)

**Fix**: Add `RateLimitConfig` fields for per-endpoint limits, global cap, and IP-based tracking.

---

### 6.3: REST API WebSocket Support

**File**: `tool/protocol/mcp/streaming.rs:1-19`

**Issue**: File only contains `StreamEvent` for SSE. No WebSocket code. `tokio-tungstenite` exists in Cargo.toml but not integrated.

**Fix**: Add WebSocket endpoint to REST router using existing `tokio-tungstenite` dependency.

---

### 6.4: UDP IP Spoofing Integration

**File**: `stress/udp.rs:19-117`

**Issue**: `raw_udp` module defines `build_udp_packet()`, checksum functions — complete raw UDP packet builder with IP spoofing. But `run_udp_flood()` uses standard `UdpSocket` and never calls `raw_udp::*`.

**Fix**: Integrate with `--spoof-ip` flag when `stress-testing` feature enabled. When spoof enabled, use `raw_udp::build_udp_packet()` + raw socket instead of `UdpSocket`.

---

### 6.5: Ruby API block_on Deadlock Risk

**File**: `slapper-ruby/src/api.rs`

**Issue**: 35 instances of `rt.block_on()` confirmed. Classic deadlock risk — if Ruby plugin callback invoked from tokio async context, calling `block_on()` on same runtime handle panics.

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

### 6.6: SessionManager Integration (TUI)

**Location**: `tui/session.rs` (145 lines)

**Issue**: `SessionManager` fully implemented but never instantiated in `App`.

**Fix - Phase 1: App integration** (`app/mod.rs`):
```rust
pub struct App {
    // ... existing fields ...
    pub session_manager: SessionManager,
    last_auto_save: std::time::Instant,
}

impl App {
    pub fn new(history: SharedHistory) -> Self {
        let config = SessionConfig::default();
        let session_manager = SessionManager::new(config.clone());
        let last_auto_save = std::time::Instant::now();

        let pending_restore = session_manager
            .load_latest_session()
            .ok()
            .flatten();

        Self {
            // ... existing fields ...
            session_manager,
            last_auto_save,
            pending_restore: None,
        }
    }

    pub fn auto_save_if_due(&mut self) {
        let elapsed = self.last_auto_save.elapsed().as_secs();
        let interval = self.session_manager.config.auto_save_interval_secs;
        if elapsed >= interval {
            if let Err(e) = self.session_manager.save_quick(self) {
                tracing::warn!("Auto-save failed: {}", e);
            }
            self.last_auto_save = std::time::Instant::now();
        }
    }
}
```

**Fix - Phase 2: Event loop** (`app/runner.rs`):
Add at end of `run_app` loop: `app.auto_save_if_due();`
On exit: `app.session_manager.save_quick(app);`

---

### 6.7: ThemeManager Integration (TUI)

**Location**: `tui/theme.rs` (239 lines)

**Issue**: Theme system fully implemented but completely disconnected from App.

**Fix - Phase 1: App integration** (`app/mod.rs`):
```rust
pub struct App {
    // ... existing fields ...
    pub theme_manager: ThemeManager,
}

impl App {
    pub fn new(history: SharedHistory) -> Self {
        let theme_manager = ThemeManager::new();
        Self {
            // ... existing fields ...
            theme_manager,
        }
    }

    pub fn toggle_theme(&mut self) {
        self.theme_manager.toggle();
        self.needs_redraw = true;
    }
}
```

**Fix - Phase 2: Event loop** (`app/runner.rs`):
Add Ctrl+T keybinding: `app.toggle_theme();`

**Fix - Phase 3: ui.rs colors** — Replace hardcoded `Color::` references with `tc!()` macro calls.

**Note**: Requires changing `CURRENT_THEME` from `LazyLock` to `Mutex<Theme>` for safe runtime updates.

---

### 6.8: Clipboard Integration (TUI)

**Location**: `tui/utils/clipboard.rs`

**Issue**: `Clipboard` utility exists but never wired to input fields.

**Fix - Phase 1**: Add paste event detection in `app/runner.rs`:
```rust
(KeyModifiers::CONTROL, KeyCode::Char('v')) => {
    if let Some(text) = crate::tui::utils::Clipboard::get_text() {
        app.dispatcher_mut().handle_paste(&text);
        app.needs_redraw = true;
    }
}
```

**Fix - Phase 2**: Add `handle_paste()` to `TabInput` trait in `tabs/mod.rs`.

**Fix - Phase 3**: Implement `handle_paste()` on each tab's InputGroup (can use default).

---

## Wave 7: Dependency Updates (Sequential - Highest Risk)

Wave 7 must be LAST — after all other waves. Test thoroughly after each update.

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

Wave 8 can start after Wave 1. Independent of Waves 3-7.

---

### 8.1: Global Search (Ctrl+F)

**Files**: `tui/search.rs` (new), `tui/app/runner.rs`, `tui/ui.rs`

**Issue**: Search infrastructure partially exists. `search_query` field in `App` struct, search input handled in `runner.rs:422`. But no dedicated global search module.

**Fix**: Create `tui/search.rs` with global search that works across all tabs. Add Ctrl+F keybinding.

---

### 8.2: Pause/Resume (Ctrl+Z/Ctrl+Y)

**Files**: `tui/workers/pause.rs` (new), `tui/app/mod.rs`, `tui/app/runner.rs`

**Issue**: Help text at `help.rs:180` shows `"Space" => "Pause/Resume"` but no implementation exists.

**Fix**: Create `tui/workers/pause.rs` with pause/resume mechanism for running scan tasks.

---

### 8.3: Tab Overflow Display

**File**: `tui/ui.rs:255-272`

**Issue**: `draw_tabs()` renders ALL 29 tabs with no overflow handling. Will overflow on narrow terminals.

**Fix**: Add horizontal scroll or tab groups.

---

### 8.4: Input Validation Visual Feedback

**File**: `tui/components/input.rs:8-12`

**Issue**: `ValidationResult` IS used in `scan_ports.rs:153` and `scan_ports.rs:163`. Validation methods exist. Missing: **visual feedback** — red border for invalid input.

**Fix**: Implement visual rendering of validation state (red border for invalid, green for valid).

---

### 8.5: Keyboard Shortcuts Inline Display

**File**: `tui/ui.rs`

**Issue**: Help exists in `tui/help.rs`. Issue is discoverability.

**Fix**: Show contextual keyboard hints inline (e.g., "[Ctrl+F] Search" in status bar).

---

### 8.6: Tab Bookmarks/Favorites

**Verified**: No bookmark/favorite functionality exists.

**Fix**: Add bookmark persistence (save to config), keyboard shortcut to toggle bookmark.

---

### 8.7: Hardcoded Colors → Theme System

**Issue**: 366+ hardcoded `Color::` occurrences across 40+ files.

**Fix**: Replace with `tc!()` macro calls using the color mapping:
| Current | Replace With |
|---------|-------------|
| `Color::Cyan` (tab inactive) | `tc!(tab_inactive)` |
| `Color::Yellow` (tab highlight) | `tc!(highlight)` |
| `Color::White` (text) | `tc!(text)` |
| `Color::DarkGray` (dim text) | `tc!(text_dim)` |

**Approach**: Process in batches by file count:
1. `ui.rs` — highest impact per line (~28 changes)
2. `components/` — reusable components (~25 changes)
3. `tabs/` — batch by tab priority (312 changes)

---

### 8.8: Dead Code Cleanup (dispatch.rs)

**File**: `tui/app/dispatch.rs`

**Issue**: `#[allow(dead_code)]` on `handle_char` and `handle_backspace` is redundant (compiler understands dynamic dispatch).

**Fix**: Remove redundant `#[allow(dead_code)]` attributes.

---

## Wave 9: Plugin Architecture Unification (Long Term)

---

### 9.1: Enhanced PluginBackend Trait

**Current State**: Basic `Plugin` trait at `slapper-plugin/src/lib.rs:98-113`:
```rust
pub trait Plugin: Send + Sync {
    fn info(&self) -> &PluginInfo;
    fn language(&self) -> PluginLanguage;
    fn list_checks(&self) -> Vec<PluginCheck>;
    async fn run_check(&self, check_name: &str, target: &str) -> Result<PluginResult>;
    async fn run(&self, target: &str, config: &PluginConfig) -> Result<PluginResult>;
}
```

**What's needed**: Add lifecycle methods, health checking, priority:
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

### 9.4: Metasploit Integration Enhancement

**Current State**: Basic MSF RPC integration in `slapper-ruby/src/api.rs` (connect, disconnect, list_modules, execute_module, session management).

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
     ├── Wave 6 (API) ─────────────────────── After Wave 1
     ├── Wave 8 (TUI Features) ────────────── After Wave 1
     │
     ├── Wave 3 (Refactoring) ────────────── After Waves 1-2
     │   └── Wave 4 (Performance) ────────── After Wave 3
     │
     └── Wave 7 (Dependencies) ────────────── After ALL other waves (highest risk)

     Wave 9 (Plugin Unification) ─────────── Long term, anytime
```

### Sub-Agent Assignment Strategy

| Agent | Wave | Items | Dependencies |
|-------|------|-------|-------------|
| Agent 1 | Wave 1 | 1.1-1.10 | None (start first) |
| Agent 2 | Wave 2 | 2.1-2.13 | Wave 1 complete |
| Agent 3 | Wave 3 | 3.1-3.10 | Waves 1-2 complete |
| Agent 4 | Wave 4 | 4.1-4.10 | Wave 3 complete |
| Agent 5 | Wave 5 | 5.1-5.18 | Wave 1 complete |
| Agent 6 | Wave 6 | 6.1-6.8 | Wave 1 complete |
| Agent 7 | Wave 8 | 8.1-8.8 | Wave 1 complete |
| Agent 8 | Wave 7 | 7.1-7.2 | ALL other waves complete |

### Within-Wave Parallelization

| Wave | Parallel Group A | Parallel Group B |
|------|-----------------|-----------------|
| 1 | 1.1, 1.7, 1.8 (quick fixes) | 1.2, 1.3, 1.4, 1.5, 1.6, 1.9, 1.10 (security) |
| 2 | 2.1, 2.2, 2.3, 2.8, 2.10, 2.11 (patterns/TLS) | 2.4, 2.5, 2.6, 2.7, 2.9, 2.12, 2.13 (scope/validation) |
| 3 | 3.1, 3.2, 3.3, 3.4 (file splits) | 3.5, 3.6, 3.7, 3.8, 3.9, 3.10 (plugin/TUI) |
| 4 | 4.1, 4.2, 4.3, 4.6, 4.8, 4.9, 4.10 (data structures) | 4.4, 4.5, 4.7 (async/lock-free) |
| 5 | All 5.1-5.18 independent | - |
| 6 | All 6.1-6.8 independent | - |
| 8 | All 8.1-8.8 independent | - |

---

## Verification Commands

Before starting any work:
```bash
cargo test --lib -p slapper
cargo clippy --lib -p slapper
cargo test --lib -p slapper -- --list 2>/dev/null | wc -l  # Expected: 1109+
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

11. **Proxy credential exposure (2.8)** is broader than listed — also affects `proxy/health.rs`, `proxy/rotator.rs`, and `commands/handlers/stress.rs:93`.

12. **Formula injection (2.10)** has an implicit partial defense via `first_char_is_control` (checks `!c.is_ascii()`) but relies on CSV quoting.

13. **UTF-8 byte slicing crash (3.7)** affects `InputField` when user types multi-byte UTF-8 characters. Use character-based indexing, not byte indexing.

14. **IMAP injection (1.9)** is CRITICAL — 12 injection points in slapper-nse IMAP library.

15. **SessionManager and ThemeManager integration** (6.6, 6.7) require changes to `App` struct — can be done together by same agent.

---

## Implementation Status (2026-04-24)

### Build Fix
- [x] **DependencyScanReport struct missing** - Added struct to `recon/dependency_scan/mod.rs`

### Wave 1: CRITICAL Security Fixes
- [x] 1.1 git_secrets test improved
- [x] 1.2 Plugin timeout enforcement (Python timeout, Ruby recv_timeout)
- [x] 1.3 Race condition in port scanner (AtomicU64)
- [x] 1.4 Path traversal in plugin loading (validate_plugin_path)
- [x] 1.5 TUI Plugin Tab compiles (PluginsLoaded variant)
- [x] 1.6 unwrap_u8() pattern replaced with bool::from()
- [x] 1.7 Silent data loss fixed (to_json_line returns Result)
- [x] 1.8 TOCTOU in config loading (canonicalize on read)
- [ ] 1.9 IMAP Injection (pending - slapper-nse crate)
- [ ] 1.10 resolve_host() private IP blocking (pending)

### Wave 2: HIGH Priority Security
- [x] 2.1 Ruby sandbox escape (removed dangerous APIs)
- [x] 2.2 Python suspicious patterns (expanded)
- [x] 2.3 Ruby pattern detection (expanded, case-insensitive)
- [x] 2.4 TLS bypass warnings (centralized create_insecure_http_client)
- [x] 2.5 REST API scope validation
- [x] 2.6 MCP scope validation
- [x] 2.7 OpenAI scope validation
- [x] 2.8 Credential exposure (to_log_key method)
- [x] 2.9 ai_client integration (analyze_findings_typed in handle_findings)
- [x] 2.10 Formula injection unicode (NFKC normalization)
- [ ] 2.11 HMAC webhook signing (pending)
- [ ] 2.12 Stack trace regex (pending)
- [ ] 2.13 OpenAI unconditional scope (pending)

### Wave 3: Code Quality - TUI & Plugin Refactoring
- [x] 3.1 TUI tab dispatching (partial)
- [x] 3.2 TUI architecture (settings.rs split)
- [x] 3.3 MCP handlers split
- [x] 3.4 dependency_scan split
- [x] 3.5 Plugin system fixes (PLG-007-018 partial)
- [x] 3.6 CircuitBreakerRegistry (removed as dead code)
- [ ] 3.7 UTF-8 byte slicing crash (pending)
- [ ] 3.8 Tab match duplication (pending)
- [ ] 3.9 Reset method conflict (pending)
- [ ] 3.10 Dead handle_search() (pending)

### Wave 4: Performance Optimization
- [x] 4.1 HashMap->FxHashMap (hot paths confirmed)
- [x] 4.2 to_lowercase() optimization (verified)
- [x] 4.3 std::Mutex->parking_lot (verified)
- [x] 4.4 TimingAnalyzer lock-free (verified - atomics)
- [x] 4.5 RateLimiter DashMap (verified)
- [x] 4.6 Regex caching (verified - ChainExecutor has regex_cache)
- [x] 4.7 tokio::sync::watch (verified)
- [x] 4.8 String allocation optimizations (verified)
- [ ] 4.9 String interpolation optimization (pending)
- [ ] 4.10 Severity comparison (pending)

### Wave 5: Agent System
- [ ] 5.1 ResponseSeverity → Severity (pending)
- [ ] 5.2 AlertRouter channel persistence (pending)
- [ ] 5.3 Silent timezone fallback (pending)
- [ ] 5.4 Priority-based target sorting (pending)
- [ ] 5.5 Full AI client integration (pending)
- [ ] 5.6 Silent JSON serialization failure (pending)
- [ ] 5.7 ToolDispatcher error swallowing (pending)
- [ ] 5.8 SkillRegistry wired into Agent (pending)
- [ ] 5.9 Graceful shutdown (pending)
- [ ] 5.10 Event loop error handling (pending)
- [ ] 5.11 Dedup key collision (pending)
- [ ] 5.12 Lock poisoning (pending)
- [ ] 5.13 TOCTOU in AlertRouter dedup (pending)
- [ ] 5.14 Memory TTL cleanup (pending)
- [ ] 5.15 Portfolio save() blocking I/O (pending)
- [ ] 5.16 Memory storage failure escalation (pending)
- [ ] 5.17 Scan failure escalation (pending)
- [ ] 5.18 Integration tests for Agent (pending)

### Wave 6: REST API & External Integrations
- [ ] 6.1 REST API TLS (pending)
- [ ] 6.2 Rate limiting improvements (pending)
- [ ] 6.3 WebSocket support (pending)
- [ ] 6.4 UDP IP spoofing integration (pending)
- [ ] 6.5 Ruby API block_on (pending)
- [ ] 6.6 SessionManager integration (pending)
- [ ] 6.7 ThemeManager integration (pending)
- [ ] 6.8 Clipboard integration (pending)

### Wave 7: Dependency Updates
- [ ] 7.1 Axum 0.7.x -> 0.8.x (pending - highest risk)
- [ ] 7.2 Tonic 0.12.x -> 0.14.x (pending - highest risk)

### Wave 8: TUI Usability Improvements
- [ ] 8.1 Global search (pending)
- [ ] 8.2 Pause/resume (pending)
- [ ] 8.3 Tab overflow display (pending)
- [ ] 8.4 Input validation visual feedback (pending)
- [ ] 8.5 Keyboard shortcuts inline (pending)
- [ ] 8.6 Tab bookmarks (pending)
- [ ] 8.7 Hardcoded colors → theme (pending)
- [ ] 8.8 Dead code cleanup (pending)

### Wave 9: Plugin Architecture Unification
- [ ] 9.1 Enhanced Plugin trait (pending)
- [ ] 9.2 Shared security patterns (pending)
- [ ] 9.3 Move Ruby loader (pending)
- [ ] 9.4 Metasploit session caching (pending)

---

## Historical Context

Original plan files consolidated (no longer exist as separate planning documents):
- `plan.md` - Codebase review, critical issues
- `plan2.md` - Session & theme integration, code quality
- `plan3.md` - Performance improvement
- `plan4.md` - Code quality & architecture
- `plan5.md` - Code quality deep dive, clippy, dead code
- `plan6.md` - Security vulnerabilities (IMAP injection, etc.)
- `plan7.md` - Performance optimization (HashMap migration)
- `plan8.md` - TUI refactoring (TabState trait issues)
- `plan9.md` - Agent harness (ResponseSeverity, AlertRouter, etc.)
- `plan10.md` - TUI improvements (UTF-8 crash, integration gaps)

All items from these plans have been merged into this consolidated plan.md.

---

*End of Consolidated Plan*