# Vuln Module — Deep Dive

## Purpose

Vulnerability management and prioritization using CVSS 3.1 scoring, exploitability assessment, asset criticality, and risk-based triage with remediation guidance. Provides the `VulnAssessment` aggregate type consumed by the TUI Vuln tab, CLI `vuln` subcommands, the pipeline `Stage::Vuln`, and the dispatch `TaskKind::Vuln`.

## Location & Feature Gating

| Crate | Module path | Feature gate | lib.rs lines | Visibility |
|-------|-------------|-------------|--------------|------------|
| `eggsec` | `vuln/` | `vuln-management` | `lib.rs:147-151` | `pub mod` when enabled, `mod` (dead_code) when disabled |

The `vuln-management` feature is a **marker-level dependency** — no external system dependencies required. It is included in the `rest-api` and `full-no-system` feature profiles.

## Files

| File | Lines | Purpose |
|------|-------|---------|
| `mod.rs` | 63 | Module root: `VulnAssessment` struct, re-exports of all sub-module public types |
| `cvss.rs` | 454 | CVSS 3.1 vector parsing, base/temporal/environmental score computation, severity mapping |
| `exploit.rs` | 127 | CVE year-based exploitability heuristic, `ExploitInfo::exploit_pipeline_score()` |
| `asset.rs` | 104 | Asset criticality scoring with builder pattern, weighted composite formula |
| `prioritizer.rs` | 200 | `RiskScore` weighted combination, `PrioritizedFinding` sorting, `prioritize_findings()` |
| `triage.rs` | 144 | Keyword-based triage: duplicate, false-positive, true-positive, needs-review classification |
| `remediation.rs` | 181 | Severity-based remediation plans: effort estimates, step lists, priority mapping |

**Total: 7 files, 1,273 lines (including tests).**

## Architecture

### Type Inventory

| Type | File:line | Fields / Variants | Role |
|------|-----------|-------------------|------|
| `VulnAssessment` | `mod.rs:37` | `mode`, `assessed_at`, `cvss_score`, `exploit_info`, `asset_criticality`, `prioritized_findings`, `triage_results`, `remediation_plans`, `summary` | Top-level aggregate |
| `CvssScore` | `cvss.rs:4` | `base_score: f32`, `temporal_score: f32`, `environmental_score: f32`, `vector: String` | CVSS 3.1 score container |
| `ParsedVector` | `cvss.rs:189` | 22 fields: 8 base + 3 temporal + 4 environmental requirement + 7 modified | Internal vector representation |
| `ExploitInfo` | `exploit.rs:4` | `cve_id`, `has_public_exploit`, `exploit_db_id`, `metasploit_module`, `in_cisa_kev`, `is_actively_exploited`, `exploit_score` | Exploit availability |
| `AssetCriticality` | `asset.rs:3` | `asset_id`, `technology_score`, `environment_score`, `data_sensitivity`, `user_base`, `overall_score` | Asset risk scoring |
| `RiskScore` | `prioritizer.rs:7` | `cvss_score`, `exploitability_score`, `asset_criticality`, `combined_score`, `priority_level` | Combined risk |
| `PriorityLevel` | `prioritizer.rs:16` | `P0`, `P1`, `P2`, `P3` (ordered) | Priority classification |
| `PrioritizedFinding` | `prioritizer.rs:80` | `finding_id`, `title`, `severity`, `risk_score`, `exploit_info`, `asset_criticality`, `priority_rank` | Ranked finding |
| `TriageResult` | `triage.rs:4` | `finding_id`, `triage_status`, `confidence: f32`, `reason` | Triage decision |
| `TriageStatus` | `triage.rs:13` | `New`, `TruePositive`, `FalsePositive`, `NeedsReview`, `Duplicate` | 5-variant triage enum |
| `Remediation` | `remediation.rs:5` | `finding_id`, `title`, `severity`, `effort_hours: f32`, `steps`, `references`, `priority` | Remediation plan |
| `RemediationPriority` | `remediation.rs:16` | `Critical`, `High`, `Medium`, `Low` (ordered) | Remediation urgency |

### CVSS 3.1 Implementation

#### Vector Parsing (`cvss.rs:214-275`)

The parser splits the vector string on `/`, then on `:` for each key-value pair. It populates a `ParsedVector` with defaults (base: `AV:N/AC:L/PR:N/UI:N/S:U/C:N/I:N/A:N`, temporal: `E:X/RL:X/RC:X`, environmental: `CR:X/IR:X/AR:X/MAV:X/MAC:X/MPR:X/MUI:X/MS:X/MC:X/MI:X/MA:X`). Unknown keys are silently ignored.

