# NSE Production Readiness Roadmap

## Purpose

This roadmap tightens `eggsec-nse` from a broad NSE/Nmap compatibility shim into a production-grade execution layer that is explicit about compatibility, predictable under load, policy-aware across execution surfaces, and safe enough for automated agent/MCP/daemon use.

The target is not full upstream Nmap NSE parity. The target is selective compatibility with clear semantics, accurate output, hardened execution controls, and a documented compatibility matrix.

## Current Baseline

The current structure is a good starting point:

- NSE support is isolated in the `eggsec-nse` crate.
- NSE functionality is feature-gated behind `nse`, `nse-ssh2`, `sandbox`, and related optional features.
- CLI integration routes through central operation enforcement before invoking the NSE crate.
- The crate already has a large library surface and a Lua executor abstraction.
- AGENTS guidance already identifies pending concerns around TOCTOU, DNS rebinding, sandbox behavior, and dead code.

The main production-readiness issues are semantic and control-plane issues rather than lack of breadth:

- Timeout behavior can return to the caller while script execution continues on a spawned thread.
- Sandbox defaults are too implicit for automated use.
- CLI script-file loading bypasses the sandbox resolver path.
- `require` resolution lacks a strict module-name grammar and silently suppresses some load failures.
- Available libraries are represented through multiple parallel lists that can drift.
- The rule engine approximates NSE but lacks a compatibility contract and robust metadata parsing.
- Public API helpers can bypass the same policy/cancellation model expected of Lua-side execution.
- Some protocol helpers return placeholder or inferred fields without enough provenance.
- Structured output is not yet strong enough for production reporting or downstream agent consumption.

## Production Target State

By the end of this line of work, `eggsec-nse` should provide:

1. A single execution policy model used by sync, async, CLI, TUI, daemon, and agent/MCP surfaces.
2. Actual cancellation semantics for timeouts and execution budgets.
3. Named sandbox profiles with stricter defaults for automated surfaces.
4. One canonical script/module resolver for built-in scripts, script files, and `require` loading.
5. A declarative NSE library registry that drives globals, registration, `require`, docs, and tests.
6. Clear distinction between supported, partial, stub, manual-only, and agent-safe functionality.
7. Structured run reports with evidence provenance, warnings, limits, and compatibility notes.
8. A compatibility corpus that guards all claimed NSE behavior.
9. Documentation that accurately describes where Eggsec intentionally diverges from Nmap.

## Milestone 1: Execution Safety Baseline

Milestone 1 is the minimum security and correctness baseline. It should be completed before expanding the NSE library surface further.

Scope:

- Replace misleading timeout behavior with real cancellation or explicitly renamed non-canceling behavior.
- Introduce a unified execution budget model.
- Introduce named sandbox profiles for manual, strict manual, agent-safe, CI-safe, and compatibility-lab use.
- Wire sandbox/profile selection from the central operation enforcement path into `eggsec-nse`.
- Route script-file loading and `require` loading through one canonical resolver.
- Harden module names, path canonicalization, script roots, max script size, and load diagnostics.

Detailed files:

- `plans/nse-milestone-1-execution-safety-baseline.md`
- `plans/nse-milestone-1-phase-01-execution-limits-and-cancellation.md`
- `plans/nse-milestone-1-phase-02-sandbox-profiles-and-policy-wiring.md`
- `plans/nse-milestone-1-phase-03-script-and-module-loading-hardening.md`

Exit criteria:

- A timeout means script work has stopped or the API no longer claims cancellation.
- Agent/MCP/daemon surfaces cannot instantiate a permissive NSE runtime by accident.
- Manual CLI/TUI can remain operator-discretion oriented, but the selected profile is visible and audited.
- Script files and module requires cannot escape approved roots through traversal, symlinks, absolute paths, or fallback string-prefix checks.
- Failing script/module loads return structured diagnostics.

## Milestone 2: Registry and Compatibility Semantics

Milestone 2 addresses maintainability and explicit compatibility.

