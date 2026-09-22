# Eggfetch 0.2.0 adoption and requalification

Status: Executed

Date: 2026-09-22

Eggsec planning baseline: `e4cd12becd486316eeb7876e4272507fb7deb042`

Upstream release baseline:

- published crate: `eggfetch-core 0.2.0` on crates.io;
- coordinated Eggfetch release commit:
  `8959ca890ee34f4cf456aed648315322f1e83ef7`;
- release tag: `v0.2.0`;
- MSRV remains Rust 1.89;
- upstream changelog states no intentional breaking changes from 0.1.7 in
  public Rust/Python/C/CLI/HTTPX APIs, feature graph/defaults, MSRV, or
  dependency policy;
- the release includes the issue-24 streaming decompression correction and
  the API-preserving private architecture/performance maintenance accumulated
  after 0.1.7.

Parent Eggsec transport work:

- [eggfetch-0.1.7-adoption-and-direct-route-simplification-2026-09-18.md](eggfetch-0.1.7-adoption-and-direct-route-simplification-2026-09-18.md)
- [eggfetch-0.1.7-post-adoption-qualification-corrective-pass-2026-09-19.md](eggfetch-0.1.7-post-adoption-qualification-corrective-pass-2026-09-19.md)
- [eggfetch-0.1.7-final-qualification-deep-check-corrective-pass-2026-09-19.md](eggfetch-0.1.7-final-qualification-deep-check-corrective-pass-2026-09-19.md)

This plan consumes the new published release. It does not reopen the completed
Eggfetch transport architecture.

## Purpose

Move Eggsec's production scoped HTTP backend from the currently required
`eggfetch-core 0.1.7` release line to the newly published `0.2.0` line and
requalify the already-established transport invariants against that release.

The intended change is deliberately small:

```toml
eggfetch-core = {
    version = "0.2.0",
    default-features = false,
    features = ["http1", "http2", "tls-rustls", "proxy"],
}
```

Cargo will not select 0.2.0 from the existing `version = "0.1.7"`
requirement because pre-1.0 compatibility treats the 0.1.x and 0.2.x lines as
distinct compatibility ranges. The manifest therefore requires an explicit
version bump.

The expected end state is the same transport architecture that is already
qualified on 0.1.7:

```text
Eggsec resolver
  -> NetworkAuthority DNS authorization
  -> singular selected socket authorization
  -> logical URL remains authoritative
  -> eggfetch resolved_addresses([exact selected SocketAddr])
  -> H1/H2 pooled transport
```

For supported proxied routes:

```text
proxy endpoint
  -> independent proxy DNS/socket authorization
  -> Proxy::resolved_addresses([exact authorized proxy peer])

ultimate target
  -> independent target DNS/socket authorization
  -> proxy_target_addresses([exact authorized ultimate target])
```

No second resolver, transport pool, policy language, automatic redirect loop,
or fallback path should be introduced.

## Confirmed baseline findings

At the planning baseline:

1. `eggsec-transport-eggfetch` is the sole Eggfetch adapter crate and the
   production load-test backend.
2. Its manifest requires:
   `eggfetch-core = { version = "0.1.7", default-features = false, features =
   ["http1", "http2", "tls-rustls", "proxy"] }`.
3. No Reqwest dependency or Reqwest production fallback remains in the scoped
   adapter.
4. Direct hostname routes retain the logical URL and pass exactly one
   socket-authorized address through
   `RequestBuilder::resolved_addresses([selected_target])`.
5. Supported proxy routes independently pin the proxy peer and ultimate target.
6. Redirect following is owned manually by Eggsec so every hop passes
   `NetworkAuthority` before I/O.
7. `Timeout.total` is mapped from the remaining Eggsec aggregate request
   budget and is qualified through response-body EOF.
8. Backend retries, automatic decompression, environment-proxy discovery, and
   HTTP/3 are disabled.
9. The adapter has dedicated parity, H2, and local-resolution SOCKS5
   qualification suites.
