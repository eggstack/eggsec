# CLI & Commands

The CLI and Commands layer is responsible for parsing user input, managing global state (`CommandContext`), and dispatching execution to the appropriate handlers.

## CLI Parsing (`src/cli/`)

Eggsec uses `clap` for command-line argument parsing. The CLI is organized into several modules, each defining the arguments for a specific category of commands:

- **`mod.rs`**: Defines the main `Cli` entry point, `Commands` enum (52 variants), `CommonHttpArgs`, `ScanProfile`, and daemon client CLI types.
- **`scan.rs`**: Arguments for the `scan` command (port scanning, endpoint discovery).
- **`fuzz.rs`**: Arguments for the `fuzz` command (security fuzzing).
- **`http.rs`**: Arguments for HTTP-specific operations (load, recon, graphql, oauth).
- **`packet.rs` & `stress.rs`**: Arguments for low-level networking and stress testing.
- **`agent.rs` & `ai_analyze.rs`**: Arguments for AI-driven features.
- **`auth.rs`**: Arguments for `auth-test` (credential testing).
- **`ci.rs`**: Arguments for CI/CD integration mode.
- **`cluster.rs`**: Arguments for distributed cluster management.
- **`db_pentest.rs`**: Arguments for database pentesting (feature-gated).
- **`evasion.rs`**: Arguments for evasion detection (feature-gated).
- **`explain.rs`**: Arguments for `policy-explain` and `scope-explain`.
- **`mobile.rs`**: Arguments for mobile APK/IPA analysis (feature-gated).
- **`plan.rs`**: Arguments for execution plan preview.
- **`preflight.rs`**: Arguments for preflight enforcement preview.
- **`postex.rs`**: Arguments for post-exploitation simulation (feature-gated).
- **`storage.rs`**: Arguments for database storage operations.
- **`vuln.rs`**: Arguments for vulnerability management.
- **`web_proxy.rs`**: Arguments for web proxy / MITM interception (feature-gated).
- **`wireless.rs`**: Arguments for wireless network scanning (feature-gated).

### Key CLI Patterns

- **Global flags**: `--json`, `--config`, `--scope`, `--strict-scope` apply to all commands
- **Manual override flags**: `--yes`, `--allow-out-of-scope`, `--allow-excluded-target`, `--allow-high-risk`, `--allow-db-pentest`, `--allow-web-proxy`, `--allow-nonbaseline-capability`, `--allow-private-resolution`, `--allow-cross-host-redirect`, `--manual-override-reason` (see Manual Override Flags below)
- **Feature-gated commands**: `stress-testing`, `packet-inspection`, `nse`, `ai-integration`, `rest-api`, `grpc-api`, `sbom`, `mobile`, `daemon-client` (`daemon`, `session`, `task`)
- **Output flag**: Use `-o` / `--output` for file output (consistent across commands)
- **Scope validation**: Handlers call `evaluate_and_enforce_operation()` with an `OperationDescriptor` to validate targets against scope and execution policy. For ManualPermissive, `RequireConfirmation` is satisfied only via narrow `--yes` (out-of-scope/target-expansion only) or dedicated `--allow-*` flags; precise required-flag errors are returned; strict profiles ignore overrides.

### Manual Override Flags

Override flags are **manual-only** — honored exclusively for `ManualPermissive` (default CLI/TUI). Strict profiles (`--strict-scope`), CI, MCP, and agent paths reject or ignore them.

