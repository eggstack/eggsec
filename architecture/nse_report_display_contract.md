# NSE Report Display Contract

This document defines the structured display model for TUI and future frontends rendering NSE run results. It maps `NseRunReport` fields to display sections without forcing immediate implementation.

## Data Sources

Two types are valid input for rendering:

| Type | Source | When to Use |
|------|--------|-------------|
| `NseRunReport` | `report.rs` | Direct rendering when envelope bridge is unnecessary |
| `ReportEnvelope` | `bridge.rs` | Cross-domain report aggregation; wraps `NseRunReport` findings |

The TUI may accept either type. `NseRunReport` provides richer per-section detail; `ReportEnvelope` provides normalized findings and evidence with redaction metadata.

## Display Sections

### 1. Summary Panel

| Field | Source | Display |
|-------|--------|---------|
| Target | `NseRunReport.target` | Host/IP string |
| Script | `NseRunReport.script_name` | Script filename |
| Source | `NseRunReport.script_source.label` + `.kind` | e.g. "builtin-module-require (Builtin)" |
| Profile | `NseRunReport.profile.kind` | e.g. "AgentSafe", "ManualPermissive" |
| Elapsed | `NseRunReport.stats.elapsed_secs` | Formatted as `{:.2}s` |
| Status | `NseRunReport.compatibility.status` | UPPERCASE label (Compatible, Partial, Failed, etc.) |
| Fidelity | `NseRunReport.compatibility.fidelity` | `~` prefix for non-Full (e.g. `~approximate`) |

### 2. Rule Panel

**Source**: `NseRunReport.rules: Vec<NseRuleEvaluationReport>`

| Field | Source | Display |
|-------|--------|---------|
| Kind | `.kind` | Label (portrule, hostrule, prerule, postrule) |
| Matched | `.matched` | Boolean badge or icon |
| Evaluated | `.evaluated` | Whether the rule was evaluated |
| Exactness | `.exactness` | "exact" or "unsupported" |
| Summary | `.summary` | Free text description |
| Unsupported | `.unsupported` | Optional unsupported return type info |
| Context Source | `.host_port_context_source` | Provenance: Scan, Fixture, Synthetic, Unknown |
| Fidelity Reason | `.fidelity_reason` | Why fidelity was downgraded (if applicable) |

**Empty state**: If `rules` is empty, display "No rules evaluated."

### 3. Libraries Panel

**Source**: `NseRunReport.libraries: Vec<NseLibraryUseReport>`

| Field | Source | Display |
|-------|--------|---------|
| Name | `.name` | Library identifier |
| Category | `.category` | Core, Protocol, Utility, Exploit, Auth |
| Status | `.loaded` / `.registered` | "loaded" if `loaded=true`, "registered" if `registered=true`, else "unregistered" |
| Side Effects | `.side_effects` | List (e.g. "NetworkAccess", "FileSystemRead") |
| Warnings | `.warnings` | Per-library warnings, prefixed with `[*]` |
| Fallback | `.fallback_behavior` | HardFail, GracefulDegrade, Skip |
| Notes | `.notes` | Additional metadata |

**Empty state**: If `libraries` is empty, display "No libraries loaded."

### 4. Capability Denials Panel

**Source**: `NseRunReport.capability_events: Vec<NseCapabilityEventSummary>`, filtered to `allowed == false`

| Field | Source | Display |
|-------|--------|---------|
| Kind | `.kind` | Operation class (e.g. "filesystem_write", "process_exec") |
| Operation | `.operation` | Specific helper (e.g. "io.write", "os.execute") |
| Target | `.target` | Path/host/command (optional) |
| Allowed | `.allowed` | Always `false` in this panel |
| Reason | `.reason` | Denial reason string, default "denied by policy" |

**Visual treatment**: `[!]` prefix for denials, color-coded as error/warning.

**Empty state**: If no denials, display "No capability denials."

### 5. Evidence Panel

**Source**: `NseRunReport.evidence: Vec<NseEvidenceItem>`

| Field | Source | Display |
|-------|--------|---------|
| Kind | `.kind` | Category label (service-fingerprint, version-info, etc.) |
| Title | `.title` | Short observation title |
| Summary | `.summary` | Detailed observation text |
| Confidence | `.confidence` | "confirmed", "likely", "possible", "low" |
| Target | `.target` | Host/port string |
| Port | `.port` | Optional port number |
| Service | `.service` | Optional service name |
| Source Module | `.source` | Module/script that produced evidence |
| Raw Excerpt | `.raw_excerpt` | Optional raw output excerpt |
| References | `.references` | CWE, CVE, URLs |
| Tags | `.tags` | Classification tags |

**Visual treatment**: Confidence-based coloring (confirmed > likely > possible > low). CapabilityDenial evidence items are informational, not vulnerabilities.

**Empty state**: If `evidence` is empty, display "No structured evidence."

### 6. Raw Output Panel

**Source**: `NseRunReport.output.content`

