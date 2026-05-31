# Consolidated Architecture Review Summary

**Date:** 2026-05-31
**Documents Reviewed:** 34
**Review Files:** 34

## High-Priority Bugs

- [Workflow] Incorrect SLA violation calculation: `calculate_metrics()` sets `sla_violations = open_findings`, ignoring actual SLA logic in `sla.rs` (file: `crates/slapper/src/workflow/mod.rs:36`)
- [Notify] Incomplete Discord dispatch in `notify_findings()`: sends to webhooks, Slack, and Teams but skips Discord, unlike `notify_scan_complete()` and `notify_error()` which include Discord (file: `crates/slapper/src/notify/mod.rs:199`)
- [Storage] Stub database implementation: all `Database` methods (`insert_scan`, `get_scan`, `list_scans`, `insert_finding`, `get_findings_for_scan`, `update_finding_status`) return hardcoded empty values; no actual PostgreSQL connection or SQLx usage (file: `crates/slapper/src/storage/postgres.rs:19-54`)
- [Auth] Dead code: `multi_protocol.rs` and submodules `ssh.rs`, `ftp.rs`, `smtp.rs` exist on disk but are never declared as `pub mod` in `auth/mod.rs`, making them completely unreachable (file: `crates/slapper/src/auth/mod.rs:6-12`)
- [Feature Matrix] Mathematical inconsistency in summary: "Features with deps" listed as 18 + "Marker-only features" 12 = 30, but "Total features" is 28. Actual features-with-deps count is 16 (file: `architecture/feature_matrix.md:8-11`)
- [Findings] `FindingLifecycle` type does not exist: document references a non-existent type at `findings/lifecycle.rs`. Actual types are `FindingStatus`, `StoredFinding`, `StatusChange`, `ScanRun` (file: `architecture/findings.md:21`)
- [Overview] `lib.rs` docstrings stale: claims "22 payload types" (actual 30) and "26 WAF products" (actual 34) (file: `crates/slapper/src/lib.rs:16-17`)
- [Output] `AttackGraphBuilder::to_html()` documented as usable without `advanced-hunting` feature, but the entire `attack_graph` module is behind `#[cfg(feature = "advanced-hunting")]` at `output/mod.rs:51` (file: `output/mod.rs:51-52`)

## High-Priority Discrepancies