#### Base Score Calculation (`cvss.rs:277-308`)

The implementation follows the CVSS 3.1 specification exactly:

1. **Impact Sub Score (ISS)**: `1 - ((1-C) × (1-I) × (1-A))` where C/I/A are CIA weights (`cvss.rs:287`)
2. **Scope Unchanged** (`S:U`): `Impact = 6.42 × ISS`, `Exploitability = 8.22 × AV × AC × PR × UI` (`cvss.rs:289-297`)
3. **Scope Changed** (`S:C`): `Impact = 7.52 × (ISS - 0.029) - 3.25 × (ISS - 0.02)^15`, `Exploitability = 8.22 × AV × AC × PR × UI` (`cvss.rs:298-307`)
4. **Rounding**: `ceil(score × 10) / 10` (CVSS "Roundup" function) (`cvss.rs:83,92,296,305`)

The `calculate_base()` method (`cvss.rs:55-95`) is a public static for direct base-score computation without full vector parsing.

#### Temporal Score (`cvss.rs:310-318`)

`Temporal = Base × E × RL × RC`, where weights are:

| Metric | H | F | P | U | C | R | X |
|--------|---|---|---|---|---|---|---|
| E (Exploit Code Maturity) | 0.91 | 0.97 | 0.94 | 1.0 | — | — | 1.0 |
| RL (Remediation Level) | — | — | — | 0.95 | — | — | 1.0 |
| RC (Report Confidence) | — | — | — | — | 0.96 | 1.0 | 1.0 |

#### Environmental Score (`cvss.rs:320-363`)

Modified CIA impact sub-scores use `cia_requirement_weight` (H=1.5, M=1.0, L=0.5) multiplied by the base/modified CIA weight, clamped to 1.0 (`cvss.rs:365-369`). Modified base metrics override AV/AC/PR/UI/S when `MX != X`. The environmental score applies temporal weights on top.

#### Severity Mapping (`cvss.rs:32-44`)

| Base Score Range | Severity |
|-----------------|----------|
| ≤ 0.0 | NONE |
| 0.1 – 3.9 | LOW |
| 4.0 – 6.9 | MEDIUM |
| 7.0 – 8.9 | HIGH |
| ≥ 9.0 | CRITICAL |

#### Base Metric Weights

| Metric | N | A | L | P | H | L/U | R |
|--------|---|---|---|---|---|-----|---|
| AV (Attack Vector) | 0.85 | 0.62 | 0.55 | 0.20 | — | — | — |
| AC (Attack Complexity) | — | — | 0.77 | — | 0.44 | — | — |
| PR (Scope Unchanged) | 0.85 | — | 0.62 | 0.27 | — | — | — |
| PR (Scope Changed) | 0.85 | — | 0.68 | 0.50 | — | — | — |
| UI (User Interaction) | — | — | — | — | — | 0.85/0.62 | — |
| C/I/A (CIA) | — | — | — | — | 0.56 | 0.22 | 0.0 |

### Exploitability Assessment (`exploit.rs`)

The `ExploitInfo::assess()` method (`exploit.rs:16-40`) uses a **CVE year heuristic** — no external database lookups:

| CVE Year | `has_public_exploit` | `in_cisa_kev` | `exploit_score` |
|----------|---------------------|--------------|-----------------|
| ≤ 2010 | `true` | `true` | 10.0 |
| 2011–2015 | `true` | `false` | 7.5 |
| ≥ 2016 | `false` | `false` | 2.5 |

`CVE-YYYY` parsing (`exploit.rs:63-74`) extracts exactly 4 digits after `CVE-`, validates range 1999–2099, returns `None` for invalid IDs.

`exploit_pipeline_score()` (`exploit.rs:54-60`) returns `Some(exploit_score)` only when `has_public_exploit` is `true`, `None` otherwise.

### Asset Criticality (`asset.rs`)

#### Builder Pattern

`AssetCriticality::new()` sets all scores to 5.0 (`asset.rs:14-23`). Builder methods (`with_technology`, `with_environment`, `with_data_sensitivity`, `with_user_base`) clamp inputs to [0.0, 10.0] and recalculate (`asset.rs:25-47`).

#### Weighted Formula (`asset.rs:58-62`)

```
overall = technology × 0.30 + environment × 0.25 + data_sensitivity × 0.30 + user_base × 0.15
```

Capped at 10.0.

#### Preset Asset Types (`asset.rs:66-76`)

