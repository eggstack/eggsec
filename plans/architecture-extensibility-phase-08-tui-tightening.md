# Architecture Extensibility Phase 8: TUI Architecture Tightening

## Objective

Tighten the TUI architecture so it consumes canonical command, domain, operation, and enforcement metadata without becoming another source of policy or capability drift. The TUI should remain a manual operator frontend with explicit posture controls, preflight visibility, and ergonomic workflows, while avoiding duplicated descriptor logic and large state-container sprawl.

This phase does not redesign the entire TUI. It incrementally reduces risk by moving the TUI toward metadata-driven tab/action descriptions, clearer state boundaries, and stronger enforcement-preflight consistency.

## Current context

The TUI already has important safety improvements:

- `TuiEnforcementState` supports manual vs guarded posture.
- The TUI can preflight current operations.
- It has task state, tab workflows, overlay/palette behavior, and packaged themes.
- It should remain manual/operator-directed, not agent/autonomous.

Known long-term pressure points:

- `App` is still a large state holder and controller.
- Tab descriptors and operation descriptors can drift from `OperationMetadata` and `DomainDescriptor`.
- CLI-equivalent command preview can drift from command registry/CLI handling.
- TUI-visible actions can become inconsistent with feature gates and manual-only policy.
- Enforcement status presentation can drift from the actual approval path.

## Non-goals

- Do not rewrite the TUI from scratch.
- Do not replace `ratatui` or change the theme system.
- Do not remove manual permissive mode.
- Do not make the TUI a strict programmatic surface by default.
- Do not add new tabs for new capabilities.
- Do not introduce blocking startup work.

## Design target

Move the TUI toward a small set of metadata-backed concepts:

```rust
pub struct TuiActionSpec {
    pub action_id: &'static str,
    pub operation_id: Option<&'static str>,
    pub command_id: Option<&'static str>,
    pub tab_id: &'static str,
    pub feature: Option<&'static str>,
    pub risk_hint: OperationRisk,
    pub manual_only: bool,
    pub descriptor_builder: TuiDescriptorBuilder,
}

pub struct TuiTabSpec {
    pub tab_id: &'static str,
    pub title: &'static str,
    pub domain_id: Option<&'static str>,
    pub feature: Option<&'static str>,
    pub actions: &'static [TuiActionSpec],
}
```

Exact types can differ. The core requirement is that TUI actions and tabs point back to canonical metadata and do not independently invent risk/capability/scope semantics.

## Work item 1: Inventory TUI action and descriptor generation

Document all places where the TUI builds or approximates operation descriptors.

Inspect at minimum:

- `crates/eggsec-tui/src/app/mod.rs`
- `crates/eggsec-tui/src/app/enforcement.rs`
- tab/action modules under `crates/eggsec-tui/src/`
- CLI-equivalent preview code
- preflight rendering/status code
- task spawn paths

Deliverable:

- Add/update `docs/TUI_ARCHITECTURE.md` or a TUI section in `docs/ARCHITECTURE.md`.
- Inventory each tab/action with:
  - tab ID;
  - operation ID;
  - feature gate;
  - descriptor builder location;
  - whether it has CLI-equivalent preview;
  - whether it uses enforcement preflight;
  - whether it spawns side effects.

Acceptance criteria:

- TUI descriptor-generation hotspots are explicit.
- Manual-only and feature-gated actions are identified.

## Work item 2: Add metadata-backed tab/action specs

Introduce a small registry of TUI tab/action specs. Start with a subset; do not convert all tabs at once.

Recommended pilot tabs/actions:

- scan/recon baseline actions;
- db-pentest tab if currently stable and metadata is strong;
- mobile static action if feature-gated behavior is simple.

Avoid starting with mobile dynamic, C2, proxy intercept, raw packet, or advanced wireless unless the TUI path is already clean.

Required behavior:

- Each spec references `OperationMetadata` by operation ID.
- Each feature-gated spec declares a feature string.
- Descriptor builders use metadata as source of risk/capabilities/features where possible.
- TUI display labels can be TUI-specific, but risk and policy semantics cannot be.

Acceptance criteria:

- At least two TUI actions are represented in a metadata-backed registry.
- Registry can be enumerated by tests.
- Existing TUI behavior remains intact.

## Work item 3: Align preflight display with real enforcement

Ensure the TUI's preflight/status display is generated from the same descriptor that would be used for execution.

Required checks:

