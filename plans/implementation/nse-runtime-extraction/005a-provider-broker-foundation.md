# NSE Runtime Extraction Milestone 005A — Provider Broker Foundation and Deterministic Host Services

Status: ready for handoff

Eggsec planning baseline: `b5d27348a829a4c1c2a49fbc267349c52e86a1f6`

Standalone runtime baseline: `eggstack/eggsec-nse@854f153f56d1abc929d9abd255f0606759342f81`

Source roadmap:

- `plans/subsystems/nse-runtime-extraction-roadmap.md#milestone-005--host-provider-inversion-and-portability-hardening`

Long-term requirements:

- `plans/000-long-term-specification.md#2-primary-product-goals`
- `plans/000-long-term-specification.md#5-crate-ownership`
- `plans/001-terminology-and-domain-model.md#3-execution-terms`
- `plans/002-long-term-roadmap.md#phase-7--standing-maintenance-and-future-capability-open`

Applicable ADRs:

- `plans/adrs/ADR-0003-nse-host-provider-boundary.md`
- `plans/adrs/ADR-0001-scoped-transport-eggfetch-backend.md` remains external to the standalone runtime and is not implemented in this slice.

Primary class: infrastructure

Affected repositories:

- implementation target: `eggstack/eggsec-nse`;
- planning/consumer verification: `eggstack/eggsec`.

## 1. Objective

Establish the additive, per-run host-provider composition boundary required by ADR-0003 and prove it with low-risk deterministic host domains before touching raw network, HTTP, filesystem-handle, or process execution paths.

This slice must introduce:

- narrow clock, randomness, and environment-read provider contracts;
- a native default implementation for each;
- a lightweight per-run provider/service bundle;
- injection through the canonical `NseRunRequest -> execute_nse_run` pipeline;
- capability-aware broker functions that own policy/preflight/provider/accounting/event sequencing;
- migration of representative runtime libraries away from direct clock/random/environment calls;
- deterministic test providers demonstrating that injected behavior is used.

The milestone is complete only when existing callers receive native-default behavior unchanged and injected providers can deterministically drive representative NSE scripts without bypassing `NseCapabilityContext`.

## 2. Why this milestone is ready

Milestones 001-004 are closed. `eggsec-nse 0.1.0` is standalone, published, and consumed by Eggsec from crates.io.

ADR-0003 resolves the cross-milestone architecture:

- narrow provider traits rather than one host trait;
- a per-run composition bundle;
- broker-owned capability/cancellation/accounting/event sequencing;
- native defaults;
- additive injection;
- no Eggsec dependency in the standalone runtime.

The current runtime already centralizes `NseCapabilityContext`, execution limits, cancellation, resource counters, and canonical request/report orchestration. It therefore has a stable place to thread provider state without redesigning authorization or execution ownership.

## 3. Current implementation evidence

At the standalone baseline:

- `NseRunRequest` carries target, script, profile, host/port context, limits override, and cancellation, but no host-service injection.
- `ExecutorCore` constructs its own `NseCapabilityContext` and registers libraries.
- `wrappers.rs` contains check-only helpers plus concrete executing helpers.
- `datetime.rs` reads `SystemTime` directly after time capability checks.
- `rand.rs` calls `rand::random` directly after randomness capability checks.
- `stdnse.rs` contains direct `Utc::now`, `SystemTime`, `thread_rng`, and sleep calls.
- `nmap.rs` contains direct time/random operations in several helpers.
- `os.rs` performs direct environment access.
- `ExecutorCore::add_default_scripts_path` reads process environment directly.
- CI-safe policy already denies randomness and environment access and warns on time reads, but direct host mechanics are not injectable per run.

Clock/random/environment are intentionally chosen first because they prove the provider contract without introducing opaque socket/file handles or Eggsec transport coupling.

## 4. Invariants that must not regress

- Existing `NseRunRequest::new` callers must compile and use native providers automatically.
- Eggsec authorization remains outside `eggsec-nse`.
- `NseCapabilityContext` remains the runtime policy owner.
- Provider traits must not contain policy decisions or Eggsec scope concepts.
- Provider calls from migrated production libraries occur only through the runtime broker/wrapper path.
- CiSafe continues to deny randomness/environment operations before a provider is called.
- AgentSafe/manual profile behavior remains semantically unchanged.
- Cancellation and report/capability-event behavior remain compatible.
- No `eggsec-*` dependency/import enters the standalone crate.
- `NseRunReport` serialization is unchanged.
- The provider bundle is a composition object, not a monolithic trait.
- Native providers may use `std`/chrono/rand internally; migrated NSE libraries should not.

## 5. Scope

### In scope

- Add a provider module hierarchy or equivalent runtime-owned location.
- Define narrow object-safe/thread-safe provider contracts for:
  - clock/time reads;
  - randomness;
  - environment variable reads and basic environment-derived path lookup needed by runtime setup.
