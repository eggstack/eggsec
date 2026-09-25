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
| Package tests | `cargo test -p eggsec --features rest-api --tests --no-fail-fast` | All integration tests (MCP, REST, enforcement, dispatch, scanner, fuzzer, agent, NSE, and more) |
| Report envelope | `cargo test -p eggsec-output --tests` + `cargo test -p eggsec-report-model --tests` | Output rendering tests + model contract roundtrip |
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
cargo test -p eggsec --features rest-api --tests --no-fail-fast
cargo test -p eggsec-output --tests
cargo test -p eggsec-report-model --tests
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

### NSE Subsystem Invariants
- NSE script/module loading flows through `ScriptResolver`.
- `NseRunReport.libraries` is per-run require activity, not registry dump.
- `ManualPermissive` stays in manual CLI/TUI surfaces only.
- NSE automated surfaces use `with_profile()` not `with_policy()`.
- `NseLibraryDescriptor` instantiation is registry-owned.
- NSE registry entries have corresponding Rust modules.

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
| NSE tests | `libssl-dev` | Lua VM, sandbox |
| Stress testing | Root/CAP_NET_RAW | Raw sockets, IP spoofing |
| Packet inspection | `libpcap-dev` | Live capture |
| Mobile dynamic | ADB + emulator | Frida, device interaction |
| Wireless | `wireless-tools` (iwlist) | WiFi scanning |
| Web proxy interception | Network stack | MITM proxy |
