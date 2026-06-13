# Interactive Web Proxy / Traffic Interception Loadout - Phase 4 Pipeline Profile Integration & MCP/Agent Surface Handoff Plan

**Date**: 2026-06-12  
**Status**: Ready for Execution After Phase 3  
**Phase**: 4 — Pipeline Profile Integration + MCP/Agent Surface + Advanced Features  
**Parent Documents**:
- `plans/interactive-web-proxy-loadout-design-plan.md`
- `plans/interactive-web-proxy-implementation-roadmap.md`
- Phase 1, 2, and 3 handoff plans
**Precedent**: database-pentesting Phase 4/5 patterns, mobile-dynamic advanced integration phases  
**Target Branch**: `feature/interactive-web-proxy-loadout`  
**Authoring Note**: This document provides the detailed execution blueprint for Phase 4. It assumes Phases 1–3 are complete. Focus on integration depth and agent-facing capabilities while preserving the standalone defense-lab experience.

---

## 1. Phase 4 Executive Summary & Scope

**Goal**: Deeply integrate the interactive web proxy into Eggsec’s broader pipeline and agent ecosystem. Enable the proxy to participate in orchestrated workflows via Pipeline Profiles, expose a clean MCP/agent surface for autonomous or semi-autonomous usage, and deliver advanced capabilities that make the proxy a first-class citizen across the tool.

**In Scope for Phase 4**:
- Full Pipeline Profile integration (proxy as a stage in multi-loadout pipelines)
- MCP / agent surface exposure (tools, resources, prompts for the interactive proxy)
- Advanced features: transparent proxy mode (best-effort), request/response scripting hooks, advanced manipulation templates
- Evidence bundle generation and consumption (shared with other loadouts)
- Session templating and reusable attack/defense playbooks
- Enhanced correlation and narrative generation across loadouts
- TUI and CLI improvements for pipeline and agent-aware usage
- Policy and safety model extensions for agent-driven and pipeline execution

**Out of Scope for Phase 4** (deferred to Phase 5)
- Complete multi-loadout evidence bundle orchestration
- Full visual narrative and timeline views
- Release hardening, final documentation polish, and marketing examples
- Transparent proxy on all platforms (Linux focus in Phase 4)

**Success Vision**: After Phase 4, the web proxy can be invoked as part of a Pipeline Profile, controlled or observed by an MCP-compatible agent, generate and consume evidence bundles, and support scripted or templated advanced manipulation workflows — all while remaining fully usable as a standalone interactive tool.

---

## 2. Key Decisions Confirmed for Phase 4

- **Pipeline Integration Approach**: Proxy loadout exposes `PipelineStage` interface and configuration schema. Profiles can declare proxy stages with parameters (target, ruleset, budgets, interactive vs headless).
- **MCP Surface Scope**: Read-only observation + controlled manipulation tools. Full interactive TUI remains human-only; agent surface focuses on actionable operations.
- **Transparent Proxy**: Linux nftables/iptables best-effort support in Phase 4. Other platforms and full transparency deferred.
- **Scripting Hooks**: Lightweight Lua or Rust-script hooks for request/response transformation (sandboxed). Full plugin system in Phase 5+.
- **Evidence Bundles**: Proxy can produce and consume bundles. Focus on flow + manipulation + correlation data.
- **Safety Model**: Agent and pipeline execution must still go through `EnforcementContext` with explicit provenance and dry-run support.

---

## 3. Detailed Deliverables & Task Breakdown

### 3.1 Pipeline Profile Integration
1. Define `ProxyPipelineStage` configuration schema (target, CA, rules, budgets, mode).
2. Implement `PipelineStage` trait for the proxy loadout.
3. Add headless/non-interactive execution mode for pipeline use.
4. Support passing rule sets and session templates into pipeline stages.
5. Implement stage output that feeds into downstream loadouts or evidence bundles.
6. Add pipeline-aware logging and progress reporting.
7. Update CLI to support `eggsec pipeline run` with proxy stages.

### 3.2 MCP / Agent Surface Exposure
8. Design and implement MCP tools for the proxy:
   - `proxy_start_session`
   - `proxy_list_flows`
   - `proxy_get_flow`
   - `proxy_apply_manipulation`
   - `proxy_forward_flow`
   - `proxy_drop_flow`
   - `proxy_load_ruleset`
   - `proxy_export_session`
9. Expose key resources (active sessions, current rules, CA info).
10. Create agent-friendly prompts and guidance for proxy usage.
11. Ensure all MCP operations respect `EnforcementContext` and dry-run.
12. Add provenance tracking for agent-initiated actions.
13. Implement rate limiting and safety guardrails for agent usage.

### 3.3 Transparent Proxy Mode (Best-Effort)
14. Implement Linux transparent proxy support (nftables/iptables + TProxy).
15. Add automatic CA installation guidance for transparent mode.
16. Handle edge cases (localhost, Docker networking, IPv6).
17. Document limitations and security considerations.
18. Provide fallback to explicit proxy mode when transparent setup fails.

### 3.4 Advanced Manipulation & Templating
19. Implement manipulation templates (JWT tampering, header injection patterns, common auth bypasses).
20. Add request/response transformation scripting hooks (sandboxed Lua or Rust).
21. Support reusable session templates and attack playbooks.
22. Implement diff and impact analysis between original and manipulated flows.
23. Add bulk manipulation and replay capabilities with safety confirmations.

### 3.5 Evidence Bundle Integration
24. Define proxy contribution to evidence bundles (flows, manipulations, correlations).
25. Implement bundle production and consumption for the proxy.
26. Enable correlation across proxy + db-pentest + auth + mobile bundles.
27. Add bundle-aware reporting and narrative generation hooks.

