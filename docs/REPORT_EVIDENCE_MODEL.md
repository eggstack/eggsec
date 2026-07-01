# Report & Evidence Model

Comprehensive inventory of report, finding, and evidence types across the Eggsec workspace. Documents the current landscape, conversion bridges, known incompatibilities, and the target normalized model.

## Current Report Types

### eggsec-output Crate Types

| Type | File | Line | Purpose |
|------|------|------|---------|
| `ScanReportData` | `crates/eggsec-output/src/convert.rs` | 8 | Main report data structure. Fields: target, scan_type, timestamp, findings, open_ports, services, duration_ms, wireless_networks, policy_summary. |
| `FindingData` | `crates/eggsec-output/src/convert.rs` | 23 | Lightweight finding for serialization. Fields: title, severity (`String`), category, description, location, evidence (`Option<String>`), remediation (`Option<String>`), cwe_ids. |
| `AgentFinding` | `crates/eggsec-output/src/agent.rs` | 73 | Rich finding type. Fields: id, tool_id, vulnerability_type, severity (`Severity`), title, description, evidence (`Evidence`), remediation (`Remediation`), confidence, cvss, cwe_ids, target, endpoint, parameter, timestamp, attack_surface, status. |
| `Evidence` | `crates/eggsec-output/src/agent.rs` | 303 | Agent evidence struct. Fields: request, response_snippet, diff_indicator, matched_pattern, timing_ms, status_code. |
| `Remediation` | `crates/eggsec-output/src/agent.rs` | 344 | Agent remediation struct. Fields: summary, references, code_example, priority, effort. |
| `Confidence` | `crates/eggsec-output/src/agent.rs` | 6 | Enum: Confirmed, Likely, Possible, Unlikely. |
| `AttackSurface` | `crates/eggsec-output/src/agent.rs` | 40 | Enum: Web, Api, Network, Authentication, Session, FileSystem, Internal, Cloud, Cdn, Database. |
| `FindingStatus` | `crates/eggsec-output/src/agent.rs` | 93 | Enum: New, Confirmed, FalsePositive, Ignored, Remediated. |
| `FindingSummary` | `crates/eggsec-output/src/agent.rs` | 390 | Aggregated summary with `risk_score()`. |
| `PolicySummary` | `crates/eggsec-output/src/policy_summary.rs` | 9 | Policy enforcement summary. Fields: operation_mode, max_risk, total_decisions, denied_count, warning_count, denied_reasons, warnings. |
| `BaselineComparison` | `crates/eggsec-output/src/baseline.rs` | 4 | Baseline diff. Fields: new_findings, resolved_findings, unchanged_findings (all `Vec<AgentFinding>`). |
| `DiffSummary` | `crates/eggsec-output/src/diff.rs` | 4 | Diff statistics. Fields: total_new, total_resolved, total_escalated, total_deescalated, net_change. |

### eggsec-core Types

| Type | File | Line | Purpose |
|------|------|------|---------|
| `Severity` | `crates/eggsec-core/src/types.rs` | 13 | Canonical severity enum: Critical, High, Medium, Low, Info. Single source of truth; re-exported by `eggsec-output`. |
| `SensitiveString` | `crates/eggsec-core/src/types.rs` | 136 | Zeroized credential wrapper for safe handling of secrets. |

### Canonical Finding (eggsec main crate)

| Type | File | Line | Purpose |
|------|------|------|---------|
| `Finding` | `crates/eggsec/src/findings/mod.rs` | 252 | Canonical finding record. Fields: id, fingerprint, title, description, severity, confidence, finding_type, cwe, owasp, cve, affected_asset, location, evidence (`Vec<Evidence>`), reproduction, remediation, discovered_at, source, tags, metadata. |
| `EvidenceKind` | `crates/eggsec/src/findings/mod.rs` | 88 | Enum: HttpRequest, HttpResponse, Header, BodySnippet, Timing, Diff, Banner, DnsRecord, Certificate, PortState, Screenshot, FilePath, LogLine. |
| `Evidence` | `crates/eggsec/src/findings/mod.rs` | 126 | Structured evidence. Fields: kind (`EvidenceKind`), redacted (`bool`), summary, data (`serde_json::Value`). |
| `AffectedAsset` | `crates/eggsec/src/findings/mod.rs` | 161 | Asset reference. Fields: asset_type, identifier, host, port, protocol. |
| `FindingLocation` | `crates/eggsec/src/findings/mod.rs` | 176 | Location details. Fields: url, path, parameter, header, method, line, file. |
| `Reproduction` | `crates/eggsec/src/findings/mod.rs` | 194 | Reproduction steps. Fields: steps, expected, actual. |
| `FindingType` | `crates/eggsec/src/findings/mod.rs` | 207 | Enum: Vulnerability, Misconfiguration, InformationLeak, PolicyViolation, AssetDiscovery, ServiceDetection, WafDetection, FuzzResult, ScanResult. |
| `FindingSource` | `crates/eggsec/src/findings/mod.rs` | 237 | Source provenance. Fields: tool, module, run_id. |

