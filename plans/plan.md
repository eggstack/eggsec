# Slapper Implementation Plan

**Created:** 2026-05-30
**Last Updated:** 2026-05-31
**Status:** Active

---

## Summary

This plan consolidates all findings from 20 architecture review documents. Items are organized into waves that can be parallelized. Each wave contains independent tasks that can be executed concurrently via sub-agents.

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

## Wave 0: Critical Bug Fixes (Parallel)

These are bugs in the codebase itself (not documentation). Can all be done in parallel.

| # | Module | File | Issue | Priority | Status |
|---|--------|------|-------|----------|--------|
| 0.1 | agent | `crates/slapper/src/agent/alerts/routing.rs:79` | `.expect("Failed to create fallback HTTP client")` will panic if client creation fails. Convert to graceful error handling with `?` or match | **Critical** | Pending |
| 0.2 | cli | `crates/slapper/src/cli/scan.rs:195` | `spoof_ip` field name inconsistent with `source_ip`/`source_port` convention used elsewhere. Rename to `source_ip` | Medium | Pending |
| 0.3 | cli | `commands/handlers/cluster.rs:349` | `.unwrap_or(22)` silently falls back to port 22 on parse failure. Consider `.unwrap_or_else(\|_\| 22)` for explicit intent (not a panic risk, but code clarity) | Low | Pending |

**Acceptance:** After these fixes, `cargo clippy --lib -p slapper` should show no new warnings in the affected files. Item 0.1 is the only true crash risk.

---

## Wave 1: Architecture Documentation Updates (Parallel)

All items are doc-only changes. Can be parallelized across sub-agents.

### Wave 1A: Statistical & Count Corrections

| # | File | Fix | Detail |
|---|------|-----|--------|
| 1.1 | `architecture/overview.md` | Source files: 526 → **522** | Count `.rs` files |
| 1.2 | `architecture/overview.md` | Feature flags: 30 → **28** | Cargo.toml has exactly 28 |
| 1.3 | `architecture/overview.md` | CLI commands: "35+" → **38** | Count match arms in `commands/handlers/mod.rs` |
| 1.4 | `architecture/tui.md` | Payload types: 31 → **30** | `PayloadType` enum has 30 variants |
| 1.5 | `architecture/tui.md` | Tab count headers: 29 → **28** | `Tab` enum has 28 variants |
| 1.6 | `architecture/scanner.md` | Endpoint count: 224 → **223** | Built-in endpoint paths in `scanner/endpoints.rs` |
| 1.7 | `architecture/waf.md` | XSS count: 18 → **17** | `waf/payloads/encoding.rs:74-175` |
| 1.8 | `architecture/waf.md` | SSRF count: 15 → **16** | `waf/payloads/encoding.rs:74-175` |
| 1.9 | `architecture/waf.md` | Traversal count: 11 → **10** | `waf/payloads/encoding.rs:74-175` |
| 1.10 | `architecture/waf.md` | WAF list incomplete: 25 → **34** | `waf/data/patterns.rs` |
| 1.11 | `architecture/feature_matrix.md` | Marker-only features: 10 → **11** | Includes `default` |
| 1.12 | `architecture/overview.md` | Output formats: 8 → **7** | JSON, CSV, HTML, Markdown, SARIF, JUnit, PDF |

### Wave 1B: Structural Documentation Updates

