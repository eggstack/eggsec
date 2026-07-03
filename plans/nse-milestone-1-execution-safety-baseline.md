# NSE Milestone 1: Execution Safety Baseline

## Purpose

Milestone 1 establishes the minimum safety and correctness baseline for production-grade NSE compatibility in Eggsec.

The goal is to make NSE execution bounded, policy-aware, and honest. A caller should know which execution profile is active, which operations are permitted, which limits apply, and whether a timeout or sandbox violation actually stopped the work.

This milestone should be completed before adding more NSE libraries or broadening script compatibility.

## Scope

Milestone 1 covers three implementation phases:

1. Execution limits and cancellation.
2. Sandbox profiles and policy wiring.
3. Script and module loading hardening.

These phases are intentionally grouped because they depend on each other. Cancellation without sandbox wiring still allows unsafe automated execution. Sandbox profiles without script loading hardening still allow path and module-resolution ambiguity. Script loading hardening without execution limits still leaves denial-of-service and runaway behavior unresolved.

## Non-Goals

This milestone does not attempt full upstream Nmap NSE parity.

This milestone does not add new NSE protocol libraries.

This milestone does not fully rewrite the rule engine.

This milestone does not finalize the public API evidence model or structured report schema, though it should avoid introducing incompatible design choices.

## Design Principles

### Manual and Automated Surfaces Must Diverge Deliberately

Eggsec supports two different operational modes:

- Manual CLI/TUI use, where the human operator has discretion similar to other legitimate security tools.
- Agent/MCP/daemon use, where the software must enforce stricter boundaries because the operator is not directly approving each low-level action.

Milestone 1 should preserve this distinction explicitly. Do not make the manual CLI unusably strict by default, but do not let the agent path inherit manual permissiveness.

### Timeout Must Mean Something Real

A timeout must not merely mean that the caller stopped waiting. It must either halt script work or the API must be renamed/documented to avoid implying cancellation.

The production target is real cancellation through cooperative VM interruption plus cancellation-aware Rust-side capability wrappers.

### Policy Must Flow Into the Runtime

The CLI already has central operation enforcement. That enforcement should produce or select a concrete NSE runtime profile. The executor should not silently fall back to permissive defaults after the policy layer approves an operation.

### Script Loading Is Part of the Security Boundary

Script file loading and `require` resolution are equivalent to code loading. They must be handled by one resolver with canonicalization, allowed roots, maximum sizes, diagnostics, and surface-specific policy.

## Phase Breakdown

### Phase 1: Execution Limits and Cancellation

Add a unified execution budget model and replace the misleading timeout implementation.

Expected outputs:

- `NseExecutionLimits` or equivalent.
- Cancellation token integrated into executor state.
- Lua instruction or hook-based interruption where feasible.
- Output-size, script-size, wall-clock, and operation-count limits.
- Tests for infinite loops, timeout, large output, and repeated side effects.

Detailed plan: `plans/nse-milestone-1-phase-01-execution-limits-and-cancellation.md`.

### Phase 2: Sandbox Profiles and Policy Wiring

Add named profiles and connect the central enforcement result to the NSE runtime.

Expected outputs:

- `NseExecutionProfile` or equivalent.
- Profile presets for manual permissive, manual strict, agent safe, CI safe, and compatibility lab.
- Explicit mapping from CLI/TUI/agent/MCP/daemon surfaces to profiles.
- Derived network allowlists from target/scope where possible.
- Audit output indicating selected profile and limits.

Detailed plan: `plans/nse-milestone-1-phase-02-sandbox-profiles-and-policy-wiring.md`.

### Phase 3: Script and Module Loading Hardening

Create a canonical resolver for built-in scripts, script files, and `require` modules.

Expected outputs:

- `ScriptResolver` or equivalent.
- Strict module-name grammar.
- Canonical path validation under allowed roots.
- Script size limits.
- Structured load errors.
- Removal of direct CLI `std::fs::read_to_string()` execution path.
- Tests for traversal, symlink escape, absolute paths, missing files, and invalid modules.

Detailed plan: `plans/nse-milestone-1-phase-03-script-and-module-loading-hardening.md`.

## Suggested Implementation Order

1. Add new data types without changing runtime behavior.
2. Add tests documenting the current unsafe or ambiguous behavior.
3. Implement execution limits and cancellation.
4. Add profiles and wire manual CLI to explicit manual profile.
5. Wire automated surfaces to strict profile selection.
6. Add script resolver and migrate CLI script-file loading.
7. Migrate `require` resolution to the resolver.
8. Add architecture guard tests for forbidden direct loading and side-effect APIs.
9. Update docs and help text to match actual behavior.

## Acceptance Criteria

Milestone 1 is complete when all of the following are true:

- A timeout cannot leave a script running invisibly after the caller receives an error.
- Infinite-loop Lua scripts are interrupted under configured limits.
- Excessive output is capped and reported as a limit violation.
- Automated surfaces cannot run arbitrary script files by default.
- Automated surfaces cannot use ambient Nmap script paths by default.
- Agent/MCP/daemon execution requires an explicit target/scope-derived network allowlist or fails closed.
- Manual CLI/TUI execution still supports operator-discretion use, but the active profile is visible.
- `script_file` loading goes through the same resolver as other script loads.
- `require` cannot traverse out of allowed roots.
- Invalid module names are rejected before filesystem access.
- Script loading failures return structured diagnostics.
- Documentation and CLI help no longer claim a stricter policy than the code enforces.

## Verification Commands

Run the following during implementation and before handoff:

```bash
cargo check -p eggsec-nse
cargo check -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse,sandbox
cargo check -p eggsec --features nse
make test-nse
```

Add new targeted tests under `eggsec-nse` for cancellation, sandbox profiles, resolver behavior, and CLI integration.

## Handoff Notes

Prefer small, reviewable commits. The safest ordering is type introduction, failing tests, implementation, docs, then cleanup.

Avoid broad protocol-library edits in this milestone unless they are necessary to route side effects through the new limit/profile model.

When in doubt, preserve manual-mode flexibility and tighten automated-mode defaults.
