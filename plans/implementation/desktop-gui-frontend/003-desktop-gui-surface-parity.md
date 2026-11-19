# Desktop GUI Frontend Milestone 003 — Desktop GUI Surface Parity

Status: ready for handoff

Repository baseline: `203ab5dd958415340d39cd47fb1add432c687edd` (`feat/auto-20261022-76692679`)

Source roadmap:

- `plans/subsystems/desktop-gui-frontend-roadmap.md#milestone-3--desktop-gui-surface-parity`

Long-term requirements:

- `plans/000-long-term-specification.md#1-product-definition`
- `plans/000-long-term-specification.md#4-enforcement-invariants`
- `plans/000-long-term-specification.md#5-crate-ownership`
- `plans/000-long-term-specification.md#6-dispatch-runtime-and-wire-identity`
- `plans/002-long-term-roadmap.md#phase-7--standing-maintenance-and-future-capability-open`

Applicable ADRs:

- `plans/adrs/ADR-0007-desktop-gui-frontend-boundary.md`

Primary class: invariant

## 1. Objective

Introduce a `DesktopGuiManual` surface identity that is visible to policy
evaluation and to the audit trail, and prove that it behaves as a **manual**
surface — permissive profile, operator overrides allowed — without weakening
any strict-surface path.

Concretely: add one variant to `RuntimeSurface`, one to `ExecutionSurface`,
the conversion between them, inclusion in `honors_manual_override()`, the
audit label, and the tests and guards that enumerate surfaces.

## 2. Why this milestone is ready

- **No hard dependencies.** This milestone is independent of M001 and M002
  and may proceed in parallel with them.
- **Stable interface dependency.** `ADR-0007` is accepted and fixes the
  semantics: the GUI is a manual surface; a strict GUI variant is **not**
  introduced; `Unknown` remains rejected.
- **Existing pattern to follow.** The CLI and TUI manual surfaces already
  exist and are already exercised. This milestone adds a peer to an existing
  pattern rather than inventing one.

## 3. Current implementation evidence

### The two surface enums

`RuntimeSurface` (`crates/eggsec-runtime/src/request.rs`) has 10 variants:

```rust
pub enum RuntimeSurface {
    CliManual,
    CliManualStrict,
    TuiManual,
    TuiManualStrict,
    Ci,
    McpServer,
    RestApi,
    GrpcApi,
    SecurityAgent,
    #[default]
    Unknown,
}
```

It implements `label()` returning stable lowercase hyphenated strings
(`"cli-manual"`, `"tui-manual"`, `"rest-api"`, …).

`ExecutionSurface` (`crates/eggsec-policy/src/policy.rs`) has 9 variants:

```rust
pub enum ExecutionSurface {
    CliManual,
    TuiManual,
    CliManualStrict,
    TuiManualStrict,
    McpServer,
    SecurityAgent,
    Ci,
    RestApi,
    GrpcApi,
}
```

Its manual-override rule is:

```rust
pub fn honors_manual_override(self) -> bool {
    matches!(self, Self::CliManual | Self::TuiManual)
}
```

`ExecutionSurface` derives the `ExecutionProfile` for its surface, and also
provides `requires_explicit_manifest_for_networked()` (true only for
agent-controlled surfaces) and `label()` returning human-readable labels
(`"CLI manual"`, `"TUI manual"`, …).

### The conversion boundary

`crates/eggsec/src/runtime_bridge/surface.rs` owns the conversion between
the wire-side `RuntimeSurface` and the policy-side `ExecutionSurface`. Per
`plans/000-long-term-specification.md#6-dispatch-runtime-and-wire-identity`,
`runtime_bridge` owns **both** conversion directions and `Unknown` is
rejected. This is the single place that must learn about the new variant.

`crates/eggsec/src/runtime_bridge/executor.rs` reads the acting surface from
the **session** context (`context.surface.clone()`, around line 138), rejects
`RuntimeSurface::Unknown`, and resolves scope with these rules:

- a session with an explicit scope path attempts to load it;
- if loading fails, the executor **fails closed** for strict surfaces and
  falls back to a permissive default for manual surfaces;
- a strict surface with no explicit scope **fails closed**;
- a manual surface with no explicit scope receives a default-empty scope.

The executor's manual-versus-strict decision consults
`ExecutionSurface::honors_manual_override()`.

### The audit path

The daemon records audit events with a `surface` string
(`crates/eggsec-daemon/src/host.rs`, e.g. `"daemon"` for the
`ApprovePolicy` stub). The surface label that reaches the audit trail is
therefore derived from the surface identity established at session creation.