### 3.6 TUI & CLI Enhancements
28. Add pipeline status and agent connection indicators in TUI.
29. Implement evidence bundle browser/viewer in TUI.
30. Add CLI flags for pipeline and MCP mode.
31. Improve headless output for pipeline consumption.

### 3.7 Policy, Safety & Governance
32. Extend `EnforcementContext` for pipeline and agent execution contexts.
33. Add provenance and actor tracking for agent vs human actions.
34. Implement budget and risk controls specific to automated usage.
35. Update policy decision recording for pipeline/agent flows.

### 3.8 Testing
36. Unit and integration tests for pipeline stage, MCP tools, and transparent proxy.
37. Lab tests with Pipeline Profile execution involving proxy + other loadouts.
38. Agent simulation tests (mock MCP client exercising proxy tools).
39. Security and safety tests for scripting hooks and transparent mode.
40. Full regression suite.

### 3.9 Documentation & Examples
41. Update `docs/WEB_PROXY.md` with pipeline, MCP, transparent mode, and templating sections.
42. Add Pipeline Profile examples that include the proxy.
43. Document MCP tool usage and agent best practices.
44. Create example evidence bundles and manipulation templates.
45. Update architecture, AGENTS, and governance documentation.

---

## 4. Recommended Implementation Order (Lowest Risk First)

1. Pipeline Profile integration (stage trait + schema + headless mode)
2. MCP tool surface (core observation and manipulation tools)
3. Evidence bundle production/consumption
4. Manipulation templates and basic scripting hooks
5. Transparent proxy (Linux)
6. TUI/CLI enhancements for pipeline and agent awareness
7. Policy/safety extensions for automated execution
8. Full testing (pipeline + agent simulation + lab)
9. Documentation and examples

This order prioritizes high-leverage integration points (pipeline + MCP) early while delivering advanced features in a controlled sequence.

---

## 5. Success Criteria (Measurable)

- Proxy can be used as a stage inside a Pipeline Profile with other loadouts.
- MCP-compatible agents can start sessions, inspect flows, apply manipulations, and export results safely.
- Transparent proxy mode works on Linux with documented setup.
- Manipulation templates and basic scripting hooks are functional.
- Evidence bundles are produced and can be correlated with other loadouts.
- All safety gates function correctly in pipeline and agent contexts.
- Tests pass (unit + integration + pipeline lab + agent simulation).
- Documentation covers pipeline, MCP, transparent, and templating usage.
- Phase 5 handoff plan is ready.

---

## 6. Risks & Mitigations Specific to Phase 4

| Risk                                              | Likelihood | Impact     | Mitigation Strategy                                                                 |
|---------------------------------------------------|------------|------------|-------------------------------------------------------------------------------------|
| MCP surface introducing new attack surface        | Medium     | High       | Strict safety model enforcement; provenance; rate limits; dry-run by default       |
| Transparent proxy complexity and platform issues  | High       | Medium     | Linux-only in Phase 4; clear documentation; graceful fallback                      |
| Scripting hooks creating safety or performance issues | Medium  | High       | Sandboxing; strict limits; opt-in; extensive testing                               |
| Pipeline integration coupling loadouts too tightly| Medium     | Medium     | Well-defined interfaces and schemas; loose coupling where possible                 |
| Evidence bundle size and complexity               | Medium     | Medium     | Incremental approach; focus on proxy contribution first                            |

---

## 7. Dependencies & Coordination Points

- **Pipeline team** — stage interface and profile execution engine
- **MCP / Agent team** — tool surface design, safety model for agents
- **Evidence bundle team** — bundle format and correlation standards
- **TUI / CLI team** — pipeline and agent indicators, headless output
- **Scripting / extensibility team** — sandboxed hook design
- **Policy / safety team** — extensions for automated and pipeline contexts
- **Testing / DevEx** — pipeline lab environments and agent simulation harness

Close coordination with Pipeline and MCP teams is essential for Phase 4 success.

---

## 8. Phase 4 Handoff Checklist (Before Merging to Main)

- [ ] All numbered tasks in Section 3 completed or explicitly deferred
- [ ] Pipeline Profile integration complete and tested
- [ ] MCP tool surface functional with safety controls
- [ ] Transparent proxy (Linux) working with documentation
- [ ] Manipulation templates and scripting hooks available
- [ ] Evidence bundle support implemented
- [ ] All safety and policy controls extended for pipeline/agent use
- [ ] Tests green across relevant dimensions
- [ ] Documentation updated
- [ ] Phase 5 handoff plan draft created
- [ ] Short Phase 4 closeout note added

---

## 9. Next Steps After Phase 4

1. Merge Phase 4 to main.
2. Create `plans/interactive-web-proxy-phase5-polish-release-handoff-plan.md`.
3. Begin Phase 5 (final polish, full integration, release readiness).
4. Conduct internal and limited external testing of pipeline + MCP capabilities.
5. Prepare final documentation, examples, and release notes.

---

## 10. References

- Parent Design, Roadmap, and prior phase handoff plans
- Database-pentesting and mobile-dynamic Phase 4/5 patterns
- Pipeline Profile and MCP architecture documentation
- Evidence bundle specification

---

**End of Phase 4 Pipeline Profile Integration & MCP/Agent Surface Handoff Plan**

This document is the execution blueprint for Phase 4. Implement after Phases 1–3 are complete. Prioritize integration depth while maintaining the high safety and usability bar of the Eggsec project.

**Phases 1–3 Closeout Note** (to be filled after Phase 3 completion):

Phases 1–3 complete. Foundation, interactive TUI, and advanced protocols/rule engine delivered. Ready for deep pipeline and agent integration in Phase 4.
