# Adding Report and Evidence Output with the Normalized Report Envelope

This guide explains how to produce structured, normalized report and evidence
output using the `ReportEnvelope` contract defined in `eggsec-output::envelope`.
Every domain crate that produces findings or evidence should convert its
domain-specific types into a `ReportEnvelope` rather than inventing its own
report schema.

See also:

- [REPORT_EVIDENCE_MODEL.md](../REPORT_EVIDENCE_MODEL.md) for the full type
  inventory, known incompatibilities, and design rationale.
- [architecture/report_envelope.md](../../architecture/report_envelope.md) for
  the architecture-level contract and conversion pattern.

## 1. What Belongs in eggsec-output

The `eggsec-output` crate owns all shared report, finding, and evidence types.
Domain crates depend on `eggsec-output` but never depend on each other for
report types. The `envelope` module (`crates/eggsec-output/src/envelope.rs`)
defines:

| Type | Purpose |
|------|---------|
| `ReportEnvelope` | Top-level report container with report_id, operation_id, domain_id, findings, evidence manifest, policy summary, baseline summary |
| `FindingRecord` | Normalized finding with id, domain, operation_id, severity, title, description, evidence, remediation, references, category, location |
| `EvidenceItem` | Single evidence entry with id, kind, source, summary, data_ref, redaction state, collected_at |
| `EvidenceManifest` | Manifest tracking all evidence items with total/redacted counts and redaction policy |
| `EvidenceKind` | Category enum for evidence data (HttpRequest, DatabaseFinding, MobileManifest, TrafficCapture, etc.) |
| `EvidenceSource` | Provenance struct with tool, module, and run_id |
| `RedactionState` | Per-item sensitivity classification (None, FullyRedacted, PartiallyRedacted, Summarized) |
| `RedactionPolicy` | Manifest-level redaction strategy (None, RedactAll, RedactSensitive, SummarizeAll, DomainSpecific) |
| `BaselineSummary` | Standardized baseline comparison with added/resolved/unchanged counts and severity deltas |
| `ToolMetadata` | Tool name and version metadata |

Domain-specific report types (e.g., `MobileScanReport`, `DbPentestReport`,
`WebProxySessionReport`) live in their own crates. They are internal to the
domain. The only types that cross the crate boundary into output consumers are
the normalized types in `eggsec-output::envelope`.

## 2. How to Use the Report Envelope

### 2.1 Creating an Envelope

Every envelope requires an operation ID. Use the operation ID from your
`DomainDescriptor` or `OperationMetadata`:

```rust
use eggsec_output::envelope::ReportEnvelope;

let envelope = ReportEnvelope::new("db-pentest")
    .with_domain_id("db-pentest")
    .with_target("localhost:5432");
```

### 2.2 Adding Findings

Convert each domain finding into a `FindingRecord` and attach it:

```rust
use eggsec_output::envelope::{EvidenceItem, EvidenceKind, EvidenceSource, FindingRecord, RedactionState};
use eggsec_core::types::Severity;

let record = FindingRecord::new(
    "db-postgres-0",           // unique finding ID
    "db-pentest",              // domain
    "db-pentest",              // operation_id
    Severity::High,
    "Dangerous Extension",
    "Postgres extension allows arbitrary code execution",
)
.with_category("db-postgres-misconfig-dangerous-extension")
.with_location("localhost:5432")
.with_remediation("Revoke dangerous extensions")
.with_reference("CWE-94");

let envelope = envelope.with_finding(record);
```

### 2.3 Attaching Evidence to Findings

Every finding should carry evidence items that support the finding. Use the
builder methods:

```rust
let evidence = EvidenceItem::new(
    "db-postgres-0-ev-0",
    EvidenceKind::DatabaseFinding,
    EvidenceSource {
        tool: "eggsec-db-lab".to_string(),
        module: Some("db-pentest".to_string()),
        run_id: None,
    },
    "Extension pg_exec loaded in database",
)
.with_data_ref("SELECT * FROM pg_extension")
.with_redaction(RedactionState::PartiallyRedacted);

let record = FindingRecord::new(/* ... */)
    .with_evidence(evidence);
```

