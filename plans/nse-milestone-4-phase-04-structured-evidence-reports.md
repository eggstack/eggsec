# NSE Milestone 4 Phase 04: Structured Evidence Reports

> **Status: Executed** — Completed 2026-07-06.

## Purpose

Upgrade NSE run output from raw script text plus compatibility metadata into structured evidence that can feed Eggsec findings, audits, CLI summaries, and TUI views.

Milestone 2 introduced `NseRunReport`. Milestone 3 added `capability_events`. This phase adds evidence objects: normalized, structured observations extracted from script output and runtime context without hiding raw output.

## Non-Goals

- Do not replace raw script output.
- Do not invent findings from weak evidence.
- Do not claim vulnerability confirmation when a script only reports a heuristic.
- Do not add internet-dependent evidence enrichment.
- Do not create a circular dependency between `eggsec-nse` and `eggsec-output`.

## Evidence Model

### NSE-Internal Types (in `report.rs`)

Add to `crates/eggsec-nse/src/report.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

impl fmt::Display for NseEvidenceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ServiceFingerprint => write!(f, "service-fingerprint"),
            Self::VersionInfo => write!(f, "version-info"),
            Self::CertificateInfo => write!(f, "certificate-info"),
            Self::VulnerabilitySignal => write!(f, "vulnerability-signal"),
            Self::Misconfiguration => write!(f, "misconfiguration"),
            Self::CapabilityDenial => write!(f, "capability-denial"),
            Self::CompatibilityWarning => write!(f, "compatibility-warning"),
            Self::ScriptOutput => write!(f, "script-output"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NseEvidenceItem {
    /// Unique identifier for this evidence item (e.g. "nse-ev-0").
    pub id: String,
    /// Category of the evidence.
    pub kind: NseEvidenceKind,
    /// Title summarizing the observation.
    pub title: String,
    /// Detailed summary of the evidence.
    pub summary: String,
    /// Target host/port the evidence relates to.
    pub target: String,
    /// Optional port number.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    /// Optional service name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service: Option<String>,
    /// Confidence level: "confirmed", "likely", "possible", "low".
    pub confidence: String,
    /// Source module/script that produced this evidence.
    pub source: String,
    /// Optional raw excerpt from script output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_excerpt: Option<String>,
    /// References (CWE, CVE, URLs).
    pub references: Vec<String>,
    /// Classification tags.
    pub tags: Vec<String>,
}
```

**Rationale for NSE-internal types:**
- Avoids circular dependency (`eggsec-nse` → `eggsec-output` → `eggsec-nse`)
- NSE evidence has domain-specific semantics (port, service, raw_excerpt) not in the generic `EvidenceItem`
- A separate bridge function (workstream 3) maps to eggsec-output `EvidenceItem` when needed

### Add `evidence` field to `NseRunReport`

```rust
pub struct NseRunReport {
    // ... existing fields ...
    /// Structured evidence items extracted from script output and runtime context.
    pub evidence: Vec<NseEvidenceItem>,
}
```

Add builder method:

```rust
pub fn with_evidence(mut self, evidence: Vec<NseEvidenceItem>) -> Self {
    self.evidence = evidence;
    self
}
```

Initialize to `Vec::new()` in `NseRunReport::new()` (empty is valid).

---

## Workstream 1: Evidence Field in `NseRunReport`

### Files to Modify

1. **`crates/eggsec-nse/Cargo.toml`** — Add `eggsec-core` and `eggsec-output` as dependencies (required for bridge to `ReportEnvelope`). Follow `eggsec-db-lab/Cargo.toml` pattern:
   ```toml
   eggsec-core = { path = "../eggsec-core" }
   eggsec-output = { path = "../eggsec-output" }
   ```

2. **`crates/eggsec-nse/src/report.rs`** — Add `NseEvidenceKind` enum, `NseEvidenceItem` struct, `with_evidence()` builder method, update `new()` to include `evidence: Vec::new()`.

3. **`crates/eggsec-nse/src/lib.rs`** (line 311-317) — Add re-exports for `NseEvidenceKind` and `NseEvidenceItem` in the `#[cfg(feature = "nse")]` report re-export block.

4. **`crates/eggsec-nse/src/report.rs`** — Add `extract_evidence()` free function that takes report state and returns `Vec<NseEvidenceItem>`.

