# Interactive Manual Web Proxy / Traffic Interception & Manipulation Loadout - High-Level Incremental Implementation Roadmap

**Date**: 2026-06-12  
**Status**: High-Level Roadmap — Ready for Team Alignment & Detailed Planning  
**Branch**: feature/interactive-web-proxy-loadout  
**Related**: `plans/interactive-web-proxy-loadout-design-plan.md` (authoritative detailed design), wireless/mobile/db-pentest loadout plans & phase handoff documents (implementation pattern), `architecture/defense_lab.md`, `crates/eggsec/src/proxy/`, Cargo.toml features, EnforcementContext model, recent TUI architecture pass  
**Purpose**: Provide a clear, incremental, high-level roadmap for implementing the full Interactive Manual Web Proxy / Traffic Interception & Manipulation loadout end-to-end. This complements the detailed design plan by focusing on sequencing, dependencies, parallel tracks, milestones, and risk-managed rollout. Use this to drive issue creation, sprint planning, and phase-specific handoff plans. Follows the exact successful pattern established by the database pentesting, mobile-dynamic, and wireless-active loadouts.

---

## 1. Overall Philosophy & Principles for Implementation

- **Incremental & Gated**: Deliver value early (core HTTP/HTTPS MITM proxy server + flow logging + dry-run + policy gating + basic reporting bridge) while deferring higher-complexity/risk pieces (full interactive TUI editing, WebSocket/HTTP2, advanced rule engine).
- **Follow Proven Pattern**: Mirror the successful wireless-active and mobile-dynamic (and db-pentest) execution model exactly: detailed design plan → high-level roadmap → phase handoff plans → feature branch (already created) → implementation + tests + docs updates → merge + smoke validation.
- **Safety First**: Every phase must preserve or strengthen the existing `EnforcementContext`, scope/manifest integration, dry-run, feature-gating, redaction, and standalone defense-lab model. No shortcuts on policy, warnings, or auditability.
- **Documentation & Testing Parallel**: Docs (`docs/WEB_PROXY.md`), architecture updates, AGENTS.md, examples (CA install, lab workflows), and tests (heavy dry-run + lab traffic fixtures) run in parallel from Phase 1.
- **Reporting Bridge Early**: Ensure `to_scan_report_data_proxy()` works from the first usable phase so intercepted flows and manipulation findings integrate immediately with `report convert`, SARIF, trend analysis, and combined reports (web + proxy + db).
- **Standalone → Integrated**: Start as pure standalone CLI + basic server (like current `auth-test` / `mobile` / `wireless` / `db pentest`). Add rich TUI tab and pipeline profile support only after core MITM + reporting is stable.
- **Measure & Validate**: Each phase ends with explicit success criteria, lab smoke tests (using docker targets like httpbin/dvwa), and regression validation against main. Feature branch `feature/interactive-web-proxy-loadout` already created as the working branch.
- **TUI Leverage**: Heavily reuse the recent 10-phase eggsec-tui architecture (UiAction layer, OverlayController, TabSpec registry, manual-mode preflight/status indicators, enforcement posture, global task strip, semantic styling) for the interactive experience.

**High-Level Timeline Vision** (aggressive but realistic, assuming dedicated focus, mirroring recent June 2026 loadout velocity):
- Phase 0–1: 5–7 weeks (foundations + first usable proxy value)
- Phase 2: 4–6 weeks (interactive core)
- Phase 3: 3–5 weeks
- Phase 4+: Ongoing / as prioritized

---

## 2. High-Level Phased Roadmap

### Phase 0: Planning, Alignment, Setup & Branch Initialization (Completed / 1 week)
**Goal**: Solidify decisions from the design plan, prepare the codebase, and initialize the working branch (already done).

**Key Deliverables**:
- Team review & sign-off on `plans/interactive-web-proxy-loadout-design-plan.md` + this roadmap.
- Final decisions on open questions from the design plan (feature flag name — recommend `web-proxy`; CLI naming — recommend `eggsec proxy intercept`; TUI tab name — recommend "Proxy" or "Intercept"; proxy engine choice — extend existing `proxy/` module vs lightweight hudsucker; lab authorization via scope vs lightweight proxy-manifest; standalone confirmation).
- Create this high-level implementation roadmap (`plans/interactive-web-proxy-implementation-roadmap.md`).
- Create detailed Phase 1 handoff plan (`plans/interactive-web-proxy-phase1-foundation-handoff-plan.md` — next immediate task).
- Feature branch `feature/interactive-web-proxy-loadout` already created and plan committed.
- Update `AGENTS.md` and root planning docs with high-level intent for the new loadout.
- Initial issue tracker breakdown (GitHub issues or internal) mapped to phases and workstreams.

