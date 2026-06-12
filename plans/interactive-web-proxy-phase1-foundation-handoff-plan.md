# Interactive Web Proxy / Traffic Interception Loadout - Phase 1 Foundation Implementation Handoff Plan

**Date**: 2026-06-12  
**Status**: Ready for Execution on Feature Branch  
**Phase**: 1 — Foundations: Core HTTP/HTTPS MITM Proxy Server + Flow Logging + CLI + Dry-Run + Safety/Policy + Reporting Bridge  
**Parent Documents**:
- `plans/interactive-web-proxy-loadout-design-plan.md` (authoritative detailed design)
- `plans/interactive-web-proxy-implementation-roadmap.md` (high-level sequencing & philosophy)
**Precedent**: `plans/database-pentesting-phase1-foundation-handoff-plan.md`, `plans/mobile-dynamic-phase1-implementation-handoff-plan.md`, wireless phase handoff plans  
**Target Branch**: `feature/interactive-web-proxy-loadout` (already created)  
**Authoring Note**: This document provides the detailed, actionable task breakdown for Phase 1 implementation. It translates the parent design plan and roadmap into concrete deliverables, recommended implementation order (lowest risk first), success criteria, and cross-cutting updates. Follow the exact execution pattern established by the mobile-dynamic, wireless-active, and db-pentest loadouts. Phase 0 (planning, design plan, high-level roadmap, and feature branch creation) is complete as of 2026-06-12.

---

## 1. Phase 1 Executive Summary & Scope

**Goal**: Deliver the foundational infrastructure + first real defensive value for interactive manual web proxy / traffic interception: a working, safely gated MITM proxy server (HTTP + HTTPS with CA support) capable of capturing and logging traffic flows, with complete safety model integration, dry-run support, basic CLI usability, and reporting bridge. This enables immediate non-interactive traffic inspection and logging in authorized lab environments.

**In Scope for Phase 1**:
- Feature flag `web-proxy` + Cargo.toml plumbing (propagated to CLI and TUI)
- Core module structure under `crates/eggsec/src/proxy/` (or new `web_proxy/` submodule): types, CA handling, basic MITM server, flow capture, utils (redaction/pretty-print), bridge
- Full `EnforcementContext` / policy integration (new risk tier/capability, confirmation paths, audit records, `--allow-web-proxy`)
- Basic HTTP + HTTPS MITM proxy server (listen, handle requests, perform TLS interception with generated or user-provided CA, capture request/response)
- Flow model + in-memory + optional file logging to `WebProxySessionReport`
- Complete CLI surface: `eggsec proxy intercept` with dry-run, JSON output, budgets, provenance prompt, and `--allow-web-proxy`
- `to_scan_report_data_proxy()` bridge + initial finding categories (`proxy-intercept-*`, `web-traffic-*`)
- Dry-run path exercising 100% of report generation, CA logic, and policy without binding or network activity
- Unit + integration tests + lab smoke tests (dockerized targets like httpbin/dvwa)
- Documentation, architecture, README, AGENTS, CA install guide, and example updates

**Out of Scope for Phase 1** (deferred to later phases):
- Interactive TUI tab or editing capabilities (Phase 2)
- WebSocket, HTTP/2, or gRPC protocol support (Phase 3+)
- Rule engine or persistent intercept rules (Phase 3)
- Manipulation / edit / forward / drop / replay (Phase 2)
- Pipeline `ScanProfile` integration or dedicated proxy regression profiles
- Advanced correlation or evidence bundles with other loadouts
- Transparent proxy mode or high-scale production use

**Success Vision**: After Phase 1, a user with a lab target can run a safely gated `eggsec proxy intercept --listen 127.0.0.1:8080 --dry-run --json` and a real non-interactive capture session (browser/curl → proxy → target) that produces high-quality, bridgeable `WebProxySessionReport` output while respecting every existing Eggsec safety control. HTTPS interception works with a generated CA (clear install instructions provided).

---

## 2. Key Decisions Confirmed for Phase 1

