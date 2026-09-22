# Egress Reuse Decision Record (Phase E WS1 + Eggress 1.0.8 addendum)

Status: Decided 2026-09-13 (1.0.6: all candidates rejected). Superseded
2026-09-22 for the narrow `eggsec-web-proxy` edge only by Eggress 1.0.8
(`eggress-outbound` / `eggress-uri` accepted; all other Eggress surfaces
remain rejected).

Evaluated `eggress-uri 1.0.6` (checksum `414a171b…6090e1`) and
`eggress-routing 1.0.6` (checksum `86fe780d…0f79c7af`) plus `eggress-core 1.0.6`
(checksum `20bf3299…492d7c22`) as a transitive dependency. Protocol/transport
stacks (`eggress-embed`, `eggress-runtime`, server/protocol/SSH/QUIC/proxy)
were not probed beyond the default disposition (do not consume wholesale)
because no specific duplication case exists.

## Measurements

Isolated probe (`cargo new probe`, add crates one at a time):

- `eggress-uri` alone: 10 packages total (`probe` + `eggress-uri` + `serde`,
  `serde_core`, `serde_derive`, `proc-macro2`, `unicode-ident`, `quote`, `syn`,
  `thiserror`, `thiserror-impl`). Net new in the EggSec workspace: 1 package
  (`eggress-uri` itself; `serde`/`thiserror` already present).
- `eggress-routing` added: 62 packages total (adds `arc-swap`, `eggress-core`,
  `fastrand`, `ipnet v2`, `once_cell`, `regex`, `serde_json`, `tokio`,
  `tokio-util`, `tracing`, plus transitive `bytes`, `mio`, `socket2`, `ring`,
  `rustls`, etc.). Net new in the EggSec workspace: `eggress-routing`,
  `eggress-core`, `eggress-uri`, `ipnet v2` (workspace uses `ipnetwork`, a
  different crate), `fastrand` — at least 5 new packages plus a second IP-CIDR
  crate family.

Workspace before (2026-09-13, no egress integration):

- `cargo metadata --locked`: 591 packages (registry + path; zero git).
- `cargo tree -p eggsec-transport-eggfetch --edges normal --prefix none`: 216 lines.
- `cargo tree -p eggsec-transport --edges normal --prefix none`: 87 lines.
- `cargo tree -p eggfetch-core -i eggress-uri`: no match (eggfetch does not
  depend on eggress; no shared substrate to join).

After (projected, not applied): `+5` packages minimum for routing, `+1` for
URI alone, with no removed packages (see below: no duplicate code is deleted).
`cargo tree -d` would gain an `ipnet v2` vs `ipnetwork` duplicate family and a
second `socket2 0.5/0.6` edge via `eggress-core` (workspace already carries
both `socket2` versions via `tokio`, so no new duplicate there, but no
reduction either).

## `eggress-uri`: REJECT

Model: `ProtocolSpec` (14 variants: `Http`, `HttpOnly`, `Socks4`, `Socks5`,
`Shadowsocks`, `ShadowsocksR`, `Trojan`, `Http2`, `Http3`, `Quic`,
`WebSocket`, `Raw`, `Ssh`, `Unix`), `EndpointSpec`, `CredentialSpec`,
`ProxyHopSpec`, `ProxyChainSpec`, `RedactedUri`, `parse_proxy_chain`,
`redact_proxy_uri`.

Decision questions:

1. *Can the model represent EggSec's required HTTP/SOCKS routes without
   importing unrelated protocols into policy logic?* No. EggSec needs only
   `Direct` / `Http{endpoint, credential?}` / `All{endpoint, credential?}`
   (`eggsec-transport/src/request.rs:ProxyIntent`, endpoint separate from
   ultimate destination for two distinct authorization decisions). Adopting
   `eggress-uri` imports 11 unrelated protocols (Shadowsocks, Trojan, SSH,
   Unix-socket, Raw, QUIC, H2/H3, WebSocket, etc.) into the policy-adjacent
   type surface. Filtering them out at the boundary is itself an adapter
   layer that lives indefinitely.
