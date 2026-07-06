# NSE Milestone 5 Phase 04: Deferred Library Migration

## Purpose

Start reducing the deferred NSE library surface identified at Milestone 4 closure. The goal is not broad parity; it is to migrate the highest-value deferred or partially wrapped library classes behind capability wrappers with runtime corpus coverage.

## Current Deferred/Partial Surface

Milestone 4 identified deferred or partial areas including:

- `http` partially wrapped;
- `ssl` / TLS handshake support deferred;
- `ssh` deferred;
- `smb` / `smb2` deferred;
- database libraries: `mysql`, `postgres`, `redis`, `mongodb`;
- directory/network management: `ldap`, `snmp`;
- auth helpers: `creds`, `unpwdb`, `brute`;
- target registry manipulation.

## Non-Goals

Do not migrate every deferred library in one pass.

Do not implement brute-force behavior for automated profiles.

Do not allow deferred protocols in AgentSafe/CiSafe without wrappers, bounds, and tests.

Do not require public network services.

## Workstream 1: Prioritization

Select one or two targets for this phase based on:

1. common NSE script usage;
2. ability to test with local fixtures;
3. safety and profile policy clarity;
4. small implementation surface;
5. report/evidence value.

Recommended first targets:

- `http`: promote from PartiallyWrapped to more fully wrapped with local fixture support.
- `ssl`/certificate parsing: support deterministic certificate evidence without requiring full TLS parity.
- `unpwdb`/`creds`: read-only local fixture support under scoped roots; deny under AgentSafe/CiSafe unless fixture-scoped.

Avoid starting with SMB/LDAP/SNMP/database protocols unless a small local stub already exists.

## Workstream 2: Migration Contract Per Library

For each selected library, define:

```markdown
| Library | Current Status | Target Status | Capability Wrappers | AgentSafe Policy | CiSafe Policy | Local Fixtures | Report Evidence |
```

Required statuses:

- `deferred`
- `partial`
- `wrapped-readonly`
- `wrapped-network-local`
- `wrapped-denial-only`
- `pure`

## Workstream 3: Capability Wrapper Integration

For migrated libraries:

- route filesystem reads/writes through filesystem wrappers;
- route network connect/send/receive through network wrappers;
- route DNS through DNS wrapper;
- route process execution through process wrapper or deny;
- record capability events and counters;
- ensure cancellation checks before/after blocking calls.

### Acceptance Criteria

- No new direct side-effect operations appear outside wrapper modules.
- AgentSafe/CiSafe behavior is explicit and tested.

## Workstream 4: Runtime Corpus Fixtures

Add runtime fixtures for each migrated library:

- success under manual/compatibility profile using local fixtures;
- denial under AgentSafe/CiSafe where appropriate;
- capability event assertions;
- library report assertions;
- evidence assertions if the library produces evidence.

### Acceptance Criteria

- Compatibility matrix status changes only after runtime fixtures pass.
- Tests assert observed fields strictly.

## Workstream 5: Compatibility Matrix Update

Update `docs/NSE_COMPATIBILITY.md`:

- move selected library from Deferred/Partial to the new target status;
- cite fixture IDs;
- state profile compatibility;
- list remaining gaps.

Do not overclaim full upstream behavior.

## Workstream 6: Architecture Guards

For migrated classes, convert guard checks from informational to failing where feasible.

Examples:

- direct HTTP client calls outside wrapper fail;
- direct credential/wordlist file reads outside wrapper fail;
- direct TLS network handshake outside wrapper fails if migrated.

## Verification

Run:

```bash
cargo test -p eggsec-nse --features nse --test runtime_corpus_tests -- --test-threads=1
cargo test -p eggsec-nse --features nse --test runtime_smoke_tests
cargo test -p eggsec-nse --features nse
bash scripts/check-architecture-guards.sh
cargo clippy --lib -p eggsec-nse --features nse
```

## Final Acceptance Criteria

Phase 04 is complete when:

- At least one deferred or partially wrapped library class is migrated to a stronger documented status.
- Runtime corpus fixtures verify the new behavior.
- AgentSafe/CiSafe behavior is enforced through capability wrappers.
- Compatibility matrix and guards match the implementation state.