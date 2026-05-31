# Vuln Architecture Review
**Document:** architecture/vuln.md
**Reviewed:** 2026-05-31
**Accuracy:** High
**Lines Reviewed:** 36

## Verified Claims
- `VulnAssessment` struct: Verified at `crates/slapper/src/vuln/mod.rs:37` with fields `mode`, `results`, `assessed_at`
- `CvssScore` struct: Verified at `crates/slapper/src/vuln/cvss.rs:15` with fields `base_score`, `temporal_score`, `environmental_score`, `vector`
- `ExploitInfo` struct: Verified at `crates/slapper/src/vuln/exploit.rs:5` with fields `cve_id`, `has_public_exploit`, `exploit_db_id`, `metasploit_module`, `in_cisa_kev`, `is_actively_exploited`, `exploit_score`
- `AssetCriticality` struct: Verified at `crates/slapper/src/vuln/asset.rs:4` with fields `asset_id`, `technology_score`, `environment_score`, `data_sensitivity`, `user_base`, `overall_score`
- `PrioritizedFinding` struct: Verified at `crates/slapper/src/vuln/prioritizer.rs:57` with fields `finding_id`, `title`, `severity`, `risk_score`, `exploit_info`, `asset_criticality`, `priority_rank`
- `PriorityLevel` enum: Verified at `crates/slapper/src/vuln/prioritizer.rs:16` with variants `P0`, `P1`, `P2`, `P3`
- `RiskScore` struct: Verified at `crates/slapper/src/vuln/prioritizer.rs:7` with fields `cvss_score`, `exploitability_score`, `asset_criticality`, `combined_score`, `priority_level`
- `TriageResult` struct: Verified at `crates/slapper/src/vuln/triage.rs:5` with fields `finding_id`, `triage_status`, `confidence`, `reason`
- `TriageStatus` enum: Verified at `crates/slapper/src/vuln/triage.rs:13` with variants `New`, `TruePositive`, `FalsePositive`, `NeedsReview`, `Duplicate`
- `Remediation` struct: Verified at `crates/slapper/src/vuln/remediation.rs:5` with fields `finding_id`, `title`, `severity`, `effort_hours`, `steps`, `references`, `priority`
- All 6 sub-modules present: `mod.rs`, `cvss.rs`, `exploit.rs`, `asset.rs`, `prioritizer.rs`, `triage.rs`, `remediation.rs` - verified
- CVSS 3.1 score calculation: Verified at `cvss.rs:54` (`calculate_base()`) with full vector parsing at `cvss.rs:147`
- Exploitability assessment: Verified at `exploit.rs:16` (`assess()`)
- Asset criticality scoring: Verified at `asset.rs:58` (`assess_asset()`)
- Combined risk prioritization: Verified at `prioritizer.rs:24` (`calculate()`)
- Finding triage: Verified at `triage.rs:36` (`triage_finding()`)
- Remediation guidance: Verified at `remediation.rs:24` (`for_finding()`)

## Discrepancies
- `TriageStatus` has a `New` variant not mentioned in the document description "Triage status enum". This is a minor omission.

## Bugs Found
- None

## Improvement Opportunities
- Document the `RemediationPriority` enum (`remediation.rs:16`) with variants `Critical`, `High`, `Medium`, `Low`
- The `VulnAssessment` struct (`mod.rs:37`) is very minimal (just `mode: String`, `results: Vec<String>`, `assessed_at`) and is not used by any sub-module. It appears to be a placeholder. Consider expanding or removing it.
- The `ExploitInfo::assess()` method (`exploit.rs:17`) uses heuristic string matching on CVE IDs (`contains("2021")`) rather than real exploit databases. Document this limitation.

## Stale Items
- None
