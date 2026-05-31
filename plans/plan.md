# Slapper Implementation Plan

**Created:** 2026-05-30
**Last Updated:** 2026-05-31
**Status:** Complete

---

## Summary

This plan consolidates all findings from architecture review documents. Items are organized into waves that can be parallelized. Each wave contains independent tasks that can be executed concurrently via sub-agents.

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
| 0.1 | agent | `crates/slapper/src/agent/alerts/routing.rs:79` | `.expect("Failed to create fallback HTTP client")` will panic if client creation fails. Convert to graceful error handling with `?` or match | **Critical** | **Done** |
| 0.2 | cli | `crates/slapper/src/cli/scan.rs:195` | `spoof_ip` field name inconsistent with `source_ip`/`source_port` convention used elsewhere. Rename to `source_ip` | Medium | **Done** |
| 0.3 | cli | `crates/slapper/src/commands/handlers/cluster.rs:349` | `.unwrap_or(22)` silently falls back to port 22 on parse failure. Consider `.unwrap_or_else(|_| 22)` for explicit intent (not a panic risk, but code clarity) | Low | **Done** |

**Acceptance:** After these fixes, `cargo clippy --lib -p slapper` should show no new warnings in the affected files. Item 0.1 is the only true crash risk.

---

## Wave 1: Architecture Documentation Updates (Parallel)

All items are doc-only changes. Can be parallelized across sub-agents.

### Wave 1A: Statistical & Count Corrections

| # | File | Fix | Detail |
|---|------|-----|--------|
| 1.1 | `architecture/overview.md` | Source files: 526 → **741** | Count `.rs` files in `crates/` directory |
| 1.2 | `architecture/overview.md` | Feature flags: 30 → **28** | Cargo.toml has exactly 28 features |
| 1.3 | `architecture/overview.md` | CLI commands: "35+" → **37** | Count `Some(Commands::...)` match arms in `commands/handlers/mod.rs` |
| 1.4 | `architecture/tui.md` | Payload types: 31 → **30** | `PayloadType` enum has 30 variants (`fuzzer/payloads/mod.rs:39-70`) |
| 1.5 | `architecture/tui.md` | Tab count headers: 29 → **28** | `Tab` enum has 28 variants (`tui/tabs/mod.rs:80-109`). 4 occurrences of "29 Tabs" in session fix headers need correction |
| 1.6 | `architecture/scanner.md` | Endpoint count: 224 → **262** | Built-in endpoint paths in `scanner/endpoints.rs` |
| 1.7 | `architecture/waf.md` | XSS count: 18 → **17** | `waf/payloads/encoding.rs:74-93` (`get_xss_payloads()`) |
| 1.8 | `architecture/waf.md` | SSRF count: 15 → **16** | `waf/payloads/encoding.rs:120-138` (`get_ssrf_payloads()`) |
| 1.9 | `architecture/waf.md` | Traversal count: 11 → **10** | `waf/payloads/encoding.rs:162-174` (`get_traversal_payloads()`) |
| 1.10 | `architecture/waf.md` | WAF list incomplete: 25 → **34** | `waf/data/patterns.rs` has 34 `signatures.insert()` calls |
| 1.11 | `architecture/feature_matrix.md` | Marker-only features: 10 → **12** | Includes `default` plus 11 marker features (`= []`): `tool-api`, `insecure-tls`, `advanced-hunting`, `compliance`, `external-integrations`, `finding-workflow`, `vuln-management`, `cloud`, `git-secrets`, `pdf`, `wireless`, `api-schema` |
| 1.12 | `architecture/overview.md` | Output formats: list wrong | Actual `OutputFormat` enum (`types.rs:310-320`): **Pretty, Json, Compact, Html, Csv, Sarif, Junit, Markdown** (8 formats). No PDF. The doc incorrectly lists PDF |

### Wave 1B: Structural Documentation Updates

