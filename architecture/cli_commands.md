# CLI & Commands

The CLI and Commands layer is responsible for parsing user input, managing global state (`CommandContext`), and dispatching execution to the appropriate handlers.

Parent overview: [overview.md](overview.md). Related: [dispatch.md](dispatch.md), [config.md](config.md), [audit.md](audit.md), [tui.md](tui.md).

**Corrections (2026-10-06 re-verification)**: `CodeggMcp` no longer shares `command_id = "mcp-serve"` — it reports its own `"codegg-mcp"` (`cli/mod.rs:519`) while still sharing the single `mcp-serve` *registry* entry and handler. The operation catalog alias count was corrected 43 → **42** (34 canonical + 42 aliases). Every `Commands` variant line, `handle_command()` arm line, `cli/mod.rs` enum/`command_id()` span, `main.rs` step line, registry span, and test-range cite was re-derived from source and shifted. Counts of 52 variants / 50 match arms / 32 handler modules / 27 `cli/` files / 49 registry entries (29 + 13 + 7) all re-verified as correct.

## Role & Responsibilities

The CLI layer spans three crates with distinct responsibilities:

| Crate | Scope | Contents |
|-------|-------|----------|
| `crates/eggsec-cli/` | Binary shell | `main.rs` entry point, logging setup, daemon CLI intercept, surface resolution. No command types or handlers. |
| `crates/eggsec/src/cli/` | Argument types | `Cli` struct, `Commands` enum (52 variants), arg structs (`PortScanArgs`, `FuzzArgs`, etc.), `CommonHttpArgsCli`, `FuzzMode`. **27 files, types only — no handler logic.** |
| `crates/eggsec/src/commands/handlers/` | Execution logic | `handle_command()` exhaustive match (50 arms covering 52 variants at `handlers/mod.rs:430`, arms `handlers/mod.rs:466–553`), 32 handler modules, `CommandContext` struct, enforcement integration. |

**Critical invariant**: `crates/eggsec-cli/` never contains command types or handler logic. The CLI binary is a thin shell. All argument types live in the engine crate's `cli/` module; all handler logic lives in `commands/handlers/`.

---

## Binary Shell Flow (`crates/eggsec-cli/src/main.rs`)

The `main()` function (`:87`) executes this exact sequence:

| Step | Line(s) | Description |
|------|---------|-------------|
| 1 | `:88` | `eggsec::install_tls_provider()` — initialize rustls ring-only TLS backend |
| 2 | `:89` | `Cli::parse()` — clap argument parsing |
| 3 | `:91–94` | `--generate-config` early return — prints default config to stdout, exits |
| 4 | `:96–99` | `--generate-shell-completion` early return — generates shell completion script, exits |
| 5 | `:70`, `:110/:116`, `:123–131` | Rich-TUI launch intent resolved before logging (`rich_tui_launch_requested`, feature-split `tui`/no-`tui` call sites); `init_logging_with_console()` with `console_policy_for_launch(is_rich_tui_launch)` → `ConsoleLogging::Disabled` for TUI launches, `Enabled` otherwise — format from `--json` flag plus optional agent log directory |
| 6 | `:138–154` | TUI launch (feature `tui`) — when no command and stdout is a terminal; `--runtime daemon` maps to `RuntimeMode::Daemon`, an unknown runtime value falls back to `Embedded` with a pre-terminal `eprintln!` |
| 7 | `:157–163` | Headless fallback (no `tui` feature) — prints guidance to stderr when no command given |
| 8 | `:166–171` | Daemon client intercept (feature `daemon-client`) — `is_daemon_command()` (`main.rs:168`) routes `Daemon`/`Session`/`Task` variants to `daemon_cli::handle_daemon_command()` before general dispatch |
| 9 | `:173–174` | Config + scope loading — `load_config()` and `load_scope_with_source()` from `eggsec::config` |
| 10 | `:176` | Surface resolution — `resolve_execution_surface(&cli)` derives `ExecutionSurface` from command + flags |
| 11 | `:178–182` | `CommandContext` construction — builder chain: `new()` → `with_config_path()` → `with_execution_surface()` → `with_loaded_scope()` |
| 12 | `:186–198` | Manual override population — maps `--allow-*` CLI flags into `ManualOverride` struct, attached via `with_manual_override()` |
| 13 | `:200` | `handle_command(cli, &ctx).await` — dispatches to `commands/handlers/mod.rs` |