### The surface-parity test

`crates/eggsec/tests/frontend_surface_matrix.rs` (1027 lines) derives a
support matrix from canonical registries — the CLI command registry, the
Clap tree, `ALL_OPERATION_METADATA`, and the runtime `TaskKind` — and
covers workstreams for the support matrix, Clap tree reflection, runtime
mapping guards, and feature-profile parity. It is gated behind
`feature = "cli"` plus `tool-api`, `rest-api`, and `grpc-api`.

The TUI has its own parity tests in `crates/eggsec-tui/src/parity.rs` and
`crates/eggsec-tui/src/tabs/spec.rs`, because the engine must not depend on
the TUI.

### Related guards

`scripts/check-architecture-guards.sh` contains, among others:

- check 11 — runtime is free of TUI dependencies;
- check 58 — `EggsecRuntimeExecutor::execute()` does not hardcode
  `CliManual`;
- check 59 — `EggsecRuntimeExecutor::execute()` does not hardcode
  `default_empty` scope;
- check 79 — the TUI approval cache uses exact binding.

Checks 58 and 59 are directly relevant: they exist to prevent a surface or a
scope from being silently assumed, which is exactly the class of bug a new
surface variant could reintroduce.

## 4. Invariants that must not regress

- `EnforcementContext::evaluate()` remains the mandatory pre-dispatch gate
  for **all** surfaces.
- Strict surfaces dispatch only via `EnforcedDispatcher::dispatch_execution()`
  with an `ApprovedExecution` bundle from the same context.
- Strict-surface scope remains `LoadedScope`, never raw `Scope`; effective
  authorization remains engine-scope ∩ converted-spec, and
  `eggsec::config::scope_from_spec` remains fail-closed.
- `OperationMetadata` remains the single source of truth for operation
  policy. **This milestone MUST NOT change any operation's policy, risk
  tier, exposure flags, or intent.** The GUI is a manual surface, and manual
  surfaces do not gain or lose operation exposure through the surface enum.
- `RuntimeSurface::Unknown` remains rejected by `runtime_bridge`. The new
  variant MUST NOT become a new way to express "unknown".
- `runtime_bridge` remains the single owner of both conversion directions.
  No parallel conversion table may be introduced.
- The manual-override rule MUST NOT widen beyond the CLI and TUI manual
  surfaces by accident. Adding `DesktopGuiManual` to
  `honors_manual_override()` is the intended, explicit change; adding a
  strict variant to it is not.
- Strict surfaces MUST continue to fail closed when scope cannot be
  resolved. The new surface must not participate in that path.
- `OperationMetadata`'s per-surface exposure flags
  (`manual_exposable`, `tui_exposable`, `mcp_exposable`, `rest_exposable`,
  `agent_exposable`, `grpc_exposable`) remain the authority for which
  operations a surface may invoke. **This milestone does not add a
  `gui_exposable` flag** — the GUI is a manual surface and reuses
  `manual_exposable`. Adding a flag would be a second policy surface.
- TLS stays ring-only; workspace Tokio baseline unchanged; MSRV 1.89 and the
  license allow-list intact.
- Never `let _ =` or `filter_map(|e| e.ok())`; log via `tracing`.

## 5. Scope

### In scope

- `RuntimeSurface::DesktopGuiManual` with a stable label.
- `ExecutionSurface::DesktopGuiManual` with a profile and a human-readable
  label.
- The `runtime_bridge` conversion for the new variant, in both directions.
- Inclusion of the new `ExecutionSurface` variant in
  `honors_manual_override()`.
- Audit labeling so GUI-originated activity is distinguishable from
  TUI-originated activity.
- The surface-parity test and any guard that enumerates surfaces.
- Documentation and skill updates listed in §12.

### Explicitly out of scope

- Any strict or headless GUI surface variant. That would require its own
  ADR.
- Any change to `OperationMetadata`, including any new per-surface exposure
  flag.
- Any change to the `ExecutionProfile` set or to how a profile is derived.
- Any change to the daemon protocol.
- Any change to scope resolution or the fail-closed path.
- The GUI crate itself (M004), the frontend-client extraction (M002), and
  schema publication (M001).
- Any change to the TUI's own surface handling beyond what the shared
  conversion requires.

## 6. Required production changes

### Core/domain

- Add `RuntimeSurface::DesktopGuiManual` to
  `crates/eggsec-runtime/src/request.rs` with `label()` returning
  `"desktop-gui-manual"`. Place it beside the other manual variants for
  readability. Do **not** change the `#[default]` variant — `Unknown` stays
  the default.
