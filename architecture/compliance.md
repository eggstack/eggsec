# Compliance Module

## Purpose

Compliance scanning and reporting against four major security frameworks: OWASP Top 10, PCI DSS v4.0, HIPAA Security Rule, and SOC 2 Type II. Maps scan findings to framework-specific requirements and generates scored compliance reports with pass/fail/not-applicable/needs-review status. Feature-gated: `compliance`.

## Location & Feature Gating

| Path | Feature Gate |
|------|-------------|
| `crates/eggsec/src/compliance/mod.rs` | `compliance` (`lib.rs:92`) |
| `crates/eggsec/src/compliance/owasp.rs` | `compliance` |
| `crates/eggsec/src/compliance/pci.rs` | `compliance` |
| `crates/eggsec/src/compliance/hipaa.rs` | `compliance` |
| `crates/eggsec/src/compliance/soc2.rs` | `compliance` |
| `crates/eggsec/src/compliance/report.rs` | `compliance` |

When the feature is disabled, `lib.rs:94-96` compiles the module as `#[allow(dead_code)] mod compliance` (private, unused). The `ComplianceReport` variant in `TaskResult` is gated at `dispatch/types.rs:118-119`.

## Key Types

| Type | Location | Description |
|------|----------|-------------|
| `ComplianceFramework` | `compliance/mod.rs:62` | Enum (4 variants): `OWASP`, `PCIDSS`, `HIPAA`, `SOC2` |
| `ComplianceReport` | `compliance/mod.rs:22` | Report: framework, target, overall_score, total_requirements, passed, failed, findings |
| `ComplianceFinding` | `compliance/mod.rs:32` | Individual check: requirement_id, description, severity, status, remediation |
| `ComplianceStatus` | `compliance/mod.rs:41` | Enum (4 variants): `Pass`, `Fail`, `NotApplicable`, `NeedsReview` |
| `ComplianceSummary` | `compliance/report.rs:4` | Summary: framework, score, risk_level, top_findings (up to 5 critical/high IDs) |
| `RiskLevel` | `compliance/report.rs:12` | Enum (4 variants): `Low` (>=90), `Medium` (>=70), `High` (>=50), `Critical` (<50) |

## Architecture

### Module structure

| File | Lines | Role |
|------|-------|------|
| `compliance/mod.rs` | 84 | `ComplianceReport`, `ComplianceFramework` enum, `generate_compliance_report()` dispatcher, tests |
| `compliance/owasp.rs` | 159 | OWASP Top 10 report generation (5 controls), tests |
| `compliance/pci.rs` | 137 | PCI DSS v4.0 report generation (4 controls), tests |
| `compliance/hipaa.rs` | 137 | HIPAA Security Rule report generation (4 controls), tests |
| `compliance/soc2.rs` | 158 | SOC 2 Type II report generation (5 controls), tests |
| `compliance/report.rs` | 118 | `ComplianceSummary`, `RiskLevel`, `summarize()`, `to_html()`, tests |

### Control catalogs per framework

**OWASP Top 10** (`owasp.rs:19-117`) — **5 controls**:

| # | Requirement ID | Fail Trigger | Severity on Fail |
|---|---------------|-------------|-----------------|
| 1 | `A01:2021 - Broken Access Control` | `has_high` (High severity findings) | Critical |
| 2 | `A02:2021 - Cryptographic Failures` | `!is_https` (target not HTTPS) | High |
| 3 | `A03:2021 - Injection` | `has_critical` (Critical severity findings) | Critical |
| 4 | `A05:2021 - Security Misconfiguration` | `has_medium` (Medium severity findings) | Medium |
| 5 | `A09:2021 - Security Logging & Monitoring` | `has_low` (Low severity findings) | Medium (NeedsReview) |

**PCI DSS v4.0** (`pci.rs:18-96`) — **4 controls**:

| # | Requirement ID | Fail Trigger | Severity on Fail |
|---|---------------|-------------|-----------------|
| 1 | `Req 2.1 - Default credentials` | `has_critical` | Critical |
| 2 | `Req 3.4 - Data encryption at rest` | `!is_https` | Critical |
| 3 | `Req 6.5 - Address common coding vulnerabilities` | `has_high` | High |
| 4 | `Req 11.3 - External penetration testing` | `has_medium` | Medium (NeedsReview) |

**HIPAA Security Rule** (`hipaa.rs:18-96`) — **4 controls**:

| # | Requirement ID | Fail Trigger | Severity on Fail |
|---|---------------|-------------|-----------------|
| 1 | `§164.312(a)(1) - Access Control` | `has_critical` | Critical |
| 2 | `§164.312(d) - Person or Entity Authentication` | `has_high` | High |
| 3 | `§164.312(e)(1) - Transmission Security` | `!is_https` | High |
| 4 | `§164.312(b) - Audit Controls` | `has_medium` | Medium (NeedsReview) |

