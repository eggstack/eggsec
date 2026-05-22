# Slapper CLI Commands Skill

CLI parsing, command dispatch, and handler patterns for Slapper.

## Overview

The CLI layer parses user input and dispatches to handler modules. The two-layer architecture separates argument parsing (`src/cli/`) from execution dispatch (`src/commands/`).

## Directory Structure

```
src/cli/           # Argument parsing (clap)
src/commands/      # Handler dispatch and implementations
```

## CLI Parsing (`src/cli/`)

### Main Entry Point
- `mod.rs` - `Cli` struct with global flags and `Commands` enum
- `scan.rs` - `PortScanArgs`, `EndpointScanArgs`, `FingerprintArgs`, `NseArgs`, `ScanArgs`, `ResumeArgs`
- `fuzz.rs` - `FuzzArgs`, `WafStressArgs`, `WafArgs`
- `http.rs` - `LoadArgs`, `ReconArgs`, `GraphQlArgs`, `OAuthArgs`
- `auth.rs` - `AuthTestArgs`
- `stress.rs` - `IcmpArgs`, `TracerouteArgs`, `StressArgs`, `ProxyArgs`
- `cluster.rs` - `ClusterArgs`, `ClusterCommand`
- `storage.rs` - `StorageArgs`, `StorageCommand`
- `vuln.rs` - `VulnArgs`, `VulnCommand`
- `misc.rs` - `ConfigArgs`, `NotifyArgs`, `RemoteArgs`, `ExecArgs`, `PluginArgs`, `ReportArgs`, `SbomArgs`

### Key Types

```rust
pub struct Cli {
    pub command: Option<Commands>,
    pub json: bool,
    pub config: Option<String>,
    pub scope: Option<String>,
    pub generate_config: bool,
    pub generate_shell_completion: Option<Shell>,
}

pub enum Commands {
    ScanPorts(PortScanArgs),
    ScanEndpoints(EndpointScanArgs),
    Fingerprint(FingerprintArgs),
    Scan(ScanArgs),
    Resume(ResumeArgs),
    Fuzz(FuzzArgs),
    Waf(WafArgs),
    // ... 35+ variants
}
```

### Common Arguments
`CommonHttpArgs` provides shared HTTP arguments:
- `--insecure` - Skip TLS verification
- `--proxy`, `--proxy-auth` - HTTP proxy
- `--auth`, `--bearer`, `--cookie`, `--api-key` - Authentication
- `--user-agent`, `--stealth` - Request customization
- `--rate-limit`, `--jitter` - Rate limiting

## Command Dispatch (`src/commands/handlers/`)

### Handle Command Pattern
```rust
pub async fn handle_command(cli: Cli, ctx: &CommandContext) -> Result<()> {
    match cli.command {
        None => handle_no_command(&cli).await,
        Some(Commands::ScanPorts(args)) => handle_scan_ports(ctx, args).await,
        Some(Commands::Fuzz(args)) => handle_fuzz(ctx, args).await,
        // ... exhaustive match (no wildcard arm)
    }
}
```

### CommandContext
```rust
pub struct CommandContext {
    pub config: SlapperConfig,
    pub scope: Scope,
    pub json: bool,
    config_path: Option<String>,
}

impl CommandContext {
    pub fn ensure_scope(&self, target: &str) -> ErrorResult<()>
    pub fn ensure_scope_url(&self, url: &str) -> ErrorResult<()>
}
```

## Handler Files

| Handler | File | Purpose |
|---------|------|---------|
| `handle_scan_ports` | `handlers/scan.rs` | TCP port scanning |
| `handle_scan_endpoints` | `handlers/scan.rs` | Hidden endpoint discovery |
| `handle_fingerprint` | `handlers/scan.rs` | Service fingerprinting |
| `handle_fuzz` | `handlers/fuzz.rs` | Security fuzzing |
| `handle_waf` | `handlers/fuzz.rs` | WAF detection/bypass |
| `handle_waf_stress` | `handlers/fuzz.rs` | WAF stress testing |
| `handle_load` | `handlers/load.rs` | HTTP load testing |
| `handle_recon` | `handlers/recon.rs` | Reconnaissance |
| `handle_graphql` | `handlers/fuzz.rs` | GraphQL testing |
| `handle_oauth` | `handlers/fuzz.rs` | OAuth/OIDC testing |
| `handle_auth_test` | `handlers/auth_test.rs` | Auth security testing |
| `handle_packet` | `handlers/network.rs` | Packet inspection |
| `handle_icmp` | `handlers/network.rs` | ICMP probing |
| `handle_traceroute` | `handlers/network.rs` | Traceroute |
| `handle_stress` | `handlers/stress.rs` | Stress/DoS testing |
| `handle_config` | `handlers/config.rs` | Config validation |
| `handle_sbom` | `handlers/sbom.rs` | SBOM generation |
| `handle_vuln` | `handlers/vuln.rs` | Vulnerability management |
| `handle_storage` | `handlers/storage.rs` | Storage operations |
| `handle_cluster` | `handlers/cluster.rs` | Cluster management |

## Common Patterns

### Scope Validation
Most handlers call `ctx.ensure_scope()` or `ctx.ensure_scope_url()` before processing:
```rust
pub async fn handle_fuzz(ctx: &CommandContext, args: FuzzArgs) -> Result<()> {
    ctx.ensure_scope_url(&args.url)?;
    // ... proceed with fuzzing
}
```

### Error Handling
Handlers return `Result<()>` and use `map_err()` for context:
```rust
Ok::<(), anyhow::Error>(/* ... */)
```

### Feature Gating
Commands and handlers are gated with the same feature flags:
```rust
// CLI (mod.rs)
#[cfg(feature = "stress-testing")]
Stress(StressArgs),

// Handler (handlers/mod.rs)
#[cfg(feature = "stress-testing")]
Some(Commands::Stress(args)) => handle_stress(ctx, args).await,
```

## Testing

### Running CLI Tests
```bash
cargo test --lib -p slapper commands::
cargo test --lib -p slapper cli::
```

### Writing Tests
- Test argument parsing with `Cli::parse_from()`
- Test handler dispatch by mocking `CommandContext`
- Verify scope validation calls

## Common Tasks

### Adding a New CLI Command
1. Add arguments struct in appropriate `cli/*.rs` file
2. Add variant to `Commands` enum in `cli/mod.rs`
3. Add handler function in `commands/handlers/*.rs`
4. Add dispatch arm in `handle_command()` (exhaustive match)
5. Gate with feature flag if needed
6. Add scope validation in handler
7. Add tests

### Bug Fixes in Handlers
- **Never use `unwrap()`** - Use `ok_or_else()` or `context()`
- **Never call `std::process::exit()`** - Return `Err(...)` instead
- **Always validate scope** - Call `ensure_scope()` or `ensure_scope_url()`
- **Never use `unwrap_or()` with constants** - Use `unwrap_or_else(|| ...)` to avoid panics

### Known Bug Fixes (2026-05-22)
| Issue | Fix | Location |
|-------|-----|----------|
| Missing scope validation in auth-test | Added `ctx.ensure_scope_url(&args.target)?` | `handlers/auth_test.rs:10` |
| Hardcoded `unwrap_or(22)` in parse | Changed to `unwrap_or_else(\|_\| 22)` | `handlers/cluster.rs:348` |
| Hardcoded list in `handle_no_command` | Replaced with `slapper --help` guidance | `handlers/mod.rs:155-169` |

## Resources
- `architecture/cli_commands.md` - CLI architecture documentation
- `AGENTS.md` - General project guidelines
- `ARCHITECTURE.md` - Overall design