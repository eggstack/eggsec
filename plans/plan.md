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

All items in this wave are independent and can be worked on simultaneously. Each bug is confirmed against the codebase as of 2026-05-31.

### 4.1 Workflow SLA Calculation Bug
- **File:** `crates/slapper/src/workflow/mod.rs:35-37`
- **Bug:** `calculate_metrics()` sets `self.sla_violations = self.open_findings` — treats ALL open findings as SLA violations instead of using the actual SLA logic.
- **Current code:**
  ```rust
  pub fn calculate_metrics(&mut self) {
      self.sla_violations = self.open_findings;
  }
  ```
- **Fix:** Replace with a call to `calculate_sla()` from `workflow/sla.rs:57-75` which correctly checks severity-based `SlaPolicy` targets against time elapsed. The `calculate_sla()` function takes `&[Finding]` and `&[SlaPolicy]` and returns the count of actual violations.
- **Note:** The existing test at line 50 asserts the wrong behavior (`assert_eq!(report.sla_violations, 5)`) — update the test to match corrected logic.
- **Reference:** `workflow/sla.rs` has `SlaPolicy` (line 5), `SlaStatus` (line 48), and `calculate_sla()` (line 57).

### 4.2 Notify Discord Dispatch Bug
- **File:** `crates/slapper/src/notify/mod.rs:199-258`
- **Bug:** `notify_findings()` dispatches to webhooks (line 209), Slack (lines 226-240), and Teams (lines 243-257) but **skips Discord**. Other notify methods (`notify_scan_complete()` at lines 164-178, `notify_error()` at lines 283-298) both include Discord dispatch via `notifier.notify_discord(discord_url, &payload)`.
- **Fix:** Add Discord dispatch block to `notify_findings()` matching the pattern in `notify_scan_complete()`:
  ```rust
  if let Some(ref discord_url) = self.discord_webhook {
      if let Err(e) = notifier.notify_discord(discord_url, &payload).await {
          tracing::warn!("Failed to send Discord notification: {}", e);
      }
  }
  ```
  Place this after the Teams block (after line 257) and before the final Ok(()).

### 4.3 Storage Module Stubs
- **File:** `crates/slapper/src/storage/postgres.rs:19-56`
- **Bug:** All 8 `Database` methods are stubs returning hardcoded empty values (`Ok(())`, `Ok(None)`, `Ok(vec![])`). No actual PostgreSQL connection pool or SQLx usage. The struct has `#[allow(dead_code)]` on line 6.
- **Fix options:**
  1. **If implementing:** Add SQLx `PgPool` to the `Database` struct, implement real CRUD queries using `storage/queries.rs` and `storage/models.rs` types (`StoredScan`, `StoredFinding`).
  2. **If placeholder:** Add `/// WARNING: Stub implementation — not connected to a real database` doc comment to each method and the struct itself. Add a `todo!()` or `unimplemented!()` note in the module doc.
- **Reference:** `storage/queries.rs` exists with SQL queries. `storage/models.rs` has `StoredScan` (line 6) and `StoredFinding` (line 25).

### 4.4 Feature Matrix Math Error
- **File:** `architecture/feature_matrix.md:7-11`
- **Bug:** Summary table says 18 features-with-deps + 12 marker-only = 30, but total is listed as 28. The math doesn't add up.
- **Actual counts from `Cargo.toml:213-296`:**
  - Features **with dependencies**: 16 (`rest-api`, `ws-api`, `grpc-api`, `stress-testing`, `packet-inspection`, `nse`, `nse-ssh2`, `nse-sandbox`, `ai-integration`, `websocket`, `headless-browser`, `database`, `container`, `sbom`, `pdf`, `full`)
  - Marker-only features: 12 (`tool-api`, `insecure-tls`, `advanced-hunting`, `compliance`, `external-integrations`, `finding-workflow`, `vuln-management`, `cloud`, `git-secrets`, `wireless`, `api-schema`, `default`)
  - **Correct math: 16 + 12 = 28**
- **Fix:** Change "Features with deps" from 18 to **16** in the summary table. Leave marker-only at 12 (correct).

### 4.5 Findings Architecture Doc Wrong Type
- **File:** `architecture/findings.md:21`
- **Bug:** References non-existent `FindingLifecycle` type at `findings/lifecycle.rs`.
- **Actual types in `findings/lifecycle.rs`:**
  - `FindingStatus` (enum, line 4) — 6 states: `New`, `Confirmed`, `AcceptedRisk`, `FalsePositive`, `Remediated`, `Reopened`
  - `StoredFinding` (struct, line 29) — persisted finding with lifecycle metadata
  - `StatusChange` (struct, line 38) — audit trail entry
  - `ScanRun` (struct, line 72) — scan execution record
