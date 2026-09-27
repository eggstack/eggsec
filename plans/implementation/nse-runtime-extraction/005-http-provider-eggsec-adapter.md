# NSE Runtime Extraction Milestone 005C — HTTP Provider and Eggsec Scoped-Transport Adapter

Status: blocked

Eggsec planning baseline: `b5d27348a829a4c1c2a49fbc267349c52e86a1f6`

Standalone runtime baseline: `eggstack/eggsec-nse@854f153f56d1abc929d9abd255f0606759342f81`

Source roadmap:

- `plans/subsystems/nse-runtime-extraction-roadmap.md#milestone-005--host-provider-inversion-and-portability-hardening`

Hard dependency:

- accepted closure of `plans/implementation/nse-runtime-extraction/005-authority-preserving-network-dns.md`.

Long-term requirements:

- `plans/000-long-term-specification.md#2-primary-product-goals`
- `plans/000-long-term-specification.md#5-crate-ownership`
- `plans/001-terminology-and-domain-model.md#3-execution-terms`
- `plans/002-long-term-roadmap.md#phase-7--standing-maintenance-and-future-capability-open`

Applicable ADRs:

- `plans/adrs/ADR-0003-nse-host-provider-boundary.md`
- `plans/adrs/ADR-0001-scoped-transport-eggfetch-backend.md`

Primary class: infrastructure

Affected repositories:

- runtime provider contract and Lua migration: `eggstack/eggsec-nse`;
- Eggsec adapter and authority wiring: `eggstack/eggsec`.

## 1. Objective

Introduce a runtime-neutral HTTP provider contract, migrate the NSE HTTP-family libraries away from direct `reqwest` execution where parity permits, and turn Eggsec's existing `nse_http_capability` seam into a real provider adapter backed by `HttpTransport` + `NetworkAuthority`.

The runtime must remain scanner-independent and must not gain an `eggsec-transport` dependency.

## 2. Why this milestone is blocked

HTTP provider execution should reuse the stable per-run service bundle/broker from 005A and the authority-preserving endpoint/network semantics from 005B.

The current concrete consumer is already present in Eggsec:

- `crates/eggsec/src/nse_http_capability.rs` builds scoped transport requests and dispatches through `HttpTransport::execute(authority, request)`;
- its documentation explicitly says the Lua `http`/`httppipeline`/`brute`/`vulns`/`comm`/`upnp` libraries still use direct reqwest/native paths and that backend cutover is deferred to this provider-inversion phase.

## 3. Current implementation evidence

Standalone:

- `http.rs` owns static blocking/async reqwest clients and global invalid-cert/hostname flags;
- `httppipeline.rs`, `comm.rs`, `brute.rs`, `vulns.rs`, and `upnp.rs` contain direct reqwest calls;
- capability checks happen before requests, but transport authority/accounting remains separate from execution.

Eggsec:

- `nse_http_capability.rs` contains transport-neutral request construction, TLS-policy mapping, method coverage, same-host redirect policy, and a thin `dispatch_scoped_request`;
- no Lua HTTP library calls it today;
- the adapter remains engine-owned and currently has no authority-bearing integration from the NSE execution request path.

## 4. Invariants that must not regress

- Standalone runtime has zero Eggsec dependencies.
- Eggsec remains authorization/scope authority.
- Runtime HTTP provider traits use runtime-owned DTOs, not reqwest or Eggsec transport types.
- Insecure TLS remains profile-gated; scripts cannot escalate verification policy.
- Same-host redirect/scoped authority semantics must not be weakened.
- Existing NSE HTTP method/result compatibility remains intact for supported methods.
- Provider denial/cancellation/accounting must occur around the actual request.
- No direct frontend dependency on `eggsec-nse`.
- Existing native-default standalone behavior remains available outside Eggsec.
- No broad proxy/cookie/retry redesign in this slice.

## 5. Scope

### In scope

Standalone:

- define HTTP request/response/body/header/provider types;
- add a native reqwest-backed provider;
- thread HTTP provider through the M005 service bundle;
- migrate `http.rs` first;
- migrate `httppipeline.rs` and direct HTTP portions of `comm.rs`, `brute.rs`, `vulns.rs`, `upnp.rs` where behavior can be preserved;
- centralize profile/TLS/cancellation/accounting checks in the broker;
- remove or contain process-global TLS/client state where per-run provider configuration makes it obsolete;
- add provider test doubles and static bypass guards.

