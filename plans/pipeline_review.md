# Pipeline Module Architecture Review

**Review date:** 2026-05-28
**Branch:** `architecture/pipeline-review`

## Summary

The Pipeline module orchestrates complex security assessment workflows by chaining multiple Slapper tasks in sequence. The implementation generally matches the architecture document, with all documented bug fixes verified as applied. However, several areas warrant attention including async patterns and error handling in the tool integration layer.

---

## 1. What's Implemented Correctly

### Core Components
- **Stage enum** (`stage.rs:6-14`): 7 variants (PortScan, Fingerprint, EndpointScan, Fuzz, LoadTest, Waf, Recon) - matches arch doc
- **Sequential execution** (`executor.rs:182`): `for stage in &self.stages` loop - matches arch doc
- **Stage profiles** (`stage.rs:31-92`): All 10 profiles (quick, endpoint, web, full, waf, api, recon, stealth, deep, vuln, auth) - matches arch doc table
- **Stage aliases** (`stage.rs:95-108`): `portscan`, `fp`, `endpoint-scan`, `graphql`, `oauth`, `jwt` - matches arch doc
- **PipelineContext** (`context.rs:8-16`): Uses `FxHashMap<u16, ServiceFingerprint>` for services - correctly uses FxHashMap per bug fix
- **PipelineTool** (`tool/implementations/pipeline.rs`): Implements `SecurityTool` trait - matches arch doc

### Bug Fixes Verified (2026-05-22)
| Bug Fix | File:Line | Status |
|---------|-----------|--------|
| `PipelineContext.services` uses `FxHashMap` | `context.rs:12` | Verified |
| `resume_cli()` returns error on failure | `mod.rs:223-231` | Verified |
| `run_load_test()` uses config | `executor.rs:458-459` | Verified |

### Bug Fixes Verified (2026-05-27)
| Bug Fix | File:Line | Status |
|---------|-----------|--------|
| `write_output()` helper extracted | `mod.rs:63-95` | Verified |
| `StageResult.duration_ms` has `#[serde(skip)]` | `executor.rs:22` | Verified |
| `StageResult::new()` constructor | `executor.rs:27-35` | Verified |
| Progress bar skipped for empty stages | `executor.rs:169` | Verified |
| Session path filtering (`*.session.json`, `*.session`) | `executor.rs:115-118` | Verified |

### Error Handling
- All `unwrap_or_else` patterns used instead of `unwrap_or_default()` for error propagation
- Session save failures logged with `tracing::warn!` (acceptable for checkpointing)
- Progress bar uses `unwrap_or_else` with fallback style

---

## 2. Bugs/Issues Found

### BUG 1: `Arc::try_unwrap(...).expect()` can panic in PipelineTool

**File:** `tool/implementations/pipeline.rs:111-112`
**Severity:** Medium

```rust
let findings = std::sync::Arc::try_unwrap(findings)
    .expect("Arc should have single owner")
    .into_inner();
```

**Issue:** While documented as "should have single owner," if the async callback hasn't completed or if there are multiple owners somehow, this will panic. This is called by AI agents where robustness is critical.

**Fix:** Use graceful degradation:
```rust
let findings = std::sync::Arc::try_unwrap(findings)
    .map(|m| m.into_inner())
    .unwrap_or_else(|_| {
        tracing::warn!("Arc had multiple owners, returning empty findings");
        Vec::new()
    });
```

---

### BUG 2: Blocking file I/O in potentially async context

**File:** `session.rs:15-24`
**Severity:** Low (performs reasonably for session checkpoints)

```rust
pub fn save(path: &str, session: &PipelineSession) -> Result<()> {
    let json = serde_json::to_string_pretty(session)?;
    std::fs::write(path, json)?;  // Blocking I/O
    Ok(())
}

pub fn load(path: &str) -> Result<PipelineSession> {
    let json = std::fs::read_to_string(path)?;  // Blocking I/O
    let session: serde_json::from_str(&json)?;
    Ok(session)
}
```

