# Consolidated Implementation Plan (2026-05-28)

## Implementation Status: ✅ ALL COMPLETED

### Wave 1 - Completed 2026-05-23
| Item | Status | Commit |
|------|--------|--------|
| AI planner.rs clock skew | ✅ Fixed | `086452d` |
| Tool planner HashSet→FxHashSet | ✅ Fixed | `1703cab` |
| Fuzzer api.rs HashMap→FxHashMap | ✅ Fixed | `a6f77a6` |
| NSE smbauth duplicate functions | ✅ Fixed | `bfbb56f` |
| Recon email.js regex expect | ✅ Fixed | `9c238f3` |
| Networking capture error propagation | ✅ Fixed | merged with `9c238f3` |

### Wave 2 - Completed 2026-05-23
| Item | Status | Commit |
|------|--------|--------|
| AI waf_bypass persist error logging | ✅ Fixed | `b409617` |
| CLI report HashMap→FxHashMap | ✅ Fixed | `8ff9705` |
| Fuzzer calibration/ssti/oauth explicit match | ✅ Fixed | `ebce8f8` |
| Fuzzer chain variable extraction + LazyLock | ✅ Fixed | `ebce8f8` |
| Networking traceroute JoinError logging | ✅ Fixed | `b409617` |
| Pipeline Arc::try_unwrap graceful fallback | ✅ Fixed | `b409617` |
| NSE datafiles duplicate entries | ✅ Fixed | `ac77386` |
| NSE io fd error handling | ✅ Fixed | `ac77386` |
| Recon mod.rs serialization error | ✅ Fixed | `5756885` |
| Recon stubbed functions documentation | ✅ Fixed | `5756885` |

### Wave 3 - Completed 2026-05-23
| Item | Status | Commit |
|------|--------|--------|
| AI cache eviction loop | ✅ Fixed | `85394ef` |
| AI CacheKeyBuilder separator note | ✅ Documented | `85394ef` |
| Config docs ScanConfig→SlapperConfig | ✅ Fixed | `7ac370d` |
| Config --strict-permissions flag | ℹ️ Deferred | informational |
| Loadtest metrics expect→unwrap_or_else | ✅ Fixed | `7dd8934` |
| Loadtest architecture run_cli doc | ✅ Fixed | `7dd8934` |
| Networking magic number constants | ✅ Fixed | `7dd8934` |
| Networking DNS parse debug | ✅ Fixed | `7dd8934` |
| Networking udp.rs task error logging | ✅ Fixed | `7dd8934` |
| Output convert Result error propagation | ✅ Fixed | `46e0178` |
| Output markdown Result error propagation | ✅ Fixed | `46e0178` |
| Output architecture docs | ✅ Fixed | `46e0178` |
| Pipeline session blocking I/O | ℹ️ Acceptable | no change |
| Pipeline FuzzArgs construction | ℹ️ Informational | no change |
| Recon runner test assertion | ✅ Already correct | `dad5682` |
| Recon cve capacity hints | ✅ Fixed | `dad5682` |
| Scanner marketplace unwrap defensive | ✅ Fixed | `6780c48` |
| TUI key binding Ctrl+b doc | ✅ Fixed | `6780c48` |
| TUI tab traits documentation | ✅ Fixed | `6780c48` |
| TUI state_update clone before Option | ✅ Fixed | `6780c48` |
| Architecture overview payload count 31 | ✅ Fixed | `85394ef` |
| Architecture undocumented modules | ℹ️ Feature-gated | informational |
| NSE HashMap replacements (7 files) | ✅ Fixed | `85394ef` |

### Notes on Implementation Diversions
1. **recon/runner.rs:863-867** - Test assertion was already correct; domain port stripping happens via `Url::host()` which only returns the host without port.
2. **scanner-tui-wave3 branch** - This branch was not pushed to origin separately; its commits were merged via `fix/pipeline-recon-wave3` branch which contained the changes.

---

## Phase/Wave Structure

### Wave 1: High-Priority & Cross-Module Independent
Production safety issues, panic prevention, and performance fixes that can be executed in parallel.

