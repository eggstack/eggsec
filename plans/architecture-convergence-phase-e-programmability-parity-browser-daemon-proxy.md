# Phase E Plan: Programmability Parity for Browser, Daemon, and Proxy

## Status

Status: Executed (2026-09-09). No domain promoted; all three areas stay
provisional with documented rationale (see `docs/python/domain-maturity.md`
"Architecture Convergence Phase E" section).

## Objective

Close the largest gaps between Eggsec's programmable APIs and its underlying Rust execution capabilities, prioritizing areas where substantial type/schema work already exists but execution behavior remains placeholder or incomplete.

The three target areas are:

1. managed browser sessions in `eggsec-python`;
2. local versus daemon execution parity;
3. Python/protocol exposure of web-proxy exchange/results/events.

This phase should improve the usefulness of existing interfaces before any new security-testing domains are added.

## Preconditions

Phase C should provide canonical operation request/result contracts. Phase D should provide injected engine/protocol service boundaries. Do not wire provisional APIs directly to transitional dispatch internals if those internals are scheduled for immediate replacement.

## Primary files/areas

```text
crates/eggsec/src/browser/
crates/eggsec-python/src/browser_session.rs
crates/eggsec-python/src/browser_assess.rs
crates/eggsec-python/src/browser_events.rs
crates/eggsec-python/src/async_engine.rs
crates/eggsec-runtime/
crates/eggsec-daemon/
crates/eggsec-daemon-protocol/
crates/eggsec-cli/src/daemon_cli.rs
crates/eggsec-python/src/*daemon*
crates/eggsec-web-proxy/
crates/eggsec/src/tool/implementations/proxy.rs
crates/eggsec-python/src/*proxy*
docs/python/domain-maturity.md
docs/DAEMON.md
docs/WEB_PROXY.md
docs/python/
```

## Non-goals

This phase does not add browser exploitation primitives, new proxy attack features, new daemon transports, or remote authorization shortcuts. It does not promote a domain to stable solely because methods stop being placeholders; graduation still requires the documented maturity checklist.

## Workstream 1 — Browser backend contract

Define a backend trait/interface that represents the capabilities advertised by managed browser sessions, conceptually:

```text
BrowserBackend
BrowserSessionHandle
navigate
wait_for_selector
capture_dom
console_events
network_events
cookies/storage
screenshot
execute_script
close
```

The backend should be implemented by the existing Rust headless-browser infrastructure where feasible. Avoid binding the Python class directly to a concrete `headless_chrome` object if a small engine abstraction provides testability and future backend flexibility.

Capabilities must be truthful: `BrowserCapabilities` fields should be derived from the active backend, not hard-coded aspirational values.

## Workstream 2 — Wire synchronous and asynchronous Python browser sessions

Replace placeholder behavior in managed browser sessions.

Required behavior:

- `start()` launches/attaches a backend and reaches `Ready` or fails with a structured error;
- `navigate()` performs real navigation and records status/final URL/redirect/load timing;
- `wait_for_selector()` uses actual DOM state and bounded timeout/cancellation;
- `get_dom_snapshot()` reflects the current page;
- console/network event getters return captured events;
- cookie/storage getters return actual backend state subject to configured collection settings;
- `take_screenshot()` writes through the artifact store and returns a resolvable artifact reference;
- `execute_script()` runs only when backend capability and security policy allow it;
- `stop()` closes resources idempotently;
- async APIs use shared runtime ownership correctly and do not block Python's event loop unnecessarily.

State transitions should be explicit and tested for failures/cancellation, not just happy path.

## Workstream 3 — Browser security/policy integration

Browser navigation is network execution and must obey authoritative scope throughout redirects and subresource behavior according to existing product policy.

Review:

- initial URL authorization;
- redirect target checks;
- private/non-public resolution behavior;
- proxy configuration;
- insecure certificate settings;
- cross-host navigation;
- script execution risk classification;
- artifact redaction of cookies/storage/headers.

Do not let the browser backend become an alternate path around `EnforcementContext` or redirect policy.

## Workstream 4 — Daemon parity contract

Define an explicit parity matrix between local `Engine`/runtime execution and daemon-backed execution.

At minimum cover:

- request normalization;
- policy/scope enforcement;
- task submission identifiers;
- progress/event ordering;
- lag/replay semantics;
- result retrieval after completion;
- cancellation before/after task start;
- client disconnect/reconnect;
- daemon restart with persisted sessions/tasks where supported;
- timeouts;
- structured errors;
- artifact references and retrieval;
- capability discovery/version negotiation;
- ownership/RBAC.

Document any behavior intentionally different because daemon execution is durable or multi-client.

## Workstream 5 — Result retrieval and reconnect/replay

Close the current provisional gaps around daemon result use from programmable clients.

Requirements:

- completed task result is retrievable without relying on transient event delivery;
- reconnecting clients can re-establish session context subject to authorization;
- event sequence numbers/replay windows have documented semantics;
- lagged receivers distinguish missed events from terminal closure;
- duplicate/replayed events can be detected by clients;
- cancellation state persists consistently;
- artifact metadata survives the same lifecycle as the task result.

Version protocol changes if wire compatibility requires new response fields.

## Workstream 6 — Python daemon client parity

Expose a coherent Python API for daemon-backed execution that maps onto the same canonical request/result classes used locally.

Do not create a second family of incompatible result DTOs. Prefer:

```text
local Engine -> canonical result
DaemonClient -> canonical result
```

with transport metadata available separately.

Add sync/async daemon client tests using a spawned local daemon fixture.

## Workstream 7 — Proxy result/exchange parity

Audit the Rust web-proxy session/report model and the Python/protocol binding model.

