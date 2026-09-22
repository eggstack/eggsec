# Eggfetch Transport Adapter (Phase C + corrective passes + 0.1.7 adoption + 2026-09-19 qualification + 0.2.0 adoption)

Status: adapter implemented 2026-09-12 (`eggsec-transport-eggfetch`).
Corrective pass (2026-09-17): production load-test backend over published
`eggfetch-core 0.1.5` (H1/H2 via ALPN, pinned proxy peers/targets where enforceable).
Follow-up (2026-09-17): proxied backend route narrowed to one
socket-authorized address per leg (singular `proxy_peer` / `ultimate_peer`;
no silent multi-address fallback; truthful `ConnectionInfo`).
Adoption (2026-09-19): published `eggfetch-core 0.1.7` (logical-URL +
singular `resolved_addresses` direct, H1/H2 route reuse, `Timeout.total`
through body EOF; direct IP-literal shim removed).
Qualification (2026-09-19 corrective): Eggsec-local H2 ALPN/multiplex/reuse
proof (`tests/h2_mux.rs`, 5 tests) + supported local-resolution SOCKS5
success/pinning proof (`tests/socks5_local.rs`, 4 tests); deep gates +
concurrency 1/10/50/100 evidence recorded (see Completion addendum).
Adoption (2026-09-22): published `eggfetch-core 0.2.0` (explicit pre-1.0
version-line adoption; upstream states no intentional public API/feature/MSRV
break from 0.1.7; issue-24 streaming-decompression correction is outside
Eggsec's decompression-off production path). The adapter compiles and
requalifies unchanged: logical-URL + singular resolved direct, H1/H2 route
reuse, `Timeout.total` through body EOF, manual per-hop redirect
authorization, singular per-leg proxy pins, fail-closed unsupported proxy
shapes, no H3/retries/env-proxies/decompression.

## Role & Responsibilities

[`EggfetchTransport`](../../../crates/eggsec-transport-eggfetch/src/adapter.rs)
implements [`HttpTransport`](transport.md) over the **published**
`eggfetch-core 0.2.0` client using only stable public APIs.

**Non-responsibilities:**

- The adapter does not migrate any consumer (Phase D). Reqwest call sites
  are untouched; the only new wiring is a `#[cfg(test)]`-adjacent
  dev-dependency for engine-level parity tests.
- The adapter does not invent policy. Every checkpoint delegates to the
  caller-supplied `NetworkAuthority` (canonically
  `config::ScopeAuthority`); the adapter only sequences checks and binds
  their results to the connection path.
- The adapter does not retry, decompress, jar cookies, or build multipart
  bodies. Those behaviors stay above the transport or are explicitly off.

## Location & Feature Gating

| Item | Path | Notes |
|------|------|-------|
| Adapter crate | `crates/eggsec-transport-eggfetch/` | 19th workspace crate in member order; lib only, no features |
| Parity/adversarial suite | `crates/eggsec-transport-eggfetch/tests/parity.rs` + `tests/common/` | Local loopback fixtures only (plain + TLS + CONNECT proxy + slow-body/trickle/keep-alive); 52 tests |
| H2 qualification suite | `crates/eggsec-transport-eggfetch/tests/h2_mux.rs` | Loopback H2-over-TLS via `EggfetchTransport` (ALPN h2, concurrent multiplex + sequential reuse + logical-origin and selected-address isolation); 5 tests |
| SOCKS5-local suite | `crates/eggsec-transport-eggfetch/tests/socks5_local.rs` | Loopback SOCKS5 local-resolution via production proxy path (success + peer/ultimate fallback-forbidden + fail-closed rerun); 4 tests |
| Engine interop tests | `crates/eggsec/tests/transport_eggfetch_parity.rs` | 5 tests through `ScopeAuthority` (dev-dep only) |
| Guards | Checks 102 + 135 + 136 + 137 in `scripts/check-architecture-guards.sh` | Feature allowlist, no direct concrete clients, production load-test backend only, singular per-leg route, direct resolved-address pin, no env-proxy/H3 |

Dependency envelope (`cargo tree -p eggsec-transport-eggfetch -e features`):

- `eggsec-transport`, `eggfetch-core` (published `0.2.0`,
  `default-features = false`, `features = ["http1", "http2", "tls-rustls", "proxy"]`),
  `bytes`, `http`, `url`, `tracing`
- NOT enabled: `http3`, `cookies`, `multipart`, any compression codec
- The `proxy` feature serves pinned routing: direct logical-URL +
  `RequestBuilder::resolved_addresses([selected_target])` (singular
  socket-authorized pin; no origin DNS; Hyper H1/H2 route reuse for equal
  origin + address + SNI keys) plus `Proxy::resolved_addresses` (pinned
  proxy peer) and `RequestBuilder::proxy_target_addresses` (pinned ultimate
  target, where enforceable). Direct hops set `without_proxy` so
  environment-style proxy selection cannot divert them (`ProxyEnvironment`
  never constructed); proxied hops configure exactly one pinned peer + one
   pinned ultimate (singular per-leg rule below). `Timeout.total` spans
   response-body EOF/trailers (0.1.7 correction, retained in 0.2.0); redirect downgrade stays
  compatibility-`Allow` (no silent `Deny`).

## Architecture

### Resolution binding without an upstream hook (WS1 disposition)

Upstream `eggfetch-core` performs its own DNS (`tokio::net::lookup_host`)
inside every connector and exposes no resolver/connection authorization
hook, so a policy that resolves and then hands over a hostname cannot prove
the authorized address is the connected address. The adapter closes the
gap through public APIs instead of an upstream change:

```text
Eggsec resolver
  -> NetworkAuthority DNS authorization
  -> singular selected socket authorization
  -> logical URL remains the request URL
  -> eggfetch resolved_addresses([exact selected SocketAddr])
  -> Hyper-owned H1/H2 connection reuse for that exact route identity
```

One authoritative path: every hostname hop carries the logical URL plus the
singular socket-authorized pin (no origin DNS inside Eggfetch; proven by
`test.local` succeeding with no system DNS entry + panicking-resolver
literal tests). Literals use the same resolved-route shape with the
literal/port as the singular pin (no second hidden DNS path). Each
redirect/retry hop re-resolves fresh through the same sequence. The
pre-0.1.7 IP-literal wire-URL + logical Host + logical SNI shim is removed;
the adapter retains the explicit Host header + SNI hint as the step-1
compatibility shim (identical to the logical-URL identity).

### Proxied binding is singular per leg (2026-09-17 follow-up)

A DNS-approved set is an input to selection, never permission for the
backend to choose another member after the selected-socket checkpoint.
`AuthorizedProxyRoute` carries singular fields by construction:

```text
proxy_peer:    exactly the address passed to authorize_proxy_socket
ultimate_peer: exactly the address passed to authorize_socket
```

The Eggfetch boundary converts them to single-element pin sets
(`Proxy::resolved_addresses([proxy_peer])`,
`proxy_target_addresses([ultimate_peer])`), so the connector has no
authorized alternate to fail over to. Production rule:

> One authorization cycle selects one physical address per connection leg.
> If that address fails, the request fails. A retry may select another
> candidate only after a fresh authorization cycle and selected-socket
> checkpoint.

Future failover must live above the opaque backend retry layer (Eggsec
iterates candidates with per-attempt checkpoints, a pre-dial authorize
callback, or backend control-return on connect failure) — never as
backend-internal fallback. Redirects re-resolve/re-authorize and build a
fresh single-address route per hop; direct routes carry one selected
`SocketAddr` per hop (proven by the direct-fallback-forbidden fixture).
Supported matrix (Eggsec-local proof as of 2026-09-19): direct HTTP/HTTPS,
HTTP/HTTPS proxy → HTTPS (CONNECT) with both pins (CONNECT fixtures green),
SOCKS5 local-resolution → HTTP/HTTPS with both pins (`socks5_local` success
proves IP-target + both pins + relay; peer/ultimate fallback-forbidden
green); SOCKS5H remote-DNS and plaintext HTTP forward-proxy fail closed at
the `Proxy` checkpoint (rerun green in both `parity.rs` and `socks5_local`).

### Manual redirect loop (WS2 disposition)

Eggfetch automatic following stays doubly disabled (client
`follow_redirects(false)` + per-request `RedirectPolicy::new(false, 0)`),
so no second unrestricted redirect path exists underneath. Each hop is
computed with the public `redirect::build_redirect_request` primitive —
method rewrite, sensitive-header stripping, and body-replay rules stay
owned upstream — then:

1. `authorize_redirect(from, to)` runs before anything else;
2. the redirect-policy gate runs (`None` never follows; `SameHostOnly`
   stops cross-host and surfaces the 3xx; `AuthorityChecked` follows with
   per-hop authorization);
3. the next iteration re-runs the full checkpoint sequence on the target.

Same-origin hops restore `authorization`/`proxy-authorization` from the
previous hop (Reqwest parity; Eggfetch strips-then-reapplies from client
config, which the adapter leaves empty by design). Cross-origin hops keep
the stripped state. Only Eggfetch-followable statuses (301/302/303/307/308)
count as redirects; 300/304/305 and friends surface verbatim. Hop caps
surface the last redirect response (same as the fake), never error.

### Checkpoint order (mirrors the fake exactly)

initial-URL → host → DNS/re-resolution → socket → TLS-consistency → proxy
(+ proxy-peer DNS/socket binding when proxied) → dispatch → redirect.
Hostname hops after the first call `authorize_reresolution` (checkpoint
`reresolution`); the default delegation keeps `ScopeAuthority` verdicts
identical while custom authorities can audit distinctly. A failed
`validate_binding` maps to the same hop's DNS checkpoint (`dns` first hop,
`reresolution` later). IP literals always use `authorize_resolved`
(nothing re-resolves). Proxy endpoints resolve through the same
TOCTOU-closed path via `authorize_proxy_resolved` /
`authorize_proxy_socket` (independent from ultimate-origin resolution;
both bindings recorded/tested separately).

