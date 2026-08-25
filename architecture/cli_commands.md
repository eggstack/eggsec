# CLI & Commands

The CLI and Commands layer is responsible for parsing user input, managing global state (`CommandContext`), and dispatching execution to the appropriate handlers.

Parent overview: [overview.md](overview.md). Related: [dispatch.md](dispatch.md), [config.md](config.md), [audit.md](audit.md), [tui.md](tui.md).

## Role & Responsibilities

The CLI layer spans three crates with distinct responsibilities:

| Crate | Scope | Contents |
|-------|-------|----------|
| `crates/eggsec-cli/` | Binary shell | `main.rs` entry point, logging setup, daemon CLI intercept, surface resolution. No command types or handlers. |
| `crates/eggsec/src/cli/` | Argument types | `Cli` struct, `Commands` enum (52 variants), arg structs (`PortScanArgs`, `FuzzArgs`, etc.), `CommonHttpArgsCli`, `FuzzMode`. **27 files, types only — no handler logic.** |
| `crates/eggsec/src/commands/handlers/` | Execution logic | `handle_command()` exhaustive match (52 arms at `:489`), 32 handler modules, `CommandContext` struct, enforcement integration. |

**Critical invariant**: `crates/eggsec-cli/` never contains command types or handler logic. The CLI binary is a thin shell. All argument types live in the engine crate's `cli/` module; all handler logic lives in `commands/handlers/`.

---

## Binary Shell Flow (`crates/eggsec-cli/src/main.rs`)

The `main()` function (`:64`) executes this exact sequence:

| Step | Line(s) | Description |
|------|---------|-------------|
| 1 | `:65` | `eggsec::install_tls_provider()` — initialize rustls ring-only TLS backend |
| 2 | `:66` | `Cli::parse()` — clap argument parsing |
| 3 | `:68–71` | `--generate-config` early return — prints default config to stdout, exits |
| 4 | `:73–76` | `--generate-shell-completion` early return — generates shell completion script, exits |
| 5 | `:78–86` | `init_logging()` — configures tracing subscriber based on `--json` flag and optional agent log directory |
| 6 | `:89–105` | TUI launch (feature `tui`) — when no command and stdout is a terminal; supports `--runtime daemon` mode |
| 7 | `:108–114` | Headless fallback (no `tui` feature) — prints guidance to stderr when no command given |
| 8 | `:117–122` | Daemon client intercept (feature `daemon-client`) — `is_daemon_command()` routes `Daemon`/`Session`/`Task` variants to `daemon_cli::handle_daemon_command()` before general dispatch |
| 9 | `:124–125` | Config + scope loading — `load_config()` and `load_scope_with_source()` from `eggsec::config` |
| 10 | `:127` | Surface resolution — `resolve_execution_surface(&cli)` derives `ExecutionSurface` from command + flags |
| 11 | `:129–133` | `CommandContext` construction — builder chain: `new()` → `with_config_path()` → `with_execution_surface()` → `with_loaded_scope()` |
| 12 | `:137–149` | Manual override population — maps `--allow-*` CLI flags into `ManualOverride` struct, attached via `with_manual_override()` |
| 13 | `:151` | `handle_command(cli, &ctx).await` — dispatches to `commands/handlers/mod.rs` |

### Surface Resolution

Two compile-time variants of `resolve_execution_surface()` exist:

**With `rest-api` feature** (`main.rs:38–50`):

| Match Arm | Surface |
|-----------|---------|
| `Commands::Ci(_)` | `Ci` |
| `Commands::Agent(_)` | `SecurityAgent` |
| `Commands::McpServe(_)` \| `Commands::CodeggMcp(_)` | `McpServer` |
| `Commands::Serve(_)` | `RestApi` |
| `_ if cli.strict_scope` | `CliManualStrict` |
| `_` | `CliManual` |

**Without `rest-api` feature** (`main.rs:52–61`):

| Condition | Surface |
|-----------|---------|
| `Commands::Ci(_)` | `Ci` |
| `cli.strict_scope` | `CliManualStrict` |
| default | `CliManual` |

