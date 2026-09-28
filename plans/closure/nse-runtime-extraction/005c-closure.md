# NSE Runtime Extraction Milestone 005C — Closure Status

Status: conditionally closed — corrective pass required

Source implementation plan:

- `plans/implementation/nse-runtime-extraction/005-http-provider-eggsec-adapter.md`

Source subsystem roadmap:

- `plans/subsystems/nse-runtime-extraction-roadmap.md#milestone-005--host-provider-inversion-and-portability-hardening`

Repository baselines reviewed: Eggsec `3b951d0e`; standalone `89290f9` (base `b3c43b8`, the accepted 005D tip; plan baseline `854f153f56d1abc929d9abd255f0606759342f81`).

Implementation commits or pull requests:

- Standalone `89290f9` — "nse: add M005C runtime-neutral HTTP provider and family migration"; branch `m005c-http-provider-eggsec-adapter` on `eggstack/eggsec-nse` (stacked on the 005D tip). Contains the HTTP contract/native/broker (`src/providers.rs`), `NseHostServices` http domain, migrated `http`/`httppipeline`/`comm.tryssl`/`brute.http_auth`/`vulns`-NVD/`upnp.get_devices` paths, `tests/http_provider_tests.rs` (18 tests), M005C boundary guards, `docs/PROVIDERS.md` HTTP section.
- Eggsec main: `nse_http_capability.rs` refactor only (additive `build_scoped_request_with_tls_policy` core; no behavior change; compiles against pinned `eggsec-nse 0.1.0`). No production dispatch rewiring.
- Eggsec branch `m005c-eggsec-http-adapter` (from the 005C main tip): engine-owned `NseHttpTransportProvider` adapter (`crates/eggsec/src/nse_http_provider.rs`, 14 tests) implementing the runtime trait over `HttpTransport` + injected `NetworkAuthority`. Staged off-main because it requires the unreleased runtime provider contract (see §11 disposition).

## 1. Executive finding

Milestone 005C is complete. The standalone runtime exposes a narrow HTTP provider trait with a native reqwest backend, runtime-owned request/response DTOs, typed errors preserving the timeout/connection/request classification, and a capability-aware broker owning check → preflight → provider → accounting. The HTTP family (`http`, `httppipeline`, `comm.tryssl`, `brute.http_auth`, `vulns` NVD lookups, `upnp.get_devices`) is provider-backed with no reqwest outside `providers.rs`; process-global HTTP clients and TLS flags are gone (deprecated no-op shims); Lua shapes are preserved. The Eggsec adapter implements the runtime contract over scoped transport with injected authority, proven by mapping/authority tests: in-scope requests reach transport with the same authorized target, out-of-scope/cross-host fail closed with no native fallback, and script TLS intent cannot escalate. Production dispatch is deliberately NOT rewired (no scope exists at NSE dispatch; see §6/§11).

## 2. Requirement-to-evidence matrix

| Requirement | Evidence | Result | Notes |
|---|---|---|---|
| Runtime-neutral HTTP contract | `NseHttpMethod` (8 verbs, fail-closed parse), `NseHttpRequest`/`NseHttpResponse`, `NseHttpError` (denied/cancelled/timeout/connection/request + legacy `reason` values), `NseHttpProvider` (sync, object-safe) | pass | No reqwest/Eggsec types in signatures. |
| Native reqwest provider preserves defaults | `NativeHttpProvider` (per-instance pooled clients keyed by TLS/timeout); native loopback GET/POST round-trips + error-classification tests green | pass | Redirects follow reqwest defaults (legacy parity). |
| `http.rs` provider-backed, no global static policy | All 11 request entry points brokered; 4 static clients + 2 global flags removed; Lua shapes asserted via mock end-to-end | pass | `set_accept_invalid_*` kept as deprecated no-ops (had no callers). |
| Adjacent sites migrated or deferred with evidence | Migrated: `httppipeline` go/queue, `comm.tryssl`, `brute.http_auth`, `vulns` NVD ×2, `upnp.get_devices`. Deferred: brute-TCP/SSDP-socket surfaces (non-HTTP, out of scope), `nmap` (no direct HTTP), protocol libs (005E audit) | pass | Per-site parity notes in §7 + PROVIDERS.md. |
| Broker owns capability/cancellation/accounting | `broker_http_request` (host-identity check → preflight → provider → byte accounting); CiSafe-denied + cancelled zero-call tests; per-op accounting test | pass | HTTP gains accounting (previously uncounted). |
| Eggsec seam implements runtime provider, standalone stays independent | `NseHttpTransportProvider` on a feature branch; standalone has zero Eggsec imports (boundary script); adapter source-scan test bans concrete clients | pass | Staging rationale in §11. |
| Adapter uses only existing approved authority | Constructor takes `Arc<dyn NetworkAuthority>`; tests bind `OwnedScopeAuthority` over real `Scope` objects; in-scope/out-of-scope/deny-all tests | pass | No authority manufactured from URLs. |
| Out-of-scope/cross-host fail closed, no native fallback | `out_of_scope_fails_closed_without_transport_contact` (hop_count 0); adapter has no reqwest path (compile-guaranteed + scan test) | pass | SameHostOnly redirect posture asserted on mapped requests. |
| Local HTTP compatibility green | Full standalone suite 629 passed (baseline 611 + 18 new); Eggsec NSE suites green | pass | — |
| Static guards prevent regression | M005C reqwest ban in 6 migrated libs + broker presence; Eggsec adapter scan test; both guard suites pass | pass | — |
| Release/adoption disposition | §11: YES, a release/adoption step is required (adapter staged on branch) | pass | — |

