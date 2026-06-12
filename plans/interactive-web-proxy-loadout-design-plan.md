# Interactive Manual Web Proxy / Traffic Interception & Manipulation Loadout Design Plan

**Date**: 2026-06-12  
**Status**: Design Phase — Ready for Phased Implementation Review  
**Branch**: feature/interactive-web-proxy-loadout (created from main)  
**Related**: `architecture/defense_lab.md`, `architecture/proxy.md` (or stress/proxy), `crates/eggsec/src/proxy/`, `docs/SAFETY.md`, `docs/CAPABILITIES.md`, existing proxy management in stress-testing feature, mobile-dynamic (proxy usage in traffic-capture), wireless/mobile loadout plans (pattern reference), EnforcementContext / OperationRisk model, AGENTS.md, Cargo.toml features, eggsec-tui recent architecture, report convert bridge pattern.  
**Authoring Note**: Created via detailed analysis of Eggsec codebase using GitHub connector (root listing, README.md, crates structure, plans/ for precedents, src/ tree revealing existing `proxy/` module, Cargo features, recent phase completions on 2026-06-12). Intended as complete handoff artifact for expanding Eggsec with a dedicated Interactive Manual Web Proxy / Traffic Interception & Manipulation loadout while strictly preserving the safety, scope-enforcement, defense-lab philosophy, and Rust-native approach.

---

## 1. Executive Summary

This plan outlines the design for adding an **Interactive Manual Web Proxy / Traffic Interception & Manipulation Loadout** to Eggsec. The goal is to provide a controlled, interactive MITM-style web proxy capability for authorized lab and defense-validation environments, enabling security professionals to manually inspect, intercept, modify, and forward HTTP/HTTPS (and eventually WebSocket/gRPC) traffic in real-time — similar to Burp Suite Proxy or mitmproxy interactive mode, but deeply integrated into Eggsec's ecosystem.

**Current Gap**: Eggsec has solid web security testing (fuzzing, WAF, endpoint discovery), basic proxy management/pool under `stress-testing` feature (SOCKS/HTTP/Tor for scanning), and mobile-dynamic uses `--proxy` + `--traffic-capture` for external proxying. However, there is no built-in **interactive manual interception and manipulation** surface — no native MITM proxy server with CA for HTTPS, no TUI for live flow inspection/editing, no dedicated report type for intercepted sessions with manipulation audit trails. Users currently rely on external tools (mitmproxy, Burp, OWASP ZAP) for this critical workflow, breaking the unified Eggsec experience, reporting, and safety model.

**Proposed Solution**: A new gated `web-proxy` (or `interactive-proxy` / `mitm-proxy`) feature flag enabling a standalone defense-lab CLI surface (e.g. `eggsec proxy intercept` or `eggsec mitm start --interactive`), following the proven standalone pattern of `eggsec auth-test`, `eggsec wireless ... deauth`, `eggsec mobile dynamic`, and `eggsec db pentest`. Leverage and extend the existing `crates/eggsec/src/proxy/` module. Add TUI support in eggsec-tui for interactive mode. Produce local `WebProxySessionReport` / `ProxyFlow` types bridgeable via `eggsec report convert` to unified ScanReportData with `proxy-intercept-*` and `web-traffic-*` categories.

**Key Principles** (non-negotiable):
- **Defense-lab / regression / hardening validation + manual adversarial simulation only** — never general offensive proxy or credential harvesting platform. All use on traffic to systems you own, operate, or have explicit authorization to test (lab manifests or scope strongly enforced).
- **Standalone-complete surface** initially (MCP/agent tool exposure absent by design, consistent with wireless active / mobile dynamic / db-pentest; optional reporting bridge remains for downstream use).
- **Heavily gated & auditable**: New feature flag, `OperationRisk::Intrusive` or dedicated `TrafficInterception` tier, explicit `--allow-web-proxy` / `--allow-traffic-intercept` + provenance confirmation, dry-run supported, strict budgets (flows, bytes, duration), policy decision records for every intercept/manipulation.
- **Rust-native + extend existing proxy**: Build on/current `proxy/` module; prefer pure-Rust async (tokio + hyper or hudsucker-like patterns, rustls for MITM). Add CA generation/management (rcgen or similar) behind feature.
- **Structured outputs + unified bridge**: New report types + `to_scan_report_data_proxy()` bridge producing rich categories integrable with SARIF/JUnit/HTML/trend pipelines and combined web + proxy + db reports.
- **Interactive-first with TUI excellence**: Leverage recent eggsec-tui 10-phase architecture (UiAction, OverlayController, TabSpec, etc.) for a first-class "Proxy" or "Intercept" tab/experience. Manual mode preflight, enforcement posture indicators, copy-CLI equivalent.
- **Phased & high-signal**: Prioritize core HTTP/HTTPS MITM + basic interactive edit/forward, logging, findings (sensitive data, insecure configs in transit), then advanced protocols and automation rules.