### Wave 2: Medium-Priority & Module-Specific
Error handling improvements and silent failure fixes within specific modules.

### Wave 3: Low-Priority & Documentation
Cleanup, docs, and optional enhancements.

---

## Wave 1: HIGH Priority (Production Safety & Performance)

### AI Module

#### `crates/slapper/src/ai/planner.rs:206-209,467-470,480-483`
**Issue**: `unwrap()` on `SystemTime::now().duration_since(UNIX_EPOCH)` can panic if system clock moves backwards (NTP correction or clock skew).

**Fix**:
```rust
.last_used = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap_or_else(|_| std::time::Duration::from_secs(0))
    .as_secs(),
```
**Rationale**: Prevents panic on NTP correction or clock skew. All three locations must be fixed (learning cache insert, cache update, fallback cache insert).

---

### Tool Module

#### `crates/slapper/src/tool/planner.rs:4,80,204,247,309,351,386,429,492`
**Issue**: Uses `std::collections::HashSet` instead of `FxHashSet` at 9 locations.

**Fix**:
```rust
use rustc_hash::FxHashSet;
// Replace all: use std::collections::HashSet;
// With: use rustc_hash::FxHashSet;
```
**Rationale**: Performance optimization - FxHashSet is significantly faster for hash collections.

---

### Fuzzer Module

#### `crates/slapper/src/fuzzer/targets/api.rs:4,12,47,64,77,95,96,101,102,117`
**Issue**: Uses `std::collections::HashMap` instead of `FxHashMap` in OpenAPI target parsing (10 locations).

**Fix**:
```rust
use rustc_hash::FxHashMap;
// Lines 12, 47, 64, 77, 95, 96, 101, 102, 117: Change type usages
pub paths: FxHashMap<String, PathItem>,
pub responses: FxHashMap<String, Response>,
// etc.
```
**Rationale**: Performance - 10 locations in hot path for API fuzzing.

---

### NSE Module (plugins_nse)

#### `crates/slapper-nse/src/libraries/smbauth.rs:40-54,56-81,83-93,95-107,109-119,121-131,133-149,151-166,168-193,195-205,207-219,221-231,233-243,245-256,258-276,278-299,344-364`
**Issue**: Multiple functions defined TWICE (or THREE times for `signing_hmac_md5`), causing shadowing:
- `compute_lm_hash` (lines 40-54 and 151-166)
- `ntlmv1_session` (lines 56-81 and 168-193)
- `ntlmv2_session` (lines 83-93 and 195-205)
- `get_ntlm_challenge` (lines 95-107 and 207-219)
- `signing_md5` (lines 109-119 and 221-231)
- `signing_hmac_md5` (lines 121-131, 233-243, AND 245-256 - THREE times)
- `encrypt_password` (lines 133-149 and 258-276)
- `decrypt_password` (lines 278-299 and 344-364)

**Fix**: Remove duplicate definitions, keeping only the first occurrence (lower line numbers). For `signing_hmac_md5` which appears 3 times, keep the first (lines 121-131) and remove the other two.
**Rationale**: Code correctness - second/third definition shadows first, causing unexpected behavior.

---

### Recon Module

#### `crates/slapper/src/recon/email.rs:10,14-16,24,28,30,33,37,41,45,49,53,60-61` and `crates/slapper/src/recon/js.rs:12-18,26-29,33-36,40-43,47,51,54-55,64,68,72,76,80,84,90`
**Issue**: `LazyLock` regex initialization uses `.unwrap()` which can panic at startup if regex compilation fails.

**Fix**:
```rust
// email.rs - Change from:
static EMAIL_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}").unwrap());

// To:
static EMAIL_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}").expect("VALID_REGEX: email pattern")
});
```
**Rationale**: Startup panic prevention - invalid regex would crash at first use. Apply same pattern to all regex static definitions in both files.

---

### Networking Module

#### `crates/slapper/src/packet/capture.rs:46-53`
**Issue**: System time errors are suppressed (logs warning but returns `Ok(())`), losing packet data silently.