| `asset_type` | technology | environment | data_sensitivity | user_base |
|--------------|-----------|-------------|-----------------|-----------|
| `"database"` | 9.0 | 5.0 | 10.0 | 5.0 |
| `"web_server"` | 7.0 | 8.0 | 5.0 | 5.0 |
| `"api"` | 8.0 | 5.0 | 7.0 | 5.0 |
| `"workstation"` | 4.0 | 5.0 | 5.0 | 5.0 |
| other | 5.0 | 5.0 | 5.0 | 5.0 |

### Risk Prioritization (`prioritizer.rs`)

#### Weighted Formula (`prioritizer.rs:48`)

```
combined = cvss × 0.4 + exploitability × 0.3 + asset_criticality × 0.3
```

#### Priority Level Assignment (`prioritizer.rs:50-55`)

| CVSS Score | Priority |
|-----------|----------|
| ≥ 9.0 | P0 |
| ≥ 7.0 | P1 |
| ≥ 4.0 | P2 |
| < 4.0 | P3 |

`PriorityLevel` implements `Ord` (`prioritizer.rs:29-33`) mapping P0→4, P1→3, P2→2, P3→1 for sorting.

#### `prioritize_findings()` (`prioritizer.rs:108-139`)

Accepts `&[(String, String, Severity, Option<f32>)]` tuples (id, title, severity, optional CVSS). When CVSS is `None`, defaults are assigned:

| Severity | Default CVSS |
|----------|-------------|
| Critical | 9.0 |
| High | 7.5 |
| Medium | 5.0 |
| Low | 2.5 |
| Info | 0.1 |

Exploitability and asset criticality default to 5.0. Results are sorted by `combined_score` descending (`prioritizer.rs:91-105`).

### Triage (`triage.rs`)

#### Keyword Classification (`triage.rs:43-95`)

Triage is keyword-based on `title` and `description` (lowercased):

| Priority | Keywords | Status | Confidence |
|----------|---------|--------|-----------|
| 1 | `"example"`, `"demo"`, `"sample"`, `"localhost"` | `Duplicate` | 0.95 |
| 2 | `"informational"`, `"no risk"`, `"not vulnerable"`, `"safe"`, `"no impact"` | `FalsePositive` | 0.85 |
| 3 | CVSS ≥ 9.0 | `TruePositive` | 0.99 |
| 4 | Other / no CVSS | `NeedsReview` | 0.50 |

### Remediation (`remediation.rs`)

#### Severity-to-Plan Mapping (`remediation.rs:48-101`)

| Severity | Effort (hours) | Priority | Steps | References |
|----------|---------------|----------|-------|------------|
| Critical | 24.0 | Critical | 4 (isolate, patch, verify, monitor) | CVE Database, Vendor Advisory |
| High | 16.0 | High | 4 (plan, test, patch, verify) | CVE Database, OWASP |
| Medium | 8.0 | Medium | 4 (schedule, develop, deploy, document) | Security Best Practices |
| Low | 4.0 | Low | 3 (review, plan, implement) | — |
| Info | 0.0 | Low | 1 (no action) | — |

`RemediationPriority` implements `Ord` (`remediation.rs:29-33`) mapping Critical→4, High→3, Medium→2, Low→1.

## Behavior / Flow

### CVSS Score Calculation Flow

```
Input: "CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H"
  → parse_vector()         [cvss.rs:214]  → ParsedVector (22 fields)
  → compute_base_score()   [cvss.rs:277]  → f32 (9.8)
  → compute_temporal_score() [cvss.rs:310] → f32 (base × E × RL × RC)
  → compute_environmental_score() [cvss.rs:320] → f32 (modified base × temporal)
  → severity()             [cvss.rs:32]   → "CRITICAL"
```

### Assessment Pipeline Flow

```
VulnAssessment::new(mode)            [mod.rs:50]
  → CvssScore::from_vector(vector)   [cvss.rs:13]     (optional)
  → ExploitInfo::assess(cve_id)      [exploit.rs:16]  (optional)
  → assess_asset(target, asset_type)  [asset.rs:66]    (optional)
  → prioritize_findings(findings)     [prioritizer.rs:108]
  → triage_finding(id, title, ...)    [triage.rs:36]   (per finding)
  → Remediation::for_finding(...)     [remediation.rs:47] (per finding)
```

## Public API

