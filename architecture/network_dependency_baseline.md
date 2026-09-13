# Network Dependency Baseline — Phase A (measurement only)

Status: Baseline recorded 2026-09-12; Phase D increment-1 addendum §7
(2026-09-12); Phase E closure addendum §8 (2026-09-13, no HTTP client
migration in Phase A, no egress/new-crate additions in Phase E).

This is the retained per-artifact dependency baseline, concrete-client
inventory summary, migration parity matrix, and security-policy state for the
network-dependency hardening roadmap. Full generated `cargo tree` dumps are
not retained here; the commands below reproduce them. Counts were captured on
2026-09-12 against `reqwest 0.13.4` / `rustls 0.23.43` /
`tokio-rustls 0.26.4` / `hickory-resolver 0.26.1` (all registry-only;
`openssl` / `native-tls` match no workspace package).

Canonical enforcement context: `EnforcementContext::evaluate()` remains the
mandatory pre-dispatch gate. `Scope` / `LoadedScope` / `TargetScope` +
`HostResolver` (`SystemResolver` default) remain the single scope model. Do
not invent a second scope model. Executable invariants live in
`crates/eggsec/tests/network_policy_invariants.rs`.

## 1. Reproduction commands

```text
cargo metadata --locked --format-version 1
cargo tree -p eggsec --no-default-features
cargo tree -p eggsec-cli
cargo tree -p eggsec-cli --features full
cargo tree -p eggsec-agent
cargo tree -p eggsec-nse --features nse
cargo tree -p eggsec-web-proxy --features web-proxy
cargo tree -d
cargo tree -e features
cargo tree -i reqwest
cargo tree -i rustls
cargo tree -i tokio-rustls
cargo tree -i hickory-resolver
cargo tree -i openssl
cargo tree -i native-tls
```

`openssl` / `native-tls` correctly match no package (`cargo tree -i`
reports "did not match any packages"). `nse` needs `libssl-dev`;
`wireless` needs `wireless-tools`; `packet-inspection` needs `libpcap-dev`;
`grpc-api` needs `protobuf-compiler` (descriptor only, code checked in).
Run system-dep profiles in the same provisioned environment as Deep Checks
and distinguish source/build failures from missing host packages.

## 2. Per-artifact baseline (2026-09-12)

| Artifact / profile | Direct deps (`--depth 1` count) | Default + enabled features | HTTP/TLS/DNS owners | Native deps / build scripts |
|---|---|---|---|---|
| `eggsec --no-default-features` | 55 direct (63 lines incl. header) | `default = ["cli"]`; this profile disables `cli` clap surface but keeps unconditional `reqwest` + `rustls` + `tokio-rustls` + `hickory-resolver` | `reqwest 0.13` (`json`, `rustls-no-provider`, `socks`, `http2`, `form`, `query`, `blocking`, `cookies`, no default features); `rustls 0.23` (`ring`, `std`, `tls12`, no default); `tokio-rustls 0.26` (`ring`, `tls12`, `logging`, no default); `hickory-resolver 0.26` (`tokio`, no default); `webpki-roots 0.26`; `rustls-pki-types 1` | No native link in this profile. `rcgen 0.14` (ring-backed cert gen). No build script in `eggsec` itself |
| `eggsec-cli` (default) | 16 direct | Thin shell: `clap`, `clap_complete`, `eggsec` (default → `cli`), `eggsec-runtime`, `eggsec-ui-model`; optional `tui` (`eggsec-tui`) and `daemon-client` | Inherits engine owners transitively; no direct `reqwest`/`rustls`/`hickory` in `eggsec-cli/Cargo.toml` | None |
| `eggsec-cli --features full` | superset of default; `full` is curated (28 pinned entries, `FULL_MEMBERS` / `FULL_EXCLUDED_WITH_REASON` in `feature_registry.rs`) | `full` pulls `stress-testing`, `packet-inspection`, `rest-api`, `nse`, `ai-integration`, `websocket`, `headless-browser`, `database`, `container`, `sbom`, `advanced-hunting`, `compliance`, `external-integrations`, `finding-workflow`, `vuln-management`, `wireless`, `wireless-advanced`, `mobile`, `mobile-dynamic`, `db-pentest`, `web-proxy`, `evasion`, `postex`, `c2`, `email-notifications`, `logging-subscriber`, `config-watch` | Adds system-dep owners only under their features (see below) | `pnet`/`libpcap-dev` (`packet-inspection`, `stress-testing`), `libssl-dev` (`nse`), `libssh2-dev` (`nse-ssh2`), `protobuf-compiler` (`grpc-api`) |
| `eggsec-agent` | 12 direct | No feature gates on network deps | Direct `reqwest 0.13` (`rustls-no-provider`), direct `rustls 0.23` (ring init + `Client::builder` health checks in `lifecycle.rs`) | None |
| `eggsec-nse --features nse` | 34 direct (+ dev `toml`) | `nse = ["tool-api", "dep:eggsec-nse", "eggsec-nse/nse"]` | Direct `reqwest` (`json`, `blocking`, `rustls-no-provider`), direct `rustls`, direct `hickory-resolver` (dns/dnsbl libs), `tokio::net::lookup_host` in `socket.rs`; blocking client fan-out across 10+ Lua libraries | `openssl`/`native-tls` are **not** in the resolved graph for this feature set on this host (see `cargo tree -i` above); `nse-ssh2` (`libssh2-dev`) and system `libssl-dev` apply to adjacent feature sets |
| `eggsec-web-proxy --features web-proxy` | 33 direct (+ dev `criterion`, `tempfile`) | `web-proxy` pulls `eggsec-web-proxy/web-proxy` + `tokio-tungstenite` + `h2` + `http` + `prost`/`prost-types` | Direct `reqwest` (full engine-equivalent feature set), direct `rustls` + `tokio-rustls` (MITM `TlsAcceptor` in `intercept/mod.rs`), `tokio::net::lookup_host` in `lib.rs:309` | `rcgen 0.14` for on-the-fly cert gen; no native link beyond ring |