### Surface Resolution

Two compile-time variants of `resolve_execution_surface()` exist:

**With `rest-api` feature** (`main.rs:37–51`):

| Match Arm | Surface |
|-----------|---------|
| `Commands::Ci(_)` | `Ci` |
| `Commands::Agent(_)` | `SecurityAgent` |
| `Commands::McpServe(_)` \| `Commands::CodeggMcp(_)` | `McpServer` |
| `Commands::Serve(_)` | `RestApi` |
| `_ if cli.strict_scope` | `CliManualStrict` |
| `_` | `CliManual` |

**Without `rest-api` feature** (`main.rs:53–61`):

| Condition | Surface |
|-----------|---------|
| `Commands::Ci(_)` | `Ci` |
| `cli.strict_scope` | `CliManualStrict` |
| default | `CliManual` |

Key detail: `Commands::Grpc(_)` does **not** appear in `resolve_execution_surface()` — gRPC uses the default `CliManual` surface (the `grpc-api` feature does not gate surface resolution).

---

## Command Inventory

The `Commands` enum (`cli/mod.rs:271–472`) defines exactly **52 clap subcommand variants**: 27 unconditional + 25 feature-gated. Each variant has a `command_id()` method (`cli/mod.rs:479–563`) returning a stable kebab-case string used for registry lookup and diagnostics.

### Unconditional Commands (27)

| # | Variant | `command_id` | Line | Purpose | Handler (`handlers/mod.rs`) |
|---|---------|-------------|------|---------|----------------------------|
| 1 | `ScanPorts` | `scan-ports` | `:274` | TCP port scan | `handle_scan_ports` `:467` |
| 2 | `ScanEndpoints` | `scan-endpoints` | `:276` | HTTP endpoint discovery | `handle_scan_endpoints` `:468` |
| 3 | `Fingerprint` | `fingerprint` | `:278` | Service fingerprinting (AMAP-style) | `handle_fingerprint` `:469` |
| 4 | `Scan` | `scan` | `:280` | Chained security assessment pipeline | `handle_scan` `:477` |
| 5 | `Resume` | `resume` | `:282` | Resume previous scan from session file | `handle_resume` `:478` |
| 6 | `Fuzz` | `fuzz` | `:291` | Security fuzzing | `handle_fuzz` `:474` |
| 7 | `Waf` | `waf` | `:293` | WAF detection and evasion resistance | `handle_waf` `:476` |
| 8 | `WafStress` | `waf-stress` | `:295` | WAF stress testing | `handle_waf_stress` `:475` |
| 9 | `Graphql` | `graphql` | `:297` | GraphQL endpoint security | `handle_graphql` `:487` |
| 10 | `OAuth` | `oauth` | `:307` | OAuth/OIDC endpoint security | `handle_oauth` `:488` |
| 11 | `AuthTest` | `auth-test` | `:309` | Authentication testing (credential validation) | `handle_auth_test` `:489` |
| 12 | `Recon` | `recon` | `:313` | Reconnaissance information gathering | `handle_recon` `:479` |
| 13 | `Plan` | `plan` | `:317` | Preview execution plan (no execution) | `handle_plan` `:480` |
| 14 | `Preflight` | `preflight` | `:319` | Preview enforcement decision (no execution) | `handle_preflight` `:481` |
| 15 | `Ci` | `ci` | `:321` | CI/CD security checks mode | `handle_ci` `:482` |
| 16 | `Config` | `config` | `:323` | Validate configuration files | `handle_config` `:483` |
| 17 | `Doctor` | `doctor` | `:325` | System dependency diagnostics | `handle_doctor` `:484` |
| 18 | `PolicyExplain` | `policy-explain` | `:330` | Explain policy decisions for target + profile | `handle_policy_explain` `:485` |
| 19 | `ScopeExplain` | `scope-explain` | `:335` | Explain scope matching for a target | `handle_scope_explain` `:486` |
| 20 | `Load` | `load` | `:342` | HTTP load testing | `handle_load` `:466` |
| 21 | `Report` | `report` | `:352` | Convert and generate security reports | `handle_report` `:498` |
| 22 | `Vuln` | `vuln` | `:354` | Vulnerability management (CVSS, triage) | `handle_vuln` `:544` |
| 23 | `Storage` | `storage` | `:356` | Database storage and query operations | `handle_storage` `:545` |
| 24 | `Cluster` | `cluster` | `:384` | Distributed scanning cluster management | `handle_cluster` `:505` |
| 25 | `Notify` | `notify` | `:386` | Notification management and testing | `handle_notify` `:506` |
| 26 | `Remote` | `remote` | `:388` | Start remote listener for distributed commands | `handle_remote` `:507` |
| 27 | `Exec` | `exec` | `:390` | Execute commands on remote systems | `handle_exec` `:508` |

