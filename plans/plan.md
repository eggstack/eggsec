# Slapper Implementation Plan

**Created:** 2026-05-30
**Last Updated:** 2026-06-02
**Status:** All items complete

---

## Summary

Previous implementation waves (0–7) are complete. A full architecture review of 34 documents against the codebase identified **7 high-priority bugs**, **20 discrepancies**, and **23+ improvement items**. All have been resolved.

---

## Completed Waves

### Wave 0: Critical Bug Fixes ✅
- Clock skew panic prevention (`routing.rs`)
- `spoof_ip` rename
- `unwrap_or` clarity improvements

### Wave 1: Architecture Documentation (52 items) ✅
- 5 sub-waves (1A-1E): counts, structure, AI/MCP, recon, stub modules

### Wave 2: Agent & MCP Profile Productionization (12 phases) ✅
- Phase 7: `CodingAgentFindingReport` typed struct (new)
- All other phases pre-existing

### Wave 3: Output Module Documentation ✅

### Wave 4: Critical Bug Fixes ✅
- 4.1 SLA calculation bug fix (`workflow/mod.rs`)
- 4.2 Discord notify dispatch bug (`notify/mod.rs`)
- 4.3 Storage module stubs documented (`storage/postgres.rs`)
- 4.4 Feature matrix math error (`architecture/feature_matrix.md`)
- 4.5 Findings architecture doc wrong type (`architecture/findings.md`)
- 4.6 Lib.rs stale docstrings
- 4.7 ~~Output AttackGraph feature gate~~ — Removed (not a bug)

### Wave 5: Type Name & Count Corrections (15 items) ✅
- All type names, counts, and descriptions corrected across architecture docs

### Wave 6: Documentation Gaps (14 items) ✅
- **Group A:** Error From impls, Recon counts, Proxy completeness, Output report_summary.rs
- **Group B:** TUI completeness, Pipeline executor fields, Distributed completeness, Loadtest completeness, Findings completeness
- **Group C:** Networking completeness, Container missing types, Compliance report.rs, Vuln completeness, Hunt feature gate

### Wave 7: Uncovered Module Documentation (9 items) ✅
- Created `architecture/` docs for: stress, utils, types, constants, probe, auth_context, logging, macros, generated

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