Workspace totals: `cargo metadata --locked` reports 587 packages; `Cargo.lock`
sources are 827 × `registry+https://github.com/rust-lang/crates.io-index` +
16 × path (workspace crates). Zero git sources.

Duplicate-version families (`cargo tree -d`, 36 unique families; load-bearing
examples): `cpufeatures 0.2.17/0.3.0`, `derive_more 0.99.20/2.1.1`,
`dirs-sys 0.4.1/0.5.0`, `getrandom 0.2.17/0.3.4/0.4.3`,
`hashbrown 0.14.5/0.16.1/0.17.1`, `itertools 0.10.5/0.14.0`,
`phf 0.11.3/0.12.1`, `phf_shared 0.11.3/0.12.1`,
`rand 0.8.7/0.9.5/0.10.2`, `rand_chacha 0.3.1/0.9.0`,
`rand_core 0.6.4/0.9.5/0.10.1`, `strum 0.27.2/0.28.0`,
`strum_macros 0.27.2/0.28.0`, `syn 2.0.119/3.0.3`,
`webpki-roots 0.26.11/1.0.9`. TLS-relevant dup note: `webpki-roots` appears
twice because `reqwest` pulls the platform verifier path alongside the pinned
`webpki-roots 0.26`; no `aws-lc-rs` in the graph (banned in `deny.toml`).

Release binary size for `eggsec-cli`: not recorded in this pass (no release
artifact built; `target/release/eggsec` absent). Build/check wall-clock time
is supporting evidence only, not an acceptance threshold — not recorded as a
gate.

## 3. Concrete client leakage inventory (summary)

Mechanical grep for `reqwest::` / `rustls::` / `tokio_rustls::` /
`hickory_resolver::` / `lookup_host` / `Client::builder` / `RequestBuilder`.
Classification: ordinary outbound HTTP (OOHTTP), TLS/interception server
(TLS/SRV), raw protocol/networking (RAWNET), compatibility-only
(COMPAT), process-host/integration (PROCESS).

No direct uses in `eggsec-core`, `eggsec-tool-core`, `eggsec-output`,
`eggsec-ui-model`, `eggsec-runtime`, `eggsec-cli`, `eggsec-tui`,
`eggsec-db-lab`, `eggsec-mobile-lab`, `eggsec-daemon-protocol`. `eggsec-daemon`
uses `reqwest` only in `#[cfg(test)]` (`http.rs` integration tests) plus a
ring provider init; `eggsec-cli`/`eggsec-tui` delegate to the engine.

