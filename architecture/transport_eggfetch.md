# Eggfetch Transport Adapter (Phase C)

Status: adapter implemented 2026-09-12 (`eggsec-transport-eggfetch`).
Phase D increment 1 (2026-09-12) migrated consumer *interfaces* to the
contract (`eggsec-agent` injection, shared-helper cleanup, NSE capability,
proxy boundary) but wired no production backend yet — this crate still has
no production consumers (guard Checks 102 + 103). Migration proceeds per
consumer with focused parity tests; see
[network_dependency_baseline.md](network_dependency_baseline.md) §7.

## Role & Responsibilities

[`EggfetchTransport`](../../../crates/eggsec-transport-eggfetch/src/adapter.rs)
implements [`HttpTransport`](transport.md) over the **published**
`eggfetch-core 0.1` client using only stable public APIs. It proves the
Phase B contract can be enforced against a real backend with no upstream
Eggfetch change and no production migration.

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
| Adapter crate | `crates/eggsec-transport-eggfetch/` | 18th workspace crate; lib only, no features |
| Parity/adversarial suite | `crates/eggsec-transport-eggfetch/tests/parity.rs` + `tests/common/` | Local loopback fixtures only (plain + TLS); 33 tests |
| Engine interop tests | `crates/eggsec/tests/transport_eggfetch_parity.rs` | 5 tests through `ScopeAuthority` (dev-dep only) |
| Guard | Check 102 in `scripts/check-architecture-guards.sh` | Feature allowlist, no direct concrete clients, no production consumers |

Dependency envelope (`cargo tree -p eggsec-transport-eggfetch -e features`):

- `eggsec-transport`, `eggfetch-core` (published `0.1`,
  `default-features = false`, `features = ["http1", "tls-rustls", "proxy"]`),
  `bytes`, `http`, `url`, `tracing`
- NOT enabled: `http3`, `cookies`, `multipart`, any compression codec
- The `proxy` feature is enabled for one narrow reason only: without it the
  SNI-direct connector path builds no TLS connector, and the adapter needs
  that path (pinned-IP URL with SNI = logical hostname) for verified HTTPS.
  Proxy *routing* stays disabled by construction — no proxy is ever
  configured and every request sets `without_proxy` — so environment or
  per-request proxy selection (including remote-DNS SOCKS semantics, where
  the local process never observes the target IP) cannot divert a hop.

## Architecture

### Resolution binding without an upstream hook (WS1 disposition)

Upstream `eggfetch-core` performs its own DNS (`tokio::net::lookup_host`)
inside every connector and exposes no resolver/connection authorization
hook, so a policy that resolves and then hands over a hostname cannot prove
the authorized address is the connected address. The adapter closes the
gap through public APIs instead of an upstream change:

```text
resolve host (injected TransportResolver facts)
  -> authority.authorize_resolved / authorize_reresolution
  -> validate_binding (approved must be a non-empty subset of candidates)
  -> authority.authorize_socket (primary, immediately before dispatch)
  -> rewrite the wire URL host to the approved IP literal
  -> connector resolves the literal to itself (no network DNS)
logical hostname preserved via the adapter-owned Host header + TLS SNI
```

One authoritative path: every hostname hop is pinned; IP literals skip
resolution (proven by a panicking-resolver test). Each redirect/retry hop
re-resolves fresh through the same sequence.

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
→ dispatch → redirect. Hostname hops after the first call
`authorize_reresolution` (checkpoint `reresolution`); the default
delegation keeps `ScopeAuthority` verdicts identical while custom
authorities can audit distinctly. A failed `validate_binding` maps to the
same hop's DNS checkpoint (`dns` first hop, `reresolution` later). IP
literals always use `authorize_resolved` (nothing re-resolves). One
intentional divergence remains: the fake simulates proxy-endpoint DNS and
binding while the adapter fails closed on any proxy (deferred past
Phase C) — there is no backend proxy behavior to mirror.

### TLS policy parity (WS4)

| Behavior | Current (Reqwest) | Adapter | Fixture |
|----------|-------------------|---------|---------|
| Verified roots | Mozilla/WebPKI default | `TrustStore::WebPkiOnly` (same default) | self-signed rejected |
| Insecure mode | `danger_accept_invalid_certs` (warn-logged) | separate client, warn-logged at construction, per-request opt-in | self-signed accepted |
| Hostname/SNI mismatch | rejected | rejected (SNI = logical host) | wrong-SAN cert rejected |
| Custom CA / client identity / version bounds | unused (Phase A) | not representable in `TlsPolicy` | none needed |
| Misconfigured TLS | build error, no fallback | stock configs only; backend surfaces errors at dispatch, never falls back | unit: roots build |
| ALPN/HTTP2 | negotiated where enabled | HTTP/1.1 pinned (`Http1Only`) | all fixtures |