| Field | Source | Display |
|-------|--------|---------|
| Content | `.content` | Full script stdout/stderr |
| Truncated | `.truncated` | Whether output was truncated |
| Truncation Reason | `.truncation_reason` | Why output was truncated (if applicable) |

**Rendering**: Full content in scrollable panel. Truncation indicator when `.truncated == true`.

### 7. Diagnostics Panel

**Source**: Multiple `NseRunReport` fields

| Sub-section | Source | Content |
|-------------|--------|---------|
| Resolver | `NseRunReport.resolver` | Resolution summary and diagnostics |
| Errors | `NseRunReport.errors` | Error messages (prefixed with `-`) |
| Warnings | `NseRunReport.warnings` | Warning messages (prefixed with `[*]`) |
| Unsupported | `NseRunReport.compatibility.unsupported_features` | Unsupported feature list |
| Approximations | `NseRunReport.compatibility.approximations` | Approximation list |

## ReportEnvelope Mapping

When rendering from `ReportEnvelope` instead of `NseRunReport`:

| Display Section | ReportEnvelope Source |
|-----------------|----------------------|
| Summary | `envelope.target`, `envelope.tool_metadata`, `envelope.domain_id` |
| Findings | `envelope.findings: Vec<FindingRecord>` |
| Evidence | `envelope.evidence_manifest` + individual `EvidenceItem`s |
| Policy | `envelope.policy_summary` |
| Baseline | `envelope.baseline_summary` |

`ReportEnvelope` findings use `eggsec-output` normalized types (`FindingRecord`, `EvidenceItem`, `Severity`). The NSE bridge (`bridge.rs`) maps `NseEvidenceItem` → `FindingRecord` + `EvidenceItem`. TUI rendering should use `NseRunReport` for section-level detail and `ReportEnvelope` for cross-domain aggregation.

## Color/Semantic Mapping

| Status | Color | Prefix |
|--------|-------|--------|
| Compatible/Full | Green | (none) |
| CompatibleWithWarnings | Yellow | `[*]` |
| Partial | Yellow | (none) |
| Unsupported | Red | (none) |
| Failed | Red | `-` |
| Denial | Red | `[!]` |
| Approximate fidelity | Cyan | `~` |

## Conventions

- All sections are optional; render only non-empty sections.
- Empty states use descriptive text, not "N/A".
- Evidence confidence levels determine visual prominence, not severity.
- Capability denials are execution limitations, not target vulnerabilities.
- Raw output is always available separately from evidence items.
- The TUI consumes structured report fields from `NseRunReport` — it does not parse human-formatted text.
- Raw output is rendered separately from evidence items and is never treated as evidence.

## TUI Implementation (Phase 01)

### Data Flow

The NSE dispatch layer (`run_nse()` in `crates/eggsec/src/dispatch/api.rs`) builds an `NseRunReport` after `run_script_with_rules()` completes. The report is carried through `NseResults { report: Option<NseRunReport>, output: String, ... }` into the TUI rendering path.

The TUI consumes `NseRunReport` directly via `nse_report_view::render_report()` in `crates/eggsec-tui/src/tabs/nse_report_view.rs`. When `NseRunReport` is absent (e.g. legacy dispatch path or parse failure), the TUI falls back to simple text rendering of the raw output string.

### Report Construction

```
run_script_with_rules() → NseExecutor::build_report() → NseRunReport
         ↓
   run_nse() stores report in NseResults
         ↓
   TUI receives NseResults via progress channel
         ↓
   render_report() maps NseRunReport → styled ratatui::text::Lines
```

### View Model

`render_report()` maps all 7 display sections defined in this contract to styled `ratatui::text::Line` values:

| Section | Mapping |
|---------|---------|
| Summary | Target, script, source, profile, elapsed, status, fidelity |
| Rule Evaluation | Per-rule kind/matched/exactness/summary with context source |
| Libraries | Per-library name/category/status/side-effects/warnings |
| Capability Denials | Filtered `capability_events` where `allowed == false`, prefixed `[!]` |
| Evidence | Per-evidence kind/title/summary/confidence/target/references |
| Raw Output | `output.content` with truncation indicator |
| Diagnostics | Resolver, errors, warnings, unsupported features, approximations |

### Visual Treatment

- **Capability denials** are prefixed with `[!]` and colored as errors. They represent execution limitations enforced by the profile, not vulnerabilities found on the target.
- **Evidence items** and **raw output** are rendered as separate, visually distinct sections. Evidence is structured observation data; raw output is the script's stdout/stderr. They are never conflated.
- **Empty sections** render their descriptive empty-state text ("No rules evaluated.", "No libraries loaded.", etc.) and are collapsible.

### Test Coverage

Tests in `crates/eggsec-tui/src/tabs/nse_report_view.rs` cover:

- **Compatible report**: All 7 sections populated, full display.
- **Denied report**: Capability denials present, denials panel populated with `[!]` prefixed lines.
- **Empty report**: All optional sections empty, empty-state text rendered.
- **Partial report**: Some sections present (e.g. rules + evidence), others absent — only populated sections rendered.