| Owner | Representative sites | Class |
|---|---|---|
| `eggsec::utils::http` (`http.rs:29-249`) | 8× `Client::builder`, `Proxy::http`, `same_host_redirect_policy` | OOHTTP (central factory) |
| `eggsec::utils::client_pool` | `Client` + `Proxy` in public struct, `Client::builder` | OOHTTP (pool) |
| `eggsec::fuzzer` (`engine/core.rs:230`, `engine/advanced.rs:22`, `chain.rs:93`, `targets/api.rs:179` blocking, `advanced.rs:18` trait) | `Client::builder`, `reqwest::Client` trait param | OOHTTP |
| `eggsec::waf` (`bypass/mod.rs:75`, `headers.rs`, `evasion.rs`, `detector/mod.rs:22`, `bypass/smuggling.rs:5-13` raw TLS) | `reqwest::Client` fields; `smuggling.rs` also `rustls::*` + `tokio_rustls::TlsConnector` | OOHTTP except `smuggling.rs` = RAWNET |
| `eggsec::scanner` (`endpoints.rs:1011`, `cms/mod.rs:349`, `templates/marketplace.rs:64,307`, `templates/executor.rs:210` hickory, `icmp_probe.rs:117` lookup_host) | `Client::builder`, `TokioResolver`, `lookup_host` | OOHTTP + RAWNET (DNS) |
| `eggsec::recon` (`reverse_dns.rs`, `subdomain.rs`, `dns_records.rs` hickory; `dns_enhanced.rs:172-334` `lookup_host` ×5; `asn.rs`/`cve_lookup.rs`/`techdetect.rs` reqwest) | `TokioResolver`, `lookup_host`, `blocking::Client` | RAWNET (DNS) + OOHTTP (APIs) |
| `eggsec::hunt` (`mod.rs:42-52` + `request()` fns) | `Client::builder`, cookie store, same-host policy | OOHTTP |
| `eggsec::loadtest` (`runner.rs:340-353`), `stress/http.rs:123-169`, `c2/beacon.rs:76-138`, `c2/tasking.rs:290-416`, `agent/alerts/routing.rs:67-80`, `pipeline/executor.rs:24-37` (LazyLock ×2) | `Client::builder`, `Proxy::all`, `lookup_host` (c2/stress) | OOHTTP + RAWNET (DNS where noted) |
| `eggsec::integrations` (`jira.rs:22`, `github.rs:21`, `gitlab.rs:21`, `common.rs:42` `send_with_retry(reqwest::RequestBuilder)`) | `Client::builder`, `RequestBuilder` param | OOHTTP; `common.rs:42` is boundary-crossing (see below) |
| `eggsec::ai` (`client.rs:75,138` `apply_auth(reqwest::RequestBuilder)`) | `Client::builder` (60s), `RequestBuilder` | OOHTTP; `apply_auth` is boundary-crossing |
| `eggsec::distributed` (`io.rs:6-312` rustls server+client, `remote.rs:318,723-1012` `TlsAcceptor` + `lookup_host` ×5) | `rustls::*`, `tokio_rustls::*` | TLS/SRV + RAWNET |
| `eggsec::packet` (`traceroute.rs:1-499` hickory + `lookup_host`) | `TokioResolver`, `lookup_host` | RAWNET |
| `eggsec::dispatch/network.rs:228`, `stress/utils.rs:19` | `lookup_host` | COMPAT (validation) / RAWNET |
| `eggsec::error` (`From<hickory NetError>`, `From<reqwest::Error>`, `From<InvalidHeaderValue>`) | error conversions | COMPAT |
| `eggsec-agent::lifecycle.rs:2,84-98` | `reqwest::Client`, ring init, `Client::builder` (5s) | OOHTTP + COMPAT |
| `eggsec-web-proxy` (`intercept/mod.rs:61-1203` rustls/tokio-rustls MITM; `utils.rs:16-41` ring init + insecure factory; `health.rs:35,91-93` `Client` + `Proxy`; `lib.rs:309` `lookup_host`) | MITM server + health + resolution | TLS/SRV + OOHTTP + RAWNET |
| `eggsec-nse` (10+ Lua libs: `http.rs` sync+async ×4, `httppipeline.rs` ×2, `brute.rs`, `vulns.rs` ×2, `comm.rs`, `upnp.rs` `blocking::get`, `mobileme.rs`, `httpspider.rs`, `elasticsearch.rs`, `helpers.rs` sync+async, `public_api/api.rs` ×3, `cve/{osv,cisa_kev,nvd}.rs`; `dns.rs`/`dnsbl.rs` hickory; `socket.rs:884` `lookup_host`) | `blocking::Client` fan-out, `RequestBuilder` (nvd), `TokioResolver` | OOHTTP + RAWNET |
| `eggsec-python` (`http_client.rs:933-973` builder+redirect+proxy+defaults, `probes.rs:1533-1550`, `network.rs:1189-1193` `lookup_host`; `requests.rs` PyO3 `RequestBuilder` is **not** `reqwest::RequestBuilder`) | Python HTTP/DNS surface | OOHTTP + RAWNET; `requests.rs` = COMPAT |
| TLS provider init (idempotent `install_default`) | `eggsec/src/lib.rs:218-222`, `eggsec-agent/lifecycle.rs:84`, `eggsec-web-proxy/utils.rs:13-22` + `intercept/mod.rs:1203`, `eggsec-nse/lib.rs:21`, `eggsec-daemon/http.rs:692` | COMPAT |

### Boundary-crossing APIs (concrete type crosses a domain boundary)

| API | Location | Note |
|---|---|---|
| `auth_context::apply_auth_context_to_request(reqwest::RequestBuilder, &AuthContextEntry) -> reqwest::RequestBuilder` | `crates/eggsec/src/auth_context/mod.rs:107-110` | **Known Phase A example.** Engine core exposes `reqwest::RequestBuilder` in its public API. Migration checklist item for Phase B (transport-neutral header/cookie application). Guard: none yet — migration pending, do not add a failing ban here |
| `integrations::common::send_with_retry(reqwest::RequestBuilder, ...)` | `crates/eggsec/src/integrations/common.rs:42` | All three tracker clients depend on the reqwest type |
| `ai::client::apply_auth(reqwest::RequestBuilder) -> reqwest::RequestBuilder` | `crates/eggsec/src/ai/client.rs:138` | AI client leaks reqwest through its public interface |
| `fuzzer::advanced::fuzz(&mut self, client: &reqwest::Client)` trait method | `crates/eggsec/src/fuzzer/advanced.rs:30` | Fuzzing trait bound requires `reqwest::Client` |
| `utils::client_pool::ClientPool` public struct | `crates/eggsec/src/utils/client_pool.rs:1` | Pool exposes `reqwest::Client` + `reqwest::Proxy` |

Durable guard added in Phase A (`scripts/check-architecture-guards.sh`
Check 99): `eggsec-runtime`, `eggsec-tool-core`, `eggsec-output`,
`eggsec-ui-model`, `eggsec-daemon-protocol` must not gain
`reqwest`/`rustls`/`tokio-rustls`/`hickory-resolver` dependencies; concrete
`RequestBuilder` must not appear in `auth_context` beyond the single
enumerated compatibility function. The guard permits the existing
`apply_auth_context_to_request` (migration checklist) but fails on any
*additional* concrete-client boundary.

## 4. Migration parity matrix (Reqwest behavior EggSec relies on)