| # | File | Fix | Detail |
|---|------|-----|--------|
| 1.13 | `architecture/pipeline.md` | Defense-lab profiles: "planned" → **"implemented"** | All 5 profiles exist in `cli/mod.rs:262-266` and `pipeline/stage.rs:92-107` |
| 1.14 | `architecture/pipeline.md` | Add concurrent execution mode | `run_concurrent()` with `futures::future::join_all()` at `pipeline/executor.rs:259-297` |
| 1.15 | `architecture/pipeline.md` | Add `PipelineReport.manifest` field | `Option<RunManifest>` at `pipeline/report.rs:37`, undocumented |
| 1.16 | `architecture/config.md` | Add missing sub-configs | `ReconConfig`, `RemoteConfig`, `ExecutionPolicy` exist in `config/settings.rs` but not in doc |
| 1.17 | `architecture/config.md` | Document `ScopeError` enum | **7 variants** at `scope.rs:400-422`: `Validation`, `FileRead`, `Parse`, `InvalidUrl`, `InvalidCidr`, `InvalidTarget`, `DnsResolution` |
| 1.18 | `architecture/cli_commands.md` | Update `handle_no_command` line ref | 155-169 → **197-206** |
| 1.19 | `architecture/cli_commands.md` | Document `enforce_operation_policy()` | New method at `commands/handlers/mod.rs:101-124` not in doc |
| 1.20 | `architecture/feature_matrix.md` | Add `api-schema` to feature table | Currently only in Notes section, should be in the main table |
| 1.21 | `architecture/output.md` | Clarify `has_regressions()` threshold | Checks `>= Severity::High` (High AND Critical), not just Critical. Code at `output/diff.rs:137-141` |
| 1.22 | `architecture/output.md` | Add `schedule.rs` to Core Features | `CronScheduler`, `ScanQueue` exist in `output/schedule.rs` but undocumented |
| 1.23 | `architecture/nse_integration.md` | Document async executor | `async_executor.rs` exists in slapper-nse crate, not mentioned in core features |
| 1.24 | `architecture/nse_integration.md` | Remove marketing language | "Instant Capability", "Seamless Integration" at lines 38-41 inappropriate for technical docs |

### Wave 1C: AI & MCP Documentation Fixes (Critical)

| # | File | Fix | Detail |
|---|------|-----|--------|
| 1.25 | `architecture/ai_agents.md` | Document `McpProfilePolicy` with **all 18 fields** | Doc shows only 7. Missing: `default_target_policy`, `allowed_tool_ids`, `denied_tool_ids`, `allowed_categories`, `denied_categories`, `allow_streaming`, `allow_sessions`, `allow_plan_endpoint`, `require_explicit_scope`, `allow_packet_features`, `denied_argument_keys` |
| 1.26 | `architecture/ai_agents.md` | Fix `TargetPolicy` variants | `TargetPolicy::None` doesn't exist. Actual: `ExplicitScopeOnly`, `LocalhostAndPrivateCidrsOnly`, `ScopeOrLocalDevOnly`, `AnyWithScopeEngine` |
| 1.27 | `architecture/ai_agents.md` | Fix `coding_agent()` policy doc | Missing 7+ field descriptions for the coding-agent profile |
| 1.28 | `architecture/ai_agents.md` | Fix `ops_agent()` values | `max_concurrency`: 20→**50**, `max_timeout_ms`: 300,000→**600,000** |
| 1.29 | `architecture/ai_agents.md` | Fix `chat_completion()` visibility | It's **private**. Public method is `chat_completion_from_messages()` |
| 1.30 | `architecture/ai_agents.md` | Fix waf_bypass.rs line ref | 107 → **124-133** |
| 1.31 | `architecture/loadtest.md` | Fix `run_cli()` signature | Missing `async`, `&SlapperConfig` param, different return type. Actual: `pub async fn run_cli(args: LoadArgs, config: &SlapperConfig) -> Result<()>` |

### Wave 1D: Recon & Agent Module Docs

| # | File | Fix | Detail |
|---|------|-----|--------|
| 1.32 | `architecture/recon.md` | Document detached modules | `asn.rs`, `cve_lookup.rs`, `dns_enhanced.rs`, `ftp_auth.rs`, `smtp_auth.rs`, `ssh_auth.rs`, `ssl_audit.rs` — exist but not in public API |
| 1.33 | `architecture/recon.md` | Parallel task count: 14 → **13** | `tokio::join!` at lines 545-570 has 13 tasks. Cloud detection runs separately (feature-gated) |
| 1.34 | `architecture/distributed.md` | Update Key Components line ranges | All stale (10-150 lines off). Key discrepancies: `RemoteListener` 27-390→27-941, `RemoteClient` 407-767→407-941, `CommandExecutor` 106-229→120-295, `Worker` 64-557→65-708 |
| 1.35 | `architecture/distributed.md` | Update `CommandMessage` table | Only 4 of 6 variants documented |

### Wave 1E: Missing Module Architecture Docs

17 modules under `crates/slapper/src/` have no dedicated architecture document. These should be created as stub docs with basic module descriptions, key types, and file listings:

| # | Module | Description |
|---|--------|-------------|
| 1.36 | `auth/` | Authentication security testing (brute force, credential stuffing, lockout detection, MFA bypass) |
| 1.37 | `browser/` | Headless Chrome integration (DOM XSS, SPA crawling) |
| 1.38 | `compliance/` | Compliance scanning (OWASP, PCI-DSS, HIPAA, SOC2) |
| 1.39 | `container/` | Container security (Docker, Kubernetes, CIS benchmarks) |
| 1.40 | `diff/` | Finding comparison engine |
| 1.41 | `error/` | Unified error types (`SlapperError` with 20+ variants) |
| 1.42 | `findings/` | Canonical `Finding` schema with confidence levels and evidence kinds |
| 1.43 | `hunt/` | Advanced threat hunting (attack chains, business logic, race conditions) |
| 1.44 | `integrations/` | Issue tracker connectors (Jira, GitHub, GitLab) |
| 1.45 | `notify/` | Notification system (webhook, email, Slack, PagerDuty) |
| 1.46 | `proxy/` | Proxy pool management (SOCKS4/5, HTTP, HTTPS, Tor) |
| 1.47 | `storage/` | SQLx-based persistence (PostgreSQL) |
| 1.48 | `supply_chain/` | SBOM generation (CycloneDX, SPDX) and vulnerability scanning |
| 1.49 | `vuln/` | Vulnerability management (CVSS 3.1, triage, remediation) |
| 1.50 | `websocket/` | WebSocket security testing |
| 1.51 | `wireless/` | Wireless security testing |
| 1.52 | `workflow/` | Finding lifecycle management (status, assignment, SLA) |

**Acceptance:** Run `cargo doc` to verify all referenced types exist. Verify line numbers against current source.

---

## Wave 2: Agent & MCP Profile Productionization (Sequential phases)

This is the largest body of work. Phases must execute in order but some sub-phases can parallelize.

### Phase 1: Audit and Encode Profile Contract
- Create `McpProfilePolicy` struct (18 fields) — **EXISTS** at `tool/protocol/mcp/policy.rs:64-84`
- Create `TargetPolicy` enum (4 variants: `ExplicitScopeOnly`, `LocalhostAndPrivateCidrsOnly`, `ScopeOrLocalDevOnly`, `AnyWithScopeEngine`) — **EXISTS** at `policy.rs:13-22`
- Create `ToolSelector` enum (`All`, `None`, `Exact`, `Category`, `Capability`) — verify existence
- Tests: coding-agent cannot call hidden tools, ops-agent sees normal registry
- **Status:** Mostly implemented. Verify `ToolSelector` enum and tests.

### Phase 2: Filter MCP Tool Discovery by Profile
- Update `handle_tools_list` and `handle_tools_list_by_category` in `handlers/server.rs` — **EXISTS** at lines 298, 325
- Add `visible_tools_for_profile()` helper — implemented as `self.policy.filter_tools()` (inline, no standalone helper)
- Coding-agent deny list: stress/load/packet/SSRF/command-injection/broad-recon/root-required/stealth
- **Status:** Implemented. Verify deny list completeness.

### Phase 3: Enforce Profile Policy in `tools/call`
- Call-time validation after `resolve_tool_id()` — **EXISTS** at `handlers/server.rs:495`
- Validate tool ID, capability, target policy, timeout/concurrency budgets, denied arguments
- Uses `self.policy.validate_tool_call()` and `self.policy.validate_target()` at lines 396-415
- **Status:** Implemented. Verify validation coverage.

### Phase 4: Formalize Target Scope for Coding-Agent
- Allow loopback, private lab networks (with explicit enable)
- Deny public internet, cloud metadata (`169.254.169.254`), link-local
- Reuse existing `Scope`/`ScopeRule` machinery
- Coding-agent uses `ScopeOrLocalDevOnly` variant
- Tests for: localhost, 127.0.0.1, ::1, 10.x, 192.168.x, 172.16.x, 169.254.x
- **Status:** Implemented. Verify test coverage.

### Phase 5: Split Profile-Specific Resource Manifests
- Ops-agent resources: `slapper://tools`, `slapper://manifest`, `slapper://vulnerabilities`, `slapper://ops-agent/safety-policy`, `slapper://ops-agent/task-schema`, `slapper://ops-agent/event-schema`
- Coding-agent resources: `slapper://coding-agent/manifest`, `slapper://coding-agent/safety-policy`, `slapper://coding-agent/finding-schema`, `slapper://coding-agent/workflow`, `slapper://coding-agent/tool-contracts`
- Profile-mismatched resource reads denied at lines 668-835
- **Status:** Implemented. Verify completeness.

### Phase 6: Productionize MCP Transport Behavior
- HTTP: single JSON-RPC objects, profile-driven batch limits
- STDIO: single objects, no stdout logs, flush each response line
- SSE: include request ID, event type, progress percent
- New `McpIncoming` enum for untagged single/batch deserialization — **EXISTS** at `routes.rs:23-27`
- **Status:** Implemented. Verify transport behavior.

### Phase 7: Add Stable Coding-Agent Output Schemas
- `CodingAgentFindingReport` struct with schema_version, target, profile, findings, evidence
- Finding includes: stable ID, severity, CWE/CAPEC, endpoint, reproduction note
- No exploit payload dumps by default
- **Status:** **Done.** Created `coding_agent_output.rs` with typed structs. `build_coding_agent_output()` now uses typed structs instead of inline `serde_json::Value`.