### Acceptance Criteria

- [x] Existing JSON remains backward-compatible (new field with empty vec defaults).
- [x] Empty evidence is valid.
- [x] Evidence serializes deterministically.

---

## Workstream 2: Evidence Extraction Rules

### `extract_evidence()` Function

Add a new public function in `report.rs`:

```rust
/// Extract structured evidence items from report state.
///
/// Conservative extraction: only produces evidence from structured sources
/// (capability events, compatibility state, service context). No fragile prose parsing.
pub fn extract_evidence(
    target: &str,
    script_name: &str,
    capability_events: &[NseCapabilityEventSummary],
    compatibility: &NseCompatibilitySummary,
    rules: &[NseRuleEvaluationReport],
    output: &NseOutputSummary,
) -> Vec<NseEvidenceItem> {
    let mut evidence = Vec::new();
    let mut counter = 0u32;

    // 1. Capability denials → CapabilityDenial evidence
    for event in capability_events.iter().filter(|e| !e.allowed) {
        let id = format!("nse-ev-{}", counter);
        counter += 1;
        let target_detail = event.target.as_deref().unwrap_or(target);
        evidence.push(NseEvidenceItem {
            id,
            kind: NseEvidenceKind::CapabilityDenial,
            title: format!("{} denied by policy", event.kind),
            summary: event.reason.clone().unwrap_or_else(|| {
                format!("{} operation '{}' was denied", event.kind, event.operation)
            }),
            target: target_detail.to_string(),
            port: None,
            service: None,
            confidence: "confirmed".to_string(),
            source: script_name.to_string(),
            raw_excerpt: None,
            references: Vec::new(),
            tags: vec!["capability".to_string(), event.kind.clone()],
        });
    }

    // 2. Compatibility issues → CompatibilityWarning evidence
    for feature in &compatibility.unsupported_features {
        let id = format!("nse-ev-{}", counter);
        counter += 1;
        evidence.push(NseEvidenceItem {
            id,
            kind: NseEvidenceKind::CompatibilityWarning,
            title: format!("Unsupported feature: {}", feature),
            summary: format!("Module '{}' is not supported in this environment", feature),
            target: target.to_string(),
            port: None,
            service: None,
            confidence: "confirmed".to_string(),
            source: script_name.to_string(),
            raw_excerpt: None,
            references: Vec::new(),
            tags: vec!["compatibility".to_string(), "unsupported".to_string()],
        });
    }

    for approx in &compatibility.approximations {
        let id = format!("nse-ev-{}", counter);
        counter += 1;
        evidence.push(NseEvidenceItem {
            id,
            kind: NseEvidenceKind::CompatibilityWarning,
            title: format!("Approximate result: {}", approx),
            summary: format!("Rule evaluation is approximate: {}", approx),
            target: target.to_string(),
            port: None,
            service: None,
            confidence: "likely".to_string(),
            source: script_name.to_string(),
            raw_excerpt: None,
            references: Vec::new(),
            tags: vec!["compatibility".to_string(), "approximate".to_string()],
        });
    }

    // 3. Rule evaluation errors → CompatibilityWarning evidence
    for rule in rules.iter().filter(|r| r.error.is_some()) {
        let id = format!("nse-ev-{}", counter);
        counter += 1;
        evidence.push(NseEvidenceItem {
            id,
            kind: NseEvidenceKind::CompatibilityWarning,
            title: format!("Rule error: {}", rule.kind),
            summary: rule.error.clone().unwrap_or_default(),
            target: target.to_string(),
            port: None,
            service: None,
            confidence: "confirmed".to_string(),
            source: script_name.to_string(),
            raw_excerpt: None,
            references: Vec::new(),
            tags: vec!["rule-error".to_string(), rule.kind.clone()],
        });
    }

    // 4. Script output → ScriptOutput evidence (conservative: only if non-empty)
    if output.has_output && !output.content.is_empty() {
        let excerpt = if output.content.len() > 500 {
            format!("{}...", &output.content[..500])
        } else {
            output.content.clone()
        };
        let id = format!("nse-ev-{}", counter);
        evidence.push(NseEvidenceItem {
            id,
            kind: NseEvidenceKind::ScriptOutput,
            title: "Script output captured".to_string(),
            summary: format!("{} lines of output", output.line_count),
            target: target.to_string(),
            port: None,
            service: None,
            confidence: "confirmed".to_string(),
            source: script_name.to_string(),
            raw_excerpt: Some(excerpt),
            references: Vec::new(),
            tags: vec!["output".to_string()],
        });
    }

    evidence
}
```