**Fix**:
```rust
pub fn write_packet(&mut self, data: &[u8]) -> std::io::Result<()> {
    let ts = match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!("Failed to get system time: {}", e);
            return Err(e)?;  // Propagate error instead of silent suppression
        }
    };
    // ...
}
```
**Rationale**: Silent data loss - packet timestamps incorrect when clock fails. Error should be propagated to caller.

---

## Wave 2: MEDIUM Priority (Error Handling & Silent Failures)

### AI Module

#### `crates/slapper/src/ai/waf_bypass.rs:204-211`
**Issue**: `persist()` silently ignores file operation failures using `let _ = ...`.

**Fix**: Add error logging:
```rust
if let Err(e) = persist(...) {
    tracing::warn!("Failed to persist WAF bypass knowledge base: {}", e);
}
```
**Rationale**: Silent failure means knowledge base updates are lost without user awareness.

---

### CLI/Commands Module

#### `crates/slapper/src/commands/handlers/report.rs:44-57`
**Issue**: Uses `std::collections::HashMap` for severity counts.

**Fix**:
```rust
use rustc_hash::FxHashMap;
let before_counts: FxHashMap<String, usize> = before
    .findings
    .iter()
    .fold(FxHashMap::default(), |mut acc, f| {
        *acc.entry(f.severity.clone()).or_insert(0) += 1;
        acc
    });
```
**Rationale**: Performance optimization.

#### `crates/slapper/src/commands/handlers/ai_analyze.rs:53-59`
**Note**: This pattern is actually acceptable - nested `.and_then()` with `.unwrap_or()` is safely traversing a JSON-like response structure with a graceful fallback. No fix needed.

---

### Fuzzer Module

#### `crates/slapper/src/fuzzer/calibration.rs:104`
**Issue**: `response.text().await.unwrap_or_default()` silently fails.

**Fix**:
```rust
let body = match response.text().await {
    Ok(text) => text,
    Err(e) => {
        tracing::debug!("Failed to read calibration response body: {}", e);
        String::new()
    }
};
```

#### `crates/slapper/src/fuzzer/payloads/ssti.rs:280`
**Fix**: Same pattern as calibration.rs.

#### `crates/slapper/src/fuzzer/payloads/oauth.rs:559`
**Fix**: Same pattern as calibration.rs.

#### `crates/slapper/src/fuzzer/chain.rs:288,293,298,303`
**Issue**: `unwrap_or_default()` on variable extraction loses debug info when variable not found.

**Fix**:
```rust
.map(|s| s.clone())
.unwrap_or_else(|| {
    tracing::debug!("Variable {} not found in chain execution", "_last_body");
    String::new()
});
```

#### `crates/slapper/src/fuzzer/chain.rs:381`
**Issue**: `LazyLock::new(|| Regex::new(...).unwrap())` - use `expect()` for clearer message.

**Fix**:
```rust
static RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\$\{(\w+)\}").expect("Interpolation regex must be valid")
});
```

---

### Loadtest Module

#### `crates/slapper/src/loadtest/runner.rs:257,292,336`
**Issue**: Lock contention on shared `Arc<Mutex<Metrics>>` creates bottleneck.

**Fix**: Consider per-worker metrics aggregation with atomic counters, merge at end.
**Rationale**: Performance - high concurrency bottleneck. Requires design work (deferred to Future Items).

#### `crates/slapper/src/loadtest/loadtest_tests.rs`
**Issue**: Missing rate limit tests.

**Fix**: Add tests for RPS limits, timeout behavior, connection failures, header/body functionality.

---

### Networking Module

#### `crates/slapper/src/packet/traceroute.rs:361-372`
**Issue**: Catch-all `_` arm silently ignores `JoinError` in traceroute probe task join.

**Fix**: Add debug logging:
```rust
if let Err(e) = join_result {
    tracing::debug!("Traceroute probe task failed: {}", e);
}
```

---

### Pipeline Module

#### `crates/slapper/src/tool/implementations/pipeline.rs:111-112`
**Issue**: `Arc::try_unwrap(...).expect()` can panic if async callback hasn't completed.