- [Workflow] Documented `SlaTracking` type does not exist; actual types are `SlaPolicy` (`sla.rs:5`) and `SlaStatus` (`sla.rs:48`) (file: `crates/slapper/src/workflow/sla.rs`)
- [Storage] Documented model type names `ScanModel` and `FindingModel` do not exist; actual names are `StoredScan` (`models.rs:6`) and `StoredFinding` (`models.rs:25`) (file: `crates/slapper/src/storage/models.rs`)
- [Storage] Documented as "PostgreSQL connection pool and operations"; actual `Database` struct only holds `StorageConfig` with no connection pool; all CRUD methods are stubs (file: `crates/slapper/src/storage/postgres.rs`)
- [Proxy] Documented `ProxyType` variants missing `Tor`; actual enum has 5 variants (Socks4, Socks5, Http, Https, Tor) (file: `crates/slapper/src/proxy/config.rs:9-22`)
- [Proxy] Documented rotation strategies list 3 (round-robin, least-used, random); actual has 5 (RoundRobin, Random, Weighted, LeastUsed, LowestLatency) (file: `crates/slapper/src/proxy/rotator.rs:25-37`)
- [WebSocket] Document says "Fully implemented" but all test methods are feature-gated behind `#[cfg(feature = "websocket")]`; without the feature, public API exists but methods return errors or are compile-time excluded (file: `crates/slapper/src/websocket/connection.rs:28`)
- [Supply Chain] Documented as "Dependency vulnerability scanner"; actual `scanner.rs` discovers manifest files and checks Dockerfile/GitHub Actions misconfigurations; does not scan dependencies for known vulnerabilities (file: `crates/slapper/src/supply_chain/scanner.rs`)
- [Wireless] Documented "WPA/WPA2 handshake capture analysis" is aspirational; actual code only does iwlist scan parsing and basic security type analysis (file: `crates/slapper/src/wireless/mod.rs:89-93`)
- [Diff] Documented `diff_findings_from_files()` function does not exist; closest is `load_findings_from_file()` which loads a single file (file: `crates/slapper/src/diff/mod.rs:103`)
- [Notify] Documented "retry logic" does not exist in webhook implementation; `send_webhook()` sends a single request with no retry loop or backoff (file: `crates/slapper/src/notify/webhook.rs:89`)
- [Notify] `WebhookEvent` variant documented as `Error`; actual variant name is `ScanError` (file: `crates/slapper/src/notify/webhook.rs:46`)
- [Container] Documented "Feature-gated behind appropriate flags"; actual has no `#[cfg(feature = ...)]` attributes anywhere in container module (file: `crates/slapper/src/container/mod.rs:1-66`)
- [Networking] Documented "BPF-style filters" for capture; actual filtering is a custom TCP/UDP/ICMP/port matching implementation, not true BPF (file: `crates/slapper/src/packet/capture.rs:276-306`)
- [Findings] `FindingStore` described as "In-memory finding storage"; actual is JSONL-based persistent file storage at `findings.jsonl` (file: `crates/slapper/src/findings/store.rs:20-21`)
- [Overview] Documented `ScanResults` at `scanner/mod.rs`; actual struct is at `waf/types.rs:188` (file: `waf/types.rs:188`)
- [Overview] Documented `FingerprintResult` (singular); actual struct is `FingerprintResults` (plural) (file: `crates/slapper/src/scanner/fingerprint.rs:83`)
- [Overview] Documented `FuzzResult` at `fuzzer/mod.rs`; actual defined at `fuzzer/engine/types.rs:10` (file: `crates/slapper/src/fuzzer/engine/types.rs:10`)
- [Overview] Documented `WafProfile` at `waf/types.rs`; actual defined at `waf/bypass/profiles.rs:9` (file: `crates/slapper/src/waf/bypass/profiles.rs:9`)
- [Overview] Documented `Pipeline` at `pipeline/mod.rs`; actual defined at `pipeline/executor.rs:38` (file: `crates/slapper/src/pipeline/executor.rs:38`)
- [Overview] "Commands enum (35+ variants)" — actual count is 37 (file: `crates/slapper/src/cli/mod.rs:83-201`)
- [TUI] Documented "app/ has 7 files" but actual directory has 18 files; missing: bookmarks.rs, command.rs, confirmation.rs, error.rs, export.rs, help_config.rs, input.rs, navigation.rs, notifications.rs, options.rs, tab_error.rs (file: `crates/slapper/src/tui/app/`)
- [TUI] Documented "7 components" but actual directory has 12 files; missing: empty_state.rs, help_bar.rs, http_options.rs, notifications.rs, palette.rs, search_popup.rs (file: `crates/slapper/src/tui/components/`)
- [TUI] Session path documented as `~/.slapper/sessions/`; actual uses `directories::ProjectDirs` with platform-specific resolution (e.g., `~/.local/share/slapper/sessions/` on Linux) (file: `crates/slapper/src/tui/session.rs:53-57`)
- [TUI] Documented "ThemeColors 30+ color fields"; actual has exactly 29 fields (file: `crates/slapper/src/tui/theme.rs:23-52`)
- [WAF] `is_bypass_successful()` documented as checking 4 conditions; actual also checks `body_looks_blocked()` and `response_diff.is_waf_blocked()` — 2 additional failure conditions undocumented (file: `crates/slapper/src/waf/bypass/mod.rs:131-164`)
- [Error] Documented "19+ variants"; actual count is 22 variants (file: `crates/slapper/src/error/mod.rs:43-116`)
- [Pipeline] `generate_html()` and `generate_csv()` documented as methods on `PipelineReport`; actual are free functions taking `&PipelineReport` as parameter (file: `crates/slapper/src/pipeline/report.rs:113,211`)
- [Recon] FxHashMap/FxHashSet count documented as "55 total collections across 14 components"; actual is 66+ per AGENTS.override.md (file: `recon/AGENTS.override.md:57`)
- [Recon] IAM pattern count documented as 12; actual is 13 (file: `recon/AGENTS.override.md:44`)
- [Feature Matrix] `full` feature documented as "Deprecated" and "currently fails to compile"; Cargo.toml now includes `k8s-openapi` features = ["v1_30"] which should resolve the issue (file: `crates/slapper/Cargo.toml:186-189`)
- [Findings] Confidence enum variants differ between modules: findings module has 5 variants (Confirmed, High, Medium, Low, Informational); output module `agent.rs` has 4 variants (Confirmed, Likely, Possible, Unlikely) — undocumented divergence (file: `crates/slapper/src/findings/mod.rs:37-43`, `crates/slapper/src/output/agent.rs:6-13`)
- [Fuzzer/WAF] `WAF_BLOCKED_STATUS_CODES` inconsistency: fuzzer uses 3 codes `[403, 406, 429]`; WAF module uses 4 codes `[403, 406, 429, 503]` — inconsistent bypass detection between modules (file: `crates/slapper/src/fuzzer/engine/utils.rs:18`, `crates/slapper/src/constants.rs:77`)

## High-Priority Improvements