10. Architecture guard Check 134 currently requires the literal
    `version = "0.1.7"`, so adoption will fail repository guards until that
    check is intentionally advanced.
11. Several comments/docs still identify 0.1.7 as the currently consumed
    release. Historical statements about what 0.1.7 introduced must remain
    historical; only current-state wording should advance to 0.2.0.
12. Eggfetch 0.2.0's issue-24 decompression correction does not justify
    changing Eggsec body semantics: Eggsec currently disables automatic
    decompression and does not enable compression codec features.

## Mandatory invariants

This adoption must preserve all of the following:

1. public Rust/Python/CLI/API behavior;
2. `eggsec-transport::HttpTransport` and `NetworkAuthority` contracts;
3. logical URL ownership of Host/TLS identity;
4. singular physical address authorization per connection leg;
5. no backend DNS fallback after socket authorization;
6. manual redirect authorization before next-hop I/O;
7. fresh resolution/authorization on redirect/retry authorization cycles;
8. aggregate request timeout through final response-body EOF;
9. explicit proxy intent only;
10. independent proxy-peer and ultimate-target policy checkpoints;
11. fail-closed SOCKS5H remote-DNS handling;
12. fail-closed plaintext HTTP forward-proxy ultimate pinning;
13. no automatic backend retries;
14. no environment-derived proxy routing;
15. HTTP/3 disabled;
16. response bytes returned verbatim with automatic decompression disabled;
17. current TLS verification/insecure-TLS opt-in semantics;
18. H1/H2 pooling and route-key isolation;
19. workspace MSRV 1.89;
20. no Git/path/patch override to an unpublished Eggfetch revision.

If any one of these requires a semantic workaround after the bump, stop and
record the incompatibility rather than weakening the invariant.

## Workstream 0 — Freeze the pre-adoption state

Before changing the dependency:

1. Record the starting Eggsec SHA and clean/dirty state.
2. Record:
   - `cargo tree -p eggsec-transport-eggfetch -e features`;
   - `cargo tree -p eggsec-transport-eggfetch`;
   - `cargo tree -d`;
   - current `eggfetch-core` and `eggfetch-http-connect` entries/checksums
     from `Cargo.lock`.
3. Run at minimum:
   - `cargo check -p eggsec-transport-eggfetch`;
   - `cargo test -p eggsec-transport-eggfetch -- --test-threads=1`;
   - `cargo test -p eggsec --features rest-api --test transport_eggfetch_parity -- --test-threads=1`;
   - `bash scripts/check-architecture-guards.sh`.
4. Record any pre-existing failure before changing the lockfile.

Do not use a failed pre-adoption baseline as evidence of a 0.2.0 regression.

## Workstream 1 — Adopt the published 0.2.0 release

Update only the adapter dependency requirement:

```toml
eggfetch-core = { version = "0.2.0", default-features = false, features = ["http1", "http2", "tls-rustls", "proxy"] }
```

Then update the lockfile through Cargo so the selected published graph contains
`eggfetch-core 0.2.0` and the matching published
`eggfetch-http-connect 0.2.0`.

Requirements:

- no Git dependency;
- no branch dependency;
- no `[patch]` override;
- no local sibling path override;
- no wildcard version;
- no broad unrelated `cargo update` churn if a targeted update is possible.

Record the final crates.io checksums from `Cargo.lock`.

If Cargo selects a dependency graph different from the upstream 0.2.0
manifest's published feature relationships, investigate before proceeding.

## Workstream 2 — Compile/API/feature-surface qualification

First attempt the dependency bump with no production Rust source changes.

The adapter's current 0.1.7 API usage must compile unchanged:

- `Client` / `Client::builder`;
- `HttpVersionPolicy::Auto { allow_http3: false }`;
- `TlsConfig` / `TrustStore`;
- `RequestBuilder::resolved_addresses`;
- `Proxy::resolved_addresses`;
- `RequestBuilder::proxy_target_addresses`;
- `TransportHints`;
- `Timeout`;
- `RedirectPolicy`;
- public redirect primitives used by the manual loop;
- `without_proxy`, `without_retry`, and `decompress(false)`.