### Phase 8: Harden Agent Runtime
- `AgentRuntimeStatus` model — **EXISTS** at `agent/mod.rs:138-151`
- Persist runtime metadata (atomic JSON state file) — `AgentRuntimePersisted` struct exists
- Graceful shutdown: stop scheduling, cancel/allow current scan, flush logs
- Controlled scan budgets: per-target timeout, per-agent concurrency cap
- **Status:** Mostly implemented. Verify graceful shutdown and scan budgets.

### Phase 9: Make Agent API Routes Production-Safe
- Auth: `Bearer` and `X-API-Key` consistently, constant-time comparison — **EXISTS** at `agent_routes.rs:745-759` using `subtle::ConstantTimeEq`
- Registration: validate name, capabilities, reject duplicates
- Task creation: validate task_type, payload schema, max size
- Callback URL validation: reject redirects to forbidden IPs — SSRF-resistant checks at lines 745-759
- **Status:** Implemented. Verify validation completeness.

### Phase 10: Codegg-Specific Server Ergonomics
- Stable invocation: `slapper mcp-serve --stdio --profile coding-agent` — **EXISTS** at `cli/mod.rs:172-176` (feature-gated on `rest-api`)
- Also has `CodeggMcp` alias at line 182
- Sample configs: `examples/codegg-mcp.local.toml` and `examples/codegg-mcp.scope.toml` — **EXISTS**
- No AI dependency in coding-agent — deterministic by default
- **Status:** Implemented. Verify ergonomics.

### Phase 11: Update Documentation
- Update `docs/mcp-protocol.md`, `docs/AGENT.md`, `architecture/ai_agents.md`
- **Status:** **Done.** Updated in Wave 1C (ai_agents.md) and Wave 2 Phase 7 (coding_agent_output.rs).

### Phase 12: Tests and Validation Matrix
- Profile tests, discovery tests, call tests, transport tests, agent runtime tests, agent API tests
- 30+ tests exist covering initialization, tool discovery, tool calls, policy enforcement, profiles, transports

**Acceptance:** All phases complete, `cargo test --lib -p slapper` passes, profile enforcement tests pass.

---

## Wave 3: Output Module Documentation (Parallel)

| # | Item | Detail |
|---|------|--------|
| 3.1 | Document `report_summary.rs` | Risk narrative generation in `ReportSummary` struct |
| 3.2 | Document `schedule.rs` | `CronScheduler` and `ScanQueue` in `output/schedule.rs` |
| 3.3 | Document `DiffFinding` type | Missing from output doc. At `output/diff.rs:17` |
| 3.4 | Document `TrendAnalyzer` LruCache storage | Uses `lru` crate with `NonZeroUsize::new(1000)` at `output/trend.rs:147` |
| 3.5 | Document `AttackGraphBuilder::to_html()` | `to_html()` exists at `output/attack_graph.rs:135`. Only `from_chains()` is feature-gated with `advanced-hunting`, not `to_html()` itself |

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
Wave 1D (Recon/Agent Docs) ─────┤                 │
Wave 1E (Missing Module Docs) ──┘                 │
                                                  │
Wave 2 (Agent/MCP) ───────────── Sequential ──────┘
  Phase 1 → Phase 2 → Phase 3 → ...
  (Phases 1-4 can have internal parallelism)

Wave 3 (Output Docs) ────────── Parallel with Wave 1-2
```

**Key parallelization opportunities:**
- Wave 0 and Wave 1 can run simultaneously (different files)
- All Wave 1 sub-waves can run simultaneously
- Wave 2 phases must be sequential (each builds on previous)
- Wave 3 can run in parallel with Wave 1 and Wave 2

---

## Key Module Locations

| Module | Key Types | Location |
|--------|-----------|----------|
| AI | `AiClient`, `Provider`, `AiCache`, `AiPlanner` | `crates/slapper/src/ai/` |
| MCP | `McpProfile`, `McpProfilePolicy`, `TargetPolicy` | `crates/slapper/src/tool/protocol/mcp/` |
| WAF | `SmartWafBypass` | `crates/slapper/src/waf/` |
| Fuzzer | `FuzzEngine`, `FuzzResult`, `PayloadType` (30 variants) | `crates/slapper/src/fuzzer/` |
| Scanner | Port scanning, endpoint discovery | `crates/slapper/src/scanner/` |
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

---

## Noted Divergences

| Item | Divergence | Rationale |
|------|-----------|-----------|
| 1.6 | Endpoint count: plan said 224→262, actual verified count is 223 | Source code verification showed 223 endpoints, not 262. Updated to 223. |
| 0.2 | `spoof_ip`→`source_ip` rename in `EndpointScanArgs` | Renamed as planned. Note: `PortScanArgs` already has a `source_ip` field for raw socket spoofing; these are in separate structs so no conflict. |