| Flag | Scope | Description |
|------|-------|-------------|
| `--yes` | Out-of-scope, target-expansion only | Narrow confirmation for low-risk scope prompts. Does **not** authorize high-risk, explicit exclusions, non-baseline capabilities, private-resolution, or cross-host redirects. |
| `--allow-out-of-scope` | Out-of-scope, target-expansion | Allow operations on targets outside configured scope |
| `--allow-excluded-target` | Explicit exclusion | Allow operations on explicitly excluded targets |
| `--allow-high-risk` | High-risk tiers | Allow intrusive, stress, load, raw-packet, credential, exploit-adjacent, remote, db-pentest operations |
| `--allow-db-pentest` | Database pentesting | Required for non-dry-run db pentest operations |
| `--allow-web-proxy` | Traffic interception | Allow MITM proxy operations |
| `--allow-nonbaseline-capability` | Non-baseline capabilities | Allow non-baseline capabilities |
| `--allow-private-resolution` | Private resolution | Allow target resolution to private/loopback addresses when detected |
| `--allow-cross-host-redirect` | Cross-host redirect | Allow cross-host redirect/canonicalization boundary changes |
| `--manual-override-reason` | Audit | Reason for manual override (recorded for audit trail) |

### CommonHttpArgs

Shared HTTP client configuration used by recon, fuzz, load, and other HTTP-based commands:

| Field | Type | Description |
|-------|------|-------------|
| `--insecure` | `bool` | Skip TLS certificate verification |
| `--proxy` | `Option<String>` | HTTP proxy URL (e.g., `http://127.0.0.1:8080`) |
| `--proxy-auth` | `Option<String>` | Proxy authentication (`user:pass`) |
| `--auth` | `Option<String>` | Basic authentication (`user:pass`) |
| `--bearer` | `Option<String>` | Bearer token |
| `--cookie` | `Option<String>` | Cookie header value |
| `--api-key` | `Option<String>` | API key header (format: `name:value` or just value for `X-API-Key`) |
| `--user-agent` | `Option<String>` | Custom User-Agent header |
| `--stealth` | `bool` | Simulate realistic user behavior with randomized timing/headers |
| `--rate-limit` | `Option<u32>` | Rate limit (requests per second) |
| `--jitter` | `Option<String>` | Random delay between requests (ms range, e.g., `100-500`) |
| `--auth-context` | `Option<String>` | Path to auth context YAML file (multi-user/multi-role testing) |
| `--auth-role` | `Option<String>` | Auth role name from the auth context file (required when `--auth-context` is set) |

## Command Dispatch (`src/commands/`)

Once arguments are parsed, the `main` function initializes a `CommandContext` and calls `handle_command` via `src/commands/mod.rs` re-exports. The implementation lives in `src/commands/handlers/mod.rs`.

- **`CommandContext`**: Carries global state including the loaded `EggsecConfig`, `Scope`, output preferences, `EnforcementContext`, `NotifyManager`, and `execution_profile`. See the struct section below.
- **`handle_command`**: A large exhaustive match statement that dispatches to the correct handler based on the subcommand.
  Because it is exhaustive (no wildcard arm), adding/removing `Commands` variants requires updating dispatch at compile time.

### CommandContext

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

Key fields:

- **`execution_surface`**: Origin of the execution request (`CliManual`, `McpServer`, `SecurityAgent`, `Ci`, `RestApi`, etc.). Derives `execution_profile` via `surface.profile()`.
- **`execution_profile`**: Derived from `execution_surface.profile()` — **not** flag-based. Default for CLI is `ManualPermissive`. Set to `McpStrict` for MCP, `AgentStrict` for agent, `CiStrict` for CI.
- **`enforcement`**: `EnforcementContext` built from the surface, policy, and loaded scope. Handles all authorization decisions.
- **`manual_override`**: `ManualOverride` struct carrying `--allow-*` flags. Only effective for `ManualPermissive`.
- **`config_path`**: Optional path to the config file used for this session.
- **`notify_manager`**: `NotifyManager` initialized from config settings.

Builder methods: `with_config_path()`, `with_execution_surface()`, `with_loaded_scope()`, `with_manual_override()`.

### `evaluate_and_enforce_operation()` Method

`CommandContext::evaluate_and_enforce_operation()` wraps the shared policy evaluator with profile-aware enforcement. It calls `self.enforcement.evaluate(&descriptor)` and processes the resulting `EnforcementOutcome`:

1. **`Allow(decision)`**: Emits audit event, returns `Ok(PolicyDecision)`
2. **`Warn(decision)`**: Emits audit event, logs warnings via `tracing::warn!`, returns `Ok(PolicyDecision)`
3. **`RequireConfirmation(decision)`**:
   - Under non-ManualPermissive profiles: treated as hard denial — returns error (JSON or human-readable)
   - Under ManualPermissive: computes `ConfirmationClass` set from the decision, checks if `manual_override` permits all classes. If permitted, records override in the decision and returns `Ok(PolicyDecision)`. If not, returns error listing the exact `--allow-*` flags needed.
4. **`Deny(decision)`**: Emits audit event, returns error (JSON serialized or human-readable)

The method returns `Result<PolicyDecision>` — **not** `ApprovedOperation`. There is no `approve_manual()` call; the flow is `self.enforcement.evaluate()` → outcome match.

## Command Registry (`src/commands/registry.rs`)

The command registry provides static, inspectable metadata for CLI/TUI dispatch. It maps command IDs to metadata and descriptor builders, enabling incremental migration from the legacy `handle_command()` match dispatch.

**The registry is metadata and routing, not authorization.** All side-effecting operations still flow through `EnforcementContext::evaluate()` before execution.

### Types

#### `CommandRegistration`

Static metadata for a registered command:

```rust
pub struct CommandRegistration {
    pub command_id: &'static str,        // Stable CLI subcommand name
    pub operation_id: Option<&'static str>, // In ALL_OPERATION_METADATA, if applicable
    pub display_name: &'static str,      // Human-readable name
    pub category: CommandCategory,       // Classification
    pub feature: Option<&'static str>,   // Feature gate, if any
    pub cli_visible: bool,               // Appears in CLI help
    pub tui_visible: bool,               // Appears in TUI tab listings
    pub programmatic_visible: bool,      // Exposed via MCP/REST/gRPC/agent
    pub cli_interactive_only: bool,      // CLI-only helper (not TUI or programmatic)
    pub registry_backed: bool,           // Uses registry metadata for dispatch
    pub dispatch_mode: CommandDispatchMode,
}
```

#### `CommandCategory`

| Variant | Description |
|---------|-------------|
| `SideEffectingNetwork` | Network operations requiring enforcement (scans, fuzz, stress) |
| `LocalFileDomain` | Local file or domain-specific operations (DB, mobile, reports) |
| `PassiveAnalytical` | Read-only analysis (explain, AI analyze) |
| `ConfigOutputHelper` | Configuration, help, diagnostics (config, doctor, plan) |
| `FrontendServer` | Server daemons (REST, MCP, gRPC, agent) |
| `LegacySpecial` | Commands with no metadata or unique dispatch needs |

#### `CommandDispatchMode`

| Variant | Description |
|---------|-------------|
| `RegistryBacked` | Descriptor/execution path uses registry metadata (Phase 6 pilot commands) |
| `LegacyWrapped` | Wraps legacy `handle_command()` dispatch (pre-migration commands) |
| `CatalogOnly` | Listed for discoverability but never dispatched |
| `ServerLifecycle` | Server lifecycle command (serve, mcp-serve, agent, grpc, etc.) |
| `HelperOnly` | Read-only helper/diagnostic (config, doctor, plan, preflight, etc.) |

### Registry API

- `lookup_command(command_id)` → `Option<&CommandRegistration>` — Look up by command ID
- `build_descriptor_for_command(command_id, target)` → `Option<OperationDescriptor>` — Build descriptor from registry metadata
- `all_command_ids()` → `Vec<&str>` — All registered command IDs
- `tui_visible_command_ids()` → `Vec<&str>` — TUI-visible commands
- `cli_interactive_only_command_ids()` → `Vec<&str>` — CLI-helper-only commands
- `registry_backed_command_ids()` → `Vec<&str>` — Commands using registry-backed dispatch
- `suggest_command(unknown)` → `Vec<&str>` — Levenshtein-based suggestions for unknown commands

### Registry-Backed Dispatch Flow

In `handle_command()`, before dispatching to the handler, the dispatch bridge validates registry metadata:

1. Calls `command.command_id()` to get the stable ID
2. Looks up the `CommandRegistration` via `lookup_command()`
3. If the entry has an `operation_id`, validates it resolves to `OperationMetadata`
4. Logs a warning if the entry is stale (metadata not found)
5. Enforcement is **not** performed here — it remains in each handler via `evaluate_and_enforce_operation()`

### Registry Entry Count

The `REGISTERED_COMMANDS` array contains entries for all 50+ commands across categories:
- **Phase 6 pilot (registry-backed)**: `recon`, `scan-ports`, `scan-endpoints`, `fingerprint`
- **Legacy-wrapped**: `scan`, `resume`, `fuzz`, `waf`, `waf-stress`, `graphql`, `oauth`, `auth-test`, `load`, `stress`, `packet`, `icmp`, `traceroute`, `nse`, `hunt`, `evasion`, `postex`, `c2`, `proxy-intercept`, `wireless`, `browser`, `mobile`, `db`
- **Config/helper**: `plan`, `preflight`, `ci`, `config`, `doctor`
- **Passive analytical**: `policy-explain`, `scope-explain`, `ai-analyze`
- **Server lifecycle**: `serve`, `mcp-serve`, `agent`, `grpc`, `cluster`, `remote`, `exec`
- **Report/vuln/storage**: `report`, `vuln`, `storage`, `sbom`, `notify`

## Handlers (`src/commands/handlers/`)

Actual command execution logic resides in the `handlers` directory. Each handler is typically an `async` function that takes the parsed arguments and the `CommandContext`.

### Handler Module Files

| File | Commands | Feature Gate |
|------|----------|--------------|
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

Registry-backed commands should use `describe_from_registry()` to build descriptors:

```rust
// Registry-backed command (preferred pattern)
pub async fn handle_recon(ctx: &CommandContext, args: ReconArgs) -> Result<()> {
    let target = args.target.clone();
    let descriptor = ctx
        .describe_from_registry("recon", Some(target))
        .ok_or_else(|| anyhow::anyhow!("No registry metadata for command"))?;
    let decision = ctx.evaluate_and_enforce_operation(descriptor)?;
    // decision is a PolicyDecision — proceed with dispatch
    // ...
    Ok(())
}

// Legacy command (manual descriptor construction)
pub async fn handle_fuzz(ctx: &CommandContext, args: FuzzArgs) -> Result<()> {
    let target = crate::utils::extract_target_from_url(&args.url)
        .unwrap_or_else(|| args.url.clone());
    let descriptor = OperationDescriptor {
        operation: "fuzz".to_string(),
        mode: crate::config::OperationMode::StandardAssessment,
        risk: crate::config::OperationRisk::Intrusive,
        intended_uses: vec![crate::config::IntendedUse::WebAssessment],
        target: Some(target),
        required_features: Vec::new(),
        required_policy_flags: Vec::new(),
        requires_private_or_local_target: false,
        requires_explicit_scope: false,
        required_capabilities: Vec::new(),
    };
    let decision = ctx.evaluate_and_enforce_operation(descriptor)?;
    // decision is a PolicyDecision — proceed with dispatch
    // ...
    Ok(())
}

// Config/helper command (no enforcement needed)
pub async fn handle_config(_ctx: &CommandContext, args: ConfigArgs) -> Result<()> {
    load_config(config_path).map_err(|e| anyhow::anyhow!("Configuration validation failed: {}", e))?;
    Ok(())
}
```

## Workflow

1. `main.rs` parses arguments using `Cli::parse()`.
2. Logging is initialized.
3. Configuration and Scope are loaded.
4. `CommandContext` is created (with `execution_surface`, `enforcement`, `manual_override`).
5. `handle_command` (implemented in `src/commands/handlers/mod.rs`) dispatches to a specific handler in `src/commands/handlers/`.
6. The handler calls `evaluate_and_enforce_operation()` with an `OperationDescriptor` (either from registry or manual construction).
7. The handler executes the requested operation, often interacting with other core modules like `scanner` or `fuzzer`.

## Bug Fixes and Consistency (2026-05-22)