- [Overview] Fix `lib.rs` docstring: update "22 payload types" to "30" and "26 products" to "34" (priority: high)
- [Feature Matrix] Correct "Features with deps" count from 18 to 16 (priority: high)
- [Findings] Enumerate all 18 fields of the canonical `Finding` struct in architecture doc (priority: high)
- [Findings] Document `FindingStatus` variants (New, Confirmed, AcceptedRisk, FalsePositive, Remediated, Reopened) (priority: high)
- [Error] Document all 14+ additional `From` impls and 3 feature-gated `From` impls (priority: high)
- [TUI] Document `app/` module completeness — list all 18 files, not just 7 (priority: high)
- [TUI] Document `components/` completeness — list all 12 files, not just 7 (priority: high)
- [TUI] Document `ui.rs` draw functions and `search.rs` GlobalSearch module (priority: high)
- [TUI] Document full `TabInput` interface — list all 25+ methods, not just a few (priority: high)
- [Overview] Fix type location claims in Key Types table: ScanResults, FingerprintResult, FuzzResult, WafProfile, Pipeline (priority: high)
- [Workflow] Fix `calculate_metrics()` to use `calculate_sla()` from `sla.rs` for accurate SLA violation counting (priority: high)
- [Notify] Add Discord dispatch to `notify_findings()` to match other notify methods (priority: high)
- [Notify] Add retry logic to webhook sends if reliability is a stated goal (priority: high)
- [Auth] Add `pub mod multi_protocol;` to `auth/mod.rs` to expose multi-protocol testing capabilities (priority: high)
- [Wireless] Correct "WPA/WPA2 handshake capture analysis" claim — module only does iwlist scanning and security type analysis (priority: high)
- [Diff] Implement `diff_findings_from_files()` or update doc to describe `load_findings_from_file()` + manual `diff_findings()` usage (priority: high)
- [Container] Either add feature-gating or correct "Feature-gated behind appropriate flags" claim (priority: high)
- [Findings] Correct FindingStore description from "In-memory" to "JSONL-based persistent file storage" (priority: high)
- [Pipeline] Document `PipelineReport` Display implementation and concurrent execution limitations (priority: medium)
- [Distributed] Document IP allowlist, connection limit, and rate limiting details (priority: medium)
- [Recon] Update FxHashMap/FxHashSet count from 55 to 66+ and IAM pattern count from 12 to 13 (priority: medium)
- [Proxy] Add Tor ProxyType variant and Weighted/LowestLatency rotation strategies to doc (priority: medium)
- [TUI] Document OverlayType (6 variants), PendingAction (4 variants), InputMode, and AppState enums (priority: medium)
- [TUI] Fix session path documentation to clarify platform-specific resolution (priority: medium)
- [TUI] Document `auth.rs` tab status — exists but is not part of `Tab` enum (priority: medium)
- [Output] Add `report_summary.rs` to FxHashMap migration list (priority: medium)
- [Error] Update variant count from "19+" to "22" for precision (priority: medium)
- [Findings] Note Confidence divergence between findings module (5 variants) and output module (4 variants) (priority: medium)
- [Overview] Update "Commands enum (35+ variants)" to "37 variants" (priority: low)
- [TUI] Fix ThemeColors count from "30+" to "29" (priority: low)
- [TUI] Fix HelpManager field path from `HelpManager.sections` to `HelpManager.content.sections` (priority: low)
- [TUI] Clarify SharedHistory uses `parking_lot::Mutex` not `std::sync::Mutex` (priority: low)
- [WAF] Document `body_looks_blocked()` function and clarify bypass category mapping (5 categories vs 3 modules) (priority: low)
- [Loadtest] Document CancellationToken, progress bar (indicatif), and Report trait (priority: low)
- [Error] Document `with_timeout()` builder pattern and reqwest::Error conversion logic (priority: low)
- [Compliance] Expand `generate_compliance_report()` to accept full finding data instead of just severity levels (priority: low)

## Stale Items Requiring Action

