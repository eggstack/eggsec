# Eggress 1.0.8 selective proxy-engine adoption roadmap

Status: Ready for handoff

Date: 2026-09-22

Eggsec baseline: `a81ae04219996d4db50b5d417b0e7e74a3666f8d`

Upstream Eggress baseline:

- release: `v1.0.8`, published 2026-09-22;
- release commit: `f0affac49c0fdaf6bcb51dfe1cb47f1f6548ffed`;
- MSRV: Rust 1.89;
- new direct dependency candidate: `eggress-outbound = 1.0.8` with
  `default-features = false`;
- native chain model: `eggress-uri = 1.0.8`;
- the base outbound profile supports listener-free HTTP/SOCKS TCP chain
  execution without `eggress-runtime`, `eggress-server`, or
  `eggress-embed`.

Parent records:

- [network-dependency-hardening-roadmap-2026-09-11.md](network-dependency-hardening-roadmap-2026-09-11.md)
- [network-dependency-phase-e-egress-and-capability-segregation.md](network-dependency-phase-e-egress-and-capability-segregation.md)
- `architecture/egress_reuse_decision.md`

The 2026-09-13 Phase E decision rejected Eggress 1.0.6 because no narrow
protocol/transport dependency existed that deleted equivalent Eggsec code.
Eggress 1.0.8 materially changes that premise by adding the dedicated
`eggress-outbound` crate and moving listener-free chain execution out of the
full service/embed stack. This roadmap is therefore a new release-specific
re-evaluation. It does not rewrite the historical Phase E record.

## Purpose

Replace duplicated low-level outbound proxy protocol machinery inside
`eggsec-web-proxy` with the narrowly scoped Eggress 1.0.8 outbound engine,
while preserving Eggsec ownership of authorization, proxy configuration,
selection/rotation, health policy, interception/MITM behavior, evidence, and
the canonical `eggsec-transport` contract.

The intended ownership boundary is:

```text
Eggsec policy / authorization
          |
          +-----------------------------+
          |                             |
          v                             v
 eggsec-transport                eggsec-web-proxy
          |                       pool / rotation
          v                             |
 eggfetch backend                       v
                              Eggress outbound adapter
                                        |
                                        v
                           HTTP/SOCKS proxy-hop execution
```

This is selective protocol reuse, not conversion of Eggsec into an Eggress
frontend.

## Confirmed baseline findings

At the Eggsec baseline:

1. `eggsec-web-proxy/src/socks.rs` independently implements SOCKS4,
   SOCKS4a/SOCKS5 framing, username/password auth, destination encoding,
   reply parsing/error mapping, timeout handling, and SOCKS5/Tor chain
   execution.
2. `eggsec-web-proxy/src/http_connect.rs` independently implements HTTP
   CONNECT request construction, Basic proxy authentication, bounded response
   parsing, status handling, and timeout behavior.
3. `ProxyManager::create_chained_connection()` maintains an additional
   SOCKS5/Tor-only chain orchestration path.
4. `ProxyPool`, `ProxyRotator`, `ProxyConfig`, and `ProxyEntry` own
   Eggsec-specific selection and compatibility semantics and should remain
   Eggsec-owned.
5. `health.rs` uses Reqwest specifically to perform an HTTP request through
   each tested proxy. It currently normalizes SOCKS4/SOCKS5 to a SOCKS5
   Reqwest proxy and HTTP/HTTPS to an HTTP Reqwest proxy; this behavior must be
   addressed deliberately rather than changed incidentally during the first
   migration.
6. The interception/MITM server still directly owns listener I/O, TLS
   termination, certificate generation, H2/WebSocket/gRPC inspection, flow
   capture, rules, budgets, and evidence. None of that belongs in this
   adoption.
7. `eggsec-transport` requires authorization to bind DNS results to actual
   connection targets. Eggress 1.0.8's public `OutboundConnector` resolves
   destinations internally and therefore is not a suitable replacement for
   the canonical scoped HTTP backend.
8. Architecture guard Check 106 currently rejects every Eggress manifest/source
   edge. It must be converted from a historical blanket rejection into a
   narrow allowlist that permits only the reviewed specialized
   `eggsec-web-proxy -> eggress-outbound/eggress-uri` edge while continuing
   to reject Eggress in `eggsec-transport`, the Eggfetch adapter, policy/core
   crates, and unrelated domains.
9. Eggress 1.0.8 shares the workspace MSRV and major protocol dependencies
   already present in web-proxy builds, but its published feature graph must
   be measured because the upstream workspace Tokio declaration is broader
   than Eggsec's intentionally narrow per-crate Tokio policy.
10. Eggress returns a generic async `BoxStream`; migration paths that carry a
    live proxied stream must not force it back into a concrete
    `tokio::net::TcpStream` merely to preserve the old implementation shape.

## Global invariants

Every phase must preserve:

1. public Rust, Python, CLI, TUI, MCP, and serialized configuration behavior;
2. Eggsec policy/enforcement ownership and all existing operation gates;
3. `eggsec-transport` and `NetworkAuthority` semantics unchanged;
4. no new `eggress` dependency in `eggsec-transport` or
   `eggsec-transport-eggfetch`;
5. no direct fallback after a configured proxy route fails;
6. current proxy pool, priority, weight, rotation, failure-count, and health
   state semantics unless Phase B explicitly proves and documents a compatible
   correction;
