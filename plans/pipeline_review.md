# Pipeline Module Architecture Review

**Date:** 2026-05-23
**Reviewer:** Architecture Review
**Files Analyzed:**
- `architecture/pipeline.md`
- `crates/slapper/src/pipeline/stage.rs`
- `crates/slapper/src/pipeline/executor.rs`
- `crates/slapper/src/pipeline/context.rs`
- `crates/slapper/src/pipeline/session.rs`
- `crates/slapper/src/pipeline/report.rs`
- `crates/slapper/src/pipeline/mod.rs`
- `crates/slapper/src/tool/implementations/pipeline.rs`

---

## Verified Claims

### 1. Stage Enum and Available Stages
**Document says:** Section "Stage (`stage.rs`)" lists 7 stages: `PortScan`, `Fingerprint`, `EndpointScan`, `Fuzz`, `LoadTest`, `Waf`, `Recon`.

**Implementation:** `stage.rs:6-14` confirms the enum has exactly these 7 variants. Verified.

### 2. Profiles Table Accuracy
**Document says:** Table at lines 23-35 shows stage compositions for all 11 profiles.

**Implementation:** `stage.rs:31-93` implements `Stage::from_profile()` with matching profiles. All profiles match:
- `quick` → PortScan, Fingerprint
- `endpoint` → PortScan, Fingerprint, EndpointScan
- `web` → PortScan, Fingerprint, EndpointScan, Fuzz
- `full` → PortScan, Fingerprint, EndpointScan, Fuzz, LoadTest
- `waf` → PortScan, Fingerprint, EndpointScan, Waf
- `api` → PortScan, Fingerprint, EndpointScan, Fuzz
- `recon` → PortScan, Fingerprint, EndpointScan, Recon, Fuzz
- `stealth` → PortScan, Fingerprint, EndpointScan, Fuzz
- `deep` → PortScan, Fingerprint, EndpointScan, Fuzz
- `vuln` → PortScan, Fingerprint, EndpointScan, Recon, Fuzz
- `auth` → PortScan, Fingerprint, EndpointScan, Fuzz

Verified. All 11 profiles match exactly.

### 3. Stage Aliases
**Document says:** "Aliases" include `portscan`, `fp`, `endpoint-scan`, `graphql`, `oauth`, `jwt`.

**Implementation:** `stage.rs:95-109` `Stage::from_string()` handles all listed aliases. Verified.

### 4. Sequential Execution
**Document says:** "Sequential Execution: Stages run in linear order (`for stage in &self.stages`)."

**Implementation:** `executor.rs:195` shows `for stage in &self.stages`. Verified.

### 5. Result Passing via PipelineContext
**Document says:** "Output from one stage is persisted into `PipelineContext` and consumed by later stages."

**Implementation:** `context.rs:9-16` shows `PipelineContext` stores `open_ports`, `services`, `endpoints`, `port_results`, `http_ports`. Methods `update_ports()`, `update_services()`, `update_endpoints()` maintain state between stages. Verified.

### 6. Failure Recording
**Document says:** "Stage errors are recorded per stage in `StageResult` and surfaced in the report."

**Implementation:** `executor.rs:19-25` `StageResult` has `success`, `error` fields populated at lines 203-208. `report.rs:85-91` has `has_failures()` and `first_failed_stage()` methods. Verified.

### 7. CLI Entry Points
**Document says:** "`run_cli(args, config)`", "`run_cli_with_callback(args, config, callback)`", "`resume_cli(args)`".

**Implementation:** `mod.rs:111` (`run_cli_with_callback`), `mod.rs:172` (`run_cli`), `mod.rs:216` (`resume_cli`). All three exist. Verified.

### 8. Session Checkpoint Logic
**Document says:** "Session checkpoints are written only when output path is explicitly a session-like file name (`*.session` or `*.session.json`)".

**Implementation:** `executor.rs:117-120` filters `session_path` with `ends_with(".session.json") || ends_with(".session")`. Verified.

### 9. Report Output Formats
**Document says:** "`Display` - Human-readable console output, `generate_html()` - Styled HTML report, `generate_csv()` - CSV report, SARIF/JUnit via `output/` module".

**Implementation:** `report.rs:33-82` (`Display` impl), `report.rs:106-202` (`generate_html()`), `report.rs:204-258` (`generate_csv()`). SARIF/JUnit in `mod.rs:81-92` using `crate::output::SarifBuilder` and `JUnitBuilder`. Verified.

### 10. PipelineTool Implements SecurityTool
**Document says:** "`PipelineTool` implements `SecurityTool` for AI agent tool registry."

