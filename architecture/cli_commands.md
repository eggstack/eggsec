# CLI & Commands

The CLI and Commands layer is responsible for parsing user input, managing global state (CommandContext), and dispatching execution to the appropriate handlers.

## CLI Parsing (`src/cli/`)

Eggsec uses `clap` for command-line argument parsing. The CLI is organized into several modules, each defining the arguments for a specific category of commands:

- **`mod.rs`**: Defines the main `Cli` entry point, `Commands` enum (39 variants), and `CommonHttpArgs`.
- **`scan.rs`**: Arguments for the `scan` command (port scanning, endpoint discovery).
- **`fuzz.rs`**: Arguments for the `fuzz` command (security fuzzing).
- **`http.rs`**: Arguments for HTTP-specific operations (load, recon, graphql, oauth).
- **`packet.rs` & `stress.rs`**: Arguments for low-level networking and stress testing.
- **`agent.rs` & `ai_analyze.rs`**: Arguments for AI-driven features.

### Key CLI Patterns

- **Global flags**: `--json`, `--config`, `--scope`, `--strict-scope` apply to all commands
- **Feature-gated commands**: `stress-testing`, `packet-inspection`, `nse`, `ai-integration`, `rest-api`, `grpc-api`, `sbom`
- **Output flag**: Use `-o` / `--output` for file output (consistent across commands)
- **Scope validation**: Handlers call `evaluate_and_enforce_operation()` with an `OperationDescriptor` to validate targets against scope and execution policy

## Command Dispatch (`src/commands/`)

Once arguments are parsed, the `main` function initializes a `CommandContext` and calls `handle_command` via `src/commands/mod.rs` re-exports. The implementation lives in `src/commands/handlers/mod.rs`.

- **`CommandContext`**: Carries global state including the loaded `EggsecConfig`, `Scope`, output preferences, and `execution_profile` (defaults to `ManualPermissive`; set to `ManualGuarded` by `--strict-scope`, `CiStrict` in CI mode).
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
    ctx.evaluate_and_enforce_operation(OperationDescriptor {
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
    // ... proceed
}

// Error handling - return Result, never std::process::exit()
pub async fn handle_config(_ctx: &CommandContext, args: ConfigArgs) -> Result<()> {
    load_config(config_path).map_err(|e| anyhow::anyhow!("Configuration validation failed: {}", e))?;
    Ok(())
}
```

### `evaluate_and_enforce_operation()` Method

`CommandContext::evaluate_and_enforce_operation()` wraps `self.enforcement.evaluate(&descriptor)` (central `EnforcementContext::evaluate` in `config/policy_decision.rs`) with profile-aware scope enforcement and structured denial output. The central `evaluate` performs LoadedScope provenance checks, DenialClass downgrade (ManualPermissive only), positive capability checks for strict profiles, and risk/feature/policy enforcement; legacy direct `evaluate_enforcement`/`evaluate_operation_policy` calls are internal/deprecated for denial paths.
1. Calls `self.enforcement.evaluate(&descriptor)`
2. On `Allow`: returns the `PolicyDecision`
3. On `Warn`: logs warnings and returns the `PolicyDecision` (manual permissive mode)
4. On `Deny`: returns an error containing the `PolicyDecision` details (JSON or human-readable)

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
6. **`handlers/auth_test.rs:10`**: Migrated from `ensure_scope_url` to `evaluate_and_enforce_operation` with `CredentialTesting` risk tier
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