- **Feature flag name**: `web-proxy` (concise; alternative `interactive-proxy` or `mitm-proxy` deferred)
- **Proxy engine approach**: Extend existing `crates/eggsec/src/proxy/` module. Primary: direct `hyper` + `hyper-rustls` + custom CA signer for full control and minimal new dependencies. Evaluate lightweight wrapper (hudsucker) as fallback if complexity grows. Pure-Rust priority.
- **CA handling**: Use `rcgen` for self-signed CA generation + per-host server cert signing. Support user-provided CA/cert/key. Clear separation and warnings. Fingerprint always included in reports.
- **Authorization mechanism**: Integrate with existing `LoadedScope` / scope.toml for host patterns where practical; add lightweight optional `proxy-manifest.toml` support if needed for extra budgets. Enforcement via `EnforcementContext`.
- **Standalone defense-lab surface**: Confirmed — no MCP/agent `SecurityTool` registration in Phase 1 (reporting bridge only).
- **Initial flow capture priority**: HTTP + HTTPS request/response headers + body (with size truncation/redaction), timing, status. No WebSocket/HTTP2 yet.
- **Finding categories prefix**: `proxy-intercept-*` and `web-traffic-*`
- **Dry-run requirement**: Must generate complete, valid `WebProxySessionReport` JSON (with synthetic flows, policy decisions, CA info) with zero server binding or network activity.
- **Redaction**: Default patterns for common PII/tokens/secrets in headers and bodies; configurable later.

---

## 3. Detailed Deliverables & Task Breakdown

### 3.1 Infrastructure & Feature Plumbing
1. Add `web-proxy = []` feature to `crates/eggsec/Cargo.toml` (and include in `full` feature). Propagate to `crates/eggsec-cli/Cargo.toml` and `crates/eggsec-tui/Cargo.toml`.
2. Add required optional dependencies behind the feature (e.g., `hyper`, `hyper-rustls`, `rcgen`, `rustls`, `tokio` extensions if needed).
3. Update any build.rs or feature propagation logic for CLI/TUI.
4. Create / extend directory structure under `crates/eggsec/src/proxy/`: `mod.rs` (public exports, dispatcher), `types.rs`, `ca.rs`, `server.rs` (or `mitm.rs`), `interceptor.rs`, `utils.rs`, `bridge.rs`.

### 3.2 Core Types & Shared Logic
5. Define `ProxyFlow`, `WebProxySessionReport`, `ManipulationRecord` (for future), `InterceptRule` (basic), `WebProxyTarget`, `BudgetUsage`, `ProxyError` in `types.rs`.
6. Implement redaction helpers (configurable + default PII/token patterns) and pretty-print utilities (JSON/XML/text, hex fallback).
7. Implement connection string / target parsing utilities and budget tracking.
8. Create `to_scan_report_data_proxy()` function in `bridge.rs` that converts `WebProxySessionReport` to `ScanReportData` with proper prefixed categories and redacted evidence.

### 3.3 Policy, Safety & EnforcementContext Integration
9. Extend `OperationRisk` or add `TrafficInterception` / `WebProxyManipulation` capability flag (coordinate with core policy team).
10. Implement policy decision record fields for proxy operations (listen addr, CA fingerprint, intercept rules active, flows captured, manifest/scope match, budgets used, actions performed).
11. Add new confirmation/denial paths in `EnforcementContext::evaluate()` for traffic interception operations.
12. Wire `--allow-web-proxy` flag (or `--allow-traffic-intercept`) + `--manual-override-reason` handling (narrow override, audited).
13. Implement provenance confirmation prompt logic for real runs ("Confirm this proxy session targets only lab systems you control...").
14. Ensure dry-run path bypasses network/server binding but still produces full policy decision record + complete report skeleton.
15. Add prominent lab-use warnings and disclaimers in CLI help text and all output.

### 3.4 CA & TLS Handling
16. Create `ca.rs` module: CA generation (self-signed root + intermediate), per-host server certificate signing, storage (in-memory + optional persistent in `~/.config/eggsec/ca` or user-specified dir), fingerprint calculation (SHA256), loading of user-provided CA/cert/key.
17. Integrate CA with rustls for server-side TLS interception.
18. Implement clear warnings when generating new CA; instructions for browser/OS trust store installation (Firefox, Chrome, system keychain, Android/iOS notes).
19. Support `--insecure-tls` reuse for upstream testing scenarios.

