# Eggfetch 0.1.7 adoption and direct-route simplification

Status: Ready for handoff

Date: 2026-09-18

Eggsec planning baseline: `49cd4bf70efe5c8cb24a8199e04d28b51244f4f9`

Upstream release baseline:

- published crate: `eggfetch-core 0.1.7` on crates.io;
- coordinated Eggfetch release commit:
  `43c3b312f2def887d0f0b7ce539faa626adf2cc8`;
- annotated tag: `v0.1.7`;
- final executable/test freeze for the 0.1.7 total-deadline correction:
  `82f3f38631b44a9a5c5ec5b40790e5015aeb40f8`;
- upstream release CI on the release commit: run `35385440508` (green);
- Eggfetch's release closure records Tier 1, extended, package, security,
  Rust 1.89 MSRV, and renewed HTTPX/HTTPX2 exact-SHA qualification before
  publication.

Depends on the already executed Eggsec load-test authorization/transport
corrective line, including the 2026-09-17 multi-address socket-binding
corrective. This plan must not reopen those completed scope or proxy-policy
changes.

## Objective

Move Eggsec's published Eggfetch dependency from the currently locked
`eggfetch-core 0.1.5` to `0.1.7`, consume the upstream route-reuse and
response-body deadline corrections that materially improve Eggsec's scoped
transport, and simplify the direct route from the adapter-owned
"IP-literal wire URL + logical Host + logical SNI" compatibility shim to
Eggfetch's first-class logical-URL + `resolved_addresses()` route.

The desired end state is:

```text
Eggsec resolver
  -> NetworkAuthority DNS authorization
  -> singular selected socket authorization
  -> logical URL remains the request URL
  -> eggfetch resolved_addresses([exact selected SocketAddr])
  -> Hyper-owned H1/H2 connection reuse for that exact route identity
```

The adapter must continue to own authorization, scope, redirect decisions,
proxy route selection, and the one-socket-per-leg security rule. Eggfetch owns
HTTP/TLS transport mechanics and physical connection reuse below that policy
boundary.

This is a bounded dependency/integration correction. It is not a new transport
architecture program.

## Upstream 0.1.7 review

### 1. Resolved direct routes are now reusable

Eggfetch 0.1.7 retains the static direct-routing contract introduced earlier:
`RequestBuilder::resolved_addresses()` keeps the logical URL authoritative
for HTTP Host, TLS SNI/certificate verification, redirects, cookies, and auth,
while the supplied `SocketAddr` set controls the physical destination and
origin DNS is not consulted.

The 0.1.7 change is connection lifetime: equal direct resolved routes now reuse
one bounded Hyper client instead of constructing an isolated client for each
request. The upstream cache is bounded to 64 configured clients and is keyed by:

```text
logical HTTP origin
+ full ordered resolved-address snapshot
+ exact optional SNI override
```

Hyper remains the physical H1/H2 pool. Upstream qualification demonstrates H1
keep-alive reuse and H2 multiplexing while preserving isolation across origin,
address-set/order, and SNI changes.

This directly removes the performance reason Eggsec previously had to retain
its IP-literal wire-URL workaround instead of using the generic resolved route.

### 2. `Timeout.total` now spans response-body EOF/trailers

Eggfetch 0.1.7 fixes native `Timeout.total` so one absolute request deadline
continues through response-body EOF/trailers on the high-level
`bytes()/text()/json()/bytes_stream()/raw_bytes_stream()` surfaces and the
native frame-preserving surfaces. It does not reset on chunk arrival, decode
selection, or first body poll.

This matters to Eggsec because `EggfetchTransport::send_hop()` passes the
remaining Eggsec request budget as `Timeout.total`, then
`HttpTransport::execute()` calls `response.bytes().await`. Under the current
0.1.5 lock, the adapter's outer `Instant` only prevents starting a later hop
after the budget is exhausted; it cannot itself stop a final response body
that returned headers before the deadline and then stalls or trickles beyond
the budget.

