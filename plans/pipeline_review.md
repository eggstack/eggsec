# Pipeline Module Architecture Review

## Verified Claims

### Stage Enum and Profiles
- **Claim**: Stage enum has 7 variants: `PortScan`, `Fingerprint`, `EndpointScan`, `Fuzz`, `LoadTest`, `Waf`, `Recon` - **VERIFIED** (`stage.rs:6-14`)
- **Claim**: All 11 profiles map to correct stage sequences - **VERIFIED** (`stage.rs:31-92`)
- **Claim**: `Stage::from_string()` handles aliases (`portscan`, `fp`, `endpoint-scan`, `graphql`, `oauth`, `jwt`) - **VERIFIED** (`stage.rs:95-109`)

### PipelineContext
- **Claim**: Uses `FxHashMap<u16, ServiceFingerprint>` for services - **VERIFIED** (`context.rs:12`)
- **Claim**: Has fields: `target`, `open_ports`, `services`, `endpoints`, `port_results`, `http_ports` - **VERIFIED** (`context.rs:10-15`)

### Executor
- **Claim**: Sequential execution via `for stage in &self.stages` - **VERIFIED** (`executor.rs:182`)
- **Claim**: `StageResult` has `#[serde(skip)]` on `duration_ms` - **VERIFIED** (`executor.rs:21`)
- **Claim**: `StageResult::new()` constructor exists - **VERIFIED** (`executor.rs:27-35`)
- **Claim**: Progress bar condition is `self.tui_mode || self.stages.is_empty()` - **VERIFIED** (`executor.rs:169`)

### Session
- **Claim**: Session saves to `*.session` or `*.session.json` - **VERIFIED** (`executor.rs:118`)

### Report
- **Claim**: `PipelineReport` has `Display`, `generate_html()`, `generate_csv()` - **VERIFIED** (`report.rs:33,106,204`)
- **Claim**: SARIF/JUnit via `output/` module - **VERIFIED** (`mod.rs:81-91`)

### CLI Entry Points
- **Claim**: `run_cli()`, `run_cli_with_callback()`, `resume_cli()` exist - **VERIFIED** (`mod.rs:111,172,216`)
- **Claim**: `write_output()` helper extracts output writing - **VERIFIED** (`mod.rs:63-95`)

### Recent Bug Fixes (2026-05-22 & 2026-05-27)
- All documented bug fixes are implemented correctly in the codebase

---

## Discrepancies

### 1. Session Checkpoint Timing (Low Impact)
**Doc**: Session checkpoints written "only when output path is explicitly a session-like file name" (`session.rs:72`)
**Impl**: Checkpoint saves *after* each stage completes, regardless of whether session file was requested (`executor.rs:202-212`)

The checkpoint logic correctly gates on `session_path` being `Some`, but the documentation implies checkpoints are only created for session files. The implementation saves to `session_path` if set, which is correct - the doc just over-specifies when sessions are saved.

### 2. StageResult Serialization (Documentation Bug)
**Doc**: "Recent Bug Fixes (2026-05-27) - `StageResult.duration_ms` was serialized to JSON (unnecessary, causes bloat)" - documented as a *fix*
**Impl**: The `#[serde(skip)]` attribute exists (`executor.rs:21`), so the fix is implemented

This is actually a **verified claim** - the bug fix was applied. The documentation correctly notes this.

### 3. PipelineTool Profile Mapping
**Doc**: `PipelineTool` implements `SecurityTool` for AI agent tool registry (`executor.rs:46`)
**Impl**: The tool lists 6 capabilities (`quick`, `endpoint`, `web`, `full`, `api`, `recon`) but the enum has 11 profiles (`pipeline.rs:162-296`)

The tool doesn't expose `waf`, `stealth`, `deep`, `vuln`, `auth` profiles as capabilities. This is an **intentional design** choice (subset of profiles exposed via tool API), not a bug.

---