### Domain-Specific Report Types

#### mobile-lab

| Type | File | Line | Purpose |
|------|------|------|---------|
| `MobileScanReport` | `crates/eggsec-mobile-lab/src/lib.rs` | 107 | Static analysis report. Fields: target, platform, findings (`Vec<MobileFinding>`), scan_timestamp, version. |
| `MobileFinding` | `crates/eggsec-mobile-lab/src/lib.rs` | 96 | Static finding. Fields: id, severity, category, title, description, evidence (`Option<String>`), recommendation, cwe_ids. |
| `DynamicMobileReport` | `crates/eggsec-mobile-lab/src/dynamic.rs` | 520 | Dynamic analysis report. Fields: findings, traffic_summary, frida_results, permission_state, etc. |
| `DynamicMobileFinding` | `crates/eggsec-mobile-lab/src/dynamic.rs` | 337 | Dynamic finding. Fields: id, severity, category, title, description, evidence (`Option<String>`), recommendation, cwe_ids, static_correlation (`Option<String>`). |
| `MobileBaseline` | `crates/eggsec-mobile-lab/src/dynamic.rs` | 104 | Lightweight baseline. Fields: target, timestamp, findings_count, frida_script_count, frida_findings, actions_sample. |
| Bridge | `crates/eggsec-mobile-lab/src/lib.rs` | 296 | `to_scan_report_data(&MobileScanReport)` |
| Bridge | `crates/eggsec-mobile-lab/src/dynamic.rs` | 1505 | `to_scan_report_data_dynamic(&DynamicMobileReport)` |
| Evidence bundle | `crates/eggsec-mobile-lab/src/dynamic.rs` | 212 | `export_evidence_bundle()` — gzipped JSON with report, traffic_summary, exported_at, frida_structured, bundle_manifest. |

#### db-pentest

| Type | File | Line | Purpose |
|------|------|------|---------|
| `DbPentestReport` | `crates/eggsec-db-lab/src/types.rs` | 8 | DB assessment report. Fields: db_type, scan_type, target, findings (`Vec<DbFinding>`), queries_executed, dry_run, manifest_path, duration_ms, correlation, compliance, baseline_label, regression_summary. |
| `DbFinding` | `crates/eggsec-db-lab/src/types.rs` | 59 | DB finding. Fields: id, category, severity (`String`), title, description, evidence (`Option<String>`), remediation, cwe_ids. |
| `DbBaseline` | `crates/eggsec-db-lab/src/baseline.rs` | 13 | Baseline with regression detection. Fields: captured_at, db_type, checks, finding_categories, severity_counts, total_findings, report, label. |
| `DbRegressionResult` | `crates/eggsec-db-lab/src/baseline.rs` | 34 | Regression comparison. Fields: new_findings, resolved_findings, severity_increases, severity_decreases, summary, is_regression, is_improvement. |
| Bridge | `crates/eggsec-db-lab/src/bridge.rs` | 9 | `to_scan_report_data_db(&DbPentestReport)` |
| Evidence bundle | `crates/eggsec-db-lab/src/lib.rs` | 645 | `export_db_evidence_bundle()` — gzipped JSON with report, manifest_path, manifest_data, exported_at, correlation, compliance, bundle_manifest. |

#### web-proxy

| Type | File | Line | Purpose |
|------|------|------|---------|
| `WebProxySessionReport` | `crates/eggsec-web-proxy/src/intercept/types.rs` | 101 | Proxy session report. Fields: flows, manipulations, ws_sessions, http2_sessions, grpc_sessions, budget, correlation, correlation_refs. |
| `EvidenceBundle` | `crates/eggsec-web-proxy/src/intercept/bundle.rs` | 19 | Signed bundle. Fields: version, manifest, flows, sessions, rules, manipulations, correlations. Supports HMAC-SHA256 signing. |
| `BundleManifest` | `crates/eggsec-web-proxy/src/intercept/bundle.rs` | 41 | Bundle manifest. Fields: target, scope, started_at, ended_at, user, dry_run, flow/session/manipulation/correlation/rule counts, signature. |
| `BundleDiff` | `crates/eggsec-web-proxy/src/intercept/bundle.rs` | 306 | Bundle comparison. Fields: added/removed/modified flows, count diffs. |
| Bridge | `crates/eggsec-web-proxy/src/intercept/bridge.rs` | 10 | `to_scan_report_data_proxy(&WebProxySessionReport)` |