Eggsec:

- implement an Eggsec-owned runtime HTTP provider adapter using `HttpTransport` and the operation's `NetworkAuthority`;
- reuse/refactor `nse_http_capability.rs` rather than duplicating transport policy;
- thread the adapter only from an execution point that already holds the required Eggsec authority;
- test that runtime HTTP requests reach scoped transport with the same authorized target.

### Explicitly out of scope

- moving Eggsec authorization into runtime profiles;
- adding `eggsec-transport` to standalone;
- cross-host redirects;
- new proxy semantics;
- cookie jar/retry/cache redesign;
- providerizing all non-HTTP protocol libraries;
- changing public report DTOs;
- forcing every standalone caller to inject HTTP services;
- publishing a new release before cross-repo qualification.

## 6. Required production changes

### Core/domain

Define runtime-owned HTTP types sufficient for the existing Lua behavior: method, URL/host/port/path identity, headers, replayable body, timeout, TLS policy intent, status, response headers/body, and structured error.

The runtime provider interface must be implementation-neutral and object-safe. Exact sync/async mechanics may vary, but it must be usable from existing Lua execution without detached tasks.

Native provider uses reqwest internally; reqwest should no longer be referenced by migrated Lua libraries.

### Storage and migrations

None.

### Protocol and DTOs

Lua result tables and supported HTTP method semantics remain compatible.

Runtime HTTP DTOs are not Eggsec `ScopedHttpRequest`; Eggsec maps between contracts inside its adapter.

### Runtime and concurrency

Remove avoidable process-global mutable TLS/client settings from the Lua library path. Provider configuration should be per-run.

If an async provider is used from sync Lua closures, the bridge must use existing bounded runtime machinery and must not create nested/unbounded runtimes.

### Frontend or operator surface

No visible CLI/TUI/Python change.

### Security and authorization

Standalone broker:

```text
runtime capability check
-> cancellation/budget preflight
-> runtime HTTP provider
-> accounting/event result
```

Eggsec adapter:

```text
already-authorized Eggsec execution
-> runtime HTTP request DTO
-> map to ScopedHttpRequest
-> HttpTransport::execute(existing NetworkAuthority, request)
```

The adapter must never manufacture a broader authority from the URL alone.

If the current Eggsec NSE dispatch point cannot access the already-authorized `NetworkAuthority` without bypassing or duplicating enforcement, stop and create a small prerequisite integration plan rather than inventing authority.

### Documentation and static guards

Standalone guard: migrated HTTP-family libraries may not import/use reqwest directly.

Eggsec guard: adapter remains engine-owned; standalone crate remains free of Eggsec transport imports.

## 7. Ordered work packages

### Work package A — Define runtime-neutral HTTP provider contract

Acceptance: no reqwest/Eggsec types leak across the provider API.

### Work package B — Add native HTTP provider and broker sequencing

Acceptance: standalone native behavior remains equivalent and provider denial/cancellation/counters are tested around actual request execution.

### Work package C — Migrate core HTTP library

Move `http.rs` to provider-backed execution and eliminate its static client/TLS state where superseded.

Acceptance: all local HTTP method fixtures and automated-profile zero-hit denial tests remain green.

### Work package D — Migrate adjacent HTTP-family call sites

Handle `httppipeline`, direct HTTP paths in `comm`, `brute`, `vulns`, and `upnp`.

Acceptance: closure lists every migrated/deferred direct reqwest site and why.

### Work package E — Implement Eggsec scoped-transport provider adapter

Refactor `nse_http_capability` into the provider implementation/mapping seam while preserving TLS/redirect policy.

Acceptance: adapter unit tests show runtime DTO -> scoped request mapping and authority-bearing dispatch.

### Work package F — Thread authority-bearing provider into Eggsec NSE execution

Use only the canonical enforcement/dispatch context that owns the approved authority.

Acceptance: integration test proves an in-scope request reaches transport and an out-of-scope/cross-host request fails closed without native reqwest fallback.

### Work package G — Static guards and cross-repo qualification

