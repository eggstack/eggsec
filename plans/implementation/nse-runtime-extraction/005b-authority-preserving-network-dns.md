# NSE Runtime Extraction Milestone 005B — Authority-Preserving Network and DNS Providers

Status: blocked

Eggsec planning baseline: `b5d27348a829a4c1c2a49fbc267349c52e86a1f6`

Standalone runtime baseline: `eggstack/eggsec-nse@854f153f56d1abc929d9abd255f0606759342f81`

Source roadmap:

- `plans/subsystems/nse-runtime-extraction-roadmap.md#milestone-005--host-provider-inversion-and-portability-hardening`

Hard dependency:

- accepted closure of `plans/implementation/nse-runtime-extraction/005a-provider-broker-foundation.md`.

Long-term requirements:

- `plans/000-long-term-specification.md#2-primary-product-goals`
- `plans/000-long-term-specification.md#5-crate-ownership`
- `plans/001-terminology-and-domain-model.md#3-execution-terms`
- `plans/002-long-term-roadmap.md#phase-7--standing-maintenance-and-future-capability-open`

Applicable ADRs:

- `plans/adrs/ADR-0003-nse-host-provider-boundary.md`

Primary class: infrastructure

Affected repositories:

- implementation target: `eggstack/eggsec-nse`;
- planning/consumer verification: `eggstack/eggsec`.

## 1. Objective

Add runtime-neutral DNS and TCP/UDP provider contracts and migrate the shared/core NSE network paths so restricted profiles resolve, authorize, and connect to the same concrete endpoint.

This milestone exists to remove the current split between hostname policy checks and later native resolution/connection, and to make resource accounting/cancellation inseparable from provider-backed network execution.

## 2. Why this milestone is blocked

The design depends on the per-run service bundle and broker sequencing from 005A. Implementation must not create a second injection mechanism.

Once 005A closes, the remaining evidence is sufficient:

- `socket.rs`, `comm.rs`, and `nmap.rs` perform direct `std::net`/Tokio connects after capability checks;
- `dns.rs` owns a process-global Hickory resolver;
- `SandboxConfig::resolve_host` documents DNS rebinding/TOCTOU risk;
- `NseNetworkPolicy::AllowCidrs`/resolved-target checks are strongest for literal IPs and do not currently bind a hostname decision to the later concrete socket endpoint;
- several direct paths do not consistently call both `before_blocking_operation` and `after_blocking_operation`.

## 3. Current implementation evidence

Core paths currently expose native implementation types:

- `socket.rs` stores `TcpStream`/`UdpSocket` directly;
- wrappers return `std::net::TcpStream`, `SocketAddr`, and native errors;
- async socket helpers connect independently through Tokio;
- DNS uses Hickory in `dns.rs` and `ToSocketAddrs` elsewhere;
- connection allow-list checks may resolve separately from the actual connect.

This prevents a durable provider boundary and makes deterministic DNS/network tests harder.

## 4. Invariants that must not regress

- Runtime capability policy remains distinct from Eggsec authorization.
- Hostname-restricted network access must fail closed if no concrete approved address can be selected.
- A connection approved for one resolved address must not silently re-resolve and connect elsewhere.
- Network operation/byte counters and cancellation checks remain accurate.
- Current manual-permissive behavior remains available.
- Existing local TCP/UDP/DNS fixture behavior remains compatible.
- Provider public interfaces do not expose `std::net`, Tokio, Hickory, or Eggsec transport types.
- No new direct network bypass is introduced in migrated modules.
- Existing public convenience APIs are not broken.

## 5. Scope

### In scope

- Add narrow DNS and TCP/UDP provider traits to the 005A service bundle.
- Define runtime-owned endpoint/address/result DTOs and opaque connection handles.
- Add native provider implementations.
- Refactor broker sequencing to support resolve -> policy evaluation -> exact-endpoint connect.
- Migrate shared/core network paths:
  - `socket.rs`;
  - `comm.rs`;
  - relevant `nmap.rs` socket operations;
  - `dns.rs`;
  - wrapper functions that currently expose native network types.
- Replace the process-global DNS resolver where it conflicts with per-run injection.
- Add deterministic resolver/network test providers and zero-I/O denial tests.
- Inventory protocol-specific libraries that still perform direct network I/O and classify whether they are active under restricted profiles.
- Add guards preventing new direct `std::net`/Tokio/Hickory calls in migrated core modules.

### Explicitly out of scope

- HTTP provider/cutover.
- Rewriting every protocol-specific library in one pass.
- Eggsec `HttpTransport` adapter wiring.
- Filesystem/process provider migration.
- Public API signature breakage.
- New proxy semantics.
- Raw-packet/ICMP redesign.

## 6. Required production changes

### Core/domain

Provider contracts must use runtime-owned types. A workable semantic model should include:

- resolved endpoint identity: original hostname/label, concrete IP, port, protocol;
- DNS query/result DTOs sufficient for current A/AAAA/PTR/etc compatibility;
- opaque TCP/UDP handles or runtime-owned stream traits that can support send/receive/timeout/close without exposing native socket types.

The broker must own the sequence:

```text
resolve (when needed)
-> choose concrete candidate
-> evaluate NseCapabilityContext/network policy against candidate
-> cancellation/resource preflight
-> connect exact candidate via provider
-> I/O via opaque handle
-> post-operation counters/events
```

Do not authorize a hostname string and later ask the provider to resolve it again.

### Storage and migrations

None.

### Protocol and DTOs

Existing Lua socket/DNS return shapes remain compatible. Provider DTOs are internal/public Rust contracts, not NSE wire-format changes.

### Runtime and concurrency

Network providers are per-run/thread-safe. No process-global mutable resolver.

Async helpers may bridge through existing runtime machinery, but no detached tasks. Timeouts must remain bounded.

