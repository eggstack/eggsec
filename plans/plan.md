# Slapper Consolidated Improvement Plan

This document tracks all deferred and remaining work items across all plan files. Completed items have been removed.

**Date**: 2026-04-14
**Total Estimated Work**: 80-110 hours across 6 waves

---

## Current Codebase Metrics

| Metric | Current Value | Note |
|--------|---------------|------|
| Tests | 1059 passing | Verified |
| Source files | 415 .rs files | Verified |
| Largest file | `tui/app/mod.rs` (1665 lines) | Needs decomposition |
| Clippy warnings | 1 (unused import `stress::*`) | Easy fix |

---

## Wave 1: Critical Security Fixes (CRITICAL/PRIORITY)

### Block A: Authentication & Access Control

#### 1.1 Agent/AI Routes Authentication Bypass (CRITICAL)

**Severity**: CRITICAL
**Impact**: Unauthorized access to all agent and AI endpoints
**Files**: `tool/protocol/agent_routes.rs:235-243`, `tool/protocol/ai_routes.rs:179-184`

**Issue**: All 9 agent endpoints and 6 AI endpoints are unprotected.

**Fix Required**:
1. Add `require_auth()` middleware to all agent routes
2. Add `require_auth()` middleware to all AI routes
3. Add integration tests verifying auth is enforced

**Estimated**: 4-6 hours

---

#### 1.2 MCP Authentication Bypass via "initialize" (HIGH)

**Severity**: HIGH
**Impact**: Authentication bypass for MCP protocol clients
**Files**: `tool/protocol/mcp/routes.rs:179-184`

**Current Code**:
```rust
if req.method != "initialize" {  // Bypasses auth!
    if let Err(e) = state.mcp_server.validate_auth(&headers) {
```

**Fix**: Remove the `initialize` exception, auth should be required for all methods

**Estimated**: 1 hour

---

#### 1.3 NSE Sandbox Enforcement (CRITICAL)

**Severity**: CRITICAL
**Impact**: Arbitrary shell command execution via Lua scripts
**Files**: `slapper-nse/src/libraries/io.rs:249-251, 284-286`

**Issue**: `io.popen` executes arbitrary shell commands. `SandboxConfig::default()` has `enabled: false`.

**Fix Options**:
1. **Preferred**: Change `SandboxConfig::default()` to have `enabled: true` by default
2. Add runtime warning when sandbox is disabled before executing NSE scripts
3. Require explicit `--no-sandbox` flag to disable sandboxing

**Estimated**: 1-2 hours

---

### Block B: Injection Vulnerabilities

#### 1.4 CSV Formula Injection (CRITICAL)

**Severity**: CRITICAL
**Impact**: Remote code execution via CSV files opened in spreadsheet applications
**Files**: `output/escape.rs:9-14`, `output/csv.rs`, `output/convert.rs`, `pipeline/report.rs`

**Current Implementation**:
```rust
pub fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}
```

**Issue**: Does not escape formula-unsafe characters (`=`, `+`, `-`, `@`, `\t`, `\r`)

**Fix Required**:
```rust
pub fn escape_csv(s: &str) -> String {
    let needs_quote = s.contains(',') || s.contains('"') || s.contains('\n')
        || s.starts_with('=') || s.starts_with('+') || s.starts_with('-')
        || s.starts_with('@') || s.contains('\t') || s.contains('\r');
    if needs_quote {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}
```

**Estimated**: 1-2 hours

---

#### 1.5 XML Injection in Port Scan Output (MEDIUM)

**Severity**: MEDIUM
**Impact**: Malformed XML output, potential XXE if XML is re-processed
**Files**: `scanner/ports/mod.rs:228, 232-233, 398, 401-403`, `slapper-nse/src/output.rs`

**Fix**: Add XML escaping before interpolation using existing `escape_xml()` function from `output/escape.rs`

**Estimated**: 1-2 hours

---

#### 1.6 Log Injection via Newlines (MEDIUM)

**Severity**: MEDIUM
**Impact**: Fake log entries, log falsification attacks
**Files**: `utils/logging.rs:29`

**Current**: `sanitize_for_logging()` preserves `\n`, `\r`, `\t`

**Fix**:
```rust
if (0x00..=0x1F).contains(&b) && b != 0x09 {  // Remove \n and \r
```

**Estimated**: 1 hour

---

