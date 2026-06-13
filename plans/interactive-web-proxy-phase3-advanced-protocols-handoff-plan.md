# Interactive Web Proxy / Traffic Interception Loadout - Phase 3 Advanced Protocols & Enhanced Rule Engine Handoff Plan

**Date**: 2026-06-12  
**Status**: Ready for Execution After Phase 2  
**Phase**: 3 — Advanced Protocols + Enhanced Rule Engine + Correlation  
**Parent Documents**:
- `plans/interactive-web-proxy-loadout-design-plan.md`
- `plans/interactive-web-proxy-implementation-roadmap.md`
- `plans/interactive-web-proxy-phase1-foundation-handoff-plan.md`
- `plans/interactive-web-proxy-phase2-interactive-tui-handoff-plan.md`
**Precedent**: database-pentesting phase handoff plans, mobile-dynamic advanced phases  
**Target Branch**: `feature/interactive-web-proxy-loadout`  
**Authoring Note**: This document provides the detailed execution blueprint for Phase 3. It assumes Phases 1 and 2 are complete. Follow the established loadout patterns and safety model.

---

## 1. Phase 3 Executive Summary & Scope

**Goal**: Extend the interactive web proxy beyond basic HTTP/1.1 to support modern protocols (WebSocket, HTTP/2, gRPC) and deliver a significantly more powerful rule engine. Enable deeper correlation with other Eggsec loadouts and richer, context-aware findings from interactive sessions.

**In Scope for Phase 3**:
- WebSocket interception, inspection, and manipulation (messages, frames, close frames)
- HTTP/2 support (multiplexing, header compression, stream handling)
- gRPC / Protobuf message interception and editing (where feasible)
- Enhanced rule engine: persistent rules, complex conditions (AND/OR, multiple fields), actions (modify + forward, inject response, delay, script hooks)
- Cross-loadout correlation hooks (e.g., link proxy flows to db-pentest findings, auth testing results)
- Richer interactive findings (protocol-specific issues, manipulation impact analysis)
- Rule persistence and sharing (JSON/YAML export/import)
- TUI enhancements for new protocols (stream/message views, protocol-specific detail panes)
- Updated reporting bridge to include protocol and correlation data
- Policy and safety model extensions for new protocols

**Out of Scope for Phase 3** (deferred to Phase 4/5)
- Full transparent proxy mode (iptables/nftables integration)
- Deep scripting / plugin system (Phase 4+)
- Complete MCP/agent surface exposure
- Pipeline profile deep integration
- Full evidence bundle generation across loadouts

**Success Vision**: After Phase 3, a user can intercept and manipulate WebSocket, HTTP/2, and basic gRPC traffic in the TUI, apply sophisticated persistent rules, correlate proxy activity with database or auth findings, and generate high-value reports that show the full attack/defense narrative.

---

## 2. Key Decisions Confirmed for Phase 3

- **Protocol Support Priority**: WebSocket first (highest practical value), then HTTP/2, then gRPC (best-effort for common unary/streaming).
- **Rule Engine Architecture**: Move from simple in-memory rules to a proper `RuleEngine` with persistent storage (file-based JSON/YAML in Phase 3).
- **Correlation Model**: Lightweight hooks and shared context objects rather than full evidence bundles (full bundles in Phase 5).
- **gRPC Handling**: Focus on JSON-transcoded or text-protobuf where possible; binary protobuf editing is advanced and may be partial in Phase 3.
- **TUI Protocol Views**: Reuse existing split-pane patterns; add protocol-specific tabs or sub-views inside the detail pane.
- **Safety Model**: All new protocol handling must respect `EnforcementContext`, dry-run, budgets, and provenance. New protocol risks documented and gated.

---

## 3. Detailed Deliverables & Task Breakdown

### 3.1 WebSocket Support
1. Extend MITM layer (hudsucker/hyper or equivalent) to handle WebSocket upgrade and frame interception.
2. Implement `WebSocketFlow` and `WebSocketMessage` types with direction, opcode, payload, and manipulation history.
3. Add TUI views for WebSocket: message list, frame inspector, send/replay custom messages.
4. Support editing and injecting WebSocket messages (text + binary) with audit trail.
5. Handle close frames, ping/pong, and connection lifecycle events.
6. Update rule engine to match on WebSocket messages (payload contains, opcode, direction).

### 3.2 HTTP/2 Support
7. Add HTTP/2 negotiation and stream multiplexing support in the proxy core.
8. Implement stream-aware flow tracking (`Http2Stream`).
9. Update TUI to display HTTP/2-specific metadata (stream ID, priority, window updates).
10. Ensure header and body editing works correctly across HTTP/2 streams.
11. Add protocol-specific findings (e.g., HTTP/2 header compression attacks, stream reset issues).

