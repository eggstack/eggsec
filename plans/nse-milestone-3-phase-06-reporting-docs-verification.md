# NSE Milestone 3 Phase 06: Reporting, Docs, Guards, and Verification Closure

## Purpose

Close Milestone 3 by making capability-wrapper enforcement visible, documented, tested, and guarded against regression.

Phases 01 through 05 inventory and migrate helper-side enforcement. This final phase aligns structured reports, documentation, architecture guards, compatibility claims, and verification records.

## Background

Capability wrappers are only useful if their outcomes are visible. Users and agents need to know when helper operations were allowed, denied, bounded, cancelled, approximated, or deferred. Future contributors need guardrails that prevent direct side-effect calls from bypassing wrappers.

## Non-Goals

Do not add new helper migrations unless a closure blocker is found.

Do not claim full Nmap parity.

Do not weaken manual mode to match automated mode.

Do not hide deferred helper classes.

## Target State

By the end of this phase:

- `NseRunReport` exposes capability events or equivalent helper-side warnings.
- Docs describe capability coverage and known gaps.
- Architecture guards prevent new direct side-effect helper bypasses.
- Compatibility matrix indicates helper-side enforcement status.
- Verification record documents commands run and results.
- Milestone 3 closure note is present.

## Workstream 1: Structured Report Integration

### Required Report Content

Reports should include, either as first-class fields or warnings/diagnostics:

- capability kind;
- operation name;
- target/path/host summary with redaction where appropriate;
- allowed or denied decision;
- denial reason;
- bytes/operations counted where available;
- cancellation/timeout/limit violation status;
- profile that made the decision.

### Implementation Steps

1. If Phase 02 added `NseCapabilityEvent`, add it to `NseRunReport` as a serializable field.
2. If a first-class field is too broad, convert events into stable warnings and compatibility summary entries.
3. Ensure helper denials affect compatibility status.
4. Ensure cancellation and limit violations are visible in `stats` or `compatibility.unsupported_features`.
5. Add serialization tests.

### Acceptance Criteria

- At least one helper denial appears in JSON output.
- Reports distinguish denied helper operation from Lua script failure.

## Workstream 2: Documentation Closure

### Files to Update

- `architecture/nse_integration.md`
- `architecture/nse_capability_inventory.md`
- `.opencode/skills/eggsec-nse/SKILL.md`
- `crates/eggsec-nse/AGENTS.override.md`
- README if needed

### Required Documentation

Add sections for:

- capability wrapper architecture;
- profile behavior summary;
- filesystem/process coverage;
- network/DNS coverage;
- time/random/env/crypto/compression coverage;
- deferred helper classes;
- report interpretation;
- manual versus automated behavior.

### Acceptance Criteria

- Docs do not imply full Nmap parity.
- Docs state what helper classes remain deferred.
- Agent-facing docs tell future agents to use wrappers for new side-effect helpers.

## Workstream 3: Compatibility Matrix Update

### Steps

1. Extend the existing library registry/compatibility matrix with helper-side enforcement status:
   - `wrapped`
   - `partially_wrapped`
   - `manual_only`
   - `deferred`
   - `pure/no_side_effect`
2. Include notes for risky libraries.
3. Link to tests/corpus where possible.

### Acceptance Criteria

- Users can see which libraries are protected by wrappers.
- Unknown/deferred status is explicit.

## Workstream 4: Architecture Guards

### Required Guards

By Milestone 3 closure, guards should fail on new direct high-risk operations in NSE helper modules unless allowlisted:

- process execution;
- filesystem mutation;
- filesystem read outside resolver/wrapper/test paths;
- network connect/send/receive outside wrappers;
- DNS resolver calls outside wrappers;
- unbounded compression/decompression helpers;
- environment reads outside wrapper/explicit manual path.

### Implementation Steps

1. Convert prior warning-only checks to failures for migrated helper classes.
2. Keep explicit allowlists for wrapper modules, tests, and docs tooling.
3. Include clear remediation text in guard failures.
4. Add a guard comment pointing to the capability inventory.

### Acceptance Criteria

- New bypasses fail fast.
- Existing intentional exceptions are documented.

## Workstream 5: Final Test Matrix

### Required Tests

- manual filesystem/process/network behavior where intended;
- AgentSafe filesystem/process/network denial;
- CiSafe external network/process denial;
- cancellation before helper call;
- post-call cancellation detection for blocking helper where feasible;
- resource counter updates;
- report events for helper denials;
- compatibility corpus still passes;
- Milestone 1 loader semantics still pass;
- Milestone 2 library/report truthfulness still passes.

### Acceptance Criteria

- Tests cover both allow and deny paths.
- No test requires public internet.

## Workstream 6: Verification Record

Record final verification in `architecture/nse_integration.md` or a dedicated verification note.

Run:

```bash
cargo check -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse,sandbox
cargo check -p eggsec --features nse
cargo test -p eggsec --features nse --test nse_tests
bash scripts/check-architecture-guards.sh
cargo fmt --all --check
cargo clippy --lib -p eggsec-nse --features nse
cargo clippy --lib -p eggsec --features nse
make test-nse
```

If `make test-nse` or a clippy command is unavailable, record the reason and closest equivalent.

## Workstream 7: Milestone 3 Closure Note

Add a closure note stating:

- what helper classes are wrapped;
- what helper classes are partially wrapped;
- what remains deferred;
- how manual and automated profiles differ;
- how reports expose helper decisions;
- what Milestone 4 should address.

Milestone 4 candidates:

- broader compatibility corpus;
- upstream NSE script subset conformance;
- advanced service/port context fidelity;
- richer structured evidence reports;
- UX polish for CLI/TUI report display.

## Final Acceptance Criteria

Phase 06 is complete when:

- Reports expose helper-side decisions and denials.
- Docs describe capability wrapper coverage and gaps.
- Guards prevent new side-effect bypasses.
- Final verification record exists.
- Milestone 3 closure note is present.
- Milestone 1 and 2 contracts remain intact.
