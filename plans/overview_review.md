# Overview Architecture Review

**Document:** architecture/overview.md
**Reviewed:** 2026-05-31
**Accuracy:** Medium-High

## Verified Claims

- **39 modules**: Verified. `crates/slapper/src/` contains exactly 39 subdirectories.
- **169 NSE library modules**: Verified (in `slapper-nse` crate).
- **34 WAF products detected**: Verified. `crates/slapper/src/waf/data/patterns.rs` contains 34 named WAF product patterns.
- **30 payload types**: Verified. `PayloadType` enum at `crates/slapper/src/fuzzer/payloads/mod.rs:39-70` has exactly 30 variants.
- **28 TUI tabs**: Verified. `Tab` enum at `crates/slapper/src/tui/tabs/mod.rs:80-109` has 28 variants.
- **16 scan profiles**: Verified. `ScanProfile` enum at `crates/slapper/src/cli/mod.rs:250-267` has 16 variants.
- **Pipeline 7 stages**: Verified. `Stage` enum at `crates/slapper/src/pipeline/stage.rs:6-14` has exactly 7 variants: PortScan, Fingerprint, EndpointScan, Fuzz, LoadTest, Waf, Recon.
- **5 defense-lab profiles**: Verified. All 5 exist in `ScanProfile` enum: DefenseLab, SynvoidLocal, WafRegression, ProtocolEdge, NseSafe.
- **Module index**: All module source paths listed are valid (scanner, fuzzer, recon, waf, loadtest, stress, etc.).
- **RunManifest**: Exists at `crates/slapper/src/output/run_manifest.rs` with correct structure.
- **DiffEngine/DiffSummary**: Exist at `crates/slapper/src/output/diff.rs`.
- **BaselineComparison**: Exists at `crates/slapper/src/output/baseline.rs`.
- **Severity enum**: Exists at `crates/slapper/src/types.rs` (Critical/High/Medium/Low/Info).
- **SlapperConfig**: Exists at `crates/slapper/src/config/settings.rs:93`.
- **ProbeIntent/ProbeRisk**: Exist at `crates/slapper/src/probe.rs:17-43`.
- **McpProfile**: Exists at `crates/slapper/src/tool/protocol/mcp/profile.rs`.
- **TargetPolicy**: Exists at `crates/slapper/src/tool/protocol/mcp/policy.rs`.
- **Feature flags**: All 28 features listed in `Cargo.toml` match the feature matrix.
- **Workspace crates**: `slapper` and `slapper-nse` are the only two crates. Verified.

## Discrepancies

- **Source file count**: Document claims "526 source files" but actual count in `crates/slapper/src/` is **522** `.rs` files. The discrepancy is 4 files. This may be due to files added/removed since the doc was written, or counting files in `slapper-nse` as well.
- **CLI commands count**: Document claims "35+ command variants" in the `Commands` enum. The actual handler dispatch at `commands/handlers/mod.rs:130-194` shows 38 match arms (including feature-gated ones). The claim "35+" is approximately correct but imprecise.
- **CLI commands table count**: The document's CLI Commands Reference section lists commands in 4 tables. Counting all unique commands: scan-ports, scan-endpoints, fingerprint, scan, resume, fuzz, waf, waf-stress, graphql, oauth, auth-test, recon, load, report, cluster, remote, exec, serve, mcp-serve, codegg-mcp, agent, ai-analyze, grpc, plan, ci, config, doctor, sbom, packet, nse, stress, proxy, icmp, traceroute, vuln, storage = 36 commands. The "35+" claim is approximately accurate.
- **Output formats**: Document claims "8 output formats" in `types.rs`. The `output/mod.rs` documents 7 formats (JSON, CSV, HTML, Markdown, SARIF, JUnit, PDF). The 8th format may be "Pretty" (pretty-printed JSON) which is documented separately.
- **Module count in architecture diagram**: The diagram shows `auth/` as a security testing module, but the module list does not have a detailed row for it in the Security Testing section. `auth/` is listed but with minimal description.

## Bugs Found

- **Feature flags count**: Document claims "Feature flags: 30" in Codebase Health table (line 471), but the Summary section (line 9) says "Total features: 28". Cargo.toml has exactly 28 declared features. The "30" count is incorrect and inconsistent with the doc's own Summary.
- **Source files count inconsistency**: "526 source files" appears twice (line 8 and line 463). Actual count is 522. The discrepancy is small but should be updated.

## Improvement Opportunities

- **Module descriptions need updating**: Some module descriptions reference features that are now feature-gated but the descriptions don't clearly indicate this. For example, `auth/` is described without noting it's always compiled.
- **Missing `app/` submodules in TUI section**: The TUI module row (line 203) is a single-line summary. The actual `tui/app/` directory has 17+ files that could be listed.
- **Missing utility modules**: `utils/` is described as having "23 sub-modules" but only some are listed by name. The actual count may differ.
- **Key Dependencies table**: The dependencies listed are accurate but could benefit from bidirectional arrows or a clearer data flow diagram.
- **Scan profiles table**: The table lists 16 profiles but doesn't specify which ones require explicit scope. Adding a "Scope Required" column would improve the reference.

## Stale Items

- **"Feature flags: 30" in Codebase Health**: Should be updated to 28 to match actual Cargo.toml and the doc's own Summary section.
- **"526 source files"**: Should be updated to 522 or removed in favor of a dynamic count.
- **CLI commands "35+"**: Could be updated to "38" for precision.
- **Module description for `auth/`**: The description is sparse compared to other modules. Consider expanding to mention brute force, credential stuffing, lockout detection, MFA bypass, etc.