### Extraction Rules (Conservative)

| Source | Evidence Kind | Confidence | Notes |
|--------|-------------|------------|-------|
| Capability denial event | `CapabilityDenial` | `"confirmed"` | Event is a factual record |
| Unsupported feature | `CompatibilityWarning` | `"confirmed"` | Resolver diagnostic is factual |
| Approximate rule result | `CompatibilityWarning` | `"likely"` | Rule evaluation was approximate |
| Rule evaluation error | `CompatibilityWarning` | `"confirmed"` | Error is factual |
| Script output (non-empty) | `ScriptOutput` | `"confirmed"` | Raw output capture, no parsing |

### What We Do NOT Extract (Yet)

- Vulnerability signals from prose (fragile, prone to false positives)
- Service fingerprints from structured output (would need Lua output table parsing)
- TLS certificate info (would need sslcert module integration)
- HTTP title/header info (would need http module integration)

These are deferred to future phases when structured Lua output table parsing is available.

### Acceptance Criteria

- [x] Evidence extraction is deterministic for corpus fixtures.
- [x] Raw output remains available for manual review.
- [x] Evidence confidence is explicit.
- [x] No fragile prose parsing.

---

## Workstream 3: Bridge to `ReportEnvelope`

### Bridge Function

Add a new module `crates/eggsec-nse/src/bridge.rs` (feature-gated on `nse`):

```rust
//! Bridge from NseRunReport to normalized ReportEnvelope.
//!
//! Maps NSE evidence items and report metadata into the eggsec-output
//! normalized report envelope for cross-domain report integration.

use crate::report::NseRunReport;
use eggsec_core::types::Severity;
use eggsec_output::envelope::{
    EvidenceItem as OutputEvidenceItem, EvidenceKind as OutputEvidenceKind, EvidenceSource,
    FindingRecord, RedactionState, ReportEnvelope, ToolMetadata,
};

/// Map NseEvidenceKind to eggsec-output EvidenceKind.
fn evidence_kind_to_output(kind: &crate::report::NseEvidenceKind) -> OutputEvidenceKind {
    match kind {
        crate::report::NseEvidenceKind::ServiceFingerprint => OutputEvidenceKind::Banner,
        crate::report::NseEvidenceKind::VersionInfo => OutputEvidenceKind::Banner,
        crate::report::NseEvidenceKind::CertificateInfo => OutputEvidenceKind::Certificate,
        crate::report::NseEvidenceKind::VulnerabilitySignal => OutputEvidenceKind::Generic,
        crate::report::NseEvidenceKind::Misconfiguration => OutputEvidenceKind::Generic,
        crate::report::NseEvidenceKind::CapabilityDenial => OutputEvidenceKind::RuntimeInstrumentation,
        crate::report::NseEvidenceKind::CompatibilityWarning => OutputEvidenceKind::LogLine,
        crate::report::NseEvidenceKind::ScriptOutput => OutputEvidenceKind::Generic,
    }
}

/// Map NseEvidenceKind to severity.
fn evidence_kind_to_severity(kind: &crate::report::NseEvidenceKind) -> Severity {
    match kind {
        crate::report::NseEvidenceKind::VulnerabilitySignal => Severity::Medium,
        crate::report::NseEvidenceKind::Misconfiguration => Severity::Medium,
        crate::report::NseEvidenceKind::CapabilityDenial => Severity::Info,
        crate::report::NseEvidenceKind::CompatibilityWarning => Severity::Info,
        _ => Severity::Info,
    }
}

/// Convert an NseRunReport into the normalized ReportEnvelope.
pub fn to_report_envelope(report: &NseRunReport) -> ReportEnvelope {
    let mut findings: Vec<FindingRecord> = Vec::new();

    // Map evidence items into findings
    for (i, ev) in report.evidence.iter().enumerate() {
        let finding_id = format!("nse-{}-{}", report.script_name, i);
        let severity = evidence_kind_to_severity(&ev.kind);
        let mut record = FindingRecord::new(
            &finding_id,
            "nse",
            &report.script_name,
            severity,
            &ev.title,
            &ev.summary,
        )
        .with_category(format!("nse-{}", ev.kind))
        .with_location(&report.target);

        let output_ev = OutputEvidenceItem::new(
            format!("{}-ev-0", finding_id),
            evidence_kind_to_output(&ev.kind),
            EvidenceSource {
                tool: "eggsec-nse".to_string(),
                module: Some(report.script_name.clone()),
                run_id: None,
            },
            &ev.summary,
        )
        .with_redaction(RedactionState::None);

        record = record.with_evidence(output_ev);

        for reference in &ev.references {
            record = record.with_reference(reference);
        }

        findings.push(record);
    }

    // Add execution metadata as info finding
    let metadata_finding = FindingRecord::new(
        "metadata-nse",
        "nse",
        &report.script_name,
        Severity::Info,
        "NSE execution metadata",
        format!(
            "target={} script={} status={} fidelity={} elapsed_secs={:.2}",
            report.target, report.script_name, report.compatibility.status,
            report.compatibility.fidelity, report.stats.elapsed_secs,
        ),
    )
    .with_category("nse-info")
    .with_location(&report.target);
    findings.push(metadata_finding);

    let mut envelope = ReportEnvelope::new(&report.script_name)
        .with_domain_id("nse")
        .with_target(&report.target)
        .with_tool_metadata(ToolMetadata {
            tool_name: "eggsec-nse".to_string(),
            tool_version: None,
            eggsec_version: None,
        });

    for finding in findings {
        envelope = envelope.with_finding(finding);
    }

    envelope.refresh_evidence_manifest();
    envelope
}
```