- Add `ExecutionSurface::DesktopGuiManual` to
  `crates/eggsec-policy/src/policy.rs`. Its derived `ExecutionProfile` must
  be the **same permissive manual profile** that `CliManual` and `TuiManual`
  derive — the point of the milestone is that the GUI is a peer of those
  surfaces, not a new policy class.
- Add `DesktopGuiManual` to `honors_manual_override()`.
- Confirm `requires_explicit_manifest_for_networked()` remains false for the
  new variant — it is a manual, operator-driven surface, not an
  agent-controlled one.
- Update `ExecutionSurface::label()` with a human-readable label
  (for example `"Desktop GUI manual"`).

### Storage and migrations

None. No persisted representation changes. Audit records written after this
milestone may carry the new surface label; historical records are unchanged
and MUST remain readable.

### Protocol and DTOs

- `RuntimeSurface` is a wire DTO serialized in `RunRequest` and in session
  creation. Adding a variant is an **additive** wire change: a client that
  sends the new variant to an older daemon will not deserialize. That is
  acceptable and expected — the GUI requires a matching daemon — and MUST be
  documented in the daemon's version notes. Existing variants MUST NOT
  change shape.
- Consider whether `DaemonCapabilities` should advertise GUI-surface support
  so a client can detect a daemon that does not recognize it. If the
  capability mechanism already carries a version or feature set, extend it;
  otherwise a documented version check is sufficient. **Do not** add an
  ad-hoc version field to an existing message without weighing it against
  the protocol's compatibility rules.

### Runtime and concurrency

- No concurrency change. The executor reads the surface from session state;
  the new variant flows through the same path.

### Frontend or operator surface

- The audit trail must distinguish GUI activity from TUI activity. Verify
  the label that reaches `PersistedAuditEvent.surface` for a GUI session.

### Security and authorization

- The new surface is **manual**: permissive profile, operator overrides
  allowed. The engine evaluates overrides, not the frontend.
- The new surface MUST NOT be added to any strict-surface code path. Check
  the `matches!` arms that decide "is this strict?" and confirm the new
  variant falls on the manual side.
- `Unknown` remains rejected.

### Documentation and static guards

- Extend the frontend surface matrix test to include the GUI surface.
- Add a guard proving `DesktopGuiManual` is present in both enums, that
  `runtime_bridge` converts it in both directions, and that it is included
  in `honors_manual_override()` — so that a future edit cannot silently
  demote the GUI to a non-manual surface or drop the conversion.
- If a skill or architecture document enumerates surfaces, update it.

## 7. Ordered work packages

### Work package A — Extend the wire-side enum

Intent: add the variant where the wire identity lives.

Required changes:

- Add `RuntimeSurface::DesktopGuiManual` and its label.
- Do not change the `#[default]` variant.
- Confirm the enum's serde representation handles the new variant without
  changing existing variants' encodings.

Acceptance evidence:

- A round-trip test proves the new variant serializes and deserializes
  correctly.
- A test proves every existing variant's encoding is unchanged — this
  guards the additive-only requirement.

### Work package B — Extend the policy-side enum and the manual rule

Intent: make the GUI a manual peer of CLI and TUI.

Required changes:

- Add `ExecutionSurface::DesktopGuiManual` deriving the **same** permissive
  manual profile as `CliManual` / `TuiManual`.
- Add it to `honors_manual_override()`.
- Confirm it is excluded from strict-surface arms and from
  `requires_explicit_manifest_for_networked()`.

Acceptance evidence:

- A test asserts the new variant's derived profile equals the profile of
  `CliManual` and `TuiManual` — proving it is not a new policy class.
- A test asserts `honors_manual_override()` is true for the new variant.
- A test asserts the strict-surface arms do not match the new variant.

### Work package C — Extend the runtime bridge conversion

Intent: wire the conversion in both directions, in the one place that owns
it.

Required changes:

- Add the new variant to `crates/eggsec/src/runtime_bridge/surface.rs` in
  both conversion directions.
- Confirm `RuntimeSurface::Unknown` still maps to a rejected state.
- Confirm no parallel conversion table is introduced.

Acceptance evidence:

- A test proves round-trip conversion in both directions for every
  `RuntimeSurface` variant, including the new one.
- A test proves `Unknown` remains rejected.

### Work package D — Audit labeling, tests, guards, and docs

Intent: make GUI activity distinguishable and freeze the properties.