#### 1.7 NSE `nmap.get_interface()` Command Injection (MEDIUM)

**Severity**: MEDIUM
**Impact**: Shell command injection via interface names
**Files**: `slapper-nse/src/libraries/nmap.rs:1518-1522`

**Fix**: Validate interface name format
```rust
if !iface_name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
    return Err("Invalid interface name".into());
}
```

**Estimated**: 1 hour

---

### Block C: Cryptography & Secrets

#### 1.8 TLS Certificate Verification Bypass - Warnings (HIGH)

**Severity**: HIGH
**Impact**: Man-in-the-middle attacks on distributed cluster communications
**Files**: `distributed/io.rs:182-195`, 47 locations with `danger_accept_invalid_certs(true)`

**Fix Required**:
1. Add runtime warning when `insecure-tls` feature is enabled
2. Consider adding `--insecure-tls` CLI flag that requires explicit opt-in
3. Add warning logs when insecure connections are established
4. Document security implications in README

**Estimated**: 2-3 hours

---

#### 1.9 HMAC Serialization Order (MEDIUM)

**Severity**: MEDIUM
**Files**: `crates/slapper/src/agent/alerts.rs:199-206`

**Issue**: HMAC computed over `payload.to_string()` which has non-deterministic JSON key ordering.

**Fix**: Use canonical JSON serialization:
```rust
mac.update(serde_json::to_string(&payload).unwrap().as_bytes());
```

**Estimated**: 30 minutes

---

## Wave 2: High Priority Security & Performance

### Block A: Path & Memory Security

#### 2.1 Path Traversal Vulnerabilities (HIGH)

**Severity**: HIGH
**Impact**: Arbitrary file read/write via user-controlled paths
**Files**: `config/loader.rs`, `config/settings.rs`, `tui/app/export.rs`, `commands/handlers/sbom.rs`, `agent/skills.rs`, `agent/portfolio.rs`, `recon/git_secrets.rs`

**Fix Pattern**:
```rust
fn validate_path(base: &Path, user_path: &Path) -> Result<PathBuf> {
    let canonical = user_path.canonicalize()?;
    let base_canonical = base.canonicalize()?;
    if !canonical.starts_with(&base_canonical) {
        return Err("Path traversal detected".into());
    }
    Ok(canonical)
}
```

**Apply to all file operations using user-controlled paths**

**Estimated**: 4-6 hours

---

#### 2.2 ReDoS (Regex DoS) Vulnerabilities (HIGH)

**Severity**: HIGH
**Impact**: CPU exhaustion via malicious regex patterns
**Files**: `fuzzer/chain.rs`, `recon/js.rs`, `recon/email.rs`

**Issue**: `Regex::new(&pattern)?` without `size_limit!`

**Fix Pattern**:
```rust
use regex::{Regex, RegexBuilder};

let re = RegexBuilder::new(&pattern)
    .size_limit(100_000)  // 100KB limit
    .build()?;
```

**Estimated**: 2-3 hours

---

#### 2.3 Unbounded Memory Allocation (HIGH)

**Severity**: HIGH
**Impact**: Memory exhaustion when scanning large ranges
**Files**: `scanner/ports/mod.rs:449`, `scanner/endpoints.rs:684`, `scanner/fingerprint.rs:227`, `agent/memory.rs:155`

**Fix Options**:
1. Implement configurable result limits (e.g., `--max-results`)
2. Add pagination/streaming for result retrieval
3. Implement periodic result flushing to disk
4. Add `Arc<Mutex<Vec<PortResult>>>` bounds checking

**Estimated**: 4-6 hours

---

### Block B: Concurrency Fixes

#### 2.4 Packet Trace OnceLock Silent Failure (HIGH)

**Severity**: HIGH
**Files**: `crates/slapper/src/scanner/ports/spoofed.rs:85`

**Issue**: `OnceLock::set().ok()` silently ignores if already initialized.

**Fix**:
```rust
PACKET_TRACE_FILE.set(std::sync::Mutex::new(file))
    .map_err(|_| "Packet trace file already initialized")?;
```

**Estimated**: 30 minutes

---

#### 2.5 Ruby API Isolated Runtime (HIGH)

**Severity**: HIGH
**Files**: `slapper-ruby/src/api.rs:5-8`

**Issue**: Creates new isolated Tokio runtime instead of using existing one.