## 3. Production implementation evidence

Standalone (`eggstack/eggsec-nse`, commit `89290f9`):

- `src/providers.rs` (+~430 lines): HTTP DTOs, `NseHttpProvider`, `NativeHttpProvider`, `MockHttpProvider`, `CountingHttpProvider`, `broker_http_request`.
- `NseHostServices`: `http` field; `new()`/`new_full` preserved (HTTP defaults native); `with_http` + `http()`; clone-shares test extended.
- `src/lib.rs`: HTTP provider/broker re-exports.
- `src/executor_core.rs`: `http`/`httppipeline`/`brute`/`vulns`/`upnp` register via `*_with_services` (`comm` already threaded).
- `http.rs`: 11 request entry points brokered (get/post/put/delete/head/options/request/post_host/put_data/async ×3); pure helpers untouched; TLS intent profile-gated.
- `httppipeline.rs`: `go`/`queue` brokered per request (keeps per-request timeout/header/body support).
- `comm.rs`: `tryssl` is an HTTPS GET through the broker (denied shape preserved).
- `brute.rs`: `http_auth` brokered with in-library Basic header (TCP helpers untouched).
- `vulns.rs`: NVD lookups brokered (capability-gated, verified TLS); `register_vulns_library` gains the `capability_ctx` parameter (single in-repo caller updated); JSON parsing untouched.
- `upnp.rs`: `get_devices` brokered (30s bound replaces unbounded `blocking::get`); SSDP sockets untouched.
- New `tests/http_provider_tests.rs` (18 tests).
- `scripts/check-boundaries.sh`: M005C guard section (incl. M005B comm-pattern tightening for the new DTO field name).
- `docs/PROVIDERS.md`: M005C contract/coverage/TLS/inventory sections.

Eggsec main (this commit): `nse_http_capability.rs` gains `build_scoped_request_with_tls_policy` (existing `build_scoped_request` delegates; 6 existing tests green; doc status updated). No behavior change.

Eggsec branch `m005c-eggsec-http-adapter`: `crates/eggsec/src/nse_http_provider.rs` (`NseHttpTransportProvider`, sync bridge over ambient/ephemeral runtime, non-escalating TLS, transport-error mapping) + 14 tests (mapping, in-scope/out-of-scope/deny-all/allow, TLS non-escalation ×2, invalid-URL, boundary scan).

## 4. Verification executed

### Commands run

```bash
# Standalone at 89290f9
cargo fmt --all --check
./scripts/check-boundaries.sh
cargo check --features nse
cargo test --features nse
cargo check --features nse-ssh2
cargo check --features nse,sandbox
cargo clippy --all-targets --features nse
cargo +1.89.0 check --locked --features nse
cargo package
cargo test --features nse --test http_provider_tests

# Eggsec main (pinned eggsec-nse 0.1.0; no override)
cargo check -p eggsec --features nse,cli
cargo test -p eggsec --features nse,cli --lib nse_http_capability
cargo test -p eggsec --features nse,cli --test nse_bridge_tests --test nse_integration_tests --test nse_real_scripts --test nse_tests
cargo test -p eggsec-tui --features nse --lib
cargo test -p eggsec-python --features nse
cargo fmt --all -- --check
cargo check --workspace --no-default-features
make test-architecture-guards
make check-deps

# Eggsec branch m005c-eggsec-http-adapter (temporary [patch.crates-io] to the
# 005C standalone tip; Cargo.toml + Cargo.lock restored afterwards)
cargo check -p eggsec --features nse,cli
cargo test -p eggsec --features nse,cli --lib nse_http
cargo test -p eggsec --features nse,cli --test nse_bridge_tests --test nse_integration_tests --test nse_real_scripts --test nse_tests
cargo clippy -p eggsec --features nse,cli
cargo fmt -p eggsec -- --check
```