Close gaps so supported proxy workflows expose real:

- intercepted requests/responses;
- protocol/method/status metadata;
- timing;
- header/body summaries subject to configured limits/redaction;
- WebSocket/gRPC exchange metadata where supported by Rust domain;
- modification decisions;
- session lifecycle/events;
- structured errors.

If a capability is not implemented in Rust, the binding must report unsupported rather than return synthetic empty success data.

## Workstream 8 — Secret and artifact handling

Browser/proxy/daemon surfaces can carry sensitive cookies, authorization headers, credentials, response bodies, and captured artifacts.

Verify:

- `SensitiveString` or equivalent handling for secrets;
- repr/log/event/report redaction;
- content-size limits;
- artifact directory permissions;
- daemon persistence redaction/encryption policy as currently documented;
- explicit opt-in before retaining full sensitive bodies if the current product contract requires it.

Add sentinel-based persistence/logging tests.

## Workstream 9 — Integration fixtures

Create deterministic local fixtures:

- browser test web app with redirect, DOM mutation, console, cookies/storage, network request, downloadable/screenshotable page;
- daemon fixture that exercises restart/reconnect and task result retrieval;
- proxy target/upstream fixture with HTTP, WebSocket, and where practical HTTP/2/gRPC scenarios.

Fixtures must bind locally and require explicit test scope. Avoid public-network dependencies.

## Workstream 10 — Maturity reevaluation

After implementation, reassess `docs/python/domain-maturity.md` using the existing graduation checklist.

Do not automatically mark browser/daemon/proxy stable. Promotion requires:

- canonical IDs/contracts where applicable;
- sync/async parity;
- structured errors/events/cancellation;
- deterministic fixtures;
- type stub/docs/wheel-profile coverage;
- daemon contract coverage for operations that claim daemon support.

## Acceptance criteria

- Python BrowserSession no longer has placeholder success/empty implementations for advertised supported capabilities;
- browser session operations are backed by real engine behavior and obey scope/redirect policy;
- daemon clients can retrieve canonical completed results after reconnect;
- event ordering/replay/cancellation semantics are explicit and tested;
- Python local and daemon execution return compatible typed results;
- proxy bindings expose actual captured exchanges for supported workflows and return explicit unsupported errors otherwise;
- secrets/artifacts are redacted/persisted according to policy;
- deterministic local integration fixtures cover all three areas;
- applicable Python, daemon, browser, proxy, and `make check` suites pass.

## Completion record

- Baseline SHA: `8dd20331` (Phase D head). Final SHA: `7b4a5ecd`
  (implementation commit; CI + Code Quality green on that SHA).
- Backend selected: `headless_chrome` (`headless-browser` Cargo feature).
  New engine contract `crates/eggsec/src/browser/backend.rs`
  (`BrowserBackendKind`, `BrowserBackendCapabilities`,
  `BrowserBackend` trait, `capabilities_for_current_build()`,
  `validate_browser_url()`); Python capabilities derive from the compiled
  backend via `BrowserCapabilities::current()` /
  `browser_backend_name()` / `browser_backend_available()`.
- Daemon protocol: v1 → v2 (additive `GetTaskResult` / `TaskResult`;
  `Observer` permission; Unix socket + HTTP
  `GET /sessions/{id}/tasks/{task_id}` + CLI `eggsec task result` +
  Python `async_daemon_get_task_result()` on the canonical `TaskOutcome`
  schema). Parity matrix: `docs/DAEMON_PARITY.md`. Removed dead duplicate
  `crates/eggsec-daemon/src/protocol.rs` (wire types live only in
  `eggsec-daemon-protocol`).
- Proxy: `FlowBuffer::flows()` returns a real ordered slice (in-place
  `make_contiguous`); MCP `proxy-start` fails explicitly for live mode and
  serves labeled synthetic fixtures in dry-run only;
  `proxy-export-session` builds a real `WebProxySessionReport`;
  `ProxyEntry`/`ProxyRoutePy` passwords follow the `DbProbeRequest`
  `[REDACTED]` pattern in every readout; `run_intercept_session()` runs a
  real timed listener with a documented per-exchange capture limitation.
- Maturity promotions: none (browser/daemon/proxy stay provisional;
  per-area remaining gaps recorded in `docs/python/domain-maturity.md`).
- Drive-by corrections required to verify: stale `host_auth.rs` RBAC tests
  updated to the current `SessionAccess`/`ClientRole` API (pre-existing
  `--lib` compile failure on main); `tool-api`-without-`cli` feature combo
  fixed (`parking_lot::Mutex` import gate in `recon/mod.rs` — the combo
  never compiled); nested `block_on` in `engine.rs::run_nse_inner` fixed
  (all full-feature `nse_run` contract tests panicked); stale
  `async_daemon_subscribe` test call fixed with a session id.
- Skips/blockers: live CONNECT/WebSocket/HTTP-2 flow capture into reports
  and binding-level exchange capture remain open (proxy); managed
  `BrowserSession` has no bound tab driver (browser); event replay stays
  state-based by design, no event log (daemon). Full-feature local profile
  (`nse,web-proxy,db-pentest,mobile,headless-browser,daemon-client`) is
  green except pre-existing skips (no Chrome binary, no emulator, daemon
  tests needing extra fixtures); routine CI uses the default-feature build.
- Verification: `make check` (exit 0), `make check-python` (exit 0),
  `cargo test -p eggsec-daemon` (74 passed), `cargo test
  -p eggsec-daemon-protocol` (72 passed), `cargo test -p eggsec-web-proxy`
  (426 passed), full-feature pytest for browser/proxy/network/daemon areas
  (753 passed; 2 new `GetTaskResult` socket tests green against a spawned
  daemon binary).