### TLS policy parity (WS4)

| Behavior | Current (Reqwest) | Adapter | Fixture |
|----------|-------------------|---------|---------|
| Verified roots | Mozilla/WebPKI default | `TrustStore::WebPkiOnly` (same default) | self-signed rejected |
| Insecure mode | `danger_accept_invalid_certs` (warn-logged) | separate client, warn-logged at construction, per-request opt-in | self-signed accepted |
| Hostname/SNI mismatch | rejected | rejected (SNI = logical host) | wrong-SAN cert rejected |
| Custom CA / client identity / version bounds | unused (Phase A) | not representable in `TlsPolicy` | none needed |
| Misconfigured TLS | build error, no fallback | stock configs only; backend surfaces errors at dispatch, never falls back | unit: roots build |
| ALPN/HTTP2 | negotiated where enabled | H1/H2 via ALPN (`Auto { allow_http3: false }`; HTTP/3 off; 0.2.0 bounded route-keyed reuse) | H1 keep-alive reuse (5 reqs, 1 accept) + origin/socket isolation; H2 Eggsec-local (`h2_mux`: warmed-route 4-concurrent on 1 accept/4 streams/max≥2 + 5-sequential on 1 accept + selected-address-only and logical-origin isolation, ALPN h2, `:authority` = logical host) |

