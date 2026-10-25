# CI Architecture Guards

CI architecture guards preserve the enforcement, registry, metadata, feature, and documentation invariants established across Phases 1–14 of the architecture extensibility work. They stop regressions at pull-request time without making the workflow brittle, slow, or dependent on platform-specific optional features.

For the full verification contract (merge readiness vs release readiness, which checks are mandatory vs optional), see [`VERIFICATION.md`](VERIFICATION.md).

## Required Fast PR Checks

These checks run on every pull request and push to `main`. They cover core architecture invariants and should complete quickly.

| Check | Command | Purpose |
|-------|---------|---------|
| Formatting | `cargo fmt --all --check` | Code style consistency |
| No-default build | `cargo check --workspace --no-default-features` | Workspace compiles without optional features |
| Dependency policy | `make check-deps` (`cargo deny --workspace --all-features check`) | Advisories, licenses, bans, sources over the full feature closure |
| Clippy | `make clippy` (engine lib + leaf crates, `-D warnings`) | Code quality on engine and leaf crates |
| Package tests | `cargo test -p eggsec --features rest-api,cli --tests --no-fail-fast` | All integration tests (MCP, REST, enforcement, dispatch, scanner, fuzzer, agent, NSE, and more) |
| Report envelope | `cargo test -p eggsec-output --tests` + `cargo test -p eggsec-report-model --tests` | Output rendering tests + model contract roundtrip |
| Policy kernel | `cargo test -p eggsec-policy --tests` | Deterministic authorization semantics (`eggsec-policy` is a leaf) |
| Knowledge corpus | `cargo test -p eggsec-{service-db,secrets,payloads,udp-scan} --tests` | Corpus data (service tables, secret patterns, payloads, UDP verdicts) — data-only leaves, engine owns I/O |
| Architecture drift | `bash scripts/check-architecture-guards.sh` | Static grep checks for stale terminology and bypass patterns (requires ripgrep) |

In CI these run as three jobs in `ci.yml`: `rust` (full `make check`,
including `check-deps`), `dependency-policy` (independent `make check-deps`
gate signal), and `dependency-review` (PR-only moderate+ vulnerability gate).
Python changes additionally run the `python` job (`make check-python`).

### Local Reproduction

Run these before pushing to match CI:

```bash
make check
```

Alternatively, run the individual commands:

```bash
cargo fmt --all --check
cargo check --workspace --no-default-features
make clippy
cargo test -p eggsec --features rest-api,cli --tests --no-fail-fast
cargo test -p eggsec-output --tests
cargo test -p eggsec-report-model --tests
cargo test -p eggsec-policy --tests
cargo test -p eggsec-service-db --tests
cargo test -p eggsec-secrets --tests
cargo test -p eggsec-payloads --tests
cargo test -p eggsec-udp-scan --tests
bash scripts/check-architecture-guards.sh
```