**SOC 2 Type II** (`soc2.rs:19-117`) — **5 controls**:

| # | Requirement ID | Fail Trigger | Severity on Fail |
|---|---------------|-------------|-----------------|
| 1 | `CC6.1 - Logical and Physical Access Controls` | `has_high` | High |
| 2 | `CC7.1 - System Operations` | `has_medium` | Medium (NeedsReview) |
| 3 | `CC8.1 - Risk Assessment` | `has_critical` | Critical |
| 4 | `CC6.6 - Security Measures` | `!is_https` | High |
| 5 | `CC5.3 - Control Policies` | `has_low` | Medium (NeedsReview) |

**Total**: 18 controls across 4 frameworks (5 + 4 + 4 + 5).

### Score calculation

All four framework modules use identical scoring logic (`owasp.rs:119-132`, `pci.rs:98-111`, `hipaa.rs:98-111`, `soc2.rs:119-132`):

```
overall_score = ((total - failed) / total) * 100.0
```

Where `total` is the number of controls evaluated and `failed` is the count of controls with `ComplianceStatus::Fail`. Controls with `NeedsReview` or `NotApplicable` status are counted as passed for scoring purposes.

### Risk level thresholds (`report.rs:22-27`)

| RiskLevel | Score Range |
|-----------|------------|
| `Low` | >= 90.0 |
| `Medium` | >= 70.0 and < 90.0 |
| `High` | >= 50.0 and < 70.0 |
| `Critical` | < 50.0 |

## Behavior & Flow

### Report generation flow

```
generate_compliance_report(target, framework, findings)
  ├── owasp::generate_report(target, findings)     // ComplianceFramework::OWASP
  ├── pci::generate_report(target, findings)       // ComplianceFramework::PCIDSS
  ├── hipaa::generate_report(target, findings)     // ComplianceFramework::HIPAA
  └── soc2::generate_report(target, findings)      // ComplianceFramework::SOC2
```

Each framework module:
1. Creates `ComplianceReport` with framework name and target.
2. Evaluates each control against `findings` (severity presence) and `target` (HTTPS check).
3. Pushes `ComplianceFinding` for each control with status, severity, and remediation.
4. Counts failed controls and computes `overall_score`.

### Input model

- `target: &str` — Target URL. Checked for `https://` prefix (`target.starts_with("https://")`).
- `findings: &[Severity]` — List of severity levels from scan results. Used to detect presence of Critical/High/Medium/Low findings.

### Dispatch flow (`dispatch/security.rs:59-114`)

`run_compliance_task()` performs a lightweight pre-scan:
1. HTTP GET to target (10s timeout).
2. Collects severity findings from response headers:
   - Non-HTTPS → `Severity::High`
   - Missing `strict-transport-security` → `Severity::Medium`
   - Missing `x-content-type-options` → `Severity::Low`
   - Missing `x-frame-options` / CSP `frame-ancestors` → `Severity::Medium`
   - `server` or `x-powered-by` headers present → `Severity::Low`
   - 5xx status → `Severity::High`
   - 4xx status (not 404) → `Severity::Medium`
3. Passes collected severities to `generate_compliance_report()`.

### HTML report generation (`report.rs:48-97`)

`ComplianceReport::to_html()` produces a self-contained HTML document with:
- Score display (color-coded by risk level).
- Risk level indicator.
- Finding list with severity-based styling (red for critical, orange for high).

### Top findings extraction (`report.rs:29-38`)

`ComplianceSummary::top_findings` collects up to 5 `requirement_id` strings from findings with `Severity::Critical` or `Severity::High`.

## Credential Handling

No credentials are involved. The compliance module operates on in-memory data (findings list and target URL). No external API calls are made from the compliance modules themselves.

## Public API

| Method | Signature | Description |
|--------|-----------|-------------|
| `generate_compliance_report` | `async (target, framework, findings) -> Result<ComplianceReport>` | Generate report for any framework (`mod.rs:49`) |
| `owasp::generate_report` | `async (target, findings) -> Result<ComplianceReport>` | OWASP Top 10 report |
| `pci::generate_report` | `async (target, findings) -> Result<ComplianceReport>` | PCI DSS v4.0 report |
| `hipaa::generate_report` | `async (target, findings) -> Result<ComplianceReport>` | HIPAA Security Rule report |
| `soc2::generate_report` | `async (target, findings) -> Result<ComplianceReport>` | SOC 2 Type II report |
| `ComplianceReport::summarize` | `(&self) -> ComplianceSummary` | Derive risk level + top findings |
| `ComplianceReport::to_html` | `(&self) -> String` | Generate HTML report |