### Feature-Gated Commands (25)

| # | Variant | `command_id` | Feature | Line | Purpose | Handler (`handlers/mod.rs`) |
|---|---------|-------------|---------|------|---------|----------------------------|
| 28 | `Hunt` | `hunt` | `advanced-hunting` | `:287` | Advanced vulnerability hunting | `handle_hunt` `:473` |
| 29 | `Sbom` | `sbom` | `sbom` | `:338` | SBOM generation + supply chain check | `handle_sbom` `:491` |
| 30 | `Packet` | `packet` | `packet-inspection` | `:347` | Packet inspection and analysis | `handle_packet` `:493` |
| 31 | `Nse` | `nse` | `nse` | `:350` | Nmap NSE-compatible script execution | `handle_nse` `:471` |
| 32 | `ProxyIntercept` | `proxy-intercept` | `web-proxy` | `:366` | Interactive MITM web proxy | `web_proxy::handle_proxy_intercept` `:502` |
| 33 | `Stress` | `stress` | `stress-testing` | `:371` | Stress/load testing | `handle_stress` `:500` |
| 34 | `Proxy` | `proxy` | `stress-testing` | `:374` | Proxy pool and rotation management | `handle_proxy` `:504` |
| 35 | `Icmp` | `icmp` | `stress-testing` | `:377` | ICMP echo probes | `handle_icmp` `:495` |
| 36 | `Traceroute` | `traceroute` | `stress-testing` | `:380` | Network path tracing | `handle_traceroute` `:497` |
| 37 | `Serve` | `serve` | `rest-api` | `:393` | REST API server | `handle_serve` `:510` |
| 38 | `McpServe` | `mcp-serve` | `rest-api` | `:400` | MCP server for AI integration | `handle_mcp_serve` `:512` |
| 39 | `CodeggMcp` | `codegg-mcp` | `rest-api` | `:406` | MCP server for coding agent (stdio) | `handle_mcp_serve` (via `McpServeArgs` conversion) `:514–523` |
| 40 | `Agent` | `agent` | `rest-api` | `:415` | Scheduled security agent | `handle_agent` `:525` |
| 41 | `AiAnalyze` | `ai-analyze` | `ai-integration` | `:420` | Post-scan AI analysis | `handle_ai_analyze` `:527` |
| 42 | `Wireless` | `wireless` | `wireless` | `:425` | WiFi security scanning | `handle_wireless` `:529` |
| 43 | `Browser` | `browser` | `headless-browser` | `:430` | Headless browser security testing | `handle_browser` `:541` |
| 44 | `Mobile` | `mobile` | `mobile` | `:435` | APK/IPA static security analysis | `handle_mobile` `:537` |
| 45 | `Evasion` | `evasion` | `evasion` | `:440` | Evasion technique detection | `handle_evasion` `:531` |
| 46 | `Postex` | `postex` | `postex` | `:445` | Post-exploitation simulation | `handle_postex` `:533` |
| 47 | `C2` | `c2` | `c2` | `:450` | C2 framework simulation | `handle_c2` `:535` |
| 48 | `Db` | `db` | `db-pentest` | `:455` | Database pentesting (subcommand enum `DbCommand`) | `handle_db_pentest` `:539` |
| 49 | `Grpc` | `grpc` | `grpc-api` | `:460` | gRPC API server | `handle_grpc_server` `:543` |
| 50 | `Daemon` | `daemon` | `daemon-client` | `:465` | Daemon process management | daemon_cli intercept `main.rs:166–171` |
| 51 | `Session` | `session` | `daemon-client` | `:468` | Daemon session management | daemon_cli intercept `main.rs:166–171` |
| 52 | `Task` | `task` | `daemon-client` | `:471` | Daemon task management | daemon_cli intercept `main.rs:166–171` |

