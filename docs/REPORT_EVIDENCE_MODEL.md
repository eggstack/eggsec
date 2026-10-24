# Report & Evidence Model

Comprehensive inventory of report, finding, and evidence types across the Eggsec workspace. Documents the current landscape, conversion bridges, known incompatibilities, and the target normalized model.

## Current Report Types

> Ownership note (Phase B, 2026-09-16): the shared data contracts below are
> canonically owned by `eggsec-report-model` (`crates/eggsec-report-model/src/`);
> `eggsec-output` re-exports them for backward compatibility, and domain crates
> depend on the model directly (guards Checks 118–120).

### eggsec-report-model Crate Types (canonical data contracts)

| Type | File | Purpose |
|------|------|---------|
| `ScanReportData` | `crates/eggsec-report-model/src/report.rs` | Main report data structure. Fields: target, scan_type, timestamp, findings, open_ports, services, duration_ms, wireless_networks, policy_summary. |
| `FindingData` | `crates/eggsec-report-model/src/report.rs` | Lightweight finding for serialization. Fields: title, severity (`String`), category, description, location, evidence (`Option<String>`), remediation (`Option<String>`), cwe_ids. |
| `ReportEnvelope` | `crates/eggsec-report-model/src/envelope.rs` | Normalized top-level report container (findings, evidence manifest, policy/baseline summaries, tool metadata). |
| `FindingRecord` | `crates/eggsec-report-model/src/envelope.rs` | Normalized finding record with evidence items, remediation, references, category, location. |
| `EvidenceItem` / `EvidenceManifest` / `EvidenceKind` / `EvidenceSource` | `crates/eggsec-report-model/src/envelope.rs` | Evidence taxonomy, provenance, manifests, redaction state/policy. |
| `BaselineSummary` / `ToolMetadata` | `crates/eggsec-report-model/src/envelope.rs` | Baseline comparison summary; tool/version metadata. |
| `PolicySummary` | `crates/eggsec-report-model/src/summary.rs` | Policy enforcement summary. Fields: operation_mode, max_risk, total_decisions, denied_count, warning_count, denied_reasons, warnings. |
| `DiffSummary` | `crates/eggsec-report-model/src/summary.rs` | Diff statistics. Fields: total_new, total_resolved, total_escalated, total_deescalated, net_change. |

### eggsec-output Crate Types (rendering/analysis over the model)

| Type | File | Purpose |
|------|------|---------|
| `AgentFinding` | `crates/eggsec-output/src/agent.rs` | Rich finding type. Fields: id, tool_id, vulnerability_type, severity (`Severity`), title, description, evidence (`Evidence`), remediation (`Remediation`), confidence, cvss, cwe_ids, target, endpoint, parameter, timestamp, attack_surface, status. |
| `Evidence` | `crates/eggsec-output/src/agent.rs` | Agent evidence struct. Fields: request, response_snippet, diff_indicator, matched_pattern, timing_ms, status_code. |
| `Remediation` | `crates/eggsec-output/src/agent.rs` | Agent remediation struct. Fields: summary, references, code_example, priority, effort. |
| `Confidence` | `crates/eggsec-output/src/agent.rs` | Enum: Confirmed, Likely, Possible, Unlikely. |
| `AttackSurface` | `crates/eggsec-output/src/agent.rs` | Enum: Web, Api, Network, Authentication, Session, FileSystem, Internal, Cloud, Cdn, Database. |
| `FindingStatus` | `crates/eggsec-output/src/agent.rs` | Enum: New, Confirmed, FalsePositive, Ignored, Remediated. |
| `FindingSummary` | `crates/eggsec-output/src/agent.rs` | Aggregated summary with `risk_score()`. |
| `BaselineComparison` | `crates/eggsec-output/src/baseline.rs` | Baseline diff. Fields: new_findings, resolved_findings, unchanged_findings (all `Vec<AgentFinding>`). |

### eggsec-core Types

