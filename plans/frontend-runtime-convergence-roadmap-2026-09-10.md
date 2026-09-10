# Frontend/Runtime Convergence Roadmap — 2026-09-10

## Status

Status: Ready for handoff.

Baseline audited: `main` at `f48b3f37a273e1e53735d98acbb0cb0bcd607260`.

This roadmap is a **post-convergence corrective pass**. It does not reopen the executed 2026-09-08 architecture-convergence roadmap. Phase C of that roadmap successfully centralized canonical operation request/default/validation contracts in `eggsec-tool-core::operation_request` with engine adapters in `eggsec::operation_request`. The remaining work is narrower: eliminate residual dispatch/metadata ownership overlap, close CLI/TUI wiring drift, make frontend parity mechanically testable, and finish the runtime/TUI migration seams that remain visibly transitional.

## Why this work is needed

The current tree has strong canonical policy and request primitives, but the user-facing execution topology still has several overlapping sources of truth.

### Confirmed residuals

1. `crates/eggsec/src/commands/registry.rs` contains the command-to-operation catalog and dispatch classification, but `crates/eggsec/src/commands/handlers/mod.rs::handle_command` currently uses the registry as a validation/tracing bridge before entering a second exhaustive `Commands` match. Registry membership and actual CLI execution routing therefore remain separate ownership points.

2. `crates/eggsec/src/operation_request.rs` is already the intended canonical request adapter layer. Do **not** replace it with a new generic request schema. The corrective work should route frontends through these existing typed contracts and reduce repeated operation/target/dispatch mappings around them.

3. `eggsec-runtime` remains a wire/lifecycle model with its own `RuntimeSurface`, `TaskKind`, parameter structs, and task metadata methods. `RuntimeSurface` explicitly documents itself as a local mirror of `eggsec::config::ExecutionSurface`, and operation identity/target mapping is repeated between runtime types and engine adapters. This is a legitimate serialization boundary, but it should not independently own domain semantics.

4. `crates/eggsec-runtime/src/dispatcher.rs::TaskDispatcher` intentionally puts concrete execution translation in frontend crates. That keeps the runtime crate engine-agnostic, but it also means embedded TUI execution can diverge from CLI/application execution unless a canonical engine dispatcher sits below the runtime adapter.

5. `crates/eggsec/src/dispatch/api.rs::CommandDispatcher` is an application-facing execution API, but parts of its API shape are TUI-oriented (progress channels and comments describing TUI-only routing). The engine execution layer should emit frontend-neutral events/results; terminal rendering and TUI state belong above it.

6. TUI surface metadata remains fragmented. `crates/eggsec-tui/src/app/action_spec.rs` explicitly describes itself as a four-action pilot (`recon`, `scan-ports`, `fuzz`, `db-pentest`) while most tabs continue through the older `TabSpec` path. `Tab`, `Tab::all`, `TabSpec`, `help_entry`, `command_to_tab`, command-palette handling, feature gates, CLI-equivalent generation, and canonical operation IDs are therefore not yet one coherent model.

7. Concrete TUI metadata drift exists at the audited baseline. Examples include TUI operation IDs such as `waf` versus canonical `waf-detect`, `scan-pipeline` versus canonical `pipeline`, and feature ownership that is not consistently expressed between `TabSpec`, `Tab::all`, and Clap command gates (notably stress/packet families). The implementation must determine which differences are intentional aliases/UI shells and which are bugs, then encode that decision in tests rather than comments.

8. `EnforcementFacade` caches an `ApprovedOperation` by operation name. The engine's approval token is descriptor-bound and dispatch performs request-binding validation, so this is not evidence of a policy bypass; however, changing the target/descriptor while retaining the same operation can reuse a stale approval until downstream binding rejects it. The cache should be keyed by the exact descriptor/target contract and invalidated on policy/config/scope changes so the TUI fails early and predictably.

9. The TUI command palette advertises `reload-scope`, but the current implementation reports that reload is unsupported and instructs the operator to restart/use CLI. This is an underdeveloped wiring surface: either make reload atomic and safe or stop advertising it as an actionable command.

10. Maintenance concentration remains high in the TUI (`tabs/core.rs`, `app/command.rs`, `help_config.rs`, `apply.rs`, `export.rs`, enforcement-related modules). Decomposition should follow stabilized action/execution boundaries, not arbitrary line-count targets.

## Research basis

Current Ratatui guidance recommends reifying UI intent into `Action`/message values and separating event mapping, state update, and rendering. This is directly applicable to the current TUI because it enables keybindings, palette commands, and tab controls to converge on one testable action path without putting execution semantics in rendering code:

- https://ratatui.rs/tutorials/counter-async-app/actions/

Clap exposes the derived command tree through `CommandFactory` / `Command` introspection. The repo should use that actual tree in parity tests rather than maintaining only hand-written spot checks of command names and feature gates:

- https://docs.rs/clap/latest/clap/trait.CommandFactory.html
- https://docs.rs/clap/latest/clap/struct.Command.html

The repository's own executed Phase C remains the architectural constraint: canonical typed requests/defaults/validation already exist and should be reused rather than replaced.

## Target architecture

The target flow is:

```text
CLI args / TUI Action / daemon DTO / protocol DTO / Python DTO
                         |
                         v
              shallow surface adapter
                         |
                         v
            canonical operation request
        (existing eggsec-tool-core contracts)
                         |
                         v
         canonical descriptor + enforcement
                         |
                         v
              ApprovedOperation
                         |
                         v
          canonical engine dispatcher
                         |
                         v
        neutral execution events/results
              /                    \
      CLI/output adapter       runtime/TUI adapter
```

Frontend code may own input ergonomics, terminal state, rendering, command aliases, and wire serialization. It must not independently own canonical operation identity, request defaults, target normalization, policy semantics, or execution routing.

## Ordered implementation plans

1. [`frontend-runtime-phase-0-binding-and-parity-guards.md`](frontend-runtime-phase-0-binding-and-parity-guards.md)
2. [`frontend-runtime-phase-1-canonical-dispatch-ownership.md`](frontend-runtime-phase-1-canonical-dispatch-ownership.md)
3. [`frontend-runtime-phase-2-tui-surface-wiring-convergence.md`](frontend-runtime-phase-2-tui-surface-wiring-convergence.md)
4. [`frontend-runtime-phase-3-runtime-contract-hotspot-closure.md`](frontend-runtime-phase-3-runtime-contract-hotspot-closure.md)

Phase 0 is mandatory before structural refactoring because it converts currently implicit parity assumptions into executable guards. Phase 1 then establishes one engine execution seam. Phase 2 simplifies the TUI against that seam. Phase 3 removes remaining runtime mirror drift, decomposes hotspots after responsibilities are stable, and closes the work with measured evidence.

## Cross-phase invariants

The following invariants are non-negotiable throughout the work:

- `ApprovedOperation` remains the required authorization token for protected execution paths.
- Approval is bound to the exact operation descriptor/request target contract; no frontend may widen approval by aliasing or cache reuse.
- Existing canonical request types/defaults/normalizers remain authoritative unless a concrete defect requires a focused migration.
- Helper commands and server-lifecycle commands are not forced into the security-operation catalog merely for architectural symmetry.
- Daemon/runtime wire DTOs may remain distinct for compatibility, but conversions into engine-domain requests must be explicit and exhaustive.
- CLI compatibility aliases and documented Python/protocol names are preserved unless deliberately deprecated with migration documentation.
- Feature-disabled capabilities must fail/appear consistently across CLI, TUI, runtime, and programmatic surfaces. A visible but disabled TUI shell is acceptable only when explicitly modeled as such.
- Cancellation, task completion, error classification, and result semantics must not depend on whether execution is embedded or daemon-backed.

## Feature-overlap decisions this roadmap should produce

The implementation is expected to produce a checked-in support matrix classifying every public surface item as one of:

- canonical operation-backed execution;
- command multiplexer over canonical operations;
- UI-only navigation/inspection state;
- helper/local transformation;
- server/runtime lifecycle;
- intentionally unsupported on a given surface.

This classification is preferable to forcing nominal 1:1 CLI/TUI parity. For example, report/history/settings tabs can legitimately be UI-only, while batch/serialization CLI options can legitimately be CLI-only. The important property is that operation-backed execution has one semantic owner and that unsupported combinations are explicit.

## Verification contract

Use repository-native verification as the minimum gate:

```text
make fmt
make test-feature-matrix
make test-architecture-guards
make check
```

Run `make check-python` when shared request/operation metadata touched by Python is changed. Run `make check-feature-profiles` for feature-routing work and `make check-features-individual` when feature declarations or exhaustive feature mappings change. Run `make check-full` before final closure where environment prerequisites permit it.

Add targeted frontend/runtime parity tests to the normal mandatory check path before declaring this roadmap complete; do not leave the new invariants in an optional/manual suite only.

## Non-goals

This roadmap is not a CLI UX redesign, not a Ratatui rewrite, not a daemon protocol redesign for its own sake, and not a security-feature expansion. It should not change scan aggressiveness, introduce new offensive capability, or broaden authorization behavior. It is architecture, correctness, testability, wiring, and maintenance work around existing functionality.

## Completion evidence

The final phase must append a completion record to each plan or a dedicated closure document containing:

- starting and final SHAs;
- files/modules materially changed;
- before/after counts for independently maintained command/operation/TUI mappings;
- operation/surface support-matrix totals;
- parity test totals and feature profiles exercised;
- mandatory verification results;
- any intentionally retained duplicate representation and why it remains a necessary wire/UI boundary.
