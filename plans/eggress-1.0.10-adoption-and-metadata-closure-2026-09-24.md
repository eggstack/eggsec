# Eggress 1.0.10 Adoption and Socket-Metadata Closure

## Status

**EXECUTED — 2026-09-25**

## Target repository

`eggstack/eggsec`

Planning baseline:

`6d25b1c86e5628aaadaf83cdc9470e5905ae647e` (`main`)

## Context

Eggsec currently consumes the deliberately narrow listener-free Eggress edge in
`eggsec-web-proxy`:

```toml
eggress-outbound = { version = "=1.0.8", default-features = false }
eggress-uri = { version = "=1.0.8" }
```

The 1.0.8 adoption successfully moved SOCKS4/SOCKS5/Tor/HTTP CONNECT chain
execution behind `eggress-outbound` while preserving Eggsec ownership of
authorization, proxy selection, health, interception, and evidence policy.

One upstream-gated compatibility debt remained: Eggress 1.0.8 returned
`OutboundInfo.local_addr = None` for chain execution, so Eggsec centralized an
explicit `0.0.0.0:0` unknown sentinel in
`crates/eggsec-web-proxy/src/eggress_outbound.rs`.

Eggress 1.0.10 is now published on crates.io and contains the socket-metadata
recovery needed to close that debt. For ordinary TCP-backed first hops,
`eggress-core::ChainExecutor::execute_with_metadata()` captures
`TcpStream::local_addr()` and `peer_addr()` before stream boxing and
`eggress-outbound::OutboundConnector` exposes those values in
`OutboundInfo`.

Eggsec's approved protocols on this edge are all ordinary TCP-backed:

- SOCKS4;
- SOCKS5;
- Tor mapped to SOCKS5;
- HTTP CONNECT;
- the existing `Https` enum compatibility behavior, which remains plaintext
  HTTP CONNECT.

Eggsec does not enable Eggress H2, SSH, QUIC/H3, UDP, routing, runtime/server,
pproxy-compat, extended protocols, or insecure TLS on this boundary. The
1.0.10 pooled-transport and TLS-policy hardening is therefore inherited
upstream safety, not a reason to widen Eggsec's capability surface.

## Goal

Adopt the exact published Eggress 1.0.10 narrow dependency edge and replace
the 1.0.8 unknown-local-address workaround with truthful measured first-hop
socket metadata, while preserving every existing Eggsec security and behavior
boundary.

The final invariant is:

> Every successful Eggsec production proxy dial through the approved
> SOCKS4/SOCKS5/Tor/HTTP CONNECT Eggress path carries a real local socket
> address for the physical TCP connection to the first proxy hop. Eggsec never
> fabricates a local address and never interprets it as the final external
> egress address.

---

# Workstream 0 — freeze and measure the baseline

Record before modification:

```sh
git rev-parse HEAD
git status --short
cargo tree -p eggsec-web-proxy --edges normal
cargo tree -p eggsec-web-proxy -e features
cargo tree -d
```

Record the current direct dependency declarations and Check 106 behavior.

Confirm the baseline tests that define the adoption contract, especially:

- `socks5_no_auth_success_via_manager`;
- `socks5_domain_target_reaches_proxy_as_domain`;
- `socks4_ip_target_success`;
- `http_connect_success`;
- `two_hop_chain_ordering`;
- `mixed_http_socks_chain_via_adapter`;
- `no_direct_fallback_on_proxy_failure`;
- `hostname_proxy_endpoint_rejected_before_network`;
- `tor_remote_domain_preserved_after_literal_gate`;
- `local_addr_is_unknown_sentinel_not_measured` (expected to be superseded).

Do not change behavior before the baseline is recorded.

---

# Workstream 1 — bump only the approved Eggress edge

Update `crates/eggsec-web-proxy/Cargo.toml`:

```toml
eggress-outbound = { version = "=1.0.10", default-features = false }
eggress-uri = { version = "=1.0.10" }
```

Retain all existing restrictions:

- exact version pins;
- `default-features = false` on `eggress-outbound`;
- no optional Eggress features;
- no umbrella `eggress` crate;
- no `eggress-embed`;
- no `eggress-runtime`;
- no `eggress-server`;
- no `eggress-routing`;
- no direct advanced-protocol crates;
- no Eggress edge in `eggsec-transport`,
  `eggsec-transport-eggfetch`, `eggsec-policy`, `eggsec-core`, DTO
  crates, or unrelated domains.