### Results

- Standalone: fmt ok; boundaries ok (incl. new M005C guards); nse + nse-ssh2 + nse,sandbox checks ok (0 errors); `cargo test --features nse` 629 passed, 1 ignored (26 suites: baseline 611 + 18 new); clippy ok (0 errors; no new-code findings); MSRV 1.89 ok; `cargo package` ok on the clean tree.
- Focused: `http_provider_tests` 18 passed (incl. loopback HTTP fixture GET/POST + error classification + contention + all Lua family shapes).
- Eggsec main: check ok; capability tests 6 passed; NSE suites 237 passed (4 suites); TUI lib 903 passed; Python 231 passed; fmt ok; no-default workspace check ok; architecture guards ok; `cargo deny` sources/advisories/bans/licenses ok.
- Eggsec branch (override active): check ok (0 errors); adapter tests 14 passed; NSE suites 237 passed; clippy ok (0 errors); fmt ok. Override fully reverted afterwards (tree verified clean).
- Not run (documented): full `make check` (workspace-wide slow suites; all affected-surface targets above are green); Windows CI (standalone; runs on push); `cargo test --tests` for unrelated `phase_d_protocol_agent` (pre-existing `eggsec::agent` feature-combo failure, untouched by this slice).

## 5. Invariant review

- Standalone has zero Eggsec dependencies (boundary script).
- Eggsec remains authorization/scope authority (adapter takes injected authority; `ScopeAuthority`/`OwnedScopeAuthority` unchanged).
- HTTP contracts use runtime-owned DTOs (no reqwest/Eggsec types in signatures).
- Insecure TLS profile-gated on both sides (library intent + adapter construction flag; scripts cannot escalate either path — tested).
- Same-host redirect/scoped authority not weakened (adapter asserts `SameHostOnly{5}` + `Direct` proxy on mapped requests; native keeps reqwest defaults = legacy).
- Method/result compatibility intact for supported methods (8-verb matrix; Lua shapes asserted).
- Denial/cancellation/accounting surround actual requests (zero-call proofs; typed errors).
- No frontend dependency on `eggsec-nse` (no frontend changes at all).
- Native-default standalone behavior available (all old constructors/registrations preserved as shims; Eggsec manual dispatch unchanged).
- No proxy/cookie/retry redesign (redirect/proxy posture pinned, not reworked).

## 6. Failure and recovery review

- Provider failures map to legacy Lua shapes (`status`/`body`/`headers`/`location`/`https`/`version`; `status: 0` + `error` + `reason` on failure; pipeline/tryssl/brute/upnp/vulns shapes preserved per-site).
- Cancellation aborts before provider invocation (typed `Cancelled`); in-flight adapter calls are bounded by the request timeout policy (documented sync-bridge constraint: async non-blocking contexts need a future async provider).
- Timeouts remain bounded everywhere (`upnp` gains the 30s bound it lacked).
- Three fix iterations during implementation, all root-caused and locked: (1) brace imbalance from a dropped `with_services` signature (httppipeline) and a legacy double-close (vulns) — both caught by compile checks; (2) hand-written NVD mock JSON malformed — rewritten readably; (3) Eggsec fake `AllowAll`/`DenyAll` are test-private — tests bind `OwnedScopeAuthority` over real `Scope` objects instead (stronger evidence).
- Tooling note (repeat from 005D): file payloads above ~16KB truncate in transit; applied in verified sub-8KB chunks, compile-checked per chunk.
- Patch probe mutated Eggsec `Cargo.toml`/`Cargo.lock`; both restored, tree verified clean (`git status` clean before commit).

## 7. Migration and compatibility review

- Additive only on both repos (deprecated shims retained; no signatures removed except `register_vulns_library`, which gains the `capability_ctx` parameter its single in-repo caller already holds — documented as the one Rust API change).
- Intended deltas (documented + tested): profile-gated TLS intent replaces inert global flags; `vulns` NVD access capability-gated; async HTTP fns share the sync broker (bounded); `upnp` bound; per-request pipeline options honored; HTTP accounting added; interface-entry normalization carried from 005D.
- `register_vulns_library(lua)` → `(lua, capability_ctx)` is the only Rust signature change in the slice (0.1.0 era; single in-repo caller; noted for the release notes).
- No release/publication from this slice; Eggsec main stays on crates.io 0.1.0; manual NSE dispatch keeps the native provider.

## 8. Security review

