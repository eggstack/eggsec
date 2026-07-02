# CLI & Commands

The CLI and Commands layer is responsible for parsing user input, managing global state (CommandContext), and dispatching execution to the appropriate handlers.

## CLI Parsing (`src/cli/`)

Eggsec uses `clap` for command-line argument parsing. The CLI is organized into several modules, each defining the arguments for a specific category of commands:

- **`mod.rs`**: Defines the main `Cli` entry point, `Commands` enum (45 variants), and `CommonHttpArgs`.
- **`scan.rs`**: Arguments for the `scan` command (port scanning, endpoint discovery).
- **`fuzz.rs`**: Arguments for the `fuzz` command (security fuzzing).
- **`http.rs`**: Arguments for HTTP-specific operations (load, recon, graphql, oauth).
- **`packet.rs` & `stress.rs`**: Arguments for low-level networking and stress testing.
- **`agent.rs` & `ai_analyze.rs`**: Arguments for AI-driven features.

### Key CLI Patterns

- **Global flags**: `--json`, `--config`, `--scope`, `--strict-scope` apply to all commands
- **Feature-gated commands**: `stress-testing`, `packet-inspection`, `nse`, `ai-integration`, `rest-api`, `grpc-api`, `sbom`, `mobile`, `daemon-client` (`daemon`, `session`, `task`)
- **Output flag**: Use `-o` / `--output` for file output (consistent across commands)
- **Scope validation**: Handlers call `evaluate_and_enforce_operation()` with an `OperationDescriptor` to validate targets against scope and execution policy. For ManualPermissive, `RequireConfirmation` is satisfied only via narrow `--yes` (out-of-scope/target-expansion only) or dedicated `--allow-private-resolution` / `--allow-cross-host-redirect` etc.; precise required-flag errors are returned; strict profiles ignore overrides.

## Command Dispatch (`src/commands/`)

Once arguments are parsed, the `main` function initializes a `CommandContext` and calls `handle_command` via `src/commands/mod.rs` re-exports. The implementation lives in `src/commands/handlers/mod.rs`.

- **`CommandContext`**: Carries global state including the loaded `EggsecConfig`, `Scope`, output preferences, `EnforcementContext`, and `execution_profile` (defaults to `ManualPermissive`; set to `ManualGuarded` by `--strict-scope`, `CiStrict` in CI mode). `evaluate_and_enforce_operation()` produces an `ApprovedOperation` token via `approve_manual()` (Phase 12 type-level dispatch).
- **`handle_command`**: A large exhaustive match statement that dispatches to the correct handler based on the subcommand.
  Because it is exhaustive (no wildcard arm), adding/removing `Commands` variants requires updating dispatch at compile time.

## Handlers (`src/commands/handlers/`)

Actual command execution logic resides in the `handlers` directory. Each handler is typically an `async` function that takes the parsed arguments and the `CommandContext`.

Examples:
- **`scan.rs`**: Entry point for port scanning and reconnaissance.
- **`fuzz.rs`**: Entry point for the security fuzzing engine.
- **`cluster.rs`**: Manages distributed scanning nodes.

### Handler Patterns

```rust
// Policy enforcement (required for all target-based commands)
pub async fn handle_fuzz(ctx: &CommandContext, args: FuzzArgs) -> Result<()> {
    let target = crate::utils::extract_target_from_url(&args.url)
        .unwrap_or_else(|| args.url.clone());
    let approved = ctx.evaluate_and_enforce_operation(OperationDescriptor {
        operation: "fuzz".to_string(),
        mode: crate::config::OperationMode::StandardAssessment,
        risk: crate::config::OperationRisk::Intrusive,
        intended_uses: vec![crate::config::IntendedUse::WebAssessment],
        target: Some(target),
        required_features: Vec::new(),
        required_policy_flags: Vec::new(),
        requires_private_or_local_target: false,
        requires_explicit_scope: false,
    })?;
    // approved is an ApprovedOperation token — carries decision, surface, profile
    // ... proceed with dispatch
}

// Error handling - return Result, never std::process::exit()
pub async fn handle_config(_ctx: &CommandContext, args: ConfigArgs) -> Result<()> {
    load_config(config_path).map_err(|e| anyhow::anyhow!("Configuration validation failed: {}", e))?;
    Ok(())
}
```

### `evaluate_and_enforce_operation()` Method

`CommandContext::evaluate_and_enforce_operation()` wraps `self.enforcement.approve_manual(&descriptor)` (Phase 12 `EnforcementContext::approve_manual` in `config/policy_decision.rs`) with profile-aware scope enforcement and structured denial output. For `ManualPermissive`, it produces an `ApprovedOperation` token on `Allow`, `Warn` (with warning), or `RequireConfirmation` with matching override. Strict profiles treat `Warn` and `RequireConfirmation` as denial. The method maps `EnforcementError` variants to structured CLI error output.
1. Calls `self.enforcement.approve_manual(surface, &descriptor, Some(&self.manual_override))`
2. On `Ok(approved)`: returns the `ApprovedOperation` (carries decision, surface, profile, audit_event_id)
3. On `Err(EnforcementError::Denied)`: returns an error with the `PolicyDecision` details
4. On `Err(EnforcementError::ConfirmationRequired)`: returns an error listing the required override flags
5. On `Err(EnforcementError::ManualOverrideUnavailable)`: returns an error for strict surfaces

## Workflow

