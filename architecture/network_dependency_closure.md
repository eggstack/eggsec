# Network-Dependency Roadmap Closure Report (Phase G)

Status: Closed 2026-09-13. The roadmap is executed; no new architecture was
introduced in this phase (measurement, verification, and documentation only).

Plan: `plans/network-dependency-phase-g-closure-measurement.md` (marked
Executed; completion record at the end of that file). The plan asked for the
report under `docs/architecture/`; the repository keeps architecture
deep-dives at the top-level `architecture/` directory, so the retained report
lives here as `architecture/network_dependency_closure.md` instead.

## Required fields

- **Baseline SHA:** `06a7c2c4` (Phase A measurement commit).
- **Final code SHA:** `f4ba2198` (all `src/` + manifest work complete; the
  Phase G closure commit on top is docs/plans only and changes no graph).
- **Eggfetch version/SHA:** `eggfetch-core 0.1.3` (registry;
  checksum `43a7c72c…272d9c` in `Cargo.lock`). No resolver hook or pre-follow
  callback exists in any release, so the adapter-level approved-IP pinning +
  manual redirect loop stands unchanged.
- **Egress disposition:** all rejected, no workspace edge. `eggress-uri 1.0.6`
  (`414a171b…6090e1`) rejected (11 unrelated protocols imported into policy,
  redact-instead-of-reject is weaker than the transport contract, no shared
  substrate with `eggfetch-core`, zero deleted code). `eggress-routing 1.0.6`
  (`86fe780d…0f79c7af` + `eggress-core 1.0.6` `20bf3299…492d7c22`) rejected
  (+5 packages minimum, second CIDR family `ipnet v2` beside `ipnetwork`, a
  second policy language beside `Scope`/`TargetScope` + `NetworkAuthority`).
  Protocol/transport stacks rejected (no duplication case). Full record:
  `architecture/egress_reuse_decision.md`.

## Removed direct dependencies by crate

- `eggsec-agent`: `reqwest` + `rustls` removed from manifest and sources
  (Phase D increment 1). `cargo tree -p eggsec-agent`: 257 → 153 lines;
  now 11 direct deps (`chrono`, `eggsec-core`, `eggsec-transport`, `rustc-hash`,
  `serde`, `serde_json`, `tokio`, `tokio-util`, `tracing`, `url`, `uuid`).
  `LifecycleManager<T: HttpTransport>` injects transport + authority.
- `eggsec-web-proxy`: `reqwest` pruned to `["rustls-no-provider", "socks"]`
  (dropped unused `json`/`http2`/`form`/`query`/`blocking`/`cookies`).
- Shared helpers: `auth_context::apply_auth_context_to_request` and
  `AiClient::apply_auth` concrete-builder wrappers removed (no `RequestBuilder`
  in shared APIs).
- `.cargo/audit.toml` deleted (24 stale ignores); `cargo audit` is
  diagnostic-only in no workflow/Makefile target.

## Remaining sensitive dependency owners (all with reason + scope)

| Owner | Dependency | Reason / scope |
|---|---|---|
| `eggsec` engine | `reqwest` (json/socks/http2/form/query/blocking/cookies), `rustls`, `tokio-rustls`, `hickory-resolver`, `webpki-roots` | Pending per-subsystem migration (Phase D increment 1 migrated interfaces only). Raw TLS (`distributed/io.rs`, `waf/bypass/smuggling.rs`) and DNS (`packet`/`recon`/`scanner` hickory + `lookup_host`) remain specialized by design |
| `eggsec-nse` (+`nse` feature) | `reqwest` (json/blocking), `rustls`, `hickory-resolver`; `openssl`/`native-tls` (feature-gated), `ssh2` (`nse-ssh2`) | Blocking clients inside sync Lua closures (async-Lua story pending); protocol-compat island (`sslcert`/`openssl` libs), never ordinary HTTP |
| `eggsec-web-proxy` | `reqwest` (minimal), `rustls` + `tokio-rustls` (MITM `TlsAcceptor`), `rcgen`, `h2`/`http`/`prost` (optional interception) | Side A intercept/server stays; side B proxy-routing probes stay minimal until the adapter supports authorized proxy routing |
| `eggsec-python` | `reqwest` | Pending migration behind the same seam (`EGGSEC_ALLOW_LOOPBACK_FIXTURE=1` fixtures) |
| `eggsec-transport-eggfetch` | `eggfetch-core 0.1.3` (http1 + tls-rustls + proxy-for-SNI) | Intentional sole backend binding; no production consumers yet |
| `eggsec-daemon` | `reqwest` in `#[cfg(test)]` only | Integration tests, no production stack |
| Transitive | `hyper`/`hyper-rustls`/`rustls-platform-verifier`, duplicate `webpki-roots 0.26/1.0` | Accepted: separate roles (ordinary clients vs pinned backend); `deny.toml` bans `aws-lc-rs` (ring-only) |