- **Fix:** Replace `FindingLifecycle` with `FindingStatus` in the Key Types table. Update the description to "Finding status transitions (6 states)".

### 4.6 Lib.rs Stale Docstrings
- **File:** `crates/slapper/src/lib.rs:16-17`
- **Bug:** Docstring says "22 payload types" and "26 products".
- **Actual counts:** 30 payload types (`fuzzer/payloads/mod.rs:39-70`), 34 WAF products (`constants.rs:77` `SUPPORTED_WAF_COUNT`).
- **Fix:** Update lines 16-17:
  ```rust
  //! - **`fuzzer`** - Security fuzzing engine with 30 payload types
  //! - **`waf`** - WAF detection (34 products) and bypass techniques
  ```

### 4.7 ~~Output AttackGraph Feature Gate~~ [REMOVED — Not a Bug]
- **Note:** The `attack_graph` module IS properly feature-gated at `output/mod.rs:51` with `#[cfg(feature = "advanced-hunting")]`. The re-exports at lines 79-82 are also gated. No action needed.

---

## Wave 5: Type Name & Count Corrections (All Parallelizable)

All items are independent doc fixes. Can run in parallel with Wave 4.

### 5.1 Workflow SlaTracking → SlaPolicy + SlaStatus
- **File:** `architecture/workflow.md:15`
- **Actual:** `SlaPolicy` at `workflow/sla.rs:5`, `SlaStatus` at `workflow/sla.rs:48`
- **Fix:** Replace all `SlaTracking` references with `SlaPolicy` + `SlaStatus` in the Key Types table and throughout the document.

### 5.2 Storage ScanModel/FindingModel Names
- **File:** `architecture/storage.md`
- **Actual:** `StoredScan` at `storage/models.rs:6`, `StoredFinding` at `storage/models.rs:25`
- **Fix:** Replace `ScanModel`/`FindingModel` with `StoredScan`/`StoredFinding` throughout.

### 5.3 Error Variant Count
- **File:** `architecture/error.md`
- **Actual:** 22 `SlapperError` variants at `error/mod.rs:43-116`
- **Fix:** Update "19+" to "22". List all 22 variants: Config, InvalidTarget, Network, RequestFailed, Timeout, RateLimited, ScanFailed, Payload, Output, ScopeViolation, Io, HttpStatus, Http, Parse, Validation, AddressParse, Runtime, Cancelled, Proxy, Recon, LoadTest, Fingerprint.

### 5.4 Recon FxHashMap Count
- **File:** `architecture/recon.md`
- **Actual:** ~55 lines containing FxHashMap/FxHashSet across 14 files (not 66+). IAM patterns: 12 entries in `KNOWN_ESCALATION_PATTERNS` at `recon/cloud/iam.rs:29-109` (not 13).
- **Fix:** Update FxHashMap/FxHashSet count from "66+" to "55". Update IAM patterns from 13 to 12.

### 5.5 TUI File Counts
- **File:** `architecture/tui.md`
- **Actual:** `app/` has 18 `.rs` files, `components/` has 12 `.rs` files
- **app/ files:** `mod.rs`, `runner.rs`, `key_handler.rs`, `state_update.rs`, `task_runtime.rs`, `export.rs`, `task_management.rs`, `navigation.rs`, `command.rs`, `help_config.rs`, `bookmarks.rs`, `dispatch.rs`, `tab_error.rs`, `confirmation.rs`, `error.rs`, `notifications.rs`, `input.rs`, `options.rs`
- **components/ files:** `mod.rs`, `progress.rs`, `selector.rs`, `popup.rs`, `palette.rs`, `scrollable.rs`, `input.rs`, `notifications.rs`, `search_popup.rs`, `empty_state.rs`, `help_bar.rs`, `http_options.rs`
- **Fix:** Update file counts and list all files.

### 5.6 ThemeColors Field Count
- **File:** `architecture/tui.md`
- **Actual:** 28 fields at `tui/theme.rs:24-51` (not "30+" or 29)
- **Fix:** Update count to 28.

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
- **Fix:** Correct the variant name in the WebhookEvent enum documentation.

