# Phase B — Scope-aware outbound transport contract

Status: Ready for handoff

Date: 2026-09-11

Depends on: Phase A

## Purpose

Introduce one EggSec-owned outbound HTTP capability contract that is independent of Reqwest, Hyper, Rustls, or Eggfetch and makes authorization enforcement part of dispatch rather than an optional caller convention.

The contract must reuse the canonical authorization/scope model established by prior architecture work. Do not create a second target-policy language.

## Crate boundary

Create a dependency-light workspace crate, tentatively `eggsec-transport`. Its public API should model EggSec's requirements, not mirror any concrete client's entire API.

Preferred dependency envelope:

```text
eggsec-core / canonical scope types as needed
http (methods/status/header types) or narrow EggSec DTOs
bytes
url
serde only if transport DTO serialization is genuinely needed
thiserror
async-trait only if required by the selected trait shape
```

It must not directly depend on:

```text
reqwest
hyper
rustls
tokio-rustls
eggfetch-core
eggress-* heavy routing/runtime crates
```

If the canonical scope types currently live in the `eggsec` composition crate and cannot be consumed without a cycle, first extract only those stable policy/value types to an existing dependency-light crate such as `eggsec-core` or a narrowly scoped policy crate. Do not move engine execution logic into `eggsec-core`.

## Workstream 1 — Define transport-neutral request/response types

Define the minimum DTOs required by the Phase A parity matrix. Suggested shape:

```text
ScopedHttpRequest
  method
  url
  headers
  body/replayability representation
  timeout policy reference/value
  redirect policy
  proxy intent if EggSec exposes one
  TLS policy intent
  optional transport hints only where current EggSec behavior requires them

ScopedHttpResponse
  status
  headers
  body/stream abstraction
  final URL
  redirect history
  connection metadata where needed
```

Avoid reproducing the full Reqwest/Eggfetch builder surface. Domain code may use local builders/adapters, but cross-domain interfaces must exchange EggSec-owned or standard stable types.

Secret-bearing fields require redacted `Debug`/`Display` behavior. Add tests for redaction.

## Workstream 2 — Define authorization-aware execution

The central execution API must make a scope/policy authority mandatory. A conceptual shape is:

```rust
trait HttpTransport {
    async fn execute(
        &self,
        authority: &dyn NetworkAuthority,
        request: ScopedHttpRequest,
    ) -> Result<ScopedHttpResponse, TransportError>;
}
```

The concrete names may differ to align with existing canonical types. The important invariant is that a caller cannot obtain an unrestricted network dispatch simply by forgetting to call a scope helper.

Define explicit policy checkpoints for:

- initial URL canonicalization;
- initial hostname/direct-IP authorization;
- DNS result authorization;
- selected socket address authorization immediately before connect;
- each redirect target;
- each reconnect/re-resolution;
- proxy/upstream endpoint and ultimate destination as separate concepts;
- TLS SNI/Host override consistency where those features are supported.

The authority result should carry enough information to bind the authorization decision to the network destination actually used. Avoid TOCTOU patterns where the policy validates one DNS answer and the client independently resolves again before connect.

## Workstream 3 — Resolver/connect contract

Define a resolver abstraction only if required to bind policy to connection establishment. It should expose resolved addresses in a deterministic/testable form and support injected fixtures.

A safe model is:

```text
resolve host -> candidate addresses
             -> authority filters/approves candidates
             -> transport receives approved address set/binding
             -> connector uses only those approved addresses
```

Do not implement a policy check that calls Hickory and then hands only the hostname to an unrelated connector that performs its own DNS resolution. That would not close DNS rebinding/TOCTOU risk.

Document IPv4/IPv6 behavior, multiple-answer ordering, TTL/cache expectations, and what causes revalidation.

## Workstream 4 — Remove concrete HTTP types from shared EggSec APIs

Convert helpers that currently mention Reqwest types to transport-neutral behavior before the concrete backend migrates.

Known example:

- replace `auth_context::apply_auth_context_to_request(reqwest::RequestBuilder, ...)` with transport-neutral header/cookie application against EggSec/HTTP header types.

Audit all similar helpers found in Phase A. Prefer pure transformation functions over builder-specific mutation.

Compatibility wrappers may remain temporarily inside the concrete Reqwest implementation during migration, but they must not remain the canonical shared API.

## Workstream 5 — Test implementation

Provide a no-network fake/recording transport for unit tests. It should let tests assert:

- exact authorized destination;
- redirect authorization order;
- header/cookie redaction/removal;
- timeout/proxy/TLS policy propagation;
- cancellation behavior if part of the current contract.

Keep the fake in the transport crate behind `test-util` or a dedicated testkit crate if public consumers need it; do not make production builds carry test-only runtime features.

## Architecture guards

Add durable checks ensuring migrated domain/shared crates do not reintroduce `reqwest::RequestBuilder` or direct concrete client construction across the boundary. Do not ban all Reqwest references globally until Phase D has completed migrations.

## Required verification

```text
cargo check -p eggsec-transport --no-default-features
cargo test -p eggsec-transport
cargo tree -p eggsec-transport
make check
make test-architecture-guards
```

Also run the Phase A scope/redirect/DNS fixture suite through the fake/contract layer where applicable.

## Acceptance criteria

1. A dependency-light `eggsec-transport` crate exists.
2. Concrete HTTP/TLS client libraries do not appear in its dependency graph.
3. Authorization authority is mandatory for network execution.
4. The contract can bind authorized DNS results to the actual connection path rather than authorizing one resolution and connecting through another.
5. Redirect/retry/proxy checkpoints are explicit and tested.
6. Shared/domain APIs no longer require Reqwest builder types for header/auth-context handling.
7. A deterministic fake transport supports domain tests without Internet access.
8. Existing scope semantics remain canonical; no parallel scope language is introduced.

## Expected files touched

- root `Cargo.toml` workspace members/dependencies;
- new `crates/eggsec-transport/`;
- canonical scope type location only if a cycle requires a narrow extraction;
- `crates/eggsec/src/auth_context/` and any other shared helpers leaking concrete types;
- focused tests/fixtures;
- architecture guards and this completion record.
