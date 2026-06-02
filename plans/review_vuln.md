# Vuln Module Architecture Review

**Document:** architecture/vuln.md
**Reviewed:** 2026-06-02
**Accuracy:** High
**Lines Reviewed:** 37

## Verified Claims
- [VulnAssessment]: Verified at `crates/slapper/src/vuln/mod.rs:37-40` - documented correctly as minimal placeholder with mode, results, assessed_at
- [CvssScore]: Verified at `crates/slapper/src/vuln/cvss.rs:15`
- [ExploitInfo]: Verified at `crates/slapper/src/vuln/exploit.rs:5`
- [AssetCriticality]: Verified at `crates/slapper/src/vuln/asset.rs:4`
- [PrioritizedFinding]: Verified at `crates/slapper/src/vuln/prioritizer.rs:57`
- [PriorityLevel]: Verified at `crates/slapper/src/vuln/prioritizer.rs:16` (P0, P1, P2, P3)
- [RiskScore]: Verified at `crates/slapper/src/vuln/prioritizer.rs:7`
- [TriageResult]: Verified at `crates/slapper/src/vuln/triage.rs:5`
- [TriageStatus enum]: Verified at `crates/slapper/src/vuln/triage.rs:12-19` (New, TruePositive, FalsePositive, NeedsReview, Duplicate)
- [Remediation]: Verified at `crates/slapper/src/vuln/remediation.rs:5`
- [RemediationPriority enum]: Verified at `crates/slapper/src/vuln/remediation.rs:15-21` (Critical, High, Medium, Low)
- [All 6 sub-module files exist]: Verified (cvss.rs, exploit.rs, asset.rs, prioritizer.rs, triage.rs, remediation.rs)

## Discrepancies
- None significant.

## Bugs Found
- [VulnAssessment is a stub that can't hold structured findings]: The `VulnAssessment` struct at `mod.rs:37-40` only has `mode: String`, `results: Vec<String>`, and `assessed_at: DateTime`. It cannot store actual structured findings. Any pipeline integration expecting structured vulnerability data will fail (priority: high)

## Improvement Opportunities
- [ExploitInfo::assess() uses year-based heuristics]: `exploit.rs:16-17` determines exploit availability based on whether the CVE ID contains "2021" or "2022". This is a heuristic that will become increasingly inaccurate as time passes. Real exploit intelligence requires external data sources (priority: medium)
- [Triage uses simple keyword matching]: `triage.rs:43-55` uses simple keyword arrays for duplicate/false positive detection. This will have high false positive/negative rates. Consider using ML or more sophisticated matching (priority: medium)
- [Remediation steps are generic templates]: `remediation.rs:25-78` returns hardcoded remediation steps based only on severity. Real remediation guidance should be tailored to the specific vulnerability/CVE (priority: medium)

## Stale Items
- [RemediationPriority comment about implementing Ord]: The document says RemediationPriority "implements Ord for sorting" - this is correct as it's derived at line 15. This is accurate.

## Code Interrogation Findings
- [CVSS calculation may have bugs]: The CVSS 3.1 implementation in `cvss.rs` has a custom `min!` macro and complex calculations. The clippy warning at line 53 `#![allow(clippy::too_many_arguments)]` suggests the function signature is known to be problematic. Consider validating against CVSS 3.1 test vectors.
- [No CVSS vector parsing validation]: `calculate_base_score_from_vector()` at line 147 silently ignores invalid vector components (line 219 `_ => {}`). Malformed vectors will produce incorrect scores without warning.
- [assess_asset() in asset.rs only matches asset_type strings exactly]: Line 61-67 does exact string matching ("database", "web_server", etc.). Typos or case differences will use the default scoring. Consider case-insensitive matching.