- Preflight descriptor and execution descriptor are identical or share a common builder.
- Manual posture toggle changes `ExecutionSurface`/profile consistently.
- Warnings vs confirmations vs denials match `EnforcementOutcome`.
- CLI-equivalent preview does not imply an override flag unless the current TUI state actually carries one.

Acceptance criteria:

- Tests cover at least one allowed, one warning/confirmation, and one denied TUI action.
- A descriptor mismatch between preflight and execution would fail a test.

## Work item 4: Reduce `App` god-object pressure

Do a targeted extraction of one or two coherent state/controller responsibilities from `App`.

Candidate extractions:

- `TaskController` for task lifecycle and global task strip state.
- `PreflightController` for current descriptor/outcome/status state.
- `ActionRouter` for mapping key events to action IDs.
- `TabModel`/`TabRegistry` for tab specs.

Do not over-abstract. Prefer one clean extraction with tests over several partial extractions.

Acceptance criteria:

- `App` loses a meaningful responsibility.
- New controller has focused tests.
- TUI behavior remains unchanged.

## Work item 5: Feature-gated tab visibility

Use metadata to ensure feature-gated tabs/actions are presented accurately.

Required behavior:

- Feature-disabled actions are hidden, disabled, or clearly marked unavailable.
- The TUI should not fail to start if an optional feature is absent.
- Missing feature messages should point to the relevant Cargo feature.
- Domain descriptors that exist independent of feature state should not make unavailable actions look executable.

Acceptance criteria:

- Tests verify feature-gated spec availability logic.
- TUI metadata distinguishes known capability from compiled availability.

## Work item 6: Preserve non-blocking startup and theme fallback

Ensure TUI metadata work does not reintroduce startup latency or theme installation coupling.

Required checks:

- No filesystem or network I/O in static tab/action registry construction.
- Theme installation remains non-blocking or graceful.
- Cyber red fallback remains available even if theme directory is unreadable/unwritable.

Acceptance criteria:

- TUI startup path remains fast and non-blocking from user perspective.
- Tests or docs preserve fallback behavior.

## Work item 7: TUI metadata consistency tests

Add tests:

- every TUI action with an operation ID resolves to `OperationMetadata`;
- every TUI action with a domain ID resolves to `DomainDescriptor`;
- feature strings are non-empty and known;
- high-risk TUI actions are manual-only unless explicitly justified;
- descriptor builder risk/capabilities match metadata;
- mobile dynamic remains non-strict and not programmatic.

Suggested file:

- `crates/eggsec-tui/tests/tui_metadata.rs`, or
- `crates/eggsec/tests/tui_metadata.rs` if types live in the main crate.

Acceptance criteria:

- Drift between TUI actions and operation metadata fails tests.

## Safety requirements

- TUI remains manual/operator-directed.
- Strict programmatic surfaces are unaffected by TUI metadata.
- TUI confirmation overlays must not create approvals for automated surfaces.
- Feature absence must not be treated as authorization denial; it is a compile/runtime availability issue.
- Execution still requires `ApprovedOperation` or the existing shared enforcement path.

## Files likely to change

- `crates/eggsec-tui/src/app/mod.rs`
- `crates/eggsec-tui/src/app/enforcement.rs`
- `crates/eggsec-tui/src/**` tab/action files
- optionally `crates/eggsec-tui/src/registry.rs`
- `crates/eggsec/src/domain/mod.rs`
- `crates/eggsec/src/config/policy.rs`
- `docs/ARCHITECTURE.md`
- optionally `docs/TUI_ARCHITECTURE.md`
- tests under `crates/eggsec-tui/tests/` or `crates/eggsec/tests/`

## Validation commands

Run:

```bash
cargo fmt --all --check
cargo check --workspace --no-default-features
cargo test -p eggsec-tui --lib
cargo test -p eggsec --test metadata_consistency
cargo test -p eggsec --test enforcement_matrix
```

Feature checks if pilot tabs are feature-gated:

```bash
cargo check -p eggsec-tui --features mobile
cargo check -p eggsec-tui --features db-pentest
cargo check -p eggsec-tui --features mobile-dynamic
```

Adjust feature commands to match actual feature forwarding in `eggsec-tui`.

## Completion criteria

Phase 8 is complete when:

- TUI action/tab metadata exists for at least two pilot actions.
- Preflight and execution descriptors share a common source.
- One focused piece of `App` state/controller responsibility is extracted.
- Feature-gated availability is clearer and tested.
- TUI remains manual, responsive, and enforcement-consistent.

## Handoff note

This phase should improve maintainability without destabilizing the TUI. Keep the migration incremental. The next TUI pass can expand registry coverage after the first pilot actions prove stable.
