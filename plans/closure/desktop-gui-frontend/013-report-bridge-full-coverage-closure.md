# Desktop GUI Frontend Milestone 013 — Closure Status

Status: closed

Source implementation plan:

- `plans/implementation/desktop-gui-frontend/013-report-bridge-full-coverage.md`

Source subsystem roadmap:

- `plans/subsystems/desktop-gui-frontend-roadmap.md#7-milestones`

Repository baseline reviewed: `0f5dbb26`

Implementation commits:

- `6de6340e` — feat(desktop-gui): implement the desktop GUI milestones 012 to 015

This milestone is **corrective**. It closes the two findings milestone 006
left open in its closure record (§10.1 "the report bridge covers only the
families with a clean mapping" and §10.8 "the report bridge's `timestamp`
field is empty for four families"). It does **not** rewrite milestone 006's
plan or its closure record: those stay as they are, and §10 of the 006 record
remains the record of what was unresolved when 006 closed.

## 1. Executive finding

**The report bridge now covers every result variant the engine declares, and
it publishes what it cannot cover so the operator can see why.**

The design has three parts:

- **One table, one home.** `crates/eggsec/src/runtime_bridge/report_coverage.rs`
  holds a row per canonical operation id — **28 rows** — and each row names
  the `TaskResult` variants that operation produces, the verdict, and, for an
  excluded operation, the fact its result is missing. The bridge consults it;
  the engine publishes it; the daemon republishes it verbatim; the GUI renders
  it. Nobody restates it.
- **The verdict is per operation, and the table says so.** A document needs an
  identity (a target or an equivalent subject) and at least one content item
  that maps onto the canonical model without a fact being invented. Eight
  operations fail that test, and the table states the missing field for each.
  A missing *timestamp* does not disqualify a variant — the bridge stays
  clock-free and leaves the field empty — but a missing *identity* does.
- **The bridge cannot silently disagree with the table.** `report_for_operation`
  asks the table first and the result second; a `Canonical` verdict whose
  result has no arm is an internal error, logged at error level with the
  operation id, never swallowed. Guard 180 makes that disagreement unreachable
  in the source, and the log is the belt to that braces.

The user-visible change is small and honest: the GUI's coverage table gains a
**report-form column**, and the export failure message names the reason the
daemon published for an operation having no report form — *"the daemon has no
report for task `<id>`: `<the reason>`"* — instead of only saying that there
is no report.

**What this milestone did NOT do**, and why: it did not add a timestamp to the
four result types whose documents carry an empty one (that is a change to four
engine result types, their serializations and their downstream consumers), it
did not give the bridge a clock (that would contradict the module's purity
contract), and it did not change the daemon's `TaskNotFound` answer for a task
with no report (the error-code set is a published contract). Closure finding 8
is therefore **restated as still open, and widened** — see §10, finding F1.

## 2. Requirement-to-evidence matrix

| Requirement | Evidence | Result | Notes |
|---|---|---|---|
| §13.1 Every canonical operation id has exactly one coverage row; the table's ids are exactly the distinct ids of the canonical task-kind table (derived) | `report_coverage.rs` test `the_coverage_table_covers_every_canonical_operation` (derives the ids from `TaskKind::all_variants()`); guard 180's set comparison | **pass** | 28 rows, 28 distinct ids, no id under two rows |
| §13.2 Every variant with a clean mapping produces a document — 17 of the 35 variants, covering 18 task kinds | `report.rs` tests `an_endpoint_scan_reports_its_interesting_endpoints`, `a_graphql_report_names_its_vulnerabilities` (inside `the_new_documents_carry_no_timestamp`), `an_oauth_report_names_its_vulnerabilities` (same), `an_auth_report_carries_the_findings_own_severity` (same), `an_nse_report_carries_the_evidence_items_own_severity`, `a_hunt_report_carries_every_finding_collection`, `a_browser_report_carries_the_xss_and_client_findings`, `a_compliance_report_carries_the_failed_requirements_only`; guard 180's canonical-row/bridge-arm cross-check | **pass** | The eight new arms are behind their feature gates; the ones without a gate (`endpoint-scan`, `graphql`, `oauth`, `auth-test`) are compiled and tested in the `--features cli` run |
| §13.3 Every variant without one is named, with the missing fact stated — 18 variants across 12 task kinds, each with a non-empty reason | `report_coverage.rs` test `every_excluded_operation_names_the_missing_fact`; the table's own text (§3 below) | **pass** | 17 excluded variants + the terminal `Error`; every reason is non-empty and names an absence or a field |
| §13.4 The export path reports "no report available" and writes no artifact for a task with no report | `report.rs` test `an_excluded_operation_produces_no_document` (the `None` return); the GUI test `the_export_path_writes_no_file_for_a_task_with_no_report` (the error is raised before `export::render` and before `export::write_export`); the existing engine test `a_load_test_has_no_canonical_report_form` | **pass** | The behaviour was already correct at the baseline; this milestone supplies the regression evidence, as §3.6 of the plan required |
| §13.5 The operator can learn the reason: the coverage table states the verdict per published task kind, and the export failure names the reason when the GUI holds one | `coverage.rs` tests `a_coverage_row_carries_the_published_report_form`, `an_unpublished_coverage_is_unknown_not_unavailable`; `commands.rs` tests `the_export_message_names_the_reason_when_it_has_one`, `the_export_message_falls_back_when_it_cannot_resolve_one` | **pass** | The GUI names no operation id of its own: the join is by the published catalog's operation id |
| §13.6 The table has exactly one home; the daemon and the GUI restate none of it | Guard 180's "exactly one place" check; guard 161 re-run standalone (PASS) | **pass** | The daemon republishes the executor's rows verbatim; the GUI's `ReportFormVerdict` is a projection, not a table |
| §13.7 Guard 180 passes, and its four negative probes fail as required | §4.3 below (four mutations, four FAIL outputs) | **pass** | The guard block is at `/tmp/eggsec-guardwork/m013-guards.sh`; the owner merges it into `scripts/check-architecture-guards.sh` |
| §13.8 The canonical report model is unchanged: `ScanReportData` keeps its nine fields, and the eight new documents use them as they are | No change to `crates/eggsec-report-model`; the eight arms construct `ScanReportData` with the same nine fields | **pass** | `git diff --stat crates/eggsec-report-model` is empty for this milestone |
| §13.9 The bridge remains clock-free: the eight new documents carry an empty `timestamp`, and no new arm reads a clock | `report.rs` test `the_new_documents_carry_no_timestamp`; no `Utc::now()` / `SystemTime` in the eight arms | **pass** | The four pre-existing empty-timestamp families are unchanged |
| §13.10 No authorization decision changed, and the published coverage carries no target, no banner and no credential | The eight arms are pure functions over a result; `report_coverage.rs` contains no `EnforcementContext`, no `Scope`, no `ApprovedExecution`; the published DTO (`ReportFormEntry`) carries an operation id, a verdict and a reason only | **pass** | The gate consults a *static table*, not a policy decision |
| §13.11 `make check` is green except for the inherited `check-deps` advisory backlog | **not run** — `make check` is the session owner's integration step and it stops at `check-deps` | **not run** | `make check-deps` **was** run and is green at the measured state (§4.2) — see the note in §4.2 about the inherited backlog |
| §13.12 No new dependency, and no `deny.toml` exception | No `Cargo.toml` was edited by this milestone; `git diff --stat Cargo.lock deny.toml crates/*/Cargo.toml` shows only milestone 015's concurrent change | **pass** | `make check-deps` is green |
| §10.23 A daemon hosting the engine executor publishes the full coverage — one row per canonical operation id, derived from the canonical task-kind table | `crates/eggsec-daemon/tests/report_form_coverage.rs` test `a_daemon_hosting_the_engine_publishes_the_full_coverage` | **pass** | 28 rows; the ids are derived from `TaskKind::all_variants()`, and every excluded row carries a non-empty reason |
| §10.24 A daemon hosting a noop executor publishes `None` | `crates/eggsec-daemon/src/host.rs` test `capabilities_report_an_unpublished_report_coverage_as_unknown` | **pass** | `None` means unknown, not an empty list |
| §10.26 The export path writes no file for a task with no report | `crates/eggsec-gui/src/commands.rs` test `the_export_path_writes_no_file_for_a_task_with_no_report` | **pass** | a source-level assertion that the error precedes the render and the write |
| §10.31 The published coverage leaks nothing | `crates/eggsec-daemon/tests/report_form_coverage.rs` test `the_published_coverage_leaks_nothing` | **pass** | the reasons are lowercase prose about a result type's *shape*; no URL, no credential, no observed value |
| §10.32 The coverage grants no authorization | `crates/eggsec-daemon/tests/report_form_coverage.rs` test `the_coverage_grants_no_authorization`; §5's invariant review | **pass** | the coverage is published beside the task-kind list and changes neither it nor the runtime's admitted set |
| §10.34 The GUI still renders the report as data | guard 162 re-run standalone: `PASS: the GUI's report display carries no remote origin and no executable content.` | **pass** | the new Report column adds no markup sink |
| §10.36 A new daemon's capabilities decode in an old client | `crates/eggsec-daemon-protocol/src/protocol.rs` test `a_new_daemons_capabilities_decode_in_an_old_client` | **pass** | the field is additive; an old client ignores it |
| §13.13 The documentation in §12 states the new coverage, and the closure record carries §15's evidence | `docs/REPORT_EVIDENCE_MODEL.md`, `docs/DAEMON.md`, `architecture/daemon.md`, `architecture/runtime.md`, `architecture/overview.md`, `AGENTS.md` (this milestone); the three shared-doc deltas handed to the owner in `/tmp/eggsec-guardwork/` | **partial** | The six documents this milestone owns are updated. `docs/CI_ARCHITECTURE_GUARDS.md`, `.opencode/skills/eggsec-gui/SKILL.md` and `architecture/desktop_gui.md` are **shared** and were **not edited**; their deltas are written to `/tmp/eggsec-guardwork/m013-guards-doc.md`, `m013-skill.md` and `m013-desktop_gui.md` for the owner to merge |

## 3. Production implementation evidence

### 3.1 The coverage table as built

**Method.** Re-measured from the landed source
(`crates/eggsec/src/runtime_bridge/report_coverage.rs`) with a
formatting-robust awk that accumulates a `variants:` array across lines (a
`rustfmt`-wrapped array reads as one row) and prints `verdict|variants` per
row; the variant names are then counted from that output. The enum's variants
were counted with `sed -n '/^pub enum TaskResult {/,/^}/p'` over
`crates/eggsec/src/dispatch/types.rs`.

| Measure | Value |
|---|---|
| `TaskResult` variants declared by the engine | **35** |
| Coverage rows | **28** |
| Rows with a `Canonical` verdict | **17** (17 variants) |
| Rows with a `NoCleanMapping` verdict | **11** (17 variants) |
| Terminal result shapes named beside the rows | **1** (`Error`) |
| Distinct canonical operation ids (derived from `TaskKind::all_variants()`) | **28** |
| Total variants accounted for | **17 + 17 + 1 = 35** |

The plan's arithmetic is confirmed against the source: the engine's result
type has **35** variants (not 30 — 30 is the `TaskKind` count), and they split
as 17 canonical + 17 excluded + 1 terminal. The 30 task kinds map to 28
operation ids, of which 18 task kinds have a canonical verdict and 12 do not.

The table is **build-independent**: it names all thirty-five variants,
feature-gated ones included, because the coverage is a statement about the
engine's result model rather than about one build of it. A build without the
`nse` feature compiles the same 28 rows but cannot reach the gated variants.

### 3.2 The bridge's arms as built

`crates/eggsec/src/runtime_bridge/report.rs` — `report_for_result` now has
**17** arms (`report.rs:69-98`), nine of them pre-existing and eight new:

| # | Variant | Arm function | Document's fields |
|---|---|---|---|
| 1 | `PortScan` | `port_scan_report` (pre-existing) | target, open ports, services, duration |
| 2 | `Pipeline` | `pipeline_report` (pre-existing) | target, ports, services, findings, duration |
| 3 | `Fingerprint` | `fingerprint_report` (pre-existing) | target, services, duration |
| 4 | `Fuzz` | `fuzz_report` (pre-existing) | target, the session's vulnerable payloads |
| 5 | `Recon` | `recon_report` (pre-existing) | target, secrets, takeover candidates |
| 6 | `EndpointScan` | `endpoint_scan_report` (**new**) | target ← `base_url`; duration ← `duration_ms`; one finding per **interesting** endpoint: title ← the path, severity ← the class constant `info`, category ← `endpoint-discovery`, description ← the status line and any redirect, location ← the path |
| 7 | `GraphQl` | `graphql_report` (**new**) | target ← `target`; duration ← `duration_ms`; `depth_limit_bypassed` → `medium`/`graphql-depth-limit`; `alias_overload_vulnerable` → `medium`/`graphql-alias-overload`; each entry of `injection_findings` → `high`/`graphql-injection` |
| 8 | `OAuth` | `oauth_report` (**new**) | target ← `target`; duration ← `duration_ms`; `redirect_vulnerabilities` → `medium`/`oauth-redirect`; `scope_vulnerabilities` → `high`/`oauth-scope`; `state_vulnerabilities` → `medium`/`oauth-state`; `grant_vulnerabilities` → `high`/`oauth-grant` |
| 9 | `Auth` | `auth_report` (**new**) | target ← `target`; one finding per `findings` entry: title, description and recommendation are the finding's own; severity ← the finding's own `severity`; category ← `auth-<test type>`; location ← the target; remediation ← the finding's own recommendation |
| 10 | `Wireless` | `crate::wireless::to_scan_report_data` (pre-existing) | reused verbatim |
| 11 | `WirelessActive` | `crate::wireless::active::to_active_scan_report_data` (pre-existing) | reused verbatim |
| 12 | `DbPentest` | `crate::db_pentest::to_scan_report_data_db` (pre-existing) | reused verbatim |
| 13 | `C2` | `crate::c2::to_scan_report_data` (pre-existing) | reused verbatim |
| 14 | `Nse` | `nse_report` (**new**, `cfg(feature = "nse")`) | target ← `target`; one finding per `report.evidence` item: title ← the item's title, severity ← **the engine's own** `nse_bridge::evidence_kind_to_severity` (reused, not restated), category ← `nse-<evidence kind>`, description ← the item's summary, evidence ← the raw excerpt, cwe_ids ← the item's references |
| 15 | `Hunt` | `hunt_report` (**new**, `cfg(feature = "advanced-hunting")`) | target ← `target`; one finding per entry of the five collections (`attack_chains`, `business_logic`, `race_conditions`, `authz_bypasses`, `session_issues`), each with its own severity; location ← the finding's own location where the type has one, otherwise the target |
| 16 | `Browser` | `browser_report` (**new**, `cfg(feature = "headless-browser")`) | target ← `target`; one finding per DOM XSS finding (`browser-dom-xss`) and per client-side issue (`browser-client-issue`), each with its own severity, description, location, evidence and remediation; the SPA routes are **not** findings |
| 17 | `Compliance` | `compliance_report` (**new**, `cfg(feature = "compliance")`) | target ← `target`; one finding per **failed** requirement (`status == ComplianceStatus::Fail`), with the requirement's own severity and description, category ← `compliance-<framework>`, remediation ← the requirement's own remediation; a passing or not-applicable requirement is not a finding |

Every new document carries an **empty `timestamp`** and a `duration_ms` of `0`
where the result type has no duration field. The severity of a new finding is
either the finding's own (`Auth`, `Hunt`, `Browser`, `Compliance`) or a
**per-class constant** (`EndpointScan` → `info`; `GraphQl` → `medium`/`high`;
`OAuth` → `medium`/`high`) — never a value read from the target.

### 3.3 The exclusion list with its reasons

Each reason is traced to the result type's own fields:

| Operation | Variants | The result type's fields | Why there is no clean mapping |
|---|---|---|---|
| `load-test` | `LoadTest` | `LoadTestResults { target_url, total_requests, successful_requests, failed_requests, total_duration_ms, requests_per_second, latency_*, status_codes, errors, error_kinds }` | carries only transport-error strings and a status-code count map; no finding with a severity, a title or a location |
| `stress-test` | `StressTest` | `StressStats { duration_ms, packets_sent, bytes_sent, errors }` | carries only counters; no finding, no port, no service |
| `waf-detect` | `WafDetection` | `WafDetectionResult { waf_name, confidence, request_error, matched_headers, matched_cookies, matched_patterns, server_header, status_code }` | **no target field**: the detection records the WAF's own facts, and the subject lives in the request |
| `waf-detect` | `WafBypass` | `WafBypass { detection, bypasses }` | **no target field**, and the bypasses are the WAF's own attempts |
| `waf-stress` | `WafStress` | `Vec<BypassResult>`; `BypassResult { technique, success, description, payload, status_code, response_diff, error }` | a bare list of attempts with no subject and no duration |
| `packet-capture` | `PacketCapture` | `PacketCapture { packets_captured, output_file }` | **no interface and no target**; only a count and an output file |
| `packet` | `PacketTraceroute` | `PacketTraceroute { hops }`; `TracerouteHopResult` | **no target**: the hops carry no subject |
| `packet` | `PacketSend` | `PacketSend { packets_sent, bytes_sent }` | **no target**: two counters |
| `storage` | `Storage` | a unit variant | a bare acknowledgement |
| `storage` | `StorageListScans` | `StorageListScans { scans }` | a catalogue read, not a scan of one subject |
| `storage` | `StorageListFindings` | `StorageListFindings { findings }` | a catalogue read, not a scan of one subject |
| `integrations` | `Integrations` | a unit variant | a bare acknowledgement |
| `integrations` | `IntegrationsCreateIssue` | `IntegrationsCreateIssue { issue }`; `Issue { id, title, description, labels, severity, assignees, status, url, created_at }` | **no target**: an issue is not a fact about a scanned subject |
| `integrations` | `IntegrationsSearchIssues` | `IntegrationsSearchIssues { issues }` | a list of work items, with no target |
| `workflow` | `Workflow` | `WorkflowReport { total_findings, open_findings, in_progress_findings, resolved_findings, sla_violations, findings }` | **no target field**: a lifecycle view over stored findings |
| `vuln` | `Vuln` | `VulnAssessment { mode, assessed_at, cvss_score, exploit_info, asset_criticality, prioritized_findings, triage_results, remediation_plans, summary }` | **no target**, and the prioritized findings carry no location |
| `proxy-intercept` | `Intercept` | `InterceptSession { listen_addr, ca_fingerprint, dry_run, started_at, ended_at, target: Option<String>, flows, manipulations, flow_actions, budget }` | the target is **optional** and absent whenever the session had none; the captured exchanges have no collection in the canonical model |
| *(terminal)* | `Error` | `Error(String)` | a task's *failure*, not an operation's result: a document would carry an empty target and no findings |

### 3.4 The verdict of §3.6's four stages, at the landed baseline

| Stage | Landed reference | What it does now |
|---|---|---|
| Bridge | `crates/eggsec/src/runtime_bridge/report.rs:69` (`report_for_result`), arms at `:71-97`; `:111` (`report_for_operation`) | the gate asks the table first; an excluded operation yields `None` **before** the result is inspected; a canonical operation with no arm logs at error level and yields `None` |
| Executor | `crates/eggsec/src/runtime_bridge/executor.rs:395-403` | `if let Some(report) = report_for_operation(&operation_id, &task_result)` → `reports.insert(task_id, document)`; nothing is inserted for a task with no report, so the store has **no entry** for it |
| Runtime | `crates/eggsec-runtime/src/runtime.rs:244` (`report_for`) | reads the executor's store; a task with no entry yields `None` |
| Daemon | `crates/eggsec-daemon/src/host.rs:754` | `None` → `ServerMessage::Error { code: ErrorCode::TaskNotFound, … }` |
| Client | `crates/eggsec-frontend-client/src/daemon.rs:274` | `TaskNotFound` → `Ok(None)` — a degradable state, not a failure |
| GUI export | `crates/eggsec-gui/src/commands.rs:702-737` | `Ok(None)` → `Err("the daemon has no report for task `<id>`: `<reason>`")`, or the plain wording when no reason can be resolved. The error is raised **before** `export::render` and `export::write_export`, so **no file is written** |
| GUI view | `crates/eggsec-gui/src/commands.rs:833` | `Ok(None)` → `Err("the daemon has no report for task `<id>`, so there is nothing to show")` — unchanged by this milestone |

### 3.5 The publication path

| Stage | Landed reference | What happens |
|---|---|---|
| Table | `crates/eggsec/src/runtime_bridge/report_coverage.rs:78` | `REPORT_COVERAGE` — 28 rows, the single home |
| Engine | `crates/eggsec/src/runtime_bridge/executor.rs:181-196` | `report_forms()` projects each row into `eggsec_runtime::report::ReportFormEntry` — the operation id, the verdict, the reason. Nothing is restated |
| Runtime | `crates/eggsec-runtime/src/runtime.rs:122` (trait), `:198` (capture at construction), `:235` (accessor) | the coverage is captured once, at construction, and never changes |
| Daemon | `crates/eggsec-daemon/src/host.rs:180`/`:212`/`:253` (capture), `:524` (publication) | the daemon republishes the executor's rows **verbatim** in `DaemonCapabilities.report_forms` |
| Protocol | `crates/eggsec-daemon-protocol/src/protocol.rs:93` | `report_forms: Option<Vec<ReportFormEntry>>` with `#[serde(default)]`; the protocol version stays **2** |
| GUI | `crates/eggsec-gui/src/coverage.rs` (`ReportFormVerdict`, `resolve_report_form`) and `crates/eggsec-gui/src/commands.rs:660-676` | the row carries the verdict and the reason; the export message names the reason |
| Frontend | `crates/eggsec-gui/frontend/app.js:324` (the column header), `:332-357` (the cell) | the coverage table gains a **Report** column, rendered from the data the daemon published |

### 3.6 The `None`-means-unknown evidence, on both sides

| Side | Evidence |
|---|---|
| The daemon publishes it | `crates/eggsec-daemon/src/host.rs` test `capabilities_publish_the_executor_report_coverage` — an executor that publishes two rows has them republished row for row, with the verdict and the reason intact |
| The daemon publishes **nothing** | `crates/eggsec-daemon/src/host.rs` test `capabilities_report_an_unpublished_report_coverage_as_unknown` — an executor that publishes nothing leaves `report_forms` as `None`, which means *unknown*, not "no operation has a report form" |
| The wire | `crates/eggsec-daemon-protocol/src/protocol.rs` test `daemon_capabilities_decode_without_the_report_forms_field` — a daemon too old to publish the field decodes as `None`; the field is `#[serde(default)]`, so the protocol version stays 2 |
| The GUI | `crates/eggsec-gui/src/coverage.rs` test `an_unpublished_coverage_is_unknown_not_unavailable` — an unpublished coverage, and a coverage that does not mention the operation, both read `Unknown` with an **empty** reason: the GUI states no reason it was not given |
| The runtime | `crates/eggsec-runtime/src/runtime.rs` test `the_default_executor_publishes_no_coverage` — the trait's default is `None`, so an executor that says nothing about report forms is not saying that no operation has one |

### 3.7 The timestamp position, restated

| Semantics | Families |
|---|---|
| the scan's own start time | `pipeline` |
| the report's own `timestamp` field | `c2`, `db-pentest` |
| the report-generation time (a clock read) | `wireless`, `wireless-active` |
| empty (no clock read) | `port-scan`, `fingerprint`, `fuzz`, `recon`, and the **eight new** families |

Closure finding 8 is **restated as covering twelve variants rather than four**,
and it **remains open** — see §10, finding F1.

### 3.8 Deviations from the plan, and why

1. **Four files outside the file-ownership list had to be edited.** The
   plan's §6.4 mandates a DTO in `crates/eggsec-runtime/src/report.rs`, the
   `RuntimeTaskExecutor::report_forms()` method in
   `crates/eggsec-runtime/src/runtime.rs`, the `DaemonCapabilities` field in
   `crates/eggsec-daemon-protocol/src/protocol.rs`, and the executor's
   implementation and call site in
   `crates/eggsec/src/runtime_bridge/executor.rs`. None of them is in another
   milestone's scope (012 is the daemon's socket path, 014 the GUI's
   qualification driver, 015 the dependency advisories), and without them the
   milestone cannot work at all. Two further one-line edits were forced by
   the same change: `crates/eggsec-cli/src/daemon_cli.rs` (a
   `DaemonCapabilities` literal that the new field breaks) and
   `crates/eggsec/src/nse_bridge.rs` (`evidence_kind_to_severity` from
   `fn` to `pub(crate)`, so the NSE arm **reuses** the engine's one severity
   mapping instead of restating it — a parallel table would have been a
   second source of truth).
