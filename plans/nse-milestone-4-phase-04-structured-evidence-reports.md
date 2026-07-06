# NSE Milestone 4 Phase 04: Structured Evidence Reports

## Purpose

Upgrade NSE run output from raw script text plus compatibility metadata into structured evidence that can feed Eggsec findings, audits, CLI summaries, and TUI views.

Milestone 2 introduced `NseRunReport`. Milestone 3 added `capability_events`. This phase adds evidence objects: normalized, structured observations extracted from script output and runtime context without hiding raw output.

## Non-Goals

Do not replace raw script output.

Do not invent findings from weak evidence.

Do not claim vulnerability confirmation when a script only reports a heuristic.

Do not add internet-dependent evidence enrichment.

## Evidence Model

Add types such as:

```rust
pub struct NseEvidenceItem {
    pub id: String,
    pub kind: NseEvidenceKind,
    pub severity: Option<String>,
    pub title: String,
    pub summary: String,
    pub target: String,
    pub port: Option<u16>,
    pub service: Option<String>,
    pub confidence: String,
    pub source: String,
    pub raw_excerpt: Option<String>,
    pub references: Vec<String>,
    pub tags: Vec<String>,
}

pub enum NseEvidenceKind {
    ServiceFingerprint,
    VersionInfo,
    CertificateInfo,
    VulnerabilitySignal,
    Misconfiguration,
    CapabilityDenial,
    CompatibilityWarning,
    ScriptOutput,
}
```

Keep names aligned with existing Eggsec finding/report types if available.

## Workstream 1: Evidence Field in `NseRunReport`

Add:

```rust
pub evidence: Vec<NseEvidenceItem>
```

or a nested `NseEvidenceSummary`.

### Acceptance Criteria

- Existing JSON remains backward-compatible if possible.
- Empty evidence is valid.
- Evidence serializes deterministically.

## Workstream 2: Evidence Extraction Rules

Start conservative. Extract evidence from:

- known structured library outputs;
- `stdnse.format_output` style tables where available;
- TLS certificate summaries;
- HTTP title/header summaries;
- service/version fields from context;
- capability denials;
- unsupported/partial compatibility states.

Avoid fragile parsing of arbitrary prose unless the script fixture declares expected extraction rules.

### Acceptance Criteria

- Evidence extraction is deterministic for corpus fixtures.
- Raw output remains available for manual review.
- Evidence confidence is explicit.

## Workstream 3: Finding Integration

Map evidence items to Eggsec findings only when confidence and category justify it.

Examples:

- certificate expired → finding candidate;
- weak TLS protocol observed → finding candidate;
- version banner observed → service fingerprint, not vulnerability;
- script output says “possible” → low-confidence signal;
- capability denied → execution limitation, not target finding.

### Acceptance Criteria

- Evidence and findings are not conflated.
- Capability/report limitations do not become target vulnerabilities.

## Workstream 4: Corpus Tests

Add fixtures for:

- HTTP title evidence;
- TLS certificate evidence;
- service fingerprint evidence;
- vulnerability signal evidence with low confidence;
- capability denial evidence;
- raw output with no extractable evidence.

### Acceptance Criteria

- Evidence item count/kind/confidence is tested.
- Tests do not depend on exact full JSON formatting.

## Workstream 5: Report Compatibility

Ensure evidence generation interacts correctly with:

- `compatibility.status`;
- `capability_events`;
- rule reports;
- library use reports;
- resolver diagnostics.

A run may be `Partial` and still produce useful evidence; reports should state that clearly.

## Workstream 6: Docs

Document:

- evidence kinds;
- confidence meanings;
- difference between evidence, finding, raw output, and compatibility warning;
- limitations of script-derived evidence;
- examples of JSON output.

## Verification

Run:

```bash
cargo test -p eggsec-nse --features nse report
cargo test -p eggsec-nse --features nse evidence
cargo test -p eggsec-nse --features nse compatibility_corpus
cargo check -p eggsec --features nse
bash scripts/check-architecture-guards.sh
```

## Final Acceptance Criteria

Phase 04 is complete when:

- `NseRunReport` can include structured evidence.
- Evidence extraction is conservative, deterministic, and tested.
- Raw output remains present.
- Findings integration, if added, does not overclaim weak signals.
- Docs explain evidence semantics and limitations.