Required changes:

- Verify the audit event's `surface` field carries the GUI label for a
  GUI-created session, and that a TUI-created session still carries the TUI
  label.
- Extend `crates/eggsec/tests/frontend_surface_matrix.rs` to include the
  GUI surface in the matrix.
- Add the guard from §6.
- Update `architecture/config.md`, `architecture/dispatch.md`,
  `architecture/runtime_bridge.md`, `architecture/overview.md`, and
  `docs/ENFORCEMENT_MODES.md` to describe the new surface and its manual
  status.
- Update any skill that enumerates surfaces.

Acceptance evidence:

- An end-to-end test creates a session with the GUI surface, runs an
  operation, and asserts the audit record carries the GUI label.
- The same test asserts a strict surface still fails closed with no
  explicit scope, proving the new variant did not relax that path.
- The matrix test and guards are green.

## 8. Failure, cancellation, restart, and contention semantics

- **Partial failure.** The conversion is total over the enum. A surface that
  cannot be converted must be rejected, not defaulted — the existing
  `Unknown` handling is the model.
- **Duplicate delivery.** Not applicable; this milestone adds no protocol
  command.
- **Process restart.** Sessions created with the GUI surface persist their
  surface across a daemon restart, as other sessions do. Verify that a
  persisted GUI session still resolves its scope under the manual rules
  after restart.
- **Cancellation races.** Not applicable; no task semantics change.
- **Stale generations.** A client sending the new surface to an older daemon
  fails to deserialize. This is a documented version-skew condition, not a
  runtime race.
- **Concurrent callers.** No new shared state. The executor's per-session
  surface read is unchanged.

## 9. Compatibility and migration

- **Wire protocol.** Additive at the enum level: `RuntimeSurface` gains a
  variant. Existing variants' encodings are unchanged — this MUST be proven
  by test. A GUI client requires a daemon that understands the new variant;
  the skew condition is documented rather than negotiated away.
- **Rust API.** `RuntimeSurface` and `ExecutionSurface` are public enums, so
  adding a variant is a source-breaking change for downstream exhaustive
  matches. Within this workspace, the compiler will identify every
  exhaustive match. `eggsec-policy` is not published to crates.io, and
  `eggsec-runtime` is a workspace-internal DTO crate; confirm both before
  treating this as an external break.
- **Enforcement.** No operation's policy, risk tier, exposure flags, or
  dispatch path changes. The GUI surface reuses `manual_exposable` and does
  not add a new exposure flag.
- **Existing frontends.** No behavior change for CLI, TUI, REST, MCP, gRPC,
  or Python. Their surfaces and labels are unchanged.
- **Audit records.** New records may carry `"desktop-gui-manual"`. Consumers
  that filter on known surface labels MUST treat it as a recognized manual
  surface rather than an unknown value. Audit-report consumers are in scope
  for this check.

## 10. Required tests

### Focused unit tests

- `RuntimeSurface::DesktopGuiManual` round-trips through serde.
- Every existing `RuntimeSurface` variant's encoding is unchanged (a
  snapshot of the encodings).
- `ExecutionSurface::DesktopGuiManual` derives the same profile as
  `CliManual` and `TuiManual`.
- `honors_manual_override()` is true for the new variant.
- `requires_explicit_manifest_for_networked()` is false for the new
  variant.
- `ExecutionSurface::label()` returns the expected human-readable label.

### Integration tests

- A session created with the GUI surface evaluates an operation through
  `EnforcementContext` with the GUI surface and dispatches successfully
  under the manual profile.
- A GUI-created session produces an audit record carrying the GUI surface
  label, distinct from the TUI label.
- The frontend surface matrix test includes the GUI surface and is green.

### Restart and recovery tests

- A persisted GUI session, after a daemon restart, still resolves its scope
  under the manual rules and still reports the GUI surface label.

### Contention and cancellation tests

Not applicable. No new concurrency is introduced.

### Security and negative tests

- **A strict surface with no explicit scope still fails closed.** This is
  the load-bearing negative test: it proves adding a manual surface did not
  widen the permissive path.
- `RuntimeSurface::Unknown` is still rejected by `runtime_bridge`.
- The new variant does not match any strict-surface arm.
- The new variant does not cause any operation's exposure flags to change —
  a test comparing the effective exposure for the GUI surface against the
  CLI manual surface's exposure.

### Migration and compatibility tests

- The existing surface-matrix test passes with the GUI surface added rather
  than replacing an existing row.
- A snapshot test of existing surface encodings passes unchanged.

