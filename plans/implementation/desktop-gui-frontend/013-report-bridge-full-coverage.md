# Desktop GUI Frontend Milestone 013 — The Report Bridge's Full Coverage

Status: ready for handoff

Repository baseline: branch `feat/auto-20261022-76692679` at commit `821c8189`
("docs(plans): correct the roadmap's remaining stale approval claims"), with
milestones 001–011 implemented or closed, 012 planned, and milestone 006's
report-export path implemented but its report-bridge coverage gap still open.

Source roadmap:

- `plans/subsystems/desktop-gui-frontend-roadmap.md#milestone-13--the-report-bridges-full-coverage`

Corrects (the plan and closure record this milestone closes findings in):

- `plans/implementation/desktop-gui-frontend/006-full-operation-coverage-and-report-export.md`
- `plans/closure/desktop-gui-frontend/006-full-operation-coverage-and-report-export-closure.md`

Long-term requirements:

- `plans/000-long-term-specification.md#4-enforcement-invariants`
- `plans/000-long-term-specification.md#6-dispatch-runtime-and-wire-identity`
- `plans/000-long-term-specification.md#10-frontend-rules`

Applicable ADRs:

- `plans/adrs/ADR-0007-desktop-gui-frontend-boundary.md` (accepted)
- `plans/adrs/ADR-0009-gui-shell-technology-selection.md` (accepted)

Primary class: corrective

## 0. Baseline and supersession record

This is a **corrective** milestone. It does not amend, rewrite, or supersede
milestone 006's plan or its closure record: both stay as they are, and the
closure record's §10 remains the record of what was unresolved when 006 closed.
What this plan does is close the two findings 006 left open that belong to the
report bridge, with the regression evidence that would have caught them.

**The findings this milestone closes**, quoted from
`plans/closure/desktop-gui-frontend/006-full-operation-coverage-and-report-export-closure.md`:

> **§10.1 — "The report bridge covers only the families with a clean mapping.**
> Nine `TaskResult` families produce a document; every other variant yields 'no
> report form' rather than a lossy document. An operator can therefore export a
> report for some completed tasks and not for others. A workspace-wide bridge
> for all thirty variants is a separate corrective milestone: it is a mapping
> exercise, not a contract change, and doing it half-way would be worse than not
> doing it. **Recorded as roadmap known gap 11.**"
> (`…-closure.md:393-400`)

> **§10.8 — "The report bridge's `timestamp` field is empty for four families.**
> A pure bridge cannot stamp a clock, and those result types carry no scan
> timestamp, so the field is empty rather than invented. The pipeline family
> carries one and populates it."
> (`…-closure.md:433-436`)

Roadmap known gap 11 is the same finding, recorded at the roadmap level
(`plans/subsystems/desktop-gui-frontend-roadmap.md:317-327`) and owned by this
milestone (`…-roadmap.md:347`).

**What this milestone closes and what it does not.**

| Closure-006 finding | Disposition here |
|---|---|
| §10.1 — the bridge covers nine families; twenty-one families have no report form | **Closed.** Every **variant** with a clean mapping produces a document — 17 of the 35 `TaskResult` variants, against 9 today — and every variant without one is named, with the missing fact stated, in one table the bridge, the daemon and the GUI all read. §3.5 is that table; §6.1 is the table; §7 A–C build it. |
| §10.8 — the `timestamp` field is empty for four families | **Not closed, and not claimed.** §3.6.4 measures the gap at the baseline (four variants, no result type carries a scan timestamp) and states why closing it is a *result-type* change, not a bridge change. This milestone's new documents join the same four in carrying an empty timestamp, so the finding's scope grows from four variants to twelve and is restated as such. |
| §10.2 — the daemon unlinks the socket path before binding | **Out of scope** (roadmap gap 10, owned by milestone 012). |
| §10.3 — `convert_to_html_self_contained` has no GUI consumer | **Out of scope** and unaffected: the GUI's export format set is unchanged by this milestone (§3.8). |
| §10.4, §10.5 — pre-existing clippy debt in the `service` profile and in `eggsec-runtime`'s integration tests | **Out of scope** (inherited; `make check` does not gate on them). |
| §10.6 — `make check` stops at `check-deps` on the inherited advisory backlog | **Out of scope**: that backlog belongs to milestone 015 and is not this milestone's to fix. §11 records it as an inherited failure the implementer must record, not fix. |
| §10.7 — `make check-python` was not run at 006's closure | **Out of scope**: no Python binding, stub, or script changes in this milestone (§5). |

**Three corrections to measurements recorded elsewhere**, each verified against
source at this baseline, and each of which this plan depends on:

1. **The canonical 30 task kinds map to 28 distinct operation ids, not 26.**
   `TaskKind::operation_id()` (`crates/eggsec-runtime/src/request.rs:646-694`)
   has 30 arms; counting the distinct string literals in that range gives
   **28** (`pipeline` twice — `Pipeline` and `Resume` at `:656`/`:662`; `packet`
   twice — `PacketTraceroute` and `PacketSend` at `:669`/`:670`). Plan 010's
   §3.2 records "26", and `plans/registry.md:206` repeats "26" and adds that
   "`Wireless`/`WirelessActive` share `wireless`" — both stale: milestone 010
   (ADR-0011) split the deauth onto its own operation identity,
   `wireless-deauth` (`:689`), deliberately. The **28** count is what this
   milestone's coverage table is keyed by, so the implementer must not copy
   "26" from the older documents.
2. **The GUI does not render reports through `eggsec-output`'s
   `RENDERER_REGISTRY`.** `RENDERER_REGISTRY` lives in
   `crates/eggsec-ui-model/src/renderer_registry.rs:27` and is a registry of
   **23 result renderers** keyed by scan kind (`port-scan`, `graphql`, `c2`, …)
   — it decides how a *task result* is drawn. The GUI's **export** path renders
   through `eggsec-output`'s six `convert_to_*` functions
   (`crates/eggsec-gui/src/export.rs:52-55`). The two are different registries
   over different data. §3.8 measures both.
3. **`MAX_RETAINED_REPORTS` is 64 with oldest-first eviction**, as the briefing
   stated: `crates/eggsec-runtime/src/report.rs:29` and `:72-81`. Nothing in
   this milestone changes the bound; the bound is recorded here only so the
   implementer does not "fix" it while touching the store.

## 1. Objective

Make the engine's report bridge **total**: every task-kind variant whose
result carries the facts the canonical report model needs produces a report
document, and every variant whose result does not is **named**, with the missing
fact stated, in one table — so an operator is told *which* completed tasks can
be exported and *why* the others cannot.

Three concrete outcomes:

1. **A coverage table with one row per canonical operation id (28 rows).** Each
   row says whether that operation's result has a clean report mapping and, if
   not, which fact the result is missing. It is the **single source of truth**:
   the bridge consults it, the daemon publishes it, the GUI renders it, and it
   is not restated anywhere.
2. **Eight new report mappings** — endpoint-scan, graphql, oauth, auth-test,
   nse, hunt, browser and compliance. The coverage goes from 9 mapped variants
   to 17 of the 35, and from 10 task kinds to 18.
3. **The operator is told the reason.** The GUI's coverage table gains a
   report-form verdict per task kind, and the export failure names the reason
   when the GUI can resolve it — instead of the bare "there is nothing to
   export".

**What this milestone explicitly does not deliver.** It does not change what a
report *contains* for the variants that already have one, does not change the
canonical report model, does not change the export format set, does not touch
the CLI's own report path, and does not give any variant a timestamp it does
not have (§3.6.4).

## 2. Why this milestone is ready

Closed hard dependencies:

- **Milestone 006** (implemented 2026-10-24) — the bridge
  (`crates/eggsec/src/runtime_bridge/report.rs`), the bounded store
  (`crates/eggsec-runtime/src/report.rs`), the daemon's report read path
  (`crates/eggsec-daemon/src/host.rs:711-735`), the client's degradation
  (`crates/eggsec-frontend-client/src/daemon.rs:257-283`) and the GUI's export
  path (`crates/eggsec-gui/src/commands.rs:663-706`) all exist. This milestone
  extends them; it re-founds none of them.
- **Milestone 010** (implemented 2026-10-24) — the daemon publishes and admits
  every canonical capability name, and the operation identity of the deauth was
  corrected onto `wireless-deauth`. The coverage table is keyed by operation id,
  so that correction is a prerequisite, not a peer.
- **Milestone 011** (implemented 2026-10-24) — the daemon holds an operation at
  its confirmation point. Not a dependency of the bridge, but it is why the
  hazardous operations are now *offered* in the GUI at all, which is what makes
  their coverage visible to an operator.

Stable interface dependencies:

- `TaskKind` — 30 variants, declared once in the `define_task_kinds!` table
  (`crates/eggsec-runtime/src/request.rs:142-173`), with
  `capability_name()` (`:106-110`), `operation_id()` (`:646-694`) and
  `canonical_target()` (`:707+`).
- `TaskResult` — 35 variants (`crates/eggsec/src/dispatch/types.rs:80-156`),
  the engine's result type.
- `outcome_kind` — the engine's canonical **result-family** discriminator
  (`crates/eggsec/src/dispatch/canonical_execution.rs:909-963`): 32 string
  literals, of which 30 are result families and two (`error`, `unknown`) are
  not.
- `ScanReportData` — the canonical document
  (`crates/eggsec-report-model/src/report.rs:12-24`).
- `RuntimeTaskExecutor` — the seam between the runtime and whatever executes
  (`crates/eggsec-runtime/src/runtime.rs:41-110`), already carrying
  `supported_task_kinds()` (`:71-73`), `enabled_features()` (`:92-94`) and
  `report_for()` (`:96-110`).
- `DaemonCapabilities` — the daemon's published build facts
  (`crates/eggsec-daemon-protocol/src/protocol.rs:48-81`), already carrying
  `enabled_features: Option<Vec<String>>` (`:80`) with the "`None` means
  unknown" contract.

The milestone is ready **because the seam the coverage travels over already
exists**: the daemon already publishes one executor-derived build fact
(`enabled_features`) through the same message the GUI's coverage table already
reads, with the same "`None` means unknown" discipline.

## 3. Current implementation evidence

### 3.1 The five stages of the report path, at this baseline

| Stage | Where | What happens |
|---|---|---|
| Produce | `crates/eggsec/src/runtime_bridge/report.rs:54-73` | `report_for_result(&TaskResult) -> Option<ScanReportData>` derives the canonical document from a completed result. The module doc calls the bridge **pure**: no I/O, no state, no clock (`report.rs:9-19`). |
| Keep | `crates/eggsec/src/runtime_bridge/executor.rs:367-380` | The executor inserts the document into the runtime's bounded store — but **only** `if let Some(report) = super::report::report_for_result(&task_result)`. A result with no document leaves no store entry at all. |
| Keep (the store) | `crates/eggsec-runtime/src/report.rs:47-51`, `:72-81` | `RuntimeReportStore` retains at most `MAX_RETAINED_REPORTS = 64` documents, evicting oldest-first. |
| Serve | `crates/eggsec-daemon/src/host.rs:711-735` | `ClientCommand::GetTaskReport` → `Runtime::report_for(&task_id)` (`crates/eggsec-runtime/src/runtime.rs:201-203`) → the executor's store read (`crates/eggsec/src/runtime_bridge/executor.rs:171-173`). A task with no document is answered `ErrorCode::TaskNotFound` with the message `"no report is available for this task"` (`host.rs:729-733`). |
| Read | `crates/eggsec-frontend-client/src/daemon.rs:257-283` | `get_task_report()` maps `TaskReport` → `Ok(Some(..))` and maps **both** `UnsupportedCommand` and `TaskNotFound` → `Ok(None)` (`:269-276`). `Ok(None)` means "no report available" and must not be treated as a failure (`:251-256`). |

### 3.2 The canonical counts, measured from source