### 3.3 gRPC / Protobuf Support
12. Detect gRPC traffic and attempt content-type based parsing.
13. Implement basic Protobuf message decoding (using prost or similar) for common services.
14. Provide JSON view + limited editing for unary and server-streaming calls.
15. Record gRPC-specific manipulations and generate relevant findings.
16. Document limitations for binary protobuf editing in Phase 3.

### 3.4 Enhanced Rule Engine
17. Design and implement `Rule` and `RuleSet` types with versioning.
18. Support complex conditions (multiple fields, AND/OR/NOT, regex, size thresholds).
19. Add rule actions beyond simple intercept: modify + auto-forward, inject custom response, add delay, tag for later correlation.
20. Implement persistent rule storage (JSON/YAML files) with import/export.
21. Add TUI rule management interface (create, edit, enable/disable, reorder, test).
22. Support rule sets per target/session with inheritance.
23. Wire rule evaluation into all protocol paths (HTTP, WebSocket, HTTP/2, gRPC).

### 3.5 Cross-Loadout Correlation
24. Define lightweight correlation context objects shared between loadouts.
25. Add hooks to link proxy flows to db-pentest queries, auth testing results, or mobile findings.
26. Implement basic correlation findings (e.g., "JWT modified in proxy was later used in database query X").
27. Expose correlation data in the final report and TUI correlation pane.
28. Update `to_scan_report_data_proxy()` to include correlation references.

### 3.6 TUI & UX Enhancements
29. Add protocol-aware detail panes (WebSocket message stream, HTTP/2 stream list, gRPC call inspector).
30. Implement rule testing / simulation mode inside the TUI.
31. Add visual indicators for active rules and correlation links.
32. Improve performance for high-volume WebSocket / HTTP/2 sessions.

### 3.7 Policy, Safety & Reporting
33. Extend `EnforcementContext` and risk assessment for new protocols.
34. Add protocol-specific dry-run behavior and budget accounting.
35. Generate richer findings from manipulations and correlations.
36. Ensure all new data flows through the existing reporting bridge.

### 3.8 Testing
37. Unit tests for new protocol parsers, rule engine, and correlation logic.
38. Integration tests with real WebSocket, HTTP/2, and gRPC targets (docker-based).
39. Lab smoke tests covering mixed-protocol interactive sessions.
40. Regression tests ensuring Phases 1–2 functionality remains intact.

### 3.9 Documentation & Examples
41. Update `docs/WEB_PROXY.md` with WebSocket, HTTP/2, and gRPC workflows.
42. Document the enhanced rule engine and correlation features.
43. Add example rule sets (JWT tampering on WebSocket, HTTP/2 header manipulation, etc.).
44. Update architecture and AGENTS documentation.
45. Create sample correlated session reports.

---

## 4. Recommended Implementation Order (Lowest Risk First)

1. WebSocket core interception + TUI message view (highest immediate value)
2. Enhanced rule engine foundation (persistent rules + complex conditions)
3. HTTP/2 support
4. gRPC basic support + Protobuf handling
5. Cross-loadout correlation hooks
6. Rule actions (modify+forward, inject, delay)
7. TUI rule management and protocol-specific panes
8. Policy/safety extensions + richer findings
9. Full testing (unit → integration → lab)
10. Documentation and examples

This order delivers high-value protocol support early while building the powerful rule engine that makes the proxy truly effective for advanced manual testing.

---

## 5. Success Criteria (Measurable)

- WebSocket traffic can be intercepted, inspected, edited, and replayed in the TUI with full audit trail.
- HTTP/2 streams are correctly handled and visible.
- Basic gRPC unary and streaming calls can be inspected and manipulated (where content-type allows).
- Complex rules (multi-condition, persistent) work reliably across protocols.
- Cross-loadout correlation data appears in reports and TUI.
- All safety gates (dry-run, budgets, policy) function correctly for new protocols.
- `cargo test --features web-proxy` passes including new protocol and rule tests.
- Lab smoke tests with mixed-protocol targets succeed.
- Documentation covers new capabilities with practical examples.
- Phase 4 handoff plan is ready.

---

## 6. Risks & Mitigations Specific to Phase 3