## Conversion Bridges

| Domain | Bridge Function | File | Line | Input Type |
|--------|----------------|------|------|------------|
| mobile-static | `to_scan_report_data()` | `crates/eggsec-mobile-lab/src/lib.rs` | 296 | `&MobileScanReport` |
| mobile-dynamic | `to_scan_report_data_dynamic()` | `crates/eggsec-mobile-lab/src/dynamic.rs` | 1505 | `&DynamicMobileReport` |
| db-pentest | `to_scan_report_data_db()` | `crates/eggsec-db-lab/src/bridge.rs` | 9 | `&DbPentestReport` |
| web-proxy | `to_scan_report_data_proxy()` | `crates/eggsec-web-proxy/src/intercept/bridge.rs` | 10 | `&WebProxySessionReport` |

### Common Bridge Pattern

All domain bridges follow the same conversion pattern:

1. Map domain findings to `FindingData` with a domain-specific category prefix (e.g., `mobile-static`, `db-pentest`).
2. Add an info-level summary `FindingData` entry for execution metadata.
3. Set `target`, `scan_type`, `timestamp` from the domain report.
4. Leave `open_ports`, `services`, `wireless_networks`, `policy_summary` empty.
5. Serialize `evidence` and `remediation` from native fields into `Option<String>`.

### Evidence Bundles

Each domain maintains its own evidence bundle format:

| Domain | Bundle Format | Signing | Compression |
|--------|---------------|---------|-------------|
| mobile-dynamic | Gzipped JSON | None | gzip |
| db-pentest | Gzipped JSON | None | gzip |
| web-proxy | JSON with `EvidenceBundle` type | HMAC-SHA256 | None (raw) |

There is no shared evidence bundle type across domains. Each includes a `bundle_manifest` with `version` and contents, but the manifest schema is domain-specific. The web-proxy bundle is the most sophisticated, supporting HMAC-SHA256 signing and structured flow/session/rule components.

### Baseline Support

| Domain | Baseline Type | Regression Detection | Severity Tracking |
|--------|---------------|---------------------|-------------------|
| db-pentest | `DbBaseline` | Full (`DbRegressionResult`) | Per-category severity counts |
| mobile-dynamic | `MobileBaseline` | Finding count comparison | None |
| web-proxy | `BundleDiff` via `compare_bundles()` | Added/removed/modified flows | None |

### DomainDescriptor Report Metadata

Domain descriptors declare report integration capabilities via `DomainDescriptor` fields:

| Type | File | Line | Purpose |
|------|------|------|---------|
| `ReportIntegration` | `crates/eggsec/src/domain/mod.rs` | 124 | Fields: `report_kind`, `operation_id`, `evidence_bundle_supported`. |
| `EvidenceSupport` | `crates/eggsec/src/domain/mod.rs` | 147 | Enum: `AlwaysAvailable`, `FeatureGated(&str)`, `NotSupported`. |
| `BaselineSupport` | `crates/eggsec/src/domain/mod.rs` | 158 | Enum: `AlwaysAvailable`, `FeatureGated(&str)`, `NotSupported`. |

## Known Incompatibilities

| # | Issue | Details |
|---|-------|---------|
| 1 | **Severity representation** | `FindingData` uses `String` for severity; `AgentFinding` uses the `Severity` enum; canonical `Finding` uses `Severity` enum. No consistent representation across serialization boundaries. |
| 2 | **Evidence representation** | `Option<String>` in domain findings (`MobileFinding`, `DbFinding`, `DynamicMobileFinding`), `Evidence` struct in agent output (`crates/eggsec-output/src/agent.rs:303`), `EvidenceKind` + `data` (`serde_json::Value`) in canonical `Finding`. Three distinct models. |
| 3 | **No shared evidence bundle manifest** | Each domain defines its own manifest schema. No common versioning, content listing, or integrity verification contract. |
| 4 | **No shared baseline summary format** | `DbBaseline`, `MobileBaseline`, and `BundleDiff` are structurally different with no common fields for normalized comparison. |
| 5 | **Bridge information loss** | All domain bridges target `ScanReportData`, which loses domain-specific information (e.g., Frida results, DB queries, proxy flow budgets, compliance data). |
| 6 | **No normalized report envelope** | No report type preserves `report_id`, `operation_id`, and `domain_id` across the conversion chain. Traceability from output back to execution context is broken. |
| 7 | **PolicySummary underutilized** | `PolicySummary` exists in `eggsec-output` but is not consistently populated by domain bridges, leaving enforcement metadata incomplete in cross-domain reports. |

