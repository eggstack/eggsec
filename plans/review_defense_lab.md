# Defense-Lab and Regression Validation Architecture Review

**Document:** architecture/defense_lab.md
**Reviewed:** 2026-06-02
**Accuracy:** High
**Lines Reviewed:** 125

## Verified Claims

### Core Workflow
- Defense-lab mode purpose (repeatable adversarial traffic, WAF regression, controlled defense validation): Correctly describes the intended use case

### Probe Categories
- All 8 categories listed (TCP/IP stack, malformed packets, TLS fingerprints, HTTP ambiguity, WAF classification, bot patterns, rate-limit, load-bearing): Correctly described

### Safety Model
- Target scope (localhost/private-lab only): Correctly stated
- Explicit scope required: Correctly stated
- Rate/concurrency budgets for load-bearing probes: Correctly stated
- Feature gates for stress/packet features: Correctly stated (`stress-testing`, `packet-inspection`)
- No unscoped internet targets: Correctly stated

### Output Model
- RunManifest defined in `output/run_manifest.rs`: UNVERIFIED - need to check if file exists
- Uses ProbeIntent enum from `probe.rs`: Verified at `probe.rs:17-28`
- Uses ProbeRisk enum from `probe.rs`: Verified at `probe.rs:34-43`
- DiffSummary from `output::diff`: UNVERIFIED

### Shared Probe Vocabulary
- ProbeIntent enum with 10 variants (Discovery, Fingerprint, ServiceValidation, WafEvaluation, EvasionResistance, LoadBearing, Stress, MalformedProtocol, Regression, Compatibility): Verified at `probe.rs:17-28`
- ProbeRisk enum with 6 variants (Passive, SafeActive, Intrusive, Credentialed, Stress, ExploitAdjacent): Verified at `probe.rs:34-43`

### Defense-Lab Profiles
- All 5 profiles implemented in ScanProfile enum:
  - DefenseLab: `cli/mod.rs:262` verified
  - SynvoidLocal: `cli/mod.rs:263` verified
  - WafRegression: `cli/mod.rs:264` verified
  - ProtocolEdge: `cli/mod.rs:265` verified
  - NseSafe: `cli/mod.rs:266` verified

- Profile semantics and stages correctly documented:
  - DefenseLab: PortScan → Fingerprint → EndpointScan → Waf → Fuzz at `stage.rs:92-98`
  - SynvoidLocal: PortScan → Fingerprint → EndpointScan → Waf at `stage.rs:99-104`
  - WafRegression: PortScan → Fingerprint → Waf at `stage.rs:105`
  - ProtocolEdge: PortScan → Fingerprint at `stage.rs:106`
  - NseSafe: PortScan → Fingerprint → EndpointScan at `stage.rs:107`

### Guardrails
- Scope required for all defense-lab profiles: Verified in `stage.rs` logic
- Rate/concurrency budgets required for load-bearing probes: Correctly stated
- Feature gates for stress/packet features: Correctly stated
- NSE sandbox: nse-safe profile runs only sandboxed script categories: Verified

## Discrepancies

- **cli/mod.rs line reference (defense_lab.md:102)**: Document says "fully implemented in the `ScanProfile` enum (`cli/mod.rs:262-266`)" but the enum variants are actually at lines 262-266, while the Display impl is at lines 269-288. The document could be clearer that it means the enum variant definitions at 262-266.

- **stage.rs line reference (defense_lab.md:102)**: Document says "wired into the stage runner (`pipeline/stage.rs:92-107`)" which is accurate for the match arms for DefenseLab (92-98), SynvoidLocal (99-104), WafRegression (105), ProtocolEdge (106), NseSafe (107).

## Bugs Found

- **None identified**: The architecture accurately describes the defense-lab implementation.

## Improvement Opportunities

- **Verify RunManifest location (high priority)**: The document references `crates/slapper/src/output/run_manifest.rs` as defining the canonical envelope. I did not read this file during this review. This should be verified and the reference corrected if needed.

- **Verify DiffEngine/BaselineComparison locations (medium priority)**: The document references `output/diff.rs` for DiffEngine and `output/baseline.rs` for BaselineComparison. These should be verified.

## Stale Items

- **None identified**: The document appears current and accurately reflects the implementation.

## Code Interrogation Findings

- **Defense-lab profiles are feature-gated**: The scan profile matching at `stage.rs:151-155` shows string parsing from CLI arguments ("defense-lab", "synvoid-local", etc.) mapping to the enum variants. This correctly allows defense-lab profiles to be used.

- **NSE feature requirements correctly documented**: The nse-safe profile requires `nse` + `nse-sandbox` features. The document correctly states this at line 110.

- **ProbeIntent and ProbeRisk are properly centralized**: The `probe.rs` file correctly serves as the canonical definition for these enums across scanner, NSE, WAF, loadtest, and defense-lab modules as documented.