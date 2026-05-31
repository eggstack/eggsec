# Overview Architecture Review

**Document:** architecture/overview.md
**Reviewed:** 2026-05-31
**Accuracy:** Medium
**Lines Reviewed:** 406

## Verified Claims

- [Line 3]: "Slapper is a high-performance, async-first security testing toolkit built in Rust" - Verified at lib.rs:1
- [Line 9]: "Commands enum (35+ variants)" - Verified 37 total variants at cli/mod.rs:83-201 (35+ is technically correct but imprecise)
- [Line 29]: "Command Dispatch Layer (commands/handlers/)" - Verified at commands/handlers/ directory
- [Line 59]: `cli/` module path - Verified at crates/slapper/src/cli/
- [Line 61]: `tool/protocol/rest` - Verified at tool/protocol/rest.rs
- [Line 62]: `tool/protocol/mcp`, `tool/protocol/openai` - Verified at tool/protocol/mcp/ and tool/protocol/openai/
- [Line 67]: `commands/` module path - Verified at crates/slapper/src/commands/
- [Line 68]: `commands/handlers/` module path - Verified at crates/slapper/src/commands/handlers/
- [Line 76]: `config/` module path - Verified at crates/slapper/src/config/
- [Line 77]: `distributed/` module path - Verified at crates/slapper/src/distributed/
- [Line 78]: `output/` module path - Verified at crates/slapper/src/output/
- [Line 79]: `storage/` module path - Verified at crates/slapper/src/storage/
- [Line 80]: `workflow/` module path - Verified at crates/slapper/src/workflow/
- [Line 85]: `ai/` module path - Verified at crates/slapper/src/ai/
- [Line 87]: `browser/` module path - Verified (feature-gated, lib.rs:73)
- [Line 88]: `integrations/` module path - Verified (feature-gated, lib.rs:99)
- [Line 97]: `recon/` module path - Verified at crates/slapper/src/recon/
- [Line 98]: `scanner/` module path - Verified at crates/slapper/src/scanner/
- [Line 99]: `probe.rs` module path - Verified at crates/slapper/src/probe.rs
- [Line 104]: "30 payload types" - Verified 30 PayloadType variants at fuzzer/payloads/mod.rs:38-70
- [Line 105]: "34 products" for WAF - Verified SUPPORTED_WAF_COUNT=34 at constants.rs:24 with test verification
- [Line 107]: `auth/` module path - Verified at crates/slapper/src/auth/
- [Line 108]: `hunt/` module path - Verified (feature-gated, lib.rs:94)
- [Line 114]: `loadtest/` module path - Verified at crates/slapper/src/loadtest/
- [Line 115]: `stress/` module path - Verified (feature-gated, lib.rs:118)
- [Line 116]: `packet/` module path - Verified (feature-gated, lib.rs:157)
- [Line 121]: `pipeline/` module path - Verified at crates/slapper/src/pipeline/
- [Line 122]: `tool/` module path - Verified at crates/slapper/src/tool/
- [Line 123]: `agent/` module path - Verified (feature-gated, lib.rs:148)
- [Line 128]: "8 report formats" - Verified 8 OutputFormat variants at types.rs:308-320
- [Line 136]: `compliance/` module path - Verified (feature-gated, lib.rs:77)
- [Line 137]: `vuln/` module path - Verified (feature-gated, lib.rs:128)
- [Line 138]: `supply_chain/` module path - Verified (feature-gated, lib.rs:120)
- [Line 139]: `container/` module path - Verified (feature-gated, lib.rs:84)
- [Line 145]: "169 NSE libraries" - Verified 168 library .rs files + mod.rs = 169 entries in crates/slapper-nse/src/libraries/
- [Line 153]: `types.rs` path for Severity, SensitiveString, OutputFormat - Verified
- [Line 154]: `error/` module path - Verified at crates/slapper/src/error/
- [Line 155]: `findings/` module path - Verified at crates/slapper/src/findings/
- [Line 156]: `diff/` module path - Verified at crates/slapper/src/diff/
- [Line 157]: `notify/` module path - Verified at crates/slapper/src/notify/
- [Line 159]: `constants.rs` path - Verified at crates/slapper/src/constants.rs
- [Line 160]: `macros.rs` path - Verified at crates/slapper/src/macros.rs
- [Line 261]: `SlapperConfig` at `config/settings.rs` - Verified at settings.rs:92
- [Line 262]: `Severity` at `types.rs` - Verified at types.rs:16
- [Line 263]: `SensitiveString` at `types.rs` - Verified at types.rs:128
- [Line 264]: `OutputFormat` at `types.rs` - Verified at types.rs:310
- [Line 265]: `SlapperError` at `error/mod.rs` - Verified at error/mod.rs:44
- [Line 266]: `TargetScope` at `config/scope.rs` - Verified at scope.rs:267
- [Line 273]: `SpoofConfig` at `scanner/spoof.rs` - Verified at spoof.rs:30
- [Line 274]: `TimingPreset` at `scanner/timing.rs` - Verified at timing.rs
- [Line 279]: `FuzzEngine` at `fuzzer/engine/` - Verified at fuzzer/engine/core.rs:97
- [Line 280]: `PayloadType` at `fuzzer/payloads/mod.rs` - Verified at fuzzer/payloads/mod.rs:38
- [Line 286]: `WafDetector` at `waf/detector/` - Verified at waf/detector/mod.rs:21
- [Line 287]: `BypassEngine` at `waf/bypass/` - Verified at waf/bypass/mod.rs:74
- [Line 293]: `ToolRegistry` at `tool/registry.rs` - Verified at tool/registry.rs:23
- [Line 294]: `SecurityTool` at `tool/traits.rs` - Verified at tool/traits.rs:144
- [Line 295]: `McpProfile` at `tool/protocol/mcp/profile.rs` - Verified at profile.rs:5
- [Line 296]: `McpProfilePolicy` at `tool/protocol/mcp/policy.rs` - Verified at policy.rs:64
- [Line 297]: `AiClient` at `ai/client.rs` - Verified at client.rs:55
- [Line 298]: `AiPlanner` at `ai/planner.rs` - Verified at planner.rs:47
- [Line 304]: `Stage` at `pipeline/stage.rs` - Verified at stage.rs:6
- [Line 305]: `PipelineContext` at `pipeline/context.rs` - Verified at context.rs:9
- [Lines 174-198]: All 24 feature flags verified against Cargo.toml features section (lines 213-296)
- [Line 388]: "1324 base, 1469+ with full features" test count - UNVERIFIED (requires running cargo test)
- [Line 393]: "Clippy warnings ~33" - UNVERIFIED (requires running cargo clippy)

