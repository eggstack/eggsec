# AGENTS.md

Guidelines for AI agents working on this codebase.

**MSRV 1.89** (workspace `rust-version` in `Cargo.toml`). CI tests it in `.github/workflows/deep-checks.yml` (`msrv` job). Verify with `make check-msrv` (needs `rustup toolchain install 1.89`).

## Verification (run before claiming correctness)

```bash
make check                  # mandatory Rust contract: fmt, no-default check, check-deps (deny), clippy, tests, guards
make check-deps             # dependency policy only: cargo deny --workspace --all-features check
make check-python           # only when Python bindings/stubs/docs/scripts change
```

- `make test` = `cargo test --lib -p eggsec` only (empty library default). Full suite: `make test-ci` (`-p eggsec --features rest-api,cli`).
- `make check-deps` is the Phase F supply-chain gate (advisories + bans + licenses + sources over the all-features closure; `deny.toml` canonical, `.cargo/audit.toml` removed — do not reintroduce). Fails closed when `cargo-deny` is absent.
- `make clippy` covers engine lib (empty default + `cli`) + leaf crates (`eggsec-core`, `eggsec-tool-core`, `eggsec-report-model`, `eggsec-output`, `eggsec-runtime`, `eggsec-ui-model`, `eggsec-agent`, `eggsec-transport`, `eggsec-transport-eggfetch`, `eggsec-policy`). Domain/platform lint is `make clippy-domain` (deep checks only).
- Guards need `ripgrep` (`rg`): `bash scripts/check-architecture-guards.sh`. No `cargo-nextest` required.
- `make check-python` builds into `.venv-ci/` (override: `EGGSEC_PYTHON_VENV`); pytest excludes `network`-marked tests by default.
- `make check-full` / `make check-features-individual` are deep-checks only, not per-PR. Contract details: `docs/VERIFICATION.md`.

## Workspace

20 crates. Engine `eggsec` is lib-only (no binary); binary shell is `eggsec-cli`; CLI handlers live in-engine at `crates/eggsec/src/commands/handlers/` (`crates/eggsec/src/cli/` holds command types only; `crates/eggsec-cli/src/` is just main/daemon-client/logging).

| Crate | Purpose |
|-------|---------|
| `eggsec-core`, `eggsec-tool-core` | shared types, tool DTOs |
| `eggsec-report-model` | stable serializable report/evidence data contracts (data only; domain DTO owner) |
| `eggsec-policy` | deterministic authorization/enforcement semantics (data + pure algorithms; no Tokio/network/filesystem/frontends; engine bridges DNS/features/transport) |
| `eggsec` | engine library |
| `eggsec-cli` / `eggsec-tui` | binary shell / terminal UI |
| `eggsec-runtime`, `eggsec-daemon`, `eggsec-daemon-protocol`, `eggsec-ui-model` | task lifecycle, session host, IPC types, view DTOs |
| `eggsec-output`, `eggsec-agent` | reports (rendering/conversion/analysis over `eggsec-report-model`; no scheduling/session), agent coordination (registry, scheduler, lifecycle, cron) |
| `eggsec-db-lab`, `eggsec-web-proxy`, `eggsec-mobile-lab`, `eggsec-nse` | domain crates (never authorize — caller enforces) |
| `eggsec-transport` | scope-aware outbound HTTP contract (neutral DTOs, mandatory `NetworkAuthority` incl. proxy-peer checkpoints, recording fake; `bytes`/`http`/`url`/`thiserror` only) |
| `eggsec-transport-eggfetch` | `HttpTransport` over published `eggfetch-core 0.1.7` (logical-URL + singular resolved-address direct, qualified proxy routes, H1/H2 route reuse; production load-test backend) |
| `eggsec-python` | PyO3/maturin bindings (`maturin develop` from `crates/eggsec-python/`) |

```bash
cargo check --workspace --no-default-features   # CI baseline
cargo check -p eggsec --features <mobile|db-pentest|web-proxy|wireless|nse|evasion|postex|c2|rest-api|grpc-api>
cargo check -p eggsec-cli --no-default-features --features daemon-client
make release-check                              # local release validation, no publication
```