| Type | File | Line | Purpose |
|------|------|------|---------|
| `Severity` | `crates/eggsec-core/src/types.rs` | 15 | Canonical severity enum: Critical, High, Medium, Low, Info. Single source of truth; re-exported by `eggsec-output`. |
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
| `DynamicMobileReport` | `crates/eggsec-mobile-lab/src/dynamic.rs` | 521 | Dynamic analysis report. Fields: findings, traffic_summary, frida_results, permission_state, etc. |
| `DynamicMobileFinding` | `crates/eggsec-mobile-lab/src/dynamic.rs` | 338 | Dynamic finding. Fields: id, severity, category, title, description, evidence (`Option<String>`), recommendation, cwe_ids, static_correlation (`Option<String>`). |
| `MobileBaseline` | `crates/eggsec-mobile-lab/src/dynamic.rs` | 105 | Lightweight baseline. Fields: target, timestamp, findings_count, frida_script_count, frida_findings, actions_sample. |
| Bridge | `crates/eggsec-mobile-lab/src/lib.rs` | 296 | `to_scan_report_data(&MobileScanReport)` |
| Bridge | `crates/eggsec-mobile-lab/src/dynamic.rs` | 1509 | `to_scan_report_data_dynamic(&DynamicMobileReport)` |
| Evidence bundle | `crates/eggsec-mobile-lab/src/dynamic.rs` | 212 | `export_evidence_bundle()` — gzipped JSON with report, traffic_summary, exported_at, frida_structured, bundle_manifest. |

#### db-pentest

| Type | File | Line | Purpose |
|------|------|------|---------|
| `DbPentestReport` | `crates/eggsec-db-lab/src/types.rs` | 8 | DB assessment report. Fields: db_type, scan_type, target, findings (`Vec<DbFinding>`), queries_executed, dry_run, manifest_path, duration_ms, correlation, compliance, baseline_label, regression_summary. |
| `DbFinding` | `crates/eggsec-db-lab/src/types.rs` | 60 | DB finding. Fields: id, category, severity (`String`), title, description, evidence (`Option<String>`), remediation, cwe_ids. |
| `DbBaseline` | `crates/eggsec-db-lab/src/baseline.rs` | 14 | Baseline with regression detection. Fields: captured_at, db_type, checks, finding_categories, severity_counts, total_findings, report, label. |
| `DbRegressionResult` | `crates/eggsec-db-lab/src/baseline.rs` | 35 | Regression comparison. Fields: new_findings, resolved_findings, severity_increases, severity_decreases, summary, is_regression, is_improvement. |
| Bridge | `crates/eggsec-db-lab/src/bridge.rs` | 9 | `to_scan_report_data_db(&DbPentestReport)` |
| Evidence bundle | `crates/eggsec-db-lab/src/lib.rs` | 655 | `export_db_evidence_bundle()` — gzipped JSON with report, manifest_path, manifest_data, exported_at, correlation, compliance, bundle_manifest. |

#### web-proxy

| Type | File | Line | Purpose |
|------|------|------|---------|
| `WebProxySessionReport` | `crates/eggsec-web-proxy/src/intercept/types.rs` | 102 | Proxy session report. Fields: flows, manipulations, ws_sessions, http2_sessions, grpc_sessions, budget, correlation, correlation_refs. |
| `EvidenceBundle` | `crates/eggsec-web-proxy/src/intercept/bundle.rs` | 19 | Signed bundle. Fields: version, manifest, flows, sessions, rules, manipulations, correlations. Supports HMAC-SHA256 signing. |
| `BundleManifest` | `crates/eggsec-web-proxy/src/intercept/bundle.rs` | 42 | Bundle manifest. Fields: target, scope, started_at, ended_at, user, dry_run, flow/session/manipulation/correlation/rule counts, signature. |
| `BundleDiff` | `crates/eggsec-web-proxy/src/intercept/bundle.rs` | 307 | Bundle comparison. Fields: added/removed/modified flows, count diffs. |
| Bridge | `crates/eggsec-web-proxy/src/intercept/bridge.rs` | 10 | `to_scan_report_data_proxy(&WebProxySessionReport)` |