Verified-success-over-TLS has no local e2e fixture (no custom-CA row in
`TlsPolicy` to trust a fixture CA with); the insecure-success test proves
the same SNI wire path handshakes, and the verified client differs only in
verification policy. A custom-CA policy row is future work only if a
consumer needs it.

### Timeouts / cancellation / bodies (parity notes)

- `total` = remaining aggregate Eggsec budget per hop (0.2.0 absolute
  deadline through body EOF/trailers, never reset by chunks; proven by
  headers-fast/body-slow, post-first-chunk stall, trickle, redirect-remainder,
  and post-timeout-reuse fixtures); `connect` mirrors the optional connect
  timeout. Pool/read/write phases stay unset — the contract carries no such
  values.
- Dropping the execute future cancels dispatch (no detached tasks); a
  regression test aborts a slow dispatch and reuses the transport.
- All contract bodies are replayable by construction; 307/308 echo tests
  prove replay across hops. Non-replayable streams are unreachable (the
  adapter never builds them; encountering one fails closed).
- Decompression is off; compressed bytes return verbatim (parity: the
  current stack configures no decompression or bomb limits either).

## Testing

- `cargo test -p eggsec-transport-eggfetch` — 6 mapping unit tests + 52
  parity/adversarial integration tests (binding, DNS-change-between-hops,
  redirect-to-denied, userinfo/unsupported-scheme redirects, cross-origin
  stripping, same-origin auth preservation, surface-vs-follow gates, hop
  cap, method/body rewrite rules, non-HTTP scheme, proxy fail-closed +
  redaction, verbatim compression, total/connect/zero timeouts,
  cancellation, TLS reject/accept/mismatch, 304 surfacing, mixed answers,
  per-hop pinning, checkpoint order, `reresolution` usage, plus the
  2026-09-17 singular-binding trio (proxy-peer/ultimate fallback forbidden,
  proxied success reports peer) plus the 0.1.7 adoption set: direct singular
  pin forbids fallback, headers-fast/body-slow + post-chunk stall + trickle
  + redirect-remainder + post-timeout reuse prove `Timeout.total` through
  EOF, H1 keep-alive reuse (5 reqs/1 accept) + target/origin isolation,
  hostile proxy env ignored, https-downgrade stays Allow, IPv6 literal,
  proxy credential isolation) + 5 H2 local tests (`h2_mux`: concurrent
  multiplex on warmed route, sequential reuse, selected-address-only +
  logical-origin isolation) + 4 SOCKS5-local tests (`socks5_local`: IP-target success with
  both pins, peer/ultimate fallback-forbidden, SOCKS5H/plaintext rerun).
  Total: 67 current tests (6 + 52 + 5 + 4) over 4 suites.
