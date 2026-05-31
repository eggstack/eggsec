# CLI & Commands Architecture Review

**Document:** architecture/cli_commands.md
**Reviewed:** 2026-05-31
**Accuracy:** Medium

## Verified Claims

- **Cli struct with Commands enum and CommonHttpArgs**: Verified at `crates/slapper/src/cli/mod.rs:56-79`
- **Commands enum variants**: Verified - 36 variants exist, "35+" is approximately correct (`crates/slapper/src/cli/mod.rs:82-201`)
- **cli/mod.rs defines main Cli entry point, Commands enum, CommonHttpArgs**: Verified at `crates/slapper/src/cli/mod.rs`
- **scan.rs, fuzz.rs, http.rs, packet.rs, stress.rs, agent.rs, ai_analyze.rs**: All files verified to exist in `crates/slapper/src/cli/`
- **Global flags --json, --config, --scope**: Verified at `crates/slapper/src/cli/mod.rs:65-72`
- **Feature-gated commands (stress-testing, packet-inspection, nse, ai-integration, rest-api, grpc-api, sbom)**: Verified via `#[cfg(feature = "...")]` attributes in `crates/slapper/src/cli/mod.rs:29,32,123,132,135,146,168,193,198`
- **handle_command is exhaustive match (no wildcard arm)**: Verified at `crates/slapper/src/commands/handlers/mod.rs:133` with comment "Keep this match exhaustive: no wildcard arm."
- **CommandContext carries SlapperConfig, Scope, output preferences**: Verified at `crates/slapper/src/commands/handlers/mod.rs:63-68`
- **ensure_scope() and ensure_scope_url()**: Verified at `crates/slapper/src/commands/handlers/mod.rs:89-95`
- **handle_config uses proper error returns**: Verified at `crates/slapper/src/commands/handlers/config.rs:11-12` (uses `map_err()`)
- **handle_auth_test has scope validation**: Verified at `crates/slapper/src/commands/handlers/auth_test.rs:10` (`ctx.ensure_scope_url(&args.target)?`)
- **cli/cluster.rs has no -o flag**: Verified - ClusterArgs has no output field (`crates/slapper/src/cli/cluster.rs:11-23`)
- **WafStressArgs has -o short flag**: Verified at `crates/slapper/src/cli/fuzz.rs:264`
- **LoadArgs has -o short flag**: Verified at `crates/slapper/src/cli/http.rs:95`
- **GraphQlArgs has -o short flag**: Verified at `crates/slapper/src/cli/http.rs:171`
- **OAuthArgs has -o short flag**: Verified at `crates/slapper/src/cli/http.rs:203`
- **ReconArgs has -o short flag**: Verified at `crates/slapper/src/cli/http.rs:145`
- **PortScanArgs has -o short flag**: Verified at `crates/slapper/src/cli/scan.rs:173`
- **EndpointScanArgs has -o short flag**: Verified at `crates/slapper/src/cli/scan.rs:225`
- **FingerprintArgs has -o short flag**: Verified at `crates/slapper/src/cli/scan.rs:252`
- **NseArgs has -o short flag**: Verified at `crates/slapper/src/cli/scan.rs:282`
- **ResumeArgs has -o flag**: Verified at `crates/slapper/src/cli/scan.rs:322`
- **Scope validation via ensure_scope_url()**: Verified at `crates/slapper/src/commands/handlers/mod.rs:89-91`
- **sbom.rs uses ok_or_else() pattern**: Verified at `crates/slapper/src/commands/handlers/sbom.rs:18-28` (uses `ok_or_else(|| anyhow::anyhow!(...))`)

## Discrepancies