Key detail: `Commands::Grpc(_)` does **not** appear in `resolve_execution_surface()` — gRPC uses the default `CliManual` surface (the `grpc-api` feature does not gate surface resolution).

---

## Command Inventory

The `Commands` enum (`cli/mod.rs:270–461`) defines exactly **52 clap subcommand variants**: 27 unconditional + 25 feature-gated. Each variant has a `command_id()` method (`cli/mod.rs:464–551`) returning a stable kebab-case string used for registry lookup and diagnostics.

### Unconditional Commands (27)

| # | Variant | `command_id` | Line | Purpose | Handler (`handlers/mod.rs`) |
|---|---------|-------------|------|---------|----------------------------|
| 1 | `ScanPorts` | `scan-ports` | `:273` | TCP port scan | `handle_scan_ports` `:494` |
| 2 | `ScanEndpoints` | `scan-endpoints` | `:275` | HTTP endpoint discovery | `handle_scan_endpoints` `:495` |
| 3 | `Fingerprint` | `fingerprint` | `:277` | Service fingerprinting (AMAP-style) | `handle_fingerprint` `:496` |
| 4 | `Scan` | `scan` | `:279` | Chained security assessment pipeline | `handle_scan` `:504` |
| 5 | `Resume` | `resume` | `:281` | Resume previous scan from session file | `handle_resume` `:505` |
| 6 | `Fuzz` | `fuzz` | `:289` | Security fuzzing | `handle_fuzz` `:501` |
| 7 | `Waf` | `waf` | `:291` | WAF detection and evasion resistance | `handle_waf` `:503` |
| 8 | `WafStress` | `waf-stress` | `:293` | WAF stress testing | `handle_waf_stress` `:502` |
| 9 | `Graphql` | `graphql` | `:295` | GraphQL endpoint security | `handle_graphql` `:514` |
| 10 | `OAuth` | `oauth` | `:297` | OAuth/OIDC endpoint security | `handle_oauth` `:515` |
| 11 | `AuthTest` | `auth-test` | `:300` | Authentication testing (credential validation) | `handle_auth_test` `:516` |
| 12 | `Recon` | `recon` | `:303` | Reconnaissance information gathering | `handle_recon` `:506` |
| 13 | `Plan` | `plan` | `:307` | Preview execution plan (no execution) | `handle_plan` `:507` |
| 14 | `Preflight` | `preflight` | `:309` | Preview enforcement decision (no execution) | `handle_preflight` `:508` |
| 15 | `Ci` | `ci` | `:311` | CI/CD security checks mode | `handle_ci` `:509` |
| 16 | `Config` | `config` | `:314` | Validate configuration files | `handle_config` `:510` |
| 17 | `Doctor` | `doctor` | `:316` | System dependency diagnostics | `handle_doctor` `:511` |
| 18 | `PolicyExplain` | `policy-explain` | `:321` | Explain policy decisions for target + profile | `handle_policy_explain` `:512` |
| 19 | `ScopeExplain` | `scope-explain` | `:326` | Explain scope matching for a target | `handle_scope_explain` `:513` |
| 20 | `Load` | `load` | `:333` | HTTP load testing | `handle_load` `:493` |
| 21 | `Report` | `report` | `:343` | Convert and generate security reports | `handle_report` `:525` |
| 22 | `Vuln` | `vuln` | `:344` | Vulnerability management (CVSS, triage) | `handle_vuln` `:571` |
| 23 | `Storage` | `storage` | `:347` | Database storage and query operations | `handle_storage` `:572` |
| 24 | `Cluster` | `cluster` | `:375` | Distributed scanning cluster management | `handle_cluster` `:532` |
| 25 | `Notify` | `notify` | `:377` | Notification management and testing | `handle_notify` `:533` |
| 26 | `Remote` | `remote` | `:378` | Start remote listener for distributed commands | `handle_remote` `:534` |
| 27 | `Exec` | `exec` | `:381` | Execute commands on remote systems | `handle_exec` `:535` |