Scope:

- Replace parallel library lists with a declarative registry.
- Generate or derive Lua globals, registration, `require` exposure, docs, and tests from the registry.
- Add metadata per library: status, feature, aliases, risk class, side-effect class, sandbox requirements, and compatibility notes.
- Improve rule execution semantics around `prerule`, `hostrule`, `portrule`, `postrule`, and action invocation.
- Replace simplistic category parsing with a robust parser for common NSE metadata forms.
- Add a script-plan phase that identifies requirements before execution.

Exit criteria:

- Adding a new NSE library requires one registry entry and one implementation module.
- Drift between globals, registered libraries, and docs is caught by tests.
- Category parsing handles representative real NSE scripts.
- Unsupported NSE semantics fail with explicit compatibility diagnostics.

## Milestone 3: Public API Truthfulness and Structured Output

Milestone 3 turns NSE results into reliable product data.

Scope:

- Route public API network, filesystem, and process interactions through policy-aware wrappers.
- Add cancellation, timeout, byte limits, and audit hooks to Rust-side helpers.
- Classify helper output as observed, inferred, partial, or placeholder.
- Remove or clearly mark synthetic protocol fields.
- Add strict/insecure TLS mode selection and make inspection-mode validation bypass explicit.
- Introduce `NseRunReport` as the internal structured output model.
- Render text and JSON from the same structured model.

Exit criteria:

- No public helper returns placeholder data as observed evidence.
- JSON output is stable enough for downstream reporting and agents.
- Text output is a renderer, not the internal data model.
- Raw side-effect APIs are confined to approved capability wrappers.

## Milestone 4: Compatibility Corpus and Security Review

Milestone 4 creates the verification floor.

Scope:

- Build a fixture corpus for supported NSE behaviors.
- Include positive tests, blocked sandbox tests, timeout tests, output-limit tests, and unsupported-feature diagnostics.
- Add architecture guards for raw network/filesystem/process APIs.
- Write a threat model for Lua execution, script loading, network policy, filesystem policy, DNS rebinding, TLS inspection, and resource exhaustion.
- Add regression tests for identified risks.

Exit criteria:

- Every claimed supported behavior has a fixture.
- Every intentionally unsupported behavior has an intentional failure mode.
- Security review findings are either fixed, documented as accepted manual-mode risk, or blocked from automated surfaces.

## Milestone 5: Documentation, Compatibility Matrix, and Release Gate

Milestone 5 prepares this line of work for a production release.

Scope:

- Publish a compatibility matrix for NSE libraries and scripts.
- Document differences from upstream Nmap NSE.
- Document manual vs agent/MCP behavior.
- Document sandbox profiles and execution limits.
- Add release checks for feature combinations.
- Add JSON schema snapshots for `NseRunReport`.
- Add CLI/TUI smoke tests for built-in scripts and at least one approved script file.

Exit criteria:

- Users can determine before execution whether a script/library is supported, partial, stubbed, manual-only, or agent-safe.
- Release readiness checks cover `nse`, `nse,sandbox`, CLI integration, compatibility corpus, and docs.
- The module can be described honestly as production-grade selective NSE compatibility.

## Sequencing Guidance

Do not expand the number of NSE libraries before Milestone 1 is complete. The current library surface is already broad enough; production risk is concentrated in execution semantics, policy wiring, and truthfulness.

Do not optimize for perfect Nmap parity before defining the Eggsec compatibility contract. The project should prefer predictable, safe, documented behavior over surprising permissive parity.

Do not make agent/MCP behavior depend on manual CLI defaults. Manual usage can preserve user discretion. Automated surfaces need hard boundaries.

## Suggested Verification Commands

Run these at minimum during the line of work, expanding as new tests land:

```bash
cargo check -p eggsec-nse
cargo check -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse,sandbox
cargo check -p eggsec --features nse
make test-nse
```

If architecture guard tests are added, include them in the final release gate and CI matrix.