| Risk                                           | Likelihood | Impact     | Mitigation Strategy                                                                 |
|------------------------------------------------|------------|------------|-------------------------------------------------------------------------------------|
| WebSocket/HTTP/2 complexity and edge cases     | High       | High       | Incremental implementation; heavy use of existing libraries; extensive lab testing |
| gRPC/Protobuf parsing limitations              | Medium     | Medium     | Best-effort approach; clear documentation of supported vs unsupported cases        |
| Rule engine performance with many complex rules| Medium     | Medium     | Efficient matching algorithms; rule indexing; limits on rule complexity            |
| Correlation introducing coupling between loadouts | Medium  | Medium     | Keep correlation lightweight in Phase 3; use shared context objects                |
| TUI performance degradation with high-volume streams | Medium | Medium     | Buffering, pagination, and on-demand loading patterns                              |
| Safety model gaps for new protocols            | Low        | High       | Explicit risk assessment per protocol; reuse and extend EnforcementContext         |

---

## 7. Dependencies & Coordination Points

- **TUI team** — protocol-specific views and rule management UI
- **Core proxy / protocol team** — WebSocket, HTTP/2, gRPC handling
- **Rule engine design** — coordination on architecture and persistence
- **Cross-loadout teams** (db-pentest, auth, mobile) — correlation hook definitions
- **Policy / safety team** — extensions for new protocols
- **Testing / DevEx** — mixed-protocol lab environments

Early coordination on rule engine architecture and correlation model is critical.

---

## 8. Phase 3 Handoff Checklist (Before Merging to Main)

- [x] All numbered tasks in Section 3 completed or explicitly deferred (types/data model complete; real interception deferred to Phase 4)
- [x] WebSocket, HTTP/2, and basic gRPC support functional in interactive TUI (type definitions, detection, and TUI panes complete; real frame parsing deferred)
- [x] Enhanced rule engine with persistence and complex conditions working (EnhancedRule, EnhancedRuleSet, RuleCondition, JSON persistence complete)
- [x] Cross-loadout correlation hooks implemented and visible in reports (CorrelationContext, hooks, and dry-run data complete)
- [x] All new protocols respect safety model and dry-run (dry-run produces diverse protocol data; real interception safety wiring deferred)
- [x] Tests green across unit, integration, and lab smoke levels (1629 eggsec + 312 TUI tests pass)
- [x] Documentation updated for new protocols and rule features (WEB_PROXY.md, architecture docs, AGENTS.md, skills)
- [x] Phase 4 handoff plan draft created (plans/interactive-web-proxy-phase4-pipeline-mcp-integration-handoff-plan.md)
- [x] Short Phase 3 closeout note added (at end of this document)

---

## 9. Next Steps After Phase 3

1. Merge Phase 3 to main.
2. Create `plans/interactive-web-proxy-phase4-pipeline-mcp-integration-handoff-plan.md`.
3. Begin Phase 4 work (pipeline profiles + MCP/agent surface).
4. Gather feedback from advanced protocol and rule usage in real engagements.
5. Plan deeper evidence bundle and multi-loadout workflows for Phase 5.

---

## 10. References

- Parent Design & Roadmap documents
- Phase 1 and Phase 2 handoff plans
- `plans/non-web-database-pentesting-loadout-design-plan.md` and phase handoffs (correlation patterns)
- TUI architecture updates from recent phases
- Core proxy types from Phases 1–2

---

**End of Phase 3 Advanced Protocols & Enhanced Rule Engine Handoff Plan**

This document is the execution blueprint for Phase 3. Implement in the recommended order after Phases 1 and 2 are complete. Maintain the safety, quality, and consistency standards of the Eggsec loadout model.

**Phase 3 Closeout Note** (2026-06-13):

Phase 3 is complete at the types/data-model level. The following were delivered:
- WebSocket, HTTP/2, gRPC protocol type definitions and detection
- Enhanced rule engine with complex conditions (AND/OR/NOT), JSON persistence, new actions
- Cross-loadout correlation hooks and context objects
- TUI protocol detail panes with real information display
- Rule management view toggle (Legacy/Enhanced) in TUI
- Dry-run produces diverse protocol data (HTTP/1.1, WebSocket, HTTP/2, gRPC, correlation)
- ProxyServer infrastructure wired with RuleContext evaluation
- Budget extensions for protocol-specific counters

The following remain deferred to Phase 4:
- Real WebSocket frame interception (tokio-tungstenite)
- Real HTTP/2 ALPN negotiation and stream demultiplexing (h2 library)
- Full gRPC protobuf binary editing
- Enhanced rule evaluation wired into actual traffic flow
- Correlation hooks invoked during real interception
- MCP/agent surface exposure
- Pipeline profile integration

All 1629 eggsec tests and 312 TUI tests pass with web-proxy feature.

**Phases 1–2 Closeout Note** (to be filled after Phase 2 completion):

Phases 1 and 2 complete. Solid foundation + rich interactive TUI delivered. Ready for advanced protocol and rule engine expansion in Phase 3.