### Fixed Issues

1. **`sbom.rs`**: Replaced `unwrap()` with `ok_or_else()` pattern for path conversion (handles invalid Unicode)
2. **`config.rs`**: Replaced `std::process::exit(1)` with proper error returns via `map_err()`
3. **`http.rs`**: Added `-o` short form to `load` and `graphql` output flags for consistency
4. **`handlers/mod.rs:197-206`**: `handle_no_command` launches TUI in interactive terminal, otherwise prints guidance
5. **`handlers/cluster.rs:348`**: Replaced `unwrap_or(22)` with `unwrap_or_else(|_| 22)` to avoid panic on invalid parsing
6. **`handlers/auth_test.rs:10`**: Migrated from `ensure_scope_url` to `evaluate_and_enforce_operation` with `CredentialTesting` risk tier (central `EnforcementContext`). Adopted model: standalone CLI command; local `AuthTestReport`/`AuthFinding` only (direct JSON/text output; no `ScanReportData`/eggsec-output conversion or pipeline profile integration). See architecture/auth.md.
7. **`cli/scan.rs`**: Added `-o` short flag to `PortScanArgs`, `EndpointScanArgs`, `FingerprintArgs`, `NseArgs`, `ResumeArgs`
8. **`cli/fuzz.rs`**: Added `-o` short flag to `WafStressArgs`; preserved `From<WafStressArgs>` implementation
9. **`cli/http.rs`**: Added `-o` short flag to `ReconArgs`
10. **`cli/cluster.rs`**: Removed unused `-o` flag from `ClusterArgs` - cluster commands are interactive and don't produce file output

### CLI Consistency Guidelines

| Issue | Recommendation |
|-------|----------------|
| `--host` vs `--target` vs `--url` | Use `--target` for hosts, `--url` for endpoints |
| Timeout defaults | Use 15s as standard default |
| WAF profile | Use `String` (not `ValueEnum`) for flexibility |
| Source IP naming | `source_ip` / `source_port` (not `spoof_ip`) |

## Skills Reference

- `.opencode/skills/eggsec-cli/` - Full CLI patterns and handler guide

## Daemon Client Commands (feature = `daemon-client`)

When built with `--features daemon-client`, the CLI gains three command groups that communicate with a running `eggsec-daemon` instance over a Unix socket:

- **`eggsec daemon start|status|stop`**: Manage the daemon process (start forks a background daemon, status checks health, stop sends shutdown).
- **`eggsec daemon history [--json]`**: List persisted sessions from the SQLite store (session ID, surface, active/completed task counts).
- **`eggsec daemon show <session-id> [--json]`**: Show full persisted snapshot details (surface, scope, generation, task list with statuses).
- **`eggsec session list|create|snapshot`**: Manage sessions on the daemon (list sessions, create new session, get session snapshot with task statuses).
- **`eggsec task submit|cancel|watch`**: Manage tasks on the daemon (submit a tool command, cancel a task, watch task events via streaming).

All daemon commands use JSON line protocol over Unix sockets. The `--socket` flag specifies the socket path (default: `/tmp/eggsec-daemon.sock`). Task submission builds a `RunRequest` from CLI args and sends it to the daemon for execution. All daemon CLI sessions use `RuntimeSurface::CliManual` (not `TuiManual`).

**Dispatch pattern**: Daemon commands are intercepted in `main.rs` before reaching the `handle_command` exhaustive match. The `daemon_cli::is_daemon_command()` check routes daemon/session/task variants to `daemon_cli::handle_daemon_command()`. The `eggsec` crate handler match arms for these variants return `anyhow::bail!` (unreachable in practice).

## Special Cases (Standalone Commands)

Some target-bearing commands are intentionally standalone and bypass (or optionally participate in) the canonical `ScanReportData` / `eggsec-output` conversion pipeline. They emit module-local report types directly as JSON/text or to `-o` files.

