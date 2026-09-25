# ADR-0002: Selective Eggress Reuse Boundary

Status: accepted

Date: 2026-09-22

Decision owners: project maintainers

Related specification sections:

- `plans/000-long-term-specification.md#7`
- `plans/001-terminology-and-domain-model.md#4`

Affected subsystem roadmaps:

- grandfathered: `plans/eggress-1.0.8-adoption-roadmap-2026-09-22.md`
- grandfathered: `plans/eggress-1.0.10-adoption-and-metadata-closure-2026-09-24.md`

## Context

Eggress 1.0.8 introduced the listener-free `eggress-outbound` crate, creating a narrow reuse seam for SOCKS/HTTP CONNECT/chain dial execution that did not exist in the evaluated 1.0.6 line. The question was how much of Eggress to adopt: only the dial-execution edge, or the broader embed/runtime/server/routing/advanced-protocol surface.

## Decision drivers

- Remove Eggsec's duplicated SOCKS/HTTP CONNECT/chain handshake code.
- Keep Eggsec authorization, proxy pool/rotation, interception/MITM, and the Eggfetch-backed scoped HTTP transport independent from Eggress.
- Avoid absorbing a second proxy architecture (runtime, server, routing, H2/SSH/QUIC/UDP, pproxy compatibility).

## Considered options

### Option A — Adopt only the listener-free outbound edge (selected)

Migrate dial execution behind `eggress-outbound` at an exact version pin; keep health probing, pool/rotation policy, and interception in Eggsec; keep unsupported shapes fail-closed.

### Option B — Adopt the broader Eggress runtime/server surface

Rejected: duplicates daemon/runtime ownership, widens the dependency allowlist without a consumer, and couples proxy policy to an external runtime.

### Option C — Keep duplicated local handshake code

Rejected: leaves two implementations of security-sensitive handshake logic with no qualification owner.

## Decision

Option A. Only the already-approved listener-free dependency edge (`eggress-outbound` plus required URI helper) is adopted, at exact pins advanced by dedicated adoption plans. Check 106 proves the exact direct dependency allowlist. Proxy-hostname DNS behavior stays literal-only (no silent endpoint broadening). Ordered health checking preserves enabled-input result order with bounded concurrency.

## Consequences

### Positive

- One qualified dial-execution implementation with parity fixtures.
- Closed the upstream-gated `ProxiedConnection.local_addr` debt once Eggress preserved first-hop TCP socket metadata (fail-closed `require_local_addr()` on all production paths; no `0.0.0.0:0` sentinel).

### Negative

- Each Eggress line advance needs its own adoption + compatibility pass.
- Advanced protocol features remain unavailable until a new ADR reopens them.

### Neutral or deferred

- Out of scope unless reopened by a new ADR: `eggress-embed`, runtime/server, routing, H2, SSH, QUIC/H3, UDP, pproxy compatibility, insecure TLS, advanced protocol features. Eggress typed detailed failure classification is explicitly deferred behavior-change work, not adopted.

## Compatibility and migration

Exact-pin advances only, with guard updates, parity/matrix fixtures, and docs naming the measured first-hop address semantics (local endpoint of the physical TCP connection to the first proxy hop — not the final external egress address).

## Security and reliability implications

Literal-only proxy endpoints; exact allowlist enforcement; fail-closed unknown local addresses; SOCKS4 fails closed where qualified; no environment-derived routing.

## Verification

Parity fixture suites per adoption, exact-allowlist guard (Check 106), `make check`, feature profiles, dependency policy, MSRV, and architecture guards green.

## Supersession

None.