System-dep features: `wireless` (wireless-tools), `packet-inspection` (libpcap-dev), `nse` (libssl-dev), `nse-ssh2` (libssh2-dev), `grpc-api` (protobuf-compiler, descriptor only — code is checked in). `http-api` is an `eggsec-daemon` feature, not an engine feature. `full` aggregates are curated, not exhaustive — the oracle is `make check-features-individual`. Feature inventory: `docs/FEATURE_MATRIX.md`.

## Enforcement (critical)

`EnforcementContext::evaluate()` is the mandatory pre-dispatch gate for **all** surfaces. Never bypass it.

- Manual (CLI/TUI): permissive profile, operator overrides allowed.
- REST/MCP (`McpStrict`), agent (`AgentStrict`), CI (`CiStrict`): no overrides, fail closed (only `Allow` dispatches), scope must be `LoadedScope` (never raw `Scope`).
- `OperationMetadata` is the single source of truth for operation policy — no inline policy checks.
- Strict surfaces dispatch only via `EnforcedDispatcher::dispatch_execution()` with an `ApprovedExecution` bundle (token + scope snapshot from the same context; tool+target verified). Get bundles via `EnforcementContext::approve_execution()`/`approve_manual_execution()`; never construct directly, never compare operation names alone (use `matches_descriptor()` + scope/policy/surface checks). Raw `dispatch_checked()` with `ApprovedOperation` alone remains for scope-insensitive tools; raw `LoadTestTool::execute()` without context fails closed.
- Authorization lives in `EnforcementContext`. Adapters use narrow service traits (`tool::service::EngineServices`, `agent::services::AgentExecutionService`, `mcp::bridge::McpEngineBridge`) and must never call `Scope::is_target_allowed` or `tool.execute()` directly.

## Dispatch / scope / runtime ownership