### 3.5 MITM Proxy Server Core (HTTP + HTTPS)
20. Create `server.rs` / `mitm.rs` with async listener (tokio + hyper or equivalent).
21. Implement HTTP request handling and forwarding to upstream target.
22. Implement HTTPS MITM: SNI parsing, dynamic certificate generation/signing per host, TLS termination, request/response capture.
23. Basic flow capture: method, URL (redacted host where appropriate), headers (map), body (truncated/redacted with size limit), response status/headers/body, duration, size.
24. Support configurable listen address/port, upstream proxy chaining (reuse existing proxy pool primitives where sensible), graceful shutdown, signal handling.
25. Implement basic concurrent connection limits and per-flow byte budgets.

### 3.6 Flow Interceptor & Logging Basics
26. Create `interceptor.rs` with flow storage (in-memory Vec + optional JSONL file append), redaction application, and basic `InterceptRule` matching stub (host/path/method patterns for Phase 1).
27. Wire flow capture into the server request/response lifecycle.
28. Populate `WebProxySessionReport` with listen addr, CA fingerprint, flows array, policy decisions, budgets, dry_run flag, manifest_matched, actions_performed audit trail.

### 3.7 CLI Surface
29. Add `proxy` command group (or top-level `intercept`) in `eggsec-cli` / `commands/` with clap derive.
30. Implement argument parsing: `--listen <addr:port>`, `--ca-dir` / `--generate-ca-if-missing`, `--ca-cert` / `--ca-key` (user-provided), `--dry-run`, `--json`, `-o`, `--max-flows`, `--max-bytes-per-flow`, `--max-duration`, `--max-concurrent`, `--allow-web-proxy`, `--manual-override-reason`, basic `--intercept-rule` (repeatable), `--upstream-proxy`.
31. Create handler `handle_proxy_intercept` that performs full policy evaluation first, then dispatches to dry-run or real server path.
32. Implement human-readable pretty output (status, CA info with install note, flow summary) + structured JSON output.
33. Add prominent lab-use warnings and disclaimers in help text and runtime banners.

### 3.8 Dry-Run & Execution Safety
34. Implement full dry-run path that constructs a complete `WebProxySessionReport` (synthetic flows, CA logic, policy decisions, budgets) without binding any server or performing network activity.
35. Ensure all budgets, manifest/scope matching, and policy decisions are exercised and recorded in dry-run.
36. Add graceful timeout / shutdown simulation even in dry-run.

### 3.9 Testing
37. Unit tests for CA generation/signing/fingerprint, redaction, pretty-print, budget tracking, manifest/scope parsing, bridge roundtrips, dry-run report population, finding generators (mocked).
38. Integration tests for CLI handler + policy paths (mock `EnforcementContext`).
39. Bridge roundtrip tests (`WebProxySessionReport` → `ScanReportData` → JSON/SARIF/HTML conversion).
40. Lab smoke tests using dockerized targets (httpbin, dvwa, or simple echo server) with known traffic patterns; validate non-interactive capture, HTTPS with generated CA, JSON output, and bridge.
41. Dry-run vs real execution parity tests (structure, policy records, redaction behavior).

### 3.10 Documentation, Architecture & Examples
42. Create / flesh out `docs/WEB_PROXY.md` with Phase 1 content (quick start, safety warnings, CA generation & browser/OS install instructions for major platforms, CLI reference, flow report structure, troubleshooting, limitations).
43. Update root `README.md` — add `eggsec proxy intercept` to lab defense commands table and quick command reference with examples.
44. Update `architecture/proxy.md` or create `architecture/web_proxy.md` stub with Phase 1 architecture notes (modeled on wireless/mobile/database_pentest).
45. Update `architecture/defense_lab.md` with dedicated loadouts subsection if needed.
46. Update `SAFETY.md` with new Web Proxy / Traffic Interception section.
47. Update `CAPABILITIES.md` and `feature_matrix.md`.
48. Create `examples/lab-proxy-manifest.toml` (or scope examples) and basic non-interactive workflow script/example.
49. Update `AGENTS.md` and create `crates/eggsec/src/proxy/AGENTS.override.md` (or web_proxy subdir) with Phase 1 notes.
50. Add Phase 1 status notes and cross-references throughout relevant files.