> **Note**: The static guard script requires [ripgrep](https://github.com/BurntSushi/ripgrep) (`rg`). Install it locally before running: `cargo install ripgrep` or use your system package manager.

## Feature-Profile Compile Guards

Representative feature profiles are checked in the optional `deep-checks.yml` workflow and locally via `make check-full`. These are `cargo check` only (no test execution). They are not required for every PR.

| Profile | Command | Category |
|---------|---------|----------|
| tool-api + rest-api | `cargo check -p eggsec --features tool-api,rest-api` | Protocol adapter |
| grpc-api | `cargo check -p eggsec --features grpc-api` | Protocol adapter |
| db-pentest | `cargo check -p eggsec --features db-pentest` | Domain capability |
| db-pentest MCP | `cargo check -p eggsec --features db-pentest-mcp,tool-api,rest-api` | Domain + protocol |
| mobile | `cargo check -p eggsec --features mobile` | Domain capability |
| mobile-dynamic | `cargo check -p eggsec --features mobile-dynamic` | Domain (platform-sensitive) |
| web-proxy | `cargo check -p eggsec --features web-proxy` | Domain capability |
| web-proxy MCP | `cargo check -p eggsec --features web-proxy-mcp,tool-api,rest-api` | Domain + protocol |
| c2 MCP | `cargo check -p eggsec --features c2-mcp,tool-api,rest-api` | Domain + protocol |
| broad TUI | `cargo check -p eggsec-tui --features db-pentest,web-proxy,c2` + lib tests | Frontend builders (fast drift detection; `full` stays in Deep Checks) |

> **Note**: `mobile-dynamic` may require platform-specific dependencies. If it fails in CI due to missing system deps, it should be documented with an issue reference rather than silently ignored.

## Optional/Deep Checks

These checks are not required for PR merge. They run in the optional `deep-checks.yml` workflow (weekly schedule or manual trigger) or locally via `make check-full`.

| Check | Command | Notes |
|-------|---------|-------|
| Domain/platform lint | `make clippy-domain` | Lint extracted implementation crates (part of `make check-full`) |
| Representative feature profiles | `make check-feature-profiles` | Coherent profile compilation |
| Exhaustive per-feature sweep | `make check-features-individual` | Every feature in its minimum set; `full` is curated, not exhaustive |

Dependency policy (`cargo deny`) is a mandatory PR gate, not an optional deep
check — see Security tool ownership below. Deep Checks retains the weekly
oracle role (domain lint, per-feature sweep, MSRV, portability,
platform-integration) but is no longer the first place advisories are found.

### Security tool ownership

| Defect class | Primary tool | Config |
|-------------|-------------|--------|
| Known advisories | `cargo deny check advisories` | `deny.toml` + `docs/DEPENDENCY_EXCEPTIONS.md` (canonical; PR gate) |
| Disallowed licenses | `cargo deny check licenses` | `deny.toml` (allow list + `auto_generate_cdp` build-time exception; PR gate) |
| Banned/duplicate dependencies | `cargo deny check bans` | `deny.toml` (deny `aws-lc-rs`/wildcards, warn multiples; PR gate) |
| Unexpected sources | `cargo deny check sources` | `deny.toml` (unknown-registry/git deny, git rev required; PR gate) |
| Manifest diff vulnerabilities | GitHub Dependency Review | `ci.yml` `dependency-review` job (fail on moderate+, read-only; deny/RustSec authoritative for Rust) |
| Secret introduction | GitHub-native secret scanning | Repository settings |

## Architecture Drift Guards

Static grep checks in `scripts/check-architecture-guards.sh` (requires ripgrep) catch common terminology and structural regressions:

### Stale Command Registry Terminology
- Fail on `manual_only` in command registry/docs/tests (historical plan files excluded).
- Fail on `interactive_only` where `cli_interactive_only` should be used (historical plan files excluded).

### MCP Exposure Terminology
- Ensure `mcp_metadata_exposable` and `mcp_default_visible` both appear in `tool/registration.rs` and `docs/TOOL_REGISTRATION.md`.
- Fail on text equating OpsAgent with conservative default listing.

### Raw Dispatch Prevention
- Strict surfaces (REST, MCP, gRPC, agent) must not call `ToolDispatcher::dispatch()` directly.
- CI handler must not import dispatch-related types.

### Scope Contract Unification
- `eggsec-tool-core` must not implement `is_allowed()`/`authorize()` on the declarative `ScopeSpec` DTO; only `eggsec::config::Scope` via `EnforcementContext` authorizes.

### Plan Retention
- Verify key phase plan files still exist for handoff/audit continuity.

### Documentation Currency
- Verify current architecture docs exist (`COMMAND_REGISTRY.md`, `TOOL_REGISTRATION.md`, `FEATURE_MATRIX.md`, `METADATA_OWNERSHIP.md`, `CI_ARCHITECTURE_GUARDS.md`).
- Verify feature docs match the Cargo source of truth (`scripts/check-feature-docs.py`: engine default, declared features, curated `full` membership, domain inventory).
- Verify the individual feature sweep is maintained and scheduled (`scripts/check-features-individual.sh`, Makefile target, `deep-checks.yml`), including the mechanically-enumerated TUI feature section and the `eggsec-tui/full` aggregate.
- Verify the broad TUI profile (`db-pentest,web-proxy,c2`) remains in `make check-feature-profiles` for fast frontend drift detection.
- Verify extensibility handoff guides exist (`EXTENSIBILITY.md`, `extending/operations.md`, `extending/domains.md`, `extending/commands.md`, `extending/tool-exposure.md`, `extending/tui-actions.md`, `extending/report-evidence.md`, `extending/features.md`, `extending/testing.md`, `extending/templates.md`).
- Verify `EXTENSIBILITY.md` Detailed Guides table links resolve to existing files. Convention: links are written repo-root-relative (`docs/extending/....md`, `docs/....md`) and the guard resolves them from the workspace root — do not "fix" them to `docs/`-relative form.
- Fail on stale field names or contradictions in current docs.

### Crate Boundary Invariants
- `eggsec-runtime` has no TUI, transport, persistence, engine, or domain crate dependencies.
- `eggsec-output` has no reverse dependencies on engine or runtime.
- `eggsec-daemon` has no TUI dependencies; engine dependency is optional/feature-gated.
- `eggsec-daemon` transport crates are feature-gated optional dependencies.
- `eggsec-frontend-client` has no frontend framework (`ratatui`/`crossterm`) and no engine dependency (guard Check 153).
- Engine crate has no TUI or daemon dependencies.
- CLI TUI dependency is feature-gated.
- TUI has no canonical `TaskConfig`/`TaskResult` enums or `match task_kind` execution dispatchers.

### Network-Dependency Baseline + Transport-Contract Invariants (Phases A–G, roadmap executed 2026-09-13)
- Leaf crates (`eggsec-runtime`, `eggsec-tool-core`, `eggsec-output`, `eggsec-ui-model`, `eggsec-daemon-protocol`) have no `reqwest`/`rustls`/`tokio-rustls`/`hickory-resolver` dependencies or uses (guard Check 99).
- Shared `reqwest::RequestBuilder` wrappers removed in Phase D increment 1: `auth_context::apply_auth_context_to_request` and `AiClient::apply_auth` are gone (callers translate via canonical helpers locally); `integrations::send_with_retry` is `pub(crate)` compat (not domain-facing). Phase A (2026-09-16) removed the `ClientPool`/`OptimizedClientPool` N-client round-robin abstraction (single shared cloned client is canonical; Checks 103 + 115).
- Scoped transport contract (guard Check 100): `eggsec-transport` exists, stays dependency-light (`bytes`/`http`/`url`/`thiserror` only, no concrete clients in manifest or `::` uses), exposes mandatory-authority `HttpTransport`, full `NetworkAuthority` checkpoints, TOCTOU-closed `validate_binding`/`ApprovedBinding`, redacted secrets, and the recording fake; engine binding is `config::ScopeAuthority`; closure tests live in `crates/eggsec/tests/transport_contract.rs`.
- Canonical helpers (guard Check 101): `apply_auth_context_to_transport`/`_to_map`, `AiClient::auth_headers`/`apply_auth_to_transport`, `should_retry_status`/`backoff_for_attempt` exist and are the single source of truth (former compat wrappers removed, not merely labeled).
- Eggfetch adapter (guards Checks 102 + 135 + 136 + 137): `eggsec-transport-eggfetch` implements `HttpTransport` over published `eggfetch-core 0.2.0` with minimal features (`http1` + `http2` + `tls-rustls` + `proxy` for pinned routing; never `http3`/`cookies`/`multipart`/compression), no direct concrete-client uses, logical-URL + singular resolved-address direct + manual authorized redirects (H1/H2 route reuse via ALPN, `Auto { allow_http3: false }`, total deadline through body EOF) + singular per-leg proxy pins (`Proxy::resolved_addresses([proxy_peer])` / `proxy_target_addresses([ultimate_peer])`, SOCKS5H/plaintext fail-closed, no multi-address fallback, truthful `ConnectionInfo`), and the engine production load-test backend (direct + supported proxied; no Reqwest fallback); parity suites are `crates/eggsec-transport-eggfetch/tests/parity.rs` (52 tests) + `tests/h2_mux.rs` (5 H2 local) + `tests/socks5_local.rs` (4 SOCKS5-local) + `crates/eggsec/tests/transport_eggfetch_parity.rs` (5 interop).
- Phase D increment 1 (guard Check 103): agent has no `reqwest`/`rustls` (generic `LifecycleManager<T: HttpTransport>` injection); NSE `http_capability.rs` + proxy `outbound.rs` boundary modules exist with no concrete clients in code; `intercept/` never touches the client contract; web-proxy reqwest is minimal (`rustls-no-provider` + `socks` only).
- Phase E closure (guards Checks 104–108): engine library-default is empty (`default = []`, `cli` opt-in via process-host crates + daemon `full-executor` → `eggsec/cli`; Check 104); workspace Tokio baseline is `default-features = false` with per-crate `features = [...]` (`test-util` nowhere, DTO crates carry no Tokio; Check 105); Eggress 1.0.10 narrow boundary — Check 106 proves the exact direct Eggress allowlist in `eggsec-web-proxy` via manifest parse (`tomllib`): exactly `eggress-outbound` (`=1.0.10`, `default-features = false`, no optional features) + `eggress-uri` (`=1.0.10`, no optional features), so any third `eggress-*` edge fails; transport/eggfetch/policy/core stay Eggress-free, embed/runtime/server/routing/advanced surfaces forbidden — with `architecture/egress_reuse_decision.md` record naming 1.0.10 (Check 106, supersedes the Phase E blanket reject; hardened by the 2026-09-22 corrective pass, metadata closed by the 2026-09-25 1.0.10 addendum); no `eggsec-net`/web-client/evidence crates with `architecture/capability_segregation.md` record (Check 107); manifest-graph direction holds via `python3`+`tomllib` (DTO crates touch no transport/TLS/HTTP impl, `eggsec-transport` stays exactly `bytes`/`http`/`url`/`thiserror`, engine touches no frontend crates, path-dependency graph acyclic; Check 108).
- Retained baseline: `architecture/network_dependency_baseline.md` (per-artifact deps, leakage inventory, parity matrix, policy state + §7 increment-1 addendum + §8 Phase E closure + §9 Phase F supply-chain + §10 Phase G measurement). Scoped contract: `architecture/transport.md`. Adapter: `architecture/transport_eggfetch.md`. Egress/capability records: `architecture/egress_reuse_decision.md` + `architecture/capability_segregation.md`. Closure report: `architecture/network_dependency_closure.md` (final graphs, fixture results, debt, acceptance mapping). Executable invariants: `crates/eggsec/tests/network_policy_invariants.rs` (12 behaviors, local fixtures only) + `crates/eggsec/tests/transport_contract.rs` (13 closure tests through the fake).

### Supply-Chain Hardening Invariants (Phase F, guards Checks 109–112)
- Cargo Deny is canonical (`deny.toml` with `[graph] all-features`, fail-closed `[sources]`, `wildcards = "deny"`); `.cargo/audit.toml` must not exist; `make check-deps` exists and `make check` runs it (guard Check 109).
- Every `uses:` in `.github/workflows/*.yml` is pinned to a full-length commit SHA with a version comment; no mutable tags (`@v4`, `@stable`, `@master`, `@v2`, `@v5`, `@cargo-deny`) remain as refs (guard Check 110).
- Workflows declare least-privilege permissions (`contents: read` at top level and per job; no `write-all`) (guard Check 111).
- `.github/dependabot.yml` automates `cargo` + `github-actions` updates (weekly, no auto-merge); `ci.yml` owns `dependency-policy` (`make check-deps`) and PR-only `dependency-review` (moderate+, read-only, no separate license allowlist) jobs (guard Check 112).
- Retained record: `docs/DEPENDENCY_EXCEPTIONS.md` (per-exception owner/review-by/blocker + license exception + yanked notice + Phase E supersession note). Policy history: `architecture/network_dependency_baseline.md` §5 (Phase A input) + §9 (Phase F closure).

### Crate-Boundary Consolidation Invariants (Phase A, guards Checks 113–117)
- `eggsec-output` owns no scheduling/session: `schedule.rs`/`session.rs` gone, no `pub mod schedule|session`, no queue/session types; canonical cron owner is `eggsec-agent::cron` (Check 113).
- Scanner service knowledge stays scanner-owned: no `utils/service_detection.rs`, no `utils::service_detection` uses; canonical owner is `scanner::service_data` (Check 114).
- No Reqwest multi-client pool abstraction: no `utils/client_pool.rs`, no `ClientPool`/`OptimizedClientPool` types or imports; canonical path is one cloned shared client (Check 115).
- No second token-bucket in output/frontend crates: no `struct RateLimiter` in `eggsec-output`/`eggsec-tui`/`eggsec-cli`/`eggsec-daemon`; canonical owner is engine `utils::rate_limiter` (Check 116).
- No utils/common catch-all crate: no `eggsec-utils`/`eggsec-common`/`eggsec-shared`/`eggsec-helpers` references, no new catch-all modules, and removed `output`/`progress`/`stealth`/`privilege` utils do not reappear (Check 117).

### Crate-Boundary Consolidation Invariants (Phase B, guards Checks 118–120)
- `eggsec-report-model` stays data-only: crate exists with no `tokio`/`quick-xml`/`hostname`/`lru`/`reqwest`/`rustls`/`axum`/`tonic`/`clap`/`ratatui`/`eggsec-output`/engine deps and no filesystem/runtime/renderer uses in `src/` (Check 118).
- `eggsec-output` depends on the model, never the reverse: output manifests `eggsec-report-model`, model never references output, moved DTOs are not redefined in output, and convert/envelope/policy_summary/diff facades re-export the model (Check 119).
- DTO-only domain crates use the model, not the renderer: `eggsec-db-lab`, `eggsec-mobile-lab`, `eggsec-web-proxy`, and `eggsec-nse` manifest `eggsec-report-model`, carry no `eggsec-output` dependency, and contain no `eggsec_output::` imports in `src/`/`tests/` (Check 120).

### Crate-Boundary Consolidation Invariants (Phase C, guards Checks 121–123)
- `eggsec-policy` stays dependency-light: crate exists with no Tokio/HTTP/TLS/filesystem/frontend/engine/transport deps, no `cfg!(feature)` queries, and no resolver/authority behavior in `src/` (Check 121).
- Engine depends one-way on the policy kernel with transport independent: engine manifests `eggsec-policy`, policy never references transport (either direction), workspace lists the member (Check 122).
- Engine policy modules stay facades with no redefined core types: no `pub enum/struct` forks of the kernel vocabulary/descriptor/catalog/decision/scope types in `config/policy*.rs`/`scope*.rs`; config facades re-export `eggsec_policy` and `policy_bridge/` owns the adapters (Check 123).

### Crate-Boundary Consolidation Invariants (Phase D, guards Checks 124–126)
- Loadtest core stays transport-neutral: `plan`/`executor`/`metrics`/`progress`/`adapter` contain no `reqwest::`/`indicatif::`/`RequestBuilder`/`Client::builder`/`ProgressBar`/`CommonHttpArgs`/`EggsecConfig`/`LoadArgs` code uses (doc prose excluded); executor dispatches through `HttpTransport`, backend implements it for `ReqwestTransport`, progress flows through `ProgressSink`, and `tui_mode` appears only in the facade (Check 124).
- No unjustified loadtest/resilience crates: no `crates/eggsec-loadtest`, `crates/eggsec-resilience`, or `crates/eggsec-utils` paths and no such manifest references; Gates D1/D2 rejection recorded in `architecture/capability_segregation.md` (Check 125).
- Removed `utils::cache` stays removed: no `utils/cache.rs`, no `pub mod cache`, no `struct ApiCache` under engine src (Check 126).

### Load-Test Authorization + Transport Corrective Pass (2026-09-17, guards Checks 127–134)
- No wildcard load-test scope synthesis: no `default_facade_scope()` / `ScopeRule::new("*")` in `loadtest/`; `LoadTestRunner` stores `Option<Scope>` and `run()` fails before I/O without it (Check 127).
- Execution-scope propagation: `ApprovedExecution` + `approve_execution()` / `approve_manual_execution()`, `ToolExecutionContext` + `execute_with_context()`, `EnforcedDispatcher::dispatch_execution()`, `execute_approved_execution()` / `execute_canonical_with_scope()` exist (Check 128).
- Raw load-test tool execution fails closed without context (Check 129).
- Reqwest backend has no semantic fallback (no `Client::new()` default, no verified/insecure cross-fallback, no placeholder proxy, no direct-for-proxy fallback) (Check 130).
- Proxied client cache partitioned by full identity (endpoint + mode + TLS + credential fingerprint, never plaintext) (Check 131).
- Production load-test dispatches through `EggfetchTransport` (runner, `run_cli_with_scope`, `run_load_test_with_scope`); no direct `ReqwestTransport::with_system_resolver()` construction on those paths (Check 132).
- SOCKS5H remote-DNS and plaintext forward-proxy ultimate pinning fail closed explicitly at the `Proxy` checkpoint (Check 133).
- MSRV truthful for the adapter dependency: workspace `rust-version = "1.89"`, adapter requires `eggfetch-core 0.2.0+` (Check 134).
- Proxied backend route is singular per leg: `AuthorizedProxyRoute` carries `proxy_peer` / `ultimate_peer` (`SocketAddr`, not `Vec`), the Eggfetch boundary pins single-element sets, and DNS-approved vectors are never iterated into pins (Check 135 + adversarial CONNECT-proxy fixtures).

### TUI Single-Writer Logging Boundary (Phase A, 2026-09-20, guard Check 138)
- Production `crates/eggsec-tui/src/**/*.rs` has no direct `println!` / `eprintln!` / `print!` / `eprint!` / `dbg!` outside `#[cfg(test)]` modules. The guard splits each file at its first `#[cfg(test)]` / `mod tests` line (same approach as Check 18) so test-only uses stay allowed.
- The CLI TUI launch path resolves intent before subscriber construction (`rich_tui_launch_requested` + `resolve_console_logging` / `console_policy_for_launch`) and initializes logging via `init_logging_with_console` with an explicit `ConsoleLogging` policy — bare `init_logging(` must not remain in `main.rs`.
- Both logging copies (`eggsec-cli/src/logging.rs`, `eggsec/src/logging/init.rs`) expose `ConsoleLogging`, `init_logging_with_console`, `resolve_console_logging`, `console_layer_enabled`, gate the console `fmt` layer on `console_layer_enabled`, and carry the `ConsoleLogging::Disabled` no-console path.
- `app/runner.rs` has no `eprintln!` and routes the sub-80x24 warning through `small_terminal_warning_message()` into the in-frame notification overlay.
- The guard deliberately does not ban `tracing::warn!` / `info!` / `error!`: tracing is the required diagnostic facade; the sink policy is the architectural control.

### TUI Cleanup-Safe Lifecycle + Process-Output Closure (Phase B, 2026-09-20, guard Check 139)
- No `Stdio::inherit()` in TUI Rust sources (`crates/eggsec-tui/src/**/*.rs`; prose docs may name the banned token to document the ban). TUI-reachable child output is captured, never inherited; the whole-workspace process audit of 2026-09-20 records every production site as `Command::output()` capture or explicit `Stdio::piped()`.
- `app/runner.rs` uses the session path: `TerminalSession::new`, `session.restore()`, `combine_body_restore`, `fn run_tui_body`, `restore_with_ops`, `block_on_ambient` are present; code-level (comment-stripped) `Terminal::new(`, `enable_raw_mode`, `EnterAlternateScreen`, `LeaveAlternateScreen`, `set_hook`/`take_hook` are absent (no open-coded lifecycle, no competing panic hooks).
- No nested Tokio runtime on the TUI daemon path: exactly one `tokio::runtime::Runtime::new` in `runner.rs` (the `block_on_ambient` standalone-host fallback) and none in `app/mod.rs` (daemon connect/attach reuse the ambient `#[tokio::main]` runtime via `block_in_place`).
- The CLI launch gate passes `has_command` correctly: `cli.command.is_some()` appears in both `cfg` branches of `main.rs`, and no `is_none()` sits inside a `rich_tui_launch_requested` call (pins the 2026-09-20 PTY-caught inversion that dead-coded the TUI launch).

### Performance Campaign Invariants (2026-09-21, guards Checks 140–142)
- Fuzzer fan-out stays bounded (Check 140): `crates/eggsec/src/fuzzer/engine/execution.rs` schedules through a bounded `JoinSet` admission loop (`in_flight.len() < concurrency`) with no `tokio::sync::Semaphore`, `DashMap`, or `join_all` retained-handle shape.
- Worker capacity stays truthful (Check 141): `crates/eggsec/src/distributed/worker.rs` carries `CapacityTracker` with `try_reserve`/`request_size` and the zero-concurrency rejection; the fixed `mpsc::channel::<Task>(100)` buffer (which let queued work exceed budget) is gone.
- Load-test prototype stays compiled once per run (Check 142): `crates/eggsec/src/loadtest/executor.rs` dispatches `prototype.clone()` in the worker loop with no `self.template.scoped_request` per-request rebuild.

### Desktop-GUI + Operation-Schema Publication Invariants (2026-10-23, guards Checks 151–154)
- The published schema stays off the authorization path (Check 151): the derived schema in `eggsec-runtime` (`schema` feature) and the daemon's `DescribeOperations` publication are descriptive only. The engine's dispatch/bridge halves, the policy kernel, and the engine's config/policy/scope files may not reach for `schemars`, `eggsec_runtime::schema`, `describe_operations`, or `DescribeOperations`, and neither `crates/eggsec/Cargo.toml` nor `crates/eggsec-policy/Cargo.toml` may gain a `schemars` dependency — so the schema is unreachable from an enforcement path by construction.
- No hand-written parameter table (Check 152): only `crates/eggsec-runtime/src/request.rs` owns the canonical request DTOs. No other frontend-neutral crate (`eggsec-core`, `eggsec-tool-core`, `eggsec-output`, `eggsec-ui-model`, `eggsec-report-model`, `eggsec-daemon-protocol`, `eggsec-policy`, `eggsec-frontend-client`) may define a `*Params` struct.
- The frontend-client crate stays framework-free (Check 153): `crates/eggsec-frontend-client/Cargo.toml` must exist and must not reference `eggsec`, `eggsec-tui`, `ratatui`, or `crossterm`; `crates/eggsec-frontend-client/src` must not reference `ratatui`, `crossterm`, `eggsec_tui`, or `eggsec::`.
- The desktop GUI stays a manual surface (Check 154): both `RuntimeSurface::DesktopGuiManual` and `ExecutionSurface::DesktopGuiManual` must exist, `runtime_bridge/surface.rs` must map the variant in **both** directions, and the variant must stay on the manual side of `honors_manual_override()` and `is_agent_controlled()`.

### Desktop-GUI Shell Invariants (2026-10-23, guards Checks 155–160)
- The GUI crate links no engine (Check 155): `crates/eggsec-gui/Cargo.toml` must not name `eggsec`, `eggsec-daemon`, `eggsec-cli`, or `eggsec-tui`, and no file under `crates/eggsec-gui/src/` may import them. `eggsec-frontend-client`, `eggsec-runtime`, `eggsec-daemon-protocol`, `eggsec-ui-model`, and `eggsec-output` are the allowed set. This is the `ADR-0007` daemon-only boundary: the desktop GUI reaches the engine only through the daemon's Unix socket, so it can never become a second composition root.
- The shell framework stays inside the GUI crate (Check 156): no crate other than `crates/eggsec-gui` may declare a `tauri` dependency. The root `Cargo.toml`'s `[workspace.dependencies]` version pins are allowed — they are the single place the exact Tauri versions are declared.
- The GUI declares no second TLS stack (Check 157): `crates/eggsec-gui/Cargo.toml` must not enable `rustls-tls` or `native-tls`, and must not depend on `reqwest`, `rustls`, or `native-tls` directly. The GUI performs no outbound HTTP, so those crates stay out of its closure entirely and the workspace's ring-only TLS invariant holds with no exception.
- The GUI's CSP forbids remote origins (Check 158): `crates/eggsec-gui/tauri.conf.json`'s `app.security.csp` must contain no wildcard source, and no `*` in `connect-src`, `script-src`, or `default-src`.
- The GUI has no per-operation form code (Check 159): the guard reads the task-kind variant names from `crates/eggsec-runtime/src/request.rs` and fails if the GUI's Rust sources or its frontend name any of them. The form must be generated from the published schema, so a task kind the engine has not published yet gets a form with no GUI change, and no hand-written table of operation names may creep in.
- The GUI retains no unbounded event log (Check 160): `crates/eggsec-gui/src/tasks.rs` must declare both the per-task retention cap (`MAX_LOG_LINES_PER_TASK`) and the forward-flush cap (`MAX_LOG_LINES_PER_FLUSH`), and the `LogRing` struct must be a capped ring rather than a `Vec<LogLine>` that grows without bound. Unbounded retention is a disqualifying defect, not a tuning problem.

### Desktop-GUI Coverage and Report-Display Invariants (2026-10-24, guards Checks 161–162)
- The GUI's coverage model is derived, not transcribed (Check 161): the guard reads the operation ids from `crates/eggsec-policy/src/catalog.rs` and the capability names from `crates/eggsec-runtime/src/capabilities.rs`, and fails if any of them appears in the GUI's **production** Rust halves or in its frontend. The names are read from their owners rather than transcribed in the guard itself, so a new operation or capability is covered the moment the engine publishes it. The Rust side is scanned above the `#[cfg(test)]` boundary, because a test may legitimately use a name as a fixture; the frontend has no test half and is searched whole. The coverage table must be computed from what the daemon publishes — the task kinds, the operations, their required features, the advertised capabilities, and the published enabled-feature set — so a hand-written table of operation names or capabilities cannot creep in and drift silently.
- The GUI's report display carries no remote origin and no executable content (Check 162): the guard parses `crates/eggsec-gui/tauri.conf.json`'s `app.security.csp` and fails if it contains `'unsafe-inline'`, if `frame-src` is absent or is not `'none'`, if any GUI production source or frontend file references a remote origin (the Tauri IPC origin `http://ipc.localhost` is the single permitted exception), if the frontend uses a markup-injection sink (`innerHTML`, `outerHTML`, `insertAdjacentHTML`, `document.write`), or if the export module consumes `convert_to_html_self_contained` or fails to refuse an HTML display rendering. The report the daemon serves is **attacker-influenced** — banners, service names and finding text all come from the target — so the CSP is the control that stops it from doing anything. The GUI therefore renders the report's structured content as text in its own DOM and never injects engine HTML into the webview; the HTML format stays export-only, where the exported file is the operator's own artifact opened in the operator's browser. A self-contained HTML document exists in `eggsec-output` and is tested there, but the CSP makes embedding it in the webview unavailable, so the GUI must not consume it.

### Desktop-GUI Daemon-Profile and Start-Honesty Invariants (2026-10-23, guards Checks 163–167)
- The daemon's service profile matches the engine's contract (Check 163): the `eggsec/<feature>` entries in `crates/eggsec-daemon/Cargo.toml`'s `service` feature must equal the set parsed from `crates/eggsec/src/config/daemon_profile.rs`'s `DAEMON_SERVICE_PROFILE`, and `full-executor` must be exactly `["service"]`. The engine owns the feature list — it is the single source of truth — and the daemon's manifest consumes it, so a drift would silently ship a daemon with a different dependency closure than the one the documentation and the conformance tests describe. Checks 161 and 162 pin the GUI side of the same enabled-feature publication.
- The CLI carries no rejecting daemon executor (Check 164): no file under `crates/eggsec-cli/src/` may declare a `NoopExecutor` or reject a task with `RuntimeError::UnsupportedTaskKind` in its production code. The CLI's `daemon start` used to host an executor that rejected every task, so the daemon it started advertised capabilities it could not execute; the fix removes the rejecting executor from the CLI entirely rather than teaching the command to lie less often.
- The daemon host derives its published capabilities (Check 165): the `ClientCommand::Capabilities` arm in `crates/eggsec-daemon/src/host.rs` must publish the host's **derived** task-kind list and must not name `RuntimeCapabilities::default()`, `daemon_conservative()`, or `full_lab()`. A host must never advertise a capability its executor rejects outright; the derivation is the executor's declaration intersected with the canonical capability list.
- The CLI's daemon start path waits for a real readiness signal (Check 166): `crates/eggsec-cli/src/daemon_cli.rs` must not scrape the server's stdout for the string `Daemon listening on`, must not use a sub-second literal sleep, and must declare a named readiness bound of at least one second. The old path slept for a fixed fraction of a second and then printed "Daemon started" whether or not the daemon had bound the socket.
- The daemon's obtainability recipe names the cargo feature (Check 167): `docs/DAEMON.md` must document a `cargo install` recipe for `crates/eggsec-daemon` that names the `service` (or the deprecated `full-executor`) cargo feature. Nothing in the workspace installs the daemon, so the recipe is the only way an operator obtains it — and a recipe that omits the feature produces a binary that accepts `--full-executor` and silently falls back to a no-op executor.

### Daemon Manual-Capability-Policy Invariants (2026-10-24, guards Checks 169–172)

Milestone 010 (`ADR-0011`) replaced the daemon's hand-written capability lists with one **derived** list covering every canonical capability kind, and made the daemon's published set and admitted set the same object. Four checks pin it:

- The capability list is **derived** (Check 169): `daemon_conservative()` and `full_lab()` must not exist anywhere under `crates/`, the private duplicate name table `task_kind_name()` must not exist, `TaskKind::capability_name()` must be the only capability-name table, and `crates/eggsec-runtime/src/capabilities.rs` must derive the list from `TaskKind::all_variants()` rather than carrying a literal one. The canonical task-kind table in `crates/eggsec-runtime/src/request.rs` and the enum's own operation-identity match must cover the same set of kinds, so a kind that reaches one but not the other fails the check.
- Publication and admission are the **same object** (Check 170): `crates/eggsec-daemon/src/host.rs` must hand the published list to the runtime as its enforced set and must not build the enforced set from a constant, and every rejecting executor must declare the empty set explicitly — the trait's default is the canonical list, so "says nothing" would mean "declares everything".
- The `wireless-active` wire kind resolves to the `wireless-deauth` operation (Check 171): the wire-side table in `crates/eggsec-runtime/src/request.rs`, the engine's canonical table in `crates/eggsec/src/dispatch/canonical_execution.rs`, and the executor routing table must all agree. While the wire kind resolved to the passive `wireless` operation, a daemon-submitted deauth was authorised under the passive operation's risk level.
- The interface-bound target policy is coherent (Check 172): the catalog's `packet-capture`, `wireless`, and `wireless-deauth` operations carry `TargetPolicyKind::InterfaceBound`, the enforcement kernel handles that value, and **no** shared operation — `packet` (which still serves `packet-send`) or `pipeline` (which still serves `resume`) — was relaxed to `OptionalTarget` to accommodate a target-less wire kind.

### Daemon-Mediated-Policy-Approval Invariants (2026-10-24, guards Checks 173–176)

Milestone 011 (`ADR-0012`) wired the daemon's `ApprovePolicy` command to the runtime's pending-approval registry: a manual operation whose policy demands a confirmation is held at that point, the decision is exposed to clients, and the client's answer releases the hold. Four checks pin it:

- The daemon **answers** a pending decision (Check 173): `crates/eggsec-daemon/src/host.rs` must resolve the runtime's pending approval, must not carry the old "not wired yet" refusal, and must report an answer for a task with nothing pending as `ErrorCode::InvalidState` — a refusal that is always the same refusal tells the operator nothing.
- The runtime stays **policy-neutral** (Check 174): `crates/eggsec-runtime` must not depend on `eggsec-policy` and must not name the policy crate's vocabulary (`ConfirmationClass`, `EnforcementContext`, `EnforcementError`, `EnforcementOutcome`, `ManualOverride`). The registry stores a prompt and returns an answer; only the kernel decides what an answer permits.
- The confirmation-class→override mapping exists in **exactly one place** (Check 175): `ManualOverride::permitting_classes` must exist in `crates/eggsec-policy/src/decision.rs`, and neither the TUI's enforcement facade nor the engine's runtime bridge may assign one of the eight override-permitting fields itself — both must call the shared helper. A second copy of the table would let the two drift, and the drift would surface as an operator whose answer grants a class the kernel never demanded.
- The daemon **audits** the answer it accepts (Check 176): the `ApprovePolicy` handler must record an audit event under the action `approve-policy`. An approval is an authorisation an operator gave; one that leaves no trace is indistinguishable from one that was never asked for.

### Dependency-Exception Ownership Invariants (2026-10-24, guard Check 190)

Milestone 015 closed the workspace's dependency advisory backlog: the 23 entries
measured at the milestone-010 baseline were either upgraded away (9 by a
lockfile refresh, 3 by the `zip` 9.x major bump) or recorded as owned,
dated exceptions. One check keeps that state from rotting:

- Every dependency exception is **owned and dated** (Check 190): every entry in
  `deny.toml`'s `[advisories] ignore` array must carry a non-empty `reason`
  that names `docs/DEPENDENCY_EXCEPTIONS.md`; every ignored advisory ID must
  have a record in `docs/DEPENDENCY_EXCEPTIONS.md` carrying a non-empty
  `| Owner |` and a `| Review-by |` that is an ISO date (`YYYY-MM-DD`) or the
  literal `— (closed)`; and no `RESOLVED` record's advisory ID may remain in
  the live `ignore` list. `deny.toml` deliberately carries no owner field — the
  house style puts ownership and the review date in the exception document — so
  the check reads both files. It fails closed: if either file cannot be
  parsed, the check reports the failure rather than passing because it found
  nothing, because a guard that silently passes is worse than none.

### Dependency-Tool Invariants (2026-10-25, guards Check 193–194)

`make check-deps` **is** the dependency policy, and it is decided by whichever
`cargo-deny` the machine happens to have. Two checks keep that decision honest
and keep it finishing:

- The CI's **cargo-deny is one version, and it is the version the local contract runs** (Check 193): every `tool: cargo-deny` in `.github/workflows/*.yml` must name an exact `x.y.z` version, every one of those versions must be the same, and that version must be at least **0.20.2**. The uniformity is milestone 015's finding **F4** — the local tool was 0.20.2 while CI ran 0.19.0, so "a local green is not proof of a CI green" — and the floor makes that finding enforceable: CI must not fall behind the version the local contract runs, or the two would again decide the same policy with different tools. A floating `tool: cargo-deny` decides the policy by whatever the day brought.
- The dependency policy is **decided without the inclusion graph** (Check 194): every `cargo deny` invocation in the `Makefile` must pass `--hide-inclusion-graph`. cargo-deny prints an inclusion graph for every duplicated crate, and this workspace duplicates **98** of them; rendering that graph is where the CI runner stalls — the Deep Checks job hangs inside `check bans`, mid-graph, with no output and no error. The stall reproduces on cargo-deny **0.19.0 and 0.20.2** alike, and a control branch proved a shallow checkout does not cause it. `--hide-inclusion-graph` removes the rendering path and changes nothing else: the same 98 duplicate warnings are printed and the same four verdicts (`advisories ok, bans ok, licenses ok, sources ok`) are reached. The flag is therefore the fix, and the check exists because a version bump, a refactor, or a new sub-command can silently drop it — and its absence brings the hang back.

> **Correction (2026-10-25).** Check 193 originally carried a floor of 0.19.9 and
> justified it by cargo-deny's thread-pool bug
> ([#868](https://github.com/EmbarkStudios/cargo-deny/issues/868), fixed by PR#870 in
> 0.19.9). That justification was wrong: the runner's stall reproduces on 0.20.2 as
> well, and it is in the inclusion graph's rendering, not in the thread pool. The
> version pin stands on F4's own terms — local/CI alignment — and the stall is
> guarded separately, by Check 194. Milestone 015's closure record carries the same
> correction as a dated addendum.

### TUI Feature-Gate Invariants (2026-10-25, guard Check 195)

The TUI's tab modules are feature-gated and `make check` builds `eggsec-tui` with
default features — which is `[]`. A tab module behind `#[cfg(feature = "...")]` is
therefore compiled only by the per-feature sweep, which runs weekly, not per PR.

- A TUI tab **does not redefine a method its own input macro emits** (Check 195): for
  every `impl TabInput for ...` block under `crates/eggsec-tui/src/tabs/`, the
  hand-written method names are compared against the methods the macros that block
  invokes actually emit, and the check fails on the intersection. It parses
  `tabs/macros.rs` for each `macro_rules!` body's `fn` names, so it tracks the
  macros rather than a hand-kept list of them. A tab that invokes no macro cannot
  collide, and a tab that defines a method no invoked macro emits is fine — only
  the overlap is a defect. It fails closed: if the macros file cannot be read or no
  macro is found in it, the check reports the failure rather than passing because
  it found nothing.

  The check exists because of a real defect that survived on `main`:
  `crates/eggsec-tui/src/tabs/vuln.rs` defined `ensure_input_focus` by hand *and*
  invoked `tab_input_custom!`, which emits `ensure_input_focus`. That is a duplicate
  definition (E0201), so the `vuln-management` feature did not compile at all — and
  no fast check built it. The defect had been present since commit `c66f0943`; the
  weekly sweep was the only thing that could see it, and it could not run until the
  cargo-deny stall was fixed (Check 194).

  The check narrows the gap to the class that actually bit. The wider gap — a
  feature-gated tab that fails to compile for any other reason — is still the
  sweep's job, and remains a known limitation of the per-PR contract.

### Desktop-GUI Platform-Qualification Invariants (2026-10-23, guard Check 168)
- The desktop GUI's platform qualification is machine-checked (Check 168): `docs/GUI_PLATFORM_QUALIFICATION.md` must carry a matrix with a row per platform and a cell per behaviour (`Build`, `Launch`, `Connect`, `Submit`, `Cancel`, `Render`, `Export`). Every cell must begin with one of the enumerated status tokens — `qualified`, `partially qualified`, `not qualified`, `out of scope` — and no cell may be blank or carry an unqualified support claim. The `windows` row must be `out of scope` in every column and must name `ADR-0010`, the decision that excludes it. `crates/eggsec-gui/tauri.conf.json` must keep `bundle.active` false, because the installers belong to the distribution workstream.

### Daemon-Endpoint-Default-and-Bind-Contract Invariants (2026-10-24, guards Checks 177–179)

Milestone 012 (`ADR-0013`) gave the daemon's socket path a single owner: the canonical default, the environment variable's name and the precedence rule live in `crates/eggsec-runtime/src/daemon_endpoint.rs`, and the server's bind refuses to displace a live daemon. Three checks pin it:

- The default socket path exists in **exactly one place** (Check 177): the literal `/tmp/eggsec-daemon.sock` may appear only inside `crates/eggsec-runtime`. The GUI's webview assets (`crates/eggsec-gui/frontend/`) and the shell scripts (`scripts/`) must hold **zero** occurrences, so neither the webview nor the build script can keep a copy that drifts from the Rust one.
- The server **refuses** to bind over a live socket (Check 178): `crates/eggsec-daemon/src/server.rs` must bind before it removes the socket path, must carry an `AddrInUse` refusal branch, and must consult the shared probe `probe_daemon_liveness` rather than a second liveness notion (no lock file, no PID file). A removal before the bind destroys the evidence of whether the path held a live daemon.
- The environment override has **one name and one resolver** (Check 179): the literal `EGGSEC_DAEMON_SOCKET` may appear only inside `crates/eggsec-runtime`; the canonical crate must declare `DAEMON_SOCKET_ENV_VAR` and `DEFAULT_DAEMON_SOCKET_PATH` and define `resolve_daemon_socket_path` and `daemon_socket_path_from_env`; and the daemon binary's `--socket-path`, the engine's global `--socket`, the daemon's `DaemonConfig::default()` and the GUI's `GuiOptions::default()` must each resolve through the canonical items.

### Report-Bridge Coverage Invariants (2026-10-24, guard Check 180)

Milestone 013 gave the report bridge a **coverage table**: the single source of truth for which canonical operations produce a report document, and — for the ones that do not — the fact their result is missing. The table lives in `crates/eggsec/src/runtime_bridge/report_coverage.rs`; the engine publishes it, the daemon republishes it verbatim in `DaemonCapabilities.report_forms`, and the GUI renders it. One check pins it:

- The coverage table and the bridge's arms **agree** (Check 180): the guard reads the `TaskResult` variants from `crates/eggsec/src/dispatch/types.rs` and the variant lists from `crates/eggsec/src/runtime_bridge/report_coverage.rs` and asserts that the two sets are equal, so a new result variant that no row names fails the check rather than silently producing no report. It also asserts that every variant the table calls *canonical* has a `TaskResult::` arm in `crates/eggsec/src/runtime_bridge/report.rs`, that every bridge arm is a canonical row, that an excluded variant has no arm, and that the table exists in exactly one place — a second copy would be a second hand-maintained catalog. The names are read from their owners, never transcribed in the guard.

### Desktop-GUI Platform-Qualification Invariants (2026-10-24, guards Check 181–189 and Checks 191–192)

Milestone 014 made the desktop GUI's qualification matrix **checkable**. The matrix's behavioural columns were unpromotable because nothing produced the evidence they claim and nothing checked the claims that existed. The procedure (`scripts/gui_qualification.sh`), the harness (`crates/eggsec-gui/tests/qualification.rs`), and the validator (`scripts/check_gui_qualification.py`) close that loop, and the checks below pin it:

- The **export procedure is written** (Check 181): `docs/GUI_BUILD.md` §5.7 must contain no `Not yet defined`, must have a non-empty Action row, a non-empty Pass criterion row, and a non-empty Evidence artefact row, must name the `export_report` command, and must name all six formats (`json`, `csv`, `html`, `md`, `sarif`, `xml`). A column with no procedure is a column nobody can run.
- The **retired milestone token stays retired** (Check 182): if `crates/eggsec-gui/frontend/app.js` contains `export_report`, then `docs/GUI_PLATFORM_QUALIFICATION.md` contains no `pending M6`. The token described a block that no longer exists; a token that can outlive its cause is a token that lies.
- A **qualified cell names resolvable evidence** (Check 183): every cell whose status is `qualified` must name either an existing `docs/evidence/gui-qualification/<file>.md` or a CI run URL that also appears in `docs/evidence/gui-qualification/CI-RUNS.md`. A claim about a run that cannot be found is not a claim about anything.
- The **matrix shape is exact** (Check 184): the header row must be exactly `| Platform | Build | Launch | Connect | Submit | Cancel | Render | Export |`, there must be exactly three platform rows (`linux`, `macos`, `windows`), and the cell count must be 21. The matrix is read by position by a validator, a guard, and a runbook, and all three must read the same matrix.
- The **runbook derives its steps from the matrix** (Check 185): `docs/GUI_BUILD.md` §5's step headings must be exactly `Step 1 — Build` … `Step 7 — Export`, one per matrix column, in the matrix's order. A runbook for a column the matrix does not have is a procedure nobody can cite.
- The **harness's identifiers and record schema are pinned** (Check 186): the harness's identifier set must equal the matrix's column set, and the record schema string `eggsec-gui-qualification-record/v1` must be the same in the harness, the procedure script, the validator, and the evidence template. A record written under one shape and read under another is a record nobody can trust.
- The **procedure is hermetic** (Check 187): `scripts/gui_qualification.sh` must not name `sudo`, must not remove anything recursively, and must name no absolute path outside the temporary root. The procedure runs on a developer's machine and on a CI runner; it must never need privileges the operator has to go and find.
- The **promotion validator is green** (Check 188): `python3 scripts/check_gui_qualification.py` must exit 0. The matrix's rules are enforced by a program, not by a reader's diligence.
- The **CI job publishes the qualification record** (Check 189): the `gui-platform` job in `.github/workflows/deep-checks.yml` must run `scripts/gui_qualification.sh` and must publish the run record as a job artefact named `gui-qualification-${{ matrix.os }}`. A CI run nobody can read is not evidence.
- An **operator attestation is attributed, and never defaulted** (Check 191): the qualification procedure must accept an `--attestation` file and must pass it to the record writer; the validator must declare the `eggsec-gui-operator-attestation/v1` schema, must refuse an operator outcome of `pass` that no named human signed, and must refuse one that carries no observation; **no code path may default an operator outcome to `pass`** — the only writer of a `pass` is the merge of a signed attestation; the harness must never emit an operator `pass`, because the machine cannot attest for a human; and the matrix's own rule **P1** must still require the operator outcome `pass` for the behavioural columns. Before this machinery existed the only documented route to a promotion was a hand edit of the record's json block, which a later run silently overwrites — a promotion with no tooling behind it.
- Every **CI checkout carries the repository's history** (Check 192): every `actions/checkout` step in `.github/workflows/*.yml` must set `fetch-depth: 0`. The architecture guard and the qualification validator both decide whether one commit descends from another, and `git merge-base` can only answer that when the checkout carries the history. `actions/checkout` fetches the tip alone by default, so a shallow checkout cannot answer the question at all — and the validator reported that "cannot tell" as a contradiction, which is how CI read the macOS operator's attestation as an attestation of another run. A false accusation, caused by the checkout rather than by the evidence; the checkout is now fixed, and the validator's answer distinguishes the two cases so the next one is honest.

### NSE Subsystem Invariants
- The standalone [eggsec-nse CI](https://github.com/eggstack/eggsec-nse/actions) owns runtime-internal checks: resolver-only script/module loading, per-run `NseRunReport.libraries`, profile/executor construction, library registry consistency, capability wrappers, and runtime corpus assertions.
- Eggsec Checks 143–146 enforce canonical engine/Python dispatch, no local re-vendoring, the released crates.io version shared by `crates/eggsec/Cargo.toml` and `Cargo.lock`, a single direct runtime consumer, the `eggsec::nse` facade, TUI/Python feature forwarding, and engine ownership of `nse_bridge`/`nse_http_capability`.
- Runtime revisions are qualified in the standalone repository before Eggsec updates its version. A Git/path/branch/tag/rev runtime source fails Check 144.

### Python-Specific Guards
These checks enforce invariants for the `eggsec-python` bindings and run within
the unified `python` job in `ci.yml`, which invokes `make check-python`. They are
executed once after a single `maturin develop` build.

| Check | Command | Purpose |
|-------|---------|---------|
| python-capability-matrix | `python scripts/check-python-capability-matrix.py` | Validates operation set, fields, and domain maturity vs Rust source |
| python-architecture-guards | `python scripts/check-python-architecture-guards.py` | Architecture drift checks (schema version, doc refs, runtime parity) |
| python-stub-parity | `python scripts/check_python_stub_parity.py` | Type stubs match runtime API surface |
| python-type-check | `bash scripts/check_python_types.sh` | Importability, __all__ resolution, stub syntax, mypy/pyright |

## Platform-Sensitive Checks

These checks require specific system dependencies or privileges and are never part of required PR CI:

| Check | Dependency | Notes |
|-------|-----------|-------|
| NSE runtime qualification | None for normal feature tests; local OpenSSH service in dedicated CI job | Runtime OpenSSL is vendored; SSH integration binds loopback only |
| Stress testing | Root/CAP_NET_RAW | Raw sockets, IP spoofing |
| Packet inspection | none (pure-Rust `pnet`) | Live capture additionally needs `CAP_NET_RAW` |
| Mobile dynamic | ADB + emulator | Frida, device interaction |
| Wireless | `wireless-tools` (iwlist) | WiFi scanning |
| Web proxy interception | Network stack | MITM proxy |