**Parallel Tracks**:
- Docs skeleton: Create `docs/WEB_PROXY.md` stub + update README lab defense commands table placeholder with `eggsec proxy intercept` examples.
- Policy: High-level design for new `TrafficInterception` / `WebProxyManipulation` risk tier + capability flag and proxy-manifest / scope integration.
- Existing proxy module audit: Quick review of `crates/eggsec/src/proxy/` to identify extension points.

**Success Criteria**:
- All open questions from design plan resolved or explicitly deferred with owners.
- Phase 1 detailed handoff plan committed.
- Feature branch ready, CI green on main, and initial plan visible.

**Risks Mitigated**: Ambiguity in scope, approach, or naming. Branch initialization complete.

**Status**: Completed as of 2026-06-12 (this document + detailed design plan + feature branch creation).

### Phase 1: Foundations — Core HTTP/HTTPS MITM Proxy Server + Flow Logging + CLI + Dry-Run + Reporting Bridge (Core Value Delivery)
**Goal**: Deliver the first usable, gated capability: a working MITM proxy server (HTTP + HTTPS with CA) that can capture and log traffic flows, with full safety model, dry-run support, basic CLI, and reporting bridge. This provides immediate defense-lab value for traffic inspection even in non-interactive mode.

**Key Deliverables**:
- Feature flag `web-proxy` in `crates/eggsec/Cargo.toml` (and propagated to `eggsec-cli` and `eggsec-tui`).
- Extend or create structure under `crates/eggsec/src/proxy/` (or new `web_proxy/` / `mitm/` submodule): `mod.rs`, `types.rs` (ProxyFlow, WebProxySessionReport, InterceptRule, ManipulationRecord, BudgetUsage), `ca.rs` (CA generation, signing, fingerprinting with rcgen + rustls), `server.rs` or `mitm.rs` (core async MITM proxy using hyper/hyper-rustls or hudsucker wrapper), `interceptor.rs` (basic flow capture + logging), `utils.rs` (redaction, pretty-print helpers), `bridge.rs` (`to_scan_report_data_proxy()`), `manifest.rs` (optional lightweight proxy-manifest parser or reuse LoadedScope).
- Core types: `ProxyFlow`, `WebProxySessionReport`, `ManipulationRecord`, `WebProxyTarget`, basic `InterceptRule`.
- Basic MITM server: Listen on configurable addr:port, handle HTTP, perform HTTPS MITM with generated or user-provided CA, capture request/response (headers + body with size limits), support upstream chaining.
- CLI: `eggsec proxy intercept` command (or `eggsec mitm intercept`) with `--listen`, `--ca-dir` / `--generate-ca-if-missing`, `--dry-run`, `--json`, `-o`, `--max-flows` / `--max-bytes-per-flow` / `--max-duration`, `--allow-web-proxy`, `--manual-override-reason`, provenance prompt, basic `--intercept-rule` support.
- Full `EnforcementContext` integration + new policy decision records + denial/confirmation paths + redaction engine.
- `to_scan_report_data_proxy()` bridge + new finding categories (e.g., `proxy-intercept-pii-detected`, `web-traffic-weak-header`, `manipulation-sensitive-token`).
- Dry-run path that exercises 100% of report generation, CA logic, and flow skeleton creation without binding a server or sending traffic.
- Unit + integration tests (mocks for flows/connections/CA, manifest/scope validation, finding generators, bridge roundtrips, policy stubs, redaction).
- Lab smoke tests using dockerized targets (httpbin, dvwa, or simple Node/Go echo server) with known traffic patterns; validate non-interactive capture + JSON output + bridge.
- Documentation: Full `docs/WEB_PROXY.md` (Phase 1 content including CA install instructions for major browsers/OS), updates to README (lab defense commands table + quick examples), `architecture/web_proxy.md` or extension of `architecture/proxy.md` / `stress/`, `SAFETY.md` (new Web Proxy / Traffic Interception section), `CAPABILITIES.md`, `feature_matrix.md`, AGENTS.md / AGENTS.override.md in proxy/ or new web_proxy/ dir.
- Examples: `examples/lab-proxy-manifest.toml` (or reuse scope), CA install guide, basic non-interactive workflow script.

**Parallel Tracks**:
- Policy & manifest: Implement and test proxy authorization (scope integration or lightweight manifest) + enforcement.
- CA handling: Secure generation, storage guidance, fingerprinting in reports.
- Redaction patterns: Curate initial high-signal PII/token/secret patterns.
- Smoke infrastructure: Ensure docker-compose testing profile supports easy target spin-up for proxy smoke tests.