### 5.9 Container Feature Gate Documentation
- **File:** `architecture/container.md`
- **Actual:** The top-level `container` module IS feature-gated behind `#[cfg(feature = "container")]` at `lib.rs:84-88`. The `recon/containers.rs` module is NOT feature-gated at the module level but has extensive internal feature gating.
- **Fix:** Correct the document to accurately describe the feature gating. Note that the top-level module requires the `container` feature, while recon's container analysis is always compiled but uses internal feature gates.

### 5.10 Findings FindingStore Description
- **File:** `architecture/findings.md`
- **Actual:** `FindingStore` is JSONL-based persistent file storage at `findings/store.rs:7-10`, writing to `findings.jsonl` on disk (line 20-21). Not "in-memory".
- **Fix:** Correct description to "JSONL-based persistent file storage".

### 5.11 Networking BPF Description
- **File:** `architecture/networking.md`
- **Actual:** Filtering at `capture.rs:276-306` is custom TCP/UDP/ICMP/port matching using string comparison, not true BPF (Berkeley Packet Filter).
- **Fix:** Correct "BPF-style filters" to "custom packet matcher" or "custom protocol/port filter".

### 5.12 Diff diff_findings_from_files() Claim
- **File:** `architecture/diff.md`
- **Actual:** `diff_findings_from_files()` does not exist. Only `load_findings_from_file()` at `diff/mod.rs:103` loads a single file. `diff_findings()` at line 39 takes `&[Finding]` slices.
- **Fix:** Remove or correct the claim. Document actual usage: load findings with `load_findings_from_file()`, then diff with `diff_findings()`.

### 5.13 WebSocket Feature Gate Documentation
- **File:** `architecture/websocket.md`
- **Actual:** There are 7 test functions in `fuzzer/payloads/websocket.rs:349-411`, and **none** are feature-gated (only `#[cfg(test)]`). The WebSocket public API in `websocket/connection.rs` may have different gating.
- **Fix:** Correct the test count from 4 to 7 and remove the claim that tests are feature-gated. Verify and document the actual feature gating on the public WebSocket API.

### 5.14 Supply Chain Scanner Description
- **File:** `architecture/supply_chain.md`
- **Actual:** `scanner.rs` discovers manifest files (Cargo.toml, package.json, etc.) and analyzes Dockerfiles and GitHub Actions workflows for misconfigurations. It does NOT scan dependencies for known vulnerabilities. Feature-gated behind `#[cfg(feature = "sbom")]` at `supply_chain/scanner.rs:67`.
- **Fix:** Correct description from "dependency vulnerability scanner" to "manifest discovery and configuration analysis tool". Note the `sbom` feature gate.

### 5.15 Wireless Handshake Claim
- **File:** `architecture/wireless.md`
- **Actual:** Code only does iwlist scan parsing + security type analysis (`wireless/mod.rs:81-209`). No WPA/WPA2 handshake capture code exists. The module docstring at line 4 falsely claims this capability.
- **Fix:** Remove or mark "WPA/WPA2 handshake capture analysis" as aspirational/future. Correct module description to "iwlist scan parsing and wireless security type analysis".

---

## Wave 6: Documentation Gaps (3 Parallel Groups)

Group A (independent, parallelizable):

### 6.1 Error Module From Impls
- **File:** `architecture/error.md`
- **Task:** Document 16 non-feature-gated `From` impls at `error/mod.rs:202-357` (not 14) and 3 feature-gated `From` impls at `error/mod.rs:275-327`.
- **From impls to document:**
  - Non-gated (16): `toml::de::Error` (202), `serde_json::Error` (208), `url::ParseError` (214), `std::net::AddrParseError` (220), `serde_yaml_neo::Error` (226), `toml::ser::Error` (232), `std::string::FromUtf8Error` (238), `tokio::time::error::Elapsed` (244), `crate::config::ScopeError` (253), `hickory_resolver::net::NetError` (259), `anyhow::Error` (265), `std::num::ParseIntError` (329), `tokio::sync::AcquireError` (335), `quick_xml::Error` (341), `maxminddb::MaxMindDbError` (347), `reqwest::header::InvalidHeaderValue` (353)
  - Feature-gated (3): `crate::ai::AiError` (276, `ai-integration`), `crate::packet::CaptureError` (316, `packet-inspection`), `crate::packet::TracerouteError` (323, `packet-inspection`/`stress-testing`)

### 6.2 Recon Module Counts
- **File:** `architecture/recon.md`
- **Task:** Update FxHashMap/FxHashSet count to 55 (not 66+), IAM pattern count to 12 (not 13). Document `ReconStep<T>` enum at `recon/runner.rs:18-35`.