## Conversion Bridges

| Domain | Bridge Function | File | Line | Input Type |
|--------|----------------|------|------|------------|
| mobile-static | `to_scan_report_data()` | `crates/eggsec-mobile-lab/src/lib.rs` | 296 | `&MobileScanReport` |
| mobile-dynamic | `to_scan_report_data_dynamic()` | `crates/eggsec-mobile-lab/src/dynamic.rs` | 1509 | `&DynamicMobileReport` |
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

## Report Delivery

### The report read path

A report reaches a client over the daemon's report read path. The document is
produced by the engine, kept by the runtime, served by the daemon, and read by
the client:

| Stage | Where | What happens |
|-------|-------|--------------|
| Produce | `crates/eggsec/src/runtime_bridge/report.rs:111` | `report_for_operation(&str, &TaskResult) -> Option<ScanReportData>` decides **whether** a document is built from the coverage table, then `report_for_result(&TaskResult) -> Option<ScanReportData>` (`:69`) derives it. The bridge is **pure**: it performs no I/O, reads no state, and consults no clock, so the same result always yields the same document. |
| Keep | `crates/eggsec-runtime/src/report.rs:77` | The engine records the document in the runtime's bounded report store (`RuntimeReportStore`). Retention is capped at `MAX_RETAINED_REPORTS` = 64 with oldest-first eviction (`crates/eggsec-runtime/src/report.rs:64`) — the bound is what stops a long-lived daemon accumulating one report document per task for the life of the process. |
| Serve | `crates/eggsec-daemon-protocol/src/protocol.rs:227` | `ClientCommand::GetTaskReport { request_id, session_id, task_id }` → `ServerMessage::TaskReport { …, report }` (`crates/eggsec-daemon-protocol/src/protocol.rs:411`). The document travels as JSON, so the protocol crate keeps no report-model dependency; the consumer deserializes it into the canonical model. |
| Read | `crates/eggsec-frontend-client/src/daemon.rs:257` | `DaemonRuntimeClient::get_task_report()` returns `Result<Option<serde_json::Value>>`. `Ok(None)` means **no report is available** — either the daemon has none for that task, or the daemon is too old to know the command. Both are degradable states, not failures. |

The bridge covers the result families that have a natural report form — port
scan, pipeline, fingerprint, fuzz, recon, and (behind their feature gates)
wireless, wireless-active, db-pentest, and c2, the last four reusing the
engine's existing per-domain bridges verbatim. Every other result yields
`None`: "no report available" is the honest answer for a result with no report
form, and a lossy document pretending to be a report is not.

### The report coverage table

**Which** operations produce a report is a fact of its own, and it lives in
exactly one place:
`crates/eggsec/src/runtime_bridge/report_coverage.rs`. The table has one row
per canonical operation id — **28 rows** — and each row names the
`TaskResult` variants that operation can produce, the verdict, and — when the
verdict is *no clean mapping* — the fact the result is missing.

The verdict is per **operation**, not per variant: the GUI's coverage table has
one row per published task kind, and three operations carry more than one
variant (`waf-detect`, `storage`, `integrations`). In all three the variants
agree, which is a measured fact about the result types.

The arithmetic, over the engine's **35** `TaskResult` variants:

| | Variants | Task kinds |
|---|---|---|
| Canonical (a report document is produced) | **17** | 18 |
| Excluded (no clean report mapping) | **17** | 12 |
| The terminal error shape (`Error`) | 1 | — |

The seventeen canonical variants are the nine the bridge already mapped
(port scan, pipeline, fingerprint, fuzz, recon, and the four reused domain
bridges) plus eight new arms: **endpoint scan, graphql, oauth, auth test, nse,
hunt, browser** and **compliance**.

The eighteen excluded variants are the seventeen named below plus the terminal
`Error` shape — a task's *failure*, not an operation's result, so a "report" of
one would be a document with an empty target and no findings. Each excluded
operation's reason names the fact its result is missing:

| Operation | Variants | Why there is no clean mapping |
|---|---|---|
| `load-test` | `LoadTest` | the result carries only transport-error strings and a status-code count map, so it has no finding with a severity, a title or a location |
| `stress-test` | `StressTest` | the result carries only counters — packets sent, bytes sent and error counts — and no finding, no port and no service |
| `waf-detect` | `WafDetection`, `WafBypass` | neither variant carries a target: the detection and the bypass record the WAF's own facts, and the subject lives in the request |
| `waf-stress` | `WafStress` | the result is a bare list of bypass attempts with no subject and no duration |
| `packet-capture` | `PacketCapture` | the result names no interface and carries only a packet count and an output file, so the captured traffic has no home in the canonical model |
| `packet` | `PacketTraceroute`, `PacketSend` | neither variant names the host it ran against: the hops and the two counters carry no subject |
| `storage` | `Storage`, `StorageListScans`, `StorageListFindings` | the result is a bare acknowledgement or a catalogue read: stored scans and stored findings are lifecycle views, not a scan of one subject |
| `integrations` | `Integrations`, `IntegrationsCreateIssue`, `IntegrationsSearchIssues` | the result is a tracker work item or a list of them and carries no target: an issue is not a fact about a scanned subject |
| `workflow` | `Workflow` | the report carries counts and findings but no target field, so a workflow run is a lifecycle view over stored findings |
| `vuln` | `Vuln` | the assessment carries no target and its prioritized findings carry no location, so there is no subject to report on |
| `proxy-intercept` | `Intercept` | the session's target is optional and absent whenever the session had none, and the captured exchanges have no collection in the canonical model |
| *(terminal)* | `Error` | the task failed: a terminal error is not a scan of a subject, so a document would carry an empty target and no findings |

A **missing timestamp or duration does not disqualify** a variant: the bridge
leaves those fields empty rather than inventing them, and the canonical model
tolerates it. A missing **identity** does disqualify it — a document with an
empty `target` does not say what it is about, and every renderer carries that
emptiness into the artifact.

The table is **published**, not merely consulted: the engine's executor
projects it into the wire DTO (`eggsec_runtime::report::ReportFormEntry`), the
daemon republishes it verbatim in its `Capabilities` reply, and the GUI joins
it to its coverage table by operation id. `None` on the wire means **unknown**
— a daemon that links no engine publishes no coverage — and never "no
operation has a report form".

### The `timestamp` semantics

The workspace has **three** timestamp semantics and one empty case:

| Semantics | Families |
|---|---|
| the scan's own start time | `pipeline` |
| the report's own `timestamp` field | `c2`, `db-pentest` |
| the report-generation time (a clock read) | `wireless`, `wireless-active` |
| empty (no clock read) | `port-scan`, `fingerprint`, `fuzz`, `recon`, and the eight new families |

None of the four *empty* families' result types carries a timestamp, so
closing that gap requires adding a timestamp to those result types or giving
the bridge a clock — and a clock would contradict the purity contract above.
The eight new families therefore follow the majority precedent and carry an
empty `timestamp`.

The **task envelope contract is untouched** by this path:
`TaskResultEnvelope.payload` stays `json!({})` and `.artifacts` stays `vec![]`
(`crates/eggsec/src/dispatch/canonical_execution.rs:1114`). The report body
travels on its own path and is never folded into the envelope.

### HTML rendering

`eggsec-output` renders two HTML variants over the same report data:

| Function | File | Remote origin |
|----------|------|---------------|
| `convert_to_html(&ScanReportData) -> String` | `crates/eggsec-output/src/convert.rs:82` | Carries the remote chart-library reference `<script src="https://cdn.jsdelivr.net/npm/chart.js">`. This is the default, and its output is **unchanged** — byte-for-byte. |
| `convert_to_html_self_contained(&ScanReportData) -> String` | `crates/eggsec-output/src/convert.rs:102` | Omits that reference, so the document carries **no remote origin** — the form to use when embedding or sharing a report where a remote fetch is not acceptable. |