- Add native provider implementations.
- Add a lightweight cloneable per-run service bundle, likely Arc-backed.
- Add provider injection to `NseRunRequest` and thread it through executor construction/library registration.
- Add broker functions that combine capability check, cancellation/resource preflight, provider call, accounting, and event/report semantics.
- Migrate `datetime.rs`, `rand.rs`, environment-read paths in `os.rs`, and representative direct time/random operations in `stdnse.rs`/`nmap.rs`.
- Migrate default script-path environment lookup if doing so is compatible with existing path behavior.
- Add deterministic fake/test providers.
- Add static guards preventing new direct time/random/environment calls in the migrated modules outside native-provider/test modules.
- Document provider semantics and default behavior.

### Explicitly out of scope

- TCP/UDP/DNS provider implementation.
- HTTP provider implementation.
- Filesystem handle abstraction.
- Process execution provider.
- Per-run virtual CWD/set-current-directory behavior.
- Eggsec `HttpTransport`/`NetworkAuthority` integration.
- Removing reqwest/Hickory/std dependencies.
- Migrating every direct time/random call in one pass if a protocol-specific path cannot safely adopt the broker without widening scope; remaining calls must be inventoried.
- Breaking `public_api` signatures.
- Publishing a new crate version.

## 6. Required production changes

### Core/domain

Introduce narrow provider traits and native implementations. Exact names may vary, but the semantic boundary should resemble:

```rust
trait NseClockProvider: Send + Sync {
    fn now_unix(&self) -> Result<...>;
}

trait NseRandomProvider: Send + Sync {
    fn fill_bytes(&self, out: &mut [u8]) -> Result<...>;
}

trait NseEnvironmentProvider: Send + Sync {
    fn var(&self, name: &str) -> Result<Option<String>, ...>;
}
```

The runtime may expose runtime-owned DTO/error types where needed. Do not leak implementation-specific types.

A lightweight `NseHostServices` or equivalent bundle carries these providers and later M005 domains. It must have a native default.

`NseRunRequest` gains additive injection, e.g. `with_host_services`. Existing constructors remain valid.

`ExecutorCore` and library registration must receive the per-run services rather than reading process-global host state directly for migrated domains.

### Storage and migrations

No persistent storage migration.

### Protocol and DTOs

No NSE report or external protocol change.

Provider error types should be runtime-owned and stable enough for future domains without forcing one giant error enum.

### Runtime and concurrency

Provider objects must be safe for concurrent runs according to their declared trait bounds.

No mutable process-global provider state.

Deterministic test providers should support per-run isolation. A test should be able to run two concurrent requests with different clocks/RNG streams without interference.

Sleep semantics may remain native in this slice unless a narrow clock/sleeper design is clean and cancellation-preserving. If sleep is migrated, cancellation checks must remain at least as responsive as the current chunked implementation.

### Frontend or operator surface

No CLI/TUI/Python behavioral change.

No frontend must be required to construct providers.

### Security and authorization

The broker must deny via `NseCapabilityContext` before the provider is invoked.

Tests must prove a denied CiSafe randomness/environment operation does not touch a counting/mock provider.

Time/random/environment provider injection does not grant authorization. Provider availability never overrides profile policy.

### Documentation and static guards

Standalone docs should explain:

- native defaults;
- injection boundary;
- distinction between runtime capability policy and provider mechanics;
- provider testability/determinism.

Add repository-local guards for direct host calls in migrated modules, with explicit allow-lists for native provider implementation and tests.

## 7. Ordered work packages

### Work package A — Add provider module and native service bundle

Intent:

Establish the additive per-run composition contract.

Required changes:

- define provider traits;
- define native implementations;
- define provider bundle/default;
- export only the intended public types;
- add construction/unit tests.

Acceptance evidence:

- default services reproduce native behavior;
- no monolithic host trait exists;
- provider bundle can be cloned/shared per run.

### Work package B — Thread services through canonical execution

Intent:

Make provider selection part of one authoritative execution pipeline.

Required changes:

- add optional/additive provider injection to `NseRunRequest`;
- derive native services when absent;
- thread services through executor/core/library registration;
- avoid secondary provider globals.

Acceptance evidence:

- existing request construction compiles unchanged;
- injected services are observable from a representative runtime path;
- two runs may carry different service bundles.

### Work package C — Establish brokered operation sequencing

Intent:

Ensure migrated operations cannot separate capability policy from host execution.

Required changes:

- implement broker/wrapper entry points for time, random, and environment;
- own capability decision, cancellation/preflight, provider invocation, and post-operation event/accounting behavior;
- return runtime-owned errors suitable for Lua mapping.

Acceptance evidence:

- denial prevents provider invocation;
- success records expected capability events;
- cancellation prevents provider invocation where applicable.

### Work package D — Migrate representative libraries

Intent:

Prove the provider design on real NSE behavior.

Required changes:

- migrate `datetime.rs`;
- migrate `rand.rs`;
- migrate `os.getenv`/equivalent environment-read paths;
- migrate directly equivalent time/random calls in `stdnse.rs` and `nmap.rs` where behavior is compatible;
- classify any remaining direct calls.

Acceptance evidence:

- migrated libraries contain no direct native time/random/environment mechanics outside permitted compatibility shims;
- clean-room/runtime tests preserve behavior.

### Work package E — Deterministic/concurrent provider tests and guards

Intent:

Prove why the provider seam exists.

Required changes:

- fixed-clock tests;
- deterministic byte-stream/random tests;
- environment-map tests;
- denied-provider-not-called tests;
- concurrent per-run isolation tests;
- static source guards.

Acceptance evidence:

- deterministic scripts/reports use injected values;
- separate concurrent runs do not leak provider state;
- guard fails on a new prohibited direct host call.

## 8. Failure, cancellation, restart, and contention semantics

Provider failure maps to the same class of Lua/runtime error the corresponding native operation produced, with enough context for diagnostics but no secret leakage.

Cancellation is checked before provider invocation. If sleep is migrated, long waits must remain cooperatively cancellable.

No durable state/restart behavior is introduced.

Provider test doubles and native providers must not depend on one process-global mutable clock/RNG/environment object.

## 9. Compatibility and migration

This is additive.

Existing users:

```rust
NseRunRequest::new(...)
```

continue receiving native behavior.

New users may inject services explicitly.

Do not remove current wrapper/check functions if they are part of the public surface; they may delegate to the new broker or remain compatibility shims until later M005 slices.

## 10. Required tests

### Focused unit tests

- provider defaults;
- service-bundle cloning/injection;
- broker allow/deny paths;
- deterministic clock/random/environment providers.

### Integration tests

- runtime script using datetime with fixed clock;
- runtime script using rand with deterministic provider;
- environment read allowed/denied according to profile;
- report capability-event parity.

### Restart and recovery tests

Not applicable.

### Contention and cancellation tests

- two concurrent runs with different provider values;
- cancellation before provider operation;
- sleep cancellation if migrated.

### Security and negative tests

- CiSafe randomness denied before provider call;
- AgentSafe/CiSafe environment denied before provider call;
- no Eggsec dependency/import;
- static guard catches direct host bypass in migrated modules.

### Migration and compatibility tests

- existing native-default corpus;
- current local protocol suite;
- report serialization snapshot/round-trip where currently covered.

## 11. Required verification commands

Standalone minimum:

```bash
cargo fmt --all --check
./scripts/check-boundaries.sh
cargo check --no-default-features
cargo check --features nse
cargo test --features nse
cargo check --features nse-ssh2
cargo check --features nse,sandbox
cargo clippy --all-targets --features nse
cargo +1.89.0 check --locked --features nse
cargo package
```

Run the new provider/broker focused tests explicitly.

Eggsec consumer verification:

```bash
cargo check -p eggsec --features nse,cli
cargo test -p eggsec --features nse,cli --test nse_tests --test nse_integration_tests
make check
```

If the standalone implementation is not yet released, Eggsec verification may use an exact temporary Git revision in a disposable branch/worktree or equivalent non-published integration method; do not alter Eggsec main's crates.io dependency until a release/adoption plan says so.

## 12. Documentation updates

Standalone:

- README provider/injection section;
- runtime architecture/provider document;
- compatibility notes where deterministic providers affect testing;
- boundary script documentation.

Eggsec:

- subsystem roadmap/registry/closure only unless consumer behavior changes.

## 13. Acceptance criteria

1. Narrow clock/random/environment provider traits exist with native defaults.
2. A lightweight per-run service bundle exists; no monolithic host trait is introduced.
3. `NseRunRequest` supports additive provider injection while current callers remain source-compatible.
4. Provider state reaches the canonical executor/library path without global mutable provider configuration.
5. Broker functions own policy/preflight/provider/post-operation sequencing for migrated domains.
6. Denied operations do not call providers.
7. Representative datetime/rand/environment library paths use providers rather than direct host calls.
8. Deterministic injected-provider tests pass.
9. Concurrent runs can use different providers without interference.
10. Static guards prevent new direct time/random/environment bypasses in migrated modules.
11. Current corpus/profile/report behavior remains compatible.
12. No Eggsec dependency is added to the standalone runtime.
13. Closure recommends GO/NO-GO for 005B and 005D based on the stability of the provider/broker contract.

## 14. Stop conditions

Stop and report if:

- provider injection requires changing Eggsec authorization semantics;
- the only workable design is a monolithic provider trait;
- the change requires a breaking `NseRunRequest` constructor;
- provider objects would need process-global mutable configuration;
- migrated behavior changes report serialization or profile semantics materially;
- implementation expands into sockets/HTTP/filesystem/process beyond incidental plumbing;
- the MSRV 1.89 contract cannot be preserved.

## 15. Closure evidence required

- standalone implementation commit(s);
- provider public API inventory;
- proof native default preserves old construction path;
- deterministic provider tests;
- denied-provider-not-called tests;
- concurrent isolation result;
- migrated-module direct-call guard result;
- corpus/local runtime verification;
- MSRV/package result;
- Eggsec consumer compatibility result;
- residual direct time/random/environment call inventory;
- recommendation for 005B and 005D.

## 16. Handoff notes

Do not design the future network/file handle interfaces prematurely in this slice. Leave the service bundle extensible enough to add them, but prove only the low-risk domains now.

The key closure signal is not merely that traits compile; it is that real NSE library execution uses injected values while policy/cancellation/report semantics remain centralized.
