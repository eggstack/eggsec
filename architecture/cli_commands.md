# CLI & Commands

The CLI and Commands layer is responsible for parsing user input, managing global state (CommandContext), and dispatching execution to the appropriate handlers.

## CLI Parsing (`src/cli/`)

Slapper uses `clap` for command-line argument parsing. The CLI is organized into several modules, each defining the arguments for a specific category of commands:

- **`mod.rs`**: Defines the main `Cli` entry point, `Commands` enum (35+ variants), and `CommonHttpArgs`.
- **`scan.rs`**: Arguments for the `scan` command (port scanning, endpoint discovery).
- **`fuzz.rs`**: Arguments for the `fuzz` command (security fuzzing).
- **`http.rs`**: Arguments for HTTP-specific operations (load, recon, graphql, oauth).
- **`packet.rs` & `stress.rs`**: Arguments for low-level networking and stress testing.
- **`agent.rs` & `ai_analyze.rs`**: Arguments for AI-driven features.

### Key CLI Patterns

- **Global flags**: `--json`, `--config`, `--scope` apply to all commands
- **Feature-gated commands**: `stress-testing`, `packet-inspection`, `nse`, `ai-integration`, `rest-api`, `grpc-api`, `sbom`
- **Output flag**: Use `-o` / `--output` for file output (consistent across commands)
- **Scope validation**: Handlers call `ensure_scope()` or `ensure_scope_url()` to validate targets

## Command Dispatch (`src/commands/`)

Once arguments are parsed, the `main` function initializes a `CommandContext` and calls `handle_command` via `src/commands/mod.rs` re-exports. The implementation lives in `src/commands/handlers/mod.rs`.

- **`CommandContext`**: Carries global state including the loaded `SlapperConfig`, `Scope`, and output preferences.
- **`handle_command`**: A large exhaustive match statement that dispatches to the correct handler based on the subcommand.
  Because it is exhaustive (no wildcard arm), adding/removing `Commands` variants requires updating dispatch at compile time.

## Handlers (`src/commands/handlers/`)

Actual command execution logic resides in the `handlers` directory. Each handler is typically an `async` function that takes the parsed arguments and the `CommandContext`.

Examples:
- **`scan.rs`**: Entry point for port scanning and reconnaissance.
- **`fuzz.rs`**: Entry point for the security fuzzing engine.
- **`cluster.rs`**: Manages distributed scanning nodes.
- **`plugin.rs`**: Handles execution of external Python/Ruby plugins.

### Handler Patterns

```rust
// Scope validation (required for target-based commands)
pub async fn handle_fuzz(ctx: &CommandContext, args: FuzzArgs) -> Result<()> {
    ctx.ensure_scope_url(&args.url)?;
    // ... proceed
}

// Error handling - return Result, never std::process::exit()
pub async fn handle_config(_ctx: &CommandContext, args: ConfigArgs) -> Result<()> {
    load_config(config_path).map_err(|e| anyhow::anyhow!("Configuration validation failed: {}", e))?;
    Ok(())
}
```

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
4. **`handlers/mod.rs:155-169`**: Replaced hardcoded command list in `handle_no_command` with guidance to use `slapper --help`
5. **`handlers/cluster.rs:348`**: Replaced `unwrap_or(22)` with `unwrap_or_else(|_| 22)` to avoid panic on invalid parsing
6. **`handlers/auth_test.rs:10`**: Added missing scope validation `ctx.ensure_scope_url(&args.target)?`

### CLI Consistency Guidelines

| Issue | Recommendation |
|-------|----------------|
| `--host` vs `--target` vs `--url` | Use `--target` for hosts, `--url` for endpoints |
| Timeout defaults | Use 15s as standard default |
| WAF profile | Use `String` (not `ValueEnum`) for flexibility |
| Source IP naming | `source_ip` / `source_port` (not `spoof_ip`) |

## Skills Reference

- `.opencode/skills/slapper-cli/` - Full CLI patterns and handler guide