2. **The coverage table is build-independent; the enum's variants are not.**
   Four variants are behind cargo features, so a build without them has
   fewer `TaskResult` variants than the table names. The plan's arithmetic is
   a statement about the *engine's* result model, so the table names all
   thirty-five variants in every build, and the Rust test checks the
   build-level subset plus the count of 35 while **guard 180** does the
   source-level set equality (it reads the declarations, so it sees the
   feature gates the compiler hides from a build).
3. **`ReportForm` is the runtime's wire DTO, reused by the engine** rather
   than a second engine-local enum. The plan's §6.1 named a `ReportForm`
   without saying which crate owns it; one enum, projected by the executor, is
   the smaller surface and avoids a second vocabulary for the same verdict.
   `ReportCoverageRow` stays engine-local: the runtime carries only the
   projected `ReportFormEntry`.
4. **The variant vocabulary is generated by a macro with an exhaustive match**
   (`declare_result_variants!` in `report_coverage.rs`) rather than
   transcribed. The match is exhaustive, so a new `TaskResult` variant breaks
   the build in the coverage module until it is named — the compile-time
   half of guard 180's first property.
5. **One hunt finding carries no remediation.** `RaceCondition.remediation`
   is a private field of `hunt::race`, so the race-condition finding carries
   `remediation: None` while the other four hunt collections carry their own.
   Reaching it would have meant widening a domain type's visibility, which
   is outside this milestone.