- Canonical executor: `dispatch::canonical_execution::execute_approved_execution` (+ `execute_approved` for scope-insensitive ops + `execute_canonical`/`execute_canonical_with_scope` match; load-test without scope fails closed). CLI routes once via `commands::route::route_for_commands`. Executor code must not import Clap/Ratatui/daemon-protocol/Python types; cancellation goes through `eggsec-runtime::race_with_cancel`.
- `eggsec-tool-core::ScopeSpec` is a transport DTO with no auth methods. Convert via `eggsec::config::scope_from_spec` (fail-closed); effective auth is engine-scope ∩ converted-spec. Never add `is_allowed()` to the DTO layer.
- `eggsec::policy_bridge::resolver::{resolve_target_facts_with, ScopeResolution}` + `HostResolver` (`SystemResolver` default) for DNS; resolver reports facts, policy decides. Pure policy evaluation takes explicit `EnabledFeatures` + `TargetScope` facts (`eggsec-policy`; no `cfg!`/DNS inside). Use `classify_address()` for address class; strict surfaces check all `resolved_addresses` against CIDR rules.
- `TaskKind::operation_id()`/`canonical_target()` is the single wire-side match; engine `operation_id_for_task_kind`/`target_for_task_kind` delegate to it (no parallel tables). `RuntimeSurface` is a wire DTO — the `runtime_bridge` owns both conversion directions (`Unknown` rejected). Result conversion is single-owned via `dispatch::task_result_envelope`.
- TUI: `TabSpec` (`tabs/spec.rs`) owns surface definitions; alias lookup via `resolve_palette_command()`; palette/keys via `app/palette.rs`. `copy-cli` builds an argv vector (`cli_argv()`, quoting only at boundary) and must emit only flags the real Clap tree accepts. Fuzz HTTP-session flag is `--http-session` (`--session` is the daemon attach ID). Single-terminal-writer rule (guard Check 138): rich TUI installs no console logger (`ConsoleLogging::Disabled` via `init_logging_with_console`; launch intent resolved before subscriber in `eggsec-cli/src/main.rs`); production `eggsec-tui` has no `println!/eprintln!/print!/eprint!/dbg!` outside `#[cfg(test)]`; user-visible recoverable errors go through notification/per-tab-error/popup state, `tracing` stays for diagnostics.
- Dependency boundaries (guard-enforced): `eggsec-runtime` stays light (serde/serde_json, thiserror, tokio, tokio-util, tracing, uuid); `eggsec-report-model` is data-only (serde/serde_json, chrono, uuid + `eggsec-core`; no Tokio/filesystem/renderers); `eggsec-policy` is deterministic semantics only (serde/serde_json, thiserror, url, ipnetwork, sha2, uuid, hex, rustc-hash; no Tokio/HTTP/TLS/filesystem/frontend/engine deps; Checks 121–123); `eggsec-output` renders over the model (`eggsec-core` + `eggsec-report-model`, never the reverse); domain DTO consumers (`eggsec-db-lab`, `eggsec-mobile-lab`, `eggsec-web-proxy`, `eggsec-nse`) depend on the model, not the renderer; `eggsec-daemon` default deps are `eggsec-runtime` + `eggsec-daemon-protocol` only (engine behind `full-executor`, transport behind `http-api`, no TUI deps); `eggsec-transport` stays light (`bytes`/`http`/`url`/`thiserror` only, no concrete clients) and independent of policy implementation; engine `policy_bridge/` owns the feature/resolver/`NetworkAuthority` adapters (never `eggsec-policy` → `eggsec-transport`).
- TLS: ring-only everywhere. `rustls`/`tokio-rustls` with `default-features = false` + `["ring", ...]`; `reqwest` with `rustls-no-provider`, never `rustls`.
- Network-dependency baseline (Phase A measurement + Phase B contract + Phase C adapter + Phase D increment 1: agent injection, shared-helper cleanup, NSE capability, proxy boundary, web-proxy pruning + corrective pass (ApprovedExecution scope propagation, Reqwest fail-closed, proxy-peer checkpoints, Eggfetch direct + qualified proxy, MSRV 1.89)): retained baseline is `architecture/network_dependency_baseline.md` (per-artifact deps, concrete-client inventory, parity matrix, policy state + §7 increment-1 addendum with remaining-owner dispositions); scoped contract is `architecture/transport.md` (`eggsec-transport` DTOs + mandatory `NetworkAuthority` + TOCTOU-closed binding + recording fake; engine binding via `config::ScopeAuthority`); eggfetch adapter is `architecture/transport_eggfetch.md` (`eggsec-transport-eggfetch` over published `eggfetch-core` with logical-URL + singular resolved-address direct + manual authorized redirect loop, HTTP/3 off, qualified proxy routes (eggfetch-core 0.1.7), singular per-leg pins, total deadline through body EOF, production load-test backend); executable invariants are `crates/eggsec/tests/network_policy_invariants.rs` (12 behaviors) + `crates/eggsec/tests/transport_contract.rs` (13 closure tests through the fake) + `crates/eggsec-transport-eggfetch/tests/parity.rs` (52 adapter tests over local fixtures) + `crates/eggsec-transport-eggfetch/tests/h2_mux.rs` (5 H2 local) + `crates/eggsec-transport-eggfetch/tests/socks5_local.rs` (4 SOCKS5-local) + `crates/eggsec/tests/transport_eggfetch_parity.rs` (5 engine interop tests); durable boundaries are guards Check 99 (Phase A) + 100/101 (Phase B) + 102 (Phase C: minimal eggfetch features + 0.1.7 proxy pinning, production load-test backend minimal eggfetch features, no direct concrete clients, production load-test backend) + 103 (Phase D increment 1: agent has no reqwest/rustls, shared `RequestBuilder` wrappers removed, NSE `http_capability.rs` + proxy `outbound.rs` boundaries hold, web-proxy reqwest minimal) + 135 (singular per-leg proxy route, no multi-address fallback). Canonical header/auth paths are transport-neutral (`apply_auth_context_to_transport`/`_to_map`, `AiClient::auth_headers`, `should_retry_status`); the former concrete-builder wrappers (`apply_auth_context_to_request`, `AiClient::apply_auth`) are removed — translate via the canonical helpers locally (see `fuzzer::engine::utils::apply_auth_context_to_builder`). New narrow builders: NSE `http_capability::build_scoped_request`, proxy `outbound::build_direct_probe_request`; agent `LifecycleManager<T: HttpTransport>` injects transport + authority (tests use the fake). Phase E closure (egress + capability segregation): `eggress-uri`/`eggress-routing`/stacks all rejected with measured graphs (`architecture/egress_reuse_decision.md`, no `eggress` edge); engine library-default is empty (`default = []`, `cli` opt-in via process-host crates + daemon `full-executor` → `eggsec/cli`); workspace Tokio baseline is `default-features = false` with per-crate `features = [...]` (`test-util` nowhere; DTO crates carry no Tokio); no new crates (`eggsec-net`/web-client/evidence all rejected in `architecture/capability_segregation.md`); durable boundaries add guards Checks 104 (empty default) + 105 (per-crate Tokio) + 106 (no eggress) + 107 (no new crates) + 108 (manifest-graph direction, no forbidden edges/cycles). Phase F supply-chain gate: `make check-deps` (deny advisories/bans/licenses/sources over all-features) runs in `make check` + dedicated CI job; audit.toml removed; actions SHA-pinned, least-privilege, dependabot + dependency-review on (guards Checks 109-112). Phase G closure (roadmap executed): retained report is `architecture/network_dependency_closure.md` (final graphs, fixture results, debt, acceptance mapping); baseline §10 is the measurement addendum.