**Deliverables**:
- This plan: `plans/interactive-web-proxy-loadout-design-plan.md`
- Feature flag + Cargo.toml plumbing in `crates/eggsec/Cargo.toml` and `crates/eggsec-cli/Cargo.toml` (and tui)
- Extensions or new submodule under `crates/eggsec/src/proxy/` (or `web_proxy/`, `mitm/`) with server, interceptor, CA, types, dispatcher
- CLI integration (new `proxy` command group or `intercept` subcommand in eggsec-cli / commands/)
- Handler + EnforcementContext policy extensions (new DenialClass/confirmation paths)
- Reporting bridge and finding categories (proxy-intercept-*, traffic-manipulation-*)
- TUI integration: new Proxy/Intercept tab or dedicated interactive session UI in eggsec-tui
- Updated documentation (new or extended `docs/WEB_PROXY.md` or `docs/PROXY_INTERCEPT.md`, `architecture/web_proxy.md` or extension of proxy/stress, README lab defense commands table, SAFETY.md, CAPABILITIES.md, feature_matrix.md)
- Unit + integration tests (dry-run heavy; lab traffic fixtures, e.g. httpbin or dvwa via docker)
- Example lab workflows, CA install instructions, scope/manifest examples, HAR export perhaps
- Smoke test script e.g. `scripts/test-web-proxy.sh` modeled on test-db-pentest.sh / test-mobile-dynamic.sh

**Success Criteria**: All proxy operations require feature + explicit policy gate + lab authorization signals + dry-run validation; dry-run produces complete structured JSON usable in `report convert`; interactive TUI allows safe manual inspection/editing/forwarding with full audit trail in report; high-signal findings (e.g. PII in transit, weak headers, JWT manipulation opportunities) generated; regression workflows (baseline traffic vs hardened) supported; seamless integration as upstream proxy for other Eggsec scans or external browsers/tools; no regressions in existing proxy pool, web fuzzing, or TUI.

---

## 2. Background & Current State

### 2.1 Existing Proxy & Traffic Capabilities
- **Basic Proxy Management (stress-testing feature)**: SOCKS4/5, HTTP, HTTPS, Tor proxy pool with health checking. Used primarily as upstream for scans/loadtests to route traffic or for evasion. See `crates/eggsec/src/proxy/` and `src/stress/`.
- **Mobile Dynamic Traffic Capture**: `eggsec mobile dynamic ... --proxy 127.0.0.1:8080 --traffic-capture /tmp/mitm.log` — consumes external proxy; captures summary. Phase 2 closed 2026-06-12. Shows demand for proxy integration but no native provider of interactive MITM.
- **Web Security Stack**: Excellent fuzzer (SQLi/XSS/etc), WAF detection/bypass, endpoint discovery, headless-browser for DOM. All HTTP-based but no live interception layer for manual manipulation during tests.
- **No Interactive MITM**: No built-in CA-signed HTTPS interception, no breakpoint/intercept mode, no TUI for editing live requests/responses, no dedicated session reports focused on traffic flows and manual changes.
- **Safety & Enforcement Model**: Mature `EnforcementContext::evaluate()`, `LoadedScope`, `OperationRisk`, dry-run planning (`eggsec plan`), scope TOML. All new capabilities must integrate here. Recent TUI pass added excellent manual-mode indicators.
- **Loadout Precedents**: Wireless (passive + active deauth under wireless-advanced), Mobile (static + dynamic phases executed June 2026), Db-pentest (Phase 5 complete 2026-06-12 with Mongo/Redis/compliance). These provide exact templates for standalone CLI, TUI future, reporting bridges, policy, docs, phased handoff plans, and smoke tests.

### 2.2 Why Add Now?
Interactive manual web proxy / traffic interception is a core workflow for web app pentesting, auth testing (JWT/cookie manipulation), API security (modify GraphQL/REST), WAF evasion testing (live header/payload tweaks), and defense regression ("what does our WAF do if I modify this header in real time?"). Integrating it natively unifies the toolchain, brings all traffic under Eggsec's safety/scope/reporting model, enables combined reports (web fuzz + intercepted manual flows), and supports agent/MCP future consumers via bridge without exposing live interactive surface. Aligns with Eggsec's mission for repeatable, scoped, defense-validation workflows and recent expansion of specialized loadouts (db, mobile, wireless).