1. `main.rs` parses arguments using `Cli::parse()`.
2. Logging is initialized.
3. Configuration and Scope are loaded.
4. `CommandContext` is created.
5. `handle_command` (implemented in `src/commands/handlers/mod.rs`) dispatches to a specific handler in `src/commands/handlers/`.
6. The handler executes the requested operation, often interacting with other core modules like `scanner` or `fuzzer`.

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
- **`eggsec session list|create|snapshot`**: Manage sessions on the daemon (list sessions, create new session, get session snapshot with task statuses).
- **`eggsec task submit|cancel|watch`**: Manage tasks on the daemon (submit a tool command, cancel a task, watch task events via streaming).

All daemon commands use JSON line protocol over Unix sockets. The `--socket` flag specifies the socket path (default: `~/.eggsec/daemon.sock`). Task submission builds a `RunRequest` from CLI args and sends it to the daemon for execution. All daemon CLI sessions use `RuntimeSurface::CliManual` (not `TuiManual`).

**Dispatch pattern**: Daemon commands are intercepted in `main.rs` before reaching the `handle_command` exhaustive match. The `daemon_cli::is_daemon_command()` check routes daemon/session/task variants to `daemon_cli::handle_daemon_command()`. The `eggsec` crate handler match arms for these variants return `anyhow::bail!` (unreachable in practice).

## Special Cases (Standalone Commands)

Some target-bearing commands are intentionally standalone and bypass (or optionally participate in) the canonical `ScanReportData` / `eggsec-output` conversion pipeline. They emit module-local report types directly as JSON/text or to `-o` files.

- `auth-test` (handler: `commands/handlers/auth_test.rs`): Uses local `AuthTestReport`/`AuthFinding` (defined in `auth/mod.rs`). Policy gate via `evaluate_and_enforce_operation` with `OperationRisk::CredentialTesting`. **No** conversion to `FindingData`/`ScanReportData`, no SARIF/JUnit/etc. via the output crate, and no integration with `eggsec scan --profile` pipelines. Distinct from `ScanProfile::Auth` (JWT/OAuth/IDOR fuzzer stages). See `architecture/auth.md`, `docs/AUTH_LAB.md`, and `commands/handlers/auth_test.rs:274-285` (direct emit logic).
- `mobile` (handler: `commands/handlers/mobile.rs`): Standalone defense-lab CLI (`eggsec mobile <apk-or-ipa>`). Uses local `MobileScanReport`/`MobileFinding` (defined in `mobile/mod.rs`). Policy via `evaluate_and_enforce_operation` with `OperationRisk::SafeActive` + `required_features: ["mobile"]`. Optional `to_scan_report_data` bridge for JSON/SARIF/JUnit consumers (mirrors wireless pattern); no pipeline profile integration in Phase 1 (no `mobile-static`/`mobile-regression` profiles yet). Pure-Rust static analysis only (APK/IPA manifest/config). Native `--json` is auto-bridged by the report handler. See `architecture/mobile.md`, `crates/eggsec/src/mobile/mod.rs`, `crates/eggsec/src/commands/handlers/mobile.rs`, and `crates/eggsec/src/cli/mobile.rs`. Dynamic loadout per `plans/dynamic-mobile-testing-loadout-design-plan.md` (still standalone, no MCP exposure; mirrors wireless bullet).
- `wireless` (handler: `commands/handlers/wireless.rs`): Standalone-complete passive WiFi recon (CLI + TUI tab under `wireless` feature). Uses local `WirelessScanResult`/`WirelessNetwork` (defined in `wireless/mod.rs`). Policy via `SafeActive` + `wireless` feature (central `evaluate_and_enforce_operation`). Optional `to_scan_report_data` bridge (populates findings + full `wireless_networks`); native `--json` (or `--repeat` wrapped form) is auto-bridged by the report handler for `report convert`. Not integrated with `ScanProfile` pipelines or dedicated profiles. **MCP / agentic tool exposure**: intentionally none (not registered as SecurityTool; invisible to tools/list and CodingAgent/OpsAgent profiles; see architecture/wireless.md MCP/Agentic section). Part of the consolidated "standalone defense-lab surfaces" (wireless + mobile + auth-test) pattern. See `architecture/wireless.md` (MCP/Agentic section + Integration), `architecture/defense_lab.md`, `docs/USAGE.md` (Output Models), and AGENTS.md (standalone note). Active extensions (Phase 1+) per `plans/wireless-active-attacks-loadout-design-plan.md` (still standalone, no MCP exposure).
- `db-pentest` (handler: `commands/handlers/db_pentest.rs`): Standalone defense-lab database security assessment (Phase 1-5). Uses local `DbPentestReport`/`DbFinding` (defined in `db_pentest/types.rs`). Policy gate via `evaluate_and_enforce_operation` with `OperationRisk::DbPentest` (real) / `SafeActive` (dry-run) + `--allow-db-pentest` for non-dry runs. Optional `to_scan_report_data_db` bridge for JSON/SARIF/JUnit consumers; native `--json` auto-bridged by report handler. TUI tab `Tab::DbPentest` + native `Stage::DbPentest` pipeline stage (via `ScanProfile::DbRegression`). Phase 5 adds MongoDB/Redis engines, cross-DB correlation, compliance mapping, optional MCP exposure via `db-pentest-mcp` marker. See `architecture/database_pentest.md`, `architecture/defense_lab.md`, `architecture/pipeline.md`.

Distinction: `auth-test` bypasses entirely (local `Auth*` only, direct handler emit, no bridge or conversion path). `wireless` and `mobile` emit local types directly for their surfaces but expose an optional `to_scan_report_data` bridge (used by `eggsec-output` converters and auto-bridged in `report convert`) for unified formats. None of the three have pipeline stage integration. Design decision recorded in the per-module architecture docs and `integration-work-plan.md` close-out.
