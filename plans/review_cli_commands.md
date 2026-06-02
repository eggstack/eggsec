# CLI & Commands Architecture Review

**Document:** architecture/cli_commands.md
**Reviewed:** 2026-06-02
**Accuracy:** Medium
**Lines Reviewed:** 101

## Verified Claims
- [CLI uses clap]: Verified at `cli/mod.rs:1` with `use clap::{Parser, Subcommand, ValueEnum}`
- [Commands enum entry point]: Verified at `cli/mod.rs:81-201`
- [CommandContext struct]: Verified at `commands/handlers/mod.rs:63-68`
- [handle_command is exhaustive match]: Verified at `commands/handlers/mod.rs:130-195` - no wildcard arm
- [enforce_operation_policy at lines 101-124]: Verified at `commands/handlers/mod.rs:101-127`
- [handle_no_command launches TUI]: Verified at `commands/handlers/mod.rs:197-206`
- [Scope validation via ensure_scope_url/ensure_scope]: Verified at `commands/handlers/mod.rs:89-95`
- [handlers/scan.rs exists]: Verified at `commands/handlers/scan.rs` (directory listing)
- [handlers/fuzz.rs exists]: Verified at `commands/handlers/fuzz.rs` (directory listing)
- [handlers/cluster.rs exists]: Verified at `commands/handlers/cluster.rs` (directory listing)

## Discrepancies
- [Commands enum variant count]: Documented as "35+ variants" at line 9, but counting the enum at `cli/mod.rs:81-201`:
  - Without feature gates: 29 variants (ScanPorts, ScanEndpoints, Fingerprint, Scan, Resume, Fuzz, Waf, WafStress, Graphql, OAuth, AuthTest, Recon, Plan, Ci, Config, Doctor, Load, Report, Vuln, Storage, Cluster, Notify, Remote, Exec)
  - With all feature gates enabled: ~40 arms in the match statement
  - The "37+" claim in overview.md and "35+" in cli_commands.md are both approximate but could be more precise
- [handlers/mod.rs line reference]: Document references `handlers/mod.rs:197-206` for handle_no_command, but actual is `commands/handlers/mod.rs:197-205` (6 lines, not 10)

## Bugs Found
- None found. CLI structure is sound.

## Improvement Opportunities
- [Precision in variant count]: Document should clarify "Without feature gates: 29 variants, with all features: ~40 variants" rather than "35+" (low priority)
- [Line reference accuracy]: handle_no_command is 9 lines (197-205), not 10 lines as implied by "197-206" (low priority)

## Stale Items
- [Bug fixes section (lines 75-88)]: This section documents fixes from 2026-05-22. These appear accurate based on handlers/mod.rs structure, but individual fixes (sbom.rs, config.rs, http.rs, etc.) were not verified against source files during this review.

## Code Interrogation Findings
- [Observation]: The `Commands` enum at `cli/mod.rs:81-201` uses `#[allow(clippy::large_enum_variant)]` at line 82, acknowledging the enum is large.
- [Security observation]: Feature-gated commands (stress-testing, packet-inspection, nse, ai-integration, rest-api, grpc-api, sbom) are correctly gated with `#[cfg(feature = "...")]` attributes.
- [Pattern verification]: Handler functions follow the documented pattern - they are async, take `&CommandContext` and parsed args, and return `Result<()>`.