After 0.1.7, the same remaining-budget mapping should enforce the intended
Eggsec request timeout through body completion. The adapter still needs its
outer elapsed-time calculation because its manual authorization redirect loop
performs multiple independent Eggfetch sends; each hop must receive only the
remaining original Eggsec budget.

### 3. 0.1.6 redirect-downgrade controls are available but are not an implicit
Eggsec policy change

The cumulative 0.1.5 -> 0.1.7 upgrade also includes
`RedirectDowngradePolicy::{Allow, Deny}`,
`RedirectPolicy::strict()`, and
`build_redirect_request_with_redirect_policy()`.

Eggsec's neutral `eggsec-transport::RedirectPolicy` currently models:

- no redirects;
- same-host-only redirects;
- authority-checked redirects.

It does not define an HTTPS -> HTTP downgrade policy. The Eggfetch adapter must
therefore **not** silently opt into `Deny` and become behaviorally stricter
than the transport contract/recording fake. Keep current downgrade behavior
unless and until a separate neutral-contract change explicitly introduces
that policy dimension.

The manual redirect loop also remains mandatory because every new logical
origin must pass Eggsec `NetworkAuthority` before dispatch.

### 4. 0.1.6 environment-proxy support must remain unused

The cumulative upgrade adds explicit opt-in `ProxyEnvironment` support.
Eggsec must not adopt it in this transport.

Eggsec proxy configuration is an authorization decision:
the logical proxy endpoint, resolved proxy peer, selected proxy socket, logical
ultimate target, resolved ultimate peer, and selected ultimate socket are
checked separately. Reading `HTTP_PROXY` / `HTTPS_PROXY` / `ALL_PROXY`
inside the backend would create an unowned routing input below that policy
boundary.

Required disposition:

- direct requests continue to call `.without_proxy()`;
- proxied requests continue to originate only from explicit
  `eggsec_transport::ProxyIntent`;
- no `ProxyEnvironment`, `proxy_environment()`, or environment-derived
  proxy fallback in the adapter.

### 5. The 0.1.7 feature split is useful to understand, but does not justify an
unproven profile change here

Eggfetch 0.1.7 splits H1/H2 transport, standard route, advanced routing, and
high-level policy features, and adds lean `standard-http1/2` recipes.

Eggsec requires:

- advanced direct resolved routing;
- H1/H2;
- Rustls;
- explicit proxy routing and pinned proxy/ultimate targets.

At 0.1.7 the `proxy` feature pulls the full H1 compatibility slice, while
Eggsec separately enables H2. Therefore this consumer should not be rewritten
onto a lean standard-only recipe as part of this pass. Record the resolved
feature graph and footprint after the version bump; change features only if
the measured graph proves an equivalent narrower profile still provides all
required advanced/proxy APIs.

### 6. Base64 moves from 0.22 to 0.23 in the upgraded graph

Eggfetch's proxy/basic-auth edge uses `base64 0.23` in the new release.
Eggsec's workspace still declares `base64 = "0.22"`, with direct consumers in
the engine/NSE/web-proxy/TUI surfaces.

Prefer one Base64 line if a focused source/build check proves those consumers
are compatible with 0.23. This is a small dependency-graph cleanup, not a
prerequisite for the transport correctness migration. If a consumer requires
0.22 behavior, retain the duplicate temporarily and record it rather than
mixing an unrelated refactor into the transport landing.

## Confirmed Eggsec state at the planning baseline

### 1. The manifest range admits newer 0.1.x Eggfetch, but the lockfile is
still 0.1.5

`crates/eggsec-transport-eggfetch/Cargo.toml` currently declares:

```toml
eggfetch-core = {
    version = "0.1.5",
    default-features = false,
    features = ["http1", "http2", "tls-rustls", "proxy"]
}
```

and `Cargo.lock` currently resolves `eggfetch-core 0.1.5`.

The implementation must raise the documented minimum to 0.1.7 and commit the
new registry lock resolution. Do not use a Git dependency.

### 2. Proxy adoption and strict scope propagation are already complete

