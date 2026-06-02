# Slapper Implementation Plan

**Created:** 2026-05-30
**Last Updated:** 2026-06-02
**Status:** Complete

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

### 4.1 Workflow SLA Calculation Bug ✅
- **File:** `crates/slapper/src/workflow/mod.rs:35-37`
- **Status:** FIXED — Added `findings` field to `WorkflowReport`, `calculate_metrics()` now calls `calculate_sla()` per open finding and counts actual violations. 5 new tests added.
- **Worktree:** `wt-wave4-sla-fix` (commit 1dea2fc)

### 4.2 Notify Discord Dispatch Bug ✅
- **File:** `crates/slapper/src/notify/mod.rs:199-258`
- **Status:** FIXED — Added Discord dispatch block to `notify_findings()` after Teams block, matching pattern in `notify_scan_complete()`.
- **Worktree:** `wt-wave4-notify-fix` (commit 0131b2b)

### 4.3 Storage Module Stubs ✅
- **File:** `crates/slapper/src/storage/postgres.rs:19-56`
- **Status:** FIXED — Added `/// WARNING: Stub implementation - not connected to a real database` doc comments to struct and all methods.
- **Worktree:** `wt-wave4-docfix` (commit 7f6b64f)

### 4.4 Feature Matrix Math Error ✅
- **File:** `architecture/feature_matrix.md:7-11`
- **Status:** FIXED — Changed "Features with deps" from 18 to 16 in summary table.
- **Worktree:** `wt-wave4-docfix` (commit 7f6b64f)

### 4.5 Findings Architecture Doc Wrong Type ✅
- **File:** `architecture/findings.md:21`
- **Status:** FIXED — Replaced `FindingLifecycle` with `FindingStatus` in Key Types table, updated description.
- **Worktree:** `wt-wave4-docfix` (commit 7f6b64f)

### 4.6 Lib.rs Stale Docstrings ✅
- **File:** `crates/slapper/src/lib.rs:16-17`
- **Status:** FIXED — Updated "22 payload types" to "30" and "26 products" to "34".
- **Worktree:** `wt-wave4-docfix` (commit 7f6b64f)

### 4.7 ~~Output AttackGraph Feature Gate~~ [REMOVED — Not a Bug]
- **Note:** The `attack_graph` module IS properly feature-gated at `output/mod.rs:51` with `#[cfg(feature = "advanced-hunting")]`. The re-exports at lines 79-82 are also gated. No action needed.

---

## Wave 5: Type Name & Count Corrections (All Parallelizable)

All items are independent doc fixes. Can run in parallel with Wave 4.

### 5.1 Workflow SlaTracking → SlaPolicy + SlaStatus ✅
- **Status:** FIXED in wt-wave5 (commit 864b378)

### 5.2 Storage ScanModel/FindingModel Names ✅
- **Status:** FIXED in wt-wave5 (commit 864b378)

### 5.3 Error Variant Count ✅
- **Status:** FIXED in wt-wave5 (commit 864b378)

### 5.4 Recon FxHashMap Count ✅
- **Status:** ALREADY CORRECT — no changes needed

### 5.5 TUI File Counts ✅
- **Status:** FIXED in wt-wave5 (commit 864b378)

### 5.6 ThemeColors Field Count ✅
- **Status:** FIXED in wt-wave5 (commit 864b378)

### 5.7 Overview Type Location Corrections ✅
- **Status:** FIXED in wt-wave5 (commit 864b378)

### 5.8 Notify WebhookEvent Variant Name ✅
- **Status:** FIXED in wt-wave5 (commit 864b378)

### 5.9 Container Feature Gate Documentation ✅
- **Status:** FIXED in wt-wave5 (commit 864b378)

### 5.10 Findings FindingStore Description ✅
- **Status:** FIXED in wt-wave5 (commit 864b378)

### 5.11 Networking BPF Description ✅
- **Status:** FIXED in wt-wave5 (commit 864b378)

### 5.12 Diff diff_findings_from_files() Claim ✅
- **Status:** FIXED in wt-wave5 (commit 864b378)

### 5.13 WebSocket Feature Gate Documentation ✅
- **Status:** FIXED in wt-wave5 (commit 864b378)

### 5.14 Supply Chain Scanner Description ✅
- **Status:** FIXED in wt-wave5 (commit 864b378)

### 5.15 Wireless Handshake Claim ✅
- **Status:** FIXED in wt-wave5 (commit 864b378)

---

## Wave 6: Documentation Gaps (3 Parallel Groups) ✅
- **Status:** ALL COMPLETED in wt-wave6 (commit 6c88299)

Group A:
- 6.1 Error Module From Impls ✅
- 6.2 Recon Module Counts ✅
- 6.3 Proxy Module Completeness ✅
- 6.4 Output Module report_summary.rs ✅

Group B:
- 6.5 TUI Module Completeness ✅
- 6.6 Pipeline Executor Fields ✅
- 6.7 Distributed Module Gaps ✅
- 6.8 Loadtest Module Gaps ✅
- 6.9 Findings Module Completeness ✅

Group C:
- 6.10 Networking Module Gaps ✅
- 6.11 Container Module Missing Types ✅
- 6.12 Compliance Module report.rs ✅
- 6.13 Vuln Module Gaps ✅
- 6.14 Hunt Module Feature Gate ✅

---

## Wave 7: Uncovered Module Documentation (Parallelizable) ✅
- **Status:** ALL COMPLETED in wt-wave7 (commit acb6cc6)

| Module | Status | File |
|--------|--------|------|
| 7.1 stress/ | ✅ | `architecture/stress.md` |
| 7.2 utils/ | ✅ | `architecture/utils.md` |
| 7.3 types.rs | ✅ | `architecture/types.md` |
| 7.4 constants.rs | ✅ | `architecture/constants.md` |
| 7.5 probe.rs | ✅ | `architecture/probe.md` |
| 7.6 auth_context/ | ✅ | `architecture/auth_context.md` |
| 7.7 logging/ | ✅ | `architecture/logging.md` |
| 7.8 macros.rs | ✅ | `architecture/macros.md` |
| 7.9 generated/ | ✅ | `architecture/generated.md` |

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