**Fix**: Use `Handle::current()`
```rust
fn get_runtime() -> &'static tokio::runtime::Handle {
    static HANDLE: std::sync::OnceLock<tokio::runtime::Handle> = std::sync::OnceLock::new();
    HANDLE.get_or_init(|| tokio::runtime::Handle::current())
}
```

**Estimated**: 30 minutes

---

#### 2.6 Distributed Worker JoinHandle Tracking (HIGH)

**Severity**: HIGH
**Files**: `crates/slapper/src/distributed/worker.rs:133`

**Issue**: Spawned task JoinHandle not stored, preventing graceful shutdown.

**Fix**:
```rust
let handle = tokio::spawn(async move {
    let result = process_task(task).await;
    if let Err(e) = result {
        tracing::error!("Task processing error: {}", e);
    }
});
self.task_handles.push(handle);
```

**Estimated**: 30 minutes

---

#### 2.7 Circuit Breaker Race Condition (MEDIUM)

**Severity**: MEDIUM
**Files**: `crates/slapper/src/utils/circuit_breaker.rs:67-81`

**Issue**: `success_count.fetch_add()` and mutex check are not atomic together.

**Fix**: Move atomic update inside mutex lock.

**Estimated**: 30 minutes

---

## Wave 3: Performance Optimizations (HIGH IMPACT)

### Block A: HashMap & Regex Optimization

#### 3.1 Mutex Contention in Scanner Result Aggregation (HIGH)

**Severity**: HIGH
**Impact**: High concurrency performance bottleneck
**Files**: `scanner/ports/mod.rs:449`, `scanner/endpoints.rs`, `scanner/fingerprint.rs`

**Issue**: All spawned tasks contend for `Arc<Mutex<Vec>>` to append results.

**Solution**: Replace `Arc<Mutex<Vec<T>>>` with `Arc<DashMap<usize, T>>` for lock-free append.

**Estimated**: 2-3 hours

---

#### 3.2 FxHashMap for Hot Paths (HIGH)

**Severity**: HIGH
**Impact**: 2-3x faster hash lookups
**Files**: Throughout (300+ HashMap usages)

**Solution**: Add `rustc-hash` and replace `std::collections::HashMap` with `FxHashMap` in hot paths:
- `fuzzer/state.rs` - Session/cookie storage
- `scanner/ports/mod.rs` - Port results
- `scanner/fingerprint.rs` - Service fingerprints
- `scanner/endpoints.rs` - Endpoint lookups
- `recon/techdetect.rs` - Technology detection

**Estimated**: 4-6 hours

---

#### 3.3 Regex Recompilation in `recon/js.rs` (HIGH)

**Severity**: HIGH
**Impact**: CPU overhead on every JS analysis call
**Files**: `recon/js.rs:146, 177, 220, 249`, `recon/email.rs:110, 143, 173`

**Issue**: `Regex::new(pattern)` called inside functions repeatedly.

**Solution**: Pre-compile regexes at module level using `LazyLock` (already done correctly in `recon/secrets.rs:23+`).

**Estimated**: 1-2 hours

---

### Block B: String & Memory Optimization

#### 3.4 String Escape Function Allocations (HIGH)

**Severity**: HIGH
**Impact**: Multi-allocation per escape operation
**Files**: `output/escape.rs`, `output/csv.rs`, `output/markdown.rs`

**Issue**: `escape_html()` creates 5 intermediate Strings via chained `.replace()`

**Solution**: Use `write!` with single buffer:
```rust
pub fn escape_html(s: &str) -> String {
    let mut buf = String::with_capacity(s.len() * 6);
    for c in s.chars() {
        match c {
            '&' => buf.push_str("&amp;"),
            // ... etc
        }
    }
    buf
}
```

**Estimated**: 1-2 hours

---

#### 3.5 Cache WAF Signatures with LazyLock (MEDIUM)

**Severity**: MEDIUM
**Files**: `waf/waf_patterns.rs:13`

**Issue**: `get_waf_signatures()` creates a new `HashMap` on every call.

**Solution**:
```rust
use std::sync::LazyLock;

static WAF_SIGNATURES: LazyLock<HashMap<String, WafSignature>> = LazyLock::new(|| {
    // ... populate signatures
});

pub fn get_waf_signatures() -> &'static HashMap<String, WafSignature> {
    &WAF_SIGNATURES
}
```

**Estimated**: 1 hour

---

#### 3.6 HTTP Client Connection Pooling (MEDIUM)

