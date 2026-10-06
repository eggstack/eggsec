# Canonical Findings Schema

Eggsec uses a canonical finding schema for consistent security result reporting.

## Finding Structure

Each finding contains:
- **id**: Unique identifier
- **fingerprint**: Stable hash for deduplication across scans
- **title**: Human-readable title
- **description**: Detailed description
- **severity**: `Severity` — Critical, High, Medium, Low, Info (`eggsec_core::types::Severity`; default `Info`)
- **confidence**: `Confidence` — Confirmed, High, Medium, Low, Informational
- **finding_type**: Vulnerability, Misconfiguration, InformationLeak, etc.
- **cwe**: CWE identifier (optional)
- **owasp**: OWASP category (optional)
- **cve**: CVE identifier (optional)
- **affected_asset**: Target information
- **location**: Where the issue was found
- **evidence**: Supporting evidence (redacted by default)
- **remediation**: Fix recommendations (optional)

Note: `Severity` and `Confidence` are distinct vocabularies with different
terminal values (`Info` vs `Informational`) — do not treat them as one enum.
This schema is the standalone finding model. The normalized report/evidence
envelope contract (`ReportEnvelope`, `FindingRecord`, `EvidenceManifest`) is
owned separately by `eggsec-report-model`; see
[REPORT_EVIDENCE_MODEL.md](REPORT_EVIDENCE_MODEL.md).

## Fingerprinting

Findings generate stable fingerprints based on:
- Target/asset identifier
- Finding type
- Location path/parameter
- CWE or vulnerability class
- Normalized title

Timestamps and random IDs are NOT included in fingerprints.

## Redaction

Redaction is not applied inline by this schema. It is driven by two contracts:

- **Manifest level** — `RedactionPolicy` on `EvidenceManifest` in
  `eggsec-report-model` selects `None`, `RedactAll`, `RedactSensitive`,
  `SummarizeAll`, or `DomainSpecific`. Default is `None`; emitting a report
  that contains secrets requires selecting a redacting policy explicitly.
- **Secret-bearing values** — `SensitiveString` renders as `[REDACTED]` in
  `Debug`, `Display`, and `for_logging(true)` output
  (`crates/eggsec/src/types.rs`), so a wrapped secret cannot leak through
  formatting.

Do not reintroduce the removed `utils::redaction` helper (guard Check 147);
per-category redaction markers belong to the manifest policy above.

See `crates/eggsec/src/findings/mod.rs` for the standalone schema and
`crates/eggsec-report-model/src/envelope.rs` for the envelope contract.