**Fix**:
```rust
let result = Arc::try_unwrap(callback)
    .map_err(|e| {
        tracing::warn!("Callback still referenced, using empty result: {}", e);
        Vec::new()
    })
    .unwrap_or_else(|v| v);
```

---

### NSE Module

#### `crates/slapper-nse/src/libraries/datafiles.rs:37,46,48,63,69,77`
**Issue**: `ssh` (lines 37,63), `ntp` (lines 48,69), `mongodb` (lines 46,77) appear twice in `get_services()`.

**Fix**: Remove duplicate entries from initialization.
**Rationale**: Data correctness - indicates copy-paste errors. Later duplicates override earlier ones.

#### `crates/slapper-nse/src/libraries/io.rs:140,163,181,194,211`
**Issue**: `file.get("fd").unwrap_or(-1)` on file descriptor retrieval silently treats missing fd as valid (-1).

**Fix**: Return explicit error when fd is missing:
```rust
let fd: i32 = file.get("fd").ok_or_else(|| {
    tracing::debug!("File descriptor missing from handle");
    -1  // or return error
})?;
```
**Rationale**: Silent masking of bugs - invalid fd could match a previously closed file.

---

### Recon Module

#### `crates/slapper/src/recon/mod.rs:256-264`
**Issue**: `unwrap_or_default()` silently converts serialization errors.

**Fix**:
```rust
m.insert(
    "cname".to_string(),
    serde_json::to_value(&result.target.cname).unwrap_or(serde_json::Value::Null),
);
```

#### `crates/slapper/src/recon/dns_enhanced.rs:225`, `crates/slapper/src/recon/subdomain.rs:141`, `crates/slapper/src/recon/cve_lookup.rs:286`, `crates/slapper/src/recon/containers.rs:237`
**Issue**: Stubbed functions return empty/incomplete results without documentation.

**Fix**: Add `#[allow(dead_code)]` and doc comments:
```rust
/// Zone transfer check - implementation incomplete, returns empty
#[allow(dead_code)]
pub fn check_zone_transfer(...) { ... }
```

---

## Wave 3: LOW Priority (Cleanup, Docs, Optional)

### AI Module

#### `crates/slapper/src/ai/cache.rs:172-182`
**Issue**: Cache eviction removes only one entry at a time.

**Fix**: Loop until under capacity:
```rust
while entries.len() > self.max_entries {
    // Find and remove oldest expired entry
    // If no expired, remove least recently used
}
```

#### `crates/slapper/src/ai/cache.rs:293-304`
**Issue**: `CacheKeyBuilder` uses colon separator which could cause collisions (e.g., `payload:a:b:c` vs `payload:a:b:c` where prefix and args are ambiguous).

**Note**: Informational - current separator unlikely to cause issues in practice, but consider alternative (e.g., `::` or `\x00`) for future-proofing.

---

### Config Module

#### `architecture/config.md:40` and `crates/slapper/src/config/AGENTS.override.md:35`
**Issue**: Documentation references `ScanConfig.profiles` which doesn't exist.

**Fix**: Update to reference `SlapperConfig.profiles`.

#### Config Module Enhancement
**Issue**: Consider adding `--strict-permissions` CLI flag.

**Fix**: Make `check_config_file_permissions()` fail loudly instead of warning.

---

### Loadtest Module

#### `crates/slapper/src/loadtest/metrics.rs:76`
**Issue**: Histogram initialization uses `expect()`.

**Fix**: Change to `unwrap_or_else` with descriptive panic.

#### `architecture/loadtest.md`
**Issue**: Missing `mod.rs` in architecture document.

**Fix**: Add `mod.rs` component and document `run_cli()`.

---

### Networking Module

#### `crates/slapper/src/packet/parse_impl.rs:593,597,724`
**Issue**: Magic numbers not extracted to constants (TLS handshake type `0x16`, TLS version `0x03`).

**Fix**: Create constants:
```rust
const TLS_RECORD_TYPE_HANDSHAKE: u8 = 0x16;
const TLS_VERSION_1_0: u16 = 0x0101;
const IPv4_VERSION_BYTE: u8 = 0x45;
```

