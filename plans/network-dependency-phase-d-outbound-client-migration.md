# Phase D — Outbound client migration and dependency consolidation

Status: Ready for handoff

Date: 2026-09-11

Depends on: Phases A-C

## Purpose

Migrate EggSec's ordinary outbound HTTP consumers to the scope-aware transport seam and remove duplicated Reqwest/TLS ownership where it is no longer needed. Preserve specialized server/interception and protocol behavior.

Migrate incrementally by consumer. Each step must pass focused parity/security tests and dependency-graph checks before the old implementation is removed.

## Migration order

Use this order unless Phase A evidence shows a stricter dependency:

1. shared pure helpers and auth-context conversion;
2. `eggsec-agent` ordinary HTTP;
3. ordinary `eggsec` engine HTTP/scanner integrations;
4. NSE HTTP capability paths;
5. `eggsec-web-proxy` ordinary outbound helper/health/upstream client requests;
6. remaining Reqwest owners with an explicit disposition.

Do not migrate database drivers, raw packet paths, WebSocket protocol implementations, browser automation, SMTP, Kubernetes clients, or interception-server I/O merely to force all networking through one library.

## Workstream 1 — Agent migration

Replace direct Reqwest/Rustls ownership in `eggsec-agent` with `eggsec-transport` injection and the Eggfetch adapter at composition time.

Requirements:

- agent coordination/domain logic must not instantiate an unrestricted client;
- timeouts/cancellation/retry behavior remains explicit;
- authorization context is propagated from the owning operation rather than reconstructed in the agent crate;
- tests use the fake transport for deterministic behavior where possible;
- remove direct `reqwest` and client-only `rustls` dependencies when no longer used.

Record `cargo tree -p eggsec-agent` before and after.

## Workstream 2 — Engine HTTP migration

Use the Phase A inventory to migrate ordinary HTTP sites in `crates/eggsec` one subsystem at a time.

For each subsystem:

1. identify existing client construction/configuration;
2. map required behavior to the shared transport contract;
3. inject/reuse a transport instead of constructing per-call clients;
4. port tests to local fixtures/fake transport;
5. remove redundant Reqwest-specific helpers/imports;
6. rerun scope, redirect, TLS, timeout and feature-profile tests;
7. update the parity matrix disposition.

Avoid creating a single global mutable HTTP client. Prefer explicit application/runtime ownership with cheap clones/Arcs of an immutable configured transport.

## Workstream 3 — NSE network capability separation

NSE is a compatibility island and should remain one. Separate language/runtime compatibility from ordinary HTTP implementation.

For NSE HTTP-facing APIs:

- expose a narrow script capability backed by `eggsec-transport`;
- do not hand scripts a raw unrestricted Eggfetch/Reqwest client;
- bind requests to the operation's canonical authorization/scope context;
- preserve sandbox restrictions and cancellation/budget enforcement;
- migrate pure HTTP helpers away from Reqwest where Eggfetch parity is proven.

After migration, remove direct Reqwest/client-Rustls dependencies from `eggsec-nse` if no remaining NSE code needs them.

Do **not** remove OpenSSL/native-tls merely for dependency aesthetics if they are required by NSE protocol compatibility. Keep those dependencies feature-gated and document their precise owner. The goal is preventing them from contaminating non-NSE builds.

## Workstream 4 — Web proxy boundary

`eggsec-web-proxy` has two fundamentally different roles:

```text
A. inbound/listening/interception/MITM
B. ordinary outbound HTTP helper/upstream requests
```

Only B migrates to the shared client transport where semantics fit.

A remains owned by `eggsec-web-proxy`, including:

- listener/server I/O;
- MITM certificate generation and CA handling;
- Rustls server/termination state;
- HTTP/2 stream demux/interception;
- WebSocket interception;
- gRPC protobuf inspection where applicable;
- transparent-proxy integration.

After migrating B, remove Reqwest from `eggsec-web-proxy` if no remaining code requires it. Do not remove `rustls`, `tokio-rustls`, `rcgen`, `h2`, `http`, or protocol dependencies that remain necessary for interception.

Add an architecture test or module boundary documenting which side may depend on `eggsec-transport` versus server TLS/interception code.

## Workstream 5 — Remove unused broad client features

Once Reqwest call sites are gone from a crate, remove the dependency rather than leaving it available as a convenience escape hatch.

For Eggfetch adapter features, enable only behavior demonstrated necessary by Phase A. In particular verify whether EggSec truly needs:

- cookies globally versus request-scoped auth-context cookies;
- multipart;
- every compression codec;
- HTTP/2 in all artifacts;
- proxy/SOCKS in all artifacts;
- HTTP/3 (expected deferred until scoped QUIC is proven).

Use package-specific features so one domain does not unnecessarily widen every artifact's graph.

## Workstream 6 — Remaining concrete transport owners

At the end of migration, generate a list of remaining direct dependencies on:

```text
reqwest
rustls
tokio-rustls
webpki-roots
hickory-resolver
```

Every remaining owner must have a documented reason such as:

- interception/server TLS;
- protocol-specific direct TLS scanner semantics not representable as HTTP;
- canonical scope resolver implementation;
- compatibility-only feature.

Do not force raw TLS/scanner behavior through an HTTP adapter merely to make the list empty.

## Dependency acceptance tests per migration

For each migrated crate run:

```text
cargo tree -p <crate>
cargo tree -p <crate> -e features
cargo tree -i reqwest
cargo tree -i rustls
```

A migration is not considered dependency consolidation if Reqwest/Rustls remains pulled into the same final artifact through the new path without an intentional owner. Record when graph reduction is impossible because both stacks are needed for separate roles.

## Required verification

```text
cargo check --workspace --no-default-features
cargo test -p eggsec-agent
cargo test -p eggsec-nse --features nse
cargo test -p eggsec-web-proxy --features web-proxy
make check-feature-profiles
make check
make check-python
make test-architecture-guards
```

Run all transport parity/security fixtures from Phases A-C.

## Acceptance criteria

1. `eggsec-agent` no longer owns a direct ordinary HTTP stack.
2. Migrated engine HTTP subsystems execute through the mandatory scope-aware transport contract.
3. NSE HTTP capabilities use the scoped transport without broadening script authority.
4. `eggsec-web-proxy` clearly separates interception/server TLS from ordinary outbound client work.
5. Reqwest is removed from each migrated crate when no specialized use remains.
6. Remaining Rustls/Tokio-Rustls ownership is explicit and justified by TLS/server/raw-protocol responsibility.
7. No domain-facing shared API exposes Reqwest/Eggfetch request-builder types.
8. All existing safety, authorization, redirect, timeout, TLS, and cancellation tests remain green.
9. Artifact-specific dependency graphs demonstrate consolidation or explicitly document why a duplicate stack remains.

## Expected files touched

- `crates/eggsec-agent/` manifest/source/tests;
- `crates/eggsec/` HTTP-using modules and manifest;
- `crates/eggsec-nse/` HTTP capability layer and manifest;
- `crates/eggsec-web-proxy/` outbound helper code and manifest;
- process-host/composition wiring that constructs the Eggfetch adapter;
- parity/architecture tests and documentation;
- this plan completion record.