2. *Are credential serialization/debug guarantees at least as strict as
   current Eggfetch/EggSec behavior?* No. EggSec rejects URL userinfo
   fail-closed at construction (`reject_url_userinfo`, checkpoint
   `initial-url`; `userinfo rejected` in `ScopedHttpRequest::new`) and redacts
   `Authorization`/`Cookie`/`Proxy-Authorization` plus proxy credentials in
   every `Debug` impl (tests assert no secret survives `format!("{:?}")`).
   `eggress-uri` parses and redacts (displays `***`), which is a weaker
   posture than reject. Adopting it would loosen the transport contract to
   match the library instead of keeping the stricter invariant.
3. *Would adoption remove duplicate parsing/redaction code?* No. EggSec's
   proxy surface is ~120 lines of narrow DTOs plus redaction helpers already
   covered by contract tests; `eggfetch-core` does not use `eggress-uri`
   (verified above), so there is no shared substrate to converge on. Adoption
   adds a conversion layer (`eggress-uri` ↔ `ProxyIntent` ↔ `eggfetch`) without
   deleting an equivalent implementation.
4. *Is dependency direction acyclic?* Yes (`eggress-uri` → consumer, no back
   edge), but acyclicity alone does not justify the edge. Direction preserved
   by rejection.

Disposition: **REJECT**. Keep `eggsec-transport::ProxyIntent` as the canonical
representation. Re-evaluate only if `eggfetch-core` itself adopts
`eggress-uri` as its native proxy-endpoint type (then the adapter could speak
it natively with zero conversion layer).

## `eggress-routing`: REJECT

Model: host exact/suffix/regex, destination CIDR/port, upstream selection,
`Router`/`SharedRoutingService`, `MatchExpr`, `PortMatcher`, explanation DTOs.
Dependencies include `regex`, `ipnet v2`, `arc-swap`, `tokio`, `tokio-util`,
`tracing`, `serde_json` (plus `eggress-core`/`eggress-uri`).

1. *Do EggSec/Eggfetch need shared route/upstream policy?* No. EggSec
   authorization is `Scope`/`TargetScope` + `HostResolver` + per-hop
   `NetworkAuthority` checkpoints (initial-url/host/dns/socket/redirect/
   reresolution/proxy/tls-consistency). Upstream selection (which healthy
   backend serves an already-authorized destination) does not exist in EggSec;
   every dispatch dials the approved binding directly (or fails closed on
   proxy). There is no equivalent code to delete — only a second policy
   language to maintain.
2. *Is the dependency increase offset?* No (see measurements: +5 packages
   minimum, new `ipnet` family, `tokio` with broad features that Phase E WS3
   is actively narrowing). `regex` and `ipnetwork` are already in the
   workspace for EggSec's own scope patterns; routing does not remove those
   uses.