- [Workflow] `SlaTracking` type name in doc is stale — should be `SlaPolicy` and `SlaStatus`
- [Wireless] "WPA/WPA2 handshake capture analysis" claim is aspirational — no such code exists
- [Supply Chain] "Dependency vulnerability scanner" description is stale — it's a manifest discovery and configuration analysis tool
- [Diff] `diff_findings_from_files()` claim is stale — the function does not exist
- [Notify] "retry logic" claim is stale — no retry logic exists in webhook implementation
- [Overview] `lib.rs` module descriptions are stale (22 payload types → 30, 26 WAF products → 34)
- [Feature Matrix] `full` "currently fails to compile" claim may be stale — k8s-openapi v1_30 feature now included
- [Feature Matrix] `full` "Deprecated" status may need reconsideration if compilation issue is resolved
- [Recon] FxHashMap/FxHashSet count of 55 is stale — actual is 66+
- [Recon] IAM pattern count of 12 is stale — actual is 13
- [Scanner] Bug fix line numbers in 2026-05-22 and 2026-05-27 sections have drifted from current source
- [TUI] Session fix logs (lines 548-1715) are extensive — 1167 lines of fix logs out of 1715 total; consider moving to separate file
- [AI Agents] "Recent Bug Fixes (2026-05-22)" section line references may become stale
- [CLI Commands] "Bug Fixes and Consistency (2026-05-22)" section line numbers may drift
- [Config] "Key Security Fixes (2026-05-22)" section is historical and could be moved to changelog
- [Pipeline] Bug fix tables (2026-05-22, 2026-05-27) are historical records that should eventually move to changelog
- [Findings] "Fully implemented" claim is partially stale — schema is defined but cross-module migration is pending (3 module-specific types remain unmigrated)

## Statistics Summary

| Metric | Documented | Actual | Match |
|--------|-----------|--------|-------|
| Source files | N/A | 523 | N/A |
| Modules | 39 | 39 dirs | Yes |
| Tab count | 28 | 28 | Yes |
| Payload types | 30 | 30 | Yes |
| WAF products | 34 | 34 | Yes |
| NSE libraries | 169 | 169 | Yes |
| Output formats | 8 | 8 | Yes |
| CLI commands | "35+" | 37 | Partial |
| Error variants | "19+" | 22 | Partial |
| Features total | 28 | 28 | Yes |
| Features with deps | 18 (doc) | 16 (actual) | No |
| Marker-only features | 12 | 12 | Yes |
| TUI app files | 7 listed | 18 actual | No |
| TUI component files | 7 listed | 12 actual | No |
| Theme color fields | "30+" | 29 | No |
| Recon FxHashMap/FxHashSet | 55 | 66+ | No |
| Recon IAM patterns | 12 | 13 | No |
| WAF blocked codes (WAF) | 4 | 4 | Yes |
| WAF blocked codes (Fuzzer) | 3 | 3 | Yes (but inconsistent across modules) |
| Finding variants (findings mod) | 5 | 5 | Yes |
| Finding variants (output mod) | 4 | 4 | Yes (but undocumented divergence) |

## Accuracy Overview

| Document | Accuracy | Key Issues |
|----------|----------|------------|
| workflow.md | High | Incorrect SLA calc bug, SlaTracking type name wrong |
| vuln.md | High | Minor TriageStatus omission |
| wireless.md | Medium | WPA handshake analysis claim is aspirational |
| supply_chain.md | High | Scanner purpose mischaracterized, feature-gate missing |
| websocket.md | Medium | Feature gate not mentioned, test methods gated |
| storage.md | Medium | Stub DB implementation, wrong model type names |
| proxy.md | Medium | Missing Tor variant, missing 2 rotation strategies |
| notify.md | High | Missing Discord dispatch, missing retry logic |
| networking.md | Medium | "BPF-style filters" is inaccurate, missing CaptureBuilder/PcapWriter |
| integrations.md | High | No discrepancies found |
| diff.md | High | diff_findings_from_files() does not exist |
| loadtest.md | High | Minor typos, missing CancellationToken mention |
| container.md | Medium | Feature-gate claim incorrect |
| distributed.md | High | Minor line range offsets |
| compliance.md | High | No discrepancies found |
| browser.md | High | No discrepancies found |
| auth.md | Medium | Dead code: multi_protocol.rs unreachable |
| hunt.md | High | Feature gate not documented |
| nse_integration.md | High | No significant issues |
| scanner.md | High | Bug fix line numbers drifted |
| waf.md | High | Missing body_looks_blocked() check |
| fuzzer.md | High | WAF blocked code divergence noted |
| defense_lab.md | High | No issues found |
| recon.md | Medium | FxHashMap count and IAM pattern count wrong |
| ai_agents.md | High | Minor discrepancies |
| findings.md | Medium | FindingLifecycle type wrong, FindingStore description wrong |
| feature_matrix.md | Medium | Feature count math wrong, full compilation status stale |
| error.md | Medium | Variant count understated, missing From impls |
| pipeline.md | High | Method vs function distinction, minor line offsets |
| output.md | High | AttackGraphBuilder feature-gate claim incorrect |
| cli_commands.md | Medium-High | Minor line range and signature differences |
| config.md | High | Minor line number offsets |
| overview.md | Medium | Multiple type location errors, lib.rs docstrings stale |
| tui.md | Medium | app/ and components/ significantly underdocumented |
