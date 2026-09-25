# CLI & Commands

The CLI and Commands layer is responsible for parsing user input, managing global state (`CommandContext`), and dispatching execution to the appropriate handlers.

Parent overview: [overview.md](overview.md). Related: [dispatch.md](dispatch.md), [config.md](config.md), [audit.md](audit.md), [tui.md](tui.md).

## Role & Responsibilities

The CLI layer spans three crates with distinct responsibilities:

| Crate | Scope | Contents |
|-------|-------|----------|
| `crates/eggsec-cli/` | Binary shell | `main.rs` entry point, logging setup, daemon CLI intercept, surface resolution. No command types or handlers. |
| `crates/eggsec/src/cli/` | Argument types | `Cli` struct, `Commands` enum (52 variants), arg structs (`PortScanArgs`, `FuzzArgs`, etc.), `CommonHttpArgsCli`, `FuzzMode`. **27 files, types only — no handler logic.** |
| `crates/eggsec/src/commands/handlers/` | Execution logic | `handle_command()` exhaustive match (50 arms covering 52 variants at `:487`), 32 handler modules, `CommandContext` struct, enforcement integration. |

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
| 5 | `main.rs` | Rich-TUI launch intent resolved before logging (`rich_tui_launch_requested` at `:70`); `init_logging_with_console()` with `ConsoleLogging::Disabled` for TUI launches, `Enabled` otherwise — format from `--json` flag plus optional agent log directory |
| 6 | `main.rs` | TUI launch (feature `tui`) — when no command and stdout is a terminal; supports `--runtime daemon` mode |
| 7 | `:160–166` | Headless fallback (no `tui` feature) — prints guidance to stderr when no command given |
| 8 | `:168–172` | Daemon client intercept (feature `daemon-client`) — `is_daemon_command()` routes `Daemon`/`Session`/`Task` variants to `daemon_cli::handle_daemon_command()` before general dispatch |
| 9 | `:174–175` | Config + scope loading — `load_config()` and `load_scope_with_source()` from `eggsec::config` |
| 10 | `:177` | Surface resolution — `resolve_execution_surface(&cli)` derives `ExecutionSurface` from command + flags |
| 11 | `:179–183` | `CommandContext` construction — builder chain: `new()` → `with_config_path()` → `with_execution_surface()` → `with_loaded_scope()` |
| 12 | `:187–197` | Manual override population — maps `--allow-*` CLI flags into `ManualOverride` struct, attached via `with_manual_override()` |
| 13 | `:199` | `handle_command(cli, &ctx).await` — dispatches to `commands/handlers/mod.rs` |

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

The `Commands` enum (`cli/mod.rs:270–470`) defines exactly **52 clap subcommand variants**: 27 unconditional + 25 feature-gated. Each variant has a `command_id()` method (`cli/mod.rs:472–559`) returning a stable kebab-case string used for registry lookup and diagnostics.

### Unconditional Commands (27)

| # | Variant | `command_id` | Line | Purpose | Handler (`handlers/mod.rs`) |
|---|---------|-------------|------|---------|----------------------------|
| 1 | `ScanPorts` | `scan-ports` | `:273` | TCP port scan | `handle_scan_ports` `:488` |
| 2 | `ScanEndpoints` | `scan-endpoints` | `:275` | HTTP endpoint discovery | `handle_scan_endpoints` `:489` |
| 3 | `Fingerprint` | `fingerprint` | `:277` | Service fingerprinting (AMAP-style) | `handle_fingerprint` `:490` |
| 4 | `Scan` | `scan` | `:279` | Chained security assessment pipeline | `handle_scan` `:498` |
| 5 | `Resume` | `resume` | `:281` | Resume previous scan from session file | `handle_resume` `:499` |
| 6 | `Fuzz` | `fuzz` | `:290` | Security fuzzing | `handle_fuzz` `:495` |
| 7 | `Waf` | `waf` | `:292` | WAF detection and evasion resistance | `handle_waf` `:497` |
| 8 | `WafStress` | `waf-stress` | `:294` | WAF stress testing | `handle_waf_stress` `:496` |
| 9 | `Graphql` | `graphql` | `:296` | GraphQL endpoint security | `handle_graphql` `:508` |
| 10 | `OAuth` | `oauth` | `:306` | OAuth/OIDC endpoint security | `handle_oauth` `:509` |
| 11 | `AuthTest` | `auth-test` | `:308` | Authentication testing (credential validation) | `handle_auth_test` `:510` |
| 12 | `Recon` | `recon` | `:312` | Reconnaissance information gathering | `handle_recon` `:500` |
| 13 | `Plan` | `plan` | `:316` | Preview execution plan (no execution) | `handle_plan` `:501` |
| 14 | `Preflight` | `preflight` | `:318` | Preview enforcement decision (no execution) | `handle_preflight` `:502` |
| 15 | `Ci` | `ci` | `:320` | CI/CD security checks mode | `handle_ci` `:503` |
| 16 | `Config` | `config` | `:322` | Validate configuration files | `handle_config` `:504` |
| 17 | `Doctor` | `doctor` | `:324` | System dependency diagnostics | `handle_doctor` `:505` |
| 18 | `PolicyExplain` | `policy-explain` | `:329` | Explain policy decisions for target + profile | `handle_policy_explain` `:506` |
| 19 | `ScopeExplain` | `scope-explain` | `:334` | Explain scope matching for a target | `handle_scope_explain` `:507` |
| 20 | `Load` | `load` | `:341` | HTTP load testing | `handle_load` `:487` |
| 21 | `Report` | `report` | `:351` | Convert and generate security reports | `handle_report` `:519` |
| 22 | `Vuln` | `vuln` | `:353` | Vulnerability management (CVSS, triage) | `handle_vuln` `:565` |
| 23 | `Storage` | `storage` | `:355` | Database storage and query operations | `handle_storage` `:566` |
| 24 | `Cluster` | `cluster` | `:383` | Distributed scanning cluster management | `handle_cluster` `:526` |
| 25 | `Notify` | `notify` | `:385` | Notification management and testing | `handle_notify` `:527` |
| 26 | `Remote` | `remote` | `:387` | Start remote listener for distributed commands | `handle_remote` `:528` |
| 27 | `Exec` | `exec` | `:389` | Execute commands on remote systems | `handle_exec` `:529` |

