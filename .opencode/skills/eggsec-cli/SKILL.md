---
name: eggsec-cli
description: "CLI parsing, command dispatch, and handler patterns - use when adding CLI commands, working with argument parsing, handler dispatch, policy enforcement, or feature gating."
---

# Eggsec CLI Commands Skill

CLI parsing, command dispatch, and handler patterns for Eggsec.

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
- `misc.rs` - `ConfigArgs`, `NotifyArgs`, `RemoteArgs`, `ExecArgs`, `ReportArgs`, `SbomArgs`
- `agent.rs` - `AgentArgs`, agent management commands
- `ai_analyze.rs` - `AiAnalyzeArgs`, AI analysis commands
- `browser.rs` - `BrowserArgs`, headless browser commands
- `ci.rs` - `CiArgs`, CI/CD integration commands
- `hunt.rs` - `HuntArgs`, vulnerability hunting commands
- `packet.rs` - `PacketArgs`, packet inspection commands
- `plan.rs` - `PlanArgs`, execution planning commands
- `timeout.rs` - `TimeoutArgs`, timeout configuration commands
- `wireless.rs` - `WirelessArgs`, wireless scanning commands

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
    // ... 52 variants (33 base, 52 total with all features)
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
    pub config: EggsecConfig,
    pub scope: Scope,
    pub json: bool,
    config_path: Option<String>,
}