Disposition: `direct` (Eggfetch already covers), `adapter` (thin EggSec-side
shim), `upstream prerequisite` (Eggfetch must gain it first), `remain
specialized` (stays on current stack, e.g. raw TLS/MITM). No Eggfetch
migration occurs in Phase A; this table is the readiness input to Phases B-D.
Do not require Eggfetch parity with unused Reqwest features merely because
they are enabled in Cargo.

| Capability | Current owner | Required behavior | Eggfetch support (Phase A) | Difference / disposition |
|---|---|---|---|---|
| Methods + arbitrary headers | `templates/executor.rs:137-144`, `hunt/authz.rs:254`, `session.rs:503-507`, `nse/http.rs:476,503,879` (GET/POST/PUT/DELETE/PATCH/HEAD/OPTIONS/TRACE) | All 8 methods, arbitrary headers | Unknown — probe in Phase B | `adapter` or `upstream prerequisite` depending on probe |
| Bodies / form / query / JSON | `.json()` (hunt, webhook, jira/github/gitlab, search), `.body()` (templates, c2, nse), `form`+`query` features enabled but no explicit `.form()`/`.query()` call sites | JSON + raw body; form/query features retained but unused | Unknown — probe | `adapter`; consider dropping unused `form`/`query` features only after probe confirms no transitive need |
| Streaming / max body | `response.bytes()` (oast), `.text()`/`.bytes()` elsewhere; no `max_body_size`/`body_limit` anywhere | Reqwest defaults (no anti-decompression-bomb limit configured) | Unknown | `upstream prerequisite` if Eggfetch needs explicit bomb limits; record current absence as accepted risk |
| Cookies | `cookie_store(true)` (hunt, insecure factory, python http_client), header injection (`session.rs:1058`), Set-Cookie inspection (hunt/session, auth/session) | Cookie jar on selected clients + manual header path | Unknown | `adapter` (jar semantics must match) |
| Redirects + limit | `same_host_redirect_policy` (hunt, pool, fuzzer, waf, techdetect), `Policy::limited(5)` (scanner endpoints), `Policy::none()` (python), default-unrestricted elsewhere; `DEFAULT_MAX_REDIRECTS 10`, `waf::MAX_REDIRECTS 5` | Same-host-only for scoped tooling; 5–10 hop caps | Unknown | `adapter` (same-host gate is EggSec policy, must survive migration verbatim) |
| HTTP/1.1 + HTTP/2 | `http2` feature on `eggsec` + `web-proxy`; `h2 0.4` direct dep for proxy demux | H2 for proxy interception path | Unknown | `remain specialized` for MITM/H2 demux; ordinary clients `adapter` |
| SOCKS/HTTP proxies + `NO_PROXY` | `Proxy::all` (stress, loadtest, python, web-proxy health, fuzzer), `Proxy::http` (`utils/http.rs:142`), `basic_auth` (stress, loadtest, health, fuzzer); `NO_PROXY` via reqwest built-in env only; custom SOCKS5 handshake in `web-proxy/socks.rs` | Scheme-routed proxies + basic auth; env `NO_PROXY` inherited | Unknown | `adapter`; custom SOCKS5 stays `remain specialized` |
| Timeouts (per-request / connect / pool / tcp) | Per-request everywhere (5s–300s; AI 60s, OAST 300s, C2 5–10s, WAF detector 15s); connect 5s (stress) / 10s (nse helpers); pool idle 30s + max-idle 20 via `eggsec-core` constants; `tcp_nodelay(true)` standard; keepalive 60s (stress) | Per-request + pool + nodelay required; connect/keepalive only where noted | Unknown | `adapter` (constants stay in `eggsec-core`) |
| TLS roots / custom trust / insecure / identity / SNI | `webpki-roots 0.26` + platform verifier; `danger_accept_invalid_certs(true)` (11 sites) + `danger_accept_invalid_hostnames(true)` (nse helpers, spider); **no** `add_root_certificate` / `identity` / `min_tls_version` / custom SNI / `https_only` anywhere | Insecure mode = cert-verification off only; no custom roots/identity/SNI in use | Unknown | `adapter` for insecure flag; custom-root/identity rows need no parity (unused) |
| Compression / bomb limits | `flate2` workspace dep; no reqwest decompression-bomb limit configured | No explicit limit today | Unknown | Document absence; `upstream prerequisite` only if Eggfetch introduces stricter defaults |
| Blocking call sites | 10 sites, all NSE Lua libs + `recon/asn.rs` + `recon/cve_lookup.rs` + `fuzzer/targets/api.rs` (`blocking::Client`) | Blocking required inside Lua sync closures (`block_on` pattern per guard 52/67) | Unknown | `remain specialized` until async Lua story exists |
| Auth-context injection | `auth_context::apply_auth_context_to_request` + HashMap variant; `Authorization` (nse/http, trackers, ai), `User-Agent` (python, pool), daemon `CLIENT_ID_HEADER` | Header overwrite + cookie replace (see `auth_context.md` gotcha: doc says merge, impl replaces) | Unknown | `adapter` (transport-neutral header/cookie application is an explicit Phase B item) |
| Tracing / metrics | `tracing::warn/debug` around insecure builds; no tower middleware / reqwest tracing layer | Log-only; no metric layer | Unknown | `direct` (keep log call sites) |
| Retry / replayability | `HttpConfig.max_retries` / `retry_delay_ms`, `DEFAULT_MAX_RETRIES 3` / `RETRY_DELAY 1000ms`; retry in `recon/whois.rs`; `replay_flow` in web-proxy MCP | No reqwest-level auto-retry middleware | Unknown | `direct` (EggSec owns retry) |
| Connection metadata for scanners | `request_id` UUID per daemon IPC; status/headers/body carried in scanner results | Status + headers + body + request id | Unknown | `adapter` (envelope fields must survive) |