### Files to Modify

1. **`crates/eggsec-nse/src/bridge.rs`** — New file with `to_report_envelope()` function.
2. **`crates/eggsec-nse/src/lib.rs`** — Add `pub mod bridge;` gated on `#[cfg(feature = "nse")]`.
3. **`crates/eggsec-db-lab/Cargo.toml`** — No change needed (bridge is in eggsec-nse, not db-lab).

### Acceptance Criteria

- [x] Evidence and findings are not conflated.
- [x] Capability/report limitations do not become target vulnerabilities.
- [x] Bridge follows db-pentest pattern.

---

## Workstream 4: Corpus Tests

### New Test File

Create `crates/eggsec-nse/tests/evidence_tests.rs`:

Tests to write:

1. **`evidence_extraction_empty_report`** — Empty report produces empty evidence vec.
2. **`evidence_extraction_capability_denial`** — Denied capability events produce `CapabilityDenial` evidence.
3. **`evidence_extraction_compatibility_warning`** — Unsupported features produce `CompatibilityWarning` evidence.
4. **`evidence_extraction_approximate_rules`** — Approximate rules produce `CompatibilityWarning` evidence.
5. **`evidence_extraction_rule_error`** — Rule errors produce `CompatibilityWarning` evidence.
6. **`evidence_extraction_script_output`** — Non-empty output produces `ScriptOutput` evidence.
7. **`evidence_extraction_combined`** — Multiple sources produce correct evidence counts and kinds.
8. **`evidence_serialization_roundtrip`** — Evidence serializes/deserializes correctly.
9. **`evidence_builder_with_evidence`** — Builder method works correctly.
10. **`bridge_to_envelope_basic`** — `to_report_envelope()` produces valid envelope.
11. **`bridge_to_envelope_empty_evidence`** — Empty evidence produces envelope with only metadata finding.
12. **`evidence_confidence_values`** — All confidence values are valid strings.

### Test Pattern

```rust
#[cfg(feature = "nse")]
#[test]
fn evidence_extraction_capability_denial() {
    use eggsec_nse::report::*;

    let events = vec![
        NseCapabilityEventSummary {
            kind: "process_exec".to_string(),
            operation: "io.popen".to_string(),
            target: Some("ls".to_string()),
            allowed: false,
            reason: Some("denied by AgentSafe policy".to_string()),
        },
    ];

    let evidence = extract_evidence(
        "192.168.1.1",
        "test_script",
        &events,
        &NseCompatibilitySummary { ... },
        &[],
        &NseOutputSummary { ... },
    );

    assert_eq!(evidence.len(), 1);
    assert_eq!(evidence[0].kind, NseEvidenceKind::CapabilityDenial);
    assert_eq!(evidence[0].confidence, "confirmed");
    assert!(evidence[0].tags.contains(&"capability".to_string()));
}
```