Verified-success-over-TLS has no local e2e fixture (no custom-CA row in
`TlsPolicy` to trust a fixture CA with); the insecure-success test proves
the same SNI wire path handshakes, and the verified client differs only in
verification policy. A custom-CA policy row is future work only if a
consumer needs it.

### Timeouts / cancellation / bodies (parity notes)

- `total` = contract `request_timeout` shrunk per hop (aggregate bound
  holds across redirects); `connect` mirrors the optional connect timeout.
  Pool/read/write phases stay unset — the contract carries no such values.
- Dropping the execute future cancels dispatch (no detached tasks); a
  regression test aborts a slow dispatch and reuses the transport.
- All contract bodies are replayable by construction; 307/308 echo tests
  prove replay across hops. Non-replayable streams are unreachable (the
  adapter never builds them; encountering one fails closed).
- Decompression is off; compressed bytes return verbatim (parity: the
  current stack configures no decompression or bomb limits either).

## Testing

- `cargo test -p eggsec-transport-eggfetch` — 8 mapping unit tests + 33
  parity/adversarial integration tests (binding, DNS-change-between-hops,
  redirect-to-denied, userinfo/unsupported-scheme redirects, cross-origin
  stripping, same-origin auth preservation, surface-vs-follow gates, hop
  cap, method/body rewrite rules, non-HTTP scheme, proxy fail-closed +
  redaction, verbatim compression, total/connect/zero timeouts,
  cancellation, TLS reject/accept/mismatch, 304 surfacing, mixed answers,
  per-hop pinning, checkpoint order, `reresolution` usage).
- `cargo test -p eggsec --features rest-api --test transport_eggfetch_parity`
  — 5 engine interop tests through `ScopeAuthority`.
- Phase A `network_policy_invariants.rs` (12) and Phase B
  `transport_contract.rs` (11) remain green and unmodified.

## Upstream assessment (WS1–WS2 handoff input)

No `eggfetch` change was required for the properties above, and none is
consumed: the adapter pins `eggfetch-core` from crates.io (`0.1`, currently
resolving to `0.1.3`) rather than a branch reference, and `eggfetch`
remains independent of EggSec (no new dependency in either direction
beyond the versioned client use).

Recommended optional hardening (not a prerequisite): a generic upstream
`DnsResolver` trait (`resolve(host, port) -> Vec<SocketAddr>`, defaulting
to the current `lookup_host` behavior and threaded through the direct,
proxy, SOCKS, and HTTP/3 connectors) plus a pre-follow redirect callback
would let a future adapter revision drop IP-literal rewriting in favor of
direct approved-address binding. Recorded here so Phase D/G can evaluate
it against a released `eggfetch-core` with the hook.

Re-evaluated 2026-09-12: latest released `eggfetch-core` is still `0.1.3`
(`cargo search`; workspace lockfile pins `0.1.3`) — no resolver hook or
pre-follow callback exists in any release, so there is nothing to adopt.
The adapter-level binding + manual loop stand unchanged; re-evaluate when
upstream ships the hook.

## Invariants & Gotchas

1. **No unchecked dispatch** — same rule as the contract; the adapter adds
   no constructor that skips the authority.
2. **Pinned literals only on the wire** — the backend never sees an
   authorizable hostname; `Host`/SNI carry the logical name.
3. **Adapter owns `Host`** — caller-supplied `Host` headers are stripped
   before the first hop and re-set per hop (verified by fixture: the
   server observes the logical host, never the literal).
4. **Auto-everything stays off** — redirects, retries, decompression,
   cookies, proxies: disabled in config *and* per request.
5. **Secrets never in `Debug`** — `EggfetchTransport` debugs as an opaque
   struct; request/response redaction comes from the contract types.
6. **No production wiring** — Checks 102/103 fail on any non-dev dependency
   from another crate; Phase D owns migration order (increment 1 migrated
   interfaces only — agent/NSE/proxy depend on `eggsec-transport` types,
   never on this backend crate).

---

See also: [transport.md](transport.md) (Phase B contract + Phase D increment 1), [network_dependency_baseline.md](network_dependency_baseline.md) (Phase A measurement + Phase D §7), [overview.md](overview.md)

*Last verified against source: 2026-09-12*
