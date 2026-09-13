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
| Report envelope | `cargo test -p eggsec-output --tests` | Output crate report/evidence envelope roundtrip |
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

### Network-Dependency Baseline + Transport-Contract Invariants (Phases A–C + Phase D increment 1 + Phase E closure)
- Leaf crates (`eggsec-runtime`, `eggsec-tool-core`, `eggsec-output`, `eggsec-ui-model`, `eggsec-daemon-protocol`) have no `reqwest`/`rustls`/`tokio-rustls`/`hickory-resolver` dependencies or uses (guard Check 99).
- Shared `reqwest::RequestBuilder` wrappers removed in Phase D increment 1: `auth_context::apply_auth_context_to_request` and `AiClient::apply_auth` are gone (callers translate via canonical helpers locally); `integrations::send_with_retry` is `pub(crate)` compat (not domain-facing); remaining `pub` concrete surfaces (`fuzzer` trait, `ClientPool`) are documented pending with no new boundary allowed (Checks 99 + 103).
- Scoped transport contract (guard Check 100): `eggsec-transport` exists, stays dependency-light (`bytes`/`http`/`url`/`thiserror` only, no concrete clients in manifest or `::` uses), exposes mandatory-authority `HttpTransport`, full `NetworkAuthority` checkpoints, TOCTOU-closed `validate_binding`/`ApprovedBinding`, redacted secrets, and the recording fake; engine binding is `config::ScopeAuthority`; closure tests live in `crates/eggsec/tests/transport_contract.rs`.
- Canonical helpers (guard Check 101): `apply_auth_context_to_transport`/`_to_map`, `AiClient::auth_headers`/`apply_auth_to_transport`, `should_retry_status`/`backoff_for_attempt` exist and are the single source of truth (former compat wrappers removed, not merely labeled).
- Eggfetch adapter (guard Check 102): `eggsec-transport-eggfetch` implements `HttpTransport` over published `eggfetch-core` with minimal features (`http1` + `tls-rustls` + `proxy`-for-SNI only; never `http3`/`cookies`/`multipart`/compression), no direct concrete-client uses, approved-IP pinning + manual authorized redirect loop, and **no production backend consumer** (only the engine test dev-dep; agent/NSE/proxy depend on `eggsec-transport` types only); parity suites are `crates/eggsec-transport-eggfetch/tests/parity.rs` + `crates/eggsec/tests/transport_eggfetch_parity.rs`.
- Phase D increment 1 (guard Check 103): agent has no `reqwest`/`rustls` (generic `LifecycleManager<T: HttpTransport>` injection); NSE `http_capability.rs` + proxy `outbound.rs` boundary modules exist with no concrete clients in code; `intercept/` never touches the client contract; web-proxy reqwest is minimal (`rustls-no-provider` + `socks` only).
- Phase E closure (guards Checks 104–108): engine library-default is empty (`default = []`, `cli` opt-in via process-host crates + daemon `full-executor` → `eggsec/cli`; Check 104); workspace Tokio baseline is `default-features = false` with per-crate `features = [...]` (`test-util` nowhere, DTO crates carry no Tokio; Check 105); no `eggress` manifest/source edge with `architecture/egress_reuse_decision.md` record (Check 106); no `eggsec-net`/web-client/evidence crates with `architecture/capability_segregation.md` record (Check 107); manifest-graph direction holds via `python3`+`tomllib` (DTO crates touch no transport/TLS/HTTP impl, `eggsec-transport` stays exactly `bytes`/`http`/`url`/`thiserror`, engine touches no frontend crates, path-dependency graph acyclic; Check 108).
- Retained baseline: `architecture/network_dependency_baseline.md` (per-artifact deps, leakage inventory, parity matrix, policy state + §7 increment-1 addendum + §8 Phase E closure). Scoped contract: `architecture/transport.md`. Adapter: `architecture/transport_eggfetch.md`. Egress/capability records: `architecture/egress_reuse_decision.md` + `architecture/capability_segregation.md`. Executable invariants: `crates/eggsec/tests/network_policy_invariants.rs` (12 behaviors, local fixtures only) + `crates/eggsec/tests/transport_contract.rs` (11 closure tests through the fake).

### Supply-Chain Hardening Invariants (Phase F, guards Checks 109–112)
- Cargo Deny is canonical (`deny.toml` with `[graph] all-features`, fail-closed `[sources]`, `wildcards = "deny"`); `.cargo/audit.toml` must not exist; `make check-deps` exists and `make check` runs it (guard Check 109).
- Every `uses:` in `.github/workflows/*.yml` is pinned to a full-length commit SHA with a version comment; no mutable tags (`@v4`, `@stable`, `@master`, `@v2`, `@v5`, `@cargo-deny`) remain as refs (guard Check 110).
- Workflows declare least-privilege permissions (`contents: read` at top level and per job; no `write-all`) (guard Check 111).
- `.github/dependabot.yml` automates `cargo` + `github-actions` updates (weekly, no auto-merge); `ci.yml` owns `dependency-policy` (`make check-deps`) and PR-only `dependency-review` (moderate+, read-only, no separate license allowlist) jobs (guard Check 112).
- Retained record: `docs/DEPENDENCY_EXCEPTIONS.md` (per-exception owner/review-by/blocker + license exception + yanked notice + Phase E supersession note). Policy history: `architecture/network_dependency_baseline.md` §5 (Phase A input) + §9 (Phase F closure).

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