**Severity**: MEDIUM
**Impact**: Reduced latency for repeated requests to same host
**Files**: `utils/http.rs`, `agent/alerts.rs`, `tool/implementations/search.rs`

**Solution**: Add connection pooling to HTTP client creation:
```rust
Client::builder()
    .pool_max_idle_per_host(20)
    .pool_idle_timeout(Duration::from_secs(30))
    .tcp_nodelay(true)
```

**Estimated**: 1-2 hours

---

#### 3.7 Payload Cache Optimization (MEDIUM)

**Severity**: MEDIUM
**Files**: `fuzzer/payloads/mod.rs:168-169`

**Issue**: `get_all_payloads_cached()` creates full owned copies.

**Solution**: Return references or use `Rc<[Payload]>` for cached data.

**Estimated**: 2 hours

---

#### 3.8 Report Generation Efficiency (MEDIUM)

**Severity**: MEDIUM
**Files**: `output/markdown.rs`, `output/html.rs`, `output/csv.rs`

**Solution**: Use `writeln!` with single String buffer; cache theme strings as `LazyLock`.

**Estimated**: 1-2 hours

---

### Block C: Allocation Reduction

#### 3.9 `to_lowercase()` in Hot Paths (HIGH)

**Severity**: HIGH
**Impact**: 241 occurrences allocating on every path check
**Files**: `scanner/endpoints.rs:343`, `scanner/fingerprint.rs`, `waf/detector/types.rs:43-46`

**Issue**: `path.to_lowercase().contains("wp-content")` allocates a new String on every check.

**Solution**: Add helper function:
```rust
fn str_contains_ignore_case(haystack: &str, needle: &str) -> bool {
    haystack.to_lowercase().contains(&needle.to_lowercase())
}
```

**Estimated**: 2-3 hours

---

#### 3.10 Banner Buffer Optimization (LOW)

**Severity**: LOW
**Files**: `scanner/fingerprint.rs`

**Solution**: Replace `Vec<u8>` with `SmallVec<[u8; 256]>` in banner parsing.

**Estimated**: 1 hour

---

#### 3.11 Grammar Fuzzer Clone Reduction (LOW)

**Severity**: LOW
**Files**: `fuzzer/grammar.rs:227,243,249`

**Issue**: `start.clone()` called on every `generate()`.

**Solution**: Pass `&Grammar` by reference instead of cloning.

**Estimated**: 30 minutes

---

## Wave 4: Code Quality

### Block A: Broken Tests & Fixes

#### 4.1 Fuzzer Test Import Fix (HIGH)

**Severity**: HIGH
**Files**: `crates/slapper/tests/fuzzer_tests.rs:4`

**Issue**: `get_all_payloads` is not re-exported from `slapper::fuzzer`.

**Fix**: Update import to use `get_all_payloads_cached`.

**Estimated**: 15 minutes

---

#### 4.2 Stress Test Feature Gate (HIGH)

**Severity**: HIGH
**Files**: `crates/slapper/tests/stress_tests.rs:1`

**Issue**: Missing `#[cfg(feature = "stress-testing")]` attribute.

**Fix**: Add proper feature gate.

**Estimated**: 15 minutes

---

#### 4.3 Doc Test Fixes (MEDIUM)

**Severity**: MEDIUM
**Files**: `fuzzer/engine/core.rs`, `output/mod.rs`, `recon/mod.rs`, `scanner/mod.rs`

**Issue**: 5 doc tests failing due to invalid examples.

**Fix**: Fix examples to compile correctly.

**Estimated**: 2-3 hours

---

### Block B: Code Organization

#### 4.4 TUI App Decomposition (MEDIUM)

**Severity**: MEDIUM
**Files**: `tui/app/mod.rs` (664 lines)

**Issue**: Large monolithic file. Already partially split (navigation.rs, command.rs, export.rs, state_update.rs, task_management.rs).

**Remaining Work**:
- Move `App` struct methods into corresponding feature-specific submodules
- Extract `match self.current_tab` dispatch into a `TabDispatcher` trait/impl

**Estimated**: 20+ hours

---

#### 4.5 SensitiveString Serialization Documentation (MEDIUM)

**Severity**: MEDIUM
**Files**: `crates/slapper/src/types.rs:193-196`

**Issue**: `SensitiveString` serializes secrets in plaintext.

**Fix**: Add prominent doc warning about plaintext serialization.

