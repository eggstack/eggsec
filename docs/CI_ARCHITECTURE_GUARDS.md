# CI Architecture Guards

CI architecture guards preserve the enforcement, registry, metadata, feature, and documentation invariants established across Phases 1–14 of the architecture extensibility work. They stop regressions at pull-request time without making the workflow brittle, slow, or dependent on platform-specific optional features.

For the full verification contract (merge readiness vs release readiness, which checks are mandatory vs optional), see [`VERIFICATION.md`](VERIFICATION.md).

## Required Fast PR Checks

These checks run on every pull request and push to `main`. They cover core architecture invariants and should complete quickly.

| Check | Command | Purpose |
|-------|---------|---------|
| Formatting | `cargo fmt --all --check` | Code style consistency |
| No-default build | `cargo check --workspace --no-default-features` | Workspace compiles without optional features |
| Clippy | `cargo clippy --lib -p eggsec -- -D warnings` | Code quality on primary engine |
| Package tests | `cargo test -p eggsec --features rest-api --tests --no-fail-fast` | All integration tests (MCP, REST, enforcement, dispatch, scanner, fuzzer, agent, NSE, and more) |
| Report envelope | `cargo test -p eggsec-output --tests` | Output crate report/evidence envelope roundtrip |
| Architecture drift | `bash scripts/check-architecture-guards.sh` | Static grep checks for stale terminology and bypass patterns (requires ripgrep) |

### Local Reproduction

Run these before pushing to match CI:

```bash
make check
```

Alternatively, run the individual commands:

```bash
cargo fmt --all --check
cargo check --workspace --no-default-features
cargo clippy --lib -p eggsec -- -D warnings
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

> **Note**: `mobile-dynamic` may require platform-specific dependencies. If it fails in CI due to missing system deps, it should be documented with an issue reference rather than silently ignored.

## Optional/Deep Checks

These checks are not required for PR merge. They run in the optional `deep-checks.yml` workflow (weekly schedule or manual trigger) or locally via `make check-full`.

| Check | Command | Notes |
|-------|---------|-------|
| Advisory/license/ban policy | `cargo deny check` | Enforced via `deny.toml` |
| Representative feature profiles | `make check-feature-profiles` | Coherent profile compilation |

### Security tool ownership

| Defect class | Primary tool | Config |
|-------------|-------------|--------|
| Known advisories | `cargo deny check advisories` | `deny.toml` + `docs/DEPENDENCY_EXCEPTIONS.md` |
| Disallowed licenses | `cargo deny check licenses` | `deny.toml` |
| Banned/duplicate dependencies | `cargo deny check bans` | `deny.toml` |
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

### Plan Retention
- Verify key phase plan files still exist for handoff/audit continuity.

### Documentation Currency
- Verify current architecture docs exist (`COMMAND_REGISTRY.md`, `TOOL_REGISTRATION.md`, `FEATURE_MATRIX.md`, `METADATA_OWNERSHIP.md`, `CI_ARCHITECTURE_GUARDS.md`).
- Verify extensibility handoff guides exist (`EXTENSIBILITY.md`, `extending/operations.md`, `extending/domains.md`, `extending/commands.md`, `extending/tool-exposure.md`, `extending/tui-actions.md`, `extending/report-evidence.md`, `extending/features.md`, `extending/testing.md`, `extending/templates.md`).
- Verify `EXTENSIBILITY.md` Detailed Guides table links resolve to existing files.
- Fail on stale field names or contradictions in current docs.

### Crate Boundary Invariants
- `eggsec-runtime` has no TUI, transport, persistence, engine, or domain crate dependencies.
- `eggsec-output` has no reverse dependencies on engine or runtime.
- `eggsec-daemon` has no TUI dependencies; engine dependency is optional/feature-gated.
- `eggsec-daemon` transport crates are feature-gated optional dependencies.
- Engine crate has no TUI or daemon dependencies.
- CLI TUI dependency is feature-gated.
- TUI has no canonical `TaskConfig`/`TaskResult` enums or `match task_kind` execution dispatchers.

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