**Implementation:** `tool/implementations/pipeline.rs:31-292` implements `SecurityTool` trait. Verified.

### 11. PipelineContext Uses FxHashMap
**Document says:** (Recent Bug Fixes 2026-05-22) "`PipelineContext.services` used `HashMap` instead of `FxHashMap`".

**Implementation:** `context.rs:12` shows `pub services: FxHashMap<u16, ServiceFingerprint>`. Fixed. Verified.

### 12. StageResult.duration_ms Serialization Skip
**Document says:** (Recent Bug Fixes 2026-05-27) "`StageResult.duration_ms` was serialized to JSON... Added `#[serde(skip)]`".

**Implementation:** `executor.rs:21-22` has `#[serde(skip)]`. Fixed. Verified.

### 13. StageResult Constructor
**Document says:** (Recent Bug Fixes 2026-05-27) "Added `StageResult::new()` constructor".

**Implementation:** `executor.rs:27-35` implements `StageResult::new()`. Fixed. Verified.

### 14. Progress Bar Empty Stage Fix
**Document says:** (Recent Bug Fixes 2026-05-27) "Progress bar created even for empty stage list... Changed condition to `self.tui_mode || self.stages.is_empty()`".

**Implementation:** `executor.rs:182` has `if self.tui_mode || self.stages.is_empty()`. Fixed. Verified.

### 15. write_output() Helper Function
**Document says:** (Recent Bug Fixes 2026-05-27) "Extracted to `write_output()` helper function in `mod.rs:63-95`".

**Implementation:** `mod.rs:63-95` defines `async fn write_output(...)`. Fixed. Verified.

---

## Discrepancies

### 1. run_cli() and run_cli_with_callback() Code Duplication NOT Fixed
**Document says:** (Recent Bug Fixes 2026-05-27) "Fixed: `run_cli()` and `run_cli_with_callback()` had duplicated output writing code. Extracted to `write_output()` helper function in `mod.rs:63-95`."

**Reality:** While `write_output()` was extracted, the functions still have extensive duplication:
- Lines 119-157 (`run_cli_with_callback`) vs Lines 183-201 (`run_cli`) share nearly identical verbose printing, JSON printing, report printing, and output file writing logic.

**Impact:** Medium - Maintenance burden, not a bug.

**Reference:** `mod.rs:111-169` vs `mod.rs:172-213`

---

## Bugs Found

### 1. Missing return when session save fails
**File:** `executor.rs:223-226`
```rust
if let Err(e) = save(path, &session) {
    tracing::warn!("Failed to save session to {:?}: {}", path, e);
}
```
The session save failure is only warned, not propagated. If disk is full or permissions denied, user is not informed.

**Severity:** Medium
**Impact:** User may not know their session wasn't saved, leading to lost work on interrupt.

---

### 2. run_fingerprint() uses hardcoded port list when context.open_ports is empty
**File:** `executor.rs:318-324`
```rust
let ports: Vec<u16> = if context.open_ports.is_empty() {
    vec![
        21, 22, 23, 25, 53, 80, 110, 143, 443, 445, 993, 995, 1433, 1521, 3306, 3389, 5432,
        5900, 6379, 8080, 8443, 27017, 9092, 9200, 5672, 2181, 2375, 2376, 6443, 10250,
    ]
} else {
    context.open_ports.clone()
};
```
This hardcoded list is nearly identical to `EXTENDED_SCAN_PORTS` in `stage.rs:120` but excludes many ports (e.g., 3000, 4200, 5000, 8000, 9000, 5601, 9090). If no ports found by port scan, fingerprinting uses an incomplete list.

**Severity:** Medium
**Impact:** Fingerprinting may miss services when port scan finds no open ports but services exist on non-standard ports.

---

### 3. run_concurrent() doesn't support session persistence
**File:** `executor.rs:245-271`
```rust
async fn run_concurrent(&self) -> Result<PipelineReport> {
    // ... no session_path handling
}
```
The concurrent execution path (`run_concurrent()`) does not save session checkpoints. Only sequential path (`run()` at lines 215-226) saves sessions.

**Severity:** Low
**Impact:** Users who enable concurrent stages lose session persistence benefit.

---

### 4. concurrent_stages field never set from args
**File:** `executor.rs:127`
```rust
concurrent_stages: false,
```
In `from_args_with_tui_mode()`, `concurrent_stages` is always `false` regardless of what was passed. There's a `with_concurrent_stages()` builder method, but no way to set it from CLI args.

**Severity:** Medium
**Impact:** The concurrent stages feature is available via builder pattern but not accessible from CLI.

