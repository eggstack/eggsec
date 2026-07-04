# NSE Milestone 2 Phase 03: Structured NSE Run Reports

## Purpose

Introduce a structured NSE run report that exposes profile policy, execution limits, resolver diagnostics, library compatibility metadata, rule evaluation metadata, warnings, and compatibility status in a stable machine-readable shape.

This phase consumes Phase 01 library registry metadata and Phase 02 rule semantics. It makes the compatibility layer inspectable by CLI, TUI, JSON output, agents, and future report/evidence systems.

## Background

Milestone 1 made execution safer and policy-aware. Phase 01 and Phase 02 make library and rule truthfulness available internally. The next step is to emit that truthfulness consistently. Without a structured report, compatibility status remains scattered across logs, strings, and implicit behavior.

## Non-Goals

Do not redesign the whole Eggsec reporting/evidence subsystem.

Do not add long-form report rendering beyond a concise JSON/human summary.

Do not change loader/profile enforcement semantics.

Do not block manual CLI usage on perfect metadata coverage; represent unknowns explicitly.

## Target State

By the end of this phase, an NSE run can produce a structured report containing:

- target and script identity;
- execution profile and audit label;
- sandbox status and warnings;
- execution limits and observed stats;
- resolver diagnostics;
- loaded libraries and compatibility metadata;
- rule evaluation reports;
- script output;
- compatibility summary;
- errors and warnings;
- machine-readable `exactness` / `fidelity` / `status` fields.

## Proposed Data Model

Add or extend a report module, likely:

```text
crates/eggsec-nse/src/report.rs
```

Suggested types:

```rust
pub struct NseRunReport {
    pub target: String,
    pub script_name: String,
    pub script_source: NseScriptSourceSummary,
    pub profile: NseProfileSummary,
    pub sandbox: NseSandboxSummary,
    pub limits: NseLimitsSummary,
    pub stats: NseExecutionStats,
    pub resolver: NseResolverSummary,
    pub libraries: Vec<NseLibraryUseReport>,
    pub rules: Vec<NseRuleEvaluationReport>,
    pub output: NseOutputSummary,
    pub compatibility: NseCompatibilitySummary,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

pub struct NseCompatibilitySummary {
    pub status: NseRunCompatibilityStatus,
    pub fidelity: NseRunFidelity,
    pub unsupported_features: Vec<String>,
    pub approximations: Vec<String>,
}

pub enum NseRunCompatibilityStatus {
    Compatible,
    CompatibleWithWarnings,
    Partial,
    Unsupported,
    Failed,
    Unknown,
}
```

Keep the first version small and serializable. Prefer `serde::Serialize` on report types.

## Workstream 1: Report Type Skeleton

### Steps

1. Add `report.rs` with serializable report types.
2. Export report types from `lib.rs` under the `nse` feature.
3. Add conversions from existing profile, limit, resolver, and rule types.
4. Avoid storing Lua `Value` directly. Convert outputs into strings/JSON-safe summaries.
5. Add unit tests that serialize a minimal report to JSON.

### Acceptance Criteria

- `NseRunReport` serializes deterministically enough for tests.
- Report types avoid non-serializable Lua internals.
- Missing metadata can be represented as `Unknown` rather than failing serialization.

## Workstream 2: Attach Resolver Diagnostics

### Steps

1. Ensure `ScriptResolver::take_diagnostics()` is called where reports are produced.
2. Include diagnostics for:
   - resolved script source;
   - blocked source;
   - invalid module name;
   - outside-root rejection;
   - symlink rejection;
   - oversized rejection;
   - module load failure.
3. Avoid leaking sensitive absolute paths in agent-safe reports if profile policy requires redaction. For manual CLI, full paths are acceptable.
4. Add a redaction hook or TODO if path redaction policy is not yet formalized.

### Acceptance Criteria

- Report includes resolver diagnostics for script-file and module cases.
- Denied automated file/module attempts are visible in reports.

## Workstream 3: Attach Library Registry Metadata

### Steps

1. Track libraries referenced through `require()` and built-in global modules where feasible.
2. For every referenced/loaded module, look up the Phase 01 registry descriptor.
3. Include status, compatibility level, side effects, sandbox posture, and known gaps in the report.
4. If a library is used but not in the registry, mark it as `Unknown` and emit a warning.
5. Do not fail execution solely because metadata is unknown in this phase, unless the architecture guard already requires metadata for known library files.

### Acceptance Criteria

- Reports can tell users whether a script used partial/stub/side-effecting libraries.
- Unknown library metadata is explicit.

## Workstream 4: Attach Rule Evaluation Metadata

### Steps

1. Include Phase 02 `NseRuleEvaluationReport` values in the run report.
2. Ensure legacy `run_script` output can still be used without reports.
3. Add a new API, for example:

```rust
pub fn run_script_report(&mut self, request: NseRunRequest) -> LuaResult<NseRunReport>;
```

or, if broad API changes are too large:

```rust
pub fn build_last_run_report(&self, context: NseReportContext) -> NseRunReport;
```

4. Ensure rule warnings contribute to the compatibility summary.

### Acceptance Criteria

- Reports show whether rules were exact, approximate, skipped, unsupported, or errored.
- Approximate rule evaluation affects compatibility summary.

## Workstream 5: CLI JSON Output Integration

### Steps

1. Update `run_cli_with_profile()` JSON mode to emit the structured report or include it under a `report` key.
2. Preserve a stable top-level `success` boolean for CLI compatibility if existing consumers rely on it.
3. In non-JSON mode, print a concise human summary:
   - profile;
   - compatibility status;
   - rule status;
   - warnings count;
   - result/output.
4. Avoid dumping full diagnostics in normal human mode unless verbose mode exists.

### Acceptance Criteria

- `--json` exposes structured NSE report data.
- Human output remains readable.
- Existing basic JSON consumers get a clear migration path.

## Workstream 6: Tests

### Required Cases

- manual-permissive script-file success report;
- agent-safe file denial report;
- module invalid-name diagnostic report;
- partial/stub library metadata in report;
- approximate rule status in report;
- Lua error represented as report error;
- JSON serialization snapshot or stable field assertions.

### Acceptance Criteria

- Tests assert field presence, not just successful serialization.
- Reports do not contain raw non-serializable Lua values.

## Documentation

Add a `Structured Reports` section to `architecture/nse_integration.md` documenting:

- report purpose;
- field groups;
- compatibility status meanings;
- relationship to CLI JSON output;
- how agent/MCP callers should consume warnings and partial status.

Update `.opencode/skills/eggsec-nse/SKILL.md` with the new report API once implemented.

## Verification

Run:

```bash
cargo check -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse,sandbox
cargo check -p eggsec --features nse
bash scripts/check-architecture-guards.sh
```

## Final Acceptance Criteria

Phase 03 is complete when:

- NSE run reports exist and serialize to JSON.
- Reports include profile, limits, resolver diagnostics, library metadata, rule metadata, output, warnings, and compatibility summary.
- CLI JSON mode exposes the report or a stable report subset.
- Tests cover success, denial, partial compatibility, rule approximation, and error cases.
- Docs describe how to interpret report status and fidelity.