7. proxy credentials remain redacted in Debug/Display/errors/evidence;
8. explicit timeout behavior and cancellation-by-future-drop;
9. interception/MITM remains Eggsec-owned;
10. no `eggress-embed`, `eggress-runtime`, `eggress-server`,
    `eggress-routing`, SSH, QUIC/H3, Shadowsocks, Trojan, WebSocket outbound,
    or pproxy-compat feature adoption in this campaign;
11. native Eggress chain objects are constructed directly; do not round-trip
    Eggsec config through TOML or pproxy URI strings;
12. Rust 1.89 MSRV and ring-only TLS policy;
13. no Git/path/branch override for Eggress: consume the published 1.0.8
    crates;
14. no capability expansion hidden inside a maintenance migration.

## Ordered implementation sequence

### Phase A — Proxy protocol engine adoption

Plan:
[eggress-1.0.8-phase-a-proxy-engine-adoption-2026-09-22.md](eggress-1.0.8-phase-a-proxy-engine-adoption-2026-09-22.md)

Re-evaluate the old rejection against the published 1.0.8 graph, add the
narrow Eggress dependencies, implement an internal `ProxyEntry`/chain adapter,
establish local protocol parity fixtures, migrate SOCKS/HTTP CONNECT and chain
execution, then remove only the duplicated low-level implementation that is
proven replaced.

This phase also rewrites Check 106 as a positive narrow-boundary guard and
updates the current architecture decision record while retaining its 1.0.6
historical rationale.

Exit condition: Eggsec's selected HTTP/SOCKS proxy route is executed by
`eggress-outbound` with parity evidence; the old low-level handshake code is
gone or has a documented remaining owner; no other Eggress edge exists.

### Phase B — Health semantics, dependency cleanup, and closure

Plan:
[eggress-1.0.8-phase-b-health-qualification-and-closure-2026-09-22.md](eggress-1.0.8-phase-b-health-qualification-and-closure-2026-09-22.md)

Rebuild proxy health probes over the migrated connector only if complete
HTTP/HTTPS-through-proxy semantics can be preserved. Correct protocol
classification where locally provable, remove Reqwest from
`eggsec-web-proxy` only after parity is demonstrated, measure the final
dependency/artifact graph, and perform broad closure qualification.

If a proper HTTP health probe over Eggress streams would require a second
general HTTP client or substantial new HTTP/TLS machinery, keep Reqwest as the
explicit health-only owner and close the campaign without forcing dependency
removal.

Exit condition: health semantics are truthful and tested, dependency ownership
is explicit, the final graph is measured, and all release/architecture gates
are green.

## Ordering rationale

Phase A must land before health refactoring. Proxy protocol execution and
health probing are distinct contracts: a successful proxy tunnel is not the
same thing as a successful HTTP request through that proxy. Combining them
would make it difficult to distinguish a protocol migration regression from a
health-semantic change.

The Phase A parity suite should become the evidence base for Phase B. Phase B
is allowed to leave Reqwest in place if removing it would weaken health
meaning or recreate general HTTP machinery locally.

## Explicitly deferred work

The following are not required for this campaign:

- making Eggress a generic `eggsec-transport::HttpTransport` backend;
- remote-DNS authorization redesign;
- injecting Eggsec's authorized resolver/socket binding into Eggress;
- replacing Eggsec proxy pool/rotation with `eggress-routing`;
- using advanced Eggress protocols;
- moving interception/MITM server behavior to Eggress;
- changing public proxy configuration to Eggress-native DTOs;
- changing the meaning of `ProxyType::Https` without compatibility evidence.

A future generic Eggress resolver/connector injection API may make deeper
transport integration possible, but no Eggsec security invariant should be
weakened to achieve it now.

## Campaign verification

At minimum across the two phases:

```sh
cargo fmt --all --check
cargo check -p eggsec-web-proxy --no-default-features
cargo check -p eggsec-web-proxy --features web-proxy
cargo test -p eggsec-web-proxy -- --test-threads=1
cargo tree -p eggsec-web-proxy
cargo tree -p eggsec-web-proxy -e features
cargo tree -d
bash scripts/check-architecture-guards.sh
make check-feature-profiles
make check-features-individual
make check
make check-msrv
```

Run `make check-full`/hosted Deep Checks for closure where current repository
policy requires them. Record skips rather than silently omitting them.

## Global completion criteria

This roadmap is complete only when:

1. the 1.0.8 decision is recorded as a release-specific supersession of the
   1.0.6 transport-stack rejection;
2. only the specialized web-proxy crate consumes Eggress;
3. duplicated SOCKS/HTTP CONNECT/chain protocol ownership is removed where
   Eggress provides proven parity;
4. proxy failures never silently bypass the configured route;
5. credentials remain redacted;
6. timeout/cancellation behavior is tested;
7. proxy health semantics are not weakened to mere TCP-connect success;
8. Reqwest is either removed from `eggsec-web-proxy` with parity evidence or
   retained with an explicit health-only rationale;
9. `eggsec-transport` remains Eggress-free and scope-aware;
10. the final dependency/Tokio feature/artifact delta is recorded and accepted;
11. architecture guards encode the new narrow boundary;
12. focused, full feature, MSRV, and hosted validation are green.

## Handoff discipline

Each phase must append a completion record with baseline/final SHAs, exact
commands, dependency graph before/after, protocol parity results, architecture
guard changes, and residual debt.

Do not edit historical plan completion records to pretend the 1.0.6 rejection
never existed. Current architecture documentation may be updated to state that
1.0.8 changed the dependency boundary and therefore justified a new decision.