3. *Authorization boundary:* EggSec authorization remains authoritative by
   construction. Egress routing may only select among already-authorized
   routes; it may never decide what is authorized to scan. Adopting a routing
   crate that owns `RouteDecision`/`SelectedRoute` invites confusion between
   route selection (performance) and authorization (security). Rejection keeps
   the single-scope-model invariant (`transport.md` §Invariants #2).

Disposition: **REJECT**. Do not adopt for CIDR matching alone
(`ipnetwork` + `TargetScope` already cover it). Reconsider only with a
concrete upstream-selection requirement plus graph evidence that equivalent
EggSec code is deleted.

## Protocol/transport stacks: REJECT (default disposition stands)

No specific duplication case was presented for `eggress-embed`,
`eggress-runtime`, server, protocol, SSH, QUIC, or proxy stacks. The
interception/MITM server (`eggsec-web-proxy/src/intercept`), raw TLS paths
(`distributed/io.rs`, `waf/bypass/smuggling.rs`), and protocol-compat NSE
libraries remain specialized by design (see baseline §7.2). No wholesale
consumption.

## Ecosystem direction (preserved)

```text
narrow egress primitives (none adopted)
        |
        v
     eggfetch (independent; no eggress edge)
        |
        v
 eggsec adapter (eggsec-transport-eggfetch, still no production consumers)
        |
        v
  eggsec domains (engine per-subsystem pending; NSE/proxy/python specialized)
```

No `eggsec -> eggress-routing` edge was added (no EggSec-specific routing
policy exists that cannot live below Eggfetch). No `eggfetch -> eggsec` or
`eggress -> eggsec` edge exists. `cargo tree -i eggress-uri` and
`cargo tree -i eggress-routing` match no workspace package (verified via the
isolated probe; workspace lockfile unchanged at 591 packages).

## Verification

- Isolated graphs captured above (`/tmp/opencode/egress-eval/probe`,
  since removed; checksums recorded here).
- Workspace graphs unchanged: `cargo tree -p eggsec-transport-eggfetch`,
  `cargo tree -d`, `cargo tree -e features` reproduce the pre-decision
  baseline (see `network_dependency_baseline.md` §1 commands).
- Guard Check 106 fails on any `eggress` manifest/source edge and requires
  this record.

*Last verified against source: 2026-09-13*

---

# Addendum 2026-09-22 — Eggress 1.0.8 selective proxy-engine adoption

Status: Accepted (narrow). Parent roadmap:
`plans/eggress-1.0.8-adoption-roadmap-2026-09-22.md`. Phase A plan:
`plans/eggress-1.0.8-phase-a-proxy-engine-adoption-2026-09-22.md`. Phase B
plan:
`plans/eggress-1.0.8-phase-b-health-qualification-and-closure-2026-09-22.md`.

The 1.0.6 rejection above remains historically valid. Eggress 1.0.8
materially changes the premise by adding the dedicated listener-free
`eggress-outbound 1.0.8` crate (published, MSRV 1.89, `default = []`) and
moving chain execution out of the full service/embed stack. This addendum
records a release-specific re-evaluation. It does not rewrite the 1.0.6
record.

## Accepted edge (only)

- `eggsec-web-proxy` may depend on published
  `eggress-outbound = "=1.0.8"` with `default-features = false` and
  `eggress-uri = "=1.0.8"`.
- No Git/path/`[patch]` override. No umbrella `eggress` facade. No
  `eggress-embed`. No optional `toml`, `pproxy-compat`, `udp`,
  `extended`, `ssh`, `quic`, `legacy-crypto`, `pproxy-legacy`, or
  `insecure-tls` feature in the initial integration.
- `eggsec-transport` and `eggsec-transport-eggfetch` remain Eggress-free.
  Eggress 1.0.8 `OutboundConnector` resolves destinations internally and
  does not expose Eggsec's authorized-resolution binding seam, so it is
  not a suitable replacement for the canonical scoped HTTP backend.
  Eggfetch remains the canonical scoped HTTP transport backend.

## Rejected (unchanged)

`eggress-routing`, `eggress-embed`, `eggress-runtime`, `eggress-server`,
advanced transports (SSH/QUIC/H3/Shadowsocks/Trojan/WebSocket outbound),
and pproxy-compat remain rejected for Eggsec. Interception/MITM remains
Eggsec-owned.

## Published 1.0.8 graph (measured 2026-09-22, isolated probe)

`eggress-outbound 1.0.8` (`default-features = false`) pulls:

- `eggress-core 1.0.8`, `eggress-relay 1.0.8`, `eggress-uri 1.0.8`,
  `eggress-protocol-http 1.0.8` (over shared `eggfetch-http-connect
  0.2.0`, already in the workspace via `eggfetch-core 0.2.0`),
  `eggress-protocol-socks 1.0.8`, `eggress-transport-tls 1.0.8`;
- shared protocol deps already present in web-proxy builds (`h2`,
  `http`, `rustls` ring-only, `tokio-rustls`, `bytes`, `thiserror`,
  `tracing`, `subtle`, `zeroize`, `base64`).

Tokio widening (explicitly recorded, not hidden): the published
`eggress-outbound` closure unifies Tokio `fs` + `signal` in addition to
the `rt`, `rt-multi-thread`, `macros`, `net`, `io-util`, `sync`, `time`
features already required by the isolated `eggsec-web-proxy` package.
`test-util` is dev-only upstream and does not enter the normal graph.
The widening is judged tolerable for the specialized web-proxy crate
only: `fs`/`signal` enable no new Eggsec code paths (Eggsec code never
calls those Tokio APIs), the MSRV stays 1.89, TLS stays ring-only, and
the maintenance benefit is deletion/consolidation of duplicated
SOCKS/HTTP-CONNECT/chain protocol ownership behind a reviewed
listener-free engine. The widening must not leak into DTO/transport
crates; guard Check 106 encodes the narrow boundary.

## Ownership boundary

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

Eggress executes the already-selected proxy route. It never performs
route selection, authorization, health policy, interception, or
evidence decisions. Proxy failures never fall back direct
(`OutboundConnector::from_chain` rejects empty chains; Eggsec maps all
Eggress failures to `WebProxyError` without retry/direct fallback).
Credentials convert at the last boundary with short plaintext lifetime;
`CredentialSpec` redacts `Debug`/`Serialize` (`****`), and Eggress typed
errors carry redacted strings only.

Protocol mapping preserves current Eggsec behavior (not aspirational
semantics): `Socks4 -> Socks4`, `Socks5 -> Socks5`, `Tor -> Socks5`,
`Http -> Http`, `Https -> Http` with `tls=false` (plaintext CONNECT;
naming debt recorded separately until a dedicated fixture proves TLS
to the proxy is intended). SOCKS4 remains IP-targeted; the explicit
SOCKS5/Tor remote-domain path is preserved and proven by fixture.

## Verification

- `cargo tree -p eggsec-web-proxy`, `cargo tree -p eggsec-web-proxy -e
  features`, `cargo tree -d` before/after recorded in the Phase A/B
  completion records.
- `cargo deny` policy green after lockfile resolution; checksums in
  `Cargo.lock`.
- Deterministic local parity fixtures for SOCKS5/SOCKS4/HTTP-CONNECT,
  auth, failures, timeout, cancellation, chaining, redaction, and
  no-direct-fallback (no Internet/root/Tor required).
- Health remains application-level HTTP(S)-through-proxy validation
  (Phase B); tunnel-only success is never reported as health.

*Addendum verified against published crates: 2026-09-22.*

---

# Corrective addendum 2026-09-22 — post-adoption compatibility pass

Status: Accepted (narrow correction, architecture unchanged). Plan:
`plans/eggress-1.0.8-post-adoption-compatibility-corrective-pass-2026-09-22.md`.

Review of the successful 1.0.8 adoption found four bounded defects; the
1.0.8 edge above remains accepted, with these corrections:

1. **Proxy endpoint literal boundary restored.** The Phase A adapter
   copied `ProxyEntry.address` into Eggress `EndpointSpec`, letting
   Eggress DNS-resolve proxy hostnames the pre-adoption
   `ProxyEntry::socket_addr()` path rejected. `hop_from_entry()` now
   validates every hop through `socket_addr()` and builds the endpoint
   from the validated IP literal (port preserved); hostname entries fail
   closed before any network behavior. Proxy endpoint configuration is
   therefore literal-address-only in this release; hostname support needs
   a separately authorized design. SOCKS5/Tor remote-domain *target*
   behavior is a separate, preserved concern. The Phase A statement that
   DNS semantics were preserved is superseded.
2. **Check 106 is now an exact allowlist.** The guard proves via
   manifest parse (`tomllib`) that the complete direct Eggress
   dependency set in `eggsec-web-proxy` is exactly `eggress-outbound`
   (=1.0.8, `default-features = false`, no optional features) and
   `eggress-uri` (=1.0.8, no optional features). Any third `eggress-*`
   edge fails even if no forbidden regex matches.
3. **Concurrent health ordering restored.** `check_concurrent()` used
   `buffer_unordered` (completion order); it now uses
   `buffered(concurrency)`, preserving enabled-input order under the
   same O(concurrency) bound with no spawn-per-proxy handles.
4. **`local_addr` classified as upstream-gated debt.** Eggress 1.0.8
   never populates `OutboundInfo.local_addr` on chain execution.
   Production paths share the centralized
   `eggress_outbound::unknown_local_addr()` sentinel — unknown metadata,
   never logged as a measured address, never read by
   policy/authorization/routing/evidence. Removal condition: a published
   Eggress release exposes the actual established-socket local address.
   Prior all-acceptance-criteria-met wording is superseded for this item.

*Corrective addendum verified against source: 2026-09-22.*