| # | File | Fix | Detail |
|---|------|-----|--------|
| 1.13 | `architecture/pipeline.md` | Defense-lab profiles: "planned" → **"implemented"** | All 5 profiles exist in `cli/mod.rs:262-266` and `pipeline/stage.rs:92-107` |
| 1.14 | `architecture/pipeline.md` | Add concurrent execution mode | `run_concurrent()` with `futures::future::join_all()` |
| 1.15 | `architecture/pipeline.md` | Add `PipelineReport.manifest` field | `Option<RunManifest>` undocumented |
| 1.16 | `architecture/config.md` | Add missing sub-configs | `ReconConfig`, `RemoteConfig`, `ExecutionPolicy` |
| 1.17 | `architecture/config.md` | Document `ScopeError` enum | 6 variants at `scope.rs:400-422` |
| 1.18 | `architecture/cli_commands.md` | Update `handle_no_command` line ref | 155-169 → **197-205** |
| 1.19 | `architecture/cli_commands.md` | Document `enforce_operation_policy()` | New method not in doc |
| 1.20 | `architecture/feature_matrix.md` | Add `api-schema` to feature table | Currently only in Notes section |
| 1.21 | `architecture/output.md` | Clarify `has_regressions()` threshold | Checks `>= Severity::High` (High AND Critical), not just Critical |
| 1.22 | `architecture/output.md` | Add `schedule.rs` to Core Features | `CronScheduler`, `ScanQueue` undocumented |
| 1.23 | `architecture/nse_integration.md` | Document async executor | `async_executor.rs` not mentioned |
| 1.24 | `architecture/nse_integration.md` | Remove marketing language | "Instant Capability", "Seamless Integration" inappropriate |

### Wave 1C: AI & MCP Documentation Fixes (Critical)

| # | File | Fix | Detail |
|---|------|-----|--------|
| 1.25 | `architecture/ai_agents.md` | Document `McpProfilePolicy` with **all 18 fields** | Doc shows only 7. Missing: `default_target_policy`, `allowed_tool_ids`, `denied_tool_ids`, `allowed_categories`, `denied_categories`, `allow_streaming`, `allow_sessions`, `allow_plan_endpoint`, `require_explicit_scope`, `allow_packet_features`, `denied_argument_keys` |
| 1.26 | `architecture/ai_agents.md` | Fix `TargetPolicy` variants | `TargetPolicy::None` doesn't exist. Actual: `AnyWithScopeEngine` |
| 1.27 | `architecture/ai_agents.md` | Fix `coding_agent()` policy doc | Missing 7+ field descriptions |
| 1.28 | `architecture/ai_agents.md` | Fix `ops_agent()` values | `max_concurrency`: 20→**50**, `max_timeout_ms`: 300,000→**600,000** |
| 1.29 | `architecture/ai_agents.md` | Fix `chat_completion()` visibility | It's **private**. Public method is `chat_completion_from_messages()` |
| 1.30 | `architecture/ai_agents.md` | Fix waf_bypass.rs line ref | 107 → **124-133** |
| 1.31 | `architecture/loadtest.md` | Fix `run_cli()` signature | Missing `async`, `&SlapperConfig` param, different return type |

### Wave 1D: Recon & Agent Module Docs

| # | File | Fix | Detail |
|---|------|-----|--------|
| 1.32 | `architecture/recon.md` | Document detached modules | `asn.rs`, `cve_lookup.rs`, `dns_enhanced.rs`, `ftp_auth.rs`, `smtp_auth.rs`, `ssh_auth.rs`, `ssl_audit.rs` — exist but not in public API |
| 1.33 | `architecture/recon.md` | Parallel task count: 14 → **13** | Cloud detection runs separately |
| 1.34 | `architecture/distributed.md` | Update Key Components line ranges | All stale (10-150 lines off) |
| 1.35 | `architecture/distributed.md` | Update `CommandMessage` table | Only 4 of 6 variants documented |

**Acceptance:** Run `cargo doc` to verify all referenced types exist. Verify line numbers against current source.

---

## Wave 2: TUI Uniform Look & Feel (Parallel sub-tasks)

These are pure TUI styling changes. Can be parallelized across sub-agents since they touch different files. See `plans/tui-uniform-look-and-feel.md` for full details.

### Wave 2A: Critical Fixes (1-3 lines each)

