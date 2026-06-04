# Vuln Module

## Purpose

Vulnerability management and prioritization using CVSS 3.1 scoring, exploitability assessment, asset criticality, and risk-based triage with remediation guidance.

## Key Types

| Type | Location | Description |
|------|----------|-------------|
| `VulnAssessment` | `vuln/mod.rs:37` | Rich assessment struct with fields: `mode`, `assessed_at`, `cvss_score`, `exploit_info`, `asset_criticality`, `prioritized_findings`, `triage_results`, `remediation_plans`, `summary` |
| `CvssScore` | `vuln/cvss.rs` | CVSS 3.1 score calculation with base, temporal, and environmental scores |
| `ExploitInfo` | `vuln/exploit.rs` | Exploit availability assessment (heuristic based on CVE year) |
| `AssetCriticality` | `vuln/asset.rs` | Asset criticality scoring |
| `PrioritizedFinding` | `vuln/prioritizer.rs` | Finding with combined risk score |
| `PriorityLevel` | `vuln/prioritizer.rs` | Priority classification |
| `RiskScore` | `vuln/prioritizer.rs` | Combined risk score calculation |
| `TriageResult` | `vuln/triage.rs` | Triage decision result |
| `TriageStatus` | `vuln/triage.rs:13` | Enum: New, TruePositive, FalsePositive, NeedsReview, Duplicate |
| `Remediation` | `vuln/remediation.rs` | Remediation guidance |
| `RemediationPriority` | `vuln/remediation.rs:16` | Enum: Critical, High, Medium, Low — derived from severity; implements `Ord` for sorting |

## Files

| File | Description |
|------|-------------|
| `mod.rs` | Module root: `VulnAssessment`, re-exports of sub-module types |
| `cvss.rs` | CVSS 3.1 base, temporal, and environmental score calculation and vector parsing |
| `exploit.rs` | Exploitability assessment (heuristic based on CVE year parsing) |
| `asset.rs` | Asset criticality scoring |
| `prioritizer.rs` | Combined risk prioritization engine |
| `triage.rs` | Finding triage workflow |
| `remediation.rs` | Remediation guidance generation |

## Implementation Status

Fully implemented with TUI worker and pipeline integration. All six sub-modules provide complete functionality for vulnerability scoring, prioritization, and remediation guidance. Integrated into the security assessment pipeline via `Stage::Vuln`.