impl CommandContext {
    pub fn evaluate_and_enforce_operation(&self, descriptor: OperationDescriptor) -> Result<PolicyDecision>
    // Deprecated legacy methods (no callers; scope checks centralized in EnforcementContext::evaluate()):
    #[deprecated] pub fn ensure_scope(&self, target: &str) -> ErrorResult<()>
    #[deprecated] pub fn ensure_scope_url(&self, url: &str) -> ErrorResult<()>
}
```
Current `evaluate_and_enforce_operation` behavior for ManualPermissive `RequireConfirmation`: narrow `--yes` (only `out-of-scope`/`target-expansion`); dedicated `--allow-private-resolution` / `--allow-cross-host-redirect` etc. for their classes; stable kebab-case audit strings via `ConfirmationClass::as_str()` and `confirmation_class_strings` dedup helper; precise "required flag" error messages listing exactly what is missing. Strict profiles/MCP/agent treat RequireConfirmation as hard Deny and ignore overrides.

## Handler Files

| Handler | File | Feature Gate | Purpose |
|---------|------|--------------|---------|
| `handle_scan_ports` | `handlers/scan.rs` | - | TCP port scanning |
| `handle_scan_endpoints` | `handlers/scan.rs` | - | Hidden endpoint discovery |
| `handle_fingerprint` | `handlers/scan.rs` | - | Service fingerprinting |
| `handle_nse` | `handlers/scan.rs` | `nse` | NSE script execution |
| `handle_fuzz` | `handlers/fuzz.rs` | - | Security fuzzing |
| `handle_waf` | `handlers/fuzz.rs` | - | WAF detection/bypass |
| `handle_waf_stress` | `handlers/fuzz.rs` | - | WAF stress testing |
| `handle_graphql` | `handlers/fuzz.rs` | - | GraphQL testing |
| `handle_oauth` | `handlers/fuzz.rs` | - | OAuth/OIDC testing |
| `handle_load` | `handlers/load.rs` | - | HTTP load testing |
| `handle_recon` | `handlers/recon.rs` | - | Reconnaissance |
| `handle_auth_test` | `handlers/auth_test.rs` | - | Auth security testing |
| `handle_packet` | `handlers/network.rs` | `packet-inspection` | Packet inspection |
| `handle_icmp` | `handlers/network.rs` | `stress-testing` | ICMP probing |
| `handle_traceroute` | `handlers/network.rs` | `stress-testing` | Traceroute |
| `handle_stress` | `handlers/stress.rs` | `stress-testing` | Stress/DoS testing |
| `handle_proxy` | `handlers/stress.rs` | `stress-testing` | Proxy pool management |
| `handle_config` | `handlers/config.rs` | - | Config validation |
| `handle_sbom` | `handlers/sbom.rs` | `sbom` | SBOM generation |
| `handle_vuln` | `handlers/vuln.rs` | - | Vulnerability management |
| `handle_storage` | `handlers/storage.rs` | - | Storage operations |
| `handle_cluster` | `handlers/cluster.rs` | - | Cluster management |
| `handle_remote` | `handlers/cluster.rs` | - | Remote listener |
| `handle_exec` | `handlers/cluster.rs` | - | Remote execution |
| `handle_notify` | `handlers/notify.rs` | - | Notifications |
| `handle_serve` | `handlers/serve.rs` | `rest-api` | REST API server; uses `EnforcementContext` with `McpStrict` profile; only `Allow` permits dispatch |
| `handle_mcp_serve` | `handlers/serve.rs` | `rest-api` | MCP server |
| `handle_agent` | `handlers/agent.rs` | `rest-api` | Autonomous agent |
| `handle_ai_analyze` | `handlers/ai_analyze.rs` | `ai-integration` | AI analysis |
| `handle_grpc_server` | `handlers/grpc.rs` | `grpc-api` | gRPC server |
| `handle_preflight` | `handlers/preflight.rs` | - | Advisory policy preflight for a target; uses shared `preflight_operation()` |
| `handle_plan` | `handlers/plan.rs` | - | Execution planning |
| `handle_ci` | `handlers/ci.rs` | - | CI/CD checks |
| `handle_report` | `handlers/report.rs` | - | Report generation |

**Daemon commands** (feature-gated: `daemon-client`, dispatched in `main.rs` before general handler):

| Command | Source | Purpose |
|---------|--------|---------|
| `daemon start/status/stop` | `daemon_cli.rs` | Daemon lifecycle management |
| `daemon history` | `daemon_cli.rs` | List persisted sessions |
| `daemon show <id>` | `daemon_cli.rs` | Show persisted snapshot |
| `session list/create/snapshot` | `daemon_cli.rs` | Session introspection |
| `task submit/cancel/watch` | `daemon_cli.rs` | Task lifecycle management |

Daemon commands connect via Unix socket using `DaemonClient` from `eggsec-daemon`. The daemon also supports an optional `http-api` loopback HTTP transport (feature-gated). Authorization uses `CommandPermission` enum — `DeclareClient` must succeed before session-scoped commands are allowed.

**Local lifecycle smoke test:** `scripts/smoke-daemon-local.sh` is the canonical local-only daemon validation. It uses an ephemeral socket and `mktemp -d` workspace, runs pre-built binaries (no `cargo run` recompile noise), and exercises observer-deny + owner-allow posture in addition to standard lifecycle steps. Run with `bash scripts/smoke-daemon-local.sh` or `bash scripts/smoke-daemon-local.sh /custom/socket/path`.

## Common Patterns

### Policy Enforcement
All target-bearing handlers call `ctx.evaluate_and_enforce_operation()` with an `OperationDescriptor` before processing:
```rust
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
cargo test --lib -p eggsec commands::
cargo test --lib -p eggsec cli::
```

### Writing Tests
- Test argument parsing with `Cli::parse_from()`
- Test handler dispatch by mocking `CommandContext`
- Verify policy enforcement via `evaluate_and_enforce_operation` (see `commands::handlers::tests`)

## Common Tasks

### Adding a New CLI Command
1. Add arguments struct in appropriate `cli/*.rs` file
2. Add variant to `Commands` enum in `cli/mod.rs`
3. Add handler function in `commands/handlers/*.rs`
4. Add dispatch arm in `handle_command()` (exhaustive match)
5. Gate with feature flag if needed
6. Add `evaluate_and_enforce_operation` with appropriate `OperationDescriptor` in handler
7. Add tests

### Preflight Command
`eggsec preflight <operation> --target <target> [--json]` runs advisory policy evaluation without executing. Uses `metadata_for_tool_id()` + `preflight_operation()` from `config::policy_decision`. Output includes outcome kind, decision, suggested flags, and required confirmation classes.

### Daemon Persistence Commands
`eggsec daemon history [--json]` lists all persisted daemon sessions. `eggsec daemon show <id> [--json]` displays a full snapshot of a persisted session including task results. Both support `--json` for structured output.

### Bug Fixes in Handlers
- **Never use `unwrap()`** - Use `ok_or_else()` or `context()`
- **Never call `std::process::exit()`** - Return `Err(...)` instead
- **Always validate policy** - Call `evaluate_and_enforce_operation()` with an `OperationDescriptor`
- **Never use `unwrap_or()` with constants** - Use `unwrap_or_else(|| ...)` to avoid panics

### Known Bug Fixes (2026-05-22)
| Issue | Fix | Location |
|-------|-----|----------|
| Missing policy validation in auth-test | Migrated to `evaluate_and_enforce_operation` with `CredentialTesting` risk | `handlers/auth_test.rs` |
| Hardcoded list in `handle_no_command` | Replaced with `eggsec --help` guidance | `handlers/mod.rs:155-169` |
| `unwrap_or(22)` in cluster parse | Changed to `unwrap_or_else(\|_\| 22)` | `handlers/cluster.rs:350` |
| Unused `-o` flag in `ClusterArgs` | Removed dead code | `cli/cluster.rs` |

### Output Flag (`-o`) Consistency (2026-05-22)
All CLI argument structs now have consistent `-o`/`--output` short flag:
- `PortScanArgs`, `EndpointScanArgs`, `FuzzArgs`, `WafStressArgs`, `WafArgs`, `LoadArgs`, `GraphQlArgs`, `OAuthArgs`, `FingerprintArgs`, `NseArgs`, `ResumeArgs`, `ScanArgs`, `ReconArgs`

**Note:** `ClusterArgs` intentionally does NOT have an output flag since cluster commands are interactive management operations that don't produce file output.

### Type Conversions for FuzzArgs
Several `From` implementations exist for converting CLI args to `FuzzArgs`:
- `From<WafStressArgs>` - defined in `cli/fuzz.rs` (WAF stress testing)
- `From<GraphQlArgs>` - defined in `commands/fuzz_convert.rs`
- `From<OAuthArgs>` - defined in `commands/fuzz_convert.rs`

## Resources
- `architecture/cli_commands.md` - CLI architecture documentation
- `AGENTS.md` - General project guidelines
- `architecture/overview.md` - Overall design