The executed 2026-09-16 corrective pass and 2026-09-17 socket-binding
corrective already establish:

- no wildcard fallback scope;
- one enforcement-scope snapshot through production load-test execution;
- explicit proxy-peer authorization checkpoints;
- Eggfetch as the production load-test transport;
- published Eggfetch proxy-peer and proxied-target pinning;
- singular selected `SocketAddr` per proxy leg;
- CONNECT and local-resolution SOCKS5 support;
- SOCKS5H and plaintext forward-proxy fail-closed behavior;
- Rust 1.89 workspace MSRV.

Do not redesign these items in this plan.

### 3. The direct Eggfetch route still uses the pre-0.1.7 compatibility shim

For a direct hostname hop the adapter currently:

1. resolves through the Eggsec resolver;
2. authorizes the DNS snapshot;
3. selects and socket-authorizes one address;
4. rewrites the request URL host to that IP literal with `pin_wire_url()`;
5. manually writes the logical `Host` header;
6. supplies the logical hostname as an SNI override;
7. dispatches through Eggfetch with `resolved_target: None`.

This is physically safe, but it duplicates URL/Host/SNI mechanics now owned by
Eggfetch's resolved-target route and does not use the 0.1.7 route cache under
the logical origin.

The proxied route already uses the preferred shape: logical URL on the request,
explicit singular physical pins below it.

### 4. The adapter's singular-pin rule must survive the resolved-route
migration

The 2026-09-17 correction deliberately changed proxy routing from "all
DNS-approved addresses" to exactly the socket-authorized selected address.

The same rule applies to the direct resolved route. Do **not** pass the whole
DNS-approved vector into `resolved_addresses()`. One authorization cycle
selects one physical socket. A later attempt at another candidate requires a
new Eggsec authorization cycle above Eggfetch.

## Security and correctness invariants

Every workstream must preserve all of these:

1. A production direct hostname request may dial only the exact
   `SocketAddr` that passed Eggsec's selected-socket authorization.
2. The full DNS-approved vector is selection input, not backend failover
   permission.
3. A direct resolved request must never fall back to system/origin DNS.
4. The logical URL remains authoritative for HTTP Host and TLS
   SNI/certificate identity.
5. Different logical origins may not reuse one resolved-route connection even
   when they map to the same physical socket.
6. A change in selected physical socket may not reuse the prior resolved-route
   connection.
7. Manual redirect handling remains above Eggfetch so each redirect target is
   separately authorized before network I/O.
8. No physical pin may be reused across a cross-origin redirect without a new
   resolution/authorization/binding cycle.
9. The original Eggsec request timeout is one aggregate budget across the
   manual redirect chain; each hop receives only the remaining budget, and the
   final hop's budget must extend through body EOF/trailers.
10. Proxy endpoint and ultimate destination remain independent authorization
    and binding decisions.
11. Proxied routes continue to use one socket-authorized proxy peer and one
    socket-authorized ultimate peer.
12. SOCKS5H and plaintext HTTP forward-proxy routes remain fail closed where
    Eggsec cannot constrain the physical ultimate peer.
13. Environment proxy variables never influence this backend.
14. Backend construction/dispatch errors never fall back to a weaker direct,
    DNS-resolved, unpinned, or differently verified route.
15. Eggfetch retries remain disabled for this adapter; retries may not select
    a second physical address without a fresh Eggsec authorization cycle.
16. HTTP/3 remains disabled until Eggsec has an authorized QUIC
    resolution/binding model.
17. `eggsec-transport` remains backend-neutral; no Eggfetch types or feature
    concepts move into the neutral contract merely to ease this migration.
18. No new workspace crate is introduced.

## Workstream 0 — Freeze the baseline and add discriminating regressions first

Record:

```text
Eggsec baseline SHA
eggfetch-core lock version/checksum
eggfetch-core feature graph
duplicate dependency graph
existing direct/proxy route fixtures
existing architecture guard numbers covering Eggfetch production use and
singular socket binding
```

Before production changes, add or adapt deterministic loopback regressions that
distinguish the old and desired behavior.

