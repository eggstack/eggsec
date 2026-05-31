# Defense Lab Architecture Review
**Document:** architecture/defense_lab.md
**Reviewed:** 2026-05-31
**Accuracy:** High
**Lines Reviewed:** 125

## Verified Claims
- ScanProfile enum with 5 defense-lab profiles: Verified at `cli/mod.rs:262-266`
- defense-lab profile stages (PortScan → Fingerprint → EndpointScan → Waf → Fuzz): Verified at `pipeline/stage.rs:92-98`
- synvoid-local profile stages (PortScan → Fingerprint → EndpointScan → Waf): Verified at `pipeline/stage.rs:99-104`
- waf-regression profile stages (PortScan → Fingerprint → Waf): Verified at `pipeline/stage.rs:105`
- protocol-edge profile stages (PortScan → Fingerprint): Verified at `pipeline/stage.rs:106`
- nse-safe profile stages (PortScan → Fingerprint → EndpointScan): Verified at `pipeline/stage.rs:107`
- RunManifest output model with all fields: Verified at `output/run_manifest.rs:24-56`
- ProbeIntent enum (Discovery, Fingerprint, ServiceValidation, WafEvaluation, EvasionResistance, LoadBearing, Stress, MalformedProtocol, Regression, Compatibility): Verified at `probe.rs:17-28`
- ProbeRisk enum (Passive, SafeActive, Intrusive, Credentialed, Stress, ExploitAdjacent): Verified at `probe.rs:36-43`
- DiffSummary struct: Verified at `output/diff.rs:27-33`
- BaselineComparison struct: Verified at `output/baseline.rs:4-9`
- DiffEngine for comparison logic: Verified at `output/diff.rs:35-50`
- Safety model constraints (target scope, explicit scope, rate/concurrency budgets, feature gates): Verified at `cli/mod.rs` and `pipeline/stage.rs`
- Profile-based policy enforcement in MCP: Verified at `tool/protocol/mcp/policy.rs:158-263`

## Discrepancies
- Document states profiles are at "cli/mod.rs:262-266" - Verified correct at `cli/mod.rs:262-266`
- Document states pipeline stages are at "pipeline/stage.rs:92-107" - Verified correct at `pipeline/stage.rs:92-107`
- Document states RunManifest is defined in "output/run_manifest.rs" - Verified correct at `output/run_manifest.rs:24-56`
- Document states ProbeIntent/ProbeRisk are in "probe.rs" - Verified correct at `probe.rs:17-43`
- Document states DiffEngine is in "output/diff.rs" and BaselineComparison in "output/baseline.rs" - Verified correct

## Bugs Found
- No bugs found in the architecture documentation. All claims are accurate.

## Improvement Opportunities
- [Item]: Document the `profile_from_str()` function for runtime profile selection (`pipeline/stage.rs:138-158`) (priority: low)
- [Item]: Document the `ProbeMetadata` struct for probe-level metadata (`probe.rs:46-55`) (priority: low)
- [Item]: Add note about `RunManifest::from_report()` for pipeline integration (`output/run_manifest.rs:103-175`) (priority: low)
- [Item]: Document the `populate_findings_from_report()` method for finding extraction (`output/run_manifest.rs:179-194`) (priority: low)
- [Item]: Consider documenting the TUI integration for defense-lab profiles (`tui/tabs/scan.rs:66-70`) (priority: low)

## Stale Items
- [Item]: "Future Integration" section lists planned features - These should be updated as they are implemented (priority: medium)
- [Item]: Document references "Synvoid-like defensive systems" - Consider clarifying if this is a specific product or generic term (priority: low)
