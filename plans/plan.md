# Slapper Implementation Plan

**Created:** 2026-05-30
**Last Updated:** 2026-05-31
**Status:** Active

---

## Summary

Previous implementation waves (0–3) are complete. A full architecture review of 34 documents against the codebase identified **7 high-priority bugs**, **20 discrepancies**, and **23+ improvement items**. This plan consolidates all remaining work into parallelizable waves.

---

## Previous Work (Completed)

- **Wave 0**: Critical bug fixes (3 items) — all applied
- **Wave 1**: Architecture documentation updates (52 items across 5 sub-waves) — all applied
- **Wave 2**: Agent & MCP profile productionization (12 phases) — all implemented
- **Wave 3**: Output module documentation (5 items) — all documented

---

## Verification Commands

```bash
cargo check --lib -p slapper
cargo check -p slapper-nse
cargo test --lib -p slapper
cargo test --test negative_tests -p slapper
cargo test --test scanner_tests -p slapper
cargo clippy --lib -p slapper
```

---

## Wave 4: Critical Bug Fixes (All Parallelizable)

All items in this wave are independent and can be worked on simultaneously.

### 4.1 Workflow SLA Calculation Bug
- **File:** `crates/slapper/src/workflow/mod.rs:36`
- **Issue:** `calculate_metrics()` sets `self.sla_violations = self.open_findings` — treats ALL open findings as SLA violations instead of using the actual `calculate_sla()` function in `sla.rs:57-75` which considers severity-based target hours and time elapsed.
- **Fix:** Replace `self.sla_violations = self.open_findings` with a call to `calculate_sla()` or equivalent logic that checks actual SLA status per finding.
- **Reference:** `workflow/sla.rs:57-75` has the correct SLA evaluation logic.

### 4.2 Notify Discord Dispatch Bug
- **File:** `crates/slapper/src/notify/mod.rs:199`
- **Issue:** `notify_findings()` dispatches to webhooks, Slack, and Teams but **skips Discord**. Other notify methods (`notify_scan_complete()` at lines 118-196, `notify_error()` at lines 261-315) both include Discord dispatch.
- **Fix:** Add Discord dispatch to `notify_findings()` matching the pattern in the other two methods.
- **Reference:** Compare with `notify_scan_complete()` for the Discord dispatch pattern.

### 4.3 Storage Module Stubs
- **File:** `crates/slapper/src/storage/postgres.rs:19-54`
- **Issue:** All `Database` methods (`insert_scan`, `get_scan`, `list_scans`, `insert_finding`, `get_findings_for_scan`, `update_finding_status`) are stubs returning hardcoded empty values. No real PostgreSQL connection pool or SQLx usage.
- **Fix:** Either implement actual database operations or clearly mark the module as experimental/placeholder. If implementing, add SQLx connection pool, real queries, and error handling.
- **Reference:** `storage/queries.rs` exists but is not integrated. `storage/models.rs` has `StoredScan` and `StoredFinding` types.

### 4.4 Feature Matrix Math Error
- **File:** `architecture/feature_matrix.md:8-11`
- **Issue:** Summary table says 18 features-with-deps + 12 marker-only = 30, but total is 28. Correct counts: **15** features-with-deps + **13** marker-only = 28. Verified against `Cargo.toml:213-296`.
- **Fix:** Update features-with-deps from 18 to 15, marker-only from 12 to 13.

### 4.5 Findings Architecture Doc Wrong Type
- **File:** `architecture/findings.md:21`
- **Issue:** References non-existent `FindingLifecycle` type. Actual types are `FindingStatus` (6 states), `StoredFinding`, `StatusChange`, `ScanRun` in `findings/lifecycle.rs`.
- **Fix:** Replace `FindingLifecycle` references with correct type names. Document `FindingStatus` variants.

### 4.6 Lib.rs Stale Docstrings
- **File:** `crates/slapper/src/lib.rs:16-17`
- **Issue:** Docstring says "22 payload types" → actual **30** (in `fuzzer/payloads/mod.rs:39-70`). Says "26 products" → actual **34** (in `waf/data/patterns.rs`).
- **Fix:** Update both counts to match actual values.

### 4.7 Output AttackGraph Feature Gate
- **File:** `crates/slapper/src/output/mod.rs:51-52`
- **Issue:** `AttackGraphBuilder::to_html()` documented as usable without `advanced-hunting` feature, but the entire `attack_graph` module is feature-gated.
- **Fix:** Add feature gate documentation or correct the availability claim.