## 5. Security policy state (Phase A input; closed by Phase F §9 — this table is historical)

| Area | Current state |
|---|---|
| `deny.toml` advisories | `db-path = "advisory-db"`, `unmaintained = "transitive"`, `unsound = "transitive"`; 2 ignores: `RUSTSEC-2025-0057` (fxhash via scraper), `RUSTSEC-2025-0119` (number_prefix via indicatif). Detail: `docs/DEPENDENCY_EXCEPTIONS.md` (reviewed 2026-08-11, review-by 2026-11-07) |
| `deny.toml` licenses | 14 allow-listed: MIT, Apache-2.0 (+LLVM exception), BSD-2/3, ISC, Unicode-3.0, MPL-2.0, CC0-1.0, Unlicense, Zlib, BSL-1.0, 0BSD, CDLA-Permissive-2.0; `confidence-threshold = 0.8` |
| `deny.toml` bans | `multiple-versions = "warn"`, `wildcards = "allow"`, `deny = ["aws-lc-rs"]` (ring-only enforcement; matches `AGENTS.md` TLS rule: `rustls`/`tokio-rustls` with `ring`, `reqwest` with `rustls-no-provider`) |
| `.cargo/audit.toml` | 23 ignored advisory IDs (see file). Canonical per `docs/VERIFICATION.md`: `cargo audit` is a local secondary check only; `deny.toml` is the canonical source and both share the ignore list |
| `Cargo.lock` sources | Registry-only + path: 827 registry, 16 path, 0 git |
| Actions refs + permissions | `ci.yml` (push→main filtered paths + PR→main; `checkout@v4`, `dtolnay/rust-toolchain@stable`, `Swatinem/rust-cache@v2`, `setup-python@v5`; **no `permissions:` key**); `deep-checks.yml` (weekly Sunday + dispatch; adds `taiki-e/install-action@cargo-deny`; **no `permissions:` key**). No pinned SHAs |
| Automated updates | None: no `.github/dependabot.yml`, no `renovate.json*` / `.renovaterc` |
| PR vs scheduled | PR (`ci.yml`): fmt, `--no-default-features` check, clippy (engine+leaf), doc + integration tests, output tests, TUI lib tests, guards. Scheduled/manual (`deep-checks.yml`): `check-full` (deny + domain clippy + feature profiles), `check-features-individual`, MSRV 1.88, portability (macOS/Windows), platform-integration. `cargo deny` and per-feature sweep are **not** PR gates; `cargo audit` is not invoked by any workflow/Makefile target (by design per `VERIFICATION.md`) |

## 6. Acceptance mapping

1. Per-artifact baseline exists → §2 + §1 commands.
2. Every direct Reqwest/Rustls/DNS owner classified → §3.
3. Concrete-client leakage enumerated → §3 boundary table (incl. `apply_auth_context_to_request`).
4. Scope tests cover DNS/redirect/re-resolution/direct-IP/proxy → `crates/eggsec/tests/network_policy_invariants.rs` (12 behaviors).
5. Parity matrix determines readiness → §4 (all rows `Unknown`, no migration in Phase A by design).
6. Deny/Audit/Actions/update policy recorded without changing policy → §5.
7. No HTTP client migration in this phase → no `src/` transport changes; only docs, tests, and Check 99 guard.

## 7. Phase D first increment addendum (2026-09-12)

Status: interfaces + boundaries migrated; backend cutover pending per consumer.
Guard: Check 103 (`scripts/check-architecture-guards.sh`). Plan record:
`plans/network-dependency-phase-d-outbound-client-migration.md` (increment 1).

### 7.1 What migrated

