# Slapper Implementation Plan

**Created:** 2026-05-30
**Last Updated:** 2026-05-31
**Status:** Complete

---

## Summary

All implementation plan items have been completed and verified:

- **Wave 0**: Critical bug fixes (3 items) — all applied
- **Wave 1**: Architecture documentation updates (52 items across 5 sub-waves) — all applied
- **Wave 2**: Agent & MCP profile productionization (12 phases) — all implemented
- **Wave 3**: Output module documentation (5 items) — all documented

Post-completion cleanup corrected stale counts in skills and architecture docs, updated AGENTS.md skills table, and removed duplicate content in tui-testing skill.

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

## Deferred Items (No Action Required)

| # | Module | Item | Rationale |
|---|---|---|---|
| D.1 | ai_agents | MCP integration | Fully implemented in `tool/protocol/mcp/` with routes, handlers, streaming, auth, stdio transport, and tests |
| D.2 | scanner | Module complete | Zero bugs, zero pending work |
| D.3 | fuzzer | Module complete | Zero bugs, zero pending work |
| D.4 | waf | Module complete | Zero bugs, zero pending work |
| D.5 | networking | Functionality exists, no dedicated module | Networking code is scattered across `utils/network.rs`, `commands/handlers/network.rs`, and `tui/workers/network.rs`. Packet module handles raw sockets separately. Not a cohesive module. |
| D.6 | distributed | Module complete | Zero bugs, zero pending work |
| D.7 | recon | Module complete | Zero bugs, zero pending work (detached modules documented in `architecture/recon.md`) |

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