**Success Criteria**:
- `cargo build/test/clippy --features web-proxy` green.
- `eggsec proxy intercept --listen 127.0.0.1:8080 --dry-run --json` produces valid, complete `WebProxySessionReport`.
- Real lab smoke test (curl/browser → proxy → target) produces expected flows, redacted sensitive data where applicable, and audit trail.
- `eggsec report convert` on the JSON output works (new proxy-intercept-* categories appear correctly; combined with web reports possible).
- All safety gates (feature, policy, scope/manifest match, provenance, budgets, dry-run, redaction) function as designed with clear user feedback and warnings.
- No regressions in existing features (web fuzzing, WAF, proxy pool under stress-testing, mobile-dynamic --proxy usage, TUI).
- Phase 2 handoff plan drafted.

**Risks Mitigated**: Policy/safety model gaps, incomplete reporting integration, poor early user experience with CA/HTTPS, foundational bugs in MITM server.

**Estimated Effort**: 5–7 weeks (heaviest phase — foundations + first real capability + CA + bridge).

### Phase 2: Interactive TUI + Core Manipulation & Editing (Interactive Value Delivery)
**Goal**: Add the rich interactive manual experience: TUI for live flow inspection, breakpoint/intercept rules, header/body editing, forward/drop/replay with full manipulation audit trail. Move from "passive capture" to "active manual adversarial simulation & defense regression".

**Key Deliverables**:
- TUI integration in `eggsec-tui`: New `TabSpec` "Proxy" or "Intercept" (or integrated into existing Web tab), flow list table view (ID, timestamp, method, host/path, status, size, modified flag), detail pane (pretty-printed headers + body with toggle for raw/hex/JSON/XML), edit modal (header map editor + body textarea with basic validation + before/after diff preview), action handling (forward, drop, replay, pause/resume, global rules editor).
- Interception engine enhancements: Rule matching engine (host/path/method/header/body regex patterns), pause/resume flows, in-memory flow store with TUI subscription, manipulation recorder (immutable `ManipulationRecord` with before/after + reason + timestamp).
- Interactive CLI mode: `eggsec proxy intercept --interactive` launches TUI session; or seamless launch from main TUI.
- Enhanced findings: Auto + manual flagging during interactive session; richer recommendations tied to manipulations.
- Session management: Save/load sessions (JSON), basic export (HAR or custom), baseline vs current diff support for regression workflows.
- CLI/TUI polish: Better help, error messages, preflight indicators (enforcement posture, scope match, CA status, budgets), graceful shutdown, Ctrl-C handling.
- Expanded tests: TUI interaction flows (dry-run heavy), manipulation audit trail integrity, edit validation, session persistence, combined proxy + other scan reports.
- Documentation refresh: Update `WEB_PROXY.md` with interactive workflows, TUI keybindings/screenshots (or ASCII), example manual JWT/header edit for auth testing, WAF live evasion regression example.
- Lab smoke: Full interactive session against lab target with deliberate manual changes and validation of audit trail in report.

**Parallel Tracks**:
- TUI architecture alignment: Ensure full reuse of recent 10-phase patterns (UiAction, OverlayController, TabSpec, manual-mode indicators, small-terminal degraded layouts).
- Performance: Flow buffering, concurrent connection limits, memory bounds for large bodies.
- Finding quality iteration: Incorporate Phase 1 smoke feedback; add manipulation-specific findings.
- Examples & workflows: Complete end-to-end defense-lab proxy regression example (baseline capture → manual edits → harden target/WAF → re-intercept → diff report).

**Success Criteria**:
- TUI tab renders cleanly and is consistent with wireless/mobile/db tabs; basic interactive flow (intercept → edit → forward) works end-to-end in dry-run and lab.
- Manipulation records are complete, immutable, and visible in both TUI and final JSON report.
- `eggsec proxy intercept --interactive` provides excellent UX with enforcement indicators and clear safety feedback.
- Phase 1 smoke tests still pass cleanly after enhancements.
- Documentation is comprehensive for Phase 2 scope; users can perform useful manual interception without external tools.

**Risks Mitigated**: TUI usability debt, incomplete manipulation audit trail, edit validation failures, performance under concurrent load.

**Estimated Effort**: 4–6 weeks.

### Phase 3: Advanced Protocols, Rule Engine, Polish & Deeper Integration
**Goal**: Expand protocol support and add automation/polish so the loadout becomes a mature, production-ready defense-lab capability. Enable more realistic traffic scenarios and tighter integration with the rest of Eggsec.

