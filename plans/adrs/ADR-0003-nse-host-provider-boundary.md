# ADR-0003: NSE host-provider and capability-broker boundary

Status: accepted

Date: 2026-09-27

Decision owners: project maintainers

Related specification sections:

- `plans/000-long-term-specification.md#2-primary-product-goals`
- `plans/000-long-term-specification.md#5-crate-ownership`
- `plans/001-terminology-and-domain-model.md#3-execution-terms`
- `plans/002-long-term-roadmap.md#phase-7--standing-maintenance-and-future-capability-open`

Affected subsystem roadmaps:

- `plans/subsystems/nse-runtime-extraction-roadmap.md`

## Context

`eggsec-nse 0.1.0` is now a standalone runtime and Eggsec consumes the registry release. Milestone 005 can therefore address the remaining host-side effects without mixing that work with extraction or publication.

The standalone runtime already has an `NseCapabilityContext` and a set of helper wrappers. Those pieces centralize policy decisions, cancellation checks, resource counters, and report events, but many production NSE libraries still perform the actual host operation directly after a capability check:

- TCP/UDP through `std::net` or `tokio::net`;
- HTTP through `reqwest`;
- DNS through a process-global Hickory resolver or `ToSocketAddrs`;
- filesystem operations through `std::fs`;
- process execution through `std::process::Command`;
- clock/random/environment access through process-global/native calls.

This produces several concrete problems:

1. a capability decision and the operation it is meant to govern are not always one atomic brokered path, so cancellation/resource accounting can be skipped even when a policy check occurred;
2. hostname policy can be evaluated separately from later resolution/connection, leaving DNS rebinding/TOCTOU and authority-preservation concerns;
3. Eggsec already has a scoped `HttpTransport` + `NetworkAuthority` adapter seam, but the standalone runtime has no neutral host-provider contract through which Eggsec can inject it;
4. global resolver/client/process-CWD state reduces per-run isolation and deterministic testing;
5. Unix-specific host operations limit portability and make Windows qualification incomplete;
6. deterministic clock/random test doubles are not injectable per run.

The decision must preserve the standalone runtime's independence. Eggsec authorization remains outside the runtime. Runtime capability policy remains defense-in-depth and NSE execution policy, not a replacement for `EnforcementContext` or Eggsec scope authority.

## Decision drivers

- Preserve the runtime's zero-`eggsec-*` dependency boundary.
- Keep policy decisions, cancellation, counters, and audit/report events centralized.
- Allow per-run host behavior injection without changing default behavior.
- Preserve authority from name resolution through connection rather than re-resolving after approval.
- Support deterministic tests and replay-oriented fixtures.
- Improve Windows/other-host portability without forcing every platform detail into NSE library code.
- Avoid a monolithic "host" trait that becomes a second runtime kernel.
- Preserve public NSE compatibility and the existing `eggsec::nse` facade.
- Permit Eggsec to adapt its existing scoped transport contract into the runtime without moving Eggsec authorization into `eggsec-nse`.

## Considered options

### Option A — Keep check-only capability calls and direct host operations

Continue adding `check_*` calls to libraries while leaving each library responsible for `std`/`tokio`/`reqwest`/Hickory execution.

Benefits:

- smallest immediate diff;
- no new public runtime interfaces.

Costs and failure modes:

- accounting/cancellation remain inconsistently attached to real operations;
- DNS resolution and later connections can diverge;
- deterministic host behavior remains difficult to test;
- Eggsec cannot inject scoped transport cleanly;
- platform-specific code remains distributed through NSE compatibility libraries.

Rejected.

### Option B — One monolithic `HostProvider` trait

Create one trait containing network, HTTP, DNS, filesystem, process, clock, random, environment, and platform operations.

Benefits:

- one injection point;
- simple wiring at first.

Costs and failure modes:

- very large interface with unrelated responsibilities;
- every test double must implement unrelated methods;
- changes in one host domain churn all consumers;
- encourages authorization/policy logic to leak into the provider;
- difficult to version as a reusable public crate contract.

Rejected.

### Option C — Narrow provider traits behind one per-run service bundle, brokered by capability-aware wrappers

Create small provider interfaces by host domain, a native default implementation for each, and a lightweight per-run bundle that carries them. Production NSE libraries perform side effects through capability-aware broker functions. The broker owns the sequence:

```text
capability decision
-> cancellation/resource preflight
-> provider operation
-> resource accounting
-> event/report result
```

Eggsec may supply adapters for selected provider domains, such as HTTP/network authority, while the standalone runtime retains native defaults and no Eggsec dependencies.

Selected.

## Decision

Milestone 005 SHALL implement Option C.

The durable boundary is:

1. **Narrow provider traits.** Provider contracts are split by host domain. At minimum the roadmap may introduce network, DNS, HTTP, filesystem, process, clock, random, and environment providers where evidence justifies them. A provider trait MUST NOT become an umbrella authorization interface.

2. **Per-run service bundle.** A lightweight `NseHostServices` (exact name may vary) owns/arcs the provider implementations for one run. It is a composition object, not a monolithic trait. Native services remain the default so existing callers continue to work without explicit injection.

3. **Capability-aware broker.** NSE compatibility libraries SHOULD NOT call provider objects directly. Runtime-owned broker/wrapper functions combine `NseCapabilityContext` with the provider operation so policy, cancellation, limits/counters, and capability events cannot drift apart.