| Set | Count | How it was counted | Owner |
|---|---:|---|---|
| `TaskKind` variants | **30** | the `define_task_kinds!` table, one arm per line | `crates/eggsec-runtime/src/request.rs:142-173` |
| distinct capability names | **30** | one per table row | same table (`:106-110` generates the match) |
| distinct operation ids | **28** | 30 arms, `pipeline` twice, `packet` twice | `crates/eggsec-runtime/src/request.rs:646-694` |
| `TaskResult` variants | **35** | the enum's variant arms | `crates/eggsec/src/dispatch/types.rs:80-156` |
| result-producing `TaskResult` variants | **34** | 35, minus the terminal `Error` shape | derived |
| result families (`outcome_kind`) | **30** | 32 string literals, minus `error` and `unknown` | `crates/eggsec/src/dispatch/canonical_execution.rs:909-963` |
| bridge arms (variants with a document) | **9** | the `TaskResult::` arms in the match | `crates/eggsec/src/runtime_bridge/report.rs:56-70` |
| …of which unconditional | **5** | the arms with no `#[cfg]` above them | `crates/eggsec/src/runtime_bridge/report.rs:56-60` |
| …of which feature-gated | **4** | `wireless`, `wireless-advanced`, `db-pentest`, `c2` | `crates/eggsec/src/runtime_bridge/report.rs:61-70` |
| variants with **no** document today | **26** | 35 − 9 | derived |
| task kinds with a document today | **10** | 9 variants, of which `pipeline` is produced by two kinds (`Pipeline`, `Resume`) | derived |
| `RENDERER_REGISTRY` entries | **23** | the `kind:` literals | `crates/eggsec-ui-model/src/renderer_registry.rs:29-217` |
| `eggsec-output` report converters | **7** | `convert_to_junit`, `convert_to_sarif`, `convert_to_html`, `convert_to_html_self_contained`, `convert_to_markdown`, `convert_to_csv`, `convert_to_json` | `crates/eggsec-output/src/convert.rs:19,53,82,102,148,190,209` |
| GUI export formats | **6** | `ReportFormat::ALL` | `crates/eggsec-gui/src/export.rs:80-87` |

**30 kinds, 35 result variants, and the relationship between them is not
one-to-one.** The bridge matches on the **`TaskResult` variant**, not on the
task kind, so the variant is the unit a coverage table must be keyed by.

- `Pipeline` and `Resume` both produce `TaskResult::Pipeline` — **one variant,
  two kinds** (−1).
- `Waf` produces either `TaskResult::WafDetection` or `TaskResult::WafBypass`
  — **one kind, two variants** (+1).
- `Storage` produces `Storage`, `StorageListScans` and `StorageListFindings` —
  **one kind, three variants** (+2).
- `Integrations` produces `Integrations`, `IntegrationsCreateIssue` and
  `IntegrationsSearchIssues` — **one kind, three variants** (+2).
- `TaskResult::Error(String)` is a **terminal error shape**, not a task's
  result: it is the thirty-fifth variant and has no kind.

30 kinds + 1 (Waf) + 2 (Storage) + 2 (Integrations) − 1 (Pipeline/Resume) =
**34 result-producing variants**, plus `Error` = **35**.

**The three multi-variant kinds are where an "every completed task produces a
report" claim fails quietly.** `Waf`, `Storage` and `Integrations` each yield
several result shapes, so a verdict stated per kind is only true if *every*
variant that kind can produce agrees. The coverage table therefore records the
verdict per **variant**, and requires that all variants of one operation share
one verdict — which is why `Waf`'s two variants, `Storage`'s three and
`Integrations`'s three each get their own row.

### 3.3 How the bridge dispatches, exactly

The bridge dispatches on the **result type**, not on the task kind
(`crates/eggsec/src/runtime_bridge/report.rs:54-73`):

```rust
pub fn report_for_result(result: &TaskResult) -> Option<ScanReportData> {
    match result {
        TaskResult::PortScan(scan)      => Some(port_scan_report(scan)),      // :56
        TaskResult::Pipeline(report)    => Some(pipeline_report(report)),     // :57
        TaskResult::Fingerprint(results)=> Some(fingerprint_report(results)), // :58
        TaskResult::Fuzz(session)       => Some(fuzz_report(session)),        // :59
        TaskResult::Recon(recon)        => Some(recon_report(recon)),         // :60
        #[cfg(feature = "wireless")]
        TaskResult::Wireless(scan)      => Some(crate::wireless::to_scan_report_data(scan)),   // :62
        #[cfg(feature = "wireless-advanced")]
        TaskResult::WirelessActive(a)   => Some(crate::wireless::active::to_active_scan_report_data(a)), // :64
        #[cfg(feature = "db-pentest")]
        TaskResult::DbPentest(report)   => Some(crate::db_pentest::to_scan_report_data_db(report)), // :68
        #[cfg(feature = "c2")]
        TaskResult::C2(report)          => Some(crate::c2::to_scan_report_data(report)),           // :70
        _ => None,                                                                   // :71
    }
}
```

Four properties follow, and all four matter to this milestone:

1. **A variant with no arm yields `None` silently.** The catch-all at `:71` is
   the whole behaviour for the **twenty-six** unmapped variants: no document,
   no log line, no reason. The executor's `if let Some(..)`
   (executor.rs:372) then inserts nothing, so the store has no entry and the
   daemon answers `TaskNotFound`.
2. **The dispatch key is the result variant, so a *task kind's* verdict is only
   as precise as its variants' verdicts.** `Waf` is the clearest case: its two
   variants must agree, or the GUI could not say anything true about "the
   `waf` task kind produces a report".
3. **The feature gates are on the arms, not on the result variants alone.**
   `wireless`, `wireless-advanced`, `db-pentest` and `c2` arms exist only under
   their engine features, matching the `#[cfg(feature = ..)]` on the result
   variants themselves (`crates/eggsec/src/dispatch/types.rs:144-154`). Any new
   arm for a feature-gated variant (`nse`, `advanced-hunting`,
   `headless-browser`, `compliance`) must carry the same gate. **A daemon
   built without those four features silently loses four more variants' reports**
   — the arms are absent, not degraded.
4. **The bridge has its own `scan_type` vocabulary, and it is a second table.**
   `report.rs:75-87` defines a private `mod scan_type` with **five**
   discriminators — `port-scan`, `pipeline`, `fingerprint`, `fuzz`, `recon` —
   used by the five unconditional arms. The four feature-gated arms emit no
   label of their own: they delegate to the domain bridges, which supply their
   own (`"wireless"` at `crates/eggsec/src/wireless/mod.rs:664`,
   `"wireless-active-{attack_type}"` at `crates/eggsec/src/wireless/active/mod.rs`,
   `"c2"` at `crates/eggsec/src/c2/mod.rs:488`, and the report's own
   `scan_type` at `crates/eggsec-db-lab/src/bridge.rs:65`). The coverage table
   therefore does **not** extend `mod scan_type`; it lives beside it (§3.7).

### 3.4 The canonical report model's required shape

`ScanReportData` (`crates/eggsec-report-model/src/report.rs:12-24`):

| Field | Type | Required? |
|---|---|---|
| `target` | `String` | **yes** — no default, no skip |
| `scan_type` | `String` | **yes** |
| `timestamp` | `String` | **yes** |
| `findings` | `Vec<FindingData>` | **yes** |
| `open_ports` | `Vec<PortData>` | **yes** |
| `services` | `Vec<ServiceData>` | **yes** |
| `duration_ms` | `u64` | **yes** |
| `wireless_networks` | `Vec<WirelessNetworkReportData>` | optional (`skip_serializing_if` empty) |
| `policy_summary` | `Option<PolicySummary>` | optional |

`FindingData` (`:28-38`) needs a `title`, a `severity`, a `category`, a
`description` and a `location`; `evidence`, `remediation` and `cwe_ids` are
optional.

**The model's own tolerance, established by the bridge at the baseline.** The
bridge already distinguishes two kinds of missing fact, and the distinction is
the criterion this milestone applies:

- A missing **timestamp** is a *decorative* gap. The bridge leaves the field
  empty rather than invent one: `port-scan` (`:118`), `fingerprint` (`:180`),
  `fuzz` (`:230`), `recon` (`:290`), with the reasoning stated in the module
  doc (`report.rs:11-16`). The pipeline variant carries a real one
  (`:158-162`, from the run manifest's `started_at`).
- A missing **duration** is likewise tolerated: the recon bridge writes
  `duration_ms: 0` (`:294`) rather than inventing one.
- A missing **identity** is *not* tolerable. `target` is the document's
  subject. A document whose `target` is empty does not say what it is about,
  and every renderer carries that emptiness into the artifact (the SARIF and
  CSV renderings key their rows off the finding's location and the report's
  subject; the JSON export carries `"target": ""`).

**The clean-mapping criterion this milestone applies**, stated once and used
for every row of §3.5:

> A result **variant** has a **clean report mapping** when its result carries
> (C1) an **identity** — a target or an equivalent subject — so the document
> can say what it describes, and (C2) at least one **content** item that maps
> onto the canonical model without a fact being invented: a finding that
> carries its own severity, title, description and location, or a port,
> service, or wireless-network record. A missing timestamp or duration does
> not disqualify a variant; a missing identity does.

### 3.5 The variant coverage table — the plan's central evidence

One row per **`TaskResult` variant** — 35 rows, in the enum's declaration
order (`crates/eggsec/src/dispatch/types.rs:80-156`), because the bridge
matches on the variant. "Today" is what `report_for_result` does **now** at
this baseline; "Verdict" is what this milestone makes it do. Every row's
"Today" is a statement about the match arms in §3.3; every row's "Verdict" is
a statement about the result type named in the first column, whose fields are
cited.

| # | `TaskResult` variant | Task kind | Operation id | Today | Verdict | The fact that decides it |
|---:|---|---|---|---|---|---|
| 1 | `LoadTest(LoadTestResults)` | `LoadTest` | `load-test` | nothing | **no clean mapping** | identity ✓ (`target_url`), duration ✓, content ✗ — `errors: Vec<String>` are raw transport-error strings with no severity, title or location, and `status_codes` is a count map (`crates/eggsec/src/loadtest/metrics.rs:118-138`). A document would be a target, a duration and an empty findings list. |
| 2 | `StressTest { .. }` | `StressTest` | `stress-test` | nothing | **no clean mapping** | identity ✓, duration ✓ (`stats.duration_ms`), content ✗ — `packets_sent`, `bytes_sent`, `errors` are counters (`crates/eggsec/src/stress/metrics.rs:78-83`). No service, no port, no finding. |
| 3 | `PortScan(PortScanResults)` | `PortScan` | `scan-ports` | **document** (`:56`) | **canonical** (unchanged) | identity ✓ (`host`), content ✓ — `open_ports` verbatim including UDP verdicts (`crates/eggsec/src/scanner/ports/mod.rs:272-286`) and the distinct detected services. |
| 4 | `EndpointScan(EndpointScanResults)` | `EndpointScan` | `scan-endpoints` | nothing | **canonical** (new) | identity ✓ (`base_url`), duration ✓, content ✓ — `results: Vec<EndpointResult>` (`crates/eggsec/src/scanner/endpoints.rs:449-457`) carries `path`, `status_code`, `status_text`, `redirect` and the `interesting` flag. The interesting endpoints become findings; the uninteresting ones do not (the fuzz bridge's rule, `report.rs:190-193`). |
| 5 | `Fingerprint(FingerprintResults)` | `Fingerprint` | `fingerprint` | **document** (`:58`) | **canonical** (unchanged) | identity ✓ (`host`), content ✓ — the identified services. |
| 6 | `WafDetection(WafDetectionResult)` | `Waf` | `waf-detect` | nothing | **no clean mapping** | identity ✗ — the detection carries `waf_name`, `confidence`, `matched_headers`, `matched_cookies`, `matched_patterns`, `server_header`, `status_code` (`crates/eggsec/src/waf/detector/types.rs:6-20`) but **no target**: it lives in the request parameters, and the bridge is a pure function of the result. |
| 7 | `WafBypass { .. }` | `Waf` | `waf-detect` | nothing | **no clean mapping** | identity ✗ — the bypass carries `technique`, `success`, `description`, `payload`, `status_code`, `response_diff` (`crates/eggsec/src/waf/bypass/mod.rs:63-71`) and **no target** either. Both variants of the `waf` kind agree, so the kind's verdict is unambiguous. |
| 8 | `WafStress(Vec<BypassResult>)` | `WafStress` | `waf-stress` | nothing | **no clean mapping** | identity ✗, duration ✗, content ✗ — a bare list of bypass attempts with no subject. |
| 9 | `Pipeline(PipelineReport)` | `Pipeline`, `Resume` | `pipeline` | **document** (`:57`) | **canonical** (unchanged) | identity ✓ (`report.target`), content ✓ — prioritized findings, ports and services; the only variant with a real timestamp (`report.rs:158-162`). |
| 10 | `Fuzz(FuzzSession)` | `Fuzz` | `fuzz` | **document** (`:59`) | **canonical** (unchanged) | identity ✓ (`target_url`), content ✓ — the payloads the session found vulnerable, with their own `detected_severity` (`crates/eggsec/src/fuzzer/engine/types.rs:22`). |
| 11 | `Recon(FullReconResult)` | `Recon` | `recon` | **document** (`:60`) | **canonical** (unchanged) | identity ✓ (`target`), content ✓ — detected secrets and *vulnerable* takeover candidates only (`report.rs:263-265`). Duration is unknown and stays 0 (`report.rs:294`). |
| 12 | `PacketCapture { .. }` | `PacketCapture` | `packet-capture` | nothing | **no clean mapping** | identity ✗ — the variant names no interface (it lives in the request, and the operation is interface-bound) and carries only `packets_captured` and `output_file` (`crates/eggsec/src/dispatch/types.rs:99-102`); the captured traffic has no collection in the canonical model. |
| 13 | `PacketTraceroute { .. }` | `PacketTraceroute` | `packet` | nothing | **no clean mapping** | identity ✗ — the hops (`hop`, `address`, `rtt_ms`, `crates/eggsec/src/dispatch/types.rs:50-54`) do not name the host the trace ran against; content ✗ — a hop is not a finding. |
| 14 | `PacketSend { .. }` | `PacketSend` | `packet` | nothing | **no clean mapping** | identity ✗, duration ✗, content ✗ — two counters (`packets_sent`, `bytes_sent`). |
| 15 | `GraphQl(GraphQlResults)` | `GraphQl` | `graphql` | nothing | **canonical** (new) | identity ✓ (`target`), duration ✓, content ✓ — `depth_limit_bypassed`, `alias_overload_vulnerable` and `injection_findings: Vec<String>` (`crates/eggsec/src/dispatch/types.rs:12-21`). The result carries no severity, so the bridge assigns per-class constants (§6.2). |
| 16 | `OAuth(OAuthResults)` | `OAuth` | `oauth` | nothing | **canonical** (new) | identity ✓ (`target`), duration ✓, content ✓ — `redirect_vulnerabilities`, `scope_vulnerabilities`, `state_vulnerabilities`, `grant_vulnerabilities` (`crates/eggsec/src/dispatch/types.rs:25-34`). Severity is per class (§6.2). |
| 17 | `Nse(NseResults)` | `Nse` | `nse` | nothing | **canonical** (new) | identity ✓ (`target`), content ✓ — `report: Option<eggsec_nse::NseRunReport>`, whose `evidence` items already have a severity mapping in the engine (`crates/eggsec/src/nse_bridge.rs:37-47`). No duration → 0. |
| 18 | `Hunt(HuntReport)` | `Hunt` | `hunt` | nothing | **canonical** (new) | identity ✓ (`target`), content ✓ — `attack_chains`, `business_logic`, `race_conditions`, `authz_bypasses`, `session_issues`, each with its own `severity` (`crates/eggsec/src/hunt/mod.rs:134-142`; `crates/eggsec/src/hunt/chain.rs:7-15`). No duration → 0. |
| 19 | `Browser(BrowserReport)` | `Browser` | `browser` | nothing | **canonical** (new) | identity ✓ (`target`), content ✓ — `dom_xss: Vec<DomXssFinding>` (`crates/eggsec/src/browser/xss_dom.rs:7-16`) and `client_issues: Vec<ClientIssue>` (`crates/eggsec/src/browser/client_checks.rs:6-14`), each with its own severity, location, evidence and remediation. `spa_routes` are routes, not findings. No duration → 0. |
| 20 | `Compliance(ComplianceReport)` | `Compliance` | `compliance` | nothing | **canonical** (new) | identity ✓ (`target`), content ✓ — `findings: Vec<ComplianceFinding>` (`crates/eggsec/src/compliance/mod.rs:22-39`), each with `requirement_id`, `description`, `severity`, `status` and `remediation`; only the **failed** requirements become findings. No duration → 0. |
| 21 | `Storage` | `Storage` | `storage` | nothing | **no clean mapping** | The variant is a unit: it acknowledges a storage operation and carries no data at all. One of three variants of this kind. |
| 22 | `StorageListScans { .. }` | `Storage` | `storage` | nothing | **no clean mapping** | The operation reads a catalogue: it lists stored scans (`StoredScan`, `crates/eggsec/src/storage/models.rs:8-15`). A catalogue read is not a scan of a target, and the canonical model describes exactly one subject. |
| 23 | `StorageListFindings { .. }` | `Storage` | `storage` | nothing | **no clean mapping** | As above: it lists stored findings (`StoredFinding`, `crates/eggsec/src/findings/lifecycle.rs:50-58`) — a lifecycle view, not a scan result. |
| 24 | `Integrations` | `Integrations` | `integrations` | nothing | **no clean mapping** | The variant is a unit: it acknowledges an integration operation and carries no data at all. One of three variants of this kind. |
| 25 | `IntegrationsCreateIssue { .. }` | `Integrations` | `integrations` | nothing | **no clean mapping** | The operation creates a tracker issue (`Issue`, `crates/eggsec/src/integrations/mod.rs:37-47`). An issue is a work item with a title, labels and assignees — not a fact about a target — and no target is carried. |
| 26 | `IntegrationsSearchIssues { .. }` | `Integrations` | `integrations` | nothing | **no clean mapping** | The operation lists tracker issues — the same reasoning as row 25. |
| 27 | `Workflow(WorkflowReport)` | `Workflow` | `workflow` | nothing | **no clean mapping** | identity ✗ — the report has `total_findings`, `open_findings`, `in_progress_findings`, `resolved_findings`, `sla_violations` and `findings`, and **no target field** (`crates/eggsec/src/workflow/mod.rs:24-31`). Its findings are well shaped (`crates/eggsec/src/findings/mod.rs:252-278`), but a workflow run is a lifecycle view over stored findings, not a scan of one subject. |
| 28 | `Vuln(VulnAssessment)` | `Vuln` | `vuln` | nothing | **no clean mapping** | identity ✗ — the assessment carries `mode`, `assessed_at`, `cvss_score`, `exploit_info`, `asset_criticality`, `prioritized_findings`, `triage_results`, `remediation_plans` and `summary`, but no target (`crates/eggsec/src/vuln/mod.rs:37-46`); and its `PrioritizedFinding` (`crates/eggsec/src/vuln/prioritizer.rs:80-87`) carries no location either. |
| 29 | `Wireless(WirelessScanResult)` | `Wireless` | `wireless` | **document** (`:62`, `cfg`) | **canonical** (unchanged) | identity ✓ (`interface`), content ✓ — the networks and the analyzer's findings (`crates/eggsec/src/wireless/mod.rs:608-670`). The arm exists only under the `wireless` feature. |
| 30 | `WirelessActive(ActiveWirelessAttackResult)` | `WirelessActive` | `wireless-deauth` | **document** (`:64`, `cfg`) | **canonical** (unchanged) | identity ✓ (`interface`), content ✓ — the active bridge's own findings (`crates/eggsec/src/wireless/active/mod.rs`). The arm exists only under the `wireless-advanced` feature. |
| 31 | `Auth(AuthTestReport)` | `AuthTest` | `auth-test` | nothing | **canonical** (new) | identity ✓ (`target`), content ✓ — `findings: Vec<AuthFinding>` (`crates/eggsec/src/auth/mod.rs:33-46`, `:61-66`), each with its own `severity`, `title`, `description` and `recommendation`. No duration in the result → 0. |
| 32 | `DbPentest(DbPentestReport)` | `DbPentest` | `db-pentest` | **document** (`:68`, `cfg`) | **canonical** (unchanged) | identity ✓ (`target`), content ✓ — the lab's findings (`crates/eggsec-db-lab/src/bridge.rs:9-73`). The arm exists only under the `db-pentest` feature. |
| 33 | `Intercept(InterceptSession)` | `Intercept` | `proxy-intercept` | nothing | **no clean mapping** | identity ✗ — `target: Option<String>`, absent whenever the session had no target (`crates/eggsec-web-proxy/src/intercept/types.rs:313-329`); content ✗ — the variant is captured flows (`ProxyFlow`) plus the operator's own manipulation log (`ManipulationRecord`, `:280-295`), and the canonical model has no collection for captured exchanges. |
| 34 | `C2(C2Report)` | `C2` | `c2` | **document** (`:70`, `cfg`) | **canonical** (unchanged) | identity ✓ (`target`), content ✓ — beacon results and task results (`crates/eggsec/src/c2/mod.rs:387-494`). The arm exists only under the `c2` feature. |
| 35 | `Error(String)` | — (terminal shape) | — | nothing | **no clean mapping** | Not a task's result but a **terminal error shape** (`crates/eggsec/src/dispatch/types.rs:155`): the task failed. There is no scan, no subject and no duration to report, so a "report" of a failure would be a document with an empty target and no findings — the lossy document the bridge refuses. The coverage table names it explicitly so the guard can account for all 35 variants. |

**The tally, by variant and by kind.**

| Verdict | `TaskResult` variants | Task kinds |
|---|---:|---:|
| canonical today (unchanged) | **9** (5 unconditional + 4 feature-gated) | 10 |
| canonical, **new** in this milestone | **8** | 8 |
| **no clean mapping** (named exclusions) | **18** | 12 |
| **Total** | **35** | **30** |

The eighteen excluded variants are: `LoadTest`, `StressTest`, `WafDetection`,
`WafBypass`, `WafStress`, `PacketCapture`, `PacketTraceroute`, `PacketSend`,
`Storage`, `StorageListScans`, `StorageListFindings`, `Integrations`,
`IntegrationsCreateIssue`, `IntegrationsSearchIssues`, `Workflow`, `Vuln`,
`Intercept`, `Error`. Every one of them is listed with the fact its result is
missing, in the table above and in the coverage table this milestone builds.

The twelve excluded **kinds** are the same twelve as the excluded variants'
kinds, each of which yields only excluded variants: `LoadTest`, `StressTest`,
`Waf` (×2), `WafStress`, `PacketCapture`, `PacketTraceroute`, `PacketSend`,
`Storage` (×3), `Integrations` (×3), `Workflow`, `Vuln`, `Intercept` — that is
twelve kinds producing seventeen variants, and the terminal `Error` shape is the
eighteenth.

### 3.6 What the export path does today for a task with no report

**It already degrades correctly.** The exit condition's third clause — "the
export path reports 'no report available' rather than an empty artifact" — is
**already satisfied at the baseline**, so this milestone supplies regression
evidence for it rather than a code change to it. The full trace:

| Stage | Where | What it does |
|---|---|---|
| Bridge | `crates/eggsec/src/runtime_bridge/report.rs:71` | `_ => None` — no document for an unmapped variant. |
| Executor | `crates/eggsec/src/runtime_bridge/executor.rs:372-380` | `if let Some(report) = ..` — nothing is inserted, so the store has **no entry** for the task. |
| Runtime | `crates/eggsec-runtime/src/runtime.rs:201-203` | `report_for` → `executor.report_for(task_id)` → `None`. |
| Daemon | `crates/eggsec-daemon/src/host.rs:722-733` | `None` → `ServerMessage::Error { code: ErrorCode::TaskNotFound, message: "no report is available for this task" }`. |
| Client | `crates/eggsec-frontend-client/src/daemon.rs:269-276` | `TaskNotFound` → `Ok(None)` — a degradable state, not a failure. |
| GUI export | `crates/eggsec-gui/src/commands.rs:684-691` | `Ok(None)` → `Err("the daemon has no report for task `{task_id}`, so there is nothing to export")`. The error is raised **before** `export::render` (`:697`) and `export::write_export` (`:699`), so **no file is written**. |
| GUI view | `crates/eggsec-gui/src/commands.rs:733-737` | `Ok(None)` → `Err("the daemon has no report for task `{task_id}`, so there is nothing to show")`. |

**No empty artifact is possible on this path.** A file is written only after a
document has been fetched, deserialized and rendered. And the difference
between "an honest report with no findings" and "no report" stays visible: the
six renderers handle an empty `findings` list correctly — `convert_to_csv`
writes its header row and no rows (`crates/eggsec-output/src/convert.rs:190-207`),
`convert_to_junit` builds a suite with no cases (`:19-51`), `convert_to_sarif`
builds a document with no results (`:53-80`) — so a mapped variant with nothing
to report produces a *valid, empty* artifact, which is a different thing from
"no report available".

**Two honest observations about the current degradation.**

1. The daemon answers "no report" with `ErrorCode::TaskNotFound`
   (`host.rs:731`). The task exists; the *report* does not. The client maps
   that code to `Ok(None)` (`daemon.rs:273-276`), so the wire is not wrong
   about the outcome — but the code is also what a genuinely unknown task id
   produces, so a client cannot tell "this task has no report" from "this task
   does not exist" over the wire. **This milestone does not change that code**:
   the protocol's error-code set is a published contract
   (`crates/eggsec-daemon-protocol/src/protocol.rs:86-101`), and a new code
   would be a contract change outside a mapping milestone. The *reason* reaches
   the operator through the coverage table instead (§6.5).
2. The GUI's message says the daemon has no report; it does not say **why**.
   That is the milestone's user value, and §6.6 closes it.

#### 3.6.4 The `timestamp` gap, measured

The four variants whose documents carry an empty `timestamp` are `port-scan`
(`report.rs:118`), `fingerprint` (`:180`), `fuzz` (`:230`) and `recon` (`:290`).
The pipeline variant carries a real one (`:158-162`). The four *domain* bridges
the engine reuses are a third case: `wireless` stamps `chrono::Utc::now()`
(`crates/eggsec/src/wireless/mod.rs:665`) and `wireless-active` does the same
(`crates/eggsec/src/wireless/active/mod.rs`), i.e. the **report-generation**
time, while `c2` carries the report's own `timestamp`
(`crates/eggsec/src/c2/mod.rs:489`) and `db-pentest` the report's own
`timestamp` (`crates/eggsec-db-lab/src/bridge.rs:66`).

So the workspace has **three** timestamp semantics and one empty case:

| Semantics | Families |
|---|---|
| the scan's own start time | `pipeline` |
| the report's own timestamp field | `c2`, `db-pentest` |
| the report-generation time (a clock read) | `wireless`, `wireless-active` |
| empty (no clock read) | `port-scan`, `fingerprint`, `fuzz`, `recon` |

None of the four *empty* variants' result types carries a timestamp:
`PortScanResults` (`crates/eggsec/src/scanner/ports/mod.rs:272-286`),
`FingerprintResults` (`crates/eggsec/src/scanner/fingerprint.rs:132-139`),
`FuzzSession` (`crates/eggsec/src/fuzzer/engine/types.rs:153-169`) and
`FullReconResult` (`crates/eggsec/src/recon/mod.rs:214+`) have no such field. Closing closure
finding 8 therefore requires **adding a timestamp to those result types** — a
change to four engine result types, their serializations and their downstream
consumers (TUI, Python bindings, the CLI) — or **giving the bridge a clock**,
which would contradict the purity contract the module doc states
(`report.rs:9-19`). Neither is a mapping. This milestone therefore:

- leaves the four existing variants exactly as they are;
- gives the **eight new** variants an empty `timestamp`, matching the majority
  precedent and keeping the bridge clock-free;
- **restates closure finding 8 as covering twelve variants rather than four**,
  and records that it stays open, with the two ways to close it named.

### 3.7 Where the no-clean-mapping list lives, and why

The list must be **one** table, and it must be readable by the engine (which
decides), by the daemon (which publishes) and by the GUI (which renders). The
dependency direction settles it. Three candidates were measured against it:

**Candidate 1 — `eggsec-report-model`.** It is a leaf data crate depending
only on `eggsec-core` (`crates/eggsec-report-model/Cargo.toml:16-18`), and the
engine could read it through `eggsec-output`'s re-export
(`crates/eggsec-output/src/convert.rs:5-7`) without a manifest change. It is
still **rejected**, for two reasons. First, the crate owns the *document*
contracts — `ScanReportData`, `FindingData`, the envelope — and a fact about
which engine result types have a mapping is not a document contract; putting
it there would make the data crate carry a statement about one consumer's
implementation. Second, and decisively: the GUI would then hold the list
locally, which would make the GUI's answer about *its own build* rather than
about the running daemon, and which guard 161 forbids outright (§3.7.1).

**Candidate 2 — `OperationMetadata` in `eggsec-policy`.** It is the engine's
single source of truth for operation *policy*, and AGENTS.md is explicit that
"no inline policy checks" live outside it. But a report-form verdict is not an
authorization fact: it grants nothing, refuses nothing, and no policy decision
reads it. The policy crate is also "deterministic authorization semantics only
(data + pure algorithms; no Tokio/network/filesystem/frontends)", and its
catalog is keyed by the **34 catalog operations**, which do not line up with
the 30 task kinds, the 28 operation ids or the 35 result variants this
milestone is about. `OperationMetadata` also has no per-*variant* dimension at
all — and the variant is the unit the bridge dispatches on. **Rejected**: wrong
owner, wrong key, wrong granularity.

**Candidate 3 — the engine, beside the bridge.** The engine owns the report
semantics: `crates/eggsec/src/runtime_bridge/report.rs` is where a result
becomes a document, and the coverage table is the statement of which results
can. The engine links the policy and report crates; the daemon links the
engine under its `service` feature (`crates/eggsec-daemon/Cargo.toml`); the
protocol carries a neutral DTO the runtime already owns; and the GUI links
neither the engine nor the daemon. **Chosen.**

The chosen shape therefore is: **the table lives in the engine**
(`crates/eggsec/src/runtime_bridge/report_coverage.rs`, §6.1), is keyed by
**operation id** with the **result variants** it covers, is **published** by
the engine through the executor seam (§6.4) and **republished verbatim** by the
daemon, and is **rendered** by the GUI from what the daemon published. No crate
restates it, and no crate other than the engine holds the fact.

#### 3.7.1 Why the GUI cannot hold it either

The GUI's coverage model is **derived from what the daemon publishes**
(`crates/eggsec-gui/src/coverage.rs:1-27`): the task kinds and operations from
the catalog, the capability list the daemon advertises, the enabled-feature set
the daemon publishes, and the denials the GUI has learned from live failures.
Guard 161 (`scripts/check-architecture-guards.sh:4932-5000`) enforces that
nothing is transcribed: it extracts every operation id and capability name from
their owners (`crates/eggsec-policy/src/catalog.rs`,
`crates/eggsec-runtime/src/capabilities.rs`) and **fails** if any GUI
production source or frontend file names one.

A coverage table held in the GUI would therefore (a) fail guard 161 the moment
it named an operation id, and (b) violate the GUI's own stated contract that
"a task kind the engine publishes tomorrow is covered by this code without a
line being added" (`coverage.rs:13-15`). The table must come **from the
daemon**, and the daemon must get it from the engine, which owns the bridge.

### 3.8 The second gap: the export format set and the renderer registry

**There is no per-format gap in the GUI's export path.** The GUI offers six
formats (`crates/eggsec-gui/src/export.rs:63-87`): JSON, CSV, HTML, SARIF,
JUnit and Markdown. `eggsec-output` provides all six plus one more:
`convert_to_json` (`crates/eggsec-output/src/convert.rs:209`),
`convert_to_csv` (`:190`), `convert_to_html` (`:82`), `convert_to_sarif` (`:53`),
`convert_to_junit` (`:19`), `convert_to_markdown` (`:148`), and
`convert_to_html_self_contained` (`:102`). The GUI imports exactly the six it
offers (`crates/eggsec-gui/src/export.rs:52-55`), and `ReportFormat::ALL` is
the single list the frontend receives (`export.rs:170-175`, delivered as
`report_formats` in `crates/eggsec-gui/src/commands.rs:283`). PDF is
deliberately absent: the PDF renderer lives in the engine
(`crates/eggsec/src/output/pdf.rs`) and the GUI links no engine
(`crates/eggsec-gui/src/export.rs:58-60`).
`convert_to_html_self_contained` is the one converter with no GUI consumer, and
that is closure 006 §10.3's already-recorded state, not a defect: the CSP
forbids embedding the engine's document
(`crates/eggsec-gui/src/export.rs:20-42`).

**The renderer registry is a different thing, and it has a real gap of its
own.** `RENDERER_REGISTRY` (`crates/eggsec-ui-model/src/renderer_registry.rs:27`)
holds **23** entries, and it is how a task *result* is drawn — not how a report
is exported. Its lookup key is the **result envelope's** `kind`
(`crates/eggsec-ui-model/src/result_view.rs:23`;
`crates/eggsec-gui/src/tasks.rs:446`).

That envelope kind is a **third vocabulary**: `task_result_envelope`
(`crates/eggsec/src/dispatch/canonical_execution.rs:975-1131`) assigns one
`kind` string per `TaskResult` variant — 35 strings, of which `storage` and
`integration` each appear three times. The registry's 23 kinds cover 28 of
them. **Seven envelope kinds have no renderer**: `packet-send`, `waf-bypass`,
`storage`, `integration`, `workflow`, `vuln`, `wireless-active`. Those results
degrade gracefully rather than breaking: the view falls back to the label
`"Unknown"`, an empty summary-field set, `supports_rich_tui = false` and
`supports_json_detail = true`
(`crates/eggsec-ui-model/src/result_view.rs:26-32`;
`crates/eggsec-gui/src/tasks.rs:463-464`).

**Conclusion for §3.8: the second gap is real, and this milestone scopes it
OUT — explicitly, with the reason.** The renderer registry is about *drawing a
result*; the bridge is about *producing a document*. A task kind can have a
renderer and no report (all twelve excluded kinds do), and can have no
renderer and no report, and neither is a defect in the other. Adding seven
renderers is a *result-rendering* piece of work with its own acceptance
criteria, and the milestone's boundary is the report bridge. Nothing in this
milestone changes the format set, the renderer registry, or the export
rendering.

The adjacent observation worth recording is that the workspace now holds
**three** vocabularies over the same 35 result variants — `outcome_kind`'s 32
diagnostic strings (`canonical_execution.rs:909-963`), the envelope's 35
`kind` strings (`:975-1131`), and the renderer registry's 23
(`renderer_registry.rs:29-217`). Consolidating them would put a UI-model
concern and an engine concern in one table in one crate, which is the wrong
owner for both; it is a **possible future consolidation**, explicitly out of
scope here (§5).

### 3.9 The publishing seam this milestone reuses

The daemon already publishes one executor-derived **build fact** through the
message the GUI's coverage table already reads:

- `RuntimeTaskExecutor::enabled_features()` —
  `crates/eggsec-runtime/src/runtime.rs:92-94`, default `None` meaning
  "unknown" (`:83-88`).
- The engine overrides it —
  `crates/eggsec/src/runtime_bridge/executor.rs:159-168` — because that
  executor links the engine, so its feature set is knowable.
- The daemon publishes it verbatim —
  `crates/eggsec-daemon/src/host.rs:90-96`, `:171`.
- The wire carries it as `DaemonCapabilities.enabled_features:
  Option<Vec<String>>` — `crates/eggsec-daemon-protocol/src/protocol.rs:80` —
  with a decode-without-the-field test at `:1522-1526`.
- The GUI reads it and treats `None` as *unknown*, never as "nothing is
  enabled" — `crates/eggsec-gui/src/coverage.rs:17-23`, `:196-211`.

**The coverage table travels the same way.** The engine publishes it, the
daemon publishes it verbatim, the GUI joins it by operation id. `None` means
"this daemon does not publish the fact", and the GUI must then say *unknown*,
exactly as it says unknown for the feature set.

## 4. Invariants that must not regress

1. **`EnforcementContext::evaluate()` remains the pre-dispatch gate, and strict
   surfaces still fail closed.** This milestone adds no dispatch path, no
   authorization decision and no operator override. The coverage table is
   published metadata; it grants nothing.
2. **The task envelope contract is untouched.**
   `TaskResultEnvelope.payload` stays `json!({})` and `.artifacts` stays
   `vec![]` (`crates/eggsec/src/dispatch/canonical_execution.rs:1114`). The
   report body travels on its own path
   (`crates/eggsec/src/runtime_bridge/executor.rs:367-380`).
3. **The report document is still produced only by the engine.** The daemon
   derives nothing and re-renders nothing; it serves what the executor
   produced (`architecture/daemon.md:145`).
4. **The runtime's report store stays bounded** at
   `MAX_RETAINED_REPORTS = 64`, oldest-first
   (`crates/eggsec-runtime/src/report.rs:29`, `:72-81`).
5. **The GUI links no engine, daemon, CLI or TUI crate.** The GUI's new
   knowledge arrives over the wire as published data.
6. **No parallel lookup tables.** One table states the coverage. The GUI must
   not name an operation id or a reason string in production code (guard 161),
   and the daemon must not restate the table — it publishes the engine's.
7. **The canonical report model's shape does not change.** `ScanReportData`
   keeps its nine fields (`crates/eggsec-report-model/src/report.rs:12-24`);
   the bridge fills them, it does not extend them.
8. **The export path never writes an artifact it could not render.** A failure
   is an `Err`, and the file is written only after the render succeeds
   (`crates/eggsec-gui/src/export.rs:353+`, `commands.rs:697-699`).
9. **The GUI's Content Security Policy is unchanged**, and the engine's HTML
   document is still never injected into the webview (guard 162,
   `scripts/check-architecture-guards.sh:5002+`).
10. **The bridge stays clock-free.** No new arm reads a clock; the eight new
    variants carry an empty `timestamp` like the four existing ones
    (§3.6.4).
11. **No new dependency, no new TLS stack, ring-only.** The DTO this milestone
    adds is `serde`-only, inside a crate that already depends on `serde`.
12. **No `let _ =` and no `filter_map(|e| e.ok())`.** Every fallible step in
    the new code logs through `tracing`.

## 5. Scope

### In scope

- The engine's **coverage table**: one row per canonical operation id (28),
  each listing the **`TaskResult` variants** it covers and stating the verdict
  and, for each exclusion, the missing fact.
- The **eight new bridge arms**: endpoint-scan, graphql, oauth, auth-test, nse,
  hunt, browser, compliance — taking the bridge from 9 mapped variants to 17
  of the 35.
- The bridge's **verdict gate**: the bridge consults the table, and a
  Canonical verdict with no arm is a loud internal error, not a silent `None`.
- The **runtime DTO** and the **`RuntimeTaskExecutor` method** that publishes
  the table, the **protocol field** that carries it, and the **daemon's
  publication** of it.
- The **GUI's coverage row** (a report-form verdict and its reason) and the
  **export failure message** (the reason, when the GUI can resolve it).
- **Guard 180** (the coverage table is total, and the bridge agrees with it)
  and the negative tests for it.
- The **documentation** that states the coverage: `docs/REPORT_EVIDENCE_MODEL.md`,
  `architecture/daemon.md`, `architecture/desktop_gui.md`,
  `architecture/runtime.md`, and the `eggsec-gui` skill.

### Explicitly out of scope

- **The CLI's own report path.** `eggsec report convert` and the CLI's scan
  commands read and write their own reports
  (`crates/eggsec/src/commands/handlers/report.rs`); the runtime bridge is not
  on that path.
- **The export format set, the renderers and `RENDERER_REGISTRY`** (§3.8).
- **The canonical report model's shape** — no new field, no new type.
- **A timestamp for the variants that have none** (closure finding 8, §3.6.4):
  closing it means changing four engine result types or giving the bridge a
  clock; neither is a mapping, and the finding is restated as open rather than
  silently closed.
- **The `ErrorCode` used for "no report"** (§3.6). The protocol's error-code set
  is a published contract; changing it is a contract change, not a mapping.
- **The daemon's socket-path hardening** (roadmap gap 10, milestone 012) and
  **the advisory backlog** (milestone 015).
- **The `RENDERER_REGISTRY` / coverage-table consolidation** and **any
  consolidation of the timestamp semantics** beyond stating them (§3.6.4,
  §3.8).
- **Python bindings, stubs and scripts.** `make check-python` is therefore not
  a gate for this milestone (§11).

## 6. Required production changes

### Core/domain

**6.1 — The coverage table (`crates/eggsec/src/runtime_bridge/report_coverage.rs`,
new module beside `report.rs`).**

A `const` table with **one row per canonical operation id — 28 rows**, each
row carrying:

| Field | Type | Meaning |
|---|---|---|
| `operation_id` | `&'static str` | the canonical operation id (`crates/eggsec-runtime/src/request.rs:646-694`) |
| `variants` | `&'static [&'static str]` | the `TaskResult` variants that operation can produce (`crates/eggsec/src/dispatch/types.rs:80-156`) — the unit the bridge dispatches on |
| `form` | `ReportForm` | `Canonical` or `NoCleanMapping` |
| `reason` | `&'static str` | **non-empty** when `NoCleanMapping`: the fact the result is missing, in operator-readable words |

The twenty-eight rows are **exactly** the twenty-eight distinct operation ids,
and their contents are derived from §3.5's table rather than re-decided here:
the `variants` lists are §3.5's rows **grouped by operation id**, and the
`form` column is §3.5's verdict under the rule that **all variants of one
operation share the verdict**. Three operations carry more than one variant —
`waf-detect` (`WafDetection`, `WafBypass`), `storage` (`Storage`,
`StorageListScans`, `StorageListFindings`) and `integrations` (`Integrations`,
`IntegrationsCreateIssue`, `IntegrationsSearchIssues`) — and in all three the
variants agree, which is what makes a per-kind verdict possible at all.

**All 35 variants are accounted for, exactly once**: 17 Canonical variants
(9 already mapped + 8 new, covering 18 task kinds) and 18 named exclusions
(covering 12 task kinds and the terminal `Error` shape). Each `reason` is
§3.5's "the fact that decides it" shortened to one operator-readable sentence,
and each names a **field or an absence** in the result type — so every reason
is checkable against the source, not against an opinion.

**The table is the single source of truth.** The bridge consults it; the
engine publishes it; the daemon republishes it verbatim; the GUI renders it.
Nothing restates it.

**6.2 — The eight new bridge arms** (`crates/eggsec/src/runtime_bridge/report.rs`).
Each arm is a small `fn` beside the existing five, and each is behind the same
`#[cfg(feature = ..)]` gate as the result variant it matches
(`nse` → `nse`, `hunt` → `advanced-hunting`, `browser` → `headless-browser`,
`compliance` → `compliance`; the other four are ungated).

| New arm | Document's `target` | Document's `findings` | Other fields |
|---|---|---|---|
| `endpoint_scan_report` | `results.base_url` | one per **interesting** `EndpointResult`: title `format!("interesting endpoint {}", path)`, severity `info`, category `endpoint-discovery`, description from `status_code`/`status_text`/`redirect`, location `path` | `duration_ms` from the result; `open_ports`/`services` empty; `timestamp` empty |
| `graphql_report` | `results.target` | `depth_limit_bypassed` → one finding (severity `medium`, category `graphql-depth-limit`); `alias_overload_vulnerable` → one finding (severity `medium`, category `graphql-alias-overload`); each string in `injection_findings` → one finding (severity `high`, category `graphql-injection`, location the target) | `duration_ms` from the result; `timestamp` empty |
| `oauth_report` | `results.target` | one finding per entry of `redirect_vulnerabilities` (`medium`, `oauth-redirect`), `scope_vulnerabilities` (`high`, `oauth-scope`), `state_vulnerabilities` (`medium`, `oauth-state`), `grant_vulnerabilities` (`high`, `oauth-grant`) | `duration_ms` from the result; `timestamp` empty |
| `auth_report` | `report.target` | one per `AuthFinding`: its own `severity`, `title`, `description`, `recommendation` as the remediation; category `auth-<test_type>`; location the target | `duration_ms` 0; `timestamp` empty |
| `nse_report` | `results.target` | one per `NseRunReport.evidence` item, with the severity the engine's own NSE bridge assigns to that evidence kind (`crates/eggsec/src/nse_bridge.rs:37-47`), title and description from the evidence item, location the target | `duration_ms` 0; `timestamp` empty |
| `hunt_report` | `report.target` | one per attack chain, business-logic flaw, race condition, authz bypass and session issue, each with its own `severity`, title/description and location where the type carries one; category `hunt-<kind>` | `duration_ms` 0; `timestamp` empty |
| `browser_report` | `report.target` | one per DOM XSS finding and one per client issue, each with its own `severity`, `description`, `location`, `evidence` and `remediation`; category `browser-dom-xss` / `browser-client-issue`. The SPA routes are **not** findings | `duration_ms` 0; `timestamp` empty |
| `compliance_report` | `report.target` | one per **failed** requirement (`ComplianceFinding` with a failing status), with its own `severity`, `requirement_id`, `description` and `remediation`; category `compliance-<framework>`; location the target | `duration_ms` 0; `timestamp` empty |

**The severity rule, stated once.** A finding's severity comes from the
result's own severity field whenever the result has one (`auth-test`, `nse`,
`hunt`, `browser`, `compliance` — five of the eight). Where the result has
none (`graphql`, `oauth`, `endpoint-scan`), the bridge assigns a **per-class
constant** recorded in the function's doc comment, drawn from
`eggsec_core::types::Severity`: `info` for a discovered endpoint, `medium` for
a depth-limit bypass, an alias overload, an open redirect and a
state-parameter weakness, `high` for an injection, a scope weakness and a
grant weakness. **Per-finding invention is forbidden** — the severity is a
property of the vulnerability class, not of the target.

**The "only the positive case is a finding" rule**, extended from the fuzz
bridge (`:190-193`): a rejected payload, an uninteresting endpoint, a passing
compliance requirement and a takeover candidate that probed safe or unknown
are not findings.

**6.3 — The verdict gate.** `report_for_result` keeps its result-type match —
that is how a document is *built* — and a new entry point consults the table
*first*:

```rust
/// The report document for a completed result, if this operation has one.
///
/// The verdict comes from the coverage table — the single source of truth —
/// and the document comes from the result. `operation_id` is the canonical
/// operation identity of the task kind that produced the result
/// (`TaskKind::operation_id()`).
pub fn report_for_operation(operation_id: &str, result: &TaskResult) -> Option<ScanReportData>
```

- The verdict is `NoCleanMapping` → return `None`, and log the table's reason
  at `debug` level so a daemon operator can find it in the log. **No document
  is invented.**
- The verdict is `Canonical` → build the document from the result. If the
  result has no arm, that is a **bridge/table disagreement — an internal
  error**, and it is logged at `error` level with the operation id, never
  swallowed and never panicked. Guard 180 makes the disagreement
  unreachable in the source; the log is the belt to that braces.
- The function stays **pure**: no I/O, no state, no clock. Logging is not a
  side effect on the document.

The executor's call site moves from
`super::report::report_for_result(&task_result)` to
`super::report::report_for_operation(&operation_id, &task_result)`. The
executor already computes `operation_id` from the request's task kind
(`crates/eggsec/src/runtime_bridge/executor.rs:222`), so **no plumbing is
added** — the string is already in scope.

### Storage and migrations

None. The runtime's bounded report store is unchanged: same key type, same
`MAX_RETAINED_REPORTS = 64`, same oldest-first eviction
(`crates/eggsec-runtime/src/report.rs:29`, `:72-81`). The store holds the same
kind of value (`serde_json::Value`) and the same documents; only the *number*
of variants that produce one changes.

The coverage table is **not stored**: it is a compile-time constant in the
engine, published per process. Nothing is persisted, so there is no migration
and no stale-copy risk on disk.

### Protocol and DTOs

**6.4 — Publishing the coverage.**

1. **A neutral DTO in `eggsec-runtime`** — the crate the trait, the protocol
   and the GUI all already depend on (`crates/eggsec-runtime/Cargo.toml:16-29`;
   `crates/eggsec-daemon-protocol/Cargo.toml:10-17`;
   `crates/eggsec-gui/Cargo.toml:37-48`). It is `serde`-only, needs no
   `eggsec-core` type, and does **not** require the `schema` feature:

   ```rust
   /// Whether an operation's result has a clean report mapping.
   pub enum ReportForm { Canonical, NoCleanMapping }

   /// One operation's report-form verdict.
   pub struct ReportFormEntry {
       pub operation_id: String,
       pub form: ReportForm,
       /// Why the operation has no report form; empty for a canonical one.
       pub reason: String,
   }
   ```

   It lives in `crates/eggsec-runtime/src/report.rs`, beside the store. **The
   module's doc comment must be corrected** when the DTO lands: it says "The
   payload is `serde_json::Value` rather than a report-model type on purpose:
   the runtime crate deliberately carries no domain types"
   (`crates/eggsec-runtime/src/report.rs:15-18`). That remains true of the
   *document payload*; the coverage *verdict* is a small typed DTO, and the
   doc must say so rather than leave the claim false.

2. **One defaulted trait method** on `RuntimeTaskExecutor`, beside
   `enabled_features()` (`crates/eggsec-runtime/src/runtime.rs:92-94`):

   ```rust
   /// The report-form coverage this executor's build publishes.
   ///
   /// `None` means **unknown**: the executor links no engine, or does not
   /// publish the fact — the same distinction `enabled_features()` draws.
   /// `Some(vec![])` means the executor published an empty coverage, which is
   /// a different, knowable fact.
   fn report_forms(&self) -> Option<Vec<ReportFormEntry>> { None }
   ```

   The default is `None`, so every other implementor — the CLI's noop
   executor, the daemon's test doubles — keeps compiling and keeps publishing
   nothing.

3. **The engine implements it**, in the same executor that implements
   `enabled_features()` (`crates/eggsec/src/runtime_bridge/executor.rs:159-168`):
   it returns the coverage table's twenty-eight rows, projected into the DTO.

4. **The wire carries it** in `DaemonCapabilities`, beside
   `enabled_features` (`crates/eggsec-daemon-protocol/src/protocol.rs:48-81`):

   ```rust
   /// The report-form coverage the connected daemon's build publishes.
   ///
   /// `None` means **unknown** — the daemon links no engine, or predates the
   /// field. `#[serde(default)]` so a newer daemon's capabilities still
   /// decode against an older client, and vice versa, exactly as
   /// `enabled_features` does.
   #[serde(default)]
   pub report_forms: Option<Vec<ReportFormEntry>>,
   ```

   The DTO type is reused from `eggsec-runtime` — the protocol crate already
   carries `eggsec_runtime::SessionScope` in a command
   (`crates/eggsec-daemon-protocol/src/protocol.rs:126-133`), so this adds no
   new crate edge and no second DTO.

   **The operation description is NOT touched.** `OperationDescription` keeps
   exactly its fifteen fields, and the pin test
   `response_carries_no_credential_or_scope_material`
   (`crates/eggsec-daemon-protocol/src/operations.rs:244-274`) is untouched.
   The coverage rides on the capabilities message because it is a **build fact
   about the executor**, which is what that message already carries.

5. **The daemon publishes it verbatim**, beside the enabled-feature set
   (`crates/eggsec-daemon/src/host.rs:90-96`, `:171`): the host asks its
   runtime's executor for the coverage and publishes what it gets, `None`
   included. The daemon **derives nothing** and **restates nothing**.

6. **The protocol version stays 2** — an additive, defaulted field on an
   existing message, the same shape as `enabled_features`.

### Runtime and concurrency

**6.5 — The executor's call site.**

- The executor's insert path keeps its current shape
  (`crates/eggsec/src/runtime_bridge/executor.rs:367-380`): a document is
  inserted only when the bridge returns one, and a serialization failure is
  logged, never swallowed (`tracing::warn!` at `:375-378`).
- No new task is spawned, no timeout wrapper is needed, and the store's lock
  discipline is unchanged: the store's critical section is a map operation
  and is never held across an await
  (`crates/eggsec-runtime/src/report.rs:38-40`).
- The bridge's call moves from `report_for_result(&task_result)` to
  `report_for_operation(&operation_id, &task_result)`; the `operation_id` is
  already in scope (`crates/eggsec/src/runtime_bridge/executor.rs:222`), so no
  plumbing is added and no task's lifetime changes.

### Frontend or operator surface

**6.6 — The GUI's coverage row and export message.**

- The GUI's coverage command gains the published coverage as an input
  (`crates/eggsec-gui/src/commands.rs:650-661`), and the coverage row gains
  two fields (`crates/eggsec-gui/src/coverage.rs:60-73`):

  ```rust
  /// Whether this task kind's operation produces a report document.
  pub report_form: ReportFormVerdict,
  /// Why, in operator-readable words. Empty when a report is available.
  pub report_reason: String,
  ```

  with `ReportFormVerdict` being the GUI's own three-state projection of the
  published DTO: `Available`, `Unavailable`, and **`Unknown`** — the last
  meaning "the daemon did not publish the fact", exactly as `None` means
  unknown for the feature set (`crates/eggsec-gui/src/coverage.rs:17-23`).
  **The GUI must never render "Unavailable" from an unpublished fact.**
- The GUI's **export failure message** gains the reason when it can resolve
  it: the export command knows the task id
  (`crates/eggsec-gui/src/commands.rs:670-691`), and the GUI's task registry
  carries each task's wire variant (`crates/eggsec-gui/src/tasks.rs:542-569`),
  so the GUI can map the task to its task kind, to its operation id, and to
  the published verdict. When it resolves one, the message becomes
  `"the daemon has no report for task `{task_id}`: {reason}"`; when it cannot
  (an unknown variant, or an unpublished coverage), the message keeps its
  current wording. **The message never claims a reason the GUI does not have.**
- The frontend renders the new cell in the coverage table
  (`crates/eggsec-gui/frontend/app.js:329-338` renders the `reachability` and
  `reason` cells today). The frontend holds **no catalog of its own**: the
  cell's text comes from the Rust side, as the existing cells do.

### Security and authorization

- **Nothing in this milestone changes an authorization decision.** The
  coverage table is published metadata about report *forms*; it grants no
  capability, widens no scope and overrides no risk flag.
- **The published data is not sensitive.** The table's rows name operation
  ids and the missing fact of a result type. No target, no banner, no finding
  text and no credential passes through it. The daemon's read levels are
  unchanged: the capabilities message is readable by any client that may read
  the catalog.
- **The report document itself is unchanged in what it carries.** The eight
  new arms map facts the engine already produced. The report read path's
  permission level stays `Observer`
  (`architecture/daemon.md:144`), the same as `GetTaskResult`.
- **The GUI's report-display discipline is unchanged.** The report still
  travels to the webview as data, the engine's HTML is still never injected,
  and the CSP is untouched (guard 162).
- **The new findings are derived from the engine's own results**, so the
  attacker-influenced content question is the one the existing variants
  already answer: the report's text is rendered as text by the GUI, and the
  exported artifact is the operator's file.

### Documentation and static guards

**6.7 — Guard 180** (new; the highest existing guard number is 176, at
`scripts/check-architecture-guards.sh:5874`). **The number 180 is reserved
for this guard**, in the execution order of the four planned milestones:
milestone 012 takes 177–179, this milestone takes **180**, milestone 014 takes
181–189, and milestone 015 takes 190. The reservation follows the precedent of
milestone 007's plan, which took 168 because the guards of milestones 006,
008, and 009 landed first. An implementer who takes this milestone out of
milestone order must re-derive the number from the script's then-current
highest, and must not reuse a number another planned milestone has reserved.
The house style is a `SECTION_FAIL` counter, `rg` probes over named source
files, and a `PASS`/`FAIL` line — see check 161 (`:4932-5000`) and check 176
(`:5874-5893`). Guard 180 is called
**"the report coverage is total, and the bridge agrees with it"**, and it
checks four properties, each of which is textual and feature-agnostic:

1. **Every `TaskResult` variant is either mapped or named.** The guard
   extracts the 35 variant names from the enum
   (`crates/eggsec/src/dispatch/types.rs:80-156`) and the variant lists from
   the coverage table
   (`crates/eggsec/src/runtime_bridge/report_coverage.rs`), and asserts that
   the two sets are **equal** — every variant listed exactly once, none
   missing, none duplicated. A new `TaskResult` variant with no coverage row
   **fails** the guard, and so does a variant listed under two operations.
2. **Totality of the operation ids.** The coverage table's operation ids are
   exactly the distinct operation ids of the canonical task-kind table. The
   guard extracts the table's ids and the canonical ids from
   `crates/eggsec-runtime/src/request.rs:646-694`, and compares the two sets.
   A new task kind with no coverage row fails the guard.
3. **No unknown operation.** The same comparison in the other direction: a
   coverage row naming an operation the canonical table does not have fails.
4. **The bridge agrees with the table.** The guard extracts the `TaskResult::`
   arms from `report_for_result`
   (`crates/eggsec/src/runtime_bridge/report.rs:54-73`) and asserts that the
   set of variants with an arm is **exactly** the set of variants under
   `Canonical` rows. A `Canonical` row with an armless variant fails; a
   `No clean mapping` row whose variant has an arm fails. **Because the arms
   are read as text, the four feature-gated arms
   (`crates/eggsec/src/runtime_bridge/report.rs:61-70`) count the same way in
   a build with or without their features** — the check is feature-agnostic by
   construction.

**The guard must not false-positive.** Three properties make it safe: it reads
**source text**, not a compiled build, so it is independent of the feature set
(the `#[cfg]` attributes do not remove text); it compares **sets**, so the
order of the arms and the order of the rows are irrelevant; and it reads the
canonical owners rather than a copy, so it cannot disagree with the table it
checks.

**6.8 — A guard extension, not a new guard.** Guard 161
(`scripts/check-architecture-guards.sh:4932-5000`) already forbids the GUI's
production sources and frontend files from naming an operation id or a
capability name. This milestone adds no new GUI-side table, so guard 161
already covers the new risk. The implementer MUST confirm that guard 161 still
passes with the new coverage column in place; if it does not, the defect is in
the GUI's implementation, not in the guard.

**6.9 — Documentation.** §12 lists the files. The load-bearing ones are
`docs/REPORT_EVIDENCE_MODEL.md` (its "The report read path" table at `:145-168`
states the nine-variant coverage and must state the new one and the exclusion
set), `architecture/daemon.md` (the report path at `:139-157`), and
`architecture/desktop_gui.md` (the coverage model and the export path).

## 7. Ordered work packages

### Work package A — The coverage table

**Intent.** One table states which canonical operations produce a report
document, and which do not, with the missing fact named, and names every
`TaskResult` variant it accounts for. It is the single source of truth
everything else reads.

**Required changes.** The new module
`crates/eggsec/src/runtime_bridge/report_coverage.rs`: the 28 rows of §6.1,
the `ReportForm` enum, a lookup by operation id and a lookup by result variant.
The module doc states §3.4's criterion in full, so a reader can check any row
against it.

**Acceptance evidence.** A unit test asserting the table's operation ids are
exactly the distinct ids of `TaskKind::all_variants()`
(`crates/eggsec-runtime/src/request.rs:119-121`) — derived, not transcribed —
that its variant lists cover the 35 `TaskResult` variants exactly once each,
and that every `NoCleanMapping` row's reason is non-empty and names a field or
an absence.

### Work package B — The eight new bridge arms

**Intent.** Every variant with a clean mapping produces a document.

**Required changes.** The eight functions of §6.2 and their arms in
`report_for_result`, each behind the same `#[cfg(feature = ..)]` gate as its
result variant.

**Acceptance evidence.** One focused unit test per variant (§10), each
asserting the document's `target`, the number and shape of its findings, and
the facts it left empty.

### Work package C — The verdict gate and the executor call site

**Intent.** The table decides *whether*; the result decides *how*. The two
cannot silently disagree.

**Required changes.** `report_for_operation(operation_id, result)` per §6.3,
and the executor's call site moving to it. The `operation_id` is already in
scope at `crates/eggsec/src/runtime_bridge/executor.rs:222`.

**Acceptance evidence.** A test that a `NoCleanMapping` operation's result
yields `None`, a test that a `Canonical` one yields a document, and guard
180's fourth property — the source-level agreement check between the table's
variant lists and the bridge's arms.

### Work package D — Publishing the coverage

**Intent.** The operator learns the coverage from the *running daemon*, not
from a constant compiled into the GUI.

**Required changes.** The runtime DTO, the defaulted trait method, the
engine's implementation, the protocol's defaulted field, the daemon's
publication (§6.4). **No new crate edge**: the DTO is in `eggsec-runtime`,
which all three crates already depend on.

**Acceptance evidence.** A protocol test that a capabilities message without
`report_forms` decodes with `None` (mirroring
`crates/eggsec-daemon-protocol/src/protocol.rs:1522-1526`); a daemon test
that an engine-backed daemon publishes twenty-eight rows and a noop-backed one
publishes `None`; a runtime test that the default trait method publishes
`None`.

### Work package E — The GUI's coverage row and export message

**Intent.** The operator sees which of the offered task kinds will produce a
report, and why the ones that will not.

**Required changes.** The GUI's coverage row and its three-state verdict
(§6.6), the coverage command's new input, the export message's reason, and
the frontend's new cell.

**Acceptance evidence.** A GUI test that a coverage row carries the published
verdict and its reason; that an **unpublished** coverage yields `Unknown` and
never `Unavailable`; and that the export message names the reason only when
the GUI holds one.

### Work package F — The guard and its negative tests

**Intent.** The coverage cannot rot: a new task kind without a row, a row
without a bridge arm, or a bridge arm without a row all fail the build.

**Required changes.** Guard 180 (§6.7), in the house style's `SECTION_FAIL`
idiom, with its negative probes — the house style requires every guard to be
*negative-tested*: the implementer mutates the source to break each property
in turn and records that the guard fails each time.

**Acceptance evidence.** The guard's four negative probes, recorded in the
closure.

### Work package G — The documentation and the closure pack

**Intent.** The coverage is documented where a reader will look, and the
closure record carries the evidence.

**Required changes.** §12's file list. **Acceptance evidence.** §15's list,
filled in.

## 8. Failure, cancellation, restart, and contention semantics

- **A task with no report.** Unchanged from today: the bridge returns `None`,
  the executor inserts nothing, the daemon answers `TaskNotFound`, the client
  degrades to `Ok(None)`, and the GUI says "there is nothing to export" (§3.6).
  **No file is written.**
- **A task whose result cannot be serialized.** Unchanged: the executor logs
  `tracing::warn!` and inserts nothing
  (`crates/eggsec/src/runtime_bridge/executor.rs:373-379`). The task itself
  succeeds; the report does not exist. The GUI's export then reports "no
  report", which is the honest outcome.
- **A bridge/table disagreement** (a `Canonical` row whose variant has no arm).
  This is an internal error, not a task failure: the bridge logs at `error`
  level and returns `None`. The task completes normally and has no report.
  Guard 180 makes the disagreement unreachable in the source; the log is the
  diagnostic if the guard is ever bypassed.
- **Cancellation.** Unchanged. The report is derived *after*
  `race_with_cancel` resolves
  (`crates/eggsec/src/runtime_bridge/executor.rs:353-360`), so a cancelled
  task produces no result and therefore no report. No new task, no new
  timeout, no detached work.
- **Restart.** The coverage table is a compile-time constant; it survives a
  restart unchanged. The **store does not**: a restarted daemon's report store
  is empty, so every task's report is gone and every export reports "no
  report" — the honest answer, and unchanged by this milestone. The *published
  coverage* is re-published by the new process, so the GUI's coverage table is
  correct immediately after a reconnect.
- **Contention.** The store's lock is unchanged: a `std::sync::Mutex` whose
  critical section is a map operation, never held across an await
  (`crates/eggsec-runtime/src/report.rs:38-40`). The coverage table is read
  once per `Capabilities` request and is immutable, so it introduces no
  contention.
- **A daemon that publishes no coverage** (`None`). The GUI says *unknown*,
  never "unavailable" (§6.5). This is the `None`-means-unknown discipline the
  enabled-feature set already established.
- **A daemon too old to publish the coverage.** The field is `#[serde(default)]`,
  so it decodes as `None`; the GUI says *unknown* (§6.4).

## 9. Compatibility and migration

- **The wire.** Additive and defaulted: `DaemonCapabilities.report_forms:
  Option<Vec<ReportFormEntry>>` with `#[serde(default)]`. An old client
  ignores an unknown field; an old daemon's message decodes with `None` in a
  new client. The protocol version stays **2**, the same treatment
  `enabled_features` received.
- **The GUI against an old daemon.** The coverage column reads *unknown*, and
  the export message keeps its current wording when no reason is published.
  **The GUI must not show a report-form verdict it was not told.**
- **The daemon against an old engine.** Not a combination the daemon can
  produce: the daemon links the engine or it does not, and the engine's
  executor publishes the coverage. A daemon built with the `service` feature
  publishes twenty-eight rows; one without it publishes `None`.
- **The `OperationDescription` is untouched** — fifteen fields, the pin test
  unchanged (`crates/eggsec-daemon-protocol/src/operations.rs:244-274`). No
  catalog consumer sees a new field.
- **The stored documents are unchanged in shape.** `ScanReportData` keeps its
  nine fields. The eight new variants produce documents of the same shape as
  the existing ones, so every renderer and every consumer (CLI's `report
  convert`, the GUI's export, the TUI) reads them unchanged.
- **No stored data migrates.** The coverage table is not persisted; the report
  store holds documents only, and it is bounded and in-memory.
- **Removal criteria.** None: nothing is deprecated or removed. If the
  canonical task-kind table grows a variant, the coverage table must grow a
  row in the same change — that is guard 180's first property.

## 10. Required tests

### Focused unit tests

**In the engine** (`crates/eggsec/src/runtime_bridge/report.rs` and the new
`report_coverage.rs`) — hermetic and synthetic: no network, no database, no
clock.

| # | Test | Asserts |
|---:|---|---|
| 1 | `the_coverage_table_covers_every_canonical_operation` | the table's operation ids are exactly the distinct ids of `TaskKind::all_variants()` — derived, not transcribed |
| 2 | `every_excluded_operation_names_the_missing_fact` | each `NoCleanMapping` row's reason is non-empty and names a field or an absence of the result type |
| 3 | `every_result_variant_is_accounted_for_exactly_once` | the union of the rows' `variants` lists is exactly the 35 `TaskResult` variants — `Error` included — with no variant under two operations |
| 4 | `an_endpoint_scan_reports_its_interesting_endpoints` | the target is the base URL; the findings are the interesting endpoints only, at severity `info`; the duration is the result's |
| 5 | `a_graphql_report_names_its_vulnerabilities` | a depth-limit bypass, an alias overload and two injection strings produce four findings at the per-class severities |
| 6 | `an_oauth_report_names_its_vulnerabilities` | the four vulnerability lists produce findings at the per-class severities; the target is the result's |
| 7 | `an_auth_report_carries_the_findings_own_severity` | the findings carry the `AuthFinding`'s own severity and title; the duration is 0 |
| 8 | `an_nse_report_carries_the_evidence_items_own_severity` | the evidence items map through the engine's existing evidence-kind severity mapping (`crates/eggsec/src/nse_bridge.rs:37-47`) |
| 9 | `a_hunt_report_carries_every_finding_collection` | each of the five hunt collections contributes its findings |
| 10 | `a_browser_report_carries_the_xss_and_client_findings` | the DOM XSS and client findings appear; the SPA routes do not |
| 11 | `a_compliance_report_carries_the_failed_requirements_only` | a passing requirement is not a finding |
| 12 | `an_excluded_operation_produces_no_document` | for each excluded operation — and for each of its variants — `report_for_operation` returns `None` |
| 12b | `the_variants_of_one_operation_share_a_verdict` | `Waf`'s two variants, `Storage`'s three and `Integrations`' three each carry one verdict, so the GUI can state a verdict per task kind |
| 13 | `a_canonical_operation_with_no_bridge_arm_is_loud` | a `Canonical` verdict whose result has no arm yields `None` **and** logs at `error` level (asserted through the module's tracing subscriber if one exists; otherwise the `None` is asserted and the closure records the log assertion as a manual probe) |
| 14 | `the_new_documents_carry_no_timestamp` | the eight new variants' documents have an empty `timestamp`, so the bridge stays clock-free (§3.6.4) |

**In the runtime** (`crates/eggsec-runtime/src/report.rs` or a sibling):
`the_default_executor_publishes_no_coverage` — the defaulted `report_forms()`
returns `None`; and `the_coverage_dto_round_trips` — the DTO survives a serde
round trip, including an entry with an empty reason.

**In the protocol** (`crates/eggsec-daemon-protocol/src/protocol.rs`):
`daemon_capabilities_decode_without_the_report_forms_field` — the additive twin
of the existing `daemon_capabilities_decode_without_the_enabled_features_field`
(`:1522-1526`); and `the_published_operation_description_is_unchanged` — the
existing fifteen-field pin (`:244-274`) still passes, unchanged.

**In the GUI** (`crates/eggsec-gui/src/coverage.rs` and `commands.rs`):
`a_coverage_row_carries_the_published_report_form` — the row's verdict and
reason come from the published coverage, joined by operation id;
`an_unpublished_coverage_is_unknown_not_unavailable` — the GUI never says
"unavailable" for a fact it was not told; `the_export_message_names_the_reason_when_it_has_one`
— the message carries the published reason, and keeps its current wording when
it has none; and `a_coverage_row_does_not_name_an_operation_id` — the GUI's
coverage module names no operation id, complementing guard 161.

### Integration tests

| # | Test | Asserts |
|---:|---|---|
| 23 | **A daemon hosting the engine executor publishes twenty-eight coverage rows** (`crates/eggsec-daemon/tests/`) | each row names a canonical operation id — derived from the catalog, not from a list — and the excluded rows carry their reasons |
| 24 | **A daemon hosting a noop executor publishes `None`** | the unknown state, not an empty list |
| 25 | **An end-to-end report read over the real Unix socket** (`eggsec-frontend-client`'s daemon-client path) | a task with a report is served with `TaskReport`; a task without one is served with `TaskNotFound` and the client degrades to `Ok(None)`. The second half is the regression evidence for §3.6's exit condition |
| 26 | **The export path writes no file for a task with no report** | the GUI's export command returns the "nothing to export" error, and the destination path does not exist afterwards |

### Restart and recovery tests

| # | Test | Asserts |
|---:|---|---|
| 27 | **The published coverage survives a daemon restart** | a daemon that publishes twenty-eight rows, restarts, and publishes the same twenty-eight rows: the coverage is a build fact, not a session fact |
| 28 | **A restarted daemon's report store is empty** | an export for a task that completed before the restart reports "no report available", and no file is written — the existing behaviour, pinned so a future change cannot quietly make it look like a lost report |

### Contention and cancellation tests

| # | Test | Asserts |
|---:|---|---|
| 29 | **Cancelling a task produces no report** | a cancelled task's result never reaches the bridge, so the store holds no document and the export reports "no report available" |
| 30 | **The coverage is immutable under concurrent readers** | many concurrent `Capabilities` requests observe the same twenty-eight rows; the `&'static` rows need no locking |

### Security and negative tests

| # | Test | Asserts |
|---:|---|---|
| 31 | **The published coverage leaks nothing** | the serialized payload carries only operation ids, form discriminators and reason strings — no target, no banner, no finding text, no credential |
| 32 | **The coverage grants no authorization** | a task of an excluded operation is refused or accepted exactly as before the coverage was published; the coverage changes no decision |
| 33 | **Guard 180's four negative probes** | mutating the source to (a) remove a coverage row, (b) add a row for an unknown operation, (c) drop a variant from a row, (d) mark a `No clean mapping` row `Canonical` without an arm fails the guard each time — the house style requires this (see checks 161 and 162's probes) |
| 34 | **The GUI still renders the report as data** | guard 162's property holds with the new coverage column: the frontend gains no markup sink, and the engine's HTML is still never injected |

### Migration and compatibility tests

| # | Test | Asserts |
|---:|---|---|
| 35 | **An old daemon's capabilities decode in the new GUI** | a message with no `report_forms` field yields `Unknown` verdicts and an unchanged export message |
| 36 | **A new daemon's capabilities decode in an old client** | the field is ignored and the client's behavior is unchanged |
| 37 | **The stored document shape is unchanged** | a document from the eight new variants deserializes into the existing `ScanReportData` with all nine fields, and the six renderers accept it (`crates/eggsec-output/src/convert.rs:19,53,82,148,190,209`) |

## 11. Required verification commands

```bash
# Format and lint
cargo fmt --all --check
cargo clippy -p eggsec --features cli -- -D warnings
cargo clippy -p eggsec-runtime -p eggsec-daemon-protocol -p eggsec-daemon -- -D warnings
cargo clippy -p eggsec-gui --features shell -- -D warnings

# Build matrix
cargo check --workspace --no-default-features
cargo check -p eggsec --all-targets --no-default-features
cargo check -p eggsec --features cli
cargo check -p eggsec-daemon --features service

# Contract
make check                  # the mandatory Rust contract (fmt, checks, clippy, tests, guards)
make check-deps             # dependency policy only
make check-daemon-profile   # the daemon's `service` profile builds and its conformance tests pass
bash scripts/check-architecture-guards.sh

# Tests
cargo test -p eggsec --lib --features cli
cargo test -p eggsec-runtime --lib
cargo test -p eggsec-daemon-protocol --lib
cargo test -p eggsec-daemon --lib
cargo test -p eggsec-gui --lib --features shell
cargo test -p eggsec-frontend-client --lib
cargo test -p eggsec-output --tests
cargo test -p eggsec-report-model --tests

# Documentation references
python3 scripts/check_doc_references.py
```

**`make check-deps` is known to fail at the baseline** on the inherited
advisory backlog. That backlog is milestone 015's and is **not** this
milestone's to fix. The implementer MUST record the baseline state and the
post-change state of every `cargo deny check` sub-check — `advisories`, `bans`,
`licenses`, `sources` — so that a **new** failure is distinguishable from the
inherited one. `bans`, `licenses` and `sources` must pass: this milestone adds
no dependency, and the DTO it adds is `serde`-only inside a crate that already
depends on `serde`, so **no `deny.toml` exception is required and none may be
added**.

`make check-features-individual` is a deep check, not a per-milestone gate.
It is **required here** only if the implementer changes a feature gate: the
eight new arms carry `#[cfg(feature = ..)]` attributes, and the implementer
MUST record whether the feature surface changed. If it did not, the deep check
is optional and the closure record says so.

`make check-python` is **not** required: this milestone changes no Python
binding, stub or script (§5).

## 12. Documentation updates

| File | What changes |
|---|---|
| `docs/REPORT_EVIDENCE_MODEL.md` | The "The report read path" section (`:145-168`) states the nine-variant coverage; it must state the new coverage, the exclusion set with its reasons, and the three timestamp semantics (§3.6.4). |
| `architecture/daemon.md` | The report path (`:139-157`) and the capabilities publication (`:1434`, `:1462` per its own line numbers) gain the published coverage and the "`None` means unknown" rule. |
| `architecture/desktop_gui.md` | The coverage model and the export path (§"Report Export and Display") gain the report-form column and the reason-bearing export message. |
| `architecture/runtime.md` | The runtime's report module gains the coverage DTO, and the module's "no domain types" claim is corrected to be true (§6.4). |
| `architecture/overview.md` | Only if it states a coverage count. It is the owner of the headline counts (`AGENTS.md`), so if it counts report variants, it must count them the same way this plan counts them. |
| `docs/DAEMON.md` | The capabilities message's published fields gain `report_forms`. |
| `.opencode/skills/eggsec-gui/SKILL.md` | The skill's claims about the coverage table and the export path. Skills drift when a counted set changes; the coverage count is such a set. |
| `plans/subsystems/desktop-gui-frontend-roadmap.md` | **Not edited by the implementer** — the roadmap and its gap list are the plan owner's. The closure record records that gap 11 is closed and names what closed it. |
| `plans/registry.md` | The milestone's registration line, per the README's handoff rules. |

## 13. Acceptance criteria

1. **Every canonical operation id has exactly one coverage row**, and the
   table's operation ids are exactly the distinct ids of the canonical
   task-kind table — derived, not transcribed (§6.1, test 1, guard 180).
2. **Every variant with a clean mapping produces a document**: seventeen of
   the thirty-five `TaskResult` variants, covering eighteen task kinds
   (§3.5's tally).
3. **Every variant without one is named, with the missing fact stated**:
   eighteen variants across twelve task kinds, each with a non-empty reason
   naming a field or an absence (§6.1).
4. **The export path reports "no report available" and writes no artifact**
   for a task with no report — the behaviour is already correct at the
   baseline, so the criterion is met by the regression evidence of §10's
   integration tests 25 and 26, not by a new code path.
5. **The operator can learn the reason**: the GUI's coverage table states the
   verdict per published task kind, and the export failure names the reason
   when the GUI holds one.
6. **The table has exactly one home.** The bridge consults it, the engine
   publishes it, the daemon republishes it verbatim, and neither the daemon,
   the GUI nor the frontend restates it. Guard 161 still passes.
7. **Guard 180 passes, and its four negative probes fail as required**
   (§10's test 33).
8. **The canonical report model is unchanged**: `ScanReportData` keeps its
   nine fields, and the eight new documents use them as they are.
9. **The bridge remains clock-free**: the eight new documents carry an empty
   `timestamp`, and no new arm reads a clock.
10. **No authorization decision changed**, and the published coverage carries
    no target, no banner and no credential (§10's tests 31 and 32).
11. **`make check` is green except for the inherited `check-deps` advisory
    backlog**, which is recorded, not fixed.
12. **No new dependency, and no `deny.toml` exception.**
13. **The documentation in §12 states the new coverage**, and the closure
    record carries §15's evidence.

## 14. Stop conditions

The agent must stop and report rather than improvise when:

- **The canonical task-kind table grows a variant** that the coverage table
  does not cover. Guard 180 fails; the agent adds the row and the arm (or the
  named exclusion) **in the same change** — that is the guard's purpose, not a
  stop condition. The agent stops only if the new variant's result type cannot
  be classified under §3.4's criterion without a judgement the plan does not
  cover.
- **A result type's shape contradicts §3.5.** If a result carries a fact the
  table says it does not (or lacks one it says it has), the agent MUST
  re-measure and report the divergence rather than bend the mapping: the rows
  are measurements, not intentions.
- **The bridge cannot be made total without changing a result type.** Adding a
  `target` or a `timestamp` to an engine result type is a contract change
  (it touches the CLI, the TUI and the Python bindings). The agent MUST stop
  and report rather than widen the result types inside this milestone.
- **The GUI would have to name an operation id or a reason string** to
  implement §6.5, or **the daemon would have to restate the coverage** to
  publish it. Either means the design is wrong; the agent reports it rather
  than weakening guard 161 or adding a second copy.
- **`make check-deps` fails on something other than the inherited advisory
  backlog**, or a `deny.toml` exception would be needed. The agent reports;
  it does not add an exception.
- **The protocol version would have to change**, or the `OperationDescription`
  would have to gain a field. The agent reports; both are contract changes.
- **A guard's house style would have to be broken** to express a check — for
  example, a check that can only pass on a specific feature set. The agent
  reports rather than adding a feature-dependent guard.

## 15. Closure evidence required

The closure record
(`plans/closure/desktop-gui-frontend/013-report-bridge-full-coverage-closure.md`)
MUST contain:

1. **The coverage table as built**, with its 28 rows, and the count of
   Canonical and excluded rows and variants — **re-measured** from the landed
   source, with the method stated.
2. **The bridge's arms as built**, and the mapping from each new variant to its
   document's fields.
3. **The exclusion list with its reasons**, each traced to the result type's
   fields.
4. **The verdict of each of §3.6's four stages**, quoted with `file:line` at
   the landed baseline, and the evidence that no artifact is written for a
   task with no report.
5. **The guard's four negative probes**, each with the mutation and the
   guard's failure output.
6. **The test results**, per crate, with counts.
7. **The `cargo deny check` sub-check states**, baseline and post-change, with
   the inherited advisory failure named as inherited.
8. **The timestamp position restated**: which variants carry which of the four
   timestamp semantics, and the statement that closure finding 8 remains open
   and now covers twelve variants.
9. **The `None`-means-unknown evidence** for the published coverage, on both
   sides (an executor that publishes it, and one that does not).
10. **The deviations from this plan**, if any, with the reason — the house
    style records them rather than silently absorbing them.

## 16. Handoff notes

- **This plan is corrective and does not rewrite milestone 006.** The 006 plan
  and its closure record stay as they are; this milestone's closure record is
  where gap 11 and closure finding 1 are closed. The 006 plan's
  "Amendment record" (§0) is **not** edited.
- **The plan is registered in `plans/registry.md`** by the plan owner, per the
  README's handoff rules; the closure record's "Roadmap disposition" is where
  gap 11's closure is stated.
- **Two measurements in older documents are wrong and this plan depends on the
  corrected numbers**: the canonical 30 task kinds map to **28** operation ids
  (not 26), and the deauth has its own operation id `wireless-deauth` (not a
  shared `wireless`). See §0. The implementer must not copy "nine variants" or
  "26 operation ids" from the older documents.
- **The eight new arms are feature-gated** for `nse`, `advanced-hunting`,
  `headless-browser` and `compliance`, exactly as the existing four are. A
  build without those features compiles the table with the same 28 rows (the
  table is a specification) but cannot reach the gated variants.
- **The eight new variants' documents carry an empty `timestamp`** — deliberate,
  matching the four existing variants, and widening closure finding 8 from four
  variants to twelve. Do not "fix" it by reading a clock: that breaks the
  bridge's purity contract (`report.rs:9-19`).
- **`make check` stops at `check-deps`** on the inherited advisory backlog. The
  implementer records it; milestone 015 owns it.
- **No Python work**: `make check-python` is not required, and no binding,
  stub or script changes.
- **No special prerequisites**: the engine, runtime, daemon and GUI crates all
  build hermetically. The GUI's shell profile needs the platform webview SDK
  (`make check-gui-prereqs` reports it; `make check-gui` is the gate).
- **The report store's bound (64) is not this milestone's to change.**
- **The implementer may adjust file-level mechanics** — the module split, the
  DTO's field names, the arm function names — as long as §6.1's table, §6.2's
  eight mappings, §6.3's gate and §6.4's publication shape hold, and no
  invariant in §4 is weakened.