**Alias note**: `CodeggMcp` is a distinct subcommand with clap alias `mcp-codegg` (`:404`), and its `command_id()` deliberately returns `"codegg-mcp"` (`cli/mod.rs:514–519`) rather than `"mcp-serve"`, so trace/denial diagnostics name the surface that actually ran. It still shares the single `mcp-serve` **registry** entry (`registry.rs` has no `codegg-mcp` entry) and the same handler: the match arm converts `CodeggMcpArgs` → `McpServeArgs` before calling `handle_mcp_serve()` (`handlers/mod.rs:514–523`). `route_for_commands()` classifies both under `CommandRoute::Lifecycle` (`route.rs:255–261`).

**Daemon intercept note**: Variants 50–52 (`Daemon`, `Session`, `Task`) are intercepted in `main.rs:166–171` by `daemon_cli::is_daemon_command()` before reaching `handle_command()`. The `handle_command()` match arm for these variants (`handlers/mod.rs:548–553`) returns `anyhow::bail!()` and is unreachable in practice.

**Scan checkpoints note**: `Scan`/`Resume` are the two CLI ends of the checkpoint store. `--save-session` (`cli/scan.rs:366`) opts a scan into a resumable checkpoint written under `pipeline::session::default_session_dir()` — the same directory the TUI Resume picker lists (see [tui.md](tui.md)). Without it, the only checkpoint is the long-standing derivation from a `.session.json` `--output`.

---

## CommandContext

`CommandContext` (`handlers/mod.rs:107–121`) carries all global state for command execution:

```rust
pub struct CommandContext {
    pub config: EggsecConfig,
    pub scope: Scope,
    pub json: bool,
    config_path: Option<String>,
    pub notify_manager: NotifyManager,
    pub execution_profile: ExecutionProfile,
    pub execution_surface: ExecutionSurface,
    pub enforcement: EnforcementContext,
    pub manual_override: ManualOverride,
}
```

### Key Fields

| Field | Source | Description |
|-------|--------|-------------|
| `execution_surface` | `resolve_execution_surface()` in `main.rs` | Origin of the request (`CliManual`, `McpServer`, `Ci`, etc.) |
| `execution_profile` | `surface.profile()` | Derived from surface — **not** flag-based. `ManualPermissive` for default CLI, `McpStrict` for MCP, `AgentStrict` for agent, `CiStrict` for CI. |
| `enforcement` | `EnforcementContext::for_surface()` | Central authorization gate. Built from surface + policy + loaded scope. |
| `manual_override` | `--allow-*` CLI flags | Only effective under `ManualPermissive`. Strict profiles reject/ignore. |
| `config_path` | `--config` flag | Optional config file path for this session. |
| `notify_manager` | `NotifyManager::from_settings()` | Notification dispatch (webhooks, Slack, etc.). |

### Builder Methods

- `with_config_path(Option<String>)` — `:153`
- `with_execution_surface(ExecutionSurface)` — `:160` — also derives profile and rebuilds enforcement context
- `with_loaded_scope(LoadedScope)` — `:175` — rebuilds enforcement context with new scope
- `with_manual_override(ManualOverride)` — `:186`
- `describe_from_registry(command_id, target)` — `:198` — builds `OperationDescriptor` from registry metadata

---

## Handler Dispatch Flow

### Single-Owner Routing Contract (`commands/route.rs` + `handlers/mod.rs`)

`handle_command()` is an async function that:

1. **Classify once** via `route_for_commands()` (exhaustive over `Commands`, no wildcard): every variant becomes `Operation` (canonical ID), `Multiplexer` (branch-selected before approval), `Helper`, or `Lifecycle`. Aliases resolve here (`waf`→`waf-detect`, `load`→`load-test`, `scan`/`resume`→`pipeline`).
2. **Fail closed on stale routes**: every candidate operation must resolve to canonical `OperationMetadata`; unknown IDs bail before any handler runs (replaces the old registry-bridge validation prelude, removed in Phase 1).
3. **Boundary conversion match**: the exhaustive `Commands` match remains as the Clap-DTO→handler boundary conversion (compile-time safe), not a second routing owner. Execution ownership lives in `dispatch::canonical_execution`.

Phase 1 completion: the old dual ownership (registry prelude + separate handler match claiming routing) is gone; registry metadata and `CommandRoute` classification are pinned together by test (`registry_operation_backed_agrees_with_route`).