6. **Six of §10's integration rows were not implemented** — see §10, finding
   F5.

## 4. Verification executed

### 4.1 Commands run

```bash
cargo fmt --all --check
rustfmt --edition 2021 crates/eggsec-daemon-protocol/src/protocol.rs \
  crates/eggsec-daemon/src/host.rs crates/eggsec-gui/src/coverage.rs \
  crates/eggsec-runtime/src/report.rs crates/eggsec/src/runtime_bridge/executor.rs \
  crates/eggsec/src/runtime_bridge/report.rs crates/eggsec/src/runtime_bridge/report_coverage.rs
cargo clippy -p eggsec --features cli -- -D warnings
cargo clippy -p eggsec-runtime -p eggsec-daemon-protocol -p eggsec-daemon -- -D warnings
cargo clippy -p eggsec-gui --features shell -- -D warnings
cargo check --workspace --no-default-features
cargo check -p eggsec --all-targets --no-default-features
cargo check -p eggsec --features cli
cargo check -p eggsec-daemon --features service
cargo test -p eggsec --lib --features cli
cargo test -p eggsec-runtime --lib
cargo test -p eggsec-daemon-protocol --lib
cargo test -p eggsec-daemon --lib
cargo test -p eggsec-daemon --features service --test report_form_coverage
cargo test -p eggsec-gui --lib --features shell
cargo test -p eggsec-frontend-client --lib
cargo test -p eggsec-output --tests
cargo test -p eggsec-report-model --tests
python3 scripts/check_doc_references.py
make check-deps
bash /tmp/eggsec-guardwork/m013-guard-standalone.sh   # the guard block, run standalone
bash /tmp/eggsec-guardwork/m013-guard161.sh            # guard 161, re-run standalone
bash /tmp/eggsec-guardwork/m013-guard162.sh            # guard 162, re-run standalone
node --check crates/eggsec-gui/frontend/app.js
```