### 6.3 Proxy Module Completeness
- **File:** `architecture/proxy.md`
- **Task:** Document intercept submodule (`cert.rs`, `interceptor.rs`, `rules.rs`), `create_chained_connection()` at `mod.rs:156-218`, background health check at `mod.rs:224-266`.

### 6.4 Output Module report_summary.rs
- **File:** `architecture/output.md`
- **Task:** Add `report_summary.rs` to FxHashMap migration list. Document `RunManifest::from_report()` at `run_manifest.rs:103-179`.

Group B (independent, parallelizable):

### 6.5 TUI Module Completeness
- **File:** `architecture/tui.md`
- **Task:** Document all 18 app/ files, all 12 components/ files, `ui.rs` draw functions, `auth.rs` tab status (exists at `tui/tabs/auth.rs` but not part of `Tab` enum), full TabInput interface (25+ methods), OverlayType/PendingAction/InputMode/AppState enums.

### 6.6 Pipeline Executor Fields
- **File:** `architecture/pipeline.md`
- **Task:** Document `spoof_config` (`SpoofConfig`), `config` (`Option<SlapperConfig>`), `session_path` (`Option<String>`) fields at `executor.rs:38-50`. Document `PipelineReport` struct (starts at `report.rs:24`, not 33) including `checkpoint_error: Option<String>` field at line 33. Note `generate_html()` and `generate_csv()` are free functions taking `&PipelineReport`, not methods on the struct.

### 6.7 Distributed Module Gaps
- **File:** `architecture/distributed.md`
- **Task:** Document IP allowlist (`remote.rs:34,70-83`), connection limits (`remote.rs:17,209-213`), rate limiting (`remote.rs:18-19,121-140`), DNS caching (`remote.rs:514-532`), `ResponseMessage` type (`command.rs:65-118`).

### 6.8 Loadtest Module Gaps
- **File:** `architecture/loadtest.md`
- **Task:** Document `CancellationToken` for graceful shutdown (`runner.rs:284,304-307`), `Report` trait impl (`runner.rs:380-387`), progress bar / indicatif integration (`runner.rs:255-266`).

### 6.9 Findings Module Completeness
- **File:** `architecture/findings.md`
- **Task:** Enumerate all 19 `Finding` struct fields (`findings/mod.rs:252-291`, not 18): `id`, `fingerprint`, `title`, `description`, `severity`, `confidence`, `finding_type`, `cwe`, `owasp`, `cve`, `affected_asset`, `location`, `evidence`, `reproduction`, `remediation`, `discovered_at`, `source`, `tags`, `metadata`. Document `FindingStatus` variants (6 states). Note Confidence divergence between findings module (5 variants: Confirmed/High/Medium/Low/Informational) and output module (4 variants: Confirmed/Likely/Possible/Unlikely).

Group C (independent, parallelizable):

### 6.10 Networking Module Gaps
- **File:** `architecture/networking.md`
- **Task:** Document `CaptureBuilder` pattern (`capture.rs:455-510`), `PcapWriter` (`capture.rs:14-74`), `PacketInfo` struct (`mod.rs:26-34`), clarify types vs impls.

### 6.11 Container Module Missing Types
- **File:** `architecture/container.md`
- **Task:** Document `ImageLayer` (`docker.rs:18`), `DockerVulnerability` (`docker.rs:25`), `DockerMisconfiguration` (`docker.rs:34`), `ClusterInfo` (`kubernetes.rs:16`), `K8sFinding` (`kubernetes.rs:23`), `EscapeRisk` (`escape.rs:12`), `EscapeRiskLevel` (`escape.rs:20`), `CisCheck` (`cis.rs:15`), `CisCheckStatus` (`cis.rs:24`).

### 6.12 Compliance Module report.rs
- **File:** `architecture/compliance.md`
- **Task:** Document `ComplianceSummary` struct (`compliance/report.rs:5`), `RiskLevel` enum (`compliance/report.rs:13`).

### 6.13 Vuln Module Gaps
- **File:** `architecture/vuln.md`
- **Task:** Document `TriageStatus::New` variant (`vuln/triage.rs:14`), `RemediationPriority` enum at `remediation.rs:16`, note `VulnAssessment` placeholder status.

### 6.14 Hunt Module Feature Gate
- **File:** `architecture/hunt.md`
- **Task:** Document `advanced-hunting` feature flag (marker-only, `Cargo.toml:248`), `HuntConfig` defaults (`hunt/mod.rs:111-132`: all check flags `true`, `concurrency: 10`, `timeout_ms: 30000`), sub-module check details.

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
