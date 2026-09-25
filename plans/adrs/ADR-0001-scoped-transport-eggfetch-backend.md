# ADR-0001: Scoped Transport Contract with Pinned Eggfetch Backend

Status: accepted

Date: 2026-09-18

Decision owners: project maintainers

Related specification sections:

- `plans/000-long-term-specification.md#7`
- `plans/000-long-term-specification.md#5`
- `plans/001-terminology-and-domain-model.md#4`

Affected subsystem roadmaps:

- grandfathered: `plans/network-dependency-hardening-roadmap-2026-09-11.md`
- grandfathered: `plans/eggfetch-0.2.0-adoption-and-requalification-2026-09-22.md`

## Context

Eggsec had duplicate outbound HTTP/TLS ownership: engine modules constructed Reqwest clients directly, load-test code owned its own client pooling, and proxy/redirect authorization checkpoints were scattered. This duplicated connection-pool management, made per-hop authorization unenforceable in one place, and coupled assessment logic to a specific HTTP implementation.

## Decision drivers

- Single point of enforcement for scope-aware outbound policy (resolved routing, redirect authorization, proxy-peer checkpoints).
- Testability without network (recording fake).
- Backend replaceability without touching assessment logic.
- No second retry owner and no automatic retries in the transport.

## Considered options

### Option A — Shared Reqwest construction helpers

Centralize client construction in one engine module. Benefits: small diff. Costs: keeps implementation coupling in every consumer; authorization checkpoints remain advisory; pooling and redirect semantics stay implicit.

### Option B — Scope-aware transport contract with pinned backend (selected)

Define `eggsec-transport` as a neutral, dependency-light contract (DTOs, mandatory `NetworkAuthority`, TOCTOU-closed resolver binding, recording fake) and implement production HTTP in `eggsec-transport-eggfetch` over published `eggfetch-core` (logical-URL + singular resolved-address routing, manual authorized redirects, H1/H2 route reuse, pinned proxy peers/targets where enforceable, total deadline through body EOF). Engine `policy_bridge/` adapts facts; policy never depends on transport.

### Option C — Full HTTP framework extraction

Extract a general Eggsec HTTP framework crate. Rejected: single-consumer abstraction with no second consumer; recreates the rejected middle-layer problem.

## Decision

Option B. `eggsec-transport` owns the contract and stays implementation-neutral (`bytes`/`http`/`url`/`thiserror` only, zero workspace deps). `eggsec-transport-eggfetch` owns the Eggfetch adapter below the engine policy layer. Consumers migrate behind the contract; unsupported routes fail closed.

## Consequences

### Positive

- One enforceable authorization surface for outbound HTTP.
- Deterministic offline tests via the recording fake.
- Backend upgrades (0.1.7 → 0.2.0) requalify without consumer rewrites.

### Negative

- Adapter maintenance on Eggfetch upgrades; each adoption requires a requalification pass.
- Unsupported proxy shapes stay fail-closed until the backend qualifies them.

### Neutral or deferred

- Reqwest remains the application-level proxy-health owner where qualified; it MUST NOT become a second production transport.

## Compatibility and migration

Backend version moves are pre-1.0 exact-line adoptions with lockfile updates, architecture-guard advancement, and rerun of the direct/H2/CONNECT/SOCKS5/redirect/timeout/security qualification. No decompression, H3, retries, or environment-proxy behavior may be enabled incidentally during an adoption.

## Security and reliability implications

Singular socket-authorized pins remain mandatory; per-hop redirect authorization is manual; proxy-peer checkpoints (`authorize_proxy_resolved` / `authorize_proxy_socket`) close TOCTOU between resolution and connect; aggregate deadlines run through body EOF.

## Verification

Direct/H2/CONNECT/SOCKS5/redirect/timeout/security fixture suites, dependency/feature graph census proving no unintended backend features, `cargo audit`, `make check`, and feature-profile sweeps per adoption plan.

## Supersession

None.