### 4.2 Results

| Command | Exit | Observed |
|---|---|---|
| `cargo fmt --all --check` | **1** | The files this milestone owns are clean. The remaining diffs are in **other milestones' files** in this tree — `crates/eggsec-daemon/src/server.rs`, `crates/eggsec-gui/tests/qualification.rs`, `crates/eggsec-runtime/src/daemon_endpoint.rs` (the last measured run; the set changes as the concurrent milestones format their own files). All are pre-existing or concurrent, and none is in this milestone's scope |
| `cargo clippy -p eggsec --features cli -- -D warnings` | 0 | clean |
| `cargo clippy -p eggsec-runtime -p eggsec-daemon-protocol -p eggsec-daemon -- -D warnings` | 0 | clean |
| `cargo clippy -p eggsec-gui --features shell -- -D warnings` | 0 | clean |
| `cargo check --workspace --no-default-features` | 0 | clean |
| `cargo check -p eggsec --all-targets --no-default-features` | 0 | clean (pre-existing warnings in the engine's own test targets) |
| `cargo check -p eggsec --features cli` | 0 | clean |
| `cargo check -p eggsec-daemon --features service` | 0 | clean; the engine's 11 warnings under this feature set are pre-existing (`crates/eggsec/src/packet/parse_impl.rs`, `crates/eggsec/src/stress/warning.rs`, `crates/eggsec/src/dispatch/canonical_execution.rs` — none is in this milestone's scope) |
| `cargo test -p eggsec --lib --features cli` | 0 | **1417 passed, 0 failed** |
| `cargo test -p eggsec-runtime --lib` | 0 | **118 passed, 0 failed** |
| `cargo test -p eggsec-daemon-protocol --lib` | 0 | **92 passed, 0 failed** |
| `cargo test -p eggsec-daemon --lib` | 0 | **110 passed, 0 failed** |
| `cargo test -p eggsec-daemon --features service --test report_form_coverage` | 0 | **3 passed, 0 failed** — the new integration file (`crates/eggsec-daemon/tests/report_form_coverage.rs`), gated on the `service` feature because a daemon without the profile links no engine and has no coverage to publish |
| `cargo test -p eggsec-gui --lib --features shell` | 0 | **92 passed, 0 failed** |
| `cargo test -p eggsec-frontend-client --lib` | 0 | **15 passed, 0 failed** |
| `cargo test -p eggsec-output --tests` | 0 | **92 passed, 0 failed** (73 + 7 + 12 across the three test binaries) |
| `cargo test -p eggsec-report-model --tests` | 0 | **11 passed, 0 failed** (0 + 11 across the two test binaries) |
| `python3 scripts/check_doc_references.py` | 1 | **30 missing references — all pre-existing.** They are in `architecture/capability_segregation.md`, `architecture/fuzzer.md`, `architecture/knowledge_corpus.md`, `architecture/network_dependency_baseline.md`, `architecture/nse_capability_inventory.md`, `architecture/nse_integration.md`, `architecture/overview.md` (`tests/fuzzer_payload_corpus_seam.rs`, which the reference at `HEAD` already carried), `architecture/proxy.md`, `architecture/transport_eggfetch.md`, `docs/CAPABILITIES.md`, `docs/CI_ARCHITECTURE_GUARDS.md`, `docs/NSE_COMPATIBILITY.md`, `docs/evidence/gui-qualification/macos-2026-10-24.md`. **None** of the 30 is a reference this milestone added: every path this milestone's documentation introduces (`crates/eggsec/src/runtime_bridge/report_coverage.rs`, `crates/eggsec-runtime/src/report.rs`) resolves |
| `make check-deps` | 0 | `advisories ok`, `bans ok`, `licenses ok`, `sources ok`. **Note on the inherited backlog:** the plan expected this check to fail on an inherited advisory backlog that milestone 015 owns. At the measured state of this tree the backlog is already closed — `deny.toml` and `Cargo.lock` carry **milestone 015's** concurrent change, not this milestone's — so the four sub-checks are green. This milestone added **no dependency** and made **no** `deny.toml` change; the baseline (pre-015) advisory failure was therefore not re-measured, because re-measuring it would require reverting another milestone's work |
| the guard block, standalone | 0 | `PASS: the coverage table accounts for every result variant the engine declares.` / `PASS: the report bridge's coverage table and its arms agree.` |
| guard 161, standalone | 0 | `PASS: the GUI's coverage model is derived, not transcribed.` — the GUI still names no operation id and no capability name |
| guard 162, standalone | 0 | `PASS: the GUI's report display carries no remote origin and no executable content.` — the new Report column adds no markup sink |
| `node --check crates/eggsec-gui/frontend/app.js` | 0 | the frontend parses |