#### `crates/slapper/src/stress/udp.rs:379`
**Issue**: `futures::future::join_all(handles).await` silently drops task results - spawned task panics would be ignored.

**Fix**: Add error logging for spawned task failures in `run_udp_flood_spoofed()`.

#### `crates/slapper/src/packet/parse_impl.rs:491`
**Issue**: Parse error details missing.

**Fix**: Add tracing debug:
```rust
tracing::debug!("DNS parse failed: data len {} < 12", data.len())
```

#### `crates/slapper/src/packet/parse_impl.rs:386-387`
**Issue**: `from_utf8_lossy` allocates unnecessarily.

**Fix**: Parse directly from `&[u8]` if possible (informational only - may not apply).

---

### Output Module

#### `crates/slapper/src/output/convert.rs:88-89`
**Issue**: `junit_report.to_xml().unwrap_or_else(|_| "<error>...".to_string())` silently swallows `quick_xml::Error`.

**Fix**: Return `Result<String, String>` with proper error propagation.

#### `crates/slapper/src/output/markdown.rs:133-136`
**Issue**: `report.generate().unwrap_or_else(|_| String::new())` silently suppresses `std::fmt::Error`.

**Fix**: Return `Result<String, std::fmt::Error>` for consistency.

#### Architecture Documentation
**Issue**: Missing files not documented (schedule.rs, ai_schema.rs, escape.rs).

**Fix**: Document in `architecture/output.md`.

---

### Pipeline Module

#### `crates/slapper/src/pipeline/session.rs:15-24`
**Issue**: Blocking file I/O in async context.

**Note**: Acceptable for infrequent checkpointing. Consider `tokio::fs` if throughput becomes issue (deferred to Future Items).

#### `crates/slapper/src/pipeline/executor.rs:368-422`
**Issue**: Manual `FuzzArgs` construction with 30+ fields is fragile.

**Fix**: Consider builder pattern or `Default::default()` + selective override.

---

### Recon Module

#### `crates/slapper/src/recon/runner.rs:863-867`
**Issue**: Test expects `domain` to include port `:8080` but should expect `Some("example.com".to_string())`.

**Fix**: Correct the test assertion.

#### `crates/slapper/src/recon/cve.rs:46,109`
**Issue**: Missing capacity hints.

**Fix**:
```rust
let mut all_vulns = Vec::with_capacity(
    tech_stack.servers.len() + tech_stack.frameworks.len() +
    tech_stack.languages.len() + tech_stack.cms.len() + tech_stack.cdns.len()
);
let mut matched_cves = Vec::with_capacity(cve_map.len().min(20));
```

---

### Scanner Module

#### `crates/slapper/src/scanner/templates/marketplace.rs:266`
**Issue**: `Self::new("https://templates.slapper.io").unwrap()` - `Default::default()` can panic if URL is invalid.

**Fix**: Apply defensive pattern:
```rust
Self::new("https://templates.slapper.io").unwrap_or_else(|e| {
    panic!("TemplateMarketplace initialization failed: {}", e)
})
```

---

### TUI Module

#### `architecture/tui.md:~143`
**Issue**: Key binding table shows `b` for toggle_bookmark but actual is `Ctrl+b`.

**Fix**: Update documentation.

#### `architecture/tui.md:57-61`
**Issue**: Tab Traits section incomplete.

**Fix**: Document additional methods: `handle_up/down/left/right()`, `handle_paste/copy()`, `handle_word_forward/backward()`, `page_up/down()`, etc.

#### `crates/slapper/src/tui/app/state_update.rs:145,157,199,200`
**Issue**: `.clone().unwrap_or_default()` pattern clones before handling Option.

**Fix**: Use explicit match or `unwrap_or(String::new())`:
```rust
// Instead of:
r.waf_name.clone().unwrap_or_default()
// Use:
r.waf_name.as_deref().unwrap_or("")
```

---

### WAF Module