### Feature-Gated Commands (25)

| # | Variant | `command_id` | Feature | Line | Purpose | Handler (`handlers/mod.rs`) |
|---|---------|-------------|---------|------|---------|----------------------------|
| 28 | `Hunt` | `hunt` | `advanced-hunting` | `:286` | Advanced vulnerability hunting | `handle_hunt` `:500` |
| 29 | `Sbom` | `sbom` | `sbom` | `:329` | SBOM generation + supply chain check | `handle_sbom` `:518` |
| 30 | `Packet` | `packet` | `packet-inspection` | `:338` | Packet inspection and analysis | `handle_packet` `:520` |
| 31 | `Nse` | `nse` | `nse` | `:341` | Nmap NSE-compatible script execution | `handle_nse` `:498` |
| 32 | `ProxyIntercept` | `proxy-intercept` | `web-proxy` | `:352` | Interactive MITM web proxy | `web_proxy::handle_proxy_intercept` `:529` |
| 33 | `Stress` | `stress` | `stress-testing` | `:362` | Stress/load testing | `handle_stress` `:527` |
| 34 | `Proxy` | `proxy` | `stress-testing` | `:364` | Proxy pool and rotation management | `handle_proxy` `:531` |
| 35 | `Icmp` | `icmp` | `stress-testing` | `:367` | ICMP echo probes | `handle_icmp` `:522` |
| 36 | `Traceroute` | `traceroute` | `stress-testing` | `:370` | Network path tracing | `handle_traceroute` `:524` |
| 37 | `Serve` | `serve` | `rest-api` | `:384` | REST API server | `handle_serve` `:537` |
| 38 | `McpServe` | `mcp-serve` | `rest-api` | `:389` | MCP server for AI integration | `handle_mcp_serve` `:539` |
| 39 | `CodeggMcp` | `mcp-serve` | `rest-api` | `:394` | MCP server for coding agent (stdio) | `handle_mcp_serve` (via `McpServeArgs` conversion) `:541–549` |
| 40 | `Agent` | `agent` | `rest-api` | `:402` | Scheduled security agent | `handle_agent` `:552` |
| 41 | `AiAnalyze` | `ai-analyze` | `ai-integration` | `:410` | Post-scan AI analysis | `handle_ai_analyze` `:554` |
| 42 | `Wireless` | `wireless` | `wireless` | `:414` | WiFi security scanning | `handle_wireless` `:556` |
| 43 | `Browser` | `browser` | `headless-browser` | `:419` | Headless browser security testing | `handle_browser` `:568` |
| 44 | `Mobile` | `mobile` | `mobile` | `:424` | APK/IPA static security analysis | `handle_mobile` `:564` |
| 45 | `Evasion` | `evasion` | `evasion` | `:429` | Evasion technique detection | `handle_evasion` `:558` |
| 46 | `Postex` | `postex` | `postex` | `:434` | Post-exploitation simulation | `handle_postex` `:560` |
| 47 | `C2` | `c2` | `c2` | `:439` | C2 framework simulation | `handle_c2` `:562` |
| 48 | `Db` | `db` | `db-pentest` | `:444` | Database pentesting (subcommand enum `DbCommand`) | `handle_db_pentest` `:566` |
| 49 | `Grpc` | `grpc` | `grpc-api` | `:450` | gRPC API server | `handle_grpc_server` `:570` |
| 50 | `Daemon` | `daemon` | `daemon-client` | `:455` | Daemon process management | daemon_cli intercept `:119–121` |
| 51 | `Session` | `session` | `daemon-client` | `:458` | Daemon session management | daemon_cli intercept `:119–121` |
| 52 | `Task` | `task` | `daemon-client` | `:461` | Daemon task management | daemon_cli intercept `:119–121` |