---

## 3. Goals, Non-Goals, and Scope

### 3.1 Primary Goals
- Deliver curated, high-signal **interactive manual web proxy primitives** for lab/defense use:
  - Full HTTP/HTTPS MITM proxy server (explicit proxy mode; optional transparent notes) with automatic CA generation or user-provided CA.
  - Real-time flow capture: method, URL, headers, body (pretty-print JSON/XML/text, hex/binary view), response status/headers/body, timing, size.
  - Interactive interception: rule-based or manual breakpoints; pause flows matching criteria (host, path, method, header, body content regex); TUI to inspect, edit (headers add/modify/delete, body text edit with basic validation), forward modified, drop, or replay original.
  - Manipulation audit trail: log every change (before/after diff) in report; support "what-if" regression by replaying modified sessions.
  - Structured findings: auto-detect sensitive data (credit cards, emails, tokens, PII patterns), insecure transit (HTTP, weak ciphers, missing HSTS/CSP in responses), auth token issues (JWT alg none/weak, long expiry), etc. Categories `proxy-intercept-*`, `web-traffic-*`, `manipulation-*`.
  - Session management: save/load sessions, export to HAR or JSON, baseline vs current diff for regression.
  - Upstream chaining & integration: use as proxy for other Eggsec commands (e.g. `eggsec scan ... --proxy 127.0.0.1:8080` while intercepting), or external browsers/tools targeting lab apps.
  - Dry-run & planning: preview proxy setup, simulate flows without binding or with mock responses.
  - Excellent TUI: dedicated Proxy/Intercept experience leveraging recent architecture (global task strip, action palette, enforcement indicators, small-terminal degraded layouts).
- Maintain **zero accidental misuse**: multi-layer gating (feature + policy + confirmation + budgets + dry-run + prominent lab disclaimers).
- Produce structured findings integrable with existing output pipeline (`eggsec report convert`, SARIF, JUnit, HTML, trend analysis, combined reports).
- Future extensibility to WebSocket interception/editing, HTTP/2, gRPC framing, scripting/rules engine (simple match-action), transparent proxy mode (with iptables guidance).

### 3.2 Non-Goals (Explicitly Out of Scope for Initial Phases)
- Full offensive proxy framework or automated exploitation (no auto-payload injection beyond manual; no sqlmap-style automation here — use existing fuzzer).
- Unbounded traffic storage or exfiltration of sensitive intercepted data (strict redaction defaults, budgets, optional encryption of session files).
- General-purpose credential harvesting or session hijacking platform (focus on authorized lab traffic to owned targets; findings for defense validation only).
- Production transparent proxy or high-scale reverse proxy (lab/explicit proxy focus; performance budgets enforced).
- Initial MCP/agent `SecurityTool` registration for the live interactive surface (standalone defense-lab like other loadouts; bridge available).
- iOS/Android specific proxying or certificate pinning bypass automation (user handles device CA install; docs provide guidance).
- Full body editing for very large/binary payloads in Phase 1 (basic text + hex view; advanced later).

### 3.3 In-Scope Priorities (Phased)
| Phase | Focus                          | Key Capabilities                                      | Risk/Complexity | CLI / TUI Example                          |
|-------|--------------------------------|-------------------------------------------------------|-----------------|--------------------------------------------|
| 1     | Foundation HTTP/HTTPS MITM    | Proxy server, CA setup, basic flow logging, non-interactive mode, report types, bridge, dry-run, feature gate, CLI skeleton | Medium         | `eggsec proxy intercept --listen 127.0.0.1:8080 --ca-dir ~/.eggsec/ca --dry-run --json` |
| 2     | Interactive TUI + Manipulation| Live flow list/detail, breakpoint/intercept rules, edit headers/body, forward/drop/replay, manipulation audit, basic findings, TUI tab | High (TUI)     | Launch TUI or `eggsec proxy intercept --interactive`; in TUI: select flow, 'e' edit, 'f' forward |
| 3     | Advanced Protocols & Polish   | WebSocket support, HTTP/2 partial, rule engine (simple), upstream integration examples, evidence bundles, correlation, HAR export, performance | Medium-High    | `--intercept-ws`, advanced TUI panes, combined reports |
| 4+    | Extensibility                 | gRPC, transparent mode guidance, scripting hooks, pipeline profile integration, MCP opt-in review | Varies         | Future                                     |

---