If source edits are needed merely to compile, classify each as:

1. mechanical API adaptation with identical semantics;
2. behaviorally meaningful adaptation; or
3. evidence of an upstream regression.

Category 2 or 3 requires stopping the narrow adoption pass and writing a
corrective plan before changing transport semantics.

Capture the post-bump:

- `cargo tree -p eggsec-transport-eggfetch -e features`;
- `cargo tree -p eggsec-transport-eggfetch`;
- `cargo tree -d`.

Compare against Workstream 0. Confirm in particular that the adapter still does
not enable:

- `http3`;
- `cookies`;
- compression codecs;
- `multipart`.

The `proxy` feature's existing implication of the full H1/high-level policy
slice is accepted historical behavior; do not redesign feature ownership in
this pass unless 0.2.0 unexpectedly changes the resolved graph.

## Workstream 3 — Requalify direct-route and TLS identity invariants

Run the existing direct-route qualification unchanged.

Required evidence:

1. logical hostname request reaches a non-system-resolvable fixture through
   only the authorized resolved pin;
2. the backend cannot fall through to a second DNS-approved but
   socket-unauthorized address;
3. caller-supplied `Host` cannot replace the adapter-owned logical Host;
4. logical Host survives direct requests and redirects;
5. verified TLS still rejects untrusted/self-signed material;
6. hostname/certificate mismatch remains rejected;
7. insecure TLS remains explicit request policy, not fallback;
8. logical SNI/certificate identity remains the logical hostname;
9. IPv4/IPv6 literals preserve the qualified behavior;
10. same direct route reuses an H1 connection;
11. H2 negotiates through ALPN, multiplexes/reuses one route, and isolates a
    changed physical pin;
12. changed logical origin cannot reuse another origin's connection.

Do not remove the explicit SNI hint or Host ownership as part of the version
bump. Those are separately qualified adapter choices, not dependency-version
cleanup.

## Workstream 4 — Requalify redirects, timeouts, and response-body semantics

Run the existing redirect matrix and timeout-through-EOF matrix.

Redirect requirements:

- automatic Eggfetch redirects remain disabled globally and per request;
- `authorize_redirect(from, to)` still executes before next-hop dispatch;
- every followed redirect performs the full Eggsec host/DNS/socket/TLS/proxy
  authorization sequence;
- same-origin credential behavior remains unchanged;
- cross-origin sensitive headers remain stripped;
- method/body transformations for 301/302/303/307/308 remain unchanged;
- redirect cap behavior remains unchanged;
- HTTPS -> HTTP behavior remains the existing compatibility disposition unless
  a separate neutral transport-policy change is approved.

Timeout requirements:

- headers-fast/body-slow exceeds the aggregate total deadline;
- post-first-chunk stall cannot reset the total deadline;
- continuous trickle cannot extend the aggregate total deadline;
- redirect final body receives only the remaining original budget;
- a timed-out request does not poison subsequent connection/client reuse.

Eggfetch 0.2.0's issue-24 decompression fix is not a reason to enable
decompression. Retain `.automatic_decompression(false)`,
`.decompress(false)`, and the current Cargo feature exclusions. Add no
compression dependency or local decompressor in this pass.

## Workstream 5 — Requalify proxy route pinning and fail-closed behavior

Run the existing CONNECT and SOCKS5-local suites.

Required evidence:

1. authorized HTTPS CONNECT succeeds;
2. only the exact authorized proxy peer may be dialed;
3. only the exact authorized ultimate target may be requested through the
   proxy;
4. local-resolution SOCKS5 sends the approved IP/port to the proxy rather than
   allowing remote hostname resolution;
5. SOCKS5 proxy-peer fallback is forbidden;
6. SOCKS5 ultimate-target fallback is forbidden;
7. SOCKS5H remains rejected before dispatch;
8. plaintext HTTP forward-proxy routes that cannot enforce the ultimate pin
   remain rejected before dispatch;