**Alias note**: Both `McpServe` and `CodeggMcp` map to `command_id = "mcp-serve"` (`cli/mod.rs:503–505`). The `CodeggMcp` variant also has clap alias `mcp-codegg` (`:395`). In the handler match, `CodeggMcp` converts its `CodeggMcpArgs` to `McpServeArgs` before calling `handle_mcp_serve()` (`handlers/mod.rs:541–549`).

**Daemon intercept note**: Variants 50–52 (`Daemon`, `Session`, `Task`) are intercepted in `main.rs:117–122` by `daemon_cli::is_daemon_command()` before reaching `handle_command()`. The `handle_command()` match arms for these variants (`handlers/mod.rs:576–580`) return `anyhow::bail!()` and are unreachable in practice.

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

### Exhaustive Match (`handlers/mod.rs:451–582`)

`handle_command()` (`:451`) is an async function that:

1. **Registry validation** (`:463–487`): For commands with an `operation_id` in the registry, validates the metadata resolves to `OperationMetadata`. Logs a warning if stale (fallback to handler).
2. **Exhaustive match** (`:489–581`): No wildcard arm — adding/removing `Commands` variants requires updating this match at compile time.

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
| `scan.rs` | `scan-ports`, `scan-endpoints`, `fingerprint`, `scan`, `resume` | — |
| `recon.rs` | `recon` | — |
| `fuzz.rs` | `fuzz`, `waf-stress` | — |
| `load.rs` | `load` | — |
| `network.rs` | `waf` | — |
| `report.rs` | `report` | — |
| `vuln.rs` | `vuln` | — |
| `storage.rs` | `storage` | `database` |
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

Registry-backed commands use `describe_from_registry()` to build descriptors from canonical `OperationMetadata`:

```rust
// Registry-backed (preferred)
pub async fn handle_recon(ctx: &CommandContext, args: ReconArgs) -> Result<()> {
    let descriptor = ctx
        .describe_from_registry("recon", Some(target))
        .ok_or_else(|| anyhow::anyhow!("No registry metadata for command"))?;
    let decision = ctx.evaluate_and_enforce_operation(descriptor)?;
    // proceed with dispatch
    Ok(())
}

// Legacy (manual descriptor construction)
pub async fn handle_fuzz(ctx: &CommandContext, args: FuzzArgs) -> Result<()> {
    let descriptor = OperationDescriptor {
        operation: "fuzz".to_string(),
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::Intrusive,
        // ...
    };
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

## Command Registry (`commands/registry.rs`)

The command registry provides static, inspectable metadata for CLI/TUI dispatch. It maps command IDs to metadata and descriptor builders, enabling incremental migration from the legacy `handle_command()` match dispatch.

**The registry is metadata and routing, not authorization.** All side-effecting operations still flow through `EnforcementContext::evaluate()` before execution.

### Registry Entry Count

The `REGISTERED_COMMANDS` array (`registry.rs:108–726`) contains **46 entries** (not 52 — daemon commands, some legacy commands, and some catalog entries are not all registered). Categories:

| Dispatch Mode | Count | Commands |
|--------------|-------|----------|
| `RegistryBacked` | 4 | `recon`, `scan-ports`, `scan-endpoints`, `fingerprint` |
| `LegacyWrapped` | 27 | `scan`, `resume`, `fuzz`, `waf`, `waf-stress`, `graphql`, `oauth`, `auth-test`, `load`, `stress`, `packet`, `icmp`, `traceroute`, `nse`, `hunt`, `evasion`, `postex`, `c2`, `proxy-intercept`, `wireless`, `browser`, `mobile`, `db`, `proxy` (not registered — omitted) |
| `HelperOnly` | 11 | `plan`, `preflight`, `ci`, `config`, `doctor`, `policy-explain`, `scope-explain`, `ai-analyze`, `report`, `vuln`, `storage`, `sbom`, `notify` |
| `ServerLifecycle` | 7 | `serve`, `mcp-serve`, `agent`, `grpc`, `cluster`, `remote-serve`, `exec` |
| `CatalogOnly` | 0 | (none currently) |

### Registry API

| Function | Location | Purpose |
|----------|----------|---------|
| `lookup_command(command_id)` | `:729` | Find `CommandRegistration` by ID |
| `build_descriptor_for_command(command_id, target)` | `:738` | Build `OperationDescriptor` from registry metadata |
| `all_command_ids()` | `:746` | All registered command IDs |
| `tui_visible_command_ids()` | `:751` | TUI-visible commands |
| `cli_interactive_only_command_ids()` | `:761` | CLI-helper-only commands |
| `registry_backed_command_ids()` | `:770` | Registry-backed dispatch commands |
| `suggest_command(unknown)` | `:779` | Levenshtein-based suggestions (edit distance ≤ 3) |

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
    pub registry_backed: bool,
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
| `RegistryBacked` | Descriptor/execution uses registry metadata (Phase 6 pilot) |
| `LegacyWrapped` | Wraps legacy `handle_command()` dispatch |
| `CatalogOnly` | Listed for discoverability, never dispatched |
| `ServerLifecycle` | Server lifecycle command |
| `HelperOnly` | Read-only helper/diagnostic |

---

## Policy Preview Commands

Three commands evaluate policy **without sending network traffic** — they are read-only diagnostic surfaces:

### `plan` (`:507`)

Previews the execution plan for a target + profile combination. Shows what stages would run, in what order, with what configuration. No enforcement evaluation.

### `preflight` (`:508`)

Previews the enforcement decision for a specific operation. Builds an `OperationDescriptor` and calls `evaluate_and_enforce_operation()`, showing the outcome (Allow/Warn/Deny/RequireConfirmation) without executing. Useful for CI debugging and policy validation.

### `policy-explain` / `scope-explain` (`:512–513`)

Explain commands provide human-readable explanations:
- `policy-explain`: Evaluates what would happen for a target + profile (operation mode, risk level, scope matching, required features, policy blocks)
- `scope-explain`: Evaluates whether a target falls within configured scope (rule matches, exclusions, private-IP detection)

Both are `PassiveAnalytical` / `HelperOnly` in the registry and `cli_interactive_only`.

---

## Logging Setup (`crates/eggsec-cli/src/logging.rs`)

`init_logging()` (`:18`) configures the `tracing` subscriber:

### Format Selection

| Condition | Format | Behavior |
|-----------|--------|----------|
| `--json` flag | `LogFormat::Json` | JSON output with span events, thread IDs, thread names |
| Default (no flag) | `LogFormat::Pretty` | Pretty-printed output with targets and line numbers |
| `Compact` (dead code) | `LogFormat::Compact` | Compact output — **defined but never selected by CLI flags** |

### Filter

- Uses `EnvFilter::try_from_default_env()` — respects `RUST_LOG` environment variable
- Default filter: `"info"` level

### Agent Log Appender

When the `Agent` command is used (`main.rs:78`), `agent_log_dir()` (`:20`) returns a log directory path (`<memory_dir>/logs`). This enables:
- Daily rolling file appender (`tracing_appender::rolling::Rotation::DAILY`)
- Non-blocking writer (`tracing_appender::non_blocking`)
- JSON format for file output (`.json` extension, no ANSI, thread IDs, file + line)
- File layer always runs alongside the console layer

The `WorkerGuard` returned by `init_logging()` must be held for the lifetime of the process to keep the non-blocking writer alive.

---

## Shell Completion & Config Generation

### `--generate-config` (`main.rs:68–71`)

Prints the default TOML configuration to stdout and exits immediately (before logging init). Implementation: `eggsec::config::get_default_config()`.

### `--generate-shell-completion` (`main.rs:73–76`)

Uses `clap_complete::generate()` to emit shell completion scripts. Supports all shells in `clap_complete::Shell` (Bash, Zsh, Fish, Elvish, PowerShell). Output goes to stdout.

---

## Integration Points

### Command Dispatch (`dispatch.md`)

CLI handlers call engine functions directly or via the dispatch layer. The dispatch layer (`crates/eggsec/src/dispatch/`) converts `TaskKind` requests into engine module calls. CLI handlers that use registry-backed dispatch build `OperationDescriptor` from registry metadata; legacy handlers construct descriptors manually.

### Configuration (`config.md`)

- `EggsecConfig` loaded from TOML/YAML via `load_config()`
- `Scope` / `LoadedScope` loaded via `load_scope_with_source()`
- `ExecutionPolicy` from config determines which risk tiers are permitted
- `OperationMetadata` (31 canonical + 42 aliases) is the single source of truth for operation policy

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

- `cli/mod.rs:666–726` — `ScanProfile` risk budget ordering, operation mode derivation
- `commands/handlers/mod.rs:600–1640` — `CommandContext` enforcement behavior (26 tests): safe/active/intrusive/stress/raw-packet/load-test/remote-execution allowed/denied with various policy flags, JSON mode denial structure, manual override flag semantics, scope evaluation
- `commands/registry.rs:818–1040` — Registry invariant tests (13 tests): unique IDs, metadata resolution, feature gates, pilot commands, dispatch mode consistency

---

## Invariants & Gotchas

1. **Handlers live in engine crate, not `eggsec-cli`**: `crates/eggsec/src/commands/handlers/` contains all 32 handler modules. `crates/eggsec-cli/` is the binary shell only (main, logging, daemon_cli).

2. **`cli/` holds types only**: `crates/eggsec/src/cli/` contains `Cli`, `Commands`, and arg structs. No handler logic. This separation allows the TUI and other frontends to reuse CLI types without depending on handler code.

3. **Exhaustive match**: `handle_command()` has no wildcard arm. Adding/removing `Commands` variants is a compile-time error until the match is updated.

4. **Daemon commands are intercepted early**: `Daemon`/`Session`/`Task` variants are handled in `main.rs:117–122` before `handle_command()`. Their match arms in `handlers/mod.rs:576–580` are dead code that bails with an error.

5. **`CodeggMcp` alias collision**: Both `McpServe` and `CodeggMcp` map to `command_id = "mcp-serve"` (`cli/mod.rs:503–505`). The registry has only one `mcp-serve` entry. This is intentional — they share the same MCP server handler.

6. **`--yes` is narrow**: Only permits `OutOfScope` and `TargetExpansion` confirmation classes. High-risk, exclusions, private resolution, cross-host redirect, and non-baseline capabilities require dedicated `--allow-*` flags (`handlers/mod.rs:373–396`).

7. **`evaluate_and_enforce_operation()` returns `PolicyDecision`, not `ApprovedOperation`**: The CLI manual path does not produce approval tokens. Tokens are only for strict surfaces via `EnforcementContext::approve()`.

8. **Grpc does not affect surface resolution**: `Commands::Grpc` is not matched in `resolve_execution_surface()` — it uses the default `CliManual` surface.

9. **`LogFormat::Compact` is dead code**: Defined in `logging.rs:15` but never selected by any CLI flag. Only `Pretty` (default) and `Json` (`--json`) are used.

10. **Double `#[cfg]` on `Db` variant**: `cli/mod.rs:522–524` has two stacked `#[cfg(feature = "db-pentest")]` attributes on the `Self::Db` match arm. This compiles but is redundant.

---

## Bugs Found (Report Only)

| # | File:Line | Description | Severity |
|---|-----------|-------------|----------|
| 1 | `logging.rs:15` | `LogFormat::Compact` variant is dead code — defined but never constructed by CLI flags. Only `Pretty` and `Json` are used. | Low |
| 2 | `cli/mod.rs:522–524` | Double `#[cfg(feature = "db-pentest")]` on `Self::Db` match arm in `command_id()`. Redundant attribute, compiles but unnecessary. | Cosmetic |
| 3 | `handlers/mod.rs:576–580` | `Daemon`/`Session`/`Task` match arms are dead code — intercepted in `main.rs` before `handle_command()` is reached. The `bail!()` is unreachable. | Informational |

---

*Last verified against source: 2026-08-25*