### 2.4 Rebuilding the Evidence Manifest

After assembling all findings, call `refresh_evidence_manifest()` to
rebuild the manifest from the current findings. This preserves the existing
redaction policy and producer version:

```rust
envelope.refresh_evidence_manifest();
```

### 2.5 Serializing

The envelope serializes to JSON for cross-surface consumption:

```rust
let json = envelope.to_json()?;
let deserialized = ReportEnvelope::from_json(&json)?;
```

## 3. Defining Evidence Items, Evidence Source, Redaction State, and Finding Records

### 3.1 EvidenceItem

`EvidenceItem` is a single piece of evidence. Every item has:

- **id**: Unique identifier (e.g., `"db-postgres-0-ev-0"`).
- **kind**: An `EvidenceKind` variant that categorizes the data. Use the most
  specific variant available (e.g., `DatabaseFinding` for DB evidence, not
  `Generic`).
- **source**: An `EvidenceSource` with `tool` (crate name), optional `module`
  (sub-component), and optional `run_id`.
- **summary**: Human-readable description of what the evidence shows.
- **data_ref**: Optional reference to the actual data (file path, URL, query
  string, or inline JSON).
- **redaction**: A `RedactionState` indicating sensitivity.
- **collected_at**: Optional timestamp.

Use the builder methods (`with_data_ref`, `with_redaction`, `with_collected_at`)
to set optional fields.

### 3.2 EvidenceSource

`EvidenceSource` tracks provenance. Always set `tool` to the crate name
(e.g., `"eggsec-db-lab"`, `"eggsec-mobile-lab"`). Set `module` when the
evidence comes from a specific sub-component (e.g., `"static-analysis"`,
`"dynamic-instrumentation"`, `"correlation-engine"`).

### 3.3 RedactionState

Classify each evidence item's sensitivity:

| State | Meaning |
|-------|---------|
| `None` | No sensitive data; full content is safe to include |
| `FullyRedacted` | Only a placeholder is included (e.g., `"[REDACTED]"`) |
| `PartiallyRedacted` | Sensitive fields are masked (e.g., credentials in a request) |
| `Summarized` | Original content replaced with a summary |

When exporting evidence bundles, the manifest-level `RedactionPolicy` governs
how these individual states are interpreted. For example, `RedactAll` overrides
individual `None` states.

### 3.4 FindingRecord

`FindingRecord` is the normalized finding. Every record requires:

- **id**: Unique within the report.
- **domain**: The domain that produced the finding (e.g., `"db-pentest"`).
- **operation_id**: The operation that generated this finding.
- **severity**: Uses the canonical `Severity` enum from `eggsec-core::types`.
- **title**: Short, human-readable title.
- **description**: Detailed description.

Optional fields via builders:

- **evidence**: One or more `EvidenceItem` entries.
- **remediation**: Recommended remediation text.
- **references**: CWE, OWASP, CVE identifiers, or URLs.
- **category**: Domain-specific classification string (e.g., `"db-postgres-sqli"`).
- **location**: Endpoint or artifact where the finding was observed.

### 3.5 EvidenceManifest

The `EvidenceManifest` is rebuilt from findings by `refresh_evidence_manifest()`.
Do not construct it manually. If you need a custom redaction policy, set it
before refreshing:

```rust
envelope = envelope.with_redaction_policy(RedactionPolicy::RedactSensitive);
envelope.refresh_evidence_manifest();
```

### 3.6 BaselineSummary

If the domain supports baseline comparison, populate a `BaselineSummary`:

```rust
use eggsec_output::envelope::BaselineSummary;

let mut baseline = BaselineSummary::new("db-pentest");
baseline.added = 1;
baseline.resolved = 0;
baseline.unchanged = 5;
baseline.severity_deltas.insert("high".to_string(), 1);
baseline.compute_flags(); // sets is_regression / is_improvement
baseline.summary = Some("1 new high finding since baseline".to_string());

envelope = envelope.with_baseline(baseline);
```

## 4. How Command/Tool/Domain Outputs Map into Normalized Evidence