### A. Response-body total deadline

Use a local server that sends response headers before the configured
`request_timeout`, then delays body EOF beyond that total.

Required proof:

- current 0.1.5 baseline behavior is recorded truthfully (expected to reproduce
  the upstream pre-0.1.7 gap);
- 0.1.7 returns a total-timeout failure within the request budget rather than
  waiting for the delayed body EOF.

Also add a continuous-trickle variant if the existing fixture makes it cheap:
regular chunks must not restart `Timeout.total`.

### B. Manual redirect remaining budget

Build a two-hop local redirect fixture:

1. hop 1 consumes a controlled fraction of the Eggsec total budget;
2. hop 2 returns headers quickly and stalls the final body.

The second hop must receive only the original remaining budget. A fresh full
timeout per hop must fail the discrimination window.

### C. Direct connection-reuse baseline

Record accepted TCP connection counts for repeated requests to one logical
origin and one selected physical socket through the current pinned-wire path.
Do not assume the exact count in advance; preserve it as baseline evidence.

The post-migration fixture must prove physical reuse, not only cache-helper
calls.

## Workstream 1 — Raise and freeze the published dependency

Update `crates/eggsec-transport-eggfetch/Cargo.toml` to require
`eggfetch-core 0.1.7` as the minimum published version while retaining the
current explicit feature selection initially:

```toml
eggfetch-core = {
    version = "0.1.7",
    default-features = false,
    features = ["http1", "http2", "tls-rustls", "proxy"]
}
```

Then update the lockfile to the crates.io release.

Record in the completion record:

```text
eggfetch-core version
eggfetch-core registry checksum
eggfetch-http-connect transitive version/checksum
resolved Eggfetch feature graph
Rust MSRV
```

Run `cargo tree -p eggsec-transport-eggfetch -e features` after the update and
confirm:

- advanced routing is compiled;
- resolved-address APIs are present;
- proxy pinning APIs are present;
- H1/H2 are both enabled;
- HTTP/3 is absent;
- no environment-proxy behavior is activated implicitly.

Do not use `git = ...`, a branch dependency, or a patch override.

### Conditional Base64 graph cleanup

Run `cargo tree -d` after the Eggfetch update.

If Base64 0.22 + 0.23 are both present because of Eggsec's own direct
declarations, test a workspace/direct TUI bump to 0.23. Accept it only if all
direct Base64 consumers compile and their focused tests pass without semantic
changes. Otherwise leave the duplicate and record the blocker.

Do not make Base64 deduplication a release blocker for the 0.1.7 timeout/route
correction.

## Workstream 2 — Replace the direct IP-literal URL shim with
`resolved_addresses()`

Refactor only the direct route first.

For a non-proxied authorized hop, carry:

```text
logical_url: Url
selected_target: SocketAddr
logical_host / connection metadata as required by Eggsec response reporting
```

rather than an IP-rewritten wire URL.

Dispatch conceptually as:

```rust
client
    .request(method, logical_url.as_str())
    ...
    .without_proxy()
    .resolved_addresses([selected_target])
    .send()
    .await
```

The exact fluent ordering may follow Eggfetch's API.

### Required ownership after migration

Eggsec still owns:

- DNS fact collection;
- DNS-set authorization;
- selection of one approved candidate;
- selected-socket authorization;
- redirect authorization;
- proxy policy;
- timeout budget across manual hops.

Eggfetch owns:

- dialing exactly the supplied resolved target;
- logical Host generation from the logical URL;
- logical TLS SNI/certificate identity;
- H1/H2 physical connection reuse inside the route-keyed Hyper client.

### Host-header transition

The adapter currently strips caller `Host` and installs its own normalized
logical Host. Preserve the anti-smuggling invariant.

Use a two-step approach:

1. first switch from IP-literal URL to logical URL +
   `resolved_addresses([selected_target])` while retaining the adapter-owned
   Host header, minimizing simultaneous semantic changes;