### Feature-Gated Commands (25)

| # | Variant | `command_id` | Feature | Line | Purpose | Handler (`handlers/mod.rs`) |
|---|---------|-------------|---------|------|---------|----------------------------|
| 28 | `Hunt` | `hunt` | `advanced-hunting` | `:286` | Advanced vulnerability hunting | `handle_hunt` `:494` |
| 29 | `Sbom` | `sbom` | `sbom` | `:337` | SBOM generation + supply chain check | `handle_sbom` `:512` |
| 30 | `Packet` | `packet` | `packet-inspection` | `:346` | Packet inspection and analysis | `handle_packet` `:514` |
| 31 | `Nse` | `nse` | `nse` | `:349` | Nmap NSE-compatible script execution | `handle_nse` `:492` |
| 32 | `ProxyIntercept` | `proxy-intercept` | `web-proxy` | `:365` | Interactive MITM web proxy | `web_proxy::handle_proxy_intercept` `:523` |
| 33 | `Stress` | `stress` | `stress-testing` | `:370` | Stress/load testing | `handle_stress` `:521` |
| 34 | `Proxy` | `proxy` | `stress-testing` | `:373` | Proxy pool and rotation management | `handle_proxy` `:525` |
| 35 | `Icmp` | `icmp` | `stress-testing` | `:376` | ICMP echo probes | `handle_icmp` `:516` |
| 36 | `Traceroute` | `traceroute` | `stress-testing` | `:379` | Network path tracing | `handle_traceroute` `:518` |
| 37 | `Serve` | `serve` | `rest-api` | `:392` | REST API server | `handle_serve` `:531` |
| 38 | `McpServe` | `mcp-serve` | `rest-api` | `:398` | MCP server for AI integration | `handle_mcp_serve` `:533` |
| 39 | `CodeggMcp` | `mcp-serve` | `rest-api` | `:404` | MCP server for coding agent (stdio) | `handle_mcp_serve` (via `McpServeArgs` conversion) `:535–544` |
| 40 | `Agent` | `agent` | `rest-api` | `:413` | Scheduled security agent | `handle_agent` `:546` |
| 41 | `AiAnalyze` | `ai-analyze` | `ai-integration` | `:418` | Post-scan AI analysis | `handle_ai_analyze` `:548` |
| 42 | `Wireless` | `wireless` | `wireless` | `:423` | WiFi security scanning | `handle_wireless` `:550` |
| 43 | `Browser` | `browser` | `headless-browser` | `:428` | Headless browser security testing | `handle_browser` `:562` |
| 44 | `Mobile` | `mobile` | `mobile` | `:433` | APK/IPA static security analysis | `handle_mobile` `:558` |
| 45 | `Evasion` | `evasion` | `evasion` | `:438` | Evasion technique detection | `handle_evasion` `:552` |
| 46 | `Postex` | `postex` | `postex` | `:443` | Post-exploitation simulation | `handle_postex` `:554` |
| 47 | `C2` | `c2` | `c2` | `:448` | C2 framework simulation | `handle_c2` `:556` |
| 48 | `Db` | `db` | `db-pentest` | `:453` | Database pentesting (subcommand enum `DbCommand`) | `handle_db_pentest` `:560` |
| 49 | `Grpc` | `grpc` | `grpc-api` | `:458` | gRPC API server | `handle_grpc_server` `:564` |
| 50 | `Daemon` | `daemon` | `daemon-client` | `:463` | Daemon process management | daemon_cli intercept `:168–172` |
| 51 | `Session` | `session` | `daemon-client` | `:466` | Daemon session management | daemon_cli intercept `:168–172` |
| 52 | `Task` | `task` | `daemon-client` | `:469` | Daemon task management | daemon_cli intercept `:168–172` |