## Discrepancies

- [Line 271]: Documented `ScanResults` at `scanner/mod.rs` - **Actual**: `ScanResults` struct is at `waf/types.rs:188`. The scanner module exports `PortScanResults` (scanner/ports.rs), `EndpointScanResults` (scanner/endpoints.rs), and `FingerprintResults` (scanner/fingerprint.rs). The type `ScanResults` belongs to the WAF module, not the scanner module.

- [Line 272]: Documented `FingerprintResult` (singular) at `scanner/fingerprint.rs` - **Actual**: The struct is named `FingerprintResults` (plural) at scanner/fingerprint.rs:83. Minor naming discrepancy.

- [Line 281]: Documented `FuzzResult` at `fuzzer/mod.rs` - **Actual**: `FuzzResult` is defined at `fuzzer/engine/types.rs:10`, not `fuzzer/mod.rs`. The fuzzer/mod.rs re-exports it but does not define it.

- [Line 288]: Documented `WafProfile` at `waf/types.rs` - **Actual**: `WafProfile` is defined at `waf/bypass/profiles.rs:9`, not `waf/types.rs`. The waf/types.rs file contains `OwaspCategory`, `Severity`, and `WafSignature` types instead.

- [Line 303]: Documented `Pipeline` at `pipeline/mod.rs` - **Actual**: `Pipeline` struct is defined at `pipeline/executor.rs:38`, not `pipeline/mod.rs`. The mod.rs re-exports it.

- [Line 9]: "Commands enum (35+ variants)" - **Actual**: 37 variants. While "35+" is technically correct, the actual count is 37 and should be stated precisely.

## Bugs Found

- [lib.rs:16]: Outdated docstring claims "fuzzer - Security fuzzing engine with 22 payload types" - **Actual**: 30 payload types (verified at fuzzer/payloads/mod.rs:38-70). The lib.rs docstring is stale.

- [lib.rs:17]: Outdated docstring claims "waf - WAF detection (26 products) and bypass techniques" - **Actual**: 34 WAF products (verified via SUPPORTED_WAF_COUNT=34 at constants.rs:24 with test assertion at constants.rs:31-37). The lib.rs docstring is stale.

## Improvement Opportunities

- [High] Fix `lib.rs` docstring: Update "22 payload types" to "30" and "26 products" to "34" to match actual implementation. These are user-facing doc comments.

- [Medium] Fix type location claims in the Key Types table: `ScanResults` should reference `waf/types.rs`, `FingerprintResult` should be `FingerprintResults`, `FuzzResult` should reference `fuzzer/engine/types.rs`, `WafProfile` should reference `waf/bypass/profiles.rs`, and `Pipeline` should reference `pipeline/executor.rs`.

- [Low] The "Commands enum (35+ variants)" count at line 9 should be updated to "37 variants" for precision.

- [Low] Consider adding a note about feature-gated modules in the module dependency table (e.g., `browser/`, `ai/`, `agent/` are feature-gated).

## Stale Items

- [lib.rs:16-17]: Module descriptions in lib.rs are stale and should be updated to match the current implementation (30 payload types, 34 WAF products).

- [Line 388]: Test count "1324 base, 1469+ with full features" should be verified and updated periodically as the codebase evolves.