#### `crates/slapper/src/waf/mod.rs:4`
**Note**: Already fixed - docstring now correctly states "Detection of 34 WAF products".

#### `crates/slapper/src/waf/detector/tests.rs:210-211,229-230`
**Note**: Test code - `unwrap_or_else` acceptable.

---

### Architecture Documentation

#### `architecture/overview.md:21`
**Issue**: Payload type count shows "30" but actual is 31 variants in `PayloadType` enum.

**Fix**: Update to "31" to match `fuzzer/payloads/mod.rs:39-70`:
```rust
pub enum PayloadType {
    Sqli, Xss, Traversal, Ssrf, Redirect, Redos, Headers, Compression,
    GraphQL, OAuth, Jwt, Idor, Ssti, Grpc, Xxe, Ldap, Cmd, Deser,
    Host, Cache, Csv, Soap, Websocket, Nosql, Xpath, Expression,
    Prototype, Race, MassAssign, Oast,  // 31 variants
}
```

#### Documentation - Additional
**Issue**: Undocumented modules: auth, browser, compliance, container, integrations, notify, proxy, storage, supply_chain, vuln, websocket, wireless, workflow.

**Note**: Feature-gated optional modules - document if needed.

---

## NSE Library HashMap Replacements (Remaining)

The following NSE library files still use `std::collections::HashMap` instead of `FxHashMap`:

| File | Lines | Type | Notes |
|------|-------|------|-------|
| `slapper-nse/src/libraries/http.rs` | 8, 143-144 | `HashMap<String, String>` | `parse_options` function |
| `slapper-nse/src/libraries/vulns.rs` | 7, 10-11 | `HashMap<&'static str, (&'static str, &'static str, &'static str)>` | CVE database static |
| `slapper-nse/src/libraries/datafiles.rs` | 6, 9-10 | `HashMap<&'static str, u16>` and `HashMap<&'static str, (u16, &'static str)>` | protocols/services |
| `slapper-nse/src/libraries/smbauth.rs` | 7, 10 | `HashMap<String, (String, String)>` | hash store static |
| `slapper-nse/src/libraries/rpc.rs` | 7, 10, 12 | `HashMap<u32, HashMap<u32, &'static str>>` | nested RPC programs |
| `slapper-nse/src/libraries/public_api/api.rs` | 107,108,381,413,463,486,532,1106 | Multiple `HashMap` | CVE database, HTTP headers |
| `slapper-nse/src/libraries/creds.rs` | 102, 123 | `std::collections::HashSet` | local vars for deduplication |

**Fix**: Replace with `rustc_hash::FxHashMap` or `FxHashSet` for consistency and performance.

---

## Phase Summary

### Wave 1 (6 items) - Can execute in parallel:
| Module | Item | File:Lines |
|--------|------|------------|
| ai | planner.rs unwrap→unwrap_or_else | `ai/planner.rs:206-209,467-470,480-483` |
| tool | HashSet→FxHashSet | `tool/planner.rs:4,80,204,247,309,351,386,429,492` |
| fuzzer | targets/api.rs HashMap→FxHashMap | `fuzzer/targets/api.rs:4,12,47,64,77,95,96,101,102,117` |
| plugins_nse | smbauth.rs duplicate function definitions | `slapper-nse/src/libraries/smbauth.rs` - 8 functions (one 3x) |
| recon | email.rs/js.rs regex unwrap→expect | `recon/email.rs:*`, `recon/js.rs:*` |
| networking | capture.rs error propagation | `packet/capture.rs:46-53` |