- `cargo test -p eggsec --features rest-api --test transport_eggfetch_parity`
  — 5 engine interop tests through `ScopeAuthority`.
- Phase A `network_policy_invariants.rs` (12) and Phase B
  `transport_contract.rs` (13) remain green.
- Correctness vs upstream vs performance: Eggsec-local fixtures prove H1
   reuse, H2 multiplex/reuse/selected-address and origin isolation, and
   SOCKS5-local success/pinning
   through `EggfetchTransport`. Upstream 0.1.7 qualification (Tier 1 +
   extended + HTTPX/HTTPX2 exact-SHA per the adoption plan) remains the
   historical upstream release-gate record; the 0.2.0 adoption reran the
   Eggsec-local suites above unchanged against the published 0.2.0 crate
   (private upstream refactors/performance work claimed as no Eggsec
   behavior change without Eggsec measurements). Measured 1/10/50/100 throughput/latency/connection evidence
   lives in `architecture/loadtest.md` (noisy, not a CI threshold).

## Upstream assessment (WS1–WS2 handoff input, closed)

No `eggfetch` fork/change was required: the adapter pins published
`eggfetch-core` from crates.io (`0.2.0`) rather than a branch reference,
and `eggfetch` remains independent of EggSec (no new dependency in either
direction beyond the versioned client use). The `0.2.0` release retains
the qualified direct resolved-route reuse (`RequestBuilder::resolved_addresses`
with logical-URL authority + bounded Hyper H1/H2 route cache), the
`Timeout.total`-through-body correction, and the proxy-pinning APIs
(`Proxy::resolved_addresses`, `RequestBuilder::proxy_target_addresses`,
route/cache identity with pin state) first qualified on the `0.1.7` line;
upstream adds the issue-24 streaming-decompression correction (outside
Eggsec's decompression-off production path) plus API-preserving private
architecture/performance maintenance. The 0.1.6 redirect-downgrade controls
and environment-proxy support are available upstream but deliberately unused
(downgrade stays compatibility-`Allow`; no `ProxyEnvironment`).

The earlier optional-hardening note (generic upstream `DnsResolver` hook
to drop IP-literal rewriting) is closed by 0.1.7: the adapter now uses the
first-class logical-URL + `resolved_addresses` route and the shim is
removed. Remaining optional work is SNI-hint removal (needs the
logical-SNI/certificate fixture proof) and lean `standard-http1/2` profile
evaluation (currently blocked: `proxy` pulls the full H1 slice while Eggsec
needs H2 + advanced routing; record the graph rather than optimizing by
assumption).

## Invariants & Gotchas

1. **No unchecked dispatch** — same rule as the contract; the adapter adds
   no constructor that skips the authority.
2. **Logical URL on the wire, singular pin below it** — the backend sees
   the logical URL (Host/SNI/redirect/cookie/auth identity) plus exactly
   one socket-authorized `SocketAddr` via `resolved_addresses`; it never
   performs origin DNS and never sees an IP-rewritten URL.
3. **Adapter owns `Host`** — caller-supplied `Host` headers are stripped
   before the first hop and re-set per hop (verified by fixture: the
   server observes the logical host, never the literal).
4. **Auto-everything stays off** — automatic redirects, backend retries,
   decompression, cookies: disabled in config *and* per request. Proxies
   are configured only as pinned single-address routes (direct hops set
   `without_proxy`); environment-style proxy selection cannot divert a hop.
5. **Secrets never in `Debug`** — `EggfetchTransport` debugs as an opaque
   struct; request/response redaction comes from the contract types.
6. **Production wiring is load-test only** — Check 102 allows the engine
   production dependency for pinned load-test execution (direct + supported
   proxied; no Reqwest fallback); no other crate may gain a production
   dependency (Check 103 keeps agent/NSE/proxy on `eggsec-transport` types).
7. **One address per leg** — `AuthorizedHop.selected_target` (direct) and
   `AuthorizedProxyRoute` (`proxy_peer` / `ultimate_peer`) are singular
   `SocketAddr` by construction; the backend receives single-element pin
   sets and cannot fail over to a DNS-approved but socket-unchecked address
   (guards Check 135 + 136 + direct/proxy fallback-forbidden fixtures).
   `ConnectionInfo.remote_addr` is the socket-authorized peer by
   construction (direct: ultimate; proxied: proxy peer).

---

See also: [transport.md](transport.md) (Phase B contract + Phase D increment 1), [network_dependency_baseline.md](network_dependency_baseline.md) (Phase A measurement + Phase D §7), [overview.md](overview.md)

*Last verified against source: 2026-09-22 (0.2.0 adoption requalification: logical-URL + singular resolved direct, total through body EOF, H1 reuse/isolation + H2 local multiplex/reuse/selected-address/origin isolation + SOCKS5-local success/pinning; 6 mapping + 52 parity + 5 H2 + 4 SOCKS5 + 5 interop green on published 0.2.0 with no adapter semantic change; repeated current-only 1/10/50/100 evidence in [loadtest.md](loadtest.md), version-to-version performance inconclusive; retained report in [network_dependency_closure.md](network_dependency_closure.md); cites re-verified 2026-09-22 (systematic review))*