| # | File | Fix | Detail |
|---|------|-----|--------|
| 2.1 | `tui/components/popup.rs:130` | Add content text styling | `.style(Style::default().fg(tc!(text)))` |
| 2.2 | `tui/ui.rs:579` | Fix notification warning color | `tc!(status_running)` → `tc!(warning)` |
| 2.3 | 13 tab files | Standardize results borders | Change `Some(tc!(success))` and `Some(tc!(info))` → `None` in: recon, packet, load, proxy, hunt, browser, compliance, storage, integrations, workflow, vuln, fuzz, resume |

### Wave 2B: Input Block Standardization (~100 lines)

| # | Files | Fix |
|---|-------|-----|
| 2.4 | recon, packet, proxy, hunt, browser, compliance, storage, integrations, workflow, vuln | Add bordered input blocks (10 files) |

### Wave 2C: Empty State & Error Standardization (~60 lines)

| # | Files | Fix |
|---|-------|-----|
| 2.5 | stress, graphql, oauth, cluster, nse, report | Add `empty_state_paragraph()` (6 files) |
| 2.6 | graphql, oauth, cluster | Add early return error block (3 files) |

### Wave 2D: Polish (~8 lines)

| # | Files | Fix |
|---|-------|-----|
| 2.7 | `tui/components/scrollable.rs` | Scrollbar theme styling |
| 2.8 | `tui/app/dashboard.rs`, `tui/app/history.rs` | Consistent `Modifier::BOLD` usage |

**Acceptance:** Visual inspection in TUI, `cargo check -p slapper`, `cargo clippy -p slapper`.

---

## Wave 3: Agent & MCP Profile Productionization (Sequential phases)

This is the largest body of work. Phases must execute in order but some sub-phases can parallelize. See `plans/agents.md` for full details.

### Phase 1: Audit and Encode Profile Contract
- Create `McpProfilePolicy` struct (18 fields)
- Create `TargetPolicy` enum (4 variants: `ExplicitScopeOnly`, `LocalhostAndPrivateCidrsOnly`, `ScopeOrLocalDevOnly`, `AnyWithScopeEngine`)
- Create `ToolSelector` enum (`All`, `None`, `Exact`, `Category`, `Capability`)
- File: `tool/protocol/mcp/policy.rs`
- Tests: coding-agent cannot call hidden tools, ops-agent sees normal registry

### Phase 2: Filter MCP Tool Discovery by Profile
- Update `handle_tools_list` and `handle_tools_list_by_category` in `handlers/server.rs`
- Add `visible_tools_for_profile()` helper
- Coding-agent deny list: stress/load/packet/SSRF/command-injection/broad-recon/root-required/stealth

### Phase 3: Enforce Profile Policy in `tools/call`
- Call-time validation after `resolve_tool_id()`
- Validate tool ID, capability, target policy, timeout/concurrency budgets, denied arguments
- Fail-closed for first implementation

### Phase 4: Formalize Target Scope for Coding-Agent
- Allow loopback, private lab networks (with explicit enable)
- Deny public internet, cloud metadata (`169.254.169.254`), link-local
- Reuse existing `Scope`/`ScopeRule` machinery
- Tests for: localhost, 127.0.0.1, ::1, 10.x, 192.168.x, 172.16.x, 169.254.x

### Phase 5: Split Profile-Specific Resource Manifests
- Ops-agent resources: manifest, tools, vulnerabilities, safety-policy, task-schema, event-schema
- Coding-agent resources: manifest, safety-policy, finding-schema, workflow, tool-contracts

### Phase 6: Productionize MCP Transport Behavior
- HTTP: single JSON-RPC objects, profile-driven batch limits
- STDIO: single objects, no stdout logs, flush each response line
- SSE: include request ID, event type, progress percent
- New `McpIncoming` enum for untagged single/batch deserialization

### Phase 7: Add Stable Coding-Agent Output Schemas
- `CodingAgentFindingReport` struct with schema_version, target, profile, findings, evidence
- Finding includes: stable ID, severity, CWE/CAPEC, endpoint, reproduction note
- No exploit payload dumps by default