---

### 5. PipelineContext.http_ports field never explicitly maintained
**File:** `context.rs:15` declares `pub http_ports: Vec<u16>`, and `update_services()` at line 59 calls `self.get_http_ports()` to populate it. However, `http_ports` is a separate field that could get out of sync if `services` is modified directly. The field is redundant since `get_http_ports()` computes the same data on demand.

**Severity:** Low
**Impact:** Minor code smell; redundant storage.

---

### 6. run_recon() ignores `config` parameter passed to it
**File:** `executor.rs:567-569`
```rust
let config = self.config.as_ref().unwrap_or(&default_config);
crate::recon::run_cli(args, config).await?;
```
The `run_recon` method correctly uses config. But `run_waf()` at line 536 calls `crate::waf::run_cli(args).await?` without passing config, despite WAF potentially needing HTTP/TLS settings from config.

**Severity:** Medium
**Impact:** WAF detection may not respect configured TLS verification settings.

---

### 7. Default concurrency is 10 regardless of config
**File:** `executor.rs:88-96`
```rust
let concurrency = if let Some(cfg) = config {
    if args.concurrency == 10 {
        cfg.scan.default_concurrency
    } else {
        args.concurrency
    }
} else {
    args.concurrency
};
```
Default `args.concurrency` is 10 (from CLI definition). If user doesn't specify `--concurrency`, the config value is used only if it differs from 10. This is a reasonable fallback but the logic is non-obvious.

**Severity:** Low
**Impact:** Could be confusing but works correctly.

---

## Improvement Opportunities

### Priority: High

#### 1. Fix hardcoded fallback port list in run_fingerprint()
**File:** `executor.rs:318-324`

**Suggestion:** Use `EXTENDED_SCAN_PORTS` as fallback instead of hardcoding a separate list. This ensures consistency and completeness.

**Estimated Impact:** Fingerprinting will be more reliable when port scan finds no results.

---

#### 2. Propagate session save errors properly
**File:** `executor.rs:215-226`

**Suggestion:** Change from `tracing::warn` to return error, or add a metrics counter for failed saves.

**Estimated Impact:** Users will know when their session wasn't saved.

---

### Priority: Medium

#### 3. Add CLI flag for concurrent stages
**File:** Need new CLI arg in `ScanArgs`

**Suggestion:** Add `--concurrent-stages` flag to enable parallel stage execution.

**Estimated Impact:** Enables the already-implemented `run_concurrent()` feature via CLI.

---

#### 4. Pass config to run_waf()
**File:** `executor.rs:536`

**Suggestion:** Change `crate::waf::run_cli(args).await?` to `crate::waf::run_cli_with_config(args, config).await?` (or equivalent).

**Estimated Impact:** WAF detection will respect TLS verification settings from config.

---

#### 5. Deduplicate run_cli() and run_cli_with_callback()
**File:** `mod.rs:111-169` and `mod.rs:172-213`

**Suggestion:** Extract common logic into a private async function like `execute_pipeline_and_output()`.

**Estimated Impact:** Reduced maintenance burden; easier to ensure both functions behave identically.

---

#### 6. Add session persistence to run_concurrent()
**File:** `executor.rs:245-271`

**Suggestion:** Either disable concurrent mode when session path is set, or implement session saves in concurrent path.

**Estimated Impact:** Consistent session behavior regardless of execution mode.

---

### Priority: Low

#### 7. Remove redundant http_ports field
**File:** `context.rs:15,59`

**Suggestion:** Remove `http_ports` field and `get_http_ports()` call in `update_services()`. Let callers use `get_http_ports()` directly.

**Estimated Impact:** Simplified state management; slightly less memory usage.

---

#### 8. Document concurrent_stages behavior
**File:** `executor.rs:43`

**Suggestion:** Add doc comment explaining that `concurrent_stages` runs all stages in parallel without dependency ordering.

**Estimated Impact:** Better developer understanding of the feature.

---

## Summary

| Category | Count |
|----------|-------|
| Verified Claims | 15 |
| Discrepancies | 1 (code duplication) |
| Bugs Found | 7 |
| High Priority | 2 |
| Medium Priority | 4 |
| Low Priority | 3 |

**Overall Assessment:** The architecture document is accurate and well-maintained. The implementation matches the documented design in all significant aspects. The recent bug fixes documented are correctly implemented. The main opportunities for improvement are around error handling propagation (session save failures), consistency (using `EXTENDED_SCAN_PORTS` everywhere), and enabling existing features (concurrent stages via CLI, config passed to WAF).