| Function / Method | Signature | Location |
|-------------------|-----------|----------|
| `CvssScore::from_vector` | `fn(vector: &str) -> Result<Self>` | `cvss.rs:13` |
| `CvssScore::calculate_base` | `fn(av, ac, pr, ui, scope, c, i, a) -> f32` | `cvss.rs:55` |
| `CvssScore::base_score` | `fn(&self) -> f32` | `cvss.rs:28` |
| `CvssScore::severity` | `fn(&self) -> &'static str` | `cvss.rs:32` |
| `CvssScore::temporal_score` | `fn(&self) -> f32` | `cvss.rs:46` |
| `CvssScore::environmental_score` | `fn(&self) -> f32` | `cvss.rs:50` |
| `ExploitInfo::assess` | `fn(cve_id: &str) -> Self` | `exploit.rs:16` |
| `ExploitInfo::for_cve` | `fn(cve_id: &str) -> Result<Self>` | `exploit.rs:42` |
| `ExploitInfo::exploit_pipeline_score` | `fn(&self) -> Option<f32>` | `exploit.rs:54` |
| `check_exploitability` | `async fn(cve_id: &str) -> Result<ExploitInfo>` | `exploit.rs:76` |
| `AssetCriticality::new` | `fn(asset_id: &str) -> Self` | `asset.rs:14` |
| `AssetCriticality::with_*` | Builder methods → `Self` | `asset.rs:25-47` |
| `assess_asset` | `fn(asset_id: &str, asset_type: &str) -> AssetCriticality` | `asset.rs:66` |
| `RiskScore::calculate` | `fn(cvss, exploitability, asset_criticality) -> Self` | `prioritizer.rs:47` |
| `RiskScore::new` | `fn(cvss, exploitability, asset_criticality) -> Self` | `prioritizer.rs:66` |
| `PrioritizedFinding::prioritize` | `fn(Vec<Self>) -> Vec<Self>` (sorted) | `prioritizer.rs:91` |
| `prioritize_findings` | `fn(&[(id, title, severity, Option<cvss>)]) -> Vec<PrioritizedFinding>` | `prioritizer.rs:108` |
| `TriageResult::new` | `fn(Option<id>, TriageStatus) -> Self` | `triage.rs:22` |
| `triage_finding` | `fn(id, title, description, severity, Option<cvss>) -> TriageResult` | `triage.rs:36` |
| `Remediation::for_finding` | `fn(id, title, severity) -> Self` | `remediation.rs:47` |
| `Remediation::from_severity` | `fn(severity: &str) -> Self` | `remediation.rs:114` |

## Integration Points

### CLI (`cli/vuln.rs`, `commands/handlers/vuln.rs`)

- **Commands**: `eggsec vuln score <vector>`, `eggsec vuln exploitability <cve>`, `eggsec vuln prioritize`, `eggsec vuln triage`, `eggsec vuln remediate`
- **Dispatch**: `Commands::Vuln(args)` → `handle_vuln()` (`handlers/mod.rs:571`) → sub-handler (`handlers/vuln.rs:6-111`)
- **Output**: Plain-text println to stdout; no JSON output mode.

### Dispatch (`dispatch/security.rs:460-638`, `dispatch/mod.rs:307-321`)

- `TaskKind::Vuln(VulnParams)` → `run_vuln_task()` with mode strings: `"cvss_calc"`, `"exploit_check"`, `"asset_assess"`, `"prioritize"`, `"triage"`, `"remediation"`
- Result type: `TaskResult::Vuln(VulnAssessment)` (`dispatch/types.rs:142-143`)
- 120-second timeout wrapper (`dispatch/security.rs:477`)

### TUI (`crates/eggsec-tui/src/tabs/vuln.rs`)

- `VulnTab` with 6 modes: `CvssCalc`, `ExploitCheck`, `AssetAssess`, `Prioritize`, `Triage`, `Remediation`
- Three focus areas: `Mode`, `Inputs`, `Results`
- Feature-gated behind `vuln-management`
- Scan profile: `ScanProfile::Vuln` for pipeline integration (`tabs/scan.rs:64,127`)

### Pipeline (`pipeline/executor.rs:1173-1247`, `pipeline/context.rs:19`)

- `Stage::Vuln` (pipeline stage 5 of 7, at step offset 3: `executor.rs:244`)
- `run_vuln()` collects findings from pipeline context (interesting endpoints, HTTP/HTTPS services), runs `prioritize_findings()`, assesses target as `"web_server"`, stores `VulnAssessment` in `PipelineContext.vuln_assessment`
- `PipelineReport.vuln_assessment: Option<VulnAssessment>` (`pipeline/report.rs:26`)

### Python Bindings (`crates/eggsec-python/`)

- No direct `VulnAssessment` Python wrapper; vuln-management is engine-only. Python-facing operations use findings and severity types from `eggsec-core`.

## Data Model