`make check` was **not run** — it is the session owner's integration step, and
the plan records that it stops at `check-deps`.

### 4.3 Guard 180's four negative probes

Each probe broke the tree, ran the guard, and **restored the tree from a `cp`
backup** (never `git checkout --`). The backups were
`/tmp/eggsec-guardwork/backups/report_coverage.rs.bak2` and
`report.rs.bak2`.

**Probe 1 — a coverage row removed.** The `c2` row was deleted from
`REPORT_COVERAGE`:

```
--- Check 180: the report bridge's coverage table and its arms agree ---
FAIL: the coverage table and the engine's result variants disagree.
  only in the result type:
    C2
  only in the coverage table:
FAIL: the bridge has a TaskResult::C2 arm, but the coverage table does not call it canonical.
FAIL=1
```

**Probe 2 — the bridge and the table disagree.** The `c2` row's verdict was
flipped from `Canonical` to `NoCleanMapping`, leaving the bridge's arm alone:

```
--- Check 180: the report bridge's coverage table and its arms agree ---
PASS: the coverage table accounts for every result variant the engine declares.
FAIL: the bridge has a TaskResult::C2 arm, but the coverage table does not call it canonical.
FAIL: the bridge has a TaskResult::C2 arm, but the coverage table excludes that variant.
FAIL=1
```