`HtmlReport::generate_self_contained()` (`crates/eggsec-output/src/html.rs:68`)
is the underlying entry point. The two documents differ only by that one
reference, so the default rendering is unaffected by the self-contained mode:
`default.replace(remote_chart_line, "") == self_contained` is the property the
renderer's tests assert (`crates/eggsec-output/tests/html_self_contained.rs:131`).

Note for accuracy: "self-contained" here means *no remote origin*. The document
still carries an inline `<style>` block and an inline `<script>` block
(`crates/eggsec-output/src/html.rs:158`). That inline script looks up
`severityChart` and draws only if such an element exists — the renderer emits
no `<canvas id="severityChart">`, so the chart never draws in either mode.

## Known Incompatibilities

| # | Issue | Details |
|---|-------|---------|
| 1 | **Severity representation** | `FindingData` uses `String` for severity; `AgentFinding` uses the `Severity` enum; canonical `Finding` uses `Severity` enum. No consistent representation across serialization boundaries. |
| 2 | **Evidence representation** | `Option<String>` in domain findings (`MobileFinding`, `DbFinding`, `DynamicMobileFinding`), `Evidence` struct in agent output (`crates/eggsec-output/src/agent.rs:304`), `EvidenceKind` + `data` (`serde_json::Value`) in canonical `Finding`. Three distinct models. |
| 3 | **No shared evidence bundle manifest** | Each domain defines its own manifest schema. No common versioning, content listing, or integrity verification contract. |
| 4 | **No shared baseline summary format** | `DbBaseline`, `MobileBaseline`, and `BundleDiff` are structurally different with no common fields for normalized comparison. |
| 5 | **Bridge information loss** | All domain bridges target `ScanReportData`, which loses domain-specific information (e.g., Frida results, DB queries, proxy flow budgets, compliance data). |
| 6 | **No normalized report envelope** | No report type preserves `report_id`, `operation_id`, and `domain_id` across the conversion chain. Traceability from output back to execution context is broken. |
| 7 | **PolicySummary underutilized** | `PolicySummary` exists in `eggsec-report-model` but is not consistently populated by domain bridges, leaving enforcement metadata incomplete in cross-domain reports. |

## Target Normalized Model

**Shipped.** The normalized contract lives in `eggsec-report-model`
(`crates/eggsec-report-model/src/envelope.rs` and `summary.rs`), not in
`eggsec-output` — `eggsec-output` renders over it and never the reverse
(guards Checks 118–120). The types are `EvidenceItem`, `FindingRecord`,
`ReportEnvelope`, `EvidenceManifest`, `BaselineSummary`, and
`DiffSummary`, as inventoried in the tables above; read those files for
the authoritative field lists rather than a duplicated sketch here.

Design goals it satisfied:

- Single serializable representation for report output across CLI, TUI, REST, MCP, and agent surfaces.
- Domain crates convert into the shared model; no domain type escapes into the output layer.
- Evidence is structured and typed, not a string blob.
- Report traceability via `report_id`, `operation_id`, and `domain_id` on `ReportEnvelope`.
- Baseline and diff summaries share a common shape.

Remaining work is consumer migration: CLI/TUI/REST/MCP renderers still
consume `ScanReportData` directly, and legacy `ScanReportData` has not
been removed.

### RedactionPolicy

The `EvidenceManifest` includes a `redaction_policy` field that declares the manifest-level
redaction strategy (`RedactionPolicy` at `crates/eggsec-report-model/src/envelope.rs:132`).
This is distinct from per-item `RedactionState` on `EvidenceItem`
(`envelope.rs:117`):

| Policy | Meaning |
|--------|---------|
| `None` | No redaction; all evidence included as-is |
| `RedactAll` | Redact all items regardless of individual state |
| `RedactSensitive` | Redact only items marked as sensitive |
| `SummarizeAll` | Replace raw content with summaries |
| `DomainSpecific` | Domain-specific logic; individual item states take precedence |