Phase 2 fix: `FuzzArgs::session` (`--session`, HTTP cookie handling) collided
with the global daemon `--session <ID>` (`Option<String>`): same Clap ID,
different types, so every successful `fuzz` parse panicked ("Could not
downcast to String, need bool"). The fuzz flag is now `--http-session`
(explicit `id = "http-session"`); the global daemon flag keeps `--session`.
`TUI copy-cli` round-trip tests (`app::surface_wiring`, parsing generated
argv through the real `Cli`) pin this: generated equivalents only emit flags
the real tree accepts (`--json` for Json-mapped tabs, `--format` only for
commands that declare it).

### Enforcement Integration

Side-effecting handlers call `ctx.evaluate_and_enforce_operation(descriptor)` (`CommandContext` `handlers/mod.rs:215–420`) which wraps `EnforcementContext::evaluate()`:

| Outcome | ManualPermissive (CLI/TUI) | Strict (CI/MCP/Agent) |
|---------|---------------------------|----------------------|
| `Allow(decision)` | Emit audit event, proceed | Emit audit event, proceed |
| `Warn(decision)` | Emit audit event, log warnings, proceed | Emit audit event, log warnings, proceed |
| `RequireConfirmation(decision)` | Check `manual_override` permits all `ConfirmationClass`es. If permitted, record override and proceed. If not, return error listing exact `--allow-*` flags needed. | Hard denial — return error. |
| `Deny(decision)` | Emit audit event, return error (JSON or human-readable) | Emit audit event, return error |

**No `approve_manual()` call in CLI path**: The CLI flow is `evaluate()` → outcome match. `ApprovedOperation` tokens are only produced for strict surfaces via `EnforcementContext::approve()`.

### Handler Module Files (32 modules)

| Module File | Commands Handled | Feature Gate |
|-------------|-----------------|--------------|
| `scan.rs` | `scan-ports`, `scan-endpoints`, `fingerprint`, `scan`, `resume`, `nse` | — |
| `recon.rs` | `recon` | — |
| `fuzz.rs` | `fuzz`, `waf`, `waf-stress`, `graphql`, `oauth` | — |
| `load.rs` | `load` | — |
| `network.rs` | `packet`, `icmp`, `traceroute` | — |
| `report.rs` | `report` | — |
| `vuln.rs` | `vuln` | — |
| `storage.rs` | `storage` | — |
| `config.rs` | `config` | — |
| `doctor.rs` | `doctor` | — |
| `explain.rs` | `policy-explain`, `scope-explain` | — |
| `plan.rs` | `plan` | — |
| `preflight.rs` | `preflight` | — |
| `ci.rs` | `ci` | — |
| `cluster.rs` | `cluster` | — |
| `notify.rs` | `notify` | — |
| `auth_test.rs` | `auth-test` | — |
| `stress.rs` | `stress`, `proxy` | `stress-testing` |
| `sbom.rs` | `sbom` | `sbom` |
| `serve.rs` | `serve` | `rest-api` |
| `agent.rs` | `agent`, `mcp-serve`, `codegg-mcp` | `rest-api` |
| `grpc.rs` | `grpc` | `grpc-api` |
| `mobile.rs` | `mobile` | `mobile` |
| `wireless.rs` | `wireless` | `wireless` |
| `db_pentest.rs` | `db` | `db-pentest` |
| `evasion.rs` | `evasion` | `evasion` |
| `postex.rs` | `postex` | `postex` |
| `c2.rs` | `c2` | `c2` |
| `web_proxy.rs` | `proxy-intercept` | `web-proxy` |
| `browser.rs` | `browser` | `headless-browser` |
| `hunt.rs` | `hunt` | `advanced-hunting` |
| `ai_analyze.rs` | `ai-analyze` | `ai-integration` |

### Handler Patterns

All side-effecting commands use `describe_from_registry()` to build descriptors from canonical `OperationMetadata`:

```rust
// All operation-backed commands (preferred pattern)
pub async fn handle_recon(ctx: &CommandContext, args: ReconArgs) -> Result<()> {
    let descriptor = ctx
        .describe_from_registry("recon", Some(target))
        .ok_or_else(|| anyhow::anyhow!("No registry metadata for command"))?;
    let decision = ctx.evaluate_and_enforce_operation(descriptor)?;
    // proceed with dispatch
    Ok(())
}

// Config/helper (no enforcement)
pub async fn handle_config(_ctx: &CommandContext, args: ConfigArgs) -> Result<()> {
    load_config(config_path)?;
    Ok(())
}
```

---

## Command Registry (`commands/registry.rs`) + Routing Contract (`commands/route.rs`)

The command registry provides static, inspectable metadata for CLI/TUI dispatch. `commands/route.rs` (Phase 1) is the single coherent owner for the `Commands → CommandRoute` conversion alongside the registry: static metadata plus one adjacent exhaustive conversion, not function pointers with heterogeneous args in metadata.

**The registry/route pair is metadata and routing, not authorization.** All side-effecting operations still flow through `EnforcementContext::evaluate()` before execution, then reach the canonical execution boundary (`dispatch::execute_approved`).

### Registry Entry Count

The `REGISTERED_COMMANDS` array (`registry.rs:111–708`) contains **49 entries**. Categories:

| Dispatch Mode | Count | Commands |
|--------------|-------|----------|
| `RegistryBacked` | 29 | `recon`, `scan-ports`, `scan-endpoints`, `fingerprint`, `scan`, `resume`, `fuzz`, `waf`, `waf-stress`, `graphql`, `oauth`, `auth-test`, `load`, `stress`, `packet`, `icmp`, `traceroute`, `nse`, `hunt`, `evasion`, `postex`, `c2`, `proxy-intercept`, `wireless`, `wireless-deauth`, `browser`, `mobile`, `mobile-dynamic`, `db` |
| `HelperOnly` | 13 | `plan`, `preflight`, `ci`, `config`, `doctor`, `policy-explain`, `scope-explain`, `ai-analyze`, `report`, `vuln`, `storage`, `sbom`, `notify` |
| `ServerLifecycle` | 7 | `serve`, `mcp-serve`, `agent`, `grpc`, `cluster`, `remote-serve`, `exec` |
| `CatalogOnly` | 0 | (none currently) |

### Registry API

| Function | Location | Purpose |
|----------|----------|---------|
| `lookup_command(command_id)` | `:712` | Find `CommandRegistration` by ID |
| `build_descriptor_for_command(command_id, target)` | `:721` | Build `OperationDescriptor` from registry metadata |
| `all_command_ids()` | `:729` | All registered command IDs |
| `tui_visible_command_ids()` | `:734` | TUI-visible commands |
| `cli_interactive_only_command_ids()` | `:744` | CLI-helper-only commands |
| `registry_backed_command_ids()` | `:756` | Registry-backed dispatch commands |
| `suggest_command(unknown)` | `:775` | Levenshtein-based suggestions (edit distance ≤ 3) |

### Types

#### `CommandRegistration`

```rust
pub struct CommandRegistration {
    pub command_id: &'static str,
    pub operation_id: Option<&'static str>,
    pub display_name: &'static str,
    pub category: CommandCategory,
    pub feature: Option<&'static str>,
    pub cli_visible: bool,
    pub tui_visible: bool,
    pub programmatic_visible: bool,
    pub cli_interactive_only: bool,
    pub dispatch_mode: CommandDispatchMode,
}
```

#### `CommandCategory`

| Variant | String | Description |
|---------|--------|-------------|
| `SideEffectingNetwork` | `"side-effecting-network"` | Network operations requiring enforcement |
| `LocalFileDomain` | `"local-file-domain"` | Local file or domain-specific operations |
| `PassiveAnalytical` | `"passive-analytical"` | Read-only analysis |
| `ConfigOutputHelper` | `"config-output-helper"` | Configuration, diagnostics |
| `FrontendServer` | `"frontend-server"` | Server daemons |
| `LegacySpecial` | `"legacy-special"` | Commands with no metadata or unique dispatch |

#### `CommandDispatchMode`

| Variant | Description |
|---------|-------------|
| `RegistryBacked` | Canonical operation-backed dispatch via registry metadata (`OperationMetadata` → `describe_from_registry()` → `EnforcementContext` → canonical dispatcher) |
| `CatalogOnly` | Listed for discoverability, never dispatched |
| `ServerLifecycle` | Server lifecycle command |
| `HelperOnly` | Read-only helper/diagnostic |

---

## Policy Preview Commands

Three commands evaluate policy **without sending network traffic** — they are read-only diagnostic surfaces:

### `plan` (`handlers/mod.rs:501`)

Previews the execution plan for a target + profile combination. Shows what stages would run, in what order, with what configuration. No enforcement evaluation.

### `preflight` (`handlers/mod.rs:502`)

Previews the enforcement decision for a specific operation. Builds an `OperationDescriptor` and calls `evaluate_and_enforce_operation()`, showing the outcome (Allow/Warn/Deny/RequireConfirmation) without executing. Useful for CI debugging and policy validation.

### `policy-explain` / `scope-explain` (`handlers/mod.rs:506–507`)

Explain commands provide human-readable explanations:
- `policy-explain`: Evaluates what would happen for a target + profile (operation mode, risk level, scope matching, required features, policy blocks)
- `scope-explain`: Evaluates whether a target falls within configured scope (rule matches, exclusions, private-IP detection)

Both are `PassiveAnalytical` / `HelperOnly` in the registry and `cli_interactive_only`.

---

## Logging Setup (`crates/eggsec-cli/src/logging.rs`)

`init_logging_with_console(format, log_dir, console)` configures the `tracing` subscriber; `init_logging()` remains as a compat wrapper (`ConsoleLogging::Enabled`). See [logging.md](logging.md) for the full console emission policy.

### Format Selection

| Condition | Format | Behavior (when console enabled) |
|-----------|--------|---------------------------------|
| `--json` flag | `LogFormat::Json` | JSON output with span events, thread IDs, thread names |
| Default (no flag) | `LogFormat::Pretty` | Pretty-printed output with targets and line numbers |
| `Compact` (dead code) | `LogFormat::Compact` | Compact output — **defined but never selected by CLI flags** |

Rich TUI launches use `ConsoleLogging::Disabled`: no console formatter is constructed (file-only JSON when `log_dir` is `Some`, silent otherwise).

### Filter

- Uses `EnvFilter::try_from_default_env()` — respects `RUST_LOG` environment variable
- Default filter: `"info"` level

### Agent Log Appender

When the `Agent` command is used, `agent_log_dir()` returns a log directory path (`<memory_dir>/logs`). This enables:
- Daily rolling file appender (`tracing_appender::rolling::Rotation::DAILY`)
- Non-blocking writer (`tracing_appender::non_blocking`)
- JSON format for file output (`.json` extension, no ANSI, thread IDs, file + line)
- File layer runs alongside the console layer on console-enabled surfaces; on TUI-disabled surfaces it runs file-only

The `WorkerGuard` returned by the init functions must be held for the lifetime of the process to keep the non-blocking writer alive.

---

## Shell Completion & Config Generation

### `--generate-config` (`main.rs:91–94`)

Prints the default TOML configuration to stdout and exits immediately (before logging init). Implementation: `eggsec::config::get_default_config()`.

### `--generate-shell-completion` (`main.rs:96–99`)

Uses `clap_complete::generate()` to emit shell completion scripts. Supports all shells in `clap_complete::Shell` (Bash, Zsh, Fish, Elvish, PowerShell). Output goes to stdout.

---

## Integration Points

### Command Dispatch (`dispatch.md`)

CLI handlers call engine functions directly or via the dispatch layer. The dispatch layer (`crates/eggsec/src/dispatch/`) converts `TaskKind` requests into engine module calls. CLI handlers that use operation-backed dispatch build `OperationDescriptor` from registry metadata via `describe_from_registry()`.

### Configuration (`config.md`)

- `EggsecConfig` loaded from TOML/YAML via `load_config()`
- `Scope` / `LoadedScope` loaded via `load_scope_with_source()`
- `ExecutionPolicy` from config determines which risk tiers are permitted
- `OperationMetadata` (34 canonical + 42 aliases) is the single source of truth for operation policy

### Enforcement & Audit (`audit.md`)

Every `evaluate_and_enforce_operation()` call emits an `EnforcementAuditEvent` via `emit_audit_event()`. Events capture surface, descriptor, outcome, override details, and confirmation classes for the audit trail.

### TUI (`tui.md`)

When no command is given and stdout is a terminal (feature `tui`), the CLI launches the TUI via `eggsec_tui::run_with_mode()`. The TUI supports two runtime modes:
- `Embedded` (default) — direct engine execution
- `Daemon` — connects to a running daemon over Unix socket

---

## Testing

### Integration Test: `command_registry.rs`

`crates/eggsec/tests/command_registry.rs` validates:
- Registry entries have unique command IDs
- All `operation_id` values resolve to `OperationMetadata`
- Feature-gated entries declare non-empty feature strings
- Registry-backed side-effecting commands build descriptors
- Helper/server commands don't require descriptors
- `cli_interactive_only` commands are not `programmatic_visible`
- Pilot commands (`recon`, `scan-ports`, `scan-endpoints`, `fingerprint`) have correct metadata
- Suggestion algorithm works for close matches
- Category classification consistency

### Unit Tests

- `cli/mod.rs:680–1038` — `ScanProfile` risk budget ordering, operation mode derivation (14 tests)
- `commands/handlers/mod.rs:574–1663` — `CommandContext` enforcement behavior (49 tests): safe/active/intrusive/stress/raw-packet/load-test/remote-execution allowed/denied with various policy flags, JSON mode denial structure, manual override flag semantics, scope evaluation
- `commands/registry.rs:815–1040` — Registry invariant tests (18 tests): unique IDs, metadata resolution, feature gates, operation-backed dispatch mode consistency, pilot commands, dispatch mode consistency

---

## Invariants & Gotchas

1. **Handlers live in engine crate, not `eggsec-cli`**: `crates/eggsec/src/commands/handlers/` contains all 32 handler modules. `crates/eggsec-cli/` is the binary shell only (main, logging, daemon_cli).

2. **`cli/` holds types only**: `crates/eggsec/src/cli/` contains `Cli`, `Commands`, and arg structs. No handler logic. This separation allows the TUI and other frontends to reuse CLI types without depending on handler code.

3. **Exhaustive match**: `handle_command()` has no wildcard arm. Adding/removing `Commands` variants is a compile-time error until the match is updated.

4. **Daemon commands are intercepted early**: `Daemon`/`Session`/`Task` variants are handled in `main.rs:166–171` before `handle_command()`. Their match arm in `handlers/mod.rs:548–553` is dead code that bails with an error.

5. **`CodeggMcp` reports its own command ID**: `command_id()` returns `"codegg-mcp"` (`cli/mod.rs:519`), not `"mcp-serve"` — diagnostics must name the surface that ran. The *registry* has only one `mcp-serve` entry (there is no `codegg-mcp` registration), and both variants route to the same handler through the `CodeggMcpArgs` → `McpServeArgs` conversion (`handlers/mod.rs:514–523`).

6. **`--yes` is narrow**: Only permits `OutOfScope` and `TargetExpansion` confirmation classes (`handlers/mod.rs:366–372`). High-risk, exclusions, private resolution, cross-host redirect, and non-baseline capabilities require dedicated `--allow-*` flags.

7. **`evaluate_and_enforce_operation()` returns `PolicyDecision`, not `ApprovedOperation`**: The CLI manual path does not produce approval tokens. Tokens are only for strict surfaces via `EnforcementContext::approve()`.

8. **Grpc does not affect surface resolution**: `Commands::Grpc` is not matched in `resolve_execution_surface()` — it uses the default `CliManual` surface.

9. **`LogFormat::Compact` is dead code**: Defined in `logging.rs:15` but never selected by any CLI flag. Only `Pretty` (default) and `Json` (`--json`) are used.

10. **Double `#[cfg]` on `Db` variant**: `cli/mod.rs:536–538` has two stacked `#[cfg(feature = "db-pentest")]` attributes on the `Self::Db` match arm. This compiles but is redundant.

---

## Bugs Found (Report Only)

| # | File:Line | Description | Severity |
|---|-----------|-------------|----------|
| 1 | `logging.rs:15` | `LogFormat::Compact` variant is dead code — defined but never constructed by CLI flags. Only `Pretty` and `Json` are used. (Annotated `#[allow(dead_code)]` for programmatic callers.) | Low |
| 2 | `cli/mod.rs:536–538` | Double `#[cfg(feature = "db-pentest")]` on `Self::Db` match arm in `command_id()`. Redundant attribute, compiles but unnecessary. | Cosmetic |
| 3 | `handlers/mod.rs:548–553` | `Daemon`/`Session`/`Task` match arm is dead code — intercepted in `main.rs:166–171` before `handle_command()` is reached. The `bail!()` is unreachable. | Informational |

---

*Last verified against source: 2026-10-06 (full re-verification: all 52 variant lines, 50 handler arm lines, `main.rs` flow steps, registry counts/spans, catalog counts, test ranges, CodeggMcp command ID); earlier passes 2026-08-25 / 2026-09-25*