**Key Deliverables**:
- WebSocket support: Intercept, inspect, and (basic) edit WS frames; upgrade handling.
- HTTP/2 support (partial or via h2 crate integration where practical).
- Simple persistent rule engine: Match-action rules (e.g., "if path contains /login add header X-Eggsec-Test: regression-123"; "if body contains token drop or modify").
- Evidence bundles + correlation: Link proxy flows/manipulations to web fuzz/WAF/db findings (e.g., "manual header edit confirmed vector also detected by fuzzer").
- Polish: Better redaction (configurable patterns + UI), improved pretty-print for more content types, HAR export fidelity, performance optimizations, robust error handling and user guidance in TUI/CLI.
- Pipeline awareness: Document (and light implementation) how proxy sessions can feed into custom CI regression jobs or be chained with other stages; placeholder for future `ScanProfile` extension.
- Expanded documentation & examples: Advanced lab workflows (OAuth/JWT manipulation, WAF evasion live testing, combined web+proxy+db reports), transparent proxy notes (iptables/pf guidance for lab), device CA install (Android/iOS), troubleshooting section.
- Full test matrix: Multiple TLS versions, HTTP/2, WS, concurrent high-volume, large bodies, different browsers/clients.
- AGENTS / architecture / SAFETY / CAPABILITIES updates to reflect mature state.

**Parallel Tracks**:
- MCP/agent consideration: Security review for optional future read-only status or replay `SecurityTool` exposure (decision documented; default remains standalone defense-lab).
- Community / external feedback incorporation (if any from earlier phases).
- Performance & stability hardening.

**Success Criteria**:
- WebSocket and basic HTTP/2 flows are interceptable and appear correctly in reports/TUI.
- Rule engine is usable and produces correct manipulation records.
- Combined proxy + web/db reports demonstrate clear value for regression and correlation.
- TUI/CLI remain stable and responsive under realistic lab load.
- Documentation is complete and high-quality for the scope of Phase 3.

**Risks Mitigated**: Protocol coverage gaps, integration complexity, advanced feature creep.

**Estimated Effort**: 3–5 weeks.

### Phase 4+: Extensibility, gRPC/Advanced Protocols, Long-Term Maintenance & Future Enhancements
**Goal**: Make the loadout extensible and maintainable long-term; add lower-priority capabilities as prioritized by the team.

**Key Deliverables (as prioritized)**:
- gRPC / protobuf framing inspection and basic editing (framed messages, service/method awareness).
- Deeper automation: More sophisticated rule engine, scripting hooks (if desired, behind heavy gating), replay of saved sessions with modifications.
- Transparent proxy mode enhancements + better lab setup automation.
- Better integration with other Eggsec modules (e.g., cloud asset discovery feeding targets, compliance feature mapping proxy findings to controls, mobile-dynamic traffic correlation).
- Ongoing maintenance: TLS/cipher updates, new finding patterns from real usage, performance tuning, dependency updates.
- Optional: MCP/agent tool exposure after formal security review (if business need arises).
- Possible evolution of CLI surface (e.g., `eggsec proxy` as top-level group with `intercept`, `pool`, `health` subcommands).

**Success Criteria**:
- New protocol or rule capabilities can be added with reasonable effort following the established patterns.
- The feature remains stable, well-tested, and aligned with Eggsec safety standards as it grows.
- Long-term technical debt is actively managed.

**Risks Mitigated**: Long-term technical debt, feature stagnation, protocol obsolescence.

---

## 3. Cross-Cutting Concerns & Parallel Workstreams

| Workstream              | Runs In Phases | Key Activities                                                                 | Dependencies                          | Owner(s) Suggested |
|-------------------------|----------------|--------------------------------------------------------------------------------|---------------------------------------|--------------------|
| Policy & Safety         | All            | EnforcementContext extensions, proxy authorization (scope/manifest), risk tier, confirmation UX, redaction engine | Design plan decisions                | Core team         |
| CLI & Commands          | 1+             | Clap integration for `proxy intercept`, handler dispatch, help text, error handling, dry-run | Types + policy                       | CLI team          |
| Proxy / MITM Engine     | 1–3            | CA handling, server core (hyper/rustls or hudsucker), flow capture, interceptor, rule engine | Driver/engine decision             | Proxy / Networking specialists |
| TUI                     | 2–3            | TabSpec implementation, flow list/detail/edit views, action handling, preflight indicators, small-terminal support | Core CLI + policy + types            | TUI team          |
| Reporting & Bridge      | 1+             | `to_scan_report_data_proxy`, finding categories, evidence richness, convert/HAR compatibility, manipulation audit | Types                                | Output team       |
| Documentation           | All (parallel) | WEB_PROXY.md, architecture updates, README, SAFETY, examples, CA install guide, lab workflows | Implementation progress              | Docs + all        |
| Testing & QA            | All            | Unit, integration, lab smoke (docker targets), regression, performance, dry-run coverage, TUI flows | All deliverables                     | QA + all          |
| Examples & Lab Fixtures | 1+             | docker-compose test targets (httpbin, dvwa, etc.), proxy-manifest examples, end-to-end workflow scripts, CA setup | Docker testing profile               | DevEx             |
| AGENTS / Architecture   | All            | AGENTS.override.md, architecture/web_proxy.md or proxy extension, defense_lab updates | Implementation + docs                | Architecture      |
| Performance & Stability | 2+             | Concurrent handling, memory/byte budgets, graceful degradation, shutdown | Engine + TUI                         | All               |