- `auth-test` (handler: `commands/handlers/auth_test.rs`): Uses local `AuthTestReport`/`AuthFinding` (defined in `auth/mod.rs`). Policy gate via `evaluate_and_enforce_operation` with `OperationRisk::CredentialTesting`. **No** conversion to `FindingData`/`ScanReportData`, no SARIF/JUnit/etc. via the output crate, and no integration with `eggsec scan --profile` pipelines. Distinct from `ScanProfile::Auth` (JWT/OAuth/IDOR fuzzer stages). See `architecture/auth.md`, `docs/AUTH_LAB.md`, and `commands/handlers/auth_test.rs:274-285` (direct emit logic).
- `mobile` (handler: `commands/handlers/mobile.rs`): Standalone defense-lab CLI (`eggsec mobile <apk-or-ipa>`). Uses local `MobileScanReport`/`MobileFinding` (defined in `mobile/mod.rs`). Policy via `evaluate_and_enforce_operation` with `OperationRisk::SafeActive` + `required_features: ["mobile"]`. Optional `to_scan_report_data` bridge for JSON/SARIF/JUnit consumers (mirrors wireless pattern); no pipeline profile integration in Phase 1 (no `mobile-static`/`mobile-regression` profiles yet). Pure-Rust static analysis only (APK/IPA manifest/config). Native `--json` is auto-bridged by the report handler. See `architecture/mobile.md`, `crates/eggsec/src/mobile/mod.rs`, `crates/eggsec/src/commands/handlers/mobile.rs`, and `crates/eggsec/src/cli/mobile.rs`. Dynamic loadout per design plan (completed; still standalone, no MCP exposure; mirrors wireless bullet).
- `wireless` (handler: `commands/handlers/wireless.rs`): Standalone-complete passive WiFi recon (CLI + TUI tab under `wireless` feature). Uses local `WirelessScanResult`/`WirelessNetwork` (defined in `wireless/mod.rs`). Policy via `SafeActive` + `wireless` feature (central `evaluate_and_enforce_operation`). Optional `to_scan_report_data` bridge (populates findings + full `wireless_networks`); native `--json` (or `--repeat` wrapped form) is auto-bridged by the report handler for `report convert`. Not integrated with `ScanProfile` pipelines or dedicated profiles. **MCP / agentic tool exposure**: intentionally none (not registered as SecurityTool; invisible to tools/list and CodingAgent/OpsAgent profiles; see architecture/wireless.md MCP/Agentic section). Part of the consolidated "standalone defense-lab surfaces" (wireless + mobile + auth-test) pattern. See `architecture/wireless.md` (MCP/Agentic section + Integration), `architecture/defense_lab.md`, `docs/USAGE.md` (Output Models), and AGENTS.md (standalone note). Active extensions (Phase 1+) completed (still standalone, no MCP exposure).
- `db-pentest` (handler: `commands/handlers/db_pentest.rs`): Standalone defense-lab database security assessment (Phase 1-5). Uses local `DbPentestReport`/`DbFinding` (defined in `db_pentest/types.rs`). Policy gate via `evaluate_and_enforce_operation` with `OperationRisk::DbPentest` (real) / `SafeActive` (dry-run) + `--allow-db-pentest` for non-dry runs. Optional `to_scan_report_data_db` bridge for JSON/SARIF/JUnit consumers; native `--json` auto-bridged by report handler. TUI tab `Tab::DbPentest` + native `Stage::DbPentest` pipeline stage (via `ScanProfile::DbRegression`). Phase 5 adds MongoDB/Redis engines, cross-DB correlation, compliance mapping, optional MCP exposure via `db-pentest-mcp` marker. See `architecture/database_pentest.md`, `architecture/defense_lab.md`, `architecture/pipeline.md`.

Distinction: `auth-test` bypasses entirely (local `Auth*` only, direct handler emit, no bridge or conversion path). `wireless` and `mobile` emit local types directly for their surfaces but expose an optional `to_scan_report_data` bridge (used by `eggsec-output` converters and auto-bridged in `report convert`) for unified formats. None of the three have pipeline stage integration. Design decision recorded in the per-module architecture docs (completed).