```
VulnAssessment
├── cvss_score: Option<CvssScore>
│   ├── base_score: f32
│   ├── temporal_score: f32
│   ├── environmental_score: f32
│   └── vector: String
├── exploit_info: Option<ExploitInfo>
│   ├── cve_id: String
│   ├── has_public_exploit: bool
│   ├── in_cisa_kev: bool
│   ├── is_actively_exploited: bool
│   └── exploit_score: f32
├── asset_criticality: Option<AssetCriticality>
│   ├── asset_id: String
│   ├── technology_score: f32     (0.0–10.0)
│   ├── environment_score: f32   (0.0–10.0)
│   ├── data_sensitivity: f32    (0.0–10.0)
│   ├── user_base: f32           (0.0–10.0)
│   └── overall_score: f32       (weighted composite)
├── prioritized_findings: Vec<PrioritizedFinding>
│   └── risk_score: RiskScore
│       ├── combined_score: f32  (cvss×0.4 + exploit×0.3 + asset×0.3)
│       └── priority_level: PriorityLevel (P0/P1/P2/P3)
├── triage_results: Vec<TriageResult>
│   ├── triage_status: TriageStatus
│   └── confidence: f32
├── remediation_plans: Vec<Remediation>
│   ├── effort_hours: f32
│   ├── steps: Vec<String>
│   ├── references: Vec<String>
│   └── priority: RemediationPriority (Critical/High/Medium/Low)
└── summary: Vec<String>
```

## Testing

Each sub-module has inline `#[cfg(test)] mod tests`:

| Module | Tests | Key assertions |
|--------|-------|----------------|
| `cvss.rs` | 8 tests | NVD-calculated values (9.8 for full impact), scope-changed PR weight, zero-impact returns 0.0, round-up behavior, severity classification |
| `exploit.rs` | 5 tests | Year parsing, pre-2010 → KEV, pre-2015 → public exploit, recent → no exploit |
| `asset.rs` | 3 tests | Database criticality ≥ 7.0, builder composition, clamp behavior |
| `prioritizer.rs` | 3 tests | Sort order (critical > high > low), P0 > P1 > P2 > P3, stable sort |
| `triage.rs` | 3 tests | Duplicate keyword match, CVSS ≥ 9.0 → true positive, false positive keyword match |
| `remediation.rs` | 4 tests | Critical = 24h, Info = 0h, priority ordering, priority sort |

Total: **26 unit tests** across all sub-modules.

## Invariants & Gotchas

1. **No external CVE/KEV data**: `ExploitInfo::assess()` uses year-only heuristics. Pre-2015 = `has_public_exploit`, pre-2010 = `in_cisa_kev`. This is a known limitation — real exploit data requires external database integration.
2. **Default fallback in weight functions**: All `*_weight()` functions in `cvss.rs` return the "most permissive" default for unrecognized inputs (e.g., unknown `AV` → 0.85 as if Network). This is silent — no error returned for malformed vectors.
3. **`parse_vector` does not validate the CVSS prefix**: `CVSS:3.1/` prefix is not validated; the parser splits on `/` and `:` regardless of prefix format.
4. **Triage keyword matching is case-insensitive but naive**: Short keywords like `"safe"` or `"demo"` could false-match legitimate finding titles.
5. **`prioritize_findings` default CVSS for Info severity is 0.1**, not 0.0, to ensure Info findings still get a non-zero risk score.
6. **Asset criticality weights sum to 1.0** (0.30 + 0.25 + 0.30 + 0.15 = 1.00), so the overall score is in [0.0, 10.0].

## Bug Sweep

| Finding | File:line | Severity | Description |
|---------|-----------|----------|-------------|
| Weighted score formula uses `f32` | `prioritizer.rs:48` | Low | `cvss × 0.4 + exploitability × 0.3 + asset_criticality × 0.3` — all `f32` arithmetic; no division, no division-by-zero risk. Floating-point precision is acceptable for scoring. |
| `unwrap_or` in sort comparator | `prioritizer.rs:97` | None | `partial_cmp(...).unwrap_or(Ordering::Equal)` — correct handling of NaN floats, not a bug. |
| `parse_vector` defaults to permissive | `cvss.rs:104,113,122,131,138,147,157,167,177,187` | Low | Unknown metric values silently default to most permissive weight. By design, but could mask malformed vectors. |
| No panicking `unwrap()`/`expect()` in non-test code | — | None | All `unwrap()` calls are in tests or in `parse_cve_year()` which uses `.ok()` on parse results. |
| No silent error suppression | — | None | `Result` types propagate errors; no `let _ =` on fallible operations. |

**Confirmed bugs: 0.** No division-by-zero, no panicking unwraps, no silent error suppression in production code.

---

*Last verified against source: 2026-08-25*