**Probe 3 — a second copy of the table.** `report_coverage_copy.rs` was written
next to the original:

```
--- Check 180: the report bridge's coverage table and its arms agree ---
PASS: the coverage table accounts for every result variant the engine declares.
FAIL: the report coverage table must live in exactly one place; found: crates/eggsec/src/runtime_bridge/report_coverage.rs crates/eggsec/src/runtime_bridge/report_coverage_copy.rs
FAIL=1
```

**Probe 4 — a canonical variant with no bridge arm.** The
`TaskResult::C2` arm was removed from `report_for_result`:

```
--- Check 180: the report bridge's coverage table and its arms agree ---
PASS: the coverage table accounts for every result variant the engine declares.
FAIL: the coverage table calls 'C2' canonical, but the bridge has no TaskResult::C2 arm.
FAIL=1
```

After the fourth probe the tree was restored and the guard reported:

```
--- Check 180: the report bridge's coverage table and its arms agree ---
PASS: the coverage table accounts for every result variant the engine declares.
PASS: the report bridge's coverage table and its arms agree.
FAIL=0
```

**A fifth, unplanned probe was needed and found.** The first version of the
guard extracted the `variants:` arrays with a single-line pattern. `rustfmt`
breaks a long array across lines, so after the table was formatted the
`integrations` row's three variants were no longer on one line — and the
guard reported a *false* failure:

```
FAIL: the coverage table and the engine's result variants disagree.
  only in the result type:
    Integrations
    IntegrationsCreateIssue
    IntegrationsSearchIssues
```

The guard's extractions are now multiline-aware (`rg -U`, and an awk that
accumulates across lines), so a reformatted table reads as the same table.
The lesson is recorded here because the next maintainer will hit it: **a
guard that reads formatted source must read it the way `rustfmt` writes it.**



### Integration verification (2026-10-24, session owner)

The milestone's own commands were run in isolation; the owner's integration
step then ran the repository's mandatory contract over the whole tree, with
all four milestones landed together.

| Command | Result |
|---|---|
| `make check` | **exit 0** — `cargo fmt --all --check`, the no-default-features and per-crate checks, `cargo deny --workspace --all-features check` (all five sub-checks), clippy, the full test suite (**4315 tests across 77 test binaries, 0 failures**), and `scripts/check-architecture-guards.sh` (**190 guards, "ALL PASSED: No architecture drift detected"**) |
| `make check-msrv` | **exit 0** — all five steps green on Rust 1.89 |
| `python3 scripts/check_doc_references.py` | 29 missing references — the unchanged pre-existing baseline, none of them in a file this milestone touched |

This is the **first time `make check` has completed on this branch**: it had
always stopped at the inherited `check-deps` advisory backlog, which milestone
015 cleared. Three integration tests that milestone 010 had left stale were
found by that first complete run and corrected in `dcfcd4e2`; the corrections
are recorded as a dated addendum on milestone 010's closure record rather
than by rewriting it.

## 5. Invariant review

