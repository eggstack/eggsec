# CI Architecture Guards

CI architecture guards preserve the enforcement, registry, metadata, feature, and documentation invariants established across Phases 1–10 of the architecture extensibility work. They stop regressions at pull-request time without making the workflow brittle, slow, or dependent on platform-specific optional features.

## Required Fast PR Checks

These checks run on every pull request and push to `main`. They cover core architecture invariants and should complete quickly.

| Check | Command | Purpose |
|-------|---------|---------|
| Formatting | `cargo fmt --all --check` | Code style consistency |
| No-default build | `cargo check --workspace --no-default-features` | Workspace compiles without optional features |
| Core lib tests | `cargo test -p eggsec --lib` | Unit test baseline |
| Metadata consistency | `cargo test -p eggsec --test metadata_consistency` | DomainDescriptor ↔ OperationMetadata ↔ capability matrix cross-validation |
| Command registry | `cargo test -p eggsec --test command_registry` | CommandRegistration dispatch-mode and visibility invariants |
| Tool registration | `cargo test -p eggsec --test tool_registration --features rest-api` | ToolRegistration Model A semantics and MCP exposure |
| Feature matrix | `cargo test -p eggsec --test feature_matrix` | Feature snapshot vs Cargo.toml keys, naming conventions |
| Enforcement matrix | `cargo test -p eggsec --test enforcement_matrix` | Cross-surface enforcement invariants (22 sections, ~95 tests) |
| Enforced dispatch regression | `cargo test -p eggsec --test enforced_dispatch_regression` | Strict surfaces do not call raw dispatch; CI handler has no dispatch path |
| Report envelope | `cargo test -p eggsec-output --test report_envelope` | Normalized report/evidence envelope roundtrip |
| Architecture drift | `bash scripts/check-architecture-guards.sh` | Static grep checks for stale terminology and bypass patterns (requires ripgrep) |

### Local Reproduction

Run these before pushing to match CI:

```bash
cargo fmt --all --check
cargo check --workspace --no-default-features
cargo test -p eggsec --lib
cargo test -p eggsec --test metadata_consistency
cargo test -p eggsec --test command_registry
cargo test -p eggsec --test tool_registration --features rest-api
cargo test -p eggsec --test feature_matrix
cargo test -p eggsec --test enforcement_matrix
cargo test -p eggsec --test enforced_dispatch_regression
cargo test -p eggsec-output --test report_envelope
bash scripts/check-architecture-guards.sh
```

> **Note**: The static guard script requires [ripgrep](https://github.com/BurntSushi/ripgrep) (`rg`). Install it locally before running: `cargo install ripgrep` or use your system package manager.

Alternatively, run the full architecture guard CI reproduction with a single Make target:

```bash
make check-architecture-ci
```

## Feature-Profile Compile Guards

Representative feature profiles are checked on every PR to catch compile regressions in optional domains. These are `cargo check` only (no test execution) to keep CI fast.

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

These checks are not required for PR merge. They may run on schedule, manually, or in a separate workflow.

| Check | Command | Notes |
|-------|---------|-------|
| Full workspace build | `cargo check --workspace --all-features` | May require all system deps |
| Full test suite | `cargo test --workspace --all-features` | Long-running, platform-sensitive |
| `full` feature profile | `cargo check -p eggsec --features full` | Aggregate of all non-default features |
| NSE tests | `cargo test -p eggsec --features nse --test nse_tests` | Requires libssl-dev |
| Stress tests | `cargo test -p eggsec --features stress-testing --test stress_tests` | Requires raw socket privileges |
| Integration tests | `cargo test -p eggsec --test '*.rs'` | May require network/wiremock |

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
- Fail on stale field names or contradictions in current docs.

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

## Key Invariants Guarded

The CI architecture guards enforce these invariants from Phases 1–10:

1. **OperationMetadata** is the canonical operation policy metadata layer.
2. **DomainDescriptor** is the canonical domain/integration grouping layer.
3. **ToolRegistration** distinguishes `mcp_metadata_exposable` from `mcp_default_visible`.
4. MCP OpsAgent uses Model A: profile-expanded metadata-exposable listing.
5. `mcp_tool_registrations_default_visible()` is the conservative default subset.
6. **CommandRegistration** separates visibility flags and dispatch modes.
7. **CommandDispatchMode** distinguishes RegistryBacked, LegacyWrapped, CatalogOnly, ServerLifecycle, HelperOnly.
8. TUI action specs point back to canonical metadata.
9. `eggsec-output` has a normalized report/evidence envelope.
10. Feature metadata snapshot is validated against `crates/eggsec/Cargo.toml`.
