# Egress Reuse Decision Record (Phase E WS1)

Status: Decided 2026-09-13. No workspace integration (all candidates rejected).

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