---

## 4. Risk Management Across the Roadmap

- **Highest Risk Phases**: Phase 1 (foundations + safety model + CA + MITM server) and Phase 2 (TUI + interactive editing).
- **Mitigation Strategy**: Heavy use of dry-run + mocks from day one, incremental merges to feature branch, explicit phase handoff plans, lab-only smoke tests with known targets, mandatory review gates before advancing phases, and strict adherence to phased scope.
- **Dependency Risks**: Proxy engine choice (extend vs hudsucker) and lab traffic test infrastructure must be resolved in Phase 0/early Phase 1. TUI architecture alignment critical for Phase 2.
- **Scope Creep**: Strict adherence to phased scope; WebSocket/HTTP2/rule engine/gRPC deferred to Phase 3+ or explicit prioritization. Advanced automation and MCP exposure deferred.
- **Adoption / Usability Risk**: Excellent dry-run + clear CA warnings + high-signal findings + polished TUI from Phase 2 reduce this. Prominent disclaimers everywhere.
- **Security / Privacy Risk** (CA, redaction, intercepted data): Addressed via dedicated workstream, immutable audit trails, default redaction, and strong documentation.

---

## 5. Recommended Immediate Next Actions

1. Review and align on this roadmap + the detailed design plan (`plans/interactive-web-proxy-loadout-design-plan.md`) — team meeting or async (include TUI, policy, proxy engine, and docs owners).
2. Resolve remaining open questions from the design plan (feature name, CLI exact command, TUI tab name, engine choice, authorization mechanism).
3. Create and commit the detailed Phase 1 foundation handoff plan (`plans/interactive-web-proxy-phase1-foundation-handoff-plan.md`) — break down into granular tasks with owners and estimates.
4. Begin Phase 1 implementation on the existing feature branch (recommended starting point: types + CA module + dry-run CLI stub + policy skeleton — lowest risk, highest learning value entry point).
5. Set up / validate lab test traffic infrastructure (docker-compose additions for easy httpbin/dvwa-style targets) in parallel.
6. Assign owners for the major cross-cutting workstreams listed above.
7. Schedule regular phase checkpoint reviews (end of Phase 1, Phase 2, etc.) with explicit "closeout + next phase kickoff" handoff documents (following mobile/wireless/db-pentest precedent).

---

## 6. How This Roadmap Relates to the Detailed Design Plan

The detailed design plan (`plans/interactive-web-proxy-loadout-design-plan.md`) remains the authoritative source for *what* to build, *why*, technical architecture (module structure, data models, CLI/TUI sketches), safety requirements, risks/edge cases, and open questions. This roadmap provides the *how* and *when* — sequencing, phasing, parallelization, milestones, success criteria, cross-cutting workstreams, and incremental value delivery. Use both documents together:
- Detailed design = specification & rationale
- This roadmap = project plan / execution guide

After each phase, create a short "phase closeout + next phase kickoff" handoff document (following the established mobile/wireless/db-pentest pattern) that captures what was delivered, any deviations or learnings, updated status, and hands off cleanly to the next phase.

The feature branch `feature/interactive-web-proxy-loadout` is already initialized with the detailed design plan and this roadmap as the foundation.

---

**End of High-Level Implementation Roadmap**

This roadmap is designed to be practical, risk-aware, and fully aligned with Eggsec’s established development culture and successful loadout patterns (db-pentest, mobile-dynamic, wireless-active). It enables the team to deliver meaningful interactive web proxy / traffic interception capability iteratively while maintaining the high bar for safety, quality, defense-lab focus, and Rust-native excellence that defines the project.

**Next Immediate Artifact Recommended**: `plans/interactive-web-proxy-phase1-foundation-handoff-plan.md` (detailed task breakdown for Phase 1).