---

## Wave 5: Type Name & Count Corrections (All Parallelizable)

All items are independent doc fixes. Can run in parallel with Wave 4.

### 5.1 Workflow SlaTracking → SlaPolicy + SlaStatus
- **File:** `architecture/workflow.md`
- **Actual:** `SlaPolicy` at `workflow/sla.rs:5`, `SlaStatus` at `workflow/sla.rs:48`
- **Fix:** Replace all `SlaTracking` references with `SlaPolicy` + `SlaStatus`.

### 5.2 Storage ScanModel/FindingModel Names
- **File:** `architecture/storage.md`
- **Actual:** `StoredScan` at `storage/models.rs:6`, `StoredFinding` at `storage/models.rs:25`
- **Fix:** Replace `ScanModel`/`FindingModel` with `StoredScan`/`StoredFinding`.

### 5.3 Error Variant Count
- **File:** `architecture/error.md`
- **Actual:** 22 `SlapperError` variants at `error/mod.rs:43-116`
- **Fix:** Update "19+" to "22".

### 5.4 Recon FxHashMap Count
- **File:** `architecture/recon.md`
- **Actual:** 66+ FxHashMap/FxHashSet usages (per `recon/AGENTS.override.md:57`)
- **Fix:** Update "55 total collections" to "66+". Update IAM patterns from 12 to 13.

### 5.5 TUI File Counts
- **File:** `architecture/tui.md`
- **Actual:** `app/` has 18 files (not 7), `components/` has 12 files (not 7)
- **Fix:** Update file counts and add missing file names.

### 5.6 ThemeColors Field Count
- **File:** `architecture/tui.md`
- **Actual:** 29 fields (not "30+")
- **Fix:** Update count to 29.

### 5.7 Overview Type Location Corrections
- **File:** `architecture/overview.md`
- **Corrections needed:**
  - `ScanResults` location: `scanner/mod.rs` → `waf/types.rs:188`
  - `FingerprintResult` → `FingerprintResults` (plural) at `scanner/fingerprint.rs:83`
  - `FuzzResult` location: `fuzzer/mod.rs` → `fuzzer/engine/types.rs:10`
  - `WafProfile` location: `waf/types.rs` → `waf/bypass/profiles.rs:9`
  - `Pipeline` location: `pipeline/mod.rs` → `pipeline/executor.rs:38`
  - Commands enum count: "35+" → 37

### 5.8 Notify WebhookEvent Variant Name
- **File:** `architecture/notify.md`
- **Actual:** Variant is `ScanError` not `Error` at `notify/webhook.rs:46`
- **Fix:** Correct the variant name.

### 5.9 Container Feature Gate Claim
- **File:** `architecture/container.md`
- **Actual:** No `#[cfg(feature = ...)]` on container module — always compiled
- **Fix:** Remove feature-gating claim or add actual feature gates.

### 5.10 Findings FindingStore Description
- **File:** `architecture/findings.md`
- **Actual:** `FindingStore` is JSONL-based persistent storage at `findings/store.rs:20-21`, not "in-memory"
- **Fix:** Correct description to "JSONL-based persistent storage".

### 5.11 Networking BPF Description
- **File:** `architecture/networking.md`
- **Actual:** Filtering at `capture.rs:276-306` is custom TCP/UDP/ICMP/port matching, not true BPF
- **Fix:** Correct "BPF-style filters" to "custom packet matcher".

### 5.12 Diff diff_findings_from_files() Claim
- **File:** `architecture/diff.md`
- **Actual:** `diff_findings_from_files()` does not exist. Only `load_findings_from_file()` at `diff/mod.rs:103` loads a single file.
- **Fix:** Remove or correct the claim. Document actual usage pattern.

### 5.13 WebSocket Feature Gate Documentation
- **File:** `architecture/websocket.md`
- **Actual:** All 4 test methods are feature-gated behind `#[cfg(feature = "websocket")]`
- **Fix:** Add prominent feature gate requirement.

### 5.14 Supply Chain Scanner Description
- **File:** `architecture/supply_chain.md`
- **Actual:** `scanner.rs` is manifest discovery + Dockerfile/GitHub Actions analysis, not "dependency vulnerability scanner". Also feature-gated behind `#[cfg(feature = "sbom")]`.
- **Fix:** Correct description and note feature gate.

### 5.15 Wireless Handshake Claim
- **File:** `architecture/wireless.md`
- **Actual:** Code only does iwlist scan parsing + security type analysis. No WPA/WPA2 handshake capture.
- **Fix:** Remove or mark "WPA/WPA2 handshake capture" as aspirational/future.

---

## Wave 6: Documentation Gaps (Parallelizable Groups)

Group A (independent, parallelizable):

### 6.1 Error Module From Impls
- **File:** `architecture/error.md`
- **Task:** Document 14 undocumented `From` impls at `error/mod.rs:202-357` and 3 feature-gated `From` impls at `error/mod.rs:275-327`.

### 6.2 Recon Module Counts
- **File:** `architecture/recon.md`
- **Task:** Update FxHashMap/FxHashSet count, IAM pattern count, document `ReconStep<T>` enum at `recon/runner.rs:18-35`.

### 6.3 Proxy Module Completeness
- **File:** `architecture/proxy.md`
- **Task:** Document intercept submodule (`cert.rs`, `interceptor.rs`, `rules.rs`), `create_chained_connection()` at `mod.rs:156-218`, background health check at `mod.rs:224-266`.

### 6.4 Output Module report_summary.rs
- **File:** `architecture/output.md`
- **Task:** Add `report_summary.rs` to FxHashMap migration list. Document `RunManifest::from_report()` at `run_manifest.rs:103-179`.

Group B (independent, parallelizable):

### 6.5 TUI Module Completeness
- **File:** `architecture/tui.md`
- **Task:** Document all 18 app/ files, all 12 components/ files, `ui.rs`, `auth.rs` tab status, full TabInput interface (25+ methods), OverlayType/PendingAction/InputMode/AppState.

### 6.6 Pipeline Executor Fields
- **File:** `architecture/pipeline.md`
- **Task:** Document `spoof_config`, `config`, `session_path` fields at `executor.rs:38-50`. Document `PipelineReport` missing `checkpoint_error` field at `report.rs:33`.

### 6.7 Distributed Module Gaps
- **File:** `architecture/distributed.md`
- **Task:** Document IP allowlist (`remote.rs:34,70-83`), connection limits (`remote.rs:17,209-213`), rate limiting (`remote.rs:18-19,121-140`), DNS caching (`remote.rs:514-532`), `ResponseMessage` type (`command.rs:65-118`).

### 6.8 Loadtest Module Gaps
- **File:** `architecture/loadtest.md`
- **Task:** Document `CancellationToken` for graceful shutdown (`runner.rs:284,304-307`), `Report` trait impl (`runner.rs:380-387`), progress bar / indicatif integration (`runner.rs:255-266`).

### 6.9 Findings Module Completeness
- **File:** `architecture/findings.md`
- **Task:** Enumerate all 18 `Finding` struct fields (`findings/mod.rs:252-291`), document `FindingStatus` variants (6 states), note Confidence divergence between modules.

Group C (independent, parallelizable):

### 6.10 Networking Module Gaps
- **File:** `architecture/networking.md`
- **Task:** Document `CaptureBuilder` pattern (`capture.rs:455-510`), `PcapWriter` (`capture.rs:14-74`), `PacketInfo` struct (`mod.rs:26-34`), clarify types vs impls.

### 6.11 Container Module Missing Types
- **File:** `architecture/container.md`
- **Task:** Document `ImageLayer`, `DockerVulnerability`, `DockerMisconfiguration`, `ClusterInfo`, `K8sFinding`, `EscapeRisk`, `EscapeRiskLevel`, `CisCheck`, `CisCheckStatus`.

### 6.12 Compliance Module report.rs
- **File:** `architecture/compliance.md`
- **Task:** Document `ComplianceSummary`, `RiskLevel` types at `compliance/report.rs:1`.

### 6.13 Vuln Module Gaps
- **File:** `architecture/vuln.md`
- **Task:** Document `TriageStatus::New` variant, `RemediationPriority` enum at `remediation.rs:16`, note `VulnAssessment` placeholder status.

### 6.14 Hunt Module Feature Gate
- **File:** `architecture/hunt.md`
- **Task:** Document `advanced-hunting` feature flag, `HuntConfig` defaults, sub-module check details.

---

## Wave 7: Uncovered Module Documentation (Parallelizable)

These 9 modules have no dedicated architecture docs. Create new docs or extend existing ones.

### 7.1 stress/ Module
- **Location:** `crates/slapper/src/stress/`
- **Task:** Create `architecture/stress.md` documenting the stress testing module (raw sockets, IP spoofing, connection flooding).
- **Feature gate:** `stress-testing`

### 7.2 utils/ Module
- **Location:** `crates/slapper/src/utils/`
- **Task:** Create `architecture/utils.md` documenting utility modules: `network.rs`, `formatting.rs`, `circuit_breaker.rs`, etc.

### 7.3 types.rs Module
- **Location:** `crates/slapper/src/types.rs`
- **Task:** Create `architecture/types.md` or extend `overview.md` documenting core types: `Severity`, `OutputFormat`, etc.

### 7.4 constants.rs Module
- **Location:** `crates/slapper/src/constants.rs`
- **Task:** Document constants (e.g., `SUPPORTED_WAF_COUNT`) in overview or dedicated doc.

### 7.5 probe.rs Module
- **Location:** `crates/slapper/src/probe.rs`
- **Task:** Document `ProbeIntent` (10 variants) and `ProbeRisk` (6 variants) in overview or dedicated doc.

### 7.6 auth_context/ Module
- **Location:** `crates/slapper/src/auth_context/`
- **Task:** Document auth context file parsing.

### 7.7 logging/ Module
- **Location:** `crates/slapper/src/logging/`
- **Task:** Document logging configuration and initialization.

### 7.8 macros.rs Module
- **Location:** `crates/slapper/src/macros.rs`
- **Task:** Document macro definitions if any are public.

### 7.9 generated/ Module
- **Location:** `crates/slapper/src/generated/`
- **Task:** Note auto-generated protobuf code, no manual documentation needed.

---

## Non-Goals

- Do NOT add new offensive capability
- Do NOT reintroduce Python/Ruby plugin runtimes
- Do NOT publish crates or flip visibility unless instructed
- Do NOT invent domains/organizations/support contacts
- Do NOT claim production maturity for experimental features
- Do NOT remove NSE support
- Do NOT perform large architectural rewrites in single passes

---

## Key Module Locations

| Module | Key Types | Location |
|--------|-----------|----------|
| AI | `AiClient`, `Provider`, `AiCache`, `AiPlanner` | `crates/slapper/src/ai/` |
| MCP | `McpProfile`, `McpProfilePolicy`, `TargetPolicy` | `crates/slapper/src/tool/protocol/mcp/` |
| WAF | `SmartWafBypass` | `crates/slapper/src/waf/` |
| Fuzzer | `FuzzEngine`, `FuzzResult`, `PayloadType` (30 variants) | `crates/slapper/src/fuzzer/` |
| Scanner | Port scanning, endpoint discovery (261 built-in paths) | `crates/slapper/src/scanner/` |
| TUI | 28 tabs, event loop | `crates/slapper/src/tui/` |
| Config | `SlapperConfig` | `crates/slapper/src/config/` |
| Output | Report formatting, exports | `crates/slapper/src/output/` |
| Recon | `runner.rs`, `FullReconResult` | `crates/slapper/src/recon/` |
| Pipeline | `Stage` (7 variants), `PipelineContext` | `crates/slapper/src/pipeline/` |
| Agent | `AgentRuntimeStatus`, routes | `crates/slapper/src/agent/` |

---

## Defense-Lab Profiles

All 5 profiles implemented in `ScanProfile` enum (`cli/mod.rs:262-266`) and `stage.rs:92-107`:

| Profile | Purpose |
|---------|---------|
| `DefenseLab` | Baseline diff and defense validation |
| `SynvoidLocal` | Localhost SYN scan testing |
| `WafRegression` | WAF detection regression testing |
| `ProtocolEdge` | Protocol edge case testing |
| `NseSafe` | Safe NSE script execution |

## Probe Classification

`crates/slapper/src/probe.rs` defines:

- **`ProbeIntent`**: Discovery, Fingerprint, ServiceValidation, WafEvaluation, EvasionResistance, LoadBearing, Stress, MalformedProtocol, Regression, Compatibility
- **`ProbeRisk`**: Passive, SafeActive, Intrusive, Credentialed, Stress, ExploitAdjacent