## Target Normalized Model

A protocol-neutral report/evidence contract that domain crates can convert into a shared `ReportEnvelope`. The model should be dependency-light and serializable, placed in `eggsec-output`.

### Design Goals

- Single serializable representation for all report output across CLI, TUI, REST, MCP, and agent surfaces.
- Domain crates convert into the shared model via bridge functions; no domain type escapes into the output layer.
- Evidence is structured and typed, not a string blob.
- Report traceability via `report_id`, `operation_id`, and `domain_id`.
- Baseline and diff summaries share a common shape.

### Implemented Types

```rust
/// Evidence item with typed kind and structured data.
pub struct EvidenceItem {
    pub kind: EvidenceKind,
    pub summary: String,
    pub data: serde_json::Value,
    pub redacted: bool,
    /// Domain-specific key for grouping (e.g., "http-request", "frida-call").
    pub domain_key: Option<String>,
}

/// Normalized finding record.
pub struct FindingRecord {
    pub id: String,
    pub title: String,
    pub severity: Severity,
    pub confidence: Confidence,
    pub category: String,
    pub description: String,
    pub evidence: Vec<EvidenceItem>,
    pub remediation: Option<String>,
    pub cwe_ids: Vec<String>,
    pub owasp: Option<String>,
    pub cve: Option<String>,
    pub target: String,
    pub location: Option<String>,
    pub source_tool: Option<String>,
    pub source_module: Option<String>,
    pub discovered_at: chrono::DateTime<chrono::Utc>,
    pub tags: Vec<String>,
    /// Opaque domain-specific payload for lossless round-trip.
    pub domain_payload: Option<serde_json::Value>,
}

/// Normalized report envelope.
pub struct ReportEnvelope {
    pub report_id: String,
    pub operation_id: String,
    pub domain_id: Option<String>,
    pub scan_type: String,
    pub target: String,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub finished_at: chrono::DateTime<chrono::Utc>,
    pub duration_ms: u64,
    pub findings: Vec<FindingRecord>,
    pub open_ports: Vec<u16>,
    pub services: Vec<String>,
    pub policy_summary: Option<PolicySummary>,
    /// Domain-specific report payload (e.g., traffic_summary, db queries).
    pub domain_payload: Option<serde_json::Value>,
}

/// Baseline summary for normalized cross-domain comparison.
pub struct BaselineSummary {
    pub baseline_id: String,
    pub domain_id: String,
    pub captured_at: chrono::DateTime<chrono::Utc>,
    pub target: String,
    pub finding_count: usize,
    pub severity_counts: std::collections::HashMap<Severity, usize>,
    pub finding_ids: Vec<String>,
}

/// Diff result between two baselines.
pub struct BaselineDiff {
    pub new_findings: Vec<FindingRecord>,
    pub resolved_findings: Vec<FindingRecord>,
    pub unchanged_findings: Vec<String>, // finding IDs
    pub severity_increases: Vec<String>,
    pub severity_decreases: Vec<String>,
    pub is_regression: bool,
    pub is_improvement: bool,
    pub summary: String,
}

/// Evidence bundle manifest for serialized export.
pub struct EvidenceManifest {
    pub version: String,
    pub bundle_id: String,
    pub target: String,
    pub domain_id: String,
    pub exported_at: chrono::DateTime<chrono::Utc>,
    pub finding_count: usize,
    pub evidence_count: usize,
    pub redaction_policy: RedactionPolicy, // manifest-level redaction strategy
    pub signature: Option<String>,
    pub contents: Vec<String>,
}
```

### RedactionPolicy

The `EvidenceManifest` includes a `redaction_policy` field that declares the manifest-level
redaction strategy. This is distinct from per-item `RedactionState` on `EvidenceItem`:

| Policy | Meaning |
|--------|---------|
| `None` | No redaction; all evidence included as-is |
| `RedactAll` | Redact all items regardless of individual state |
| `RedactSensitive` | Redact only items marked as sensitive |
| `SummarizeAll` | Replace raw content with summaries |
| `DomainSpecific` | Domain-specific logic; individual item states take precedence |

### Migration Path

1. Add these types to `eggsec-output` behind a feature flag (e.g., `normalized-report`).
2. Update each domain bridge to produce `ReportEnvelope` alongside existing `ScanReportData`.
3. Migrate CLI/TUI/REST/MCP output renderers to consume `ReportEnvelope`.
4. Remove legacy `ScanReportData` once all consumers are migrated.
5. Unify evidence bundle export under `EvidenceManifest` with domain-specific payloads.
