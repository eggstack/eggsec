# NSE Expansion Phase 05: Selective Deferred Library Migration

## Purpose

Migrate deferred NSE library families only when deterministic local fixtures and capability-wrapper tests are ready.

This phase should be selective. The goal is not to chase broad protocol parity; it is to reduce the most valuable deferred surfaces without weakening safety semantics.

## Non-Goals

Do not migrate all deferred libraries.

Do not add brute-force behavior for automated profiles.

Do not add public network integration tests.

Do not implement SSH/SMB/database parity without local fixtures.

Do not mark a library wrapped until side effects are routed through capability context and tested.

## Candidate Ranking

Recommended priority:

1. `creds` / read-only auth helpers:
   - bounded filesystem reads;
   - useful for many scripts;
   - local fixture friendly;
   - low protocol complexity.
2. Deeper `ssl`/certificate behavior:
   - builds on Phase 03;
   - local deterministic fixtures possible.
3. `target` registry shape:
   - mostly internal state;
   - useful but must avoid hidden side effects.
4. `ssh` shape/stub coverage:
   - only after local server/stub exists;
   - avoid real auth complexity.
5. Databases/SMB/LDAP/SNMP:
   - defer until local service harness exists.

## Workstream 1: Pick One Library Family

For the first implementation, pick exactly one family.

For each candidate, document:

- current registry status;
- side effects;
- profile policy;
- available local fixture strategy;
- expected compatibility level;
- implementation risk.

### Acceptance Criteria

- One family is selected.
- Deferral rationale exists for non-selected families.

## Workstream 2: Define Capability Contract

For the selected library, define:

- filesystem reads/writes;
- network operations;
- process/env/time/random operations;
- allowed behavior in ManualPermissive;
- denied/warned behavior in AgentSafe;
- denied behavior in CiSafe;
- emitted capability events.

### Acceptance Criteria

- No side-effecting function lacks a profile policy.

## Workstream 3: Implement Wrapper Routing

Route side effects through existing wrappers or add small wrappers if necessary.

Requirements:

- no direct `std::fs` side effects except inside wrappers;
- no direct network operations outside capability-gated helpers;
- denied operations produce capability events;
- fallback behavior is explicit.

### Acceptance Criteria

- Architecture guards catch obvious bypasses.
- Manual behavior remains useful.

## Workstream 4: Runtime Fixtures

Add fixtures that cover:

- ManualPermissive success path;
- AgentSafe denial or warning path;
- CiSafe denial path;
- missing/partial unsupported behavior if applicable.

### Acceptance Criteria

- Runtime tests assert libraries, rules, capability events, evidence, and compatibility status.
- Fixtures are local-only.

## Workstream 5: Registry and Docs

Update:

- `resolver/registry.rs`;
- `docs/NSE_COMPATIBILITY.md`;
- `architecture/nse_integration.md`;
- agent guidance docs.

Do not mark full fidelity unless the runtime coverage genuinely supports it.

### Acceptance Criteria

- Registry status matches implementation and tests.
- Compatibility docs list remaining gaps.

## Workstream 6: Guards

Add library-specific guards where feasible.

Examples:

- direct file reads outside wrapper fail for `creds`;
- direct network sends outside wrapper fail for protocol libraries;
- fixture scripts cannot use public targets;
- required capability events are asserted.

## Verification

Run:

```bash
cargo check -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse --test runtime_corpus_tests -- --test-threads=1
cargo test -p eggsec-nse --features nse --test local_protocol_tests
cargo test -p eggsec-nse --features nse
bash scripts/check-architecture-guards.sh
cargo fmt --all --check
cargo clippy --lib -p eggsec-nse --features nse
```

## Final Acceptance Criteria

Phase 05 is complete when:

- exactly one deferred family is migrated or explicitly deferred with evidence;
- side effects route through capability wrappers;
- ManualPermissive and automated-profile behavior are tested;
- registry/docs are truthful;
- guards protect the new wrapper path;
- remaining deferred families have a clear next-boundary note.