### Acceptance Criteria

- [x] Evidence item count/kind/confidence is tested.
- [x] Tests do not depend on exact full JSON formatting.
- [x] All 12 tests pass.

---

## Workstream 5: Report Compatibility

### Changes to `compute_compatibility()`

No changes needed — `compute_compatibility()` already works correctly with the new evidence field. Evidence extraction is independent of compatibility computation.

### Changes to `NseRunReport::new()`

Add `evidence: Vec::new()` to the constructor. This is a one-line change in `report.rs` line 319-361.

### Changes to `run_cli_with_profile()`

In `lib.rs` line 466-474, after building the report, call `extract_evidence()` and chain `.with_evidence(evidence)`:

```rust
let evidence = crate::report::extract_evidence(
    &config.target,
    &config.script,
    &report.capability_events,
    &report.compatibility,
    &report.rules,
    &report.output,
);

let report = crate::report::NseRunReport::new(&config.target, &config.script)
    .with_profile(&report_profile)
    .with_script_source(&script_source)
    .with_resolver_diagnostics(&diagnostics)
    .with_libraries(library_reports)
    .with_rules(rule_reports)
    .with_capability_events(capability_events)
    .with_output(&output)
    .with_evidence(evidence)
    .compute_compatibility();
```

### Changes to `build_failure_report()`

No change needed — failure reports produce empty evidence (the default).

### Acceptance Criteria

- [x] A run may be `Partial` and still produce useful evidence.
- [x] Evidence does not interfere with existing compatibility logic.

---

## Workstream 6: Docs

### Files to Update

1. **`architecture/nse_integration.md`** — Add Phase 04 section after Phase 03 (context fidelity) block, documenting evidence model, extraction rules, and bridge.

2. **`docs/REPORT_EVIDENCE_MODEL.md`** — Add NSE as a domain bridge example, noting the internal types and external bridge.

3. **`crates/eggsec-nse/src/report.rs`** — Doc comments on `NseEvidenceKind`, `NseEvidenceItem`, and `extract_evidence()`.

### Acceptance Criteria

- [x] Evidence kinds documented.
- [x] Confidence meanings documented.
- [x] Difference between evidence, finding, raw output, and compatibility warning explained.
- [x] Limitations of script-derived evidence documented.

---

## Implementation Order

1. **Cargo.toml** — Add `eggsec-core` and `eggsec-output` path dependencies.
2. **report.rs** — Add `NseEvidenceKind`, `NseEvidenceItem`, `extract_evidence()`, update `NseRunReport::new()` and add `with_evidence()`.
3. **lib.rs** — Add re-exports for new types.
4. **bridge.rs** — New file with `to_report_envelope()`.
5. **lib.rs** — Add `pub mod bridge;`.
6. **lib.rs** — Wire `extract_evidence()` into `run_cli_with_profile()`.
7. **evidence_tests.rs** — New test file with 12 tests.
8. **Architecture docs** — Phase 04 section in `nse_integration.md`.
9. **Report evidence model docs** — NSE bridge reference in `REPORT_EVIDENCE_MODEL.md`.

## Verification

```bash
cargo test -p eggsec-nse --features nse report
cargo test -p eggsec-nse --features nse evidence
cargo test -p eggsec-nse --features nse compatibility_corpus
cargo check -p eggsec --features nse
cargo fmt --all --check
cargo clippy --lib -p eggsec-nse --features nse
bash scripts/check-architecture-guards.sh
```

## Final Acceptance Criteria

Phase 04 is complete when:

- `NseRunReport` can include structured evidence.
- Evidence extraction is conservative, deterministic, and tested.
- Raw output remains present.
- Findings integration does not overclaim weak signals.
- Bridge to `ReportEnvelope` follows existing db-pentest pattern.
- Docs explain evidence semantics and limitations.