### Frontend or operator surface

No user-facing behavior change except closing unsafe/ambiguous hostname scope cases that should already have failed closed in restricted profiles.

### Security and authorization

Restricted-profile hostname/CIDR cases require explicit regression tests for:

- hostname resolves only outside allowed CIDR -> deny;
- hostname resolves to multiple addresses with mixed scope -> only an explicitly approved concrete address may be used; do not connect to an unapproved candidate;
- DNS answer changes between resolution and hypothetical second lookup -> provider records prove no second lookup occurs for the connect;
- DenyAll -> resolver/network provider not called where policy can reject without resolution.

Eggsec scope authority is not implemented here.

### Documentation and static guards

Update runtime architecture with provider coverage and an explicit remaining-bypass inventory.

## 7. Ordered work packages

### Work package A — Define DNS/network DTOs and provider traits

Create runtime-neutral, object-safe contracts and native implementations.

Acceptance: no native networking implementation types leak through the provider API.

### Work package B — Add authority-preserving broker flow

Implement concrete endpoint resolution/selection, policy evaluation, preflight, provider call, and accounting.

Acceptance: tests prove exact approved endpoint identity is the endpoint connected.

### Work package C — Migrate DNS library

Replace the static resolver path with injected provider usage while preserving current Lua result shapes and query support.

Acceptance: deterministic DNS fixtures work without external DNS.

### Work package D — Migrate socket/comm/nmap shared paths

Replace direct sync/async core network mechanics with provider-backed opaque handles.

Acceptance: local TCP/UDP fixture suites remain green and counters/cancellation are observed.

### Work package E — Inventory remaining protocol bypasses and add guards

Classify every remaining direct network site as migrated, restricted-profile unavailable, or explicitly deferred specialized behavior.

Acceptance: closure contains a machine-auditable inventory and new core bypasses fail CI.

## 8. Failure, cancellation, restart, and contention semantics

Provider resolution/connect errors map to current runtime/Lua failures without retrying through a different authority path unless existing semantics explicitly require multiple approved candidates.

Cancellation before connect prevents connect. Cancellation during blocking I/O must remain bounded by current timeouts.

No durable restart behavior.

Concurrent runs must use independent provider/resolver state unless an implementation is explicitly immutable/thread-safe.

## 9. Compatibility and migration

Native providers preserve default behavior for existing callers.

Current public wrapper functions may delegate to provider-backed internals or remain compatibility shims, but new provider contracts must not expose native handles.

Direct protocol libraries not migrated must be documented rather than falsely claimed provider-backed.

## 10. Required tests

### Focused unit tests

- endpoint DTOs;
- candidate filtering;
- opaque-handle semantics;
- broker counter/cancellation behavior.

### Integration tests

- local TCP connect/send/receive;
- UDP send/receive;
- deterministic A/AAAA/PTR;
- mixed allowed/denied DNS candidates.

### Restart and recovery tests

Not applicable.

### Contention and cancellation tests

- concurrent independent resolvers;
- cancellation before/during connect;
- connection timeout behavior.

### Security and negative tests

- DNS rebinding/no-second-resolution test;
- CIDR hostname deny;
- DenyAll provider-not-called;
- static direct-network guard.

### Migration and compatibility tests

- existing local protocol/corpus suites;
- report/capability-event parity.

## 11. Required verification commands

Standalone:

```bash
cargo fmt --all --check
./scripts/check-boundaries.sh
cargo check --features nse
cargo test --features nse
cargo check --features nse-ssh2
cargo check --features nse,sandbox
cargo clippy --all-targets --features nse
cargo +1.89.0 check --locked --features nse
cargo package
```

Run new network/DNS provider integration and authority-preservation tests explicitly.

Eggsec consumer smoke:

```bash
cargo check -p eggsec --features nse,cli
cargo test -p eggsec --features nse,cli --test nse_tests --test nse_integration_tests
make check
```

## 12. Documentation updates

- standalone provider architecture;
- compatibility/provider coverage table;
- direct-network bypass inventory;
- DNS/authority-preservation rationale.

## 13. Acceptance criteria

1. DNS and TCP/UDP providers are narrow and runtime-neutral.
2. Native defaults preserve existing caller behavior.
3. Core network paths use brokered provider execution.
4. Hostname policy is tied to the exact concrete endpoint connected.
5. Restricted CIDR/resolved-target profiles fail closed on unapproved resolution.
6. Counters/cancellation surround actual provider operations.
7. Static/global DNS resolver state no longer prevents per-run injection.
8. Core migrated modules contain no unguarded direct network bypass.
9. Remaining protocol-specific bypasses are explicitly inventoried.
10. Existing local protocol/corpus tests remain compatible.
11. Closure recommends whether 005C can consume the stable network/DNS contracts.

## 14. Stop conditions

Stop if:

- a correct provider boundary requires moving Eggsec authorization into the runtime;
- native socket types must remain in the public provider contract;
- hostname policy cannot be tied to concrete endpoints without breaking intended semantics;
- the work expands into all protocol libraries rather than the shared/core boundary;
- MSRV or report/API compatibility would be materially broken.

## 15. Closure evidence required

- provider API inventory;
- exact-endpoint authorization tests;
- DNS rebinding/no-second-resolution evidence;
- migrated core path list;
- counter/cancellation tests;
- remaining direct-network inventory;
- static guard evidence;
- corpus/local protocol results;
- MSRV/package result;
- GO/NO-GO for 005C.

## 16. Handoff notes

Do not treat DNS resolution itself as authorization. Resolution produces candidates; runtime policy selects concrete allowable endpoints, and the provider connects only the selected endpoint.

The main success criterion is authority preservation and operation-accounting correctness, not merely replacing `TcpStream` with a trait.