2. only remove `host_header_value()` / explicit Host insertion if fixtures
   prove Eggfetch/Hyper produces exactly the contract-compatible value for:
   default ports, non-default ports, IPv4, IPv6 literals, and redirects, and
   caller-supplied Host still cannot bypass logical URL ownership.

If there is no meaningful maintenance win after those checks, retaining the
explicit Host normalization is acceptable. Removing `pin_wire_url()` is the
important simplification.

### SNI transition

With the logical URL restored, ordinary HTTPS direct routing should not need a
redundant `sni_hostname = logical_host` transport hint. Prefer the logical URL
as the sole TLS identity source.

Before removing the explicit SNI hint, prove:

- hostname certificate validation succeeds against the logical host while the
  TCP peer is the supplied IP;
- a certificate valid only for the IP does not accidentally satisfy a logical
  hostname request;
- the TLS fixture observes the logical hostname SNI;
- Eggsec's existing mismatched SNI/Host override checks still fail before
  dispatch.

Do not add a new free-form SNI capability to Eggsec as part of this pass.

### Literal-IP routes

A logical IP-literal URL may use the same resolved-route path with
`selected_target` equal to that literal/port if doing so simplifies the
adapter and preserves TLS semantics. Otherwise retain the simpler literal
direct path. Whichever shape is chosen must be covered explicitly and must not
create a second hidden DNS path.

### No fallback

If `resolved_addresses()` construction/dispatch rejects the route, return the
mapped transport error. Do not retry through the old pinned-wire URL or normal
DNS route.

## Workstream 3 — Adopt 0.1.7 total-deadline body semantics

Retain the adapter's outer request start time:

```text
start = Instant::now()
remaining = request_timeout - elapsed
```

because Eggsec's manual redirect loop consists of multiple Eggfetch requests.

Continue to map each hop to:

```text
Timeout.total = remaining original Eggsec budget
Timeout.connect = configured Eggsec connect timeout
```

Do not create a new full total timeout after a redirect.

Add/retain regressions for:

1. headers arrive before total, body EOF after total -> total timeout;
2. first body chunk arrives before total, later body stalls -> total timeout;
3. continuous body trickle cannot extend aggregate total;
4. redirect hop consumes budget, final body times out on the remainder;
5. timeout terminalization does not poison the cached client for a subsequent
   authorized request;
6. where practical, a proxied HTTPS response body is subject to the same
   aggregate total semantics.

Keep the neutral Eggsec error contract stable unless a separate caller already
requires typed timeout phases. Do not leak Eggfetch error types through
`eggsec-transport`.

## Workstream 4 — Qualify resolved-route reuse and isolation

The migration is justified partly by allowing Hyper to reuse physically scoped
connections without weakening route identity. Add physical loopback evidence.

### H1

For one `EggfetchTransport`, one logical origin, and the same selected
`SocketAddr`:

- multiple sequential requests should reuse a keep-alive TCP connection when
  the fixture allows it;
- record accepted TCP connections.

### H2

For one logical origin + selected socket:

- concurrent requests should use the retained route client and multiplex where
  the fixture supports H2;
- record accepted TCP connections/streams.

### Isolation matrix

Prove no false reuse for:

| Change | Must reuse old physical route? |
|---|---:|
| same origin + same selected socket | allowed |
| path/query only | allowed |
| different selected socket | no |
| same physical socket, different logical origin | no |
| HTTP vs HTTPS | no |
| redirect to another origin | no, new authorization + route |
| direct vs proxied route | no |

Eggsec normally should not supply a distinct SNI override after Workstream 2;
if an override remains for compatibility, add SNI identity to the isolation
matrix as upstream does.

Do not introspect or duplicate Eggfetch's route cache in Eggsec. Physical
accept-count and destination fixtures are the evidence.

## Workstream 5 — Re-run the qualified proxy matrix without redesign

The version bump changes shared Eggfetch core behavior, so the existing proxy
security matrix must be rerun even though the proxy architecture is not
changing.

At minimum prove:

- HTTPS CONNECT: one authorized proxy peer + one authorized ultimate target;
- local-resolution SOCKS5: one authorized proxy peer + one authorized ultimate
  target;
- SOCKS5H: fail closed before target dispatch;
- plaintext HTTP forward proxy with requested ultimate pin: fail closed;
- proxy credential A cannot bleed into credential B;
- changing either singular pin cannot reuse the old route/client;
- no origin/proxy DNS fallback is introduced;
- direct requests still force `.without_proxy()`.

Do not expand the pin sets to multiple addresses to improve availability.
Resilience across candidates remains an Eggsec reauthorization concern.

## Workstream 6 — Explicit cumulative-0.1.6 policy dispositions

### Redirect downgrade

Keep current Eggsec behavior. Do not silently switch to
`RedirectDowngradePolicy::Deny`.

If implementation wants to use
`build_redirect_request_with_redirect_policy()` for API clarity, pass the
compatibility-equivalent Allow behavior and prove no fixture difference. An
actual downgrade-denial policy belongs in a separate neutral
`eggsec-transport` contract change with recording-fake parity.

### Environment proxies

Do not instantiate `ProxyEnvironment` or call
`ClientBuilder::proxy_environment()`.

Add a deterministic test/guard showing hostile `HTTP_PROXY`,
`HTTPS_PROXY`, or `ALL_PROXY` environment values cannot divert an Eggsec
direct request. Prefer process-isolated tests if environment mutation could
race the test suite.

### Lean feature recipes

Do not switch to `standard-http1/2` during this pass unless Cargo feature
resolution proves an equivalent profile can still provide:

- advanced `resolved_addresses()`;
- explicit pinned proxy routes;
- H2;
- TLS.

Current upstream feature ownership makes that unlikely while `proxy` pulls
the full H1 compatibility slice. Record the graph rather than optimizing by
assumption.

## Workstream 7 — Performance and footprint qualification

Use the existing deterministic load-test/transport fixture rather than adding
a new benchmark framework.

Compare at representative concurrency `1, 10, 50, 100` where stable:

```text
backend/version
route shape (old pinned-wire vs 0.1.7 resolved target)
protocol
request count
concurrency
RPS
p50 / p95 / p99
accepted TCP connections
CPU/memory signal if already available
```

Acceptance is:

- materially reduced connection churn where the old pinned-wire route prevented
  reuse;
- no meaningful throughput/latency regression attributable to the migration;
- no performance win obtained by weakening physical pinning or authorization.

Also record:

```text
cargo tree -p eggsec-transport-eggfetch -e features
cargo tree -d
release artifact/binary size if the repository already has a stable comparison
method
```

Do not claim the new Eggfetch lean profiles reduce Eggsec's footprint unless
the actual Eggsec feature graph and linked artifact show it.

## Workstream 8 — Documentation, dependency truth, and architecture guards

Update the smallest truthful set of documentation after behavior is green.

At minimum reconcile:

- `AGENTS.md`: adapter dependency is no longer published 0.1.5;
- `crates/eggsec-transport-eggfetch/Cargo.toml`: minimum version/comment;
- `architecture/loadtest.md`: direct route now uses logical URL +
  singular `resolved_addresses`, and 0.1.7 total covers body EOF;
- adapter module/type comments that still say there are no production
  consumers;
- dependency/closure docs that name the current Eggfetch release where they are
  normative rather than historical;
- this plan's completion record;
- `plans/README.md`.

Tighten the existing architecture guards instead of adding redundant checks
where possible. Mechanically protect at least:

1. production direct Eggfetch route uses a singular selected
   `resolved_addresses` target;
2. the direct adapter does not reintroduce `pin_wire_url` after it is removed;
3. no `ProxyEnvironment` / environment-proxy integration enters the scoped
   adapter;
4. HTTP/3 remains disabled;
5. SOCKS5H/plain forward-proxy strict cases remain fail closed;
6. singular proxy/ultimate pins remain enforced;
7. workspace MSRV remains at least the upstream requirement (1.89).