`cargo tree -i openssl` / `-i native-tls` match no package in the default
closure (feature-gated NSE only); `-i eggress-uri` / `-i eggress-routing` /
`-i quinn` match nothing anywhere. `openssl-probe` in the closure is a
platform-root transitive (via `rustls-platform-verifier`), not an OpenSSL
link.

## Before/after key measurements

| Measurement | Baseline (Phase A, 2026-09-12) | Final (Phase G, 2026-09-13) |
|---|---|---|
| `cargo metadata --locked` packages | 587 (827 registry + 16 path, 0 git) | 592 (574 registry + 18 path, 0 git; `Cargo.lock` 829 registry + 0 git) — delta is Phase C/D crates, not Phase E/G additions |
| `eggsec --no-default-features` direct deps | 55 | 59 normal-edge (+ `eggsec-agent` consumer edge + `eggsec-transport` leaf; unconditional `reqwest`+`rustls`+`tokio-rustls`+`hickory-resolver` retained for pending subsystems) |
| `cargo tree -d` duplicate families | 36 claimed (method-dependent) | 16 top-level families re-measured with `grep -v "^[ │├└]"` (cpufeatures, derive_more, dirs-sys, getrandom, hashbrown, itertools, phf, phf_shared, rand, rand_chacha, rand_core, socket2, strum, strum_macros, syn, webpki-roots); TLS-relevant dup unchanged (`webpki-roots 0.26/1.0`, no `aws-lc-rs`) |
| `eggsec-transport` normal edges | — (new Phase B) | 87 lines, unchanged by Phase E/G; exactly `bytes`/`http`/`url`/`thiserror` |
| `eggsec-transport-eggfetch` normal edges | — (new Phase C) | 217 lines (216 at Phase E; +1 from registry drift, no manifest change) |
| Engine `default` | `["cli"]` | `[]` (Phase E WS2; `cli` opt-in via process hosts + daemon `full-executor`) |
| Tokio baseline | 11 features incl. `test-util` | `default-features = false` + per-crate `features`; `test-util` nowhere (Phase E WS3; `transport:test-util` is the fake-gate, dev-deps only) |
| Release binary size / build timing | not recorded (no release artifact; supporting evidence only) | unchanged: not a gate, still unrecorded (see debt) |

Success was defined as fewer duplicated implementation owners and narrower
capability reach, not fewest crates: `eggsec-agent` lost its entire ordinary
HTTP/TLS stack, web-proxy reqwest halved its features, shared APIs carry no
concrete builders, engine default is empty, Tokio is per-crate, and no second
HTTP/TLS stack exists in normal artifacts (the pinned eggfetch backend is
dev-gated with no production consumers).

## Security fixture results (all local, deterministic)

- `eggsec-transport` unit: 18 passed (redaction, cookie merge, redirect
  policy, TLS consistency, resolver ordering, binding, fake
  destination/redirect/denial, later-hop re-resolution).
- `transport_contract` (engine `ScopeAuthority` + fake): **12 passed**
  (binding, out-of-scope DNS, mixed answers, same/cross-host redirects,
  later-hop re-resolution order, out-of-scope redirect denial, userinfo +
  redaction, secret redaction, direct IP, proxy distinctness, TLS
  orthogonality, invented-address rejection).
- `eggsec-transport-eggfetch` parity: 33 integration + 8 mapping unit passed
  (approved-address binding with `remote_addr` assertion, per-hop pinning,
  DNS-change-between-hops denial, redirect allow/deny/strip gates, method/body
  rewrite, proxy fail-closed + redaction, verbatim compression, total/connect/
  zero timeouts, cancellation reuse, TLS reject/accept/mismatch, 304
  surfacing, mixed answers, checkpoint order, `reresolution` usage).
- `transport_eggfetch_parity` (engine interop): 5 passed.
- `network_policy_invariants` (Phase A baseline): 12 passed, unmodified.
- `eggsec-agent` (incl. 2 transport success/failure tests): 24 passed.
- WS2 17-item mapping: allowed/denied/mixed DNS, re-resolution,
  same-origin + separately-authorized cross-origin + denied + loopback/private
  redirects, direct IP, cross-origin stripping, proxy endpoint-vs-target,
  remote-DNS SOCKS fail-closed, SNI/Host consistency, insecure-TLS
  orthogonality, cancellation, HTTP/3 mechanically disabled (no `quinn` in any
  graph; adapter pins `Http1Only`). Every hostname hop pins the wire URL to
  the approved IP literal; `Host`/SNI carry the logical name; peer address is
  asserted, not just policy invocation.

## CI/supply-chain policy state

- `make check` (fmt, no-default checks, `check-deps`, clippy, doc +
  integration tests, guards): exit 0.
- `make check-deps` (`cargo deny --workspace --all-features check` ×5):
  advisories/bans/licenses/sources all ok; fails closed without `cargo-deny`.