Regenerate `Cargo.lock` from crates.io only.

Verify:

```sh
cargo tree -p eggsec-web-proxy --edges normal
cargo tree -p eggsec-web-proxy -e features
cargo tree -i eggress-outbound
cargo tree -i eggress-uri
cargo tree -d
```

The resolved Eggress family must be the published 1.0.10 line with no Git/path
override.

Do not loosen exact pins to `1.0`, `^1.0.10`, or a range in this pass.

---

# Workstream 2 — update Check 106 without weakening it

Update `scripts/check-architecture-guards.sh` Check 106 from 1.0.8 to 1.0.10.

The manifest-aware exact allowlist must still prove that the complete direct
Eggress dependency set in `eggsec-web-proxy` is exactly:

```text
eggress-outbound
eggress-uri
```

Required constraints:

- `eggress-outbound.version == "=1.0.10"`;
- `eggress-outbound.default-features == false`;
- `eggress-outbound.features` is absent/empty;
- `eggress-uri.version == "=1.0.10"`;
- `eggress-uri` enables no optional features;
- no third direct `eggress-*` dependency is allowed.

Update the regex backstops and human-readable PASS/FAIL messages to 1.0.10.

Preserve all existing guard clauses that keep:

- other workspace manifests Eggress-free;
- transport/policy/core Eggress-free;
- embed/runtime/server/routing/advanced surfaces forbidden;
- external Eggress source references confined to the
  `eggsec-web-proxy::eggress_outbound` adapter and tests.

The guard must continue to fail on accidental feature/capability expansion.

---

# Workstream 3 — retire the unknown-local-address sentinel

In `crates/eggsec-web-proxy/src/eggress_outbound.rs`:

Remove:

- `unknown_local_addr()`;
- `local_addr_or_unknown()`;
- 1.0.8 comments describing `OutboundInfo.local_addr` as always absent;
- the unit test that legitimizes the unspecified sentinel.

Replace the compatibility fallback with a fail-closed helper that requires
measured metadata for Eggsec's approved TCP-backed protocol set.

Suggested private contract:

```rust
fn require_local_addr(info: &OutboundInfo) -> Result<SocketAddr> {
    info.local_addr.ok_or_else(|| {
        WebProxyError::Proxy(
            "Eggress returned no local address for TCP-backed proxy connection".to_string()
        )
    })
}
```

Exact wording may follow repository conventions, but it must:

- contain no credentials;
- not claim the address represents the final target or external egress IP;
- fail rather than fabricate a socket address.

Use the helper in:

- `ProxyManager::create_connection()`;
- `ProxyManager::create_connection_to_domain()`;
- `ProxyManager::create_chained_connection()`.

Remove debug logging that says missing `local_addr` is expected.

Do not change the public type of
`ProxiedConnection.local_addr: SocketAddr`; 1.0.10 now satisfies that
existing contract for the admitted TCP-backed path.

---

# Workstream 4 — document metadata semantics precisely

Update the adapter and public proxy-manager comments to define
`ProxiedConnection.local_addr` correctly:

- it is the local socket endpoint of Eggsec's physical TCP connection to the
  **first proxy hop**;
- for a multi-hop chain, it is still the first-hop TCP socket metadata;
- it is not the final destination socket;
- it is not the public/external IP observed after the last proxy;
- it must never be used as evidence of the final egress identity.

Likewise, where `OutboundInfo.peer_addr` is exposed/tested:

- it represents the remote socket address of the first proxy hop for ordinary
  TCP-backed chains.

This distinction should appear in
`architecture/egress_reuse_decision.md` so future consumers do not turn
transport metadata into a misleading “exit IP” claim.

---

# Workstream 5 — strengthen the Eggress parity fixtures

Extend `crates/eggsec-web-proxy/tests/eggress_parity.rs`.

Replace/supersede:

`local_addr_is_unknown_sentinel_not_measured`

with real socket-metadata coverage.

Mandatory assertions:

## 5.1 Single-hop SOCKS5

Use the existing deterministic local SOCKS5 fixture.

Assert on successful adapter/manager connection:

- `local_addr.ip()` is a concrete loopback/local fixture address;
- `local_addr.port() != 0`;
- `local_addr.ip().is_unspecified() == false`;
- `peer_addr`, when inspecting `OutboundInfo`, equals the selected proxy
  listener address;
- `hop_count == 1`.

## 5.2 SOCKS4

Extend `socks4_ip_target_success` or add a focused adjacent test proving the
same real `local_addr` invariant.

## 5.3 HTTP CONNECT

Extend `http_connect_success` or add a focused adjacent test proving real
first-hop `local_addr` and correct `peer_addr`.

## 5.4 Remote-domain SOCKS5/Tor

Extend one or both:

- `socks5_manager_domain_path`;
- `tor_remote_domain_preserved_after_literal_gate`.

Prove remote-domain target semantics remain unchanged while local socket
metadata is now measured.

## 5.5 Multi-hop chain

Extend `two_hop_chain_ordering`.

Prove:

- successful two-hop chain still follows the selected order;
- returned local socket address is real/nonzero;
- metadata describes the connection to the first hop, not the second hop or
  final target.

The fixture should compare `peer_addr` to the first hop where practical.

## 5.6 IPv6

Retain the existing
`literal_ipv4_and_ipv6_proxy_endpoints_still_work` coverage.

If the existing deterministic IPv6 fixture is reliable on CI, assert real
IPv6 local metadata there as well. If CI platform IPv6 availability is
conditional, keep IPv4 metadata mandatory and document any IPv6 skip rather
than weakening the cross-platform suite.

---

# Workstream 6 — preserve all 1.0.8 corrective boundaries

The 1.0.10 update must not regress the prior corrective pass.

Explicitly rerun and preserve:

- proxy endpoint hostnames rejected before network activity;
- credential-safe hostname rejection;
- target remote-domain semantics only for SOCKS5/Tor;
- SOCKS4 domain path fails closed;
- no direct fallback;
- timeout remains bounded;
- future-drop cancellation remains safe;
- chain ordering remains deterministic;
- `Https` enum still maps to plaintext HTTP CONNECT;
- health result ordering remains enabled-input order;
- Reqwest remains the health-only owner.

Do not broaden proxy-hostname support merely because Eggress can resolve
endpoint domains internally. Eggsec's literal proxy-endpoint gate remains an
intentional compatibility/security boundary.

---

# Workstream 7 — architecture/documentation reconciliation

Append a dated Eggress 1.0.10 addendum to:

`architecture/egress_reuse_decision.md`

Do not rewrite or delete the historical 1.0.6/1.0.8 decisions.

The addendum must record:

1. direct approved edge moved from exact 1.0.8 to exact 1.0.10;
2. architecture/capability boundary is unchanged;
3. 1.0.10 resolves the upstream socket-metadata gate;
4. the 1.0.8 unknown sentinel is removed;
5. measured `local_addr` / `peer_addr` refer to the first-hop TCP socket;
6. Eggsec still does not adopt routing/runtime/server/embed/advanced features;
7. Reqwest health ownership remains unchanged;
8. typed detailed outbound errors are deliberately deferred to a separate
   future behavior-change decision.

Update comments in:

- `crates/eggsec-web-proxy/Cargo.toml`;
- `crates/eggsec-web-proxy/src/eggress_outbound.rs`;
- `crates/eggsec-web-proxy/src/lib.rs`;
- `plans/README.md`.

Historical 1.0.8 plan files remain intact as provenance. Add completion notes
only if the repository convention calls for them.

---

# Workstream 8 — explicitly defer typed detailed-error adoption

Eggress 1.0.10 exposes:

- `connect_tcp_detailed()`;
- `connect_tcp_timeout_detailed()`;
- structured failure kind;
- failure stage;
- hop index;
- protocol label.

Do not adopt those methods in this plan.

Eggsec currently maps outbound failures to
`WebProxyError::Proxy(String)`. Changing that to `Timeout`, `Tls`,
`Network`, `Protocol`, or `Config` variants would be an observable error
classification/API behavior change and requires its own consumer audit.

Record this as a follow-up opportunity, not unfinished acceptance work.

The existing credential-safe `map_outbound_error()` path remains canonical
for this adoption.

---

# Workstream 9 — dependency and build qualification

Required focused checks:

```sh
cargo test -p eggsec-web-proxy --test eggress_parity --no-fail-fast
cargo test -p eggsec-web-proxy --test health_matrix --no-fail-fast
cargo test -p eggsec-web-proxy --lib --no-fail-fast
cargo check -p eggsec-web-proxy --no-default-features
cargo check -p eggsec-web-proxy --features web-proxy
cargo clippy -p eggsec-web-proxy --features web-proxy -- -D warnings
bash scripts/check-architecture-guards.sh
```

Required workspace gates:

```sh
cargo fmt --all -- --check
make check
make check-feature-profiles
```

Run `make check-full` when platform prerequisites for the deep domain lanes
are available.

Supply-chain checks must confirm the new lockfile state:

```sh
make check-deps
cargo tree -p eggsec-web-proxy --edges normal
cargo tree -p eggsec-web-proxy -e features
cargo tree -d
```

Record any dependency-count/duplicate-family delta from the 1.0.8 baseline.
Do not accept a new Eggress optional capability merely because it appears
transitively after a feature unification elsewhere.

---

# Workstream 10 — completion evidence

Append a completion record to this plan after implementation.

Record:

- implementation SHA;
- exact resolved Eggress versions/checksums from `Cargo.lock`;
- Check 106 result;
- focused parity test count/result;
- new metadata test names;
- workspace gate results;
- dependency graph delta;
- confirmation that no sentinel helper/reference remains;
- confirmation that no optional Eggress features/direct crates were added;
- confirmation that `eggsec-transport` and Eggfetch integration remain
  Eggress-free;
- confirmation that health still uses Reqwest;
- final CI run/result if available.

Update `plans/README.md` from READY to executed only after the implementation
and qualification evidence exists.

---

## Expected files touched

Likely:

```text
Cargo.lock
crates/eggsec-web-proxy/Cargo.toml
crates/eggsec-web-proxy/src/eggress_outbound.rs
crates/eggsec-web-proxy/src/lib.rs
crates/eggsec-web-proxy/tests/eggress_parity.rs
scripts/check-architecture-guards.sh
architecture/egress_reuse_decision.md
plans/eggress-1.0.10-adoption-and-metadata-closure-2026-09-24.md
plans/README.md
```

No new crate or public dependency surface is expected.

---

## Non-goals

- no Eggress H2 adoption;
- no SSH adoption;
- no QUIC/H3 adoption;
- no UDP adoption;
- no `eggress-routing`, runtime, server, embed, or pproxy-compat adoption;
- no proxy-hostname support;
- no change to Eggsec authorization/scope policy;
- no change to proxy rotation/health ownership;
- no Reqwest removal;
- no `Https` enum semantic correction;
- no detailed outbound-error classification migration;
- no change to public `ProxiedConnection` field types;
- no release publication.

---

## Stop conditions

Stop and document a blocker if:

- crates.io resolution cannot produce exact 1.0.10 for the approved Eggress
  edge;
- any admitted SOCKS4/SOCKS5/HTTP CONNECT success path still returns
  `local_addr = None`;
- `peer_addr` no longer represents the selected first TCP hop;
- adopting 1.0.10 requires enabling an Eggress optional feature;
- Check 106 cannot retain the exact narrow allowlist;
- proxy endpoint hostname rejection changes;
- remote-domain target behavior changes;
- no-direct-fallback changes;
- dependency policy/audit gates fail due to the new resolved graph.

Do not reintroduce the unspecified sentinel to paper over a metadata failure.
If an approved TCP path produces `None`, treat it as an upstream regression
and stop.

---

## Acceptance criteria

This adoption is complete only when:

1. `eggress-outbound` is pinned exactly to 1.0.10 with
   `default-features = false`.
2. `eggress-uri` is pinned exactly to 1.0.10.
3. `Cargo.lock` resolves the published 1.0.10 Eggress family with no Git/path
   override.
4. Check 106 enforces the exact two-crate 1.0.10 allowlist.
5. No other Eggress direct edge or optional feature is introduced.
6. `unknown_local_addr()` is removed.
7. `local_addr_or_unknown()` is removed.
8. Successful approved TCP-backed proxy paths require real `local_addr`.
9. No successful path uses an unspecified/port-zero sentinel.
10. SOCKS5 metadata is proven with a real local address and correct first-hop
    peer address.