### Wave 2 (13 items) - Can execute in parallel:
| Module | Item | File:Lines |
|--------|------|------------|
| ai | waf_bypass.rs persist() error logging | `ai/waf_bypass.rs:204-211` |
| cli | report.rs HashMap→FxHashMap | `commands/handlers/report.rs:44-57` |
| fuzzer | calibration.rs/ssti.rs/oauth.rs explicit match | `fuzzer/calibration.rs:104`, `fuzzer/payloads/ssti.rs:280`, `fuzzer/payloads/oauth.rs:559` |
| fuzzer | chain.rs variable extraction + LazyLock | `fuzzer/chain.rs:288,293,298,303,381` |
| loadtest | loadtest_tests.rs rate limit tests | `loadtest/loadtest_tests.rs` |
| networking | traceroute.rs JoinError logging | `packet/traceroute.rs:361-372` |
| pipeline | pipeline.rs Arc::try_unwrap graceful fallback | `tool/implementations/pipeline.rs:111-112` |
| plugins_nse | datafiles.rs duplicate entries | `slapper-nse/src/libraries/datafiles.rs:37,46,48,63,69,77` |
| plugins_nse | io.rs fd error handling | `slapper-nse/src/libraries/io.rs:140,163,181,194,211` |
| recon | mod.rs serialization error | `recon/mod.rs:256-264` |
| recon | Stubbed functions documentation | `recon/dns_enhanced.rs:225`, `recon/subdomain.rs:141`, `recon/cve_lookup.rs:286`, `recon/containers.rs:237` |

### Wave 3 (28 items) - Can execute in parallel:
| Module | Item count | Key Files |
|--------|-----------|-----------|
| ai | 2 | cache.rs:172-182,293-304 |
| config | 2 | architecture/config.md, --strict-permissions |
| loadtest | 2 | metrics.rs:76, architecture/loadtest.md |
| networking | 4 | parse_impl.rs:593,597,724,491, stress/udp.rs:379 |
| output | 3 | convert.rs:88-89, markdown.rs:133-136, architecture/output.md |
| pipeline | 2 | session.rs:15-24, executor.rs:368-422 |
| recon | 2 | runner.rs:863-867, cve.rs:46,109 |
| scanner | 1 | templates/marketplace.rs:266 |
| tui | 3 | architecture/tui.md, state_update.rs:145,157,199,200 |
| waf | 1 | architecture/waf.md (already fixed) |
| architecture | 2 | overview.md:21, undocumented modules |
| NSE libraries | 7 | HashMap replacements across 7 files |

---

## Implementation Notes

### Wave Assignment Rationale

**Wave 1** items are production-safety critical:
- Clock skew panics (`ai/planner.rs`)
- Performance issues in hot paths (`tool/planner.rs`, `fuzzer/targets/api.rs`)
- Code correctness bugs (`smbauth.rs` duplicate functions, `email.rs`/`js.rs` regex panics)
- Silent data loss (`packet/capture.rs`)

**Wave 2** items are error handling improvements within specific modules that don't cause panics but may hide bugs:
- Silent failure suppression in various modules
- Missing error context

**Wave 3** items are cleanup and documentation that don't affect correctness:
- Magic number extraction
- Documentation fixes
- Performance optimizations with minimal impact

### Key Implementation Patterns

1. **FxHashMap/FxHashSet replacements**: All are independent and can be batched per module.

2. **Regex fixes**: Use `.expect()` with descriptive message - invalid regex is compile-time error so these won't actually panic, but the pattern is clearer.

3. **LazyLock pattern**:
```rust
static REGEX_CACHE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(...).expect("DESCRIPTIVE_MESSAGE")
});
```

4. **Arc::try_unwrap pattern**:
```rust
Arc::try_unwrap(callback)
    .map_err(|e| {
        tracing::warn!("Callback still referenced: {}", e);
        Vec::new()
    })
    .unwrap_or_else(|v| v)
```

5. **NSE smbauth.rs deduplication**: Remove second (and third for `signing_hmac_md5`) occurrence of each duplicate function definition.

6. **Session checkpointing**: Per AGENTS.md, checkpoint failures should warn but not abort - intentional behavior.

---

## Future Items (Deferred)

These items require design work or are low priority:

- `loadtest/runner.rs` - Per-worker metrics aggregation with atomic counters
- `pipeline/session.rs` - Async file I/O conversion (current blocking I/O acceptable for infrequent checkpointing)
- `networking/parse_impl.rs` - `from_utf8_lossy` optimization (may not apply - parse may need allocation)
- AI `cache.rs` - CacheKeyBuilder separator collision (informational only)

---

*Consolidated: 2026-05-28*