## Gotchas

- TUI: use `.get(i)` not `chunks[i]`; input/nav handlers check `!self.is_running()`; `reset()` clears all state (selectors, checkboxes, fields, focus).
- Never `let _ =` or `filter_map(|e| e.ok())` — log via `tracing`. All spawned tokio tasks need 30–300s timeout wrappers. Check for file-top `#![allow(dead_code)]` before flagging dead code.
- Workspace root is a virtual manifest: `cargo install --path crates/eggsec-cli`.
- Themes: run `python3 scripts/package_themes.py` after editing `themes/*.toml`.
- Platform: `eggsec doctor` + `bash scripts/check_platform.sh` are hermetic (no root/hardware). Live scripts (`setup_packet_netns.sh`, `setup_android_emulator.sh`) SKIP on missing prerequisites. Never run the full suite as root.
- Python fixtures: `EGGSEC_ALLOW_LOOPBACK_FIXTURE=1`. Stable/provisional/experimental boundary: `docs/python/domain-maturity.md`; extras source of truth: `[project.optional-dependencies]` in `crates/eggsec-python/pyproject.toml`.
- Eggfetch qualification evidence: `tests/h2_mux.rs` contains 5 Eggsec-local H2 tests (including selected-address-only isolation with unchanged logical origin), and `tests/socks5_local.rs` contains 4 local-resolution SOCKS5 tests. Keep performance claims limited to the measurements recorded in `architecture/loadtest.md`; short or current-only samples do not establish version-to-version regression parity.
- Plans in `plans/` are retained (mark `Status: Executed`); don't delete phase plans ad hoc.

## Where to look

- Contract/docs: `docs/VERIFICATION.md`, `docs/ARCHITECTURE.md`, `docs/ENFORCEMENT_MODES.md`, `docs/CI_ARCHITECTURE_GUARDS.md`, `docs/EXTENSIBILITY.md` (adding operations/domains/commands).
- Module index: `architecture/overview.md` (Module Index maps each module → deep-dive; Deep-Dive Index catalogs all 66 docs).
- Per-module guidance: `crates/eggsec/src/<module>/AGENTS.override.md` + `architecture/<topic>.md` + skill in `.opencode/skills/` (canonical skills dir; e.g. `eggsec-tool`, `eggsec-config`, `eggsec-cli`, `eggsec-daemon`, `eggsec-tui`, `eggsec-python`). Load all three when working in a module.
- Skills: `.opencode/skills/` is canonical; `.skills/`, `.agents/skills/`, `.claude/skills/` are symlinks to it (edit once). 35 skills, one per major module, including `eggsec-compliance`, `eggsec-vuln-management`, `eggsec-findings-workflow` (cover the compliance/vuln/findings-workflow modules, which have no other skill). `eggsec-daemon` also covers `eggsec-runtime` + `eggsec-daemon-protocol`; dispatch/bridge ownership is documented in `architecture/dispatch.md` + `architecture/runtime_bridge.md` (no separate skill).