**Alias note**: Both `McpServe` and `CodeggMcp` map to `command_id = "mcp-serve"` (`cli/mod.rs:511–513`). The `CodeggMcp` variant also has clap alias `mcp-codegg` (`:404`). In the handler match, `CodeggMcp` converts its `CodeggMcpArgs` to `McpServeArgs` before calling `handle_mcp_serve()` (`handlers/mod.rs:535–544`).

**Daemon intercept note**: Variants 50–52 (`Daemon`, `Session`, `Task`) are intercepted in `main.rs:168–172` by `daemon_cli::is_daemon_command()` before reaching `handle_command()`. The `handle_command()` match arm for these variants (`handlers/mod.rs:569–574`) returns `anyhow::bail!()` and is unreachable in practice.

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

Side-effecting handlers call `ctx.evaluate_and_enforce_operation(descriptor)` (`CommandContext` `:215–440`) which wraps `EnforcementContext::evaluate()`:

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
| `stress.rs` | `stress`, `proxy`, `icmp`, `traceroute` | `stress-testing` |
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

The `REGISTERED_COMMANDS` array (`registry.rs:111–703`) contains **49 entries**. Categories:

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
- `OperationMetadata` (34 canonical + 43 aliases) is the single source of truth for operation policy

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

- `cli/mod.rs:674–858` — `ScanProfile` risk budget ordering, operation mode derivation
- `commands/handlers/mod.rs:600–1634` — `CommandContext` enforcement behavior (48 tests): safe/active/intrusive/stress/raw-packet/load-test/remote-execution allowed/denied with various policy flags, JSON mode denial structure, manual override flag semantics, scope evaluation
- `commands/registry.rs:815–1040` — Registry invariant tests (18 tests): unique IDs, metadata resolution, feature gates, operation-backed dispatch mode consistency, pilot commands, dispatch mode consistency

---

## Invariants & Gotchas

1. **Handlers live in engine crate, not `eggsec-cli`**: `crates/eggsec/src/commands/handlers/` contains all 32 handler modules. `crates/eggsec-cli/` is the binary shell only (main, logging, daemon_cli).

2. **`cli/` holds types only**: `crates/eggsec/src/cli/` contains `Cli`, `Commands`, and arg structs. No handler logic. This separation allows the TUI and other frontends to reuse CLI types without depending on handler code.

3. **Exhaustive match**: `handle_command()` has no wildcard arm. Adding/removing `Commands` variants is a compile-time error until the match is updated.

4. **Daemon commands are intercepted early**: `Daemon`/`Session`/`Task` variants are handled in `main.rs:168–172` before `handle_command()`. Their match arm in `handlers/mod.rs:569–574` is dead code that bails with an error.

5. **`CodeggMcp` alias collision**: Both `McpServe` and `CodeggMcp` map to `command_id = "mcp-serve"` (`cli/mod.rs:511–513`). The registry has only one `mcp-serve` entry. This is intentional — they share the same MCP server handler.

6. **`--yes` is narrow**: Only permits `OutOfScope` and `TargetExpansion` confirmation classes. High-risk, exclusions, private resolution, cross-host redirect, and non-baseline capabilities require dedicated `--allow-*` flags (`handlers/mod.rs:313–396`).

7. **`evaluate_and_enforce_operation()` returns `PolicyDecision`, not `ApprovedOperation`**: The CLI manual path does not produce approval tokens. Tokens are only for strict surfaces via `EnforcementContext::approve()`.

8. **Grpc does not affect surface resolution**: `Commands::Grpc` is not matched in `resolve_execution_surface()` — it uses the default `CliManual` surface.

9. **`LogFormat::Compact` is dead code**: Defined in `logging.rs:15` but never selected by any CLI flag. Only `Pretty` (default) and `Json` (`--json`) are used.

10. **Double `#[cfg]` on `Db` variant**: `cli/mod.rs:530–532` has two stacked `#[cfg(feature = "db-pentest")]` attributes on the `Self::Db` match arm. This compiles but is redundant.

---

## Bugs Found (Report Only)

| # | File:Line | Description | Severity |
|---|-----------|-------------|----------|
| 1 | `logging.rs:15` | `LogFormat::Compact` variant is dead code — defined but never constructed by CLI flags. Only `Pretty` and `Json` are used. | Low |
| 2 | `cli/mod.rs:530–532` | Double `#[cfg(feature = "db-pentest")]` on `Self::Db` match arm in `command_id()`. Redundant attribute, compiles but unnecessary. | Cosmetic |
| 3 | `handlers/mod.rs:569–574` | `Daemon`/`Session`/`Task` match arm is dead code — intercepted in `main.rs` before `handle_command()` is reached. The `bail!()` is unreachable. | Informational |

---

*Last verified against source: 2026-08-25; variant/handler/registry line cites, handler-module mapping, test counts corrected 2026-09-25 (systematic review)*
