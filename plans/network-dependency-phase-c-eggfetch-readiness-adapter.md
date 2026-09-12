# Phase C — Eggfetch readiness and EggSec adapter

Status: Executed 2026-09-12

Date: 2026-09-11

Depends on: Phases A-B

## Purpose

Make `eggfetch-core` a safe backend for EggSec's outbound HTTP contract. Close upstream gaps first, then implement an EggSec adapter. Do not migrate production consumers until the adapter passes the Phase A parity/security matrix.

## Confirmed Eggfetch fit

Current `eggfetch-core` already owns most ordinary client behavior EggSec needs: Hyper/Rustls transport, HTTP/1-3 feature gates, pooling, redirects, auth, cookies, compression, proxy/SOCKS support, retries, phase-aware timeouts, TLS configuration, request/response types, connection metadata, and redaction.

Important current limitation: the direct connector documents and implements DNS internally using `tokio::net::lookup_host`. The normal Hyper connector path likewise owns connection resolution below the EggSec layer. EggSec therefore cannot yet prove that the socket address it authorizes is the socket address Eggfetch connects to. This must be closed before Eggfetch becomes the scope-enforcing backend.

Eggfetch redirect code already strips `authorization` and `proxy-authorization` across hops and strips cookie/host on cross-origin redirects. Preserve that behavior, but authorization of the redirect target must occur before the redirected request is dispatched.

## Workstream 1 — Upstream Eggfetch resolver/connection hook

Implement the smallest generic extension in `eggstack/eggfetch` that permits a caller to control or authorize destination resolution without adding EggSec-specific concepts to Eggfetch.

Preferred design properties:

- generic public resolver/connect policy trait or approved-address connector input;
- caller can inspect canonical host/port and resolved candidate addresses;
- caller can reject all candidates or provide an approved subset;
- actual TCP connection uses only the approved addresses for that resolution attempt;
- each re-resolution/reconnect invokes the hook again;
- direct TCP, advanced socket-option, proxy, SOCKS, and any alternate protocol path have an explicit disposition;
- no callback receives request secrets unnecessarily;
- default Eggfetch behavior remains unchanged for ordinary consumers;
- API is async-safe and does not require Eggfetch to depend on EggSec.

If the normal Hyper connector cannot expose the required binding cleanly, route scope-controlled clients through Eggfetch's custom connector path or add a generic connector injection point. Prefer one authoritative connection path over parallel scope behavior.

### HTTP/3 disposition

HTTP/3/QUIC can use different connection/resolution machinery. Do not enable it in the initial EggSec adapter unless the same resolved-destination authorization guarantees are implemented and tested. Explicitly disable/defer HTTP/3 rather than silently allowing a weaker policy path.

### Proxy/SOCKS disposition

For proxies, distinguish:

1. authorization of the ultimate request target; and
2. authorization/configuration of the proxy endpoint actually connected to.

For remote-DNS SOCKS semantics (`socks5h`), the local process may not observe the target IP before the proxy resolves it. EggSec must either have an authorization model that explicitly permits this trust boundary or reject/defer remote-DNS proxy mode for strict scoped execution. Document the decision and test it.

## Workstream 2 — Redirect authorization hook

Provide a generic hook or adapter-level manual redirect loop that lets EggSec authorize every redirect URL before the next request is sent.

Requirements:

- preserve Eggfetch method rewrite/body replay rules;
- preserve sensitive-header stripping;
- authorize the canonical next URL before resolution/connection;
- preserve redirect history and max-hop behavior;
- errors distinguish scope rejection from malformed/unsupported redirect;
- no second unrestricted automatic redirect path remains enabled underneath the adapter.

If Eggfetch cannot expose a pre-follow callback safely, configure it not to auto-follow and implement the loop in `eggsec-transport-eggfetch` using Eggfetch's public redirect/request primitives. Do not duplicate redirect semantics if a reusable upstream hook can be added cleanly.

## Workstream 3 — Build `eggsec-transport-eggfetch`

Create a separate implementation crate so the transport contract remains independent of Eggfetch.

Recommended dependencies:

```text
eggsec-transport
eggfetch-core with only required features
http/url/bytes only where adapter conversion needs them
tracing if required
```

Start with the minimum Eggfetch features demonstrated by the Phase A parity matrix. Do not enable HTTP/3, every compression codec, cookies, multipart, or proxy support by default unless an EggSec consumer needs them.

Map:

- EggSec request/response DTOs;
- timeout semantics;
- TLS policy, including explicit insecure mode;
- redirects;
- cookies/auth context;
- proxy policy where supported;
- cancellation/error classification;
- connection metadata required by scanners.

Do not leak Eggfetch errors or request-builder types through `eggsec-transport` public APIs. Preserve useful source errors internally for diagnostics while mapping them into stable EggSec transport errors.

## Workstream 4 — TLS policy parity

Compare current Reqwest/Rustls behavior with Eggfetch's `TlsConfig`/trust-store API. Add fixtures for:

- normal WebPKI/native roots according to EggSec's current supported behavior;
- self-signed local certificate rejection;
- explicit custom trust acceptance if supported today;
- explicit insecure-TLS acceptance only when existing policy permits it;
- hostname/SNI mismatch rejection;
- client identity only if current EggSec paths require it;
- ALPN/HTTP2 behavior where relied upon.

There must be no silent fallback to a different trust policy if configuration fails.

## Workstream 5 — Parity and adversarial fixture suite

Run the Phase A matrix against both the current Reqwest path and the Eggfetch adapter during migration. Use local fixtures to compare semantically meaningful behavior rather than byte-identical error strings.

Mandatory adversarial cases:

- DNS answer changes from allowed to denied between connections;
- redirect from allowed hostname to loopback/private/out-of-scope destination;
- redirect with userinfo;
- cross-origin auth/cookie stripping;
- proxy credential redaction;
- compressed response exceeding configured decoded-size/ratio limits if those protections are enabled;
- retry of non-replayable bodies;
- timeout at connect/read/total boundaries;
- cancellation while resolving/connecting/reading.

## Cross-repository handoff requirement

If Eggfetch must change, land and verify the generic Eggfetch changes first. Record in this plan's completion section:

- Eggfetch commit SHA;
- release/version used by EggSec;
- exact public API added;
- Eggfetch's own tests and CI result;
- dependency/feature delta introduced by the new hook.

EggSec should consume a released/pinned sibling version according to the workspace's normal dependency policy rather than an unbounded branch reference.

## Required verification

Eggfetch repository:

```text
cargo fmt --all -- --check
cargo test -p eggfetch-core
cargo clippy -p eggfetch-core --all-targets -- -D warnings
cargo tree -p eggfetch-core -e features
```

EggSec repository:

```text
cargo check -p eggsec-transport-eggfetch --no-default-features
cargo test -p eggsec-transport-eggfetch
cargo tree -p eggsec-transport-eggfetch -e features
make check
make test-architecture-guards
```

Also run the full local scope/DNS/redirect/TLS fixture suite.

## Acceptance criteria

1. Eggfetch can bind caller-approved resolution results to actual connections or exposes an equivalent secure connector hook.
2. Every redirect hop can be authorized before dispatch.
3. Re-resolution invokes authorization again.
4. HTTP/3 and remote-DNS proxy paths have explicit safe dispositions; no weaker path is accidentally enabled.
5. `eggsec-transport-eggfetch` implements the Phase B contract without leaking Eggfetch types.
6. Required TLS, timeout, redirect, auth/cookie, body, and proxy semantics pass parity tests.
7. Eggfetch remains independent of EggSec.
8. No production EggSec consumer is migrated until this phase's adapter/security suite passes.

## Expected files touched

Eggfetch repository, if prerequisites are required:

- `crates/eggfetch-core/src/client.rs`;
- connector/resolver modules under `crates/eggfetch-core/src/transport/`;
- redirect/pipeline APIs if a pre-follow hook is selected;
- tests/docs/public exports.

EggSec repository:

- root workspace manifest;
- new `crates/eggsec-transport-eggfetch/`;
- integration fixtures/tests;
- architecture documentation and this completion record.

## Completion record (Executed 2026-09-12)

EggSec SHAs: baseline `ae9a3aa9` (Phase B head) through Phase C commit
(recorded in `git log`; no production-consumer migration in this phase).

### Workstream disposition

- **WS1 (resolver/connection hook): no upstream change required.**
  Probed `eggfetch-core 0.1.3` (crates.io): every connector resolves
  internally via `tokio::net::lookup_host` with no authorization hook,
  and the SNI-direct path builds a TLS connector only with the `proxy`
  or `http3` features enabled. Instead of an upstream change, the
  adapter binds through public APIs: resolve via the injected
  `TransportResolver` → `authorize_resolved`/`authorize_reresolution` →
  `validate_binding` → `authorize_socket` → rewrite the wire URL host to
  the approved IP literal (connector self-resolves the literal; `Host`
  header + TLS SNI preserve the logical hostname). One authoritative
  path; IP literals skip resolution entirely. A generic upstream
  `DnsResolver` trait + pre-follow redirect callback is recorded as
  optional hardening in `architecture/transport_eggfetch.md`, not a
  prerequisite.
- **WS2 (redirect authorization): adapter-level manual loop** (the
  plan-anticipated fallback). Auto-follow is doubly disabled (client +
  per-request); each hop reuses the public
  `redirect::build_redirect_request` primitive (method rewrite, header
  stripping, body-replay rules stay upstream) and passes
  `authorize_redirect` + the full checkpoint sequence before dispatch.
- **WS3 (`eggsec-transport-eggfetch`): implemented.** Published
  `eggfetch-core 0.1` with `default-features = false` +
  `["http1", "tls-rustls", "proxy"]` (`proxy` solely for the SNI-direct
  TLS connector; routing forcibly disabled via `without_proxy` + no
  configured proxy). No `eggfetch` types in the public API
  (`EggfetchTransport::new(resolver)` + `HttpTransport` only).