Prefer behavioral Rust tests over grep guards; use grep only for simple
forbidden wiring.

## Required focused tests

### Direct route

- authorized hostname resolves to multiple candidates but only the singular
  selected/socket-authorized target is passed to Eggfetch;
- logical Host is preserved;
- logical TLS SNI/certificate validation is preserved;
- no system DNS fallback;
- selected-target change cannot use old connection;
- logical-origin change cannot use old connection;
- H1 same-route keep-alive reuse;
- H2 same-route multiplex/reuse;
- literal IPv4/IPv6 behavior is explicit.

### Redirect

- same-host redirect re-runs Eggsec resolution/socket authorization;
- cross-host redirect requires `authorize_redirect` plus full next-hop
  authorization;
- old resolved target never crosses origins;
- current downgrade behavior remains unchanged;
- caller-supplied Host cannot survive as an unauthorized routing identity.

### Timeout

- slow final body exceeds total -> total timeout;
- post-first-chunk stall -> total timeout;
- trickle cannot reset total;
- redirect final body sees remaining original budget;
- subsequent request remains usable after timeout.

### Proxy

- existing CONNECT/local-SOCKS5 pin tests stay green;
- SOCKS5H/plain-forward failures stay green;
- credential isolation stays green;
- hostile proxy environment does not influence direct or explicit-proxy
  routing.

## Required verification

Run focused checks during implementation:

```sh
cargo fmt --all -- --check
cargo check -p eggsec-transport
cargo test -p eggsec-transport -- --test-threads=1
cargo check -p eggsec-transport-eggfetch
cargo test -p eggsec-transport-eggfetch -- --test-threads=1
cargo test -p eggsec --lib loadtest -- --test-threads=1
cargo test -p eggsec --test network_policy_invariants -- --test-threads=1
cargo test -p eggsec --test enforced_dispatch_regression -- --test-threads=1
cargo tree -p eggsec-transport-eggfetch -e features
cargo tree -d
```

MSRV:

```sh
make check-msrv
```

Repository mandatory contract:

```sh
make check
make check-deps
```

Because this changes a shared network dependency and its feature graph, run the
deep feature gates before closure as well:

```sh
make check-full
make check-features-individual
```

Run `make check-python` only if Python bindings/stubs/docs/scripts change.

Before release-oriented closure, run the repository's local release validation:

```sh
make release-check
```

Record hosted CI run IDs/conclusions for the final implementation SHA. Do not
treat upstream Eggfetch qualification as a substitute for Eggsec adapter
qualification.

## Expected files touched

Likely:

```text
Cargo.lock
Cargo.toml                                      # only if Base64 workspace bump
crates/eggsec-transport-eggfetch/Cargo.toml
crates/eggsec-transport-eggfetch/src/adapter.rs
crates/eggsec-transport-eggfetch/src/mapping.rs
crates/eggsec-transport-eggfetch/tests/parity.rs
crates/eggsec-tui/Cargo.toml                    # only if Base64 dedupe lands
architecture/loadtest.md
scripts/check-architecture-guards.sh            # only for durable wiring guards
AGENTS.md
plans/README.md
this plan completion record
```

Other Base64 consumer manifests may change only if the workspace bump requires
explicit reconciliation. Do not mechanically touch historical plans that
truthfully describe 0.1.5 at their execution time.

## Non-goals

- no new workspace crate;
- no reopening approval-scope propagation;
- no redesign of `NetworkAuthority`;
- no multi-address backend failover;
- no environment-derived proxy routing;
- no automatic direct fallback from a failed proxy;
- no HTTPS -> HTTP policy change without a separate neutral-contract decision;
- no HTTP/3 enablement;
- no retry enablement;
- no new Eggfetch upstream API;
- no second physical connection pool in Eggsec;
- no replacement of the manual authorization redirect loop;
- no broad migration of unrelated Reqwest owners/NSE compatibility surfaces;
- no package publication as part of implementation.

## Acceptance criteria

1. `Cargo.lock` resolves published `eggfetch-core 0.1.7`; the adapter
   manifest names 0.1.7 as its minimum required release.