## 4. Safety, Policy & Enforcement Model Extensions

### 4.1 Feature Flag
```toml
# crates/eggsec/Cargo.toml (example)
web-proxy = []  # or interactive-proxy / mitm-proxy; pulls in hyper, rustls, rcgen, etc.
# Sub-features later e.g. web-proxy-advanced for WS/HTTP2
```
Propagate to `crates/eggsec-cli/Cargo.toml` and `eggsec-tui/Cargo.toml`. Add to `full` feature. Reuse/extend `insecure-tls` for testing.

### 4.2 Risk / Capability Classification
- Add or extend `OperationRisk::Intrusive` + new capability `TrafficInterception` or `WebProxyManipulation`.
- Handler enforces via `EnforcementContext::evaluate()` (non-downgradable in strict/MCP/agent/CI).
- New confirmation path for "interactive traffic interception requires explicit lab authorization + provenance prompt".
- Policy decision records: proxy listen addr, CA used (fingerprint), intercept rules active, flows intercepted count, manipulations performed (diffs), upstream targets touched (redacted), budgets used.

### 4.3 Runtime Gating & UX (Stricter than Basic Proxy Pool)
- Prominent pre-execution / TUI banner:
  > "INTERACTIVE WEB PROXY / TRAFFIC INTERCEPTION MODE — For authorized lab traffic ONLY. All flows logged/audited. Manual edits create manipulation records. Use --dry-run first. Install CA for HTTPS. Supply scope or lab manifest."
- `--dry-run` produces full structured report skeleton (no server bind, mock flows optional).
- `--allow-web-proxy` (or `--allow-traffic-intercept`) narrow audited override + `--manual-override-reason` + interactive provenance prompt ("Confirm this proxy session targets only lab systems you control and have authorization for").
- **Lab / Scope Integration**: Reuse or extend scope.toml or add lightweight `proxy-manifest.toml` (allowed upstream host patterns, ports, max flow budget). Enforced before accepting connections or in TUI preflight.
- Strict budgets: `--max-flows 500`, `--max-bytes-per-flow 10MB`, `--max-duration 3600s`, `--max-concurrent 50` (conservative defaults; configurable).
- All runs produce auditable artifacts even in dry-run/JSON. Sensitive data redaction in reports (configurable patterns + default PII/token redaction).
- HTTPS CA handling: Clear warnings + instructions for installing generated or custom CA in browser/OS trust store. Option to use existing system CA or provide cert/key. Never auto-trust in production contexts.

### 4.4 Legal / Ethical / Documentation
Every command, help text, TUI screen, finding, and report must surface strong disclaimers: "Use ONLY for traffic to systems you own or have explicit written authorization to test. Prefer dedicated lab instances. All intercepted data is sensitive — handle securely. Know your local laws and organizational policies. Manipulation creates audit records; do not use to bypass controls without authorization."
Reference `docs/SAFETY.md` (new Web Proxy / Traffic Interception section) and updated README.

### 4.5 MCP / Agent Exposure
**Recommendation**: Keep the **interactive / live manipulation surface** as **standalone defense-lab** initially (no `SecurityTool` registration in tool registry, consistent with wireless active, mobile dynamic, db-pentest). The reporting bridge (`to_scan_report_data_proxy`) allows downstream consumers (agent skills, MCP tools, pipelines) to ingest structured flows/findings without exposing the interactive server or TUI. Future opt-in review for read-only proxy status or replay capabilities.

---

## 5. Technical Architecture

### 5.1 Module Structure (Proposed — extend existing proxy/)
```
crates/eggsec/src/proxy/
├── mod.rs                 # Public exports, dispatcher, WebProxySessionReport, run_proxy_intercept_cli
├── mitm.rs or server.rs   # Core MITM proxy server (hyper/hyper-rustls or hudsucker wrapper, CA integration)
├── interceptor.rs         # Interception logic, breakpoint engine, flow capture, manipulation recorder
├── ca.rs                  # Certificate authority generation, signing, storage, fingerprinting (rcgen + rustls)
├── types.rs               # ProxyFlow (request/response + metadata + manipulation_log), WebProxySessionReport, InterceptRule, etc.
├── tui.rs or integration  # TUI-specific adapters (if needed; prefer shared with eggsec-tui)
├── bridge.rs              # to_scan_report_data_proxy() implementation
├── utils.rs               # Redaction, pretty-print (JSON/XML), HAR export helpers, budget tracking
└── manifest.rs            # Optional proxy-manifest parser/validator (or reuse LoadedScope)
```
Extend or create `web_proxy` subdir if separation cleaner. Integrate with existing `stress` or `proxy` for shared pool primitives where sensible. CLI handler in `commands/proxy_intercept.rs` or extend `commands/`. TUI in `eggsec-tui` new tab spec "Proxy" or "Intercept".

