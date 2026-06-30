# Architecture Extensibility Phase 9: Report and Evidence Unification

## Objective

Unify report, finding, evidence, baseline, and export metadata across domains so results from scanner, WAF, db-pentest, mobile, web-proxy, wireless, and future domains can flow through consistent output adapters without each domain inventing incompatible bridges.

This phase should standardize the contract between domain crates and `eggsec-output` / main report conversion logic. It should not rewrite all report formats at once. The priority is a stable internal model, consistent evidence provenance, and tests that prevent domain report drift.

## Current context

The repo already has several report-related components:

- `eggsec-output` for output adapters such as JSON, CSV, HTML, SARIF, JUnit, and Markdown.
- Domain-local report types such as db-pentest and mobile reports.
- Conversion bridges such as mobile `to_scan_report_data` and dynamic/mobile report bridge functions.
- Evidence bundle support in some domains.
- Baseline/regression support in some domains.
- `DomainDescriptor` fields for `ReportIntegration`, `EvidenceSupport`, and `BaselineSupport`.

The issue is that report capabilities are not yet unified enough to make domain extraction cheap. Each domain can drift in how it represents target, findings, severity, evidence, remediation, timestamps, baseline identity, and export support.

## Non-goals

- Do not remove existing report formats.
- Do not force every domain to use one concrete report struct internally.
- Do not rewrite all existing output adapters.
- Do not add external storage or database dependencies.
- Do not add new scanning capabilities.
- Do not change enforcement semantics.

## Design target

Define a protocol-neutral report/evidence contract. Domain crates may keep domain-specific report structs, but they should be able to convert into a shared report envelope.

Suggested types:

```rust
pub struct EvidenceItem {
    pub id: String,
    pub kind: EvidenceKind,
    pub source: EvidenceSource,
    pub summary: String,
    pub data_ref: Option<String>,
    pub redaction: RedactionState,
}

pub struct FindingRecord {
    pub id: String,
    pub domain: String,
    pub operation_id: String,
    pub severity: Severity,
    pub title: String,
    pub description: String,
    pub evidence: Vec<EvidenceItem>,
    pub remediation: Option<String>,
    pub references: Vec<String>,
}

pub struct ReportEnvelope {
    pub report_id: String,
    pub operation_id: String,
    pub domain_id: Option<String>,
    pub target: Option<String>,
    pub generated_at: DateTime<Utc>,
    pub findings: Vec<FindingRecord>,
    pub evidence_manifest: EvidenceManifest,
    pub policy_summary: Option<PolicySummary>,
    pub baseline: Option<BaselineSummary>,
}
```

Exact names may differ. Keep the model dependency-light and serializable.

## Work item 1: Inventory existing report and evidence paths

Document current report and evidence types.

Inspect:

- `crates/eggsec-output/**`
- main report conversion modules
- db-pentest report bridge
- mobile static/dynamic report bridge
- web-proxy report/evidence support
- scanner/fuzzer/pipeline output types
- SARIF/JUnit/HTML/JSON adapters
- baseline/regression helpers

Deliverable:

- Add `docs/REPORT_EVIDENCE_MODEL.md` with:
  - current report types;
  - current conversion bridges;
  - domains with evidence support;
  - domains with baseline support;
  - known incompatibilities;
  - target normalized model.

Acceptance criteria:

- The report inventory is sufficient to guide migration without reverse-engineering every module again.

## Work item 2: Define normalized evidence model

Add a small shared evidence model in the correct crate.

Recommended placement:

- If dependency-light and broadly useful: `eggsec-core` or `eggsec-output`.
- If tied to output conversion: `eggsec-output`.
- Avoid placing it only in the main `eggsec` crate if domain crates need to emit it without depending on the composition root.

Required fields:

- evidence ID;
- evidence kind;
- source/provenance;
- summary;
- optional structured data reference or inline JSON value;
- redaction/sensitivity classification;
- collection timestamp if relevant.

Evidence kinds should include at least:

- HTTP request/response summary;
- file metadata;
- static finding evidence;
- database finding evidence;
- mobile manifest/config evidence;
- runtime/log evidence;
- traffic/proxy evidence;
- generic structured evidence.

Acceptance criteria:

- Evidence model is serializable.
- Evidence model does not require heavy optional domain dependencies.
- Sensitive data can be represented as redacted/summarized rather than blindly serialized.

## Work item 3: Define normalized finding/report envelope

Add a normalized finding/report envelope that existing domain bridges can target.

Required fields:

- report ID or deterministic report key;
- operation ID;
- domain ID;
- target or local artifact identifier;
- generated timestamp;
- finding records;
- evidence manifest;
- policy/enforcement summary if available;
- baseline/regression summary if available;
- tool/version metadata if available.

Do not require every domain to populate every field.

Acceptance criteria:

- Existing output adapters can consume the normalized envelope or can be adapted incrementally.
- Domain-specific reports can convert into the envelope without losing important information.

## Work item 4: Migrate pilot domain bridges

Migrate two pilot bridges to produce or validate against the normalized model.

Recommended pilots:

1. `mobile-static` because it is local-file based and currently has adapter tests.
2. `db-pentest` because it has evidence/baseline support and represents a domain crate extraction pattern.

Alternative:

- mobile static + mobile dynamic if db-pentest is too broad.

Required behavior:

- Existing human/JSON output remains compatible.
- Existing `to_scan_report_data` style bridge either delegates to the normalized model or is tested against it.
- Severity, remediation, evidence, target, and category are preserved.

Acceptance criteria:

- Two pilot domains can emit normalized reports.
- Existing report tests still pass.
- Roundtrip serialization tests exist.

## Work item 5: Evidence bundle manifest standardization

Standardize evidence bundle metadata independent of domain.

Required fields:

- bundle ID;
- operation ID;
- domain ID;
- target/artifact identity;
- generated timestamp;
- item list with IDs/kinds/hashes if files are stored;
- redaction policy;
- producer version;
- optional policy/enforcement correlation ID.

Acceptance criteria:

- Domains with evidence support can declare evidence bundle compatibility.
- Evidence bundle tests verify manifest serialization and redaction state.

## Work item 6: Baseline/regression metadata standardization

Define a small baseline summary model.

Required fields:

- baseline ID;
- baseline source;
- comparison timestamp;
- added/resolved/unchanged counts;
- severity deltas;
- optional domain-specific summary.

Acceptance criteria:

- Domains with `BaselineSupport::AlwaysAvailable` can expose a common baseline summary.
- Existing baseline behavior is not broken.

## Work item 7: Update `DomainDescriptor` report metadata if needed

`ReportIntegration`, `EvidenceSupport`, and `BaselineSupport` may need refinement.

Possible additions:

- normalized report support flag;
- evidence manifest support flag;
- report format compatibility;
- baseline summary support;
- redaction support.

Keep this small. Avoid turning `DomainDescriptor` into a large schema registry.

Acceptance criteria:

- Capability matrix can accurately show report/evidence/baseline support.
- Metadata consistency tests cover required report metadata fields.

## Work item 8: Report/evidence consistency tests

Add tests:

- every domain with `EvidenceSupport::AlwaysAvailable` has report integration with evidence support or an explicit exception;
- every normalized report has operation/domain IDs that resolve to metadata;
- pilot domain conversion preserves severity, target, evidence, and remediation;
- evidence manifests serialize/deserialize;
- redaction state is preserved;
- baseline summaries serialize/deserialize.

Suggested files:

- `crates/eggsec-output/tests/report_envelope.rs`
- `crates/eggsec/tests/report_metadata.rs`
- domain-specific tests in `eggsec-mobile-lab` / `eggsec-db-lab` as appropriate.

## Safety requirements

- Do not serialize secrets by default.
- Evidence should support redaction and summaries.
- Report generation must not perform network side effects.
- Domain crates must not use report metadata to authorize operations.
- Policy summaries must reflect actual enforcement results, not reconstructed assumptions.

## Files likely to change

- `crates/eggsec-output/**`
- `crates/eggsec-core/**` if shared model belongs there
- `crates/eggsec/src/output/**` or report conversion modules
- `crates/eggsec/src/domain/mod.rs`
- `crates/eggsec-mobile-lab/**`
- `crates/eggsec-db-lab/**`
- `crates/eggsec/tests/**`
- `docs/REPORT_EVIDENCE_MODEL.md`
- `docs/CAPABILITY_MATRIX.md`
- `docs/METADATA_OWNERSHIP.md`

## Validation commands

Run:

```bash
cargo fmt --all --check
cargo check --workspace --no-default-features
cargo test -p eggsec-output --lib
cargo test -p eggsec --lib
cargo test -p eggsec --test metadata_consistency
cargo test -p eggsec --test mobile_adapter --features mobile
```

Feature/domain checks:

```bash
cargo test -p eggsec --features db-pentest --test metadata_consistency
cargo test -p eggsec-mobile-lab --features mobile-dynamic
cargo test -p eggsec-db-lab --features db-drivers
```

Adjust feature commands to match actual crate feature names.

## Completion criteria

Phase 9 is complete when:

- A normalized report/evidence model exists.
- Two pilot domains can convert into it.
- Evidence manifest and baseline summary semantics are documented.
- Sensitive evidence handling has a redaction model.
- Report/evidence metadata consistency tests exist.
- Existing output adapters and tests continue to pass.

## Handoff note

Keep this phase focused on internal model unification and pilot conversions. A later phase can expand all output adapters or introduce generated docs once the model is stable.