2. No Git/branch/patch override is used.
3. Rust 1.89 remains truthful for the upgraded dependency graph.
4. Direct production hostname routing uses the logical URL plus exactly one
   socket-authorized `resolved_addresses` target.
5. The old IP-literal URL pinning helper is removed from production direct
   routing, with no weaker fallback.
6. Logical Host and TLS SNI/certificate identity are unchanged.
7. Direct resolved routes perform no origin DNS lookup inside Eggfetch.
8. H1 same-route physical reuse is proven.
9. H2 retained-client reuse/multiplexing is proven.
10. Changed origin or physical target cannot cross-reuse a connection.
11. `Timeout.total` is proven to cover final response-body EOF/trailers
    through Eggsec's `response.bytes()` path.
12. Manual redirects share one aggregate Eggsec total budget rather than
    restarting it per hop.
13. Existing proxy singular-pin/credential/fail-closed tests remain green.
14. Environment proxy variables cannot influence the scoped adapter.
15. Redirect downgrade behavior is unchanged unless a separate
    `eggsec-transport` policy change is explicitly approved.
16. HTTP/3 and automatic Eggfetch retries remain disabled.
17. Feature/dependency graph is recorded; any Base64 0.22/0.23 duplication is
    either cleanly removed or explicitly documented.
18. Mandatory, MSRV, deep-feature, release-check, and hosted CI gates are green
    on the final implementation SHA.
19. No new crate or policy/backend coupling is introduced.

## Handoff order

Implement in this order:

1. Workstream 0 baseline/discriminating fixtures.
2. Workstream 1 dependency + lock update and feature graph capture.
3. Workstream 3 timeout regressions against 0.1.7 (establish correctness
   before route simplification).
4. Workstream 2 direct `resolved_addresses` migration.
5. Workstream 4 H1/H2 reuse and isolation qualification.
6. Workstream 5 proxy regression matrix.
7. Workstream 6 cumulative-0.1.6 policy checks.
8. Workstream 7 performance/footprint measurement.
9. Workstream 8 docs/guards.
10. Full local + hosted verification and completion record.

If the version bump itself exposes an API/feature incompatibility, stop at
Workstream 1 and record the exact upstream mismatch. Do not recover by floating
to Eggfetch `main`.

## Completion record template

Append after implementation:

```text
Eggsec planning baseline:
Final implementation SHA:
Hosted CI run(s):

eggfetch-core:
  version:
  crates.io checksum:
  release/tag:
eggfetch-http-connect:
  version:
  crates.io checksum:
Eggfetch feature graph:
MSRV:

Direct route before:
Direct route after:
Selected SocketAddr proof:
Origin DNS non-fallback proof:
Logical Host proof:
Logical SNI/certificate proof:
H1 accepted-connection before/after:
H2 connection/stream proof:
Route-isolation matrix:

Total-deadline baseline result:
Headers-fast/body-slow result:
Post-first-chunk stall:
Trickle aggregate total:
Redirect remaining-budget result:
Post-timeout reuse result:

CONNECT proxy:
SOCKS5 local:
SOCKS5H:
Plain forward proxy:
Credential isolation:
Hostile proxy environment:
Redirect downgrade behavior:

Base64 graph before:
Base64 graph after / retained duplicate rationale:
Artifact/footprint measurement:
Load-test 1/10/50/100 measurements:

Focused tests:
make check:
make check-deps:
make check-msrv:
make check-full:
make check-features-individual:
make release-check:
make check-python (if applicable):

Architecture/docs updates:
Residual debt:
Unsupported route shapes:
```

## Exit criterion

This line is complete when Eggsec consumes the published 0.1.7 crate, direct
scoped traffic is expressed as logical URL + singular authorized resolved
target, the adapter demonstrably benefits from the upstream H1/H2 route reuse,
the original request timeout is enforced through body EOF across manual
redirects, and the existing proxy/scope security invariants remain green with
no environment-proxy, H3, retry, or multi-address failover expansion.