11. SOCKS4 metadata is proven.
12. HTTP CONNECT metadata is proven.
13. SOCKS5/Tor remote-domain semantics remain unchanged and metadata is real.
14. Multi-hop metadata is proven to describe the first physical proxy hop.
15. Proxy endpoint hostname rejection remains pre-network and credential-safe.
16. No-direct-fallback, timeout, cancellation, auth, failure, and chain-order
    regressions remain green.
17. Health remains application-level Reqwest-owned and ordered.
18. Architecture documentation records first-hop metadata semantics and the
    resolved 1.0.8 debt.
19. Detailed typed Eggress failure adoption is explicitly deferred.
20. Focused web-proxy tests/checks are green.
21. Check 106 and all architecture guards are green.
22. `make check` is green.
23. Representative feature-profile checks are green.
24. Dependency policy is green.
25. Completion evidence is recorded and `plans/README.md` reflects executed
    state.

## Completion record

Executed 2026-09-25.

- Implementation SHA: `fe5ec2d2d0fe0557fd5559bf6a0720d04776023f`
  (pre-implementation HEAD `e601ee3e`; plan baseline `6d25b1c8` plus the
  two handoff-registration commits).
- Exact resolved Eggress family from `Cargo.lock` (all published
  crates.io, no Git/path override):
  - `eggress-outbound 1.0.10` (`b022665c…8ab89ad`);
  - `eggress-uri 1.0.10` (`eaa253b7…f61736f60a`);
  - `eggress-core 1.0.10` (`1522de7d…21efaa8b75f`);
  - `eggress-relay 1.0.10` (`a8577196…379ea1f355`);
  - `eggress-protocol-http 1.0.10` (`a673bca0…e801a62494cb451`);
  - `eggress-protocol-socks 1.0.10` (`b039b603…941f1f6ea755`);
  - `eggress-transport-tls 1.0.10` (`f0dcaec1…023d36693c0`).
- Check 106 result: PASS — exact allowlist
  (`eggress-outbound` + `eggress-uri`, 1.0.10 pinned, no optional
  features); full guard suite: ALL PASSED.
- Focused parity: `eggress_parity` 30 passed (was 27 at 1.0.8: sentinel
  test superseded, 4 metadata tests added); `health_matrix` 15 passed;
  `eggsec-web-proxy --lib` 395 passed.
- New metadata tests: `socks5_first_hop_metadata_is_measured`,
  `socks4_first_hop_metadata_is_measured`,
  `http_connect_first_hop_metadata_is_measured`,
  `local_addr_is_measured_first_hop_socket_not_sentinel` (supersedes
  `local_addr_is_unknown_sentinel_not_measured`); extended with real
  first-hop assertions: `socks5_no_auth_success_via_manager`,
  `socks5_auth_success`, `socks5_domain_target_reaches_proxy_as_domain`,
  `socks5_manager_domain_path`, `socks4_ip_target_success`,
  `http_connect_success`, `two_hop_chain_ordering` (peer == entry hop),
  `tor_remote_domain_preserved_after_literal_gate`,
  `literal_ipv4_and_ipv6_proxy_endpoints_still_work` (IPv4 mandatory,
  live IPv6 opportunistic with documented skip).
- Workspace gates: `cargo fmt --check` clean; `make check` exit 0
  (includes `check-deps`/clippy/doc/tool-registration/rest-api-cli/
  output/report/policy/eggfetch/tui/guards); `make check-feature-profiles`
  exit 0.
- Dependency graph delta from the 1.0.8 baseline: same 7-crate Eggress
  family, versions `1.0.8` → `1.0.10` only; no new crates, no new
  duplicate families; transitive Eggress features remain `default`-only
  (no udp/ssh/quic/toml/pproxy-compat/extended/insecure-tls unification).
- No sentinel helper/reference remains in `src/` or `tests/`
  (`unknown_local_addr` / `local_addr_or_unknown` / `0.0.0.0:0` gone;
  only historical prose in decision-record/test-supersession comments).
- No optional Eggress features or direct crates added (Check 106 exact
  allowlist proves the complete direct set).
- `eggsec-transport` and Eggfetch integration remain Eggress-free
  (guard-enforced, green).
- Health still uses Reqwest (untouched; `health_matrix` green, SOCKS4
  still fails closed).
- Typed detailed Eggress failures (`connect_tcp_detailed`) explicitly
  deferred per Workstream 8 (recorded in the 1.0.10 decision addendum,
  adapter docs, and proxy skill).
