---
name: eggsec-vuln-management
description: "Vulnerability scoring, triage, and remediation - use when working with CVSS scoring, exploitability, asset criticality, prioritization, triage, or remediation guidance."
---

# Eggsec Vulnerability Management Skill

CVSS scoring, exploitability assessment, and risk-based prioritization.

## Module (`crates/eggsec/src/vuln/`)

| File | Contents |
|------|----------|
| `mod.rs` | Re-exports: `AssetCriticality`, `CvssScore`, `ExploitInfo`, `PrioritizedFinding`, `PriorityLevel`, `RiskScore`, `Remediation` |
| `cvss.rs` | CVSS 3.1 vector parsing and score calculation |
| `exploit.rs` | `ExploitInfo` — exploitability assessment |
| `asset.rs` | `AssetCriticality` — asset-side weighting |
| `prioritizer.rs` | `PrioritizedFinding`, `PriorityLevel`, `RiskScore` — combined ranking |
| `triage.rs` | Triage states and transitions |
| `remediation.rs` | `Remediation` guidance records |

## Operation & Gating

- Operation id `vuln` (`config/policy_catalog.rs`): `StandardAssessment` / `SafeActive`, requires feature `vuln-management` and explicit scope.
- CLI: `eggsec vuln <score|exploitability|prioritize|triage|remediate>` (`cli/vuln.rs`: `VulnArgs`/`VulnCommand`; handler in `commands/handlers/vuln.rs`). The `Vuln` CLI variant itself is unconditional; the operation still enforces the feature gate at dispatch.
- TUI tab `Vuln` (`tabs/spec.rs`, feature `vuln-management`, operation `vuln`).

## Common Tasks

### Adding a Scoring Signal
1. Put raw signal logic in its home module (`cvss.rs`, `exploit.rs`, `asset.rs`) — no policy checks there.
2. Combine signals only in `prioritizer.rs` so ranking stays single-owned.
3. Keep `PriorityLevel` ordering total and documented; add tests for boundary scores.

### Bug Fixes
- Never `unwrap()` on vector parsing — return typed errors for malformed CVSS vectors.
- Prioritizer must be deterministic: same inputs, same order. No wall-clock or hash-map iteration order in ranking.

## Resources
- `architecture/vuln.md` - vulnerability management deep-dive
- `architecture/overview.md` (Module Index) - vuln row
- `architecture/findings.md` - canonical finding schema the prioritizer consumes