### 4.1 Domain Bridge Pattern

Every domain crate that produces findings should provide a
`to_report_envelope()` function alongside its existing bridge (e.g.,
`to_scan_report_data()`). The pattern is:

1. Map each domain finding to a `FindingRecord` with a domain-specific
   category prefix (e.g., `"db-postgres-misconfig-dangerous-extension"`,
   `"mobile-android-insecure-permission"`).
2. Convert domain evidence fields into `EvidenceItem` entries with the
   appropriate `EvidenceKind` and `EvidenceSource`.
3. Add an info-level `FindingRecord` for execution metadata (target, scan type,
   parameters, duration).
4. Attach correlation evidence if the domain supports it.
5. Populate `ToolMetadata` with the crate name and version.
6. Attach `BaselineSummary` if baseline comparison was performed.
7. Call `refresh_evidence_manifest()`.

Example from `crates/eggsec-db-lab/src/bridge.rs`:

```rust
pub fn to_report_envelope(result: &DbPentestReport) -> ReportEnvelope {
    // Map domain findings to FindingRecords
    // Add correlation finding if present
    // Add execution metadata finding
    // Attach ToolMetadata and BaselineSummary
    // Call envelope.refresh_evidence_manifest()
    envelope
}
```

### 4.2 EvidenceKind Selection

Choose the most specific `EvidenceKind` for each evidence item:

| Domain | Recommended EvidenceKind |
|--------|--------------------------|
| DB pentest | `DatabaseFinding` for query results, `Generic` for config dumps |
| Mobile static | `StaticAnalysis` for manifest analysis, `FileMetadata` for file info |
| Mobile dynamic | `RuntimeInstrumentation` for Frida results, `TrafficCapture` for network |
| Web proxy | `TrafficCapture` for intercepted flows, `HttpRequest`/`HttpResponse` |
| Scanner | `PortState` for port results, `Banner` for service detection |
| Recon | `DnsRecord`, `Certificate`, `Banner` |
| Fuzzer | `HttpRequest`, `HttpResponse`, `Timing`, `Diff` |

### 4.3 Category Naming Convention

Use a domain-prefixed category string:

- `db-postgres-*` or `db-mysql-*` for DB findings
- `mobile-android-*` or `mobile-ios-*` for mobile findings
- `proxy-*` for web proxy findings
- `scan-*` for scanner findings
- `recon-*` for recon findings

### 4.4 Preserving Legacy Bridges

Existing `to_scan_report_data()` bridges are preserved for backward
compatibility. New `to_report_envelope()` bridges produce envelopes alongside
them. Both functions read from the same domain report type. Do not remove
legacy bridges until all consumers are migrated.

## 5. Avoiding Domain-Specific Report Schemas That Bypass the Envelope

### 5.1 The Problem

Domain crates sometimes define their own report types and export them directly.
This leads to:

- No shared finding IDs or report traceability.
- Inconsistent severity representations (String vs. enum).
- Evidence as unstructured string blobs.
- No common baseline or diff format.
- Output consumers (CLI, TUI, REST, MCP, agent) must handle N different
  report shapes.

### 5.2 The Rule

**Domain crates must not export their report types as the external output
contract.** They may define internal report structs (e.g., `DbPentestReport`,
`MobileScanReport`) for their own use, but the external output must be a
`ReportEnvelope`. The conversion happens via `to_report_envelope()`.

This means:

- `DbPentestReport` stays inside `eggsec-db-lab`. It is never returned from
  `eggsec` dispatch paths.
- `MobileScanReport` stays inside `eggsec-mobile-lab`. Output consumers see
  `ReportEnvelope`.
- If a consumer needs domain-specific detail, use the `FindingRecord.category`
  string or the `EvidenceItem.data_ref` field for domain payloads. The
  `EvidenceManifest` carries `domain_id` for filtering.

### 5.3 What to Do Instead

