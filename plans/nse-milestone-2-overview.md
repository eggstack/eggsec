# NSE Milestone 2 Overview: Registry, Semantics, Reports, and Compatibility Truthfulness

## Purpose

Milestone 2 builds on the closed Milestone 1 loader/profile contract. It must not reopen script/module loading policy unless a Milestone 1 regression is found. The loader boundary is now clear: `ScriptResolver` owns script/module file loading, manual and automated profiles have explicit semantics, and architecture guards protect the boundary.

Milestone 2 should make the NSE compatibility layer more truthful, inspectable, and production-grade by introducing a declarative library registry, explicit rule-semantics reporting, structured run reports, a compatibility corpus, and updated docs/release gates.

## Milestone 1 Boundary

Treat the following as fixed constraints:

- `ScriptResolver` remains the only path for user script files and filesystem modules.
- Lua `require()` filesystem loading continues to delegate to `ScriptResolver::resolve_module()`.
- `ManualPermissive` is manual-only.
- Empty manual script roots under `ManualPermissive` mean unrestricted manual script-file selection; extension and size limits still apply.
- Empty module roots mean no filesystem module loading.
- `AgentSafe` and `CiSafe` deny arbitrary script files and filesystem modules before path authorization.
- Rust-side blocking helper cancellation remains Milestone 3 capability-wrapper work.

Milestone 2 should not change these semantics.

## Target End State

At the end of Milestone 2, Eggsec NSE should provide:

1. A declarative registry of NSE libraries, capabilities, side effects, sandbox posture, compatibility status, and known gaps.
2. A truthful rule-semantics layer that reports whether `portrule`, `hostrule`, `prerule`, and `postrule` behavior is exact, approximated, skipped, or unsupported.
3. Structured execution reports that expose profile, limits, resolver diagnostics, library compatibility metadata, rule evaluation metadata, warnings, and compatibility status.
4. A compatibility corpus with representative safe scripts and fixtures covering supported/partial/unsupported behavior.
5. Documentation and guardrails that prevent overclaiming Nmap parity.

## Phase Files

This milestone is split into five detailed phase plans:

1. `plans/nse-milestone-2-phase-01-library-registry.md`
2. `plans/nse-milestone-2-phase-02-rule-semantics.md`
3. `plans/nse-milestone-2-phase-03-structured-run-reports.md`
4. `plans/nse-milestone-2-phase-04-compatibility-corpus.md`
5. `plans/nse-milestone-2-phase-05-docs-release-gate.md`

## Recommended Sequence

Implement the phases in order. Phase 1 provides the metadata vocabulary consumed by later phases. Phase 2 produces rule truthfulness data. Phase 3 exports it as structured reports. Phase 4 verifies the contract with representative scripts. Phase 5 hardens docs, CI, and release criteria.

## Non-Goals

Do not add new low-level NSE libraries unless needed for test fixtures.

Do not attempt full Nmap parity.

Do not redesign Milestone 1 loader/profile policy.

Do not implement Milestone 3 capability wrappers in this milestone.

Do not convert every existing library in one large risky pass if an incremental registry can express unknown/unclassified status.

## Global Acceptance Criteria

Milestone 2 is complete when:

- Every registered NSE library has machine-readable metadata.
- Unknown/unclassified library state is explicit and visible.
- Rule evaluation reports exact/approximate/skipped/unsupported status.
- JSON output includes profile, limit, resolver, rule, library, and compatibility metadata.
- The corpus tests representative working, partial, unsupported, and denied cases.
- Docs state selective compatibility, not full Nmap parity.
- CI or architecture guards prevent bypassing the registry/reporting path for new libraries and run outputs.

## Verification Gate

At the end of every phase, run at least:

```bash
cargo check -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse,sandbox
cargo check -p eggsec --features nse
bash scripts/check-architecture-guards.sh
```

At milestone completion, also run the wider NSE gate documented in `architecture/nse_integration.md`.