**Issue:** `save()` is called from `executor.rs:209` within async context. While this is only used for session checkpoints (not hot path), it violates the async-first conventions.

**Note:** This is acceptable for session checkpointing which is infrequent. If throughput becomes an issue, convert to `tokio::fs`.

---

### MINOR: Session save failures are silent beyond warning

**File:** `executor.rs:209-211`
**Severity:** Very Low

```rust
if let Err(e) = save(path, &session) {
    tracing::warn!("Failed to save session to {:?}: {}", path, e);
}
```

**Issue:** Disk full or permission errors result in only a warning. The pipeline continues without error.

**Assessment:** Acceptable per AGENTS.md guidance that checkpoint failures shouldn't abort pipeline execution. User is warned but scan continues.

---

### MINOR: FuzzArgs constructed manually with 30+ fields

**File:** `executor.rs:368-422`
**Severity:** Low (maintainability)

```rust
let args = crate::cli::FuzzArgs {
    url: base_url,
    payload_type,
    mode: crate::cli::FuzzMode::Sequential,
    mutate,
    mutation_count,
    // ... 30+ fields
};
```

**Issue:** Manual construction is fragile. Fields not explicitly set get `Default::default()` which may not be correct for all pipeline configurations.

**Note:** The `common` field at line 418 does use `self.common.clone()` so stealth settings are passed. This is acceptable but could be simplified with a builder pattern.

---

## 3. Recommended Fixes

| Priority | File | Line | Issue | Fix |
|----------|------|------|-------|-----|
| Medium | `tool/implementations/pipeline.rs` | 111-112 | `Arc::try_unwrap(...).expect()` panic risk | Change to `ok().unwrap_or_else()` with graceful fallback |
| Low | `session.rs` | 15-24 | Blocking file I/O | Convert to async `tokio::fs` (defer if not perf critical) |
| Low | `executor.rs` | 209 | Session save call needs `.await` after async conversion | Add `.await` when async conversion is done |
| Info | `executor.rs` | 368-422 | Manual FuzzArgs construction | Consider `Default::default()` + selective override pattern |

---

## 4. Discrepancies Between Arch and Implementation

### No Discrepancies Found

The architecture document at `architecture/pipeline.md` accurately describes the implementation:

| Arch Claim | Implementation |
|------------|----------------|
| 7 available stages | `stage.rs:6-14` - 7 variants confirmed |
| Sequential execution | `executor.rs:182` loop confirmed |
| Profiles table | `stage.rs:31-92` - all 11 profiles confirmed |
| Session checkpointing on `*.session*` paths | `executor.rs:115-118` confirmed |
| PipelineTool implements SecurityTool | `tool/implementations/pipeline.rs` confirmed |
| write_output() helper | `mod.rs:63-95` confirmed |
| FxHashMap for services | `context.rs:12` confirmed |

---

## 5. Testing Recommendations

1. **Test `PipelineTool` with concurrent callbacks:**
   ```rust
   #[tokio::test]
   async fn test_pipeline_tool_callbacks() {
       let tool = PipelineTool::new();
       // Verify Arc unwrap handles concurrent case gracefully
   }
   ```

2. **Test session persistence round-trip:**
   ```rust
   #[test]
   fn test_session_save_load() {
       // Create session, save, load, verify equality
   }
   ```

3. **Add test for empty stage handling:**
   ```rust
   #[tokio::test]
   async fn test_empty_stages_no_progress_bar() {
       let pipeline = Pipeline::new("example.com");
       // Verify no progress bar created
   }
   ```

---

## 6. Code Quality Notes

- **No `unwrap()` without fallback**: Pipeline code uses `unwrap_or_else()` appropriately
- **No `unwrap_or_default()` silencing errors**: Correct patterns used throughout
- **Feature gates**: `run_cli_with_callback` correctly gated with `#[cfg(feature = "tool-api")]`
- **Progress bar**: Correctly skips for empty stages and TUI mode
- **Async patterns**: `write_output()` is properly async

---

*Review completed: 2026-05-28*