Acceptance: direct reqwest reintroduction in migrated runtime libraries fails; standalone remains Eggsec-independent; Eggsec integration remains green.

## 8. Failure, cancellation, restart, and contention semantics

HTTP provider failures map to current NSE HTTP error/result semantics as closely as possible.

Cancellation must abort/stop awaiting the request path without detached work.

Timeouts remain bounded.

No durable restart semantics.

Concurrent runs may use different HTTP providers/TLS policies without global state leakage.

## 9. Compatibility and migration

Native provider remains the standalone default.

Eggsec injection is additive and internal to Eggsec execution. Existing NSE features/CLI/TUI/Python APIs remain unchanged.

Do not delete `nse_http_capability` history; refactor it into the actual adapter and update docs that currently call it dormant.

## 10. Required tests

### Focused unit tests

- HTTP DTO conversion;
- TLS policy mapping;
- provider error mapping;
- same-host redirect policy;
- denied provider-not-called.

### Integration tests

Standalone:
- GET/POST/PUT/DELETE/HEAD/OPTIONS/generic request local fixtures;
- async variants where supported;
- injected mock provider.

Eggsec:
- scoped transport adapter with fake `NetworkAuthority`;
- out-of-scope target denial;
- insecure TLS permitted only for allowed runtime profiles.

### Restart and recovery tests

Not applicable.

### Contention and cancellation tests

- concurrent runs with distinct provider state;
- request timeout/cancellation.

### Security and negative tests

- no direct reqwest in migrated runtime libraries;
- no Eggsec dependency in standalone;
- no authority fabrication from URL;
- no cross-host redirect authority widening.

### Migration and compatibility tests

- existing runtime corpus/local HTTP fixtures;
- Eggsec NSE/TUI/Python smoke.

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

Eggsec:

```bash
cargo check -p eggsec --features nse,cli
cargo test -p eggsec --features nse,cli --test nse_bridge_tests --test nse_integration_tests --test nse_real_scripts --test nse_tests
cargo test -p eggsec-tui --features nse
cargo test -p eggsec-python --features nse
make test-architecture-guards
make check-deps
make check
```

## 12. Documentation updates

Standalone:

- HTTP provider architecture;
- compatibility table/provider coverage;
- native-provider behavior.

Eggsec:

- `architecture/nse_integration.md`;
- current NSE transport documentation;
- `nse_http_capability` status comments;
- architecture guard docs.

## 13. Acceptance criteria

1. Runtime-neutral HTTP provider contract exists.
2. Native reqwest provider preserves standalone default behavior.
3. `http.rs` uses provider execution and no longer owns global static transport policy.
4. Adjacent HTTP-family direct reqwest sites are migrated or explicitly deferred with evidence.
5. Broker owns capability/cancellation/accounting around actual HTTP calls.
6. Eggsec's existing scoped transport seam implements the runtime provider without adding Eggsec dependencies to standalone.
7. Eggsec supplies the provider only with existing approved authority.
8. Out-of-scope/cross-host requests fail closed with no native fallback.
9. Local HTTP compatibility tests remain green.
10. Static guards prevent direct transport bypass regression.
11. Closure records whether an unreleased runtime revision requires a subsequent release/adoption step.

## 14. Stop conditions

Stop if:

- Eggsec authority cannot be threaded from the canonical enforcement path;
- implementing the adapter would require constructing authority from runtime URL data;
- runtime provider API must expose Eggsec/reqwest types;
- parity requires broad cookie/proxy/retry redesign;
- the work requires changing report/profile public contracts;
- dependency on 005B is not actually closed.

## 15. Closure evidence required

- runtime HTTP provider API;
- migrated/deferred direct reqwest inventory;
- native provider parity tests;
- denial/cancellation/accounting tests;
- Eggsec adapter mapping tests;
- authority-preservation integration tests;
- standalone boundary guard results;
- Eggsec architecture/dependency guard results;
- full relevant verification;
- release/adoption disposition.

## 16. Handoff notes

The existing Eggsec adapter is a concrete consumer and should shape the runtime-neutral contract, but it must not dictate Eggsec-specific types into the standalone API.

Treat any missing `NetworkAuthority` at the current dispatch call site as an integration-boundary problem, not permission to bypass the canonical Eggsec enforcement path.