- **WS4 (TLS parity): implemented per table in
  `architecture/transport_eggfetch.md`.** Verified = WebPKI-only roots
  (Reqwest-default parity); insecure = separate warn-logged client;
  SNI = logical host; HTTP/1.1 pinned. Fixtures: self-signed rejection,
  insecure acceptance (+SNI metadata), hostname-mismatch rejection.
  Verified-success e2e has no local fixture (no custom-CA row in
  `TlsPolicy`; Phase A confirms it is unused) — documented gap.
- **WS5 (parity/adversarial suite): implemented.** 33 adapter tests
  (`crates/eggsec-transport-eggfetch/tests/parity.rs`, local plain+TLS
  loopback fixtures) covering every mandatory adversarial case
  (DNS-change-between-hops, allowed→denied redirect, userinfo/unsupported
  redirects, cross-origin stripping, credential redaction, verbatim
  compression, non-replayable N/A, connect/read/total timeouts,
  cancellation) + 5 engine interop tests through `ScopeAuthority`
  (`crates/eggsec/tests/transport_eggfetch_parity.rs`).

### Cross-repository handoff

- No `eggfetch` change landed or consumed: adapter pins crates.io
  `eggfetch-core 0.1` (resolving to `0.1.3` in `Cargo.lock`), no branch
  or git reference; `eggfetch` remains independent of EggSec.
- Dependency delta: `+eggfetch-core 0.1.3` (+ its Hyper/Rustls closure,
  ring-only) in the workspace lockfile; ordinary builds gain no new
  system dependencies.

### Verification (all local, before push)

```text
cargo fmt --all -- --check
cargo check --workspace --no-default-features
cargo clippy -p eggsec-transport-eggfetch --all-targets -- -D warnings
cargo test -p eggsec-transport-eggfetch            # 8 unit + 33 parity
cargo test -p eggsec --features rest-api --test transport_eggfetch_parity  # 5 interop
cargo tree -p eggsec-transport-eggfetch -e features
make check                  # full mandatory contract incl. new Check 102
make test-architecture-guards
```

Phase A (12) + Phase B (11) suites remain green and unmodified.

### Acceptance mapping

1. Approved-resolution binding proven via pinning (adversarial
   DNS-change + mixed-answer + panicking-resolver tests). ✅
2. Every redirect hop authorized before dispatch (manual loop,
   auto-follow doubly disabled). ✅
3. Re-resolution re-invokes authorization (`authorize_reresolution` on
   hops > 0; tested). ✅
4. HTTP/3 never constructed (feature off + `Http1Only`); remote-DNS
   proxy unreachable (feature-gated routing disabled + non-Direct
   intents fail closed at `proxy`). ✅
5. Phase B contract implemented with no `eggfetch` leakage (Check 102). ✅
6. TLS/timeout/redirect/auth-cookie/body/proxy semantics pass parity
   fixtures (38 tests). ✅
7. `eggfetch` independent of EggSec (no upstream change). ✅
8. No production consumer migrated (Check 102 fails otherwise). ✅

### Residual debt (Phase D/G input)

- Upstream `DnsResolver`/pre-follow hooks remain optional hardening;
  evaluate against a released `eggfetch-core` before Phase D widens use.
- `RecordingFakeTransport` still reports `Dns` (not `Reresolution`) on
  later hops while the adapter uses `authorize_reresolution`; optional
  Phase G alignment (behavioral verdicts identical under
  `ScopeAuthority`).
- Verified-TLS-success e2e awaits a custom-CA policy row only if a
  consumer ever needs it.

### Follow-up (2026-09-12, Phase D/G input resolved)

- **Fake/adapter checkpoint alignment: landed.** `RecordingFakeTransport`
  now threads the hop index through `authorize_one_hop`: hostname hops
  after the first call `authorize_reresolution` (checkpoint
  `reresolution`) and map binding failures to that hop's DNS checkpoint,
  exactly like `EggfetchTransport::authorize_hop`. IP literals keep
  `authorize_resolved` on every hop (nothing re-resolves); proxy-endpoint
  DNS keeps `authorize_resolved` (the adapter fails closed before proxy
  DNS, so there is no backend behavior to mirror). New tests:
  `fake_uses_reresolution_on_later_hops` +
  `fake_maps_binding_failure_to_hop_checkpoint` (fake unit) and
  `later_redirect_hops_authorize_reresolution` (engine contract through
  `ScopeAuthority`). Behavior under all existing authorities is unchanged
  (default delegation); only the recorded checkpoint label and the
  invoked method on later hops changed. `architecture/transport.md`
  (fake + flow + test counts) and `architecture/transport_eggfetch.md`
  (checkpoint section) updated.
- **Upstream hooks re-evaluation: nothing to adopt.** Latest released
  `eggfetch-core` is still `0.1.3` (`cargo search 2026-09-12`; lockfile
  pins `0.1.3`) — no `DnsResolver` trait or pre-follow redirect callback
  exists in any release. Adapter-level binding + manual loop stand
  unchanged; re-evaluate when upstream ships the hook.
- **Verified-TLS-success e2e: still deferred.** No consumer needs a
  custom-CA policy row (no production migration in this pass either);
  remains conditional future work.