| Invariant | Evidence | Result |
|---|---|---|
| `EnforcementContext::evaluate()` is the only pre-dispatch gate; no authorization decision changed | The eight new arms are pure functions over a `TaskResult`; the coverage gate consults a *static table*, not a policy. `report_coverage.rs` contains no `EnforcementContext`, `Scope`, `ApprovedExecution`, `OperationMetadata` or policy import | **unchanged** |
| Strict surfaces fail closed | The milestone adds no dispatch path. The engine executor's dispatch is unchanged; only the *report derivation* after a task completed changed | **unchanged** |
| `OperationMetadata` is the single source of truth for operation policy | No inline policy check was added; the coverage table is about *report forms*, not policy | **unchanged** |
| One canonical source per fact | The coverage table lives in exactly one file (guard 180's third check); the engine publishes it; the daemon republishes it verbatim; the GUI derives its column from the published rows | **met** |
| Never `let _ =`, never `filter_map(\|e\| e.ok())`; log via `tracing` | The gate logs with `tracing::error!` (disagreement) and `tracing::debug!` (an excluded operation); no swallowed error was introduced | **met** |
| Ring-only TLS, no new TLS stack | No TLS code was touched | **unchanged** |
| Spawned tokio tasks need a 30–300 s timeout wrapper | No task was spawned | **unchanged** |
| Workspace Tokio baseline; DTO crates carry no Tokio | `eggsec-runtime` gained no Tokio usage: the coverage DTO is plain data, and the runtime's capture is synchronous at construction | **met** |
| The GUI links no engine/daemon/CLI/TUI crate (guard 155) | The GUI's new code touches only `eggsec-runtime::report` and `eggsec-daemon-protocol`'s published capabilities | **unchanged** |
| New architecture guards start at 177; **180 is this milestone's** | The guard block is written for check 180 only (`/tmp/eggsec-guardwork/m013-guards.sh`); no other number is used | **met** |
| The bridge stays pure and clock-free | No `Utc::now()`, `SystemTime` or clock read in the eight new arms; the new documents carry an empty `timestamp` (test `the_new_documents_carry_no_timestamp`) | **met** |
| The `OperationDescription` gains no field, and the protocol version does not change | `DaemonCapabilities` gained one **additive, defaulted** field; `DAEMON_PROTOCOL_VERSION` stays 2; `OperationDescription` is untouched | **met** |
| The canonical report model is unchanged | `crates/eggsec-report-model` was not edited; `ScanReportData` keeps its nine fields | **met** |

## 6. Failure and recovery review

| Situation | Behaviour | Evidence |
|---|---|---|
| A task with no report form | The bridge yields `None`; the executor inserts nothing; the daemon answers `TaskNotFound`; the client degrades to `Ok(None)`; the GUI says "no report" and **writes no file** | §3.4; `an_excluded_operation_produces_no_document` |
| A bridge/table disagreement | An internal error: logged at `error` level with the operation id, `None` returned, the task completes normally and has no report. Guard 180 makes it unreachable in the source | `a_canonical_operation_with_no_bridge_arm_is_loud`; guard probe 2 and probe 4 |
| A result that cannot be serialized | Unchanged: the executor logs `tracing::warn!` and inserts nothing; the task succeeds and the report does not exist | `crates/eggsec/src/runtime_bridge/executor.rs:395-400` |
| Cancellation | Unchanged: the report is derived *after* `race_with_cancel` resolves, so a cancelled task produces no result and no report. No new task, no new timeout, no detached work | `crates/eggsec/src/runtime_bridge/executor.rs` |
| Restart | The coverage table is a compile-time constant and survives a restart unchanged; the *store* does not, so a restarted daemon's reports are gone (unchanged). The **published coverage** is re-published by the new process, so the GUI's coverage table is correct immediately after a reconnect | the coverage is captured at construction and is immutable |
| Contention | The store's lock is unchanged (`std::sync::Mutex`, a map operation, never held across an await). The coverage is read once per `Capabilities` request and never changes, so it introduces no contention | `crates/eggsec-runtime/src/report.rs` |
| A daemon that publishes no coverage | The GUI says *unknown*, never "unavailable" | §3.6 |
| A daemon too old to publish the coverage | The field is `#[serde(default)]`, so it decodes as `None` and the GUI says *unknown* | `daemon_capabilities_decode_without_the_report_forms_field` |
| Duplicate delivery and idempotency | Unchanged: the coverage is a constant; the report store is keyed by task id and re-inserting a task replaces its document |
| Malformed or unauthorized input | Unchanged: the coverage is not an input path. The GUI's export still validates the task id and the format before anything else |

## 7. Migration and compatibility review

- **The wire is additive and defaulted.** `DaemonCapabilities.report_forms:
  Option<Vec<ReportFormEntry>>` with `#[serde(default)]`. An old client
  ignores an unknown field; an old daemon's message decodes with `None` in a
  new client. `DAEMON_PROTOCOL_VERSION` stays **2**, the same treatment
  `enabled_features` received.
- **The GUI against an old daemon.** The report-form column reads *unknown*,
  and the export message keeps its plain wording when no reason is published.
  The GUI never shows a report-form verdict it was not told.
- **The daemon against an old engine.** Not a combination the daemon can
  produce: the daemon links the engine or it does not. A daemon built with
  the `service` feature publishes twenty-eight rows; one without it publishes
  `None`.
- **`OperationDescription` is untouched.** No catalog consumer sees a new field.
- **The stored documents are unchanged in shape.** `ScanReportData` keeps its
  nine fields; the eight new variants produce documents of the same shape, so
  every renderer and consumer reads them unchanged.
- **No stored data migrates.** The coverage table is not persisted; the report
  store holds documents only, and it is bounded and in-memory.
- **Removal criteria.** None: nothing is deprecated or removed. If the
  canonical task-kind table grows a variant, the coverage table must grow a
  row in the same change — that is guard 180's first property.

## 8. Security review

- **Authorization.** Unchanged. The milestone adds no dispatch path and makes
  no authorization decision. The coverage gate reads a static table; it cannot
  permit anything the enforcement kernel would refuse.
- **Scope.** Unchanged. No scope check was added, removed or weakened.
- **The published coverage carries no target, no banner and no credential.**
  `ReportFormEntry` holds an operation id, a verdict and a reason sentence.
  The reasons are written by the engine from the *shape* of the result types
  (which field is missing), not from any observed value: none of them names a
  host, a URL, a banner, a credential or a finding title. This is the
  "published coverage carries no target" acceptance criterion, and it is the
  reason the reasons are hand-written sentences rather than derived strings.
- **The GUI names no operation id.** The GUI joins the published coverage to
  its rows by the published catalog's operation id, so it holds no table of its
  own (guard 161 re-verified: PASS).
- **Path validation.** Unchanged: the export path's destination validation and
  backup discipline are untouched.
- **Privilege boundaries.** Unchanged: the GUI remains daemon-only and links
  no engine crate.
- **Denial-of-service bounds.** Unchanged: the report store's bound of 64 is
  untouched, and the coverage table is a compile-time constant with no
  runtime cost.
- **Audit behaviour.** Unchanged: the milestone adds no command and no
  authorisation, so no audit record is required. The exclusion reason is
  published as *build* information, not as an audit event.

## 9. Documentation and operations

**Updated by this milestone**

| File | What changed |
|---|---|
| `docs/REPORT_EVIDENCE_MODEL.md` | the report read path's "Produce" row now names `report_for_operation`; a new "The report coverage table" section states the 28 rows, the 17/17/1 arithmetic, the exclusion list with its reasons, the clean-mapping criterion, and the publication path; a new "The `timestamp` semantics" section states the three semantics and the empty case |
| `docs/DAEMON.md` | a new "The published report-form coverage" subsection under **Capabilities**, stating the verbatim republication and the `None`-means-unknown rule |
| `architecture/daemon.md` | the `DaemonCapabilities` field table gains `report_forms`; the report path gains the coverage-table paragraph and the corrected `file:line` references |
| `architecture/runtime.md` | the module table, the `RuntimeTaskExecutor` method table and the public-API table gain the coverage DTO and `report_forms()`; a new "The coverage DTO" subsection corrects the "no domain types" claim so it is true |
| `architecture/overview.md` | the guard list gains the Check 180 line |
| `AGENTS.md` | the dispatch/scope section gains the one-line statement that the coverage table is the single source of truth |

**Shared documents — deltas handed to the owner, not applied**

| File | Delta |
|---|---|
| `docs/CI_ARCHITECTURE_GUARDS.md` | `/tmp/eggsec-guardwork/m013-guards-doc.md` — a new "Report-Bridge Coverage Invariants" section registering Check 180 |
| `.opencode/skills/eggsec-gui/SKILL.md` | `/tmp/eggsec-guardwork/m013-skill.md` — the "Enforced by" guard range, a new "The report-form column" subsection, and the reason-bearing export message |
| `architecture/desktop_gui.md` | `/tmp/eggsec-guardwork/m013-desktop_gui.md` — the module table, a new "The report-form column" subsection, and a new "'No report' says why" subsection |

**Static guard**

- Check 180 lives in `/tmp/eggsec-guardwork/m013-guards.sh`, written to be
  appended to `scripts/check-architecture-guards.sh`. It was verified
  standalone against the tree (PASS) and against four mutations (four FAILs,
  §4.3). The owner merges it; milestone 013 does not edit the shared script.

**Operator diagnostics**

- A task with no report now names the reason: *"the daemon has no report for
  task `<id>`: `<reason>`"*. The reason is the engine's statement about the
  operation's result type, so it is stable across runs.
- The GUI's coverage table gains a **Report** column whose values are
  `available`, `unavailable` or `unknown`.
- The engine logs at `error` level if the bridge and the table ever disagree.

**Recovery instructions**

None: the milestone adds no persisted state, no migration and no operator
action. A daemon restart re-publishes the coverage unchanged.

## 10. Unresolved findings

| Severity | Finding | Impact | Required action |
|---|---|---|---|
| **medium** | **F1 — closure finding 8 (the empty `timestamp`), restated and widened.** Milestone 006's closure record §10.8 recorded that four families' documents carry an empty `timestamp`: `port-scan`, `fingerprint`, `fuzz` and `recon`. This milestone adds **eight** more families in the same state — `endpoint-scan`, `graphql`, `oauth`, `auth-test`, `nse`, `hunt`, `browser` and `compliance` — so the gap now covers **twelve** of the thirty-five variants. **It remains open.** The reason it was not closed here is the plan's: none of the twelve result types carries a timestamp of its own (`PortScanResults`, `FingerprintResults`, `FuzzSession`, `FullReconResult` and the eight new ones have no such field), so closing it requires either **adding a timestamp to twelve engine result types** — with their serializations and their downstream consumers (TUI, Python bindings, the CLI) — or **giving the bridge a clock**, which would contradict the module's purity contract (`report.rs:9-19`). Neither is a mapping. The eight new variants were therefore given an empty `timestamp`, matching the majority precedent and keeping the bridge clock-free | twelve of thirty-five report documents carry no timestamp; a renderer that sorts by time sees them first, and an operator cannot tell when a scan ran | Two ways to close it, both outside a mapping milestone: (a) add a `timestamp` field to the twelve result types and fill it at production time, or (b) give the bridge an injected clock and a purity contract that permits a time read. Whichever is chosen must be one decision for all twelve, and the coverage table's timestamp semantics must then state which of the four semantics each family carries |
| **low** | **F2 — the guard reads formatted source.** Guard 180 extracts the `variants:` arrays and the verdict fields from the coverage module's text. The first version used single-line patterns and produced a **false failure** after `rustfmt` wrapped the `integrations` row (§4.3). The extraction is now multiline-aware, but the same class of risk exists for any future reformat of the table | a false guard failure (noise), not a false pass | when the table's shape changes, re-run the guard standalone and check that a reformat alone does not move it |
| **low** | **F3 — the coverage table's variant count is asserted twice.** The Rust test asserts the union is 35 and the guard compares the enum's declarations with the table's names as **sets**. A deliberate new variant must therefore touch the enum, the table **and** the test's count — three places. That is the plan's intent ("the coverage table must grow a row in the same change"), but the count is a second thing to remember | a new variant fails the build in a place that is not the table | none; keep the count in the test next to the table it describes |
| **low** | **F5 — six of the plan's integration rows were not implemented.** §10's rows 25, 27, 28, 29, 30 and 37 — the end-to-end report read over a real Unix socket, the two daemon-restart rows, the cancellation row, the concurrent-readers row and the stored-document-shape row — need a live daemon socket, a restart harness or a second build, none of which this milestone's scoped verification provides. The behaviour they would exercise is **unchanged** by this milestone (the report read path, the store's bound and the cancellation ordering are untouched), and the nearest existing evidence is green: the daemon's 110 unit tests, the client's 15, the output renderers' 92, and the new `report_form_coverage` integration file's 3 | those six rows carry no milestone-013-specific evidence | the GUI qualification driver (milestone 014) supplies the live-daemon harness; the rows should be executed there, where a running daemon already exists |
| **low** | **F4 — the GUI's reason-bearing export message is not covered by an end-to-end test.** The pure half (`report_reason_for_variant`) is unit-tested, and the async half (resolving the task's variant from the GUI's registry) is not, because it needs a live daemon. The message therefore rests on the unit test plus the manual path | a regression in the variant→operation lookup would show up as the plain message, not as a wrong reason | an integration test against a live daemon, when the GUI's qualification driver (milestone 014) provides one |