9. proxy credentials remain request/route isolated and redacted;
10. hostile `HTTP_PROXY`/`HTTPS_PROXY`/`ALL_PROXY`/lowercase variants
    cannot divert a direct request;
11. no implicit direct fallback occurs after a proxy error.

Upstream 0.2.0 contains internal proxy decomposition and a SOCKS multi-address
terminal-error correction. Eggsec intentionally supplies singular authorized
addresses, so no new multi-address backend failover is authorized here.

## Workstream 6 — Update durable guards and current-state documentation

Update architecture guards narrowly:

- Check 134 must require the published `eggfetch-core 0.2.0` line while
  retaining the Rust 1.89 MSRV check.
- Update stale guard comments that describe 0.1.7 as the current release where
  doing so improves accuracy.
- Preserve Check 135 singular proxy-leg enforcement.
- Preserve Check 136 logical-URL + singular `resolved_addresses` enforcement.
- Preserve Check 137 no-environment-proxy / HTTP3-off enforcement.
- Do not weaken Check 102's feature allowlist or sole-consumer rule.

Update current-state documentation, especially:

- `crates/eggsec-transport-eggfetch/Cargo.toml` comments;
- `crates/eggsec-transport-eggfetch/src/lib.rs`;
- `crates/eggsec-transport-eggfetch/src/adapter.rs` comments if they name the
  currently consumed release;
- `crates/eggsec-transport-eggfetch/src/mapping.rs` timeout version wording;
- `architecture/transport_eggfetch.md`;
- `architecture/loadtest.md` only where it names the active Eggfetch release;
- `plans/README.md`;
- relevant living contributor/verification docs if they encode an exact
  Eggfetch version.

Historical completion records that truthfully state they ran on 0.1.5 or
0.1.7 must not be rewritten to 0.2.0.

Document the release delta accurately:

- 0.2.0 is an explicit pre-1.0 version-line adoption;
- upstream states no intentional public API/feature/MSRV break from 0.1.7;
- issue #24 is fixed upstream but is outside Eggsec's current decompression-off
  production path;
- private upstream refactors/performance work are not claimed as an Eggsec
  performance improvement without Eggsec measurements.

## Workstream 7 — Validation and closure

Run focused transport validation first:

```text
cargo check -p eggsec-transport
cargo test -p eggsec-transport -- --test-threads=1
cargo check -p eggsec-transport-eggfetch
cargo test -p eggsec-transport-eggfetch -- --test-threads=1
cargo test -p eggsec-transport-eggfetch --test h2_mux -- --test-threads=1
cargo test -p eggsec-transport-eggfetch --test socks5_local -- --test-threads=1
cargo test -p eggsec --features rest-api --test transport_eggfetch_parity -- --test-threads=1
cargo test -p eggsec --lib loadtest -- --test-threads=1
cargo test -p eggsec --test network_policy_invariants -- --test-threads=1
cargo test -p eggsec --test enforced_dispatch_regression -- --test-threads=1
bash scripts/check-architecture-guards.sh
```

Then run the repository contract:

```text
cargo fmt --all --check
make check
make check-deps
make check-msrv
make check-python
make check-feature-profiles
```

Run the current deep/release gates according to repository policy:

```text
make check-full
make check-features-individual
make release-check
```

If those are scheduled/hosted-only or require unavailable system prerequisites,
record that fact precisely and require the corresponding hosted jobs before
closure. Do not silently substitute local SKIPs for deep-check evidence.

After push, record hosted CI and Code Quality/Deep Check runs for the final
implementation SHA or a docs-only descendant whose executable tree is
identical.

## Performance and footprint disposition

This adoption does not require a new benchmark framework.

Use the existing load-test/transport qualification machinery for a bounded
sanity measurement if readily available, and record:

- build/profile/toolchain;
- concurrency points used;
- errors/timeouts;
- connection accept/reuse counts where relevant.

Do not claim that 0.2.0 makes Eggsec faster merely because upstream contains
performance work.

A performance investigation is required only if:

- existing bounded measurements show a material regression;
- connection reuse/multiplexing counts regress;
- the dependency graph expands materially; or
- binary/RSS measurements already tracked by Eggsec regress outside normal
  noise.

Any optimization response to such evidence belongs in a separate corrective
plan.

## Expected files touched

Expected:

```text
Cargo.lock
crates/eggsec-transport-eggfetch/Cargo.toml
crates/eggsec-transport-eggfetch/src/lib.rs            # version wording only, if needed
crates/eggsec-transport-eggfetch/src/adapter.rs        # version wording only, if needed
crates/eggsec-transport-eggfetch/src/mapping.rs        # version wording only, if needed
scripts/check-architecture-guards.sh
architecture/transport_eggfetch.md
architecture/loadtest.md                               # only if current-version wording is stale
plans/README.md
this plan completion record
```

Production adapter logic and tests should not need semantic edits. If they do,
the implementation record must explain why.

## Non-goals

- no transport redesign;
- no new workspace crate;
- no Reqwest reintroduction;
- no Eggfetch Git/path pin;
- no new Eggfetch upstream API;
- no resolver redesign;
- no `NetworkAuthority` redesign;
- no multi-address backend failover;
- no automatic redirects;
- no environment proxy discovery;
- no direct fallback from failed proxy routing;
- no HTTP/3;
- no backend retry enablement;
- no automatic decompression enablement;
- no cookie or multipart enablement;
- no change to HTTPS-downgrade policy;
- no SNI-hint cleanup;
- no public API change;
- no benchmark threshold added from noisy loopback data;
- no unrelated dependency updates.

## Acceptance criteria

This adoption is complete only when:

1. `crates/eggsec-transport-eggfetch/Cargo.toml` requires published
   `eggfetch-core 0.2.0`.
2. `Cargo.lock` contains published `eggfetch-core 0.2.0` and matching
   `eggfetch-http-connect 0.2.0` with recorded registry checksums.
3. No Git/path/patch override is used.
4. Rust 1.89 remains the workspace MSRV and `make check-msrv` succeeds.
5. The post-bump Eggfetch feature graph contains the intended
   H1/H2/Rustls/proxy capabilities and no deferred feature expansion.
6. Existing production adapter source compiles without semantic transport
   changes, or any unavoidable mechanical API adaptation is documented.
7. Direct routes remain logical URL + exactly one socket-authorized
   `resolved_addresses` pin.
8. Logical Host and TLS SNI/certificate identity remain unchanged.
9. Direct backend DNS fallback remains impossible after pinning.
10. H1 reuse and H2 ALPN/multiplex/reuse/isolation remain green.
11. Manual redirects remain the only follow path and every next hop is
    authorized before I/O.
12. Same-origin/cross-origin credential behavior remains unchanged.
13. Aggregate total timeout remains enforced through final body EOF and across
    redirects.
14. CONNECT and SOCKS5-local singular two-leg pinning remain green.
15. SOCKS5H and unenforceable plaintext forward-proxy routes remain fail closed.
16. Hostile proxy environment variables cannot affect direct routing.
17. Automatic retries, HTTP/3, environment proxies, and automatic decompression
    remain disabled.
18. Issue #24's upstream fix is documented without changing Eggsec's
    decompression-off contract.
19. Architecture guards are advanced to the 0.2.0 current-state requirement
    without weakening the security assertions.
20. Current-state architecture docs name 0.2.0 while historical 0.1.7 records
    remain historically accurate.
21. Focused transport/load-test/security suites are green.
22. `make check`, `make check-deps`, `make check-msrv`,
    `make check-python`, and applicable feature-profile gates are green.
23. Deep/release/hosted validation required by current repository policy is
    green or any infrastructure-only limitation is recorded explicitly.
24. No new public capability, dependency owner, transport pool, or policy
    bypass is introduced.

## Handoff order

Implement in this order:

1. Workstream 0 pre-adoption evidence.
2. Workstream 1 targeted manifest/lockfile bump.
3. Workstream 2 compile and feature-graph comparison.
4. Workstreams 3-5 existing transport/security qualification.
5. Workstream 6 guards and living documentation.
6. Workstream 7 repository/deep/hosted validation.
7. Append the completion record and mark this plan executed in
   `plans/README.md`.

If the bump fails before Workstream 3 because the published 0.2.0 API or feature
graph is incompatible with Eggsec's qualified adapter, stop and record the
exact mismatch. Do not work around it by floating to Eggfetch `main` or
weakening transport policy.

## Completion record template

Append after implementation:

```text
Status:
Starting Eggsec SHA:
Final implementation SHA:
Final documentation/record SHA:
Hosted CI run(s):
Hosted Deep Checks run(s):

eggfetch-core:
  version:
  crates.io checksum:
  release/tag:
eggfetch-http-connect:
  version:
  crates.io checksum:
Eggfetch feature graph before:
Eggfetch feature graph after:
Dependency duplicate graph before:
Dependency duplicate graph after:
MSRV:

Production source semantic delta:
Direct singular-pin proof:
Origin DNS non-fallback proof:
Logical Host proof:
Logical SNI/certificate proof:
H1 reuse proof:
H2 ALPN/multiplex/reuse/isolation proof:

Redirect matrix:
Same-origin credential behavior:
Cross-origin credential stripping:
HTTPS downgrade disposition:
Total-deadline body EOF:
Post-first-chunk stall:
Trickle aggregate total:
Redirect remaining-budget:
Post-timeout reuse:

CONNECT proxy:
SOCKS5 local:
SOCKS5H:
Plain forward proxy:
Proxy peer fallback:
Ultimate target fallback:
Credential isolation:
Hostile proxy environment:

Automatic decompression:
Issue #24 disposition:
Automatic retry:
HTTP/3:
Environment proxy discovery:

Focused transport checks:
make check:
make check-deps:
make check-msrv:
make check-python:
make check-feature-profiles:
make check-full:
make check-features-individual:
make release-check:

Architecture guards updated:
Living docs updated:
Historical records preserved:
Performance/footprint disposition:
Residual debt:
```

## Exit criterion

This line is closed when Eggsec consumes the published `eggfetch-core 0.2.0`
release, the existing adapter passes its direct/H2/proxy/redirect/timeout
qualification unchanged, all security and policy invariants remain intact,
repository guards and living documentation reflect 0.2.0, and the final
implementation has current local/hosted evidence without introducing a new
transport behavior or maintenance branch.

## Completion record (executed 2026-09-22)

