# CLI & Commands Architecture Review

**Document:** architecture/cli_commands.md
**Reviewed:** 2026-05-31
**Accuracy:** Medium-High
**Lines Reviewed:** 101

## Verified Claims

- **CLI uses clap**: Verified at `cli/mod.rs:1` (`use clap::{Parser, Subcommand, ValueEnum}`)
- **Main `Cli` entry point**: Defined at `cli/mod.rs:56-79` with `#[derive(Parser)]`
- **`Commands` enum**: Defined at `cli/mod.rs:81-201` with `#[derive(Subcommand)]`
- **Commands enum has 35+ variants**: Verified -- actual count is **37 variants** (37 > 35, claim is accurate but imprecise)
- **`CommonHttpArgs`**: Defined at `cli/mod.rs:203-230`
- **Global flags `--json`, `--config`, `--scope`**: Verified at `cli/mod.rs:65-72`
- **Feature-gated commands list**: `stress-testing` (line 146-157), `packet-inspection` (line 132-134), `nse` (line 135-137), `ai-integration` (line 193-195), `rest-api` (line 168-190), `grpc-api` (line 198-200), `sbom` (line 123-125) -- all verified
- **`-o`/`--output` flag for file output**: Verified across cli/scan.rs (lines 173, 225, 252, 282, 322, 392), cli/fuzz.rs (lines 115, 264, 362), cli/http.rs (lines 95, 145, 171, 203)
- **Scope validation via `ensure_scope()` or `ensure_scope_url()`**: Verified at `commands/handlers/mod.rs:89-95`
- **`CommandContext`**: Defined at `commands/handlers/mod.rs:63-68` with fields `config`, `scope`, `json`, `config_path`
- **`handle_command`**: Exhaustive match at `commands/handlers/mod.rs:130-194` (no wildcard arm)
- **`handle_command` is exhaustive**: Confirmed -- comment at line 133: `// Keep this match exhaustive: no wildcard arm.`
- **Handler pattern `handle_fuzz`**: Verified at `commands/handlers/fuzz.rs:4-10` -- calls `ctx.ensure_scope_url(&args.url)?;`
- **Handler pattern `handle_config`**: Verified at `commands/handlers/config.rs:6-36` -- uses `load_config(config_path).map_err(|e| anyhow::anyhow!("Configuration validation failed: {}", e))?;`
- **`enforce_operation_policy()` method**: Verified at `commands/handlers/mod.rs:101-127` (doc says 101-124, actual is 101-127)
- **Workflow steps 1-6**: All steps verified against `main.rs` and `commands/handlers/mod.rs`
- **sbom.rs `unwrap()` replaced with `ok_or_else()`**: Verified at `commands/handlers/sbom.rs:18,22,26` -- no `unwrap()` calls remain
- **config.rs replaced `std::process::exit(1)` with error returns**: Verified -- no `std::process::exit` calls found in any handler files
- **http.rs `-o` short form for load and graphql**: Verified at `cli/http.rs:95` (LoadArgs) and `cli/http.rs:171` (GraphQlArgs)
- **handlers/mod.rs:197-206 `handle_no_command`**: Verified at `commands/handlers/mod.rs:197-206` -- launches TUI in interactive terminal, prints guidance otherwise
- **handlers/cluster.rs:348 `unwrap_or_else(|_| 22)`**: Verified at `commands/handlers/cluster.rs:349` -- `.parse().unwrap_or_else(|_| 22)`
- **handlers/auth_test.rs:10 scope validation**: Verified at `commands/handlers/auth_test.rs:10` -- `ctx.ensure_scope_url(&args.target)?;`
- **cli/scan.rs `-o` short flag**: Verified at `cli/scan.rs:173` (PortScanArgs), `cli/scan.rs:225` (EndpointScanArgs), `cli/scan.rs:252` (FingerprintArgs), `cli/scan.rs:282` (NseArgs), `cli/scan.rs:322` (ResumeArgs)
- **cli/fuzz.rs `-o` short flag for WafStressArgs**: Verified at `cli/fuzz.rs:264`
- **cli/http.rs `-o` short flag for ReconArgs**: Verified at `cli/http.rs:145`
- **cli/cluster.rs removed `-o` flag**: Verified -- `ClusterArgs` at `cli/cluster.rs:11-23` has only `verbose` and `quiet` flags, no `output`
- **CLI modules list**: `mod.rs`, `scan.rs`, `fuzz.rs`, `http.rs`, `packet.rs`, `stress.rs`, `agent.rs`, `ai_analyze.rs` -- all verified at `cli/mod.rs:4-41`

## Discrepancies

- **`enforce_operation_policy()` line range**: Documented as `commands/handlers/mod.rs:101-124`, actual function body spans lines 101-127 (closing brace at 127). Minor discrepancy.
- **`handle_fuzz` signature**: Document shows `args: FuzzArgs` but actual signature is `args: crate::cli::FuzzArgs` with `mut` binding (`commands/handlers/fuzz.rs:4`). The `mut` is significant because the handler modifies `args.json`.
- **`handle_config` simplified**: Document shows a simplified version of `handle_config` with just `load_config(config_path).map_err(...)` but the actual implementation at `commands/handlers/config.rs:6-36` has a `match` over `ConfigCommand::Validate` and `ConfigCommand::Show` sub-commands. The doc pattern is illustrative but not a literal match of the code.

## Bugs Found

- No bugs found in the architecture document.

## Improvement Opportunities

- **[Item]: Document the exhaustive match pattern detail**: The `handle_command` function at `commands/handlers/mod.rs:130-194` has 37 match arms including feature-gated ones. The doc should note that feature-gated variants are conditionally compiled, so the actual number of match arms varies by feature set. (priority: low)
- **[Item]: Document `CommandContext` fields more completely**: The doc mentions `SlapperConfig`, `Scope`, and "output preferences" but `CommandContext` at `commands/handlers/mod.rs:63-68` has `config`, `scope`, `json` (bool), and `config_path` (Option<String>). The `json` field is the non-interactive mode flag used by `enforce_operation_policy`. (priority: medium)
- **[Item]: Document `handle_no_command` TUI launch logic**: The function at `commands/handlers/mod.rs:197-206` checks `is_terminal()` before launching TUI. This is important behavior for CI/scripting contexts. (priority: low)
- **[Item]: Document `CodeggMcpArgs` struct**: The `CodeggMcpArgs` at `cli/misc.rs:314-334` is a separate struct from `McpServeArgs` with different defaults (stdio=true, profile=coding-agent). The dispatch at `commands/handlers/mod.rs:176-184` converts it to `McpServeArgs` before calling `handle_mcp_serve`. (priority: low)
- **[Item]: Document feature-gated CLI modules**: The `agent.rs` and `ai_analyze.rs` modules are conditionally compiled (`#[cfg(feature = "rest-api")]` and `#[cfg(feature = "ai-integration")]` respectively), which the doc mentions but doesn't show the `#[cfg]` attributes on `pub mod` declarations. (priority: low)

## Stale Items

- **[Bug Fixes and Consistency (2026-05-22)] section**: Lines 75-97 document specific bug fixes with line numbers. These are now historical. The line numbers may drift over time as code changes. Consider converting to a changelog entry or adding a "last verified" date. (priority: medium)
- **[CLI Consistency Guidelines table]**: The table at lines 92-97 documents naming conventions. These are guidelines, not code references, so they remain valid but could be referenced from a coding standards document instead. (priority: low)