**Estimated**: 15 minutes

---

#### 4.6 URL Encoding Fixes (MEDIUM)

**Files**:
- `integrations/github.rs:222` - Query parameter not encoded
- `recon/subdomain.rs:92` - crt.sh query not encoded

**Fix**:
```rust
use urlencoding::encode;
let url = format!("https://api.github.com/search/issues?q={}", encode(query));
```

**Estimated**: 30 minutes

---

### Block C: Unwrap/Expect Audit (HIGH - 8-12 hours)

#### 4.7 High-Risk Unwrap Audit

**Severity**: HIGH
**Impact**: Runtime panics on malformed data (477 total, ~200+ in production)

**Priority Locations**:
| File | Risk | Lines |
|------|------|-------|
| `fuzzer/engine/core.rs` | HIGH | 415,429,440,450,460,483,491,510 |
| `tool/response.rs` | HIGH | 887,889,893,895,909,911,923,925,966 |
| `scanner/fingerprint.rs` | HIGH | 637,638 |
| `scanner/endpoints.rs` | HIGH | 600,601 |
| `scanner/ports/mod.rs` | HIGH | 586,587 |
| `distributed/io.rs` | HIGH | 287,289,298,308,312,321,325,335,337,340,347,352,355 |

**Fix Pattern**:
```rust
// Before
let json = serde_json::to_string(&fp).unwrap();

// After
let json = serde_json::to_string(&fp)
    .map_err(|e| SlapperError::Serialization { source: e })?;
```

**Estimated**: 8-12 hours (focus on high-risk first)

---

#### 4.8 Distributed Command Dead Code Removal (CRITICAL)

**Severity**: CRITICAL (security implication)
**Files**: `crates/slapper/src/distributed/command.rs:145-161`

**Issue**: Early return at line 147 makes lines 157-161 unreachable. Dead code creates security risk.

**Fix**: Remove dead code (lines 157-161).

**Estimated**: 15 minutes

---

#### 4.9 Redundant ProxyPool Synchronization (LOW)

**Severity**: LOW
**Files**: `proxy/pool.rs:53-58`, `proxy/mod.rs:31`

**Issue**: `DashMap` is already thread-safe, wrapping in `RwLock` is redundant.

**Estimated**: 30 minutes

---

#### 4.10 Secondary Error Type Conversions (MEDIUM)

**Severity**: MEDIUM
**Files**: `ai/errors.rs`, `packet/capture.rs`, `packet/traceroute.rs`

**Issue**: `AiError`, `CaptureError`, `TracerouteError` have no conversion path to `SlapperError`.

**Fix**: Add `From` impls in `error/mod.rs`.

**Estimated**: 1 hour

---

#### 4.11 Mixing Sync Primitives (MEDIUM)

**Severity**: MEDIUM
**Files**: `scanner/ports/spoofed.rs:133-165`

**Issue**: Mixes `parking_lot::Mutex` and `tokio::sync::Mutex` in same function.

**Fix**: Standardize on `tokio::sync::Mutex` for async contexts.

**Estimated**: 30 minutes

---

## Wave 5: Testing & Documentation

### Block A: Test Improvements

#### 5.1 Serialization Roundtrip Test Helper (MEDIUM)

**Severity**: MEDIUM
**Files**: Throughout (10+ test files)

**Issue**: Repeated pattern across test files:
```rust
let json = serde_json::to_string(&fp).unwrap();
let deserialized: Type = serde_json::from_str(&json).unwrap();
```

**Fix**: Create test helper in `tests/common/mod.rs`:
```rust
pub fn assert_serialize_roundtrip<T: Serialize + DeserializeOwned + Eq>(value: &T) {
    let json = serde_json::to_string(value).unwrap();
    let decoded: T = serde_json::from_str(&json).unwrap();
    assert_eq!(value, &decoded);
}
```

**Estimated**: 2-3 hours

---

#### 5.2 Scope Enforcement Test (MEDIUM)

**Severity**: MEDIUM
**Files**: `tests/scope_tests.rs:50-58`

**Issue**: `test_scope_enforcement_in_handlers` only tests URL normalization, not scope enforcement.

**Fix**: Either rename test or add real scope enforcement test.

**Estimated**: 1 hour

---

#### 5.3 Unused Mock Helpers (LOW)

**Severity**: LOW
**Files**: `tests/common/wiremock_helpers.rs`

