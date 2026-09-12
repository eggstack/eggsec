# Phase C — Eggfetch readiness and EggSec adapter

Status: Ready for handoff

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