---

## 4. Recommended Implementation Order (Lowest Risk First)

1. **Types + CA module + Dry-Run Skeleton + Redaction** (safest starting point — no server binding yet; validates report shape and policy early)
2. **Policy / EnforcementContext wiring + `--allow-web-proxy`** (critical safety foundation — do this before real server code)
3. **CLI argument parsing + handler skeleton** (with dry-run path exercising policy + report)
4. **Bridge implementation** (`to_scan_report_data_proxy` + categories — so reporting works from the first usable build)
5. **CA loading + generation + rustls integration** (core security primitive)
6. **Basic HTTP server + flow capture** (forward proxy first)
7. **HTTPS MITM + dynamic cert signing** (add TLS interception)
8. **Upstream chaining + basic concurrent limits / budgets**
9. **Full CLI polish, human output, warnings, provenance prompt**
10. **Tests (unit → integration → policy mocks → lab smoke with docker targets)**
11. **Documentation, CA install guide, examples & AGENTS updates** (continuous but finalize last)

This order allows early validation of the safety model, dry-run experience, and reporting bridge before investing heavily in the live MITM server.

---

## 5. Success Criteria (Measurable)

- `cargo build --release -p eggsec-cli --features web-proxy` succeeds cleanly (and TUI build succeeds).
- `cargo test --features web-proxy` passes (unit + integration + bridge roundtrips).
- `eggsec proxy intercept --listen 127.0.0.1:8080 --dry-run --json -o /tmp/dry.json` produces valid, complete `WebProxySessionReport` JSON with correct structure, synthetic flows, policy decisions, CA fingerprint, and new categories.
- Real lab smoke test (e.g. `curl` or browser configured to use proxy → docker httpbin/dvwa target) produces expected flows in JSON with redaction where applicable, audit trail, and correct bridge output.
- `eggsec report convert` on Phase 1 JSON output works and includes new `proxy-intercept-*` / `web-traffic-*` findings.
- All safety controls function as designed: feature gate, policy decision record, scope/manifest enforcement (or clear warning), `--allow-web-proxy` override + provenance prompt, dry-run bypass, budgets respected, redaction applied.
- HTTPS interception works end-to-end with generated CA (browser trusts after install per docs).
- No regressions in existing features (web fuzzing, WAF, existing proxy pool under stress-testing, mobile-dynamic `--proxy` usage, TUI, db-pentest, etc.).
- Documentation (`docs/WEB_PROXY.md` + CA install guide) is accurate, usable, and reviewed for Phase 1 scope.
- Phase 2 handoff plan draft is ready.

---

## 6. Risks & Mitigations Specific to Phase 1

| Risk                                      | Likelihood | Impact     | Mitigation Strategy                                                                 |
|-------------------------------------------|------------|------------|-------------------------------------------------------------------------------------|
| Proxy engine / dependency complexity      | Medium     | High       | Start with direct hyper + rustls; evaluate hudsucker only if needed; keep surface minimal | 
| CA generation / trust store issues        | Medium     | High       | Clear documented install steps for major platforms; support user-provided CA; warnings in output | 
| Policy model integration complexity       | Medium     | High       | Coordinate closely with core policy owners early; implement in small increments; heavy mocks | 
| Incomplete safety surface (dry-run bypass)| Low        | Very High  | Dry-run must exercise full policy + manifest + report path; mandatory review gate before live server | 
| HTTPS MITM edge cases (SNI, ALPN, pinning)| Medium     | Medium     | Focus on basic happy path first; document limitations; test with common clients (curl, browsers) | 
| Redaction effectiveness / false negatives | Medium     | Medium     | Start with conservative default patterns; make configurable; test with known sensitive payloads | 
| Lab test environment instability          | Medium     | Medium     | Use existing docker-compose testing profile; provide clear one-command setup in docs | 
| Documentation & CA guide lag              | High       | Medium     | Treat docs + CA install guide as parallel workstream from day one; assign owner | 