| Anti-pattern | Correct approach |
|-------------|-----------------|
| Return `MobileScanReport` from tool dispatch | Return `ReportEnvelope`; convert in bridge |
| Define a custom finding enum in the domain crate | Use `FindingRecord` with domain-prefixed category |
| Store evidence as `Option<String>` | Use `EvidenceItem` with typed `EvidenceKind` |
| Create domain-specific JSON output format | Serialize `ReportEnvelope` to JSON |
| Skip `refresh_evidence_manifest()` | Always call it after assembling findings |

### 5.4 Domain-Specific Payloads

If the domain has data that does not fit into `FindingRecord` (e.g., Frida
instrumentation results, DB query logs, proxy flow budgets), use one of:

1. **`EvidenceItem.data_ref`**: Point to a file path, URL, or inline JSON
   containing the domain payload.
2. **`FindingRecord` with `EvidenceKind::Correlation`**: Use a dedicated
   correlation finding to carry cross-domain metadata.
3. **Execution metadata finding**: Add an info-level `FindingRecord` with
   the metadata in the description, following the pattern in existing bridges.

Do not add ad-hoc fields to `ReportEnvelope` or `FindingRecord`. The envelope
is the contract; domain specifics stay in evidence items and category strings.

## 6. Required Tests

### 6.1 Integration Tests

Every domain crate that provides a `to_report_envelope()` bridge should include
tests that verify:

1. **Serialization roundtrip**: Serialize the envelope to JSON and deserialize
   it back. Verify all fields survive.
2. **Finding count**: The envelope contains the expected number of findings.
3. **Severity preservation**: Finding severities match the domain report.
4. **Evidence attached**: Findings with evidence have at least one
   `EvidenceItem` with the correct `EvidenceKind`.
5. **Manifest rebuilt**: After `refresh_evidence_manifest()`, the manifest
   `total_items` matches the sum of evidence across all findings.
6. **Redaction state preserved**: Evidence items with non-default redaction
   states survive serialization.

Example test from `crates/eggsec-output/tests/report_envelope.rs`:

```rust
#[test]
fn report_envelope_full_roundtrip() {
    let source = EvidenceSource {
        tool: "eggsec-db-lab".to_string(),
        module: Some("db-pentest".to_string()),
        run_id: None,
    };

    let finding = FindingRecord::new(
        "f-1",
        "db-pentest",
        "db-pentest",
        Severity::High,
        "Dangerous Extension",
        "Postgres extension allows arbitrary code execution",
    )
    .with_evidence(
        EvidenceItem::new("ev-1", EvidenceKind::DatabaseFinding, source, "extension pg_exec")
            .with_redaction(RedactionState::PartiallyRedacted),
    )
    .with_remediation("Revoke dangerous extensions")
    .with_reference("CWE-94")
    .with_category("db-postgres-misconfig-dangerous-extension");

    let envelope = ReportEnvelope::new("db-pentest")
        .with_domain_id("db-pentest")
        .with_target("localhost:5432")
        .with_finding(finding);

    let json = envelope.to_json().unwrap();
    let deserialized = ReportEnvelope::from_json(&json).unwrap();

    assert_eq!(deserialized.operation_id, "db-pentest");
    assert_eq!(deserialized.findings.len(), 1);
    assert_eq!(deserialized.findings[0].severity, Severity::High);
    assert_eq!(deserialized.findings[0].evidence.len(), 1);
}
```

### 6.2 Running Tests

Run the envelope integration tests:

```bash
cargo test -p eggsec-output --test report_envelope
```

Run the full output crate tests:

```bash
cargo test -p eggsec-output
```

### 6.3 Test Checklist

When adding a new domain bridge, verify:

- [ ] `to_report_envelope()` compiles and passes `cargo check`.
- [ ] Roundtrip serialization test passes.
- [ ] Evidence items use the correct `EvidenceKind` variant.
- [ ] `EvidenceSource.tool` is set to the domain crate name.
- [ ] `refresh_evidence_manifest()` is called before returning.
- [ ] `ToolMetadata` is populated with crate name and version.
- [ ] Category strings use the domain prefix convention.
- [ ] The existing `to_scan_report_data()` bridge is unchanged.
- [ ] `cargo clippy -p eggsec-output` produces no new warnings.