- **"agent.rs & ai_analyze.rs: Arguments for AI-driven features"**: This description is inaccurate. `agent.rs` is for agent orchestration (feature-gated on `rest-api`), not AI-driven features specifically. It defines `AgentArgs` with subcommands like `Run`, `Status`, `Reload` (`crates/slapper/src/cli/agent.rs:4-27`). The actual AI-driven feature args are in `ai_analyze.rs` (feature-gated on `ai-integration`). (priority: medium)
- **handlers/mod.rs:155-169 reference for handle_no_command**: The doc says "handlers/mod.rs:155-169: Replaced hardcoded command list in `handle_no_command` with guidance to use `slapper --help`". The actual `handle_no_command` function is at lines 197-205, not 155-169. (priority: medium)
- **handlers/cluster.rs:348 - unwrap_or(22) fix NOT applied**: The doc claims "Replaced `unwrap_or(22)` with `unwrap_or_else(|_| 22)` to avoid panic on invalid parsing". The actual code at line 349 still shows `.parse().unwrap_or(22)`, not `.parse().unwrap_or_else(|_| 22)`. The fix was NOT applied. (priority: high)
- **cli/fuzz.rs: "preserved `From<WafStressArgs>` implementation"**: This is correct but the doc implies the -o flag was added to WafStressArgs as a new change. The `From<WafStressArgs>` impl is at `crates/slapper/src/cli/fuzz.rs:270-319` and correctly includes the `output` field. Verified.
- **CLI Consistency Guidelines - Source IP naming**: Doc recommends `source_ip` / `source_port` (not `spoof_ip`). However, `EndpointScanArgs` still uses `spoof_ip` at `crates/slapper/src/cli/scan.rs:195`. This contradicts the stated guideline. (priority: low)

## Bugs Found

- **cluster.rs:349 unwrap_or(22) still present**: The bug fix claim says `unwrap_or(22)` was replaced with `unwrap_or_else(|_| 22)` but the code at `crates/slapper/src/commands/handlers/cluster.rs:349` still shows `.parse().unwrap_or(22)`. This is a potential panic on invalid parsing if the fix was intended. (priority: high)
- **handlers/mod.rs line reference wrong**: The doc references lines 155-169 for `handle_no_command` but the function is at lines 197-205. This makes it impossible to verify the claim by looking at the cited lines. (priority: medium)

## Improvement Opportunities

- **Document the exhaustive match pattern in detail**: The doc mentions the exhaustive match but doesn't show it as a pattern example with the comment. Consider adding the pattern with the compile-time safety guarantee explanation. (priority: low)
- **Document `enforce_operation_policy()`**: The `CommandContext` has an `enforce_operation_policy()` method (`crates/slapper/src/commands/handlers/mod.rs:101-127`) that checks scope, risk level, and non-interactive mode. This is not mentioned in the doc. (priority: medium)
- **Document all handler modules**: The doc only mentions scan.rs, fuzz.rs, and cluster.rs as examples. There are 20+ handler modules (auth_test, ci, config, doctor, load, network, notify, plan, recon, report, sbom, stress, vuln, etc.). A full list would be more useful. (priority: low)
- **Document proxy.rs, webhook.rs, fuzz_convert.rs**: The `commands/` directory has additional modules beyond handlers: `proxy.rs`, `webhook.rs`, `fuzz_convert.rs` (`crates/slapper/src/commands/mod.rs:1-4`). These are not mentioned. (priority: low)
- **Document the `handle_no_command` TUI fallback**: When no command is specified and running in a terminal, the CLI launches the TUI (`crates/slapper/src/commands/handlers/mod.rs:197-205`). This is a key UX behavior not documented. (priority: medium)
- **Document the `codegg-mcp` alias**: The CodeggMcp command has an alias `mcp-codegg` and `codegg-mcp` (`crates/slapper/src/cli/mod.rs:178-182`). The HELP_AFTER text also references `slapper codegg-mcp`. (priority: low)

## Stale Items

- **"handlers/mod.rs:155-169" line reference**: Incorrect. Should be 197-205. (Recommended action: update line numbers)
- **"handlers/cluster.rs:348: Replaced `unwrap_or(22)` with `unwrap_or_else(|_| 22)`"**: Fix not applied. Code still has `unwrap_or(22)` at line 349. (Recommended action: either apply the fix or remove the claim)
- **CLI Consistency Guidelines `source_ip` vs `spoof_ip`**: The guideline recommends `source_ip` but `EndpointScanArgs` still uses `spoof_ip`. (Recommended action: update EndpointScanArgs to use `source_ip` or note the inconsistency)