```text
Status: Executed
Starting Eggsec SHA: 1296f6f7cb9d478197aead7bbdca8230dddc2db2 (clean tree)
Final implementation SHA: 17cf8c31
Final documentation/record SHA: 86d2670b7ce487dd1697ef120116bd811a9c5d40
  (record commit; this hosted-evidence amendment is a docs-only descendant
  with an identical executable tree)
Hosted CI run(s): CI 35698884162 success + Code Quality 35698883346 success
  (head 86d2670b, push on main)
Hosted Deep Checks run(s): Deep Checks 35698970796 success (manual
  workflow_dispatch on main @ 86d2670b; sub-jobs incl. MSRV 1.89,
  portability macos/windows, platform integration all success)

eggfetch-core:
  version: 0.2.0
  crates.io checksum: 6cd254b82aa20d1becb6e3e90a5103e8bcc2d7057bb13532096326e43cd98904
  release/tag: v0.2.0 (MSRV 1.89 retained)
eggfetch-http-connect:
  version: 0.2.0
  crates.io checksum: b16f85a1fdbc2deb5c705001a1d8ec4c05934175262a62664e634bd09043fde0
Eggfetch feature graph before: http1 + http2 + tls-rustls + proxy over
  0.1.7 (plus the long-standing implied high-level slice:
  basic-auth/high-level-url/logical-retry/native-http1/advanced-routing/
  standard-route/transport-http1/redirects)
Eggfetch feature graph after: identical slice over 0.2.0; no http3,
  cookies, compression codecs, or multipart (verified via
  `cargo tree -e features`: zero matches)
Dependency duplicate graph before: base64, hyper-util, pem, plist,
  reqwest, wiremock (with eggfetch-core/http-connect 0.1.7)
Dependency duplicate graph after: identical set (with eggfetch-core/
  http-connect 0.2.0); targeted `cargo update -p eggfetch-core` left
  155 unrelated deps untouched
MSRV: 1.89 (`make check-msrv` green)

Production source semantic delta: none. The adapter compiled unchanged
  against 0.2.0 (category 0: no mechanical API adaptation needed for
  Client/builder, HttpVersionPolicy::Auto { allow_http3: false },
  TlsConfig/TrustStore, resolved_addresses, Proxy::resolved_addresses,
  proxy_target_addresses, TransportHints, Timeout, RedirectPolicy,
  redirect primitives, without_proxy/without_retry/decompress(false)).
  Only version-wording comments changed (lib.rs, adapter.rs, mapping.rs)
  plus the manifest requirement 0.1.7 -> 0.2.0.
Direct singular-pin proof: parity suite (52 tests incl. direct singular
  pin forbids fallback, Host ownership, literal handling) + engine
  interop (5 tests) green on 0.2.0; guard Check 136 passes.
Origin DNS non-fallback proof: `test.local` no-system-DNS +
  panicking-resolver literal fixtures green (unchanged suite).
Logical Host proof: adapter-owned Host per hop fixtures green.
Logical SNI/certificate proof: self-signed rejected, wrong-SAN
  rejected, insecure mode explicit opt-in only (unchanged suite).
H1 reuse proof: 5 reqs / 1 accept + target/origin isolation green.
H2 ALPN/multiplex/reuse/isolation proof: h2_mux 5/5 green (warmed-route
  4-concurrent on 1 accept, 5-sequential on 1 accept, selected-address-only
  + logical-origin isolation, ALPN h2, :authority = logical host).

Redirect matrix: automatic Eggfetch redirects still doubly disabled;
  authorize_redirect before next-hop dispatch; full checkpoint sequence
  per followed hop; same-origin credential restore / cross-origin strip;
  301/302/303/307/308 method/body rules; hop caps surface last 3xx;
  HTTPS->HTTP stays compatibility-Allow (all in unchanged parity suite).
Same-origin credential behavior: unchanged (auth preservation fixtures green).
Cross-origin credential stripping: unchanged (stripping fixtures green).
HTTPS downgrade disposition: unchanged (compatibility-Allow, no silent Deny).
Total-deadline body EOF: headers-fast/body-slow + post-first-chunk stall
  + trickle + redirect-remainder + post-timeout-reuse fixtures green.
Post-first-chunk stall: green (cannot reset the total deadline).
Trickle aggregate total: green (cannot extend the aggregate deadline).
Redirect remaining-budget: green (final body gets only the remainder).
Post-timeout reuse: green (no connection/client poisoning).

CONNECT proxy: authorized CONNECT + both pins green.
SOCKS5 local: IP-target success with both pins green (4/4 socks5_local).
SOCKS5H: rejected before dispatch (rerun green in parity + socks5_local).
Plain forward proxy: unenforceable ultimate pin rejected before dispatch.
Proxy peer fallback: forbidden (fixture green).
Ultimate target fallback: forbidden (fixture green).
Credential isolation: proxy credential isolation fixture green.
Hostile proxy environment: HTTP_PROXY/HTTPS_PROXY/ALL_PROXY (+lowercase)
  cannot divert a direct request (fixture green).

Automatic decompression: still off (.automatic_decompression(false) +
  .decompress(false), no compression features); verbatim-bytes fixture green.
Issue #24 disposition: fixed upstream in 0.2.0; outside Eggsec's
  decompression-off production path, so no Eggsec semantic change and no
  decompressor added. Documented, not exercised.
Automatic retry: still disabled (Eggsec owns retry above the transport).
HTTP/3: still disabled (http3 feature off, allow_http3: false; guard 137).
Environment proxy discovery: still disabled (without_proxy on direct hops,
  ProxyEnvironment never constructed; guard 137).

Focused transport checks (all on published 0.2.0):
  eggsec-transport: 18 passed
  eggsec-transport-eggfetch: 67 passed (6 unit + 52 parity + 5 h2 + 4 socks5)
  h2_mux: 5 passed; socks5_local: 4 passed
  engine transport_eggfetch_parity (rest-api): 5 passed
  engine lib loadtest: 37 passed
  network_policy_invariants: 12 passed
  enforced_dispatch_regression: 5 passed
  scripts/check-architecture-guards.sh: ALL PASSED (incl. Check 134 at
    0.2.0, Checks 102/135/136/137 preserved)
make check: green (fmt, no-default checks, cargo deny, clippy, full test
  suites, guards)
make check-deps: green (cargo deny --workspace --all-features)
make check-msrv: green (exit 0)
make check-python: green (exit 0)
make check-feature-profiles: green (exit 0; required completing 4
  mechanical pre-existing FxHash corrections below)
make check-full: green (exit 0, incl. TUI PTY smoke)
make check-features-individual: green (89 passed, 0 skipped, 0 failed)
make release-check: green on the clean implementation tree (re-run
  post-commit; dirty-tree gate fails closed by design pre-commit)

Architecture guards updated: Check 134 requires published
  `eggfetch-core 0.2.0` (+1.89 MSRV retained); Check 102 floor raised to
  the 0.2.x line with 0.2.0+ wording; Checks 135/136/137 security
  assertions preserved byte-for-byte (only historical 0.1.7-adoption
  labels retained where they name the originating event).
Living docs updated: adapter Cargo.toml comment + lib.rs/adapter.rs/
  mapping.rs version wording; architecture/transport_eggfetch.md (0.2.0
  adoption entry, current-state refs, requalification footer);
  architecture/loadtest.md (active-backend + route-matrix + see-also
  refs; historical 0.1.5/0.1.7 measurement tables preserved);
  architecture/overview.md (3 refs); README.md (2 refs);
  docs/CI_ARCHITECTURE_GUARDS.md (Checks 102-line + Check 134-line);
  .opencode/skills/eggsec-loadtest + eggsec-config (current-state
  backend/guard wording); plans/README.md (marked executed).
  AGENTS.md encodes no Eggfetch version: unchanged.
Historical records preserved: 0.1.7 adoption/corrective plans, 0.1.5/0.1.7
  measurement tables, upstream 0.1.7 qualification record, guard 136/137
  adoption-event labels, parity.rs adoption comment: all untouched.
Performance/footprint disposition: no new benchmark framework; no
  performance claim from upstream maintenance without Eggsec measurements.
  Duplicate graph unchanged (only eggfetch versions moved); no binary/RSS
  regression tracked. Bounded 1/10/50/100 loopback evidence remains the
  retained 0.1.7-era record in loadtest.md (noisy, never a CI threshold);
  version-to-version comparison stays inconclusive by prior record.
Residual debt: (a) pre-existing HEAD FxHash fallout corrected here to
  unblock feature profiles (ProxyFlow std->FxHashMap constructors in
  crates/eggsec/src/pipeline/executor.rs,
  crates/eggsec/src/tool/implementations/proxy.rs,
  crates/eggsec/src/commands/handlers/web_proxy.rs,
  crates/eggsec-tui/src/tabs/intercept/tests.rs; verified failing on the
  clean 1296f6f7 baseline via stash, so unrelated to this bump);
  (b) TUI broad-profile warnings (unused imports/variables, e.g.
  mobile/mod.rs, task_dispatcher.rs) remain warnings-only, pre-existing.
```

Later closure (2026-09-22): residual debt (b) above is now closed by
[tui-broad-profile-warning-debt-cleanup-2026-09-22.md](tui-broad-profile-warning-debt-cleanup-2026-09-22.md)
without altering this historical record. Historical statement that warnings
existed at adoption time is preserved; see the cleanup pass for after-evidence.