- Broker denies via runtime capability before provider invocation (CiSafe + restricted-scope Lua tests with zero provider contact).
- Adapter enforces Eggsec authority at every transport checkpoint via the injected authority (fake records prove checkpoint order + approved addresses).
- No authority fabrication: constructor requires an existing authority; invalid URLs fail before transport; cross-host redirects stay same-host-only.
- TLS non-escalation is structural on both sides (library intent from profile; adapter ignores DTO flag).
- Redirect/proxy posture unchanged and asserted (`SameHostOnly{5}`, `Direct`).
- No secrets handled; recorded hops assert presence-only header logging (no values).
- Native reqwest confined to `providers.rs` (guard); adapter names no concrete clients (scan test).

## 9. Documentation and operations

- Standalone: `docs/PROVIDERS.md` M005C sections (sequence, coverage table, TLS alignment, deltas, Eggsec adapter summary, inventory with regeneration command, guards); rustdoc on all new public APIs; guard comments.
- Eggsec main: `nse_http_capability.rs` doc status updated (seam ready for the staged adapter).
- Eggsec: this closure record; plan status `implemented`; registry + subsystem roadmap updates (005C closed; 005E ready).
- Operator impact: none (no config/CLI/TUI/Python change).

## 10. Migrated/deferred reqwest inventory (machine-auditable)

- Migrated, guard-enforced reqwest-free: `http.rs`, `httppipeline.rs`, `comm.rs` (tryssl brokered; zero reqwest remains), `brute.rs` (HTTP-auth only), `vulns.rs` (NVD only), `upnp.rs` (description fetch only).
- Natives: `providers.rs` only (reqwest blocking backend + client cache).
- Deprecated shims: `http.rs::set_accept_invalid_certs/hostnames` (no callers, no behavior).
- Deferred (non-HTTP or non-shared/core, per plan scope): brute-TCP helpers, upnp SSDP sockets, `nmap` (no direct HTTP), all other protocol libraries — covered by the 005E source audit.
- Regeneration: `rg -n -e 'reqwest' src/libraries/http.rs src/libraries/httppipeline.rs src/libraries/comm.rs src/libraries/brute.rs src/libraries/vulns.rs src/libraries/upnp.rs` (must be empty; comment-only mentions fail the guard — the one doc mention was reworded for this reason).

## 11. Roadmap disposition and release/adoption requirement

Milestone 005C closed; the parent track is unblocked:

- **005E (provider coverage qualification): GO** — all slice contracts, guards, and inventories are closed (005A/005B/005C/005D). Status → ready for handoff.
- **Release/adoption step: REQUIRED before adapter activation.** The Eggsec adapter (`m005c-eggsec-http-adapter`) implements the unreleased runtime provider contract and cannot compile against pinned `eggsec-nse 0.1.0`. It is staged on a feature branch (verified with a temporary path override: check + 14 adapter tests + 237 NSE suite tests + clippy + fmt, all green; override reverted). Activation requires, in order: (1) standalone review/merge of the `m005A–D` branch stack and a `0.2.0` (or current-convention) release; (2) an Eggsec adoption bump (004-style: dependency version + lockfile + requalification); (3) merge of the adapter branch; (4) production dispatch threading once the NSE enforcement prerequisite (`OperationMetadata`/scope path for NSE) exists — until then manual dispatch keeps the native provider by design, not by omission.
- No corrective plan required for 005C. The prerequisite integration note above is input to 005E/release planning, not a new blocker: the adapter is complete and qualified, only its activation is sequenced.


## Post-closure operational finding — M005 landing/CI corrective pass

Subsequent repository-level review found that this implementation/qualification evidence was accepted while the standalone changes remained on the stacked feature branches rather than `eggstack/eggsec-nse/main`. The relevant branch evidence for this slice is `m005c-http-provider-eggsec-adapter (standalone stack commit 89290f95d493e557dd0f07e64818cb6ae10e39a3; Eggsec adapter remains separately staged)`. Standalone `main` is still `854f153f56d1abc929d9abd255f0606759342f81`, so the provider implementation described above is not yet canonical repository state.

The hosted GitHub Actions runs for the M005 stack are also red on Linux/macOS because `scripts/check-boundaries.sh` invokes `rg` but the workflow does not provision ripgrep. The latest cumulative 005E run `36476950685` passes MSRV, SSH runtime, and Windows but fails the Linux/macOS Rust jobs at the missing-`rg` boundary-check prerequisite.

This does not invalidate the local/focused implementation evidence recorded above, but it invalidates unconditional operational closure. The controlling corrective plan is `plans/implementation/nse-runtime-extraction/005-provider-stack-landing-ci-corrective.md`. This closure returns to `closed` only after the corrected cumulative stack has fully green branch CI, is landed on standalone `main`, and the merged main SHA has fully green post-merge CI with a corrective addendum tying this record to that SHA.
