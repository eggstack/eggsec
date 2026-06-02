# Probe Classification Architecture Review

**Document:** architecture/probe.md
**Reviewed:** 2026-06-02
**Accuracy:** High
**Lines Reviewed:** 45

## Verified Claims
- [ProbeIntent defined in crates/slapper/src/probe.rs]: Verified
- [ProbeIntent has 10 variants (Discovery, Fingerprint, ServiceValidation, WafEvaluation, EvasionResistance, LoadBearing, Stress, MalformedProtocol, Regression, Compatibility)]: Verified at `crates/slapper/src/probe.rs:17-28`
- [ProbeRisk has 6 variants (Passive, SafeActive, Intrusive, Credentialed, Stress, ExploitAdjacent)]: Verified at `crates/slapper/src/probe.rs:34-43`
- [ProbeMetadata struct with id, name, intent, risk, requires_explicit_scope, requires_budget, compatibility_source]: Verified at `crates/slapper/src/probe.rs:45-55`
- [All enums serialize to kebab-case JSON]: Verified by tests at `crates/slapper/src/probe.rs:62-123`
- [Used across scanner, NSE, WAF, loadtest, and defense-lab profiles]: UNVERIFIED - document claim about usage scope cannot be directly verified from source

## Discrepancies
- None

## Bugs Found
- None

## Improvement Opportunities
- [Low]: The document mentions ProbeRisk has 6 variants but the table shows all 6 correctly listed - no issue here
- [Low]: Could document that ProbeIntent::ServiceValidation serializes as "service-validation" (verified by test at probe.rs:94)

## Stale Items
- None

## Code Interrogation Findings
- [Info]: Both enums use `#[serde(rename_all = "kebab-case")]` attribute for serialization
- [Info]: Tests confirm exact serialization format for all variants (probe.rs:89-123)
- [Info]: No deserialization tests shown but standard serde kebab-case should work