## Integration Points

### Dispatch

- `TaskKind::Compliance` (`dispatch/mod.rs:268-276`): Dispatches to `run_compliance_task()`. Returns `TaskResult::Compliance(ComplianceReport)`.
- `dispatch/security.rs:59-114`: Pre-scan collects header-based findings, then calls `generate_compliance_report()`.

### TUI

- **Compliance tab** (`tabs/compliance.rs`): Gated behind `compliance` feature (`tabs/mod.rs:8,70`).
- **Framework selector**: 4-item selector mapping to `ComplianceFramework` variants (`compliance.rs:37-42, 65-71`).
- **Results display**: Score, risk level, pass/fail counts, finding details (`compliance.rs:74-128`).
- **Tab spec** (`tabs/spec.rs:484-493`): CLI command `eggsec compliance`, description "Generate compliance reports".
- **Task dispatch** (`app/task_dispatcher.rs:160-161`): Handles `TaskResult::Compliance(r)`.
- **State update** (`app/state_update.rs:359-361`): Stores report in `tabs.compliance.set_report(r)`.

### Python bindings

- Feature guard: `eggsec-python/src/features.rs:58,101,284` — exposes `compliance` feature status.
- Version info: `eggsec-python/src/version.rs:40,87` — lists `compliance` in stable and version feature maps.

## Testing

- **Unit tests** (`compliance/mod.rs:71-84`): 1 test for OWASP report generation with empty findings.
- **Unit tests** (`compliance/owasp.rs:137-159`): 2 tests (with findings, clean).
- **Unit tests** (`compliance/pci.rs:116-137`): 2 tests (with findings, clean).
- **Unit tests** (`compliance/hipaa.rs:116-137`): 2 tests (with findings, clean).
- **Unit tests** (`compliance/soc2.rs:137-158`): 2 tests (with findings, clean).
- **Unit tests** (`compliance/report.rs:99-118`): 1 test for risk level derivation from score.
- **Total**: 10 tests.

## Invariants & Gotchas

1. **Severity-driven evaluation**: All framework controls evaluate pass/fail based on the *presence* of severity levels in the findings list — not on specific finding types or vulnerability details. A single `Severity::Critical` in the findings list will trigger all `has_critical` controls to fail.
2. **HTTPS is the only target-aware check**: The `is_https` check (`target.starts_with("https://")`) is the only control that inspects the target string. All other controls depend solely on the findings severity list.
3. **NeedsReview for medium/low triggers**: Controls triggered by `has_medium` or `has_low` produce `ComplianceStatus::NeedsReview` (not `Fail`) in some frameworks (OWASP A09, PCI Req 11.3, HIPAA §164.312(b), SOC2 CC7.1/CC5.3). These do not count toward the `failed` score.
4. **Score counts only Fail status**: `overall_score` computation counts only `ComplianceStatus::Fail` as failed. `NeedsReview` and `NotApplicable` are treated as passed for scoring.
5. **No cross-framework correlation**: Each framework generates an independent report. There is no cross-framework control mapping or correlation.
6. **Fixed control set**: Each framework has a hardcoded set of controls (5/4/4/5 = 18 total). There is no mechanism to add or customize controls.
7. **`async` but no I/O**: The `generate_report()` functions are all `async` but perform no actual I/O — they are pure functions of `target` and `findings`. The `async` is inherited from the dispatch interface.

## Security Considerations

- **Input validation**: The `target` string is used only for the HTTPS prefix check. No URL validation, SSRF protection, or sanitization is performed on the target.
- **Findings trust boundary**: The `findings` list is trusted input from the scan engine. No validation is performed on the severity values.
- **HTML report injection**: `to_html()` (`report.rs:48-97`) interpolates `framework`, `overall_score`, `risk_level`, `requirement_id`, and `description` directly into HTML without escaping. If any of these contain HTML特殊字符, the output could be malformed. In practice, framework names and requirement IDs are hardcoded, and descriptions are static strings, so this is not exploitable.

## Bug Sweep

| Finding | Location | Severity | Description |
|---------|----------|----------|-------------|
| HTML report no escaping | `report.rs:82-94` | Low | `requirement_id` and `description` are interpolated into HTML without `html_escape`. Not exploitable with current hardcoded values, but would be if user-controlled data entered findings. |
| NeedsReview not counted as failed | `mod.rs:119-132` | Design | Controls with `NeedsReview` status are not counted toward `failed` in the score. This is intentional but may understate risk for controls that require human review. |
| No `NotApplicable` in generated reports | All framework files | Note | While `ComplianceStatus::NotApplicable` is defined, no framework module currently generates this status. All controls always produce `Pass`, `Fail`, or `NeedsReview`. |

*Last verified against source: 2026-08-25*