## Bugs Found

### 1. Session Restoration Incomplete (Medium)
**Location**: `executor.rs:134-140`

```rust
pub fn from_session(session: PipelineSession) -> Self {
    let mut pipeline = Self::new(&session.target);
    pipeline.stages = session.remaining_stages;
    pipeline.context = Arc::new(Mutex::new(session.context));
    pipeline.spoof_config = SpoofConfig::default(); // BUG: loses spoof config
    pipeline
}
```

When resuming a session, `spoof_config` is reset to default, losing IP spoofing settings from the original scan. This could cause resumed scans to behave differently.

**Fix**: Store `spoof_config` in `PipelineSession` and restore it:
```rust
pipeline.spoof_config = session.spoof_config.unwrap_or_default();
```

### 2. Session Context HTTP Ports (Not a Bug)
**Analysis**: After reviewing `context.rs:55-60`, `http_ports` is populated by `update_services()` and is serialized as part of `PipelineContext`. When `from_session()` restores the context, `http_ports` is correctly restored from the serialized state. **No bug exists here.**

### 3. Progress Bar Created for Empty Stages List (Already Fixed)
**Doc**: This was listed as a 2026-05-27 fix
**Impl**: `self.tui_mode || self.stages.is_empty()` check exists at line 169

**VERIFIED FIXED**.

---

## Improvement Opportunities

### 1. Profile-to-Stages Mapping Duplication (Medium)
**Location**: `stage.rs:31-92` and `tool/implementations/pipeline.rs:64-77`

The mapping from `ScanProfile` to stage list is defined in `Stage::from_profile()`. However, `PipelineTool` re-implements this mapping manually when converting profile string to enum.

**Suggestion**: Create a centralized `ScanProfile` metadata table or derive the tool capabilities from the stage definitions automatically.

### 2. Hardcoded Default Ports (Medium)
**Location**: `executor.rs:276-282`

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

Default ports are hardcoded in two places - here and in `get_extended_ports()` at line 534. Should be consolidated.

**Fix**: Extract to a constant or function used by both.

### 3. Session Doesn't Persist `spoof_config` (Medium)
**Location**: `session.rs:7-13` and `executor.rs:134-140`

`SpoofConfig` derives `Serialize` and `Deserialize` (`spoof.rs:29`), so it can be added to `PipelineSession`. Currently:

1. Session saves `target`, `completed_stages`, `remaining_stages`, `context` but **not** `spoof_config`
2. When `Pipeline::from_session()` restores, `spoof_config` is reset to `default()`

This means resumed sessions lose IP spoofing settings from the original scan.

**Fix**: Add `spoof_config: SpoofConfig` to `PipelineSession` and restore it in `from_session()`.

### 4. Missing `OutputFormat` Variants in `write_output` (Low)
**Location**: `mod.rs:63-95`

The `write_output` function handles Html, Json, Csv, Sarif, Junit but there's a logical issue - the `None` case falls through to HTML, but `OutputFormat::Pretty` and `OutputFormat::Compact` also use HTML. This is intentional but undocumented.

**Suggestion**: Add explicit handling for all `OutputFormat` variants to make the mapping clear.

### 5. No Concurrent Stage Execution (Design Limitation)
The documentation describes sequential execution (`for stage in &self.stages`), which is implemented. However, there's no support for stages that could run concurrently (e.g., multiple fuzzing targets).

**Suggestion**: Document this as a known limitation if parallel execution is not planned.

---

## Priority

| Finding | Priority | Type |
|---------|----------|------|
| Session restoration loses `spoof_config` | Medium | Bug |
| Session doesn't persist `spoof_config` | Medium | Bug |
| Hardcoded ports in two locations | Medium | Improvement |
| Profile mapping duplication in tool | Medium | Improvement |
| Missing explicit OutputFormat handling | Low | Improvement |
| No concurrent stage execution | Low | Design Limitation |