---

## 7. Dependencies & Coordination Points

- Core policy / EnforcementContext team (for risk tier, capability flag, and evaluation changes)
- Output / reporting team (for bridge integration and `ScanReportData` compatibility)
- CLI / commands team (for clap patterns and handler conventions)
- TUI team (for future Phase 2 tab wiring — coordination only in Phase 1)
- Testing / DevEx (for docker-compose test target fixtures and smoke scripts)
- Documentation owner (for `WEB_PROXY.md`, CA install guide, and cross-updates)
- Proxy / networking specialists (for hyper/rustls or hudsucker evaluation)

All changes must go through normal review process. Large refactors should be broken into smaller PRs where possible. Coordinate early on policy and reporting to avoid late blockers.

---

## 8. Phase 1 Handoff Checklist (Before Merging to Main)

- [ ] All numbered tasks in Section 3 completed or explicitly deferred with notes
- [ ] `cargo test --features web-proxy` green (including lab smoke tests with docker targets)
- [ ] Dry-run and real non-interactive execution produce consistent, high-quality `WebProxySessionReport` output
- [ ] All safety gates validated (feature, policy decision record, scope/manifest, override, provenance prompt, dry-run, redaction, budgets)
- [ ] Reporting bridge works end-to-end (`report convert` produces correct categories)
- [ ] HTTPS MITM works with generated CA + documented install steps succeed on test browser/OS
- [ ] Documentation updated and reviewed (`docs/WEB_PROXY.md` with CA guide, README, SAFETY.md, architecture, CAPABILITIES, examples, AGENTS)
- [ ] Examples committed (`examples/lab-proxy-manifest.toml` or scope examples + basic workflow)
- [ ] AGENTS files updated
- [ ] No regressions on main branch features (web, proxy pool, mobile, db-pentest, TUI, etc.)
- [ ] Phase 2 handoff plan draft created and committed
- [ ] Short Phase 1 closeout note added to this file or a new closeout document

---

## 9. Next Steps After Phase 1

1. Merge Phase 1 to main (after checklist complete and review).
2. Create and commit `plans/interactive-web-proxy-phase2-interactive-tui-handoff-plan.md` (detailed breakdown for TUI + manipulation).
3. Begin Phase 2 on the feature branch or a new sub-branch when prioritized.
4. Gather early user/tester feedback from Phase 1 lab usage (non-interactive capture + CA setup).
5. Iterate on finding quality, redaction effectiveness, and CLI UX based on real usage.
6. Plan TUI tab architecture alignment meeting with TUI team.

---

## 10. References

- Parent Design: `plans/interactive-web-proxy-loadout-design-plan.md`
- High-Level Roadmap: `plans/interactive-web-proxy-implementation-roadmap.md`
- Precedent Phase 1 Plans: `plans/database-pentesting-phase1-foundation-handoff-plan.md`, `plans/mobile-dynamic-phase1-implementation-handoff-plan.md`, wireless phase documents
- Architecture: `architecture/defense_lab.md`, `architecture/proxy.md` (existing), `architecture/web_proxy.md` (new stub)
- Safety: `docs/SAFETY.md`
- Current CLI patterns: `crates/eggsec/src/cli/`, `crates/eggsec/src/commands/`
- Reporting: `crates/eggsec/src/output/`, `report convert` bridge pattern
- Existing proxy module: `crates/eggsec/src/proxy/`
- TUI architecture: recent 10-phase updates in `crates/eggsec-tui/`

---

**End of Phase 1 Foundation Handoff Plan**

This document is the execution blueprint for Phase 1. Implement tasks in the recommended lowest-risk order, maintain close coordination on policy and reporting, and treat safety/validation as non-negotiable gates. Once complete, this loadout will have delivered its first meaningful defensive capability (safely gated traffic interception and logging) while preserving Eggsec’s high standards for safety, quality, and defense-lab focus.