- `make clippy-domain`, `make check-feature-profiles`,
  `make check-features-individual`, `make check-python`, `make check-msrv`
  (1.89): all exit 0. TUI `--features full` checks clean; daemon lib 74
  passed; NSE `--features nse` lib 195 passed; web-proxy lib 383 passed.
- Deny canonical (12 advisory ignores matching `docs/DEPENDENCY_EXCEPTIONS.md`
  fields; `MIT-0` allow + `auto_generate_cdp` per-crate GPL exception;
  `wildcards = "deny"`, `unknown-registry/git = "deny"`,
  `required-git-spec = "rev"`); `.cargo/audit.toml` removed.
- All Actions `uses:` SHA-pinned with version comments; top-level +
  per-job `permissions: contents: read`; `.github/dependabot.yml` (weekly
  cargo + actions, no auto-merge); `dependency-review` job (PR-only,
  moderate+, Deny authoritative for Rust).
- Negative test (ephemeral branch, fully reverted): an unknown git dependency
  fails closed before policy evaluation (cargo cannot resolve it into the
  tree; `unknown-git = "deny"` is the second layer).
- Guards `scripts/check-architecture-guards.sh`: ALL PASSED (Checks 99–112).

## Remaining debt (owner + removal criterion)

1. Engine per-subsystem backend migration (OOHTTP still on reqwest; `fuzzer`
   trait `&reqwest::Client` + `ClientPool` pub surfaces pending). Owner: next
   Phase D increments (webhook/tracker first). Criterion: each subsystem cut
   over with fake-based parity tests; remove only after graph check.
2. NSE blocking clients + hickory DNS + feature-gated `openssl`/`native-tls`.
   Owner: async-Lua story + `ScopeAuthority` binding. Criterion: new script
   code uses `http_capability.rs`; Lua dispatch migrates when sync-closure
   constraint lifts.
3. Proxy side-B minimal reqwest. Owner: scoped proxy-routing backend or direct
   probes. Criterion: adapter gains authorized proxy routing, or probes go
   direct via `outbound.rs`.
4. Python `reqwest` surface. Owner: transport-seam migration with loopback
   fixtures. Criterion: parity suite green behind the seam.
5. No production eggfetch consumers (adapter proven, wiring per consumer).
   Owner: Phase D ordering. Criterion: focused parity tests per consumer;
   guards 102/103 keep this honest.
6. Release binary size / build timing unrecorded (supporting evidence only,
   never a gate). Owner: whoever needs it; build a release artifact and record
   here.

## Acceptance mapping (roadmap §Final acceptance criteria)

1. Migrated consumers (agent) use the mandatory contract (`LifecycleManager<T:
   HttpTransport>` + fake tests); engine/NSE/proxy/python dispositions in §7.2
   of the baseline + debt above. 2. Resolved/connected destinations are
   policy-bound (approved-IP pinning, `validate_binding`, per-hop
   re-resolution; peer asserted). 3. Redirects cannot escape scope
   (authorized loop, same-host gate verbatim, per-hop re-checks; secrets
   stripped cross-origin, redacted in `Debug`). 4. Reqwest removed from the
   migrated crate (agent); residuals justified above. 5. Proxy keeps its
   MITM/server boundary (`outbound.rs` A/B split; ordinary builds avoid
   `rcgen`/`h2`/WS/gRPC via optionality — `rcgen`/`h2` in the engine closure
   are pre-existing cert-gen/reqwest-http2, not proxy-induced). 6. Engine
   default empty (`default = []`; Check 104). 7. Tokio per-crate, `test-util`
   nowhere in production (Check 105). 8. Egress decided with measured graphs,
   no umbrella edge (Check 106). 9. Deny is a PR gate; audit removed; sources
   fail closed (Checks 109–112 + `check-deps`). 10. Actions SHA-pinned,
   Dependabot on, least-privilege (Checks 110–112). 11. MSRV, no-default,
   feature sweep, TUI/daemon/Python, domain, and local gates green (hosted CI
   recorded on push). 12. Measurements retained here + baseline §10; duplicate
   ownership/capability reach reduced (agent stack removal, reqwest pruning,
   empty default, per-crate Tokio). 13. No new offensive capability or
   authorization bypass introduced (scope model untouched; enforcement paths
   unchanged).

*Measured against source: 2026-09-13 (code SHA `f4ba2198`; toolchain stable +
1.89; MSRV 1.89 per workspace `rust-version`).*

> Drift note (2026-09-25, docs-only review; closure measurements above unchanged): `Cargo.lock` now shows `eggfetch-core 0.2.0` (was `0.1.3`), `rustls 0.23.45` (was `0.23.43`), `eggress-outbound`/`eggress-uri 1.0.10` (narrow `eggsec-web-proxy` edge, Check 106 exact allowlist), `cargo metadata --locked` 608 packages (vs 592 — registry drift + Phase C/D crates, no new capability edge). `eggsec-agent` still has 11 direct deps and no `reqwest`/`rustls`; `eggsec-transport` still exactly `bytes`/`http`/`url`/`thiserror`.