**Issue**: 3 helpers never used: `mock_secure_headers()`, `mock_jwt_response()`, `mock_rate_limited()`.

**Fix**: Remove or add tests that use them.

**Estimated**: 30 minutes

---

#### 5.4 Test Organization & Coverage (MEDIUM)

**Severity**: MEDIUM
**Files**: `crates/slapper/tests/`

**Solution**: Consolidate duplicate test utilities, audit coverage.

**Estimated**: 2-3 hours

---

### Block B: Documentation

#### 5.5 Public API Documentation (MEDIUM)

**Severity**: MEDIUM
**Files**: `tool/traits.rs`, `tool/response.rs`, `tool/registry.rs`, `scanner/ports/mod.rs`, `fuzzer/engine/core.rs`

**Issue**: Minimal `#[doc(...)]` attributes on public functions.

**Fix**: Add doc comments with `# Examples` and `# Errors` sections.

**Estimated**: 4-6 hours

---

#### 5.6 Generated File Documentation (LOW)

**Severity**: LOW
**Files**: `crates/slapper/src/generated/slapper.tool.v1.rs`

**Issue**: File marked `@generated by prost-build` but no build.rs for regeneration.

**Fix**: Add comment explaining manual maintenance requirement.

**Estimated**: 10 minutes

---

#### 5.7 Architecture Decision Records (LOW)

**Severity**: LOW
**Files**: New `docs/adr/`

**Create**:
- ADR-001: Why `SensitiveString` instead of `SecretString`
- ADR-002: Feature flag design rationale
- ADR-003: Why `rustls` over `native-tls` (except nse)
- ADR-004: Error type separation (`SlapperError` vs `anyhow::Result`)

**Estimated**: 3-4 hours

---

## Wave 6: Additional Improvements

### Block A: Rate Limiting & Security

#### 6.1 API Rate Limiting (HIGH)

**Severity**: HIGH
**Files**: `tool/protocol/mcp/handlers.rs`, `tool/protocol/rest.rs`

**Issue**: MCP and REST API servers don't implement rate limiting.

**Solution**: Use existing `RateLimiter` from `utils/rate_limiter.rs` and `CircuitBreakerRegistry` from `utils/circuit_breaker.rs`.

**Estimated**: 2-3 hours

---

#### 6.2 Plugin Directory Sandboxing (MEDIUM)

**Severity**: MEDIUM
**Files**: `slapper-plugin/src/python.rs:71-119`, `slapper-ruby/src/bridge.rs:112`

**Solution**:
1. Validate plugin files before loading (extension, size, suspicious imports)
2. Add plugin signing concept
3. Create plugin allowlist

**Estimated**: 2-3 hours

---

#### 6.3 Configuration Validation Hardening (MEDIUM)

**Severity**: MEDIUM
**Files**: `config/loader.rs`, `config/settings.rs`

**Solution**:
1. Add schema validation for config files
2. Add config file signing (HMAC-SHA256)
3. Add config change alerts

**Estimated**: 2 hours

---

#### 6.4 Logging Secret Redaction Audit (MEDIUM)

**Severity**: MEDIUM
**Files**: Throughout (7447+ format! usages)

**Solution**: Audit all format! calls for potential secrets, add secret detection to logging.

**Estimated**: 2-3 hours

---

#### 6.5 Session Fixation Risk (MEDIUM)

**Severity**: MEDIUM
**Files**: `tool/state.rs`

**Fix**: Regenerate session ID after authentication state changes.

**Estimated**: 2 hours

---

### Block B: Additional Performance

#### 6.6 JSON Serialization Optimization (LOW)

**Severity**: LOW
**Files**: Throughout (67+ `to_string_pretty()` usages)

**Solution**: Use `to_string()` for internal operations, reserve `to_string_pretty()` for user-facing output only.

**Estimated**: 1 hour

---

#### 6.7 Vec Capacity Hints (LOW)

**Severity**: LOW
**Files**: Throughout

**Solution**: Add `Vec::with_capacity()` when final size is known or estimable.

**Estimated**: 30 minutes

---

#### 6.8 Async Mutex in Tool Implementations (LOW)

**Severity**: LOW
**Files**: `tool/implementations/*.rs`

**Issue**: Tool implementations use `std::sync::Mutex` where `parking_lot::Mutex` would be faster.

**Estimated**: 30 minutes

---

