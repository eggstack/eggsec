# AGENTS.md

Guidelines for AI agents working on this codebase.

**MSRV 1.89** (workspace `rust-version` in `Cargo.toml`). Verify with `make check-msrv` (needs `rustup toolchain install 1.89`).

## Verification (run before claiming correctness)

```bash
make check                  # mandatory Rust contract: fmt, no-default checks, check-deps, clippy, tests, guards
make check-deps             # dependency policy only: cargo deny over --workspace --all-features
make check-python           # only when Python bindings/stubs/docs/scripts change, plus eggsec-core or engine dispatch consumed by bindings
```

- `make test` = `cargo test --lib -p eggsec` only. Full suite: `make test-ci` (`-p eggsec --features rest-api,cli`).
- `deny.toml` is canonical; do not reintroduce `.cargo/audit.toml`. `make check-deps` fails closed when `cargo-deny` is absent. Record new dependency exceptions in `deny.toml` + `docs/DEPENDENCY_EXCEPTIONS.md` (owner + review-by date), never `cargo audit` config.
- Guards need `ripgrep` (`rg`): `bash scripts/check-architecture-guards.sh`.
- `make check-full` / `make check-features-individual` / `make clippy-domain` / `make test-tui-pty` are deep-checks only, not per-PR. Contract: `docs/VERIFICATION.md`.
- `make check-python` builds into `.venv-ci/` (override: `EGGSEC_PYTHON_VENV`); pytest excludes `network`-marked tests by default.

## Workspace

20 crates. Engine `eggsec` is lib-only (no binary); binary shell is `eggsec-cli`. CLI handlers live in-engine at `crates/eggsec/src/commands/handlers/` (`crates/eggsec/src/cli/` holds command types only; `crates/eggsec-cli/src/` is just main/daemon-client/logging).

- `eggsec-core`, `eggsec-tool-core`: shared types, tool DTOs.
- `eggsec-report-model`: stable report/evidence data contracts (data only). `eggsec-output` renders over it, never the reverse.
- `eggsec-policy`: deterministic authorization semantics only (data + pure algorithms; no Tokio/network/filesystem/frontends; engine bridges DNS/features/transport).
- `eggsec-transport`: scope-aware outbound HTTP contract (neutral DTOs, mandatory `NetworkAuthority`, recording fake). `eggsec-transport-eggfetch`: `HttpTransport` over published `eggfetch-core`.
- `eggsec-db-lab`, `eggsec-web-proxy`, `eggsec-mobile-lab`, `eggsec-nse`: domain crates (never authorize — caller enforces).
- `eggsec-runtime`, `eggsec-daemon`, `eggsec-daemon-protocol`, `eggsec-ui-model`: task lifecycle, session host, IPC types, view DTOs.
- `eggsec-agent`: coordination (registry, scheduler, lifecycle, cron). `eggsec-python`: PyO3/maturin bindings (`maturin develop` from `crates/eggsec-python/`).

```bash
cargo check --workspace --no-default-features   # CI baseline
cargo check -p eggsec --features <mobile|db-pentest|web-proxy|wireless|nse|evasion|postex|c2|rest-api|grpc-api>
make release-check                              # local release validation, no publication
```

System-dep features: `wireless` (wireless-tools), `packet-inspection` (libpcap-dev), `nse` (libssl-dev), `nse-ssh2` (libssh2-dev), `grpc-api` (protobuf-compiler, descriptor only — code is checked in). `http-api` is an `eggsec-daemon` feature, not an engine feature. `full` aggregates are curated, not exhaustive — the oracle is `make check-features-individual`. Feature inventory: `docs/FEATURE_MATRIX.md`.

## Enforcement (critical)

`EnforcementContext::evaluate()` is the mandatory pre-dispatch gate for **all** surfaces. Never bypass it.

- Manual (CLI/TUI): permissive profile, operator overrides allowed. REST/MCP/agent/CI strict profiles: no overrides, fail closed (only `Allow` dispatches), scope must be `LoadedScope` (never raw `Scope`).
- `OperationMetadata` is the single source of truth for operation policy — no inline policy checks.
- Strict surfaces dispatch only via `EnforcedDispatcher::dispatch_execution()` with an `ApprovedExecution` bundle (token + scope snapshot from the same context). Get bundles via `EnforcementContext::approve_execution()`/`approve_manual_execution()`; never construct directly, never match operation names alone (use `matches_descriptor()` + scope/policy/surface checks). Raw `dispatch_checked()` with `ApprovedOperation` alone remains for scope-insensitive tools.
- Authorization lives in `EnforcementContext`. Adapters use narrow service traits (`tool::service::EngineServices`, `agent::services::AgentExecutionService`, `mcp::bridge::McpEngineBridge`) and must never call `Scope::is_target_allowed` or `tool.execute()` directly.

## Dispatch / scope / runtime ownership

