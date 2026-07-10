# Python API Milestone E — Findings, Reporting, Storage, and Integrations

Status: Executed

## Goal

Turn Eggsec’s Python API into a durable assessment platform by standardizing findings, evidence, artifacts, vulnerability lifecycle, persistence, baseline comparison, reporting, compliance mappings, and external publication.

## Dependencies

- Milestones A and B.
- Domain result types from Milestones C and D should already emit common findings and artifacts.
- Existing Phase D `Finding`, `FindingSet`, `Evidence`, and `Report` bindings provide migration input.

## Workstream E1 — Versioned finding schema

Finalize a stable finding schema with ID, domain, title, description, severity, confidence, evidence, target/location, references, CVE/CWE identifiers, CVSS data, remediation, tags, timestamps, provenance, operation/tool metadata, suppression state, and lifecycle state.

Preserve domain-specific extension data through a versioned `details` structure with documented compatibility rules. Avoid arbitrary untyped dictionaries for core fields.

## Workstream E2 — Evidence and artifact model

Define `EvidenceKind`, `Evidence`, `Artifact`, `ArtifactReference`, and `ArtifactStore` protocols. Provide in-memory and filesystem stores first.

Artifacts should include content hash, MIME type, size, provenance, redaction metadata, retention policy, lazy-loading support, and external references. Ensure binary artifacts can use `memoryview` without unnecessary copies where practical.

## Workstream E3 — Vulnerability and CVSS primitives

Expose CVSS vector parsing/calculation for supported versions, vulnerability records, prioritization, exploitability context, remediation tracking, duplicate correlation, asset criticality, and risk acceptance.

Do not collapse observed weakness, mapped CVE, and confirmed vulnerability into one concept. Preserve confidence and provenance.

## Workstream E4 — Finding workflow

Add finding state, workflow transition, suppression, risk acceptance, and remediation-record objects. Validate transitions and emit audit events for state changes.

Support triage workflows without forcing persistence. Keep workflow logic usable with in-memory and repository-backed findings.

## Workstream E5 — Repository abstraction

Add `FindingRepository`, `AssessmentRepository`, and `ArtifactRepository` protocols with an initial `SqliteRepository` and optional database-backed implementations.

Support save/load assessments, query/filter findings, deduplication, baseline comparison, regression detection, schema migrations, and read-only access. Filters should cover target, severity, state, domain, date, tags, operation, and identifiers.

## Workstream E6 — Baselines and comparisons

Add `AssessmentBaseline`, `AssessmentDiff`, and finding correlation rules. Distinguish new, resolved, changed, unchanged, suppressed, and indeterminate findings.

Correlation must be deterministic, explainable, and overridable. Persist correlation provenance.

## Workstream E7 — Reporting

Expose reporter interfaces for JSON, JSONL, Markdown, HTML, CSV, SARIF, CycloneDX/SPDX where applicable, and PDF when feature-enabled.

Reporters consume stable result/finding protocols, not domain internals. Support redaction policies, artifact inclusion rules, deterministic ordering, and streaming output for large result sets.

## Workstream E8 — Compliance mapping

Feature-gated types should include framework, control, mapping, evidence, and assessment result objects. Clearly state that technical control observations are not definitive legal compliance determinations.

Mappings must preserve source, version, confidence, and rationale.

## Workstream E9 — External integrations

Add GitHub, GitLab, Jira, generic webhook, and optional custom Python adapter interfaces. Support dry-run, deterministic deduplication keys, create/update behavior, attachments, redaction, retry policy, and audit records.

Never send credentials or sensitive evidence unless explicitly allowed by publication policy.

## Workstream E10 — Migration and compatibility

Provide adapters from existing Phase D report/finding objects. Add schema-version metadata and migration helpers. Document stability guarantees and compatibility windows.

## Testing

- Finding schema round-trip and version migration tests.
- Evidence/artifact redaction and hash tests.
- CVSS reference-vector tests.
- Workflow transition and audit tests.
- Repository migration, concurrency, read-only, and recovery tests.
- Baseline correlation determinism tests.
- Reporter golden files and SARIF validation.
- Integration dry-run/deduplication/retry tests.
- Sensitive-data publication tests.

## Acceptance criteria

- All bound domains serialize to one versioned finding schema.
- Results persist and reload without losing domain-specific information.
- Baseline/regression comparison is available and explainable.
- Reports are generated independently of domain implementation code.
- External publication supports dry-run and deterministic deduplication.
- Sensitive evidence is redacted or excluded according to policy.
- Storage migrations are tested across supported schema versions.

## Risks

- Premature schema lock-in: use explicit versioning and extension fields.
- False deduplication: preserve correlation rationale and confidence.
- Large artifacts: use lazy references and retention policies.
- Integration side effects: default to dry-run in tests and require explicit publication.

## Handoff notes

Finalize the finding schema before broad storage/reporting work. Implement artifact stores and SQLite persistence next, then baseline comparison, reporting, workflow/CVSS, compliance, and external integrations. Every new domain should adopt the schema as it lands.