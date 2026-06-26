# CLI Architecture

**Date**: 2026-04-18
**Status**: Current

## Overview

The Eggsec CLI is built using `clap` for argument parsing and a handler-based dispatch system for command execution. This document describes the architecture patterns used.

## Command Structure

### Cli Root (`cli/mod.rs`)

The root `Cli` struct contains global options and a subcommand enum:

```rust
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
    pub json: bool,          // Global: JSON output
    pub config: Option<String>, // Global: Config file path
    pub scope: Option<String>, // Global: Scope file path
}
```

### Commands Enum

The `Commands` enum is organized into logical groups:

| Group | Commands |
|-------|----------|
| Scan operations | `ScanPorts`, `ScanEndpoints`, `Fingerprint`, `Scan`, `Resume` |
| Attack operations | `Fuzz`, `Waf`, `WafStress`, `Graphql`, `OAuth`, `AuthTest` |
| Recon operations | `Recon` |
| Planning & CI | `Plan`, `Ci`, `Sbom` |
| Load testing | `Load` |
| Tool operations | `Packet`, `Nse`, `Report` |
| Stress testing | `Stress`, `Proxy`, `Icmp`, `Traceroute` |
| Infrastructure | `Cluster`, `Notify`, `Remote`, `Exec`, `Serve`, `McpServe`, `Agent` |
| AI operations | `AiAnalyze` |
| Security testing | `Mobile`, `Wireless`, `Evasion`, `Postex`, `C2`, `DbPentest` |
| Proxy | `WebProxy` |
| Browser | `Browser` |
| Hunting | `Hunt` |
| Vulnerability | `Vuln` |
| Storage | `Storage` |

### Handler Organization (`commands/handlers/`)

Each handler module corresponds to a command group:

```
handlers/
├── mod.rs          # CommandContext, handle_command dispatcher
├── scan.rs         # Port scan, endpoint scan, fingerprint, scan, resume
├── fuzz.rs         # Fuzz, WAF stress, WAF, GraphQL, OAuth
├── recon.rs        # Reconnaissance
├── load.rs         # HTTP load testing
├── ci.rs           # CI/CD mode
├── plan.rs         # Plan preview
├── cluster.rs      # Cluster operations
├── network.rs      # Packet, ICMP, traceroute
├── stress.rs       # Stress, proxy
├── report.rs       # Report conversion and scheduling
├── notify.rs       # Notifications, serve, mcp-serve
├── auth_test.rs    # Authentication testing
├── agent.rs        # Autonomous agent
├── ai_analyze.rs   # AI analysis
├── sbom.rs         # SBOM generation
├── browser.rs      # Headless browser testing
├── config.rs       # Configuration management
├── db_pentest.rs   # Database pentesting
├── doctor.rs       # System diagnostics
├── explain.rs      # Finding explanations
├── grpc.rs         # gRPC API server
├── hunt.rs         # Advanced threat hunting
├── mobile.rs       # Mobile app security
├── storage.rs      # Database storage
├── vuln.rs         # Vulnerability management
├── web_proxy.rs    # Web proxy interception
└── wireless.rs     # Wireless security testing
```

## Command Dispatch

### handle_command Function

The main dispatcher in `handlers/mod.rs` uses a match statement:

```rust
pub async fn handle_command(cli: Cli, ctx: &CommandContext) -> Result<()> {
    match cli.command {
        None => handle_no_command(&cli).await,
        Some(Commands::ScanPorts(args)) => handle_scan_ports(ctx, args).await,
        Some(Commands::Fuzz(args)) => handle_fuzz(ctx, args).await,
        // ... 30+ arms
    }
}
```

### CommandContext

Provides shared state and utilities:

```rust
pub struct CommandContext {
    pub config: EggsecConfig,
    pub scope: Scope,
    pub json: bool,
    config_path: Option<String>,
}

impl CommandContext {
    pub fn ensure_scope(&self, target: &str) -> ErrorResult<()>
    pub fn ensure_scope_url(&self, url: &str) -> ErrorResult<()>
}
```

### require_scope! Macro

For convenient scope validation with clear error messages:

```rust
require_scope!(ctx, target)?;
require_scope!(ctx, url = url)?;
```

## Handler Patterns

### Standard Signature

All handlers follow this pattern:

```rust
pub async fn handle_<command>(
    ctx: &CommandContext,
    args: <CommandArgs>
) -> Result<()> {
    // Scope validation
    ctx.ensure_scope(&args.target)
        .context("<command> requires valid target in scope")?;
    
    // Execution with context
    some_module::run_cli(args).await
        .context("<command> failed")?;
    
    Ok(())
}
```

### Error Handling

Handlers use `anyhow::Result` with `.context()` for error chaining:

```rust
ctx.ensure_scope(&args.url)
    .context("fuzz command requires valid URL in scope")?;
crate::fuzzer::run_cli(args).await
    .context("fuzz execution failed")?;
```

### Doc Comments

Handlers include doc comments with `# Errors` sections:

```rust
/// Executes fuzzing attacks against the target URL.
///
/// # Errors
///
/// Returns an error if:
/// - The URL fails scope validation
/// - Fuzzing execution fails
pub async fn handle_fuzz(ctx: &CommandContext, args: crate::cli::FuzzArgs) -> Result<()> {
```

## CommonHttpArgs

Shared HTTP arguments for security testing commands:

```rust
pub struct CommonHttpArgs {
    pub insecure: bool,       // Skip TLS verification
    pub proxy: Option<String>,
    pub proxy_auth: Option<String>,
    pub auth: Option<String>, // Basic auth (user:pass)
    pub bearer: Option<String>,
    pub cookie: Option<String>,
    pub api_key: Option<String>,
    pub user_agent: Option<String>,
    pub stealth: bool,
    pub rate_limit: Option<u32>,
    pub jitter: Option<String>,
}
```

## Common Patterns

### Feature-Gated Commands

Commands with feature gates use both arms:

```rust
#[cfg(feature = "nse")]
Some(Commands::Nse(args)) => handle_nse(ctx, args).await,
#[cfg(not(feature = "nse"))]
Some(Commands::Nse(_)) => anyhow::bail!("NSE support requires the 'nse' feature"),
```

### Scope Validation

Always validate scope before executing commands that interact with targets:

```rust
pub async fn handle_fuzz(ctx: &CommandContext, args: crate::cli::FuzzArgs) -> Result<()> {
    ctx.ensure_scope_url(&args.url)
        .context("fuzz command requires valid URL in scope")?;
    // ... execute
}
```

## G1 Improvements (Completed)

- A1: Command dispatch uses exhaustive match (42+ arms) — no wildcard, catches missing variants at compile time
- C1: `.context()` added to all handlers
- C2: Standardized to `anyhow::Result` for command handlers
- C4: `require_scope!` macro available (legacy; replaced by `evaluate_and_enforce_operation`)

## G2 Improvements (Completed)

- A2: Flat `Commands` enum with 42+ variants — kept flat for exhaustive match safety
- A3: `CommonHttpArgs` documentation complete
- C3: All handler functions have doc comments with `# Errors` sections
- C5: Error propagation standardized via `anyhow::context()`

## G3 UX Consistency

- U1: Short flags standardized across all CLI argument structs (except `ClusterArgs` which is interactive-only)
- U2: Progressive disclosure via `help_heading` for option groups
- U3: Output formats documented per command in `--help`

## Dependencies

- `clap` for CLI parsing
- `clap_complete` for shell completion
- `anyhow` for error handling