#### 6.9 Duplicate Dependency Resolution (MEDIUM)

**Severity**: MEDIUM
**Files**: `Cargo.toml`

**Issue**: `crossterm` 0.28 (direct) vs 0.29 (ratatui-crossterm), `base64` 0.21 vs 0.22.

**Solution**: Run `cargo update -p crossterm --precise 0.29` to align.

**Estimated**: 30 minutes

---

### Block C: Tech Debt & Cleanup

#### 6.10 Extract Common URL Stripping Logic (LOW)

**Severity**: LOW
**Files**: `recon/runner.rs:14-19`

**Issue**: Duplicated URL stripping logic.

**Fix**: Use `url` crate or extract to utility.

**Estimated**: 1 hour

---

#### 6.11 Progress Bar Reuse in Scanner/Fuzzer (LOW)

**Severity**: LOW
**Files**: `scanner/ports/mod.rs`, `scanner/endpoints.rs`, `fuzzer/engine/core.rs`

**Solution**: Use `LazyLock` for progress bar templates.

**Estimated**: 2-3 hours

---

#### 6.12 Config Default Duplication (LOW)

**Severity**: LOW
**Files**: `config/mod.rs:65-115`

**Solution**: Centralize defaults in `constants.rs`.

**Estimated**: 3-4 hours

---

#### 6.13 Error Type Consistency (LOW)

**Severity**: LOW
**Files**: Various

**Issue**: Inconsistent `#[derive(...)]` patterns across error types.

**Fix**: Standardize derive order to `#[derive(Debug, thiserror::Error)]`.

**Estimated**: 2-3 hours

---

#### 6.14 Git Secrets Scanner Path Access (LOW)

**Severity**: LOW
**Files**: `recon/git_secrets.rs:68-78, 106-115, 144-146`

**Fix**: Restrict `repo_path` to user-controlled directories outside sensitive paths.

**Estimated**: 2 hours

---

## Deferred Items (Not Currently Recommended)

### D.1 Error Type Consolidation

**Status**: DEFERRED (per AGENTS.md policy)

**Issue**: Three error types (`SlapperError`, `ConfigError`, `anyhow::Result`) create friction.

**Recommendation**: Current separation serves different purposes. Consolidation deemed counterproductive.

**Estimated**: 8-12 hours

---

## Implementation Notes

### Wave 1 (Critical Security)
**Block A (Auth)**: Items 1.1, 1.2, 1.3 — Can parallelize
**Block B (Injection)**: Items 1.4, 1.5, 1.6, 1.7 — Can parallelize
**Block C (Crypto)**: Items 1.8, 1.9 — Can parallelize

### Wave 2 (High Priority)
**Block A (Path/Memory)**: Sequential (path traversal affects many files)
**Block B (Concurrency)**: Items 2.4-2.7 — Can parallelize

### Wave 3 (Performance)
All items are independent — parallelizable

### Wave 4 (Code Quality)
**Block A (Tests)**: Sequential (fix order doesn't matter)
**Block B (Code Org)**: Sequential
**Block C (Unwrap Audit)**: Sequential per file

### Wave 5 (Testing/Docs)
All items are independent — parallelizable

### Wave 6 (Additional)
All items are independent — parallelizable

---

## Verification Commands

```bash
# Core tests
cargo test --lib -p slapper
cargo clippy --lib -p slapper

# Integration tests
cargo test --test negative_tests -p slapper
cargo test --test scanner_tests -p slapper

# Build verification
cargo build --release -p slapper --features full
```

---

## Summary

| Wave | Items | Estimated Time |
|------|-------|----------------|
| 1: Critical Security | 9 | 10-15 hours |
| 2: High Priority | 7 | 9-13 hours |
| 3: Performance | 11 | 15-20 hours |
| 4: Code Quality | 10 | 35-45 hours |
| 5: Testing/Docs | 7 | 10-15 hours |
| 6: Additional | 9 | 8-12 hours |
| **Total** | **~53 items** | **80-110 hours** |

---

## Notes

- **Dependencies**: Wave 1 Block B (1.4 CSV injection) should be completed before CSV export features
- **Security First**: Always prioritize security fixes over performance
- **Backward Compatibility**: All changes must maintain backward compatibility unless explicitly breaking
- **Feature Flags**: Properly gate optional functionality
- Run `cargo test --lib -p slapper` and `cargo clippy --lib -p slapper` to verify any changes