### 5.2 Data Models (New/Extended)
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyFlow {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub method: String,
    pub url: String,           // Redacted host if needed
    pub request_headers: HashMap<String, String>,
    pub request_body: Option<Vec<u8>>, // Or truncated/redacted
    pub response_status: Option<u16>,
    pub response_headers: Option<HashMap<String, String>>,
    pub response_body: Option<Vec<u8>>,
    pub duration_ms: u64,
    pub size_bytes: u64,
    pub manipulations: Vec<ManipulationRecord>, // {field: "header:Authorization", before: "...", after: "...", reason: "manual edit"}
    pub intercepted: bool,
    pub forwarded: bool,
    pub dropped: bool,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebProxySessionReport {
    pub session_id: Uuid,
    pub scan_type: String,     // "web-proxy-intercept" or "traffic-interception"
    pub timestamp: String,
    pub listen_addr: String,
    pub ca_fingerprint: String,
    pub flows: Vec<ProxyFlow>,
    pub findings: Vec<Finding>, // proxy-intercept-pii-detected, web-traffic-weak-header, manipulation-sensitive-token, etc.
    pub recommendations: Vec<String>,
    pub duration_ms: u64,
    pub flows_intercepted: u64,
    pub manipulations_performed: u64,
    pub actions_performed: Vec<String>, // audit trail
    pub manifest_matched: bool,
    pub dry_run: bool,
    pub budgets: BudgetUsage,
}

// Bridge example
pub fn to_scan_report_data_proxy(result: &WebProxySessionReport) -> crate::output::convert::ScanReportData { ... }
// Populates findings with proxy-intercept-* + web-traffic-* categories + rich evidence (redacted)
```

### 5.3 Core Tech Choices
- **Proxy Engine**: Extend existing or adopt lightweight async Rust MITM. Options: wrap `hudsucker` (popular pure-Rust MITM), or direct `hyper` + `hyper-rustls` + custom CA signer for request/response interception. Prefer minimal new deps; evaluate what's lightest for feature.
- **CA & TLS**: `rcgen` for self-signed CA + server certs per host (or wildcard). rustls for serving. Clear separation of generated vs user-provided CA. Instructions for browser/OS install (Firefox, Chrome, system keychain, Android/iOS profiles).
- **Interception & Editing**: In-memory flow store with channel for TUI to subscribe/pause. Edit via simple diffable structures (header map + body bytes). For TUI: ratatui tables + textareas or custom editor component (reuse or extend existing TUI patterns).
- **Async Runtime**: Tokio (already core). Support graceful shutdown, signal handling.
- **Redaction & Privacy**: Configurable + default patterns for PII/tokens/secrets in bodies/headers before storage or reporting. Optional session file encryption.
- **Integration Points**: CLI dispatches after policy eval. TUI launches proxy in background task + overlays interactive UI. Can chain as `--proxy` target for other eggsec commands while running interactive session.

### 5.4 CLI Integration
Extend clap or add `proxy` command group:
```bash
# Examples
# Non-interactive / logging mode (Phase 1)
eggsec proxy intercept --listen 127.0.0.1:8080 \
  --ca-dir ~/.config/eggsec/ca --generate-ca-if-missing \
  --max-flows 200 --max-duration 1800s --dry-run --json -o proxy-session.json \
  --allow-web-proxy --manual-override-reason "Authorized lab traffic interception for WAF regression"

# Interactive TUI mode (Phase 2+)
eggsec proxy intercept --interactive --listen 127.0.0.1:8080 --ca ...
# Or from TUI main: navigate to Proxy tab, configure, start session

# With rules / upstream
eggsec proxy intercept --intercept-rule "host contains lab.example.com" --upstream-proxy socks5://127.0.0.1:9050

# List CA or sessions
eggsec proxy ca list --ca-dir ...
eggsec proxy sessions --list
```
Human output: proxy status, CA info (install instructions), live flow summary (if non-TUI), findings summary, prominent lab disclaimer.
JSON: Full WebProxySessionReport (or bridged).

**TUI Experience Sketch** (high-level, per recent TUI architecture):
- New TabSpec "Proxy" or "Intercept".
- Preflight panel: enforcement posture, scope/manifest match, budgets, CA status, listen addr.
- Main view: split or tabs — Flow list (table: ID, Time, Method, Host/Path, Status, Size, Modified?, Actions), Detail pane (headers pretty, body pretty/hex/raw toggle, manipulation history).
- Action bar / overlay: Intercept rules editor, global forward/drop, pause all, export session, copy as curl/har.
- Edit modal: Header editor (add/modify/delete with validation), Body textarea (syntax aware if possible), Diff preview before forward.
- Status strip: active flows, bytes transferred, policy decisions, task progress.
- Small terminal degraded: condensed list + key bindings help.

---

## 6. Detailed CLI / TUI Command Designs (Phase 1-2 Focus)

Core entry: `eggsec proxy intercept [OPTIONS]` (or `eggsec mitm intercept` if preferred naming).

Key options (expandable):
- `--listen <addr:port>` (default 127.0.0.1:8080)
- `--ca-dir <path>` or `--ca-cert <file> --ca-key <file>` (generate if missing with warning)
- `--interactive` / `--tui` (launch interactive mode; default non-interactive logging in Phase 1)
- `--intercept-rule <pattern>` (repeatable; e.g. "path contains /api/", "header Authorization matches .*", body regex)
- `--max-flows N`, `--max-bytes-per-flow N`, `--max-duration secs`, `--max-concurrent N`
- `--dry-run`, `--json`, `-o`, `--quiet`
- `--allow-web-proxy` / `--allow-traffic-intercept`, `--manual-override-reason`
- `--proxy-manifest examples/lab-proxy-manifest.toml` or reuse `--scope`
- `--upstream-proxy <url>` (chain to Tor/SOCKS for egress)
- `--insecure-tls` (for upstream or testing)

**Findings Examples** (illustrative, auto + manual flag):
- High: "Intercepted JWT with alg=none or weak secret in Authorization header — potential token forgery vector"
- Medium: "Sensitive PII (email, SSN pattern) detected in request body to /profile/update — recommend encryption/tokenization"
- Medium: "Response missing security headers (Content-Security-Policy, X-Frame-Options) on login endpoint"
- Low/Info: "HTTP (non-HTTPS) traffic intercepted to lab target; recommend enforcing TLS"
- Manipulation: "Manual edit: Added X-Eggsec-Test: regression-123 header; Original value: (none)"

---

## 7. Phased Implementation Roadmap

**Phase 0 (This Plan)**: Design document created, reviewed, and committed to feature branch. Team alignment on feature name (`web-proxy` recommended), CLI naming (`eggsec proxy intercept`), TUI tab name, driver/engine choice (extend proxy/ vs hudsucker), and lab-manifest vs scope integration.

**Phase 1 (Foundation — P0, ~3-5 weeks)**:
- Feature flag + Cargo plumbing + dep evaluation (hyper/rustls/rcgen or hudsucker; minimal surface).
- Extend `proxy/` module: CA handling, basic MITM server (HTTP + HTTPS), flow capture/logging to in-memory + optional file.
- Types: ProxyFlow, WebProxySessionReport, basic InterceptRule.
- CLI args + handler + full EnforcementContext integration + dry-run path (no bind or mock).
- `to_scan_report_data_proxy` bridge + new finding categories + redaction.
- Unit tests (mock flows, CA gen/sign/verify, report shape, bridge roundtrips).
- Minimal docs: README quick-ref + lab defense commands table update, new `docs/WEB_PROXY.md` stub, architecture/web_proxy.md or proxy/ extension, SAFETY.md, CAPABILITIES.md, feature_matrix.md.
- CA install instructions + example lab workflow (browser -> eggsec proxy -> lab app like dvwa/httpbin).
- Smoke test script `scripts/test-web-proxy.sh` (dry-run + basic non-interactive with curl or docker target; validate JSON + bridge).
- AGENTS.md updates.

**Phase 2 (Interactive TUI + Core Manipulation — P1, ~4-6 weeks)**:
- TUI integration: new Proxy/Intercept TabSpec, flow list/detail views, edit modal, action handling (forward/drop/replay with diff recording).
- Interception engine: pause/resume flows, rule matching, manipulation recorder (before/after + reason).
- Full interactive mode launch from CLI (`--interactive`) or TUI.
- Enhanced findings + recommendations from flows.
- Session save/load, basic export (JSON/HAR).
- Integration examples: run proxy + other eggsec scan using it as --proxy; combined report via convert.
- Polish: error handling, graceful shutdown, budget enforcement in TUI, small-terminal layouts.
- Lab smoke with real interactive session (known lab target, manual JWT/header edit test).

**Phase 3 (Advanced & Polish — P2)**:
- WebSocket interception/editing (upgrade handling, frame inspection).
- HTTP/2 support (partial or via h2 crate).
- Simple rule engine (persistent rules, match-action like "if path /login then add header X-Test").
- Evidence bundles + correlation with web/db findings (e.g. "manual manipulation confirmed SQLi vector").
- Performance tuning, concurrent flow handling, memory bounds.
- Full example regression workflow (baseline traffic capture -> harden app/WAF -> re-intercept -> diff report).
- Docs expansion: advanced usage, transparent proxy notes (iptables/pf), device CA install (Android/iOS), security review notes.

**Phase 4+ (Extensibility)**:
- gRPC / protobuf framing inspection.
- Pipeline profile integration (e.g. `defense-lab` profile with proxy stage).
- Optional gated advanced manipulation (safe test mutations).
- MCP/agent opt-in review for read-only or replay surfaces.
- Cross-loadout correlation engine extensions.

**Cross-Cutting (parallel)**:
- EnforcementContext / OperationRisk / proxy-manifest design & tests.
- Output bridge tests (proxy JSON → convert → SARIF/JUnit/HTML + combined with web fuzz reports).
- TUI architecture alignment (reuse UiAction, OverlayController, etc.).
- Hardware/lab matrix (various browsers, TLS versions, HTTP/2, WS).

**Testing Strategy**:
- Heavy dry-run + mock in CI.
- Lab-only real traffic tests (docker targets like httpbin, dvwa, juice-shop with known issues).
- Policy enforcement tests (mock EnforcementContext + manifest/scope).
- TUI interaction tests (if possible via headless or snapshot).
- Regression: add web-proxy examples to defense-lab profiles/CI once stable.

---

## 8. Risks, Edge Cases & Mitigations

| Risk / Edge Case                                      | Impact                                      | Mitigation                                                                 | Owner      |
|-------------------------------------------------------|---------------------------------------------|----------------------------------------------------------------------------|------------|
| User intercepts production or sensitive personal traffic | Legal / breach / data exposure             | Multi-layer warnings + mandatory lab signals + provenance prompt + strict budgets + default redaction + prominent disclaimers + scope/manifest | Policy + Docs |
| HTTPS MITM breaks certificate pinning or HSTS        | User frustration or incomplete testing     | Clear docs on lab setup (pinning bypass in test builds, --insecure-tls notes); recommend test builds without pinning | Docs + Impl |
| Large/binary payloads or streaming responses crash or OOM | Performance / stability issues             | Strict per-flow byte budgets + truncation + streaming support where practical; hex view for binary | Impl       |
| TUI edit introduces invalid HTTP (malformed headers) | Forward fails or app breakage              | Validation on edit (header names, basic syntax), preview diff, revert option, dry-run simulation | Impl + TUI |
| Concurrent high-volume traffic overwhelms TUI or budgets | Missed flows or poor UX                    | Configurable concurrent limits, flow sampling/throttling in interactive mode, background capture + on-demand detail | Impl       |
| CA private key compromise or weak generation         | Trust issues or MITM by others             | Strong key gen (proper entropy), secure storage guidance, option for user-provided CA, warnings never reuse prod CAs | Impl + Docs |
| Manipulation audit trail incomplete or tamperable    | Audit / compliance gaps                    | Immutable append-only manipulation log in report, session file hashing/signing optional | Impl       |
| Scope/manifest enforcement bypassed in proxy mode    | Out-of-scope traffic accepted              | Preflight enforcement + per-connection host checks against manifest/scope + redacted logging of touched hosts | Policy + Impl |
| WebSocket / HTTP/2 / gRPC edge cases incomplete      | Partial protocol support                   | Phase 3 explicit; document limitations in Phase 1/2; graceful fallback or passthrough | Docs + Impl |

**Monitoring for Abuse**: Every session (even dry-run) produces full policy decision record, manifest/scope match, budget usage, flow count, manipulation diffs, and structured findings. Lab manifests/scope provide additional control layer. All reports redacted by default.

---

## 9. Open Questions & Decisions Needed (for Team)

1. Exact feature flag name: `web-proxy` (preferred, concise) vs `interactive-proxy` vs `mitm-proxy` vs `traffic-intercept`? Sub-features for advanced protocols?
2. CLI command structure: `eggsec proxy intercept` (recommended, groups with future `proxy pool` or `proxy health`) vs top-level `eggsec mitm` or `eggsec intercept`? Naming consistency with `db pentest`, `mobile dynamic`.
3. TUI tab/experience name: "Proxy", "Intercept", "Traffic", or integrated into existing Web/WAF tab? How to launch interactive from main TUI vs dedicated CLI flag.
4. Proxy engine: Extend existing `proxy/` module with custom hyper/rustls MITM, or adopt `hudsucker` crate (evaluate license, maintenance, size)? Pure-Rust priority?
5. Lab authorization mechanism: Reuse/enhance `--scope` / LoadedScope, or new lightweight `proxy-manifest.toml` (host patterns, budgets)? Mandatory or advisory + confirmation?
6. Default redaction strictness and PII patterns? Configurable via TOML or flags? Session file encryption opt-in?
7. Edit capabilities in Phase 2: Full header map edit + body text (JSON pretty) or also binary/hex patch? Validation level?
8. Any early pipeline profile or combined web+proxy example preferences?
9. Should generated CA be per-session or persistent in ~/.config/eggsec/ca ? Fingerprint in reports always?
10. Future MCP exposure: read-only status / flow query only, or replay capability? Security review timing.

---

## 10. Handoff Checklist

- [ ] Team review & approve this plan (security + architecture + TUI review recommended).
- [ ] Commit plan file to feature/interactive-web-proxy-loadout branch (done via this action).
- [ ] Create follow-up issues/tasks for Phase 1 (feature flag, proxy/ module extensions or new mitm submod, CA handling, basic server, CLI/handler/policy, types/bridge, docs stub, smoke script).
- [ ] Decide on feature name, CLI naming, TUI tab, engine choice; assign owners for cross-cutting (EnforcementContext extensions, redaction, TUI integration).
- [ ] After Phase 1: Full `cargo test --features web-proxy`, lab traffic smoke tests (docker targets), sample reports + convert output, regression check vs main, TUI compile/run.
- [ ] Post-implementation: Update `docs/WEB_PROXY.md` (or integrate), `architecture/proxy.md` or new web_proxy section, README (lab defense commands table + quick start examples), SAFETY.md, CAPABILITIES.md, feature_matrix.md, AGENTS.md. Add CA install guide and lab workflow examples.
- [ ] Consider ADR in `docs/adr/` for the standalone defense-lab decision, interactive TUI safety model, and CA handling approach.
- [ ] Align with existing proxy pool (stress-testing) for shared primitives or future unified `eggsec proxy` command surface.

**Immediate Next Action After Handoff**: Team aligns on feature flag name (`web-proxy` recommended) and CLI/TUI naming, then spins up implementation on the feature branch following this plan and wireless/mobile/db-pentest handoff patterns. Prioritize Phase 1 foundation so interactive TUI can build on solid MITM + reporting base.

---

## 11. References & Further Reading

- Loadout precedents: `plans/non-web-database-pentesting-loadout-design-plan.md`, `plans/dynamic-mobile-testing-loadout-design-plan.md`, `plans/wireless-active-attacks-loadout-design-plan.md` and related phase handoff plans (exact structure, safety model, standalone decision, phased execution, smoke tests).
- Architecture: `architecture/defense_lab.md`, existing `proxy/` and `stress/` modules, `architecture/pipeline.md`, `architecture/findings.md`, `architecture/cli_commands.md`, `architecture/tui.md` (recent 10-phase pass).
- Safety core: `docs/SAFETY.md`, EnforcementContext implementation.
- Existing standalone loadouts: `auth-test`, wireless module (src/wireless), mobile module (src/mobile), db_pentest (src/db_pentest).
- Feature config: `crates/eggsec/Cargo.toml`, `crates/eggsec-cli/Cargo.toml` (db-pentest, mobile-dynamic, wireless-advanced, stress-testing examples).
- Output & reporting: `architecture/output.md`, `crates/eggsec/src/output/`, report convert bridge pattern.
- TUI: `crates/eggsec-tui/`, recent architecture updates for manual mode, overlays, tabs.
- Full context: root README.md (proxy management, mobile --proxy usage, lab defense commands), AGENTS.md, CONTRIBUTING.md, plans/ directory, current feature/interactive-web-proxy-loadout branch.

---

**End of Plan Document**

*This document is intended as a complete, self-contained handoff artifact. It captures context, rationale, detailed design, risks, edge cases, and actionable phased roadmap so the eggstack team can implement the Interactive Manual Web Proxy / Traffic Interception & Manipulation expansion without ambiguity while preserving Eggsec's core safety, quality, defense-lab standards, and Rust-native philosophy. The feature branch `feature/interactive-web-proxy-loadout` has been created with this plan committed as the starting point for implementation.*
