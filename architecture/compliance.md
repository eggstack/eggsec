# Compliance Module

## Purpose

Compliance scanning and reporting against major security frameworks (OWASP Top 10, PCI DSS, HIPAA, SOC 2). Maps findings to framework-specific requirements and generates compliance reports with pass/fail status.

## Key Types

| Type | Location | Description |
|------|----------|-------------|
| `ComplianceReport` | `compliance/mod.rs` | Compliance report with score and per-requirement findings |
| `ComplianceFinding` | `compliance/mod.rs` | Individual requirement check result |
| `ComplianceStatus` | `compliance/mod.rs` | Enum: Pass, Fail, NotApplicable, NeedsReview |
| `ComplianceFramework` | `compliance/mod.rs` | Enum: OWASP, PCIDSS, HIPAA, SOC2 |

## Files

| File | Description |
|------|-------------|
| `mod.rs` | Module root: `ComplianceReport`, `ComplianceFramework` enum, `generate_compliance_report()` |
| `owasp.rs` | OWASP Top 10 compliance mapping |
| `pci.rs` | PCI DSS requirement checks |
| `hipaa.rs` | HIPAA compliance checks |
| `soc2.rs` | SOC 2 compliance checks |
| `report.rs` | Report generation utilities |

## Implementation Status

Fully implemented. All four framework modules provide `generate_report()` functions dispatched by `ComplianceFramework` enum.