- Canonical executor: `dispatch::canonical_execution::execute_approved_execution` (plus `execute_approved` for scope-insensitive ops). CLI routes once via `commands::route::route_for_commands`. Executor code must not import Clap/Ratatui/daemon-protocol/Python types; cancellation goes through `eggsec-runtime::race_with_cancel`.
- `eggsec-tool-core::ScopeSpec` is a transport DTO with no auth methods. Convert via `eggsec::config::scope_from_spec` (fail-closed); effective auth is engine-scope ∩ converted-spec.
- `eggsec::policy_bridge::resolver` + `HostResolver` (`SystemResolver` default) for DNS; resolver reports facts, policy decides. Pure policy evaluation takes explicit `EnabledFeatures` + `TargetScope` facts (no `cfg!`/DNS inside `eggsec-policy`).
- `TaskKind::operation_id()`/`canonical_target()` is the single wire-side match; engine helpers delegate to it (no parallel tables). `RuntimeSurface` is a wire DTO — `runtime_bridge` owns both conversion directions (`Unknown` rejected).
- Dependency direction: `eggsec-report-model` depends only on `eggsec-core`; `eggsec-daemon` default deps are `eggsec-runtime` + `eggsec-daemon-protocol` only (engine behind `full-executor`, transport behind `http-api`, no TUI deps); `eggsec-transport` stays light (`bytes`/`http`/`url`/`thiserror` only); engine `policy_bridge/` owns feature/resolver/`NetworkAuthority` adapters (never `eggsec-policy` → `eggsec-transport`).
- TLS: ring-only everywhere. `rustls`/`tokio-rustls` with `default-features = false` + `["ring", ...]`; `reqwest` with `rustls-no-provider`, never `rustls`.
- Workspace Tokio baseline is `default-features = false` with per-crate `features = [...]` (`test-util` nowhere; DTO crates carry no Tokio).

## TUI rules (guard-enforced)

- Use `.get(i)` not `chunks[i]`; input/nav handlers check `!self.is_running()`; `reset()` clears all state (selectors, checkboxes, fields, focus).
- Production `eggsec-tui` has no `println!/eprintln!/print!/eprint!/dbg!` outside `#[cfg(test)]`; recoverable errors go through notification/per-tab-error/popup state, `tracing` stays for diagnostics. Single terminal writer: rich TUI installs no console logger.
- `copy-cli` builds an argv vector and must emit only flags the real Clap tree accepts. Fuzz HTTP-session flag is `--http-session` (`--session` is the daemon attach ID).
- `TerminalSession` (`app/runner.rs`) owns terminal setup/teardown with idempotent cleanup + silent `Drop` fallback; daemon sync paths reuse the ambient runtime via `runner::block_on_ambient` (never nested `Runtime::new`); no `Stdio::inherit()` in `eggsec-tui`.

## Gotchas

- Never `let _ =` or `filter_map(|e| e.ok())` — log via `tracing`. All spawned tokio tasks need 30–300s timeout wrappers. Check for file-top `#![allow(dead_code)]` before flagging dead code.
- Workspace root is a virtual manifest: `cargo install --path crates/eggsec-cli`.
- Themes: run `python3 scripts/package_themes.py` after editing `themes/*.toml`.
- Platform: `eggsec doctor` + `bash scripts/check_platform.sh` are hermetic (no root/hardware). Live scripts SKIP on missing prerequisites. Never run the full suite as root.
- Python fixtures: `EGGSEC_ALLOW_LOOPBACK_FIXTURE=1`. Stable/provisional/experimental boundary: `docs/python/domain-maturity.md`; extras source of truth: `[project.optional-dependencies]` in `crates/eggsec-python/pyproject.toml`.
- Keep high-cardinality fan-out bounded by configured concurrency (`WorkerConfig::max_concurrency` is a real capacity contract; no spawn-per-payload retention).
- Plans: `plans/registry.md` is the authoritative milestone/roadmap status — check it before assuming any roadmap state. New work follows `plans/003-planning-process.md` (canonical `000`/`001`/`002`, ADRs, subsystem roadmaps, bounded milestone plans, closure records). Flat-era plans at `plans/` top level are immutable history (mark `Status: Executed`); don't delete or rewrite them ad hoc.

## Where to look

- Contract/docs: `docs/VERIFICATION.md`, `docs/ARCHITECTURE.md`, `docs/ENFORCEMENT_MODES.md`, `docs/CI_ARCHITECTURE_GUARDS.md`, `docs/EXTENSIBILITY.md` (adding operations/domains/commands).
- Module index: `architecture/overview.md`. Per-module guidance: `crates/eggsec/src/<module>/AGENTS.override.md` + `architecture/<topic>.md` + skill in `.opencode/skills/`. Load all three when working in a module.
- Skills drift: when you rename a feature, change a public signature, or change a counted set (operations, aliases, tabs, payloads, techniques, endpoints, probes, descriptors), update the matching skill's claims alongside `architecture/<topic>.md` — skills are not covered by type checks, only by guard check 32 (Nmap-parity wording).
- Skills: `.opencode/skills/` is canonical; `.skills/`, `.agents/skills/`, `.claude/skills/` are symlinks to it (edit once).