4. **Provider mechanics do not authorize Eggsec operations.** The standalone runtime's `NseCapabilityContext` remains runtime policy. Eggsec's `EnforcementContext`, approved execution bundle, and `NetworkAuthority` remain authoritative for Eggsec operations.

5. **Authority-preserving network resolution.** A hostname-based connection that is constrained by CIDR/resolved-target policy must resolve to concrete endpoint data, evaluate policy/authority against that concrete data, and connect to the same approved endpoint. The implementation must not approve one resolution and silently re-resolve for the actual connect.

6. **No concrete host implementation types in public provider boundaries.** Provider contracts must use runtime-owned DTOs/opaque handles rather than exposing `std::net::TcpStream`, `std::fs::Metadata`, `std::fs::DirEntry`, `std::process::Output`, `reqwest`, Hickory resolver types, or Eggsec transport types.

7. **Additive injection.** Existing `NseRunRequest::new` semantics remain native-default. Provider injection is additive, for example through `with_host_services(...)` or an equivalent builder. Existing 0.1-series users must not be forced to construct providers.

8. **Eggsec adapters are engine-owned.** Eggsec-specific implementations that wrap `HttpTransport`, `NetworkAuthority`, or other Eggsec scope/transport contracts remain in `eggstack/eggsec`. The standalone runtime defines only neutral provider contracts.

9. **Public convenience APIs are compatibility shims unless separately migrated.** Existing `public_api` functions may keep native behavior during M005 unless a provider-aware additive variant is introduced. M005 must not silently break those public signatures.

10. **Platform specialization belongs in native providers.** Unix/Windows differences such as permissions, symlink behavior, privilege checks, interface enumeration, and shell/process mechanics should be localized in native provider implementations or explicit cfg-specific modules rather than spread through Lua library compatibility code.

## Consequences

### Positive

- Capability policy, cancellation, accounting, and actual host execution can become one auditable path.
- Deterministic clock/random/DNS/network/filesystem test doubles become possible per run.
- Eggsec can preserve scoped transport authority through a neutral runtime seam.
- DNS/connect TOCTOU can be materially reduced by carrying resolved endpoint identity.
- Platform-specific host mechanics become easier to isolate and qualify.
- Provider interfaces can evolve independently by domain.

### Negative

- Raw socket and file APIs need opaque/runtime-owned handles, which is more work than wrapping scalar functions.
- Some library implementations will need non-trivial migration to stop storing `std` handles directly.
- Sync Lua closures and async Eggsec transports require a carefully bounded bridge; this ADR does not prescribe one universal async runtime model.
- The runtime will expose additional public types that require semver discipline.

### Neutral or deferred

- M005 does not require every protocol-specific library to be provider-backed immediately. The plans must classify remaining direct side effects and prevent untracked new bypasses.
- M005 does not require removing `reqwest`/Hickory from the standalone dependency graph if native providers still use them internally.
- M005 does not make provider injection mandatory for existing users.
- M005 does not move authorization into the runtime.
- Provider-aware migration of the broad Rust `public_api` is optional/additive and must not cause a breaking cleanup.
- A later release may publish the provider interfaces under a new semver version after qualification.

## Compatibility and migration

The migration is additive and staged.

Existing callers continue to construct `NseRunRequest` and receive native host behavior by default. New injection APIs must preserve the current canonical execution/report path.

Production libraries are migrated domain by domain. During the transition, direct host operations that remain intentionally specialized must be inventoried and guarded so the project does not claim full provider coverage prematurely.

Eggsec adapters remain outside the runtime and may implement only the provider domains Eggsec needs. An Eggsec adapter must not be required to implement unrelated filesystem/process/time methods.

No persistent data, wire protocol, or `NseRunReport` schema migration is required by this ADR.

## Security and reliability implications

Provider invocation must occur only after the runtime capability decision and cancellation/resource preflight for the corresponding operation. Post-operation accounting and report events must reflect actual bytes/operations where available.

Network/DNS providers must preserve concrete endpoint identity through policy/authority evaluation and connect. Hostname allow decisions that cannot be tied to a concrete approved address must fail closed in restricted profiles.

Eggsec adapters must carry the operation's existing scope/transport authority into the concrete transport call. They must never broaden a runtime target simply because the provider interface permits an operation mechanically.

Provider implementations must be per-run or explicitly thread-safe. Global mutable clients/resolvers/CWD state should be removed where it conflicts with per-run behavior.

Cancellation must remain cooperative and bounded. Provider operations that can block require timeouts consistent with existing limits and repository policy; no detached background tasks are introduced.

## Verification

Conforming M005 implementation evidence must include:

- native-default behavior parity with the published `0.1.0` runtime for representative corpus/local-protocol tests;
- deterministic injected provider tests proving clock/random and later host domains use the injected implementation;
- static guards preventing migrated libraries from bypassing the broker;
- resource-counter and cancellation tests around actual provider calls;
- hostname/CIDR tests proving resolve-authorize-connect identity is preserved;
- Eggsec adapter tests proving scoped transport receives the same authorized target/authority;
- no new `eggsec-*` dependency/import in the standalone runtime;
- Windows compilation/qualification for the portability slice;
- closure evidence that explicitly lists any intentionally remaining direct host operations.

## Supersession

None.