### Phase 8: Harden Agent Runtime
- `AgentRuntimeStatus` model
- Persist runtime metadata (atomic JSON state file)
- Graceful shutdown: stop scheduling, cancel/allow current scan, flush logs
- Controlled scan budgets: per-target timeout, per-agent concurrency cap

### Phase 9: Make Agent API Routes Production-Safe
- Auth: `Bearer` and `X-API-Key` consistently, constant-time comparison
- Registration: validate name, capabilities, reject duplicates
- Task creation: validate task_type, payload schema, max size
- Callback URL validation: reject redirects to forbidden IPs

### Phase 10: Codegg-Specific Server Ergonomics
- Stable invocation: `slapper mcp-serve --stdio --profile coding-agent`
- Sample configs: `examples/codegg-mcp.local.toml`
- No AI dependency in coding-agent — deterministic by default

### Phase 11: Update Documentation
- Update `docs/mcp-protocol.md`, `docs/AGENT.md`, `architecture/ai_agents.md`

### Phase 12: Tests and Validation Matrix
- Profile tests, discovery tests, call tests, transport tests, agent runtime tests, agent API tests

**Acceptance:** All phases complete, `cargo test --lib -p slapper` passes, profile enforcement tests pass.

---

## Wave 4: Output Module Documentation (Parallel)

| # | Item | Detail |
|---|------|--------|
| 4.1 | Document `report_summary.rs` | Risk narrative generation |
| 4.2 | Document `schedule.rs` | `CronScheduler`, `ScanQueue` |
| 4.3 | Document `DiffFinding` type | Missing from output doc |
| 4.4 | Document `TrendAnalyzer` LruCache storage | Different from other FxHashMap usages |
| 4.5 | Document `AttackGraphBuilder::to_html()` return type | Feature-gated (`advanced-hunting`) |

---

## Deferred Items (No Action Required)

| # | Module | Item | Rationale |
|---|---|-------|-----------|
| D.1 | ai_agents | MCP integration | Fully implemented in `tool/protocol/mcp/` with routes, handlers, streaming, auth, stdio transport, and tests |
| D.2 | scanner | Module complete | Zero bugs, zero pending work |
| D.3 | fuzzer | Module complete | Zero bugs, zero pending work |
| D.4 | waf | Module complete | Zero bugs, zero pending work |
| D.5 | networking | Module complete | Zero bugs, zero pending work |
| D.6 | distributed | Module complete | Zero bugs, zero pending work |
| D.7 | recon | Module complete | Zero bugs, zero pending work (detached modules documented in Wave 1D) |

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

## Parallelization Strategy

```
Wave 0 (Bug Fixes) ──────────────────────────────┐
                                                  │
Wave 1A (Counts) ───────────────┐                 │
Wave 1B (Structure) ────────────┤                 │
Wave 1C (AI/MCP Docs) ──────────┼─── All parallel │
Wave 1D (Recon/Agent Docs) ─────┘                 │
                                                  │
Wave 2A (Critical TUI) ─────────┐                 │
Wave 2B (Input Blocks) ─────────┤                 │
Wave 2C (Empty/Error) ──────────┼─── All parallel │
Wave 2D (Polish) ───────────────┘                 │
                                                  │
Wave 3 (Agent/MCP) ───────────── Sequential ──────┘
  Phase 1 → Phase 2 → Phase 3 → ...
  (Phases 1-4 can have internal parallelism)

Wave 4 (Output Docs) ────────── Parallel with Wave 2-3
```

**Key parallelization opportunities:**
- Wave 0 and Wave 1 can run simultaneously (different files)
- All Wave 1 sub-waves can run simultaneously
- All Wave 2 sub-waves can run simultaneously (different TUI files)
- Wave 3 phases must be sequential (each builds on previous)
- Wave 4 can run in parallel with Wave 2 and Wave 3

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