## 11. Required verification commands

```bash
make check                  # mandatory Rust contract
make check-msrv             # no dependency floor change expected
bash scripts/check-architecture-guards.sh
```

The matrix test is feature-gated; run the suite that enables it:

```bash
make test-ci                # -p eggsec --features rest-api,cli
```

Do not claim commands that were not actually run in the closure record.

## 12. Documentation updates

- `architecture/config.md` — the surface list and the manual/strict
  classification.
- `architecture/dispatch.md` — how the GUI surface reaches the enforcement
  gate.
- `architecture/runtime_bridge.md` — the conversion in both directions.
- `architecture/overview.md` — the surface count, if it is stated there.
- `docs/ENFORCEMENT_MODES.md` — the GUI's manual status and override
  allowance.
- `architecture/daemon.md` — the surface label that reaches the audit trail,
  and the version-skew note.
- `plans/registry.md` — this milestone's status.
- The matching skill under `.opencode/skills/` — if it enumerates surfaces
  or states the manual/strict split.

## 13. Acceptance criteria

1. `DesktopGuiManual` exists in both `RuntimeSurface` and
   `ExecutionSurface`, and `runtime_bridge` converts it in both directions.
2. The GUI surface derives the **same** permissive manual profile as the CLI
   and TUI manual surfaces, and `honors_manual_override()` returns true for
   it.
3. Existing surface enum encodings are unchanged, proven by snapshot test.
4. `RuntimeSurface::Unknown` remains the default and remains rejected by
   the runtime bridge.
5. A GUI-created session produces audit records carrying the GUI surface
   label, distinguishable from TUI activity.
6. A strict surface with no explicit scope still fails closed; the new
   variant does not match any strict-surface arm; no operation's exposure
   flags changed.
7. No `OperationMetadata` entry changed, and no new per-surface exposure
   flag was introduced.
8. `make check` is green, the matrix test includes the GUI surface, and the
   new guard passes.

## 14. Stop conditions

The agent must stop and report rather than improvise when:

- adding the variant would require changing an existing variant's encoding
  or the `#[default]` variant — this milestone is additive only;
- the new variant cannot derive the same manual profile as the CLI and TUI
  manual surfaces — report rather than creating a distinct profile;
- any operation's exposure flags would need to change for the GUI to reach
  it — report and stop; that would be a policy change requiring its own
  decision;
- `runtime_bridge` cannot own both conversion directions without a cycle —
  report rather than adding a parallel conversion;
- a strict surface's fail-closed behavior is observed to change — this is a
  contract violation and MUST be reported immediately;
- the audit label for the GUI surface cannot be made distinct from the TUI's
  — report, since misattribution is the point of the milestone.

## 15. Closure evidence required

- The implementation commits or pull requests.
- A requirement-to-evidence matrix mapping each §13 acceptance criterion to
  the specific test or guard that proves it.
- The snapshot output proving existing surface encodings are unchanged.
- The `make check`, `make check-msrv`, guard, and `make test-ci` run
  outcomes, including any command not run and why.
- The evidence that strict-surface fail-closed behavior is unchanged,
  including the negative test's output.
- The evidence that no `OperationMetadata` entry changed — a diff of the
  catalog showing no behavioral change.
- The audit-record evidence showing the distinct GUI label.
- Known limitations — in particular, the version-skew condition where a GUI
  client meets a daemon that does not recognize the new surface.
- Unresolved findings classified by severity.
- Recommendation: closed, conditionally closed, corrective pass required, or
  blocked.

## 16. Handoff notes

- **Baseline.** `203ab5dd958415340d39cd47fb1add432c687edd` on
  `feat/auto-20261022-76692679`.
- **The compiler is your ally here.** Adding an enum variant breaks every
  exhaustive match, which is exactly the review you want. Do not silence
  those errors with a catch-all arm — each one is a site that must decide
  what a GUI surface means.
- **Do not add `gui_exposable`.** The GUI is a manual surface and reuses
  `manual_exposable`. A new exposure flag would be a second policy surface
  and would require its own decision.
- **Do not add a strict GUI variant.** `ADR-0007` explicitly excludes it.
- **Watch checks 58 and 59.** They exist to stop the executor from
  hardcoding a surface or a scope. The new variant must not tempt you into
  either.
- **The matrix test is feature-gated.** It compiles only with
  `cli`, `tool-api`, `rest-api`, and `grpc-api`. Run the suite that enables
  it rather than assuming the default build exercises it.
- **Preserve unrelated user changes** in the worktree.
