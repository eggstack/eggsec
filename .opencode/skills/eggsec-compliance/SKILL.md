---
name: eggsec-compliance
description: "Compliance framework scanning and reporting - use when working with OWASP/PCI-DSS/HIPAA/SOC2 checks, compliance reports, or the compliance feature gate."
---

# Eggsec Compliance Skill

Framework-mapped compliance scanning and reporting for Eggsec.

## Module (`crates/eggsec/src/compliance/`)

| File | Contents |
|------|----------|
| `mod.rs` | `ComplianceReport`, `ComplianceFinding`, `ComplianceStatus` |
| `owasp.rs` | OWASP Top 10 mapping and checks |
| `pci.rs` | PCI DSS requirement checks |
| `hipaa.rs` | HIPAA safeguard checks |
| `soc2.rs` | SOC 2 trust-criteria checks |
| `report.rs` | Report rendering for all frameworks |

## Operation & Gating

- Operation id `compliance` (`config/policy_catalog.rs`): `StandardAssessment` / `SafeActive`, requires feature `compliance` and explicit scope (`TargetPolicyKind::ExplicitScopeRequired`).
- Exposed on all surfaces: manual (CLI/TUI), MCP, REST, agent, gRPC.
- TUI tab `Compliance` (`tabs/spec.rs`, feature `compliance`, operation `compliance`).

## Key Types

```rust
pub struct ComplianceReport {
    pub framework: String,
    pub target: String,
    pub overall_score: f32,
    pub total_requirements: usize,
    pub passed: usize,
    pub failed: usize,
    pub findings: Vec<ComplianceFinding>,
}

pub struct ComplianceFinding {
    pub requirement_id: String,   // e.g. "PCI-6.5", "OWASP-A03"
    pub description: String,
    pub severity: Severity,
    pub status: ComplianceStatus, // Pass / Fail / NotApplicable / NeedsReview
    pub remediation: String,
}
```

## Common Tasks

### Adding a Framework Check
1. Add the check function in the framework module (`owasp.rs`, `pci.rs`, `hipaa.rs`, `soc2.rs`).
2. Emit a `ComplianceFinding` with a stable `requirement_id` and actionable `remediation`.
3. Wire aggregation in `mod.rs` / `report.rs` so `passed`/`failed`/`overall_score` stay consistent.
4. Gate new deps behind the `compliance` feature; keep the module compiling with `--no-default-features` off-paths intact.

### Enforcement Notes
- Never bypass `EnforcementContext::evaluate()`; compliance scans still need an approved scope.
- Findings are advisory mappings, not proof of compliance — say so in report text.

## Resources
- `architecture/compliance.md` - compliance architecture deep-dive
- `architecture/overview.md` (Module Index) - compliance row
- `docs/CAPABILITIES.md` - capability/risk matrix entry