## 11. Roadmap disposition

**Milestone closed and next dependency may proceed.**

Gap 11 of the desktop GUI frontend roadmap — "the report bridge covers only the
families with a clean mapping" — is **closed** by this milestone: the bridge
now has an arm for all seventeen variants that have a clean mapping (nine
pre-existing, eight new), the eighteen variants that do not are **named** with
the fact they are missing, and the reason reaches the operator through the
published coverage, the GUI's coverage table and the export message.

Closure finding 8 (the empty `timestamp`) is **restated, widened to twelve
variants, and left open** — see §10, finding F1. It is the only substantive
residual, and it needs a decision that is not a mapping.

The roadmap's own file (`plans/subsystems/desktop-gui-frontend-roadmap.md`)
and `plans/registry.md` are the plan owner's to update; this milestone edits
neither.

## 12. Registry updates

Required in `plans/registry.md` (the session owner's edit):

- Register milestone 013 as **closed**, with this closure record as its
  evidence.
- The milestone's headline: the report bridge gained a full-coverage table
  (17 canonical variants across 18 task kinds; 18 variants excluded with a
  reason each), the coverage is published by the engine, republished by the
  daemon and rendered by the GUI, and the operator can learn **why** a task has
  no report.

Required in `plans/subsystems/desktop-gui-frontend-roadmap.md` (the session
owner's edit):

- Close gap 11 and name what closed it: this milestone and this closure
  record.
- Note that closure finding 8 remains open and now covers twelve variants.