| Workstream | Change | Evidence |
|---|---|---|
| WS1 agent | `eggsec-agent` owns no direct ordinary HTTP stack. `LifecycleManager<T: HttpTransport>` injects transport + `NetworkAuthority` at composition; probes are GET with 5s timeout, `SameHostOnly{5}`, verified TLS. Tests use `RecordingFakeTransport` (deterministic, no I/O). | `cargo tree -p eggsec-agent`: 257 → 153 lines; no `reqwest`/`rustls`/`hyper` in tree. `cargo test -p eggsec-agent` 24 passed (incl. 2 new transport success/failure tests). Manifest: `reqwest`/`rustls` removed, `eggsec-transport` + `url` added. |
| WS2 shared helpers | Removed `auth_context::apply_auth_context_to_request` and `AiClient::apply_auth` compat wrappers (no `RequestBuilder` in shared APIs). Fuzzer 3 call sites translate via canonical `apply_auth_context_to_map` locally (`fuzzer::engine::utils::apply_auth_context_to_builder`); AI call site loops `auth_headers()` directly. | `rg RequestBuilder auth_context/mod.rs` = docs only (removed fn). `cargo test -p eggsec --lib ai/auth_context/fuzzer` green. |
| WS3 NSE capability | New narrow script capability `eggsec-nse/src/http_capability.rs` (pure DTO builder + profile-gated TLS + 8-method allowlist, no I/O). Existing Lua libs unchanged (still `blocking` + `check_network_tcp` preflight); full backend cutover deferred pending async-Lua story + engine `ScopeAuthority` binding. | `cargo test -p eggsec-nse --features nse http_capability` 6 passed (incl. boundary self-scan). `eggsec-nse` gains `eggsec-transport` (types only, no backend). |
| WS4 proxy boundary | New `eggsec-web-proxy/src/outbound.rs` documents side A (intercept/server TLS, never touches client contract) vs side B (outbound probes). Direct-probe builder added; proxy-testing `health.rs` stays on `reqwest` (needs SOCKS/HTTP routing; adapter fails closed on proxy). | `outbound.rs` tests 4 passed (incl. boundary scan). Guard asserts `intercept/`/`socks.rs`/`http_connect.rs` contain no `eggsec_transport`. |
| WS5 pruning | `eggsec-web-proxy` reqwest pruned to `["rustls-no-provider", "socks"]` (dropped unused `json`/`http2`/`form`/`query`/`blocking`/`cookies`). Adapter features unchanged (still minimal: `http1` + `tls-rustls` + `proxy`-for-SNI only). Engine `form`/`query`/`blocking`/`cookies`/`socks`/`http2` retained (all have call sites — corrects §4's "no explicit `.form()`/`.query()`" note: `waf/detector/compare.rs` uses `.query()`, `fuzzer/payloads/oauth.rs` uses `.form()`). | `cargo check -p eggsec-web-proxy --features web-proxy` green. `cargo tree -e features` shows no cookie/multipart/compression/http3 in proxy reqwest. |

### 7.2 Remaining owners and dispositions (WS6)

`cargo tree -i` after increment 1 (see §1 commands to reproduce):

| Owner | Dependency | Disposition |
|---|---|---|
| `eggsec` engine | `reqwest` (json/socks/http2/form/query/blocking/cookies), `rustls`, `tokio-rustls`, `hickory-resolver`, `webpki-roots` | **Pending per-subsystem.** OOHTTP sites (§3) still dispatch on reqwest; `distributed/io.rs` + `waf/bypass/smuggling.rs` own raw TLS (`TLS/SRV` + `RAWNET`, remain specialized); `packet`/`recon`/`scanner` DNS stays on hickory/`lookup_host` (`RAWNET`, canonical resolver impl is engine `HostResolver`, not the transport). Next: migrate webhook/notifier + tracker clients one subsystem at a time with fake-based parity tests. |
| `eggsec-nse` | `reqwest` (json/blocking), `rustls`, `hickory-resolver`; `openssl`/`native-tls` (feature-gated `nse`), `ssh2` (`nse-ssh2`) | **Remain specialized (blocking + compat).** 10+ Lua libs use `blocking::Client` inside sync closures (guard 52/67 pattern; async-Lua story pending). `dns.rs`/`dnsbl.rs` hickory + `socket.rs` `lookup_host` are `RAWNET`. `openssl`/`native-tls` (feature-gated) + `des` serve `sslcert`/`openssl` protocol libs (compatibility island, never for ordinary HTTP aesthetics). New code uses `http_capability.rs`. |
| `eggsec-web-proxy` | `reqwest` (minimal: rustls-no-provider + socks), `rustls` + `tokio-rustls` (MITM `TlsAcceptor`), `rcgen`, `h2`/`http`/`prost` (optional, interception) | **Split.** Side A (intercept/server TLS/H2/WS/gRPC, `lookup_host` RAWNET) remains. Side B proxy-routing probes remain on minimal reqwest (adapter has no authorized proxy routing). `reqwest` removed only when proxy routing gains a scoped backend or probes go direct. |
| `eggsec-python` | `reqwest` (HTTP/DNS surface) | **Pending.** Python `http_client`/`probes`/`network` OOHTTP + `lookup_host` RAWNET; migrate behind the same transport seam with `EGGSEC_ALLOW_LOOPBACK_FIXTURE=1` fixtures. |
| `eggsec-transport-eggfetch` | `eggfetch-core 0.1` (http1 + tls-rustls + proxy-for-SNI; no http3/cookies/multipart/compression), `bytes`/`http`/`url`/`tracing` | **Intentional owner.** Sole concrete-backend binding; still no production consumers (agent/NSE/proxy depend on `eggsec-transport` types only; engine dev-dep for parity tests). Production wiring happens per consumer with focused parity tests (guard 102). |
| `eggsec-daemon` | `reqwest` in `#[cfg(test)]` only + ring init | **Compat (tests only).** No production HTTP stack. |
| Transitive | `hyper`/`hyper-rustls`/`rustls-platform-verifier` (via reqwest/eggfetch), duplicate `webpki-roots 0.26/1.0` | **Accepted duplicates.** Both stacks needed for separate roles (ordinary reqwest clients vs pinned eggfetch backend); `deny.toml` bans `aws-lc-rs` (ring-only) unchanged. |

No domain-facing shared API exposes `reqwest`/`eggfetch` builder types after
increment 1: `apply_auth_context_to_request` and `AiClient::apply_auth`
removed; `integrations::common::send_with_retry` is `pub(crate)` (compat,
not domain-facing); `fuzzer::advanced::fuzz(&reqwest::Client)` trait and
`utils::client_pool::ClientPool` remain as documented pending full backend
migration (next increments), with no *new* `RequestBuilder` boundary allowed
(guard 103).

## 8. Phase E closure addendum (2026-09-13)

Status: egress decided (all rejected), engine default empty, Tokio narrowed,
no new crates. Guards: Checks 104–108 (`scripts/check-architecture-guards.sh`).
Decision records: `architecture/egress_reuse_decision.md` (WS1) +
`architecture/capability_segregation.md` (WS2–WS4). Plan record:
`plans/network-dependency-phase-e-egress-and-capability-segregation.md`
(marked Executed).

### 8.1 Egress (WS1)

`eggress-uri 1.0.6` / `eggress-routing 1.0.6` / `eggress-core 1.0.6` evaluated
in an isolated probe (checksums in the decision record). URI alone is 10
packages (net-new 1); routing is 62 packages (net-new ≥5: routing + core + URI
+ `ipnet v2` — a second CIDR family beside workspace `ipnetwork` — +
`fastrand`). `eggfetch-core` has no `eggress` edge (`cargo tree -i` no match),
so adoption adds conversion layers without deleting code. `ProtocolSpec` (14
variants) imports 11 unrelated protocols into HTTP/SOCKS policy; EggSec
`ProxyIntent` (`Direct`/`Http`/`All` with endpoint≠ultimate) plus fail-closed
userinfo rejection is stricter than parse-and-redact. Routing would add a
second policy language beside `Scope`/`TargetScope` + `NetworkAuthority`
(EggSec authorization stays authoritative; route selection ≠ authorization).
Disposition: **all rejected**, no workspace edge, lockfile unchanged in this
respect (workspace totals §8.4).

### 8.2 Library default (WS2)

`crates/eggsec/Cargo.toml` `default = ["cli"]` → `default = []`. `cli`
(`dep:clap` + `dep:clap_complete`) retained for process hosts: binary shell
and TUI depend on the engine with explicit `features = ["cli"]`; daemon
`full-executor = ["dep:eggsec", "eggsec/cli"]` (its `runtime_bridge` use is
cli-gated); Python already builds with `default-features = false`.
`make check` now runs `cargo check -p eggsec`, `cargo check -p eggsec-cli`,
and `cargo check -p eggsec-cli --no-default-features` explicitly; clippy lints
the engine both empty-default and with `cli`; integration suites request `cli`
explicitly (`--features rest-api,cli`); `feature_matrix.rs` +
`check-feature-docs.py` + `docs/FEATURE_MATRIX.md` + `docs/extending/features.md`
assert `[]`.

### 8.3 Tokio (WS3)

Workspace baseline was 11 features (`rt-multi-thread`, `net`, `sync`, `time`,
`macros`, `fs`, `process`, `io-util`, `io-std`, `signal`, `test-util`); now
`tokio = { version = "1", default-features = false }` with per-crate
`features = [...]`. `test-util` enabled nowhere (no paused-time sites).
DTO crates (`core`, `tool-core`, `ui-model`, `daemon-protocol`) carry no Tokio;
`output` keeps `io-util` only; `transport`/`eggfetch` keep Tokio in dev-deps
only; engine keeps the broadest set (all except `test-util`); process hosts
keep `rt-multi-thread` + `signal` as needed. Full feature matrix re-run
(unification cannot hide missing declarations).

### 8.4 Workspace totals (Phase E)

`cargo metadata --locked` reports 591 packages (was 587 in §2; delta is prior
Phase C/D additions already in the tree, not Phase E — Phase E adds zero
packages: no `eggress`, no new crates). `cargo tree -p
eggsec-transport-eggfetch --edges normal` = 216 lines;
`cargo tree -p eggsec-transport --edges normal` = 87 lines (both unchanged by
Phase E by design).

### 8.5 Capability extraction (WS4)

All rejected (see `capability_segregation.md` for the one-page records):
`eggsec-net` (no clean boundary; transport facts vs engine verdicts stay
split), web-client split (clients already live in the engine; non-proxy builds
already avoid `rcgen`/`h2`/WS/gRPC via optional `web-proxy`), evidence crypto
(webhook HMAC vs bundle HMAC are unrelated policies; centralizing saves one
dep line at the cost of a false abstraction). No `crates/eggsec-net`,
`crates/eggsec-web-client`, `crates/eggsec-evidence`/`*-signing`.

*Last verified against source: 2026-09-13*

## 9. Phase F supply-chain hardening addendum (2026-09-13)

Status: dependency and workflow supply-chain policy enforceable on pull
requests. Guards: Checks 109–112
(`scripts/check-architecture-guards.sh`). Plan record:
`plans/network-dependency-phase-f-supply-chain-ci-hardening.md` (marked
Executed).

### 9.1 Dependency policy is a PR gate (WS1/WS8)

`cargo deny check` moved from scheduled/manual-only (`deep-checks.yml` +
local `make check-full`) into the required PR contract:

- New `make check-deps` target runs `cargo deny --workspace
  --all-features check` (+ per-check advisories/bans/licenses/sources).
  Fails closed when `cargo-deny` is absent (never silently skips).
- `make check` runs `$(MAKE) check-deps` before clippy/tests/guards, so the
  `ci.yml` `rust` job enforces policy on every PR/push.
- New independent `ci.yml` `dependency-policy` job runs the same
  `make check-deps` as a dedicated gate signal.
- `make check-full` no longer runs `cargo deny check` directly (covered via
  `check`); it adds domain lint + feature profiles only.
- `deny.toml [graph] all-features = true` so even bare `cargo deny check`
  covers optional closures. Default-feature-only checks hid real advisories
  (lopdf via `printpdf`, sqlx/rsa via `db-pentest`, webpki via `tiberius`);
  the all-features gate re-surfaced 10 errors, all now documented in
  `docs/DEPENDENCY_EXCEPTIONS.md` (see §9.2).

### 9.2 Deny canonical, audit removed (WS2)

Cargo Deny is the canonical policy source. `.cargo/audit.toml` (24 stale
ignores, most for already-fixed advisories, no owner metadata) was deleted;
`cargo audit` is diagnostic/non-canonical and runs in no workflow or Makefile
target. Historical ignores are never retained "just in case."

`docs/DEPENDENCY_EXCEPTIONS.md` (reviewed 2026-09-13, review-by 2026-12-12)
now carries 12 advisory ignores with full fields (ID, path, feature, API use,
exploitability, compensating control, owner, created/review-by, blocker):

- Unmaintained (transitive, no upstream fix): `RUSTSEC-2025-0057` (fxhash via
  scraper), `RUSTSEC-2025-0119` (number_prefix via indicatif),
  `RUSTSEC-2024-0384` (instant via notify — documented before but missing from
  `deny.toml`, now added), `RUSTSEC-2024-0436` (paste via sqlx-macros,
  build-time only), `RUSTSEC-2025-0134` (rustls-pemfile via tiberius),
  `RUSTSEC-2026-0192` (ttf-parser via printpdf, generation-only).
- Vulnerabilities (blocked upgrades): `RUSTSEC-2026-0187` (lopdf, printpdf
  0.7→0.12 breaking bump; generation-only so unparsable input never reaches
  `load*`), `RUSTSEC-2023-0071` (rsa, no fixed upgrade; lab-only),
  `RUSTSEC-2026-0104`/`-0099`/`-0098` (rustls-webpki via tiberius 0.12.3
  latest; lab-only), `RUSTSEC-2024-0363` (sqlx 0.8.0; ≥0.8.1 blocked by
  `libsqlite3-sys` links conflict with daemon `rusqlite`).
- License: `MIT-0` added to the global allow list (OSI-approved permissive);
  `auto_generate_cdp` (GPL-3.0-or-later, build-time codegen for
  `headless_chrome`) allowed via per-crate exception with owner/review-by.
- Yanked `libssh2-sys` 0.3.2 stays a warning (upgrade signal, not suppression).

Phase E "resolved" claims for lopdf/webpki/sqlx/paste/ttf-parser were
default-closure artifacts and are superseded (documented in the exceptions
doc); genuinely fixed IDs (rand, crossbeam-epoch, idna, lettre, reqwest, pyo3,
quick-xml, anyhow) stay resolved under the all-features gate.

### 9.3 Sources fail closed, wildcards denied (WS3)

- `[sources] unknown-registry = "deny"`, `unknown-git = "deny"`,
  `required-git-spec = "rev"`. `Cargo.lock` verified registry-only + path (no
  git/alternate-registry deps); any new git dep fails until pinned to a full
  commit rev and documented with owner/removal criteria.
- `wildcards = "deny"` (was `"allow"`). No `version = "*"` existed; 21
  workspace path deps without `version` (in `eggsec-tui`, `eggsec-cli`,
  `eggsec-python`) were pinned to explicit `version = "0.1.0"` so the lint
  fires only on real wildcards. `multiple-versions = "warn"` retained.

### 9.4 Actions pinned, least privilege, deterministic toolchain (WS4/WS7)

All `uses:` in `ci.yml` + `deep-checks.yml` pinned to full-length commit SHAs
with version comments: `actions/checkout` v4.4.0, `dtolnay/rust-toolchain`
stable/master HEADs, `Swatinem/rust-cache` v2.9.2 (peel commit),
`actions/setup-python` v5.6.0, `taiki-e/install-action` cargo-deny ref,
`actions/dependency-review-action` v4.8.3. `cargo-deny` installs at pinned
`0.19.0` via the SHA-pinned install-action (both `rust` and
`dependency-policy` jobs, plus deep `deep` job). Top-level
`permissions: contents: read` in both workflows, repeated per job; no write
permissions anywhere (no release/publication jobs exist — releases are
manual). Normal CI tracks documented stable; MSRV 1.88 stays explicit
(`msrv` job + `make check-msrv`).

### 9.5 Maintenance automation (WS5/WS6)

- `.github/dependabot.yml` (new): weekly Monday updates for `cargo` (grouped
  patch+minor, limit 10, `chore(deps)`) and `github-actions` (limit 10,
  `chore(actions)`). No auto-merge — updates must pass the full CI/security
  contract.
- `ci.yml` `dependency-review` job (PR-only, read-only, SHA-pinned v4.8.3):
  fails on `moderate`+ vulnerabilities; Cargo Deny/RustSec stays authoritative
  for Rust advisories. No separate license allowlist (deny.toml owns license
  policy; contradictory lists are a violation). No `pull-requests: write`.
  Hosted-platform limitation: none encountered — the job is standard for
  public repos; if it ever cannot run, that is recorded here, never compensated
  by weakening Deny.

*Last verified against source: 2026-09-13 (Phase F)*