## Phase 0 Closeout Note (Completed 2026-06-12)

**Phase 0 Deliverables Completed**:
- Detailed design plan created and committed: `plans/interactive-web-proxy-loadout-design-plan.md`
- High-level implementation roadmap created and committed: `plans/interactive-web-proxy-implementation-roadmap.md`
- Feature branch created: `feature/interactive-web-proxy-loadout`
- All parent planning artifacts aligned with established loadout patterns (db-pentest, mobile-dynamic, wireless)

Phase 0 complete. Ready for Phase 1 execution on the feature branch.

## Phase 1 Closeout Note (Completed 2026-06-12)

**Phase 1 Deliverables Completed**:
- Feature flag `web-proxy` added to eggsec, eggsec-cli, eggsec-tui Cargo.toml files + included in `full`
- `OperationRisk::TrafficInterception` added to policy system with `ExecutionPolicy::allow_traffic_interception` enforcement
- `Capability::TrafficInterception` added (not baseline-allowed)
- `ConfirmationClass::TrafficInterception` + `ManualOverride::allow_web_proxy` + global `--allow-web-proxy` CLI flag
- Core types: `WebProxySessionReport`, `ProxyFlow`, `BudgetUsage`, `RedactionPattern` (`proxy/intercept/types.rs`)
- Bridge: `to_scan_report_data_proxy()` with `proxy-intercept-*` / `web-traffic-*` categories (`proxy/intercept/bridge.rs`)
- Auto-bridge wired in `commands/handlers/report.rs` via `try_bridge_defense_lab()` helper
- CLI: `eggsec proxy-intercept` command with full argument set (`cli/web_proxy.rs`)
- Handler: `handle_proxy_intercept` with policy evaluation + dry-run path (`commands/handlers/web_proxy.rs`)
- Dry-run produces complete `WebProxySessionReport` with synthetic flows, budget tracking, zero network activity
- Documentation: `docs/WEB_PROXY.md`, `architecture/web_proxy.md`, updated `architecture/proxy.md`, `architecture/defense_lab.md`, `README.md`, `AGENTS.md`, `proxy/AGENTS.override.md`

**Verification**:
- `cargo test --lib -p eggsec --features web-proxy` — 1553 tests pass
- `cargo test --test enforcement_tests -p eggsec --features web-proxy` — 48 enforcement tests pass
- `cargo clippy --lib -p eggsec --features web-proxy` — no new warnings (5 pre-existing)
- Dry-run smoke test: `eggsec proxy-intercept --listen 127.0.0.1:8080 --dry-run --json` produces valid JSON
- Report convert bridge: `eggsec report convert <dry.json> --format json` produces correct `proxy-intercept-flow` and `web-traffic-summary` findings

**Checklist Status**:
- [x] All numbered tasks in Section 3 completed (dry-run path exercises 100% of report, policy, bridge)
- [x] `cargo test --features web-proxy` green
- [x] Dry-run produces consistent, high-quality `WebProxySessionReport` output
- [x] Safety gates validated (feature flag, policy decision, --allow-web-proxy, dry-run bypass)
- [x] Reporting bridge works end-to-end (`report convert` produces correct categories)
- [x] Documentation updated and reviewed
- [x] No regressions on main branch features
- [ ] Real MITM interception (deferred to Phase 2)
- [ ] Phase 2 handoff plan (to be created)

**Key Decisions During Implementation**:
- Created separate `cli/web_proxy.rs` module rather than modifying stress-testing-gated `ProxyCommand`
- Added `Commands::ProxyIntercept` as top-level command (not subcommand of existing `Proxy`)
- Refactored `report.rs` auto-bridge to use `try_bridge_defense_lab()` helper to avoid cfg nesting issues
- Dry-run handler generates synthetic flows inline (no separate dry_run.rs module needed for Phase 1)

Phase 1 complete. Ready for Phase 2 (real MITM interception + interactive TUI).
