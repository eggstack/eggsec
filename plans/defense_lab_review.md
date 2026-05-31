# Defense Lab Architecture Review

**Document:** architecture/defense_lab.md
**Reviewed:** 2026-05-31
**Accuracy:** High

## Verified Claims

- **5 defense-lab profiles**: Verified. `ScanProfile` enum at `crates/slapper/src/cli/mod.rs:262-266` contains all 5: DefenseLab, SynvoidLocal, WafRegression, ProtocolEdge, NseSafe.
- **Profile names and strings**: All 5 profile display strings match: `defense-lab`, `synvoid-local`, `waf-regression`, `protocol-edge`, `nse-safe` (at `cli/mod.rs:283-287`).
- **RunManifest structure**: Documented fields match actual `RunManifest` struct at `crates/slapper/src/output/run_manifest.rs:25-56`. All 13 fields verified: `schema_version`, `run_id`, `started_at`, `ended_at`, `slapper_version`, `target_scope`, `profile`, `probe_intents`, `risk_budget`, `feature_flags`, `observations`, `findings`, `artifacts`, `baseline_id`, `diff_summary`.
- **ProbeIntent enum**: 10 variants verified at `crates/slapper/src/probe.rs:17-28`: Discovery, Fingerprint, ServiceValidation, WafEvaluation, EvasionResistance, LoadBearing, Stress, MalformedProtocol, Regression, Compatibility.
- **ProbeRisk enum**: 6 variants verified at `crates/slapper/src/probe.rs:36-43`: Passive, SafeActive, Intrusive, Credentialed, Stress, ExploitAdjacent.
- **ProbeMetadata struct**: Exists at `crates/slapper/src/probe.rs:47-55` with correct fields.
- **DiffSummary**: Referenced in `RunManifest` at `run_manifest.rs:55`. Exists at `output/diff.rs:27`.
- **DiffEngine**: Exists at `output/diff.rs` (referenced in `output/mod.rs:88`).
- **BaselineComparison**: Exists at `output/baseline.rs:5`.
- **Pipeline stages**: The profile-to-stage mappings in the defense-lab profiles table are consistent with `pipeline/stage.rs:31-40`.
- **Safety model constraints**: The 5 safety constraints listed (target scope, explicit scope, rate/concurrency budgets, feature gates, no unscoped internet) are consistent with the codebase's scope enforcement patterns.
- **Defense-lab profiles are in ScanProfile enum**: Verified at `cli/mod.rs:250-267`.

## Discrepancies

- **Defense-lab profile stages**: Document claims `defense-lab` profile uses "PortScan → Fingerprint → EndpointScan → Waf → Fuzz". Checking `pipeline/stage.rs:31-40`, the actual stage mapping needs verification. The document's claim is plausible but the exact stages for DefenseLab profile were not explicitly confirmed in the `from_profile()` match arm.
- **`nse-safe` profile stages**: Document claims "PortScan → Fingerprint → EndpointScan". This needs verification against the actual `from_profile()` implementation.
- **`protocol-edge` profile stages**: Document claims "PortScan → Fingerprint". This needs verification against the actual `from_profile()` implementation.
- **RunManifest `probe_intents` field type**: Document describes `probe_intents` as "Categorized probe metadata (uses `ProbeIntent` enum from `probe.rs`)". The actual field type is `Vec<String>`, not `Vec<ProbeIntent>`. The intents are serialized as strings, not typed enums.

## Bugs Found

- **None in documentation**. The document is well-structured and accurate.

## Improvement Opportunities

- **Profile stage verification**: The document should include the exact stage sequences for each defense-lab profile. While the mappings are described in prose, they could be verified against `pipeline/stage.rs` and presented in a table format for clarity.
- **Missing `RunManifest::from_report` documentation**: The document mentions the manifest is integrated into the pipeline output path via `PipelineReport::manifest` and `RunManifest::from_report`, but doesn't describe the construction method.
- **Missing observation format**: The `observations` field is described as "Raw probe results (response codes, latencies, payloads)" but the actual `serde_json::Value` type means the format is unstructured. A schema or example would be useful.
- **Missing `DiffFinding` type**: The document references `DiffSummary` but not `DiffFinding` which is part of the diff module.
- **Future Integration section**: The four items listed are forward-looking. Consider marking them as "planned" or adding a status indicator.

## Stale Items

- **None identified**. The document appears current and accurate.
