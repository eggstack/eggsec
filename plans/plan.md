# Slapper Implementation Plan

**Created:** 2026-05-30
**Last Updated:** 2026-05-30
**Status:** All Items Complete (verified 2026-05-30)

---

## Summary

All 4 waves of implementation are complete. The original 51-item plan plus all wave-specific work has been verified and merged to main.

## Deferred Items (No Action Required)

| # | Module | Item | Rationale |
|---|---|-------|-----------|
| 24 | ai_agents | MCP integration | Fully implemented in `tool/protocol/mcp/` with routes, handlers, streaming, auth, stdio transport, and tests. No remaining work. |

## Completed Items (2026-05-30)

### Dependency Scanning Enhancement

Implemented Ruby, PHP, and Java manifest file scanning:

| Scanner | Files | Location |
|---------|-------|----------|
| RubyScanner | Gemfile, Gemfile.lock | `recon/dependency_scan/ruby/mod.rs` |
| PhpScanner | composer.json | `recon/dependency_scan/php/mod.rs` |
| JavaScanner | pom.xml | `recon/dependency_scan/java/mod.rs` |

All 6 ecosystem scanners now operational: npm, cargo, go, ruby, php, java.

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

## Key Module Locations

| Module | Key Types | Location |
|--------|-----------|----------|
| AI | `AiClient`, `Provider`, `AiCache`, `AiPlanner` | `crates/slapper/src/ai/` |
| MCP | `McpProfile`, `McpProfilePolicy`, `TargetPolicy` | `tool/protocol/mcp/` |
| WAF | `SmartWafBypass` | `crates/slapper/src/waf/` |
| Scanner | `PayloadType` (30 variants) | `types.rs` |
| Fuzzer | `FuzzEngine`, `FuzzResult` | `crates/slapper/src/fuzzer/` |
| TUI | 28 tabs, event loop | `crates/slapper/src/tui/` |
| Config | `SlapperConfig` | `crates/slapper/src/config/` |
| Output | Report formatting, exports | `crates/slapper/src/output/` |
| Recon | `dependency_scan` standalone | `crates/slapper/src/recon/dependency_scan/` |

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

---

## Non-Goals

- Do NOT add new offensive capability
- Do NOT reintroduce Python/Ruby plugin runtimes
- Do NOT publish crates or flip visibility unless instructed
- Do NOT invent domains/organizations/support contacts
- Do NOT claim production maturity for experimental features
- Do NOT remove NSE support
- Do NOT perform large architectural rewrites in single passes