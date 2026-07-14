# Eggsec Python API Release 3 — Major Subsystem Completion

## Handoff objective

Release 3 completes the programmable Python surfaces for three major Eggsec subsystems that currently expose partial, one-shot, or CLI-shaped bindings:

- NSE runtime and script execution;
- interception proxy lifecycle and HTTP replay;
- stateful database assessment sessions.

The objective is not to add more isolated wrappers. It is to expose durable lifecycle APIs, typed state, streaming, cancellation, evidence, artifacts, policy enforcement, and engine integration for these domains.

Release 3 should begin only after the Release 1/2 closure pass has resolved unexplained failures, published the canonical validation matrix, clarified maturity for all 22 operations, tightened registry/executor architecture, and established live transition and cleanup evidence.

## Release outcome

At completion, Python users can:

- inspect, configure, and execute NSE scripts through a managed runtime;
- register and override NSE libraries safely;
- inspect metadata, dependencies, categories, and rule behavior;
- start and stop an interception proxy session;
- stream captured HTTP and WebSocket exchanges;
- filter, mutate, replay, compare, and export traffic;
- manage interception CA and certificate material safely;
- open reusable database assessment sessions;
- inspect schemas, versions, privileges, and capabilities;
- run controlled read-only queries with bounds, timeouts, cancellation, and evidence capture;
- compose all three subsystems with common policy, events, findings, artifacts, pipelines, and checkpoints.

## Scope

### Mandatory domains

1. NSE runtime completion;
2. interception proxy lifecycle;
3. HTTP replay and comparison;
4. stateful database session API;
5. common lifecycle, event, cancellation, timeout, and artifact integration;
6. operation registry integration where subsystem actions are canonical operations;
7. typing, documentation, fixtures, packaging, and release validation.

### Conditional domains

Python-implemented NSE libraries, unrestricted mutation callbacks, destructive database operations, database exploitation helpers, and malformed WebSocket frame mutation remain provisional or experimental unless portability and enforcement semantics are fully defined.

### Explicit non-goals

Release 3 does not complete mobile dynamic sessions, browser lifecycle, daemon parity, persistent repositories, public 1.0 stabilization, unrestricted database writes, or general-purpose proxying unrelated to assessment workflows.

## Architectural principles

- Expose explicit lifecycle objects: `NseRuntime`, `InterceptSession`, and `DatabaseSession`.
- Use canonical engine operations for bounded assessments and managed sessions for reusable state.
- Opening sessions, loading scripts, replaying requests, and executing queries must not bypass scope, policy, audit, feature, privilege, or risk classification.
- Do not expose native runtime handles, internal proxy actors, raw driver connections, or Tokio task types.
- Sync and async APIs must share semantics and deterministic cleanup.

## Workstream 1 — NSE capability inventory and runtime boundary

Document the current parser, compiler/interpreter boundary, metadata, categories, dependencies, rules, libraries, arguments, target context, sandbox controls, instruction/memory accounting, timeout, cancellation, output conversion, and CLI coupling.

Deliver:

- `docs/python/NSE_RUNTIME_ARCHITECTURE.md`;
- capability manifest entries separating one-shot `nse_run`, runtime lifecycle, script inspection, library registration, rule evaluation, and experimental callback extensions.

## Workstream 2 — NSE runtime lifecycle

### Proposed API

- `NseRuntimeConfig`
- `NseRuntime`
- `AsyncNseRuntime`
- `NseRuntimeStats`
- `NseRuntimeCapabilities`
- `NseSandboxPolicy`

### Required behavior

- explicit construction and feature checks;
- sync and async context managers;
- deterministic close;
- runtime reuse;
- bounded concurrent execution;
- default and per-script timeout;
- cancellation;
- instruction, memory, and output-size limits;
- library registry identity;
- events and audit identity;
- no cross-session mutable global state.

## Workstream 3 — NSE script inspection

### Proposed types

- `NseScript`
- `NseScriptSource`
- `NseScriptMetadata`
- `NseScriptCategory`
- `NseScriptDependency`
- `NseDiagnostic`
- `NseCompileResult`

Support loading from path/source, metadata extraction, categories, dependencies, syntax validation, preparation without execution, source hashes, structured diagnostics, safe malformed-input handling, and allowed-root path enforcement.

## Workstream 4 — NSE rule evaluation

### Proposed types

- `NseTargetContext`
- `NsePortContext`
- `NseHostContext`
- `NseRuleResult`

Support separate deterministic evaluation of `portrule`, `hostrule`, and `postrule`, including an explanation of why a rule matched or failed. Pure rule evaluation must not perform network contact.

## Workstream 5 — NSE arguments and target context

Provide typed string, integer, boolean, list, and map arguments; secret references; deterministic precedence; secret-safe serialization; and a target context containing hostnames, resolved addresses, ports, services, TLS state, and evidence references.

## Workstream 6 — NSE library registry

### Proposed API

- `NseLibraryRegistry`
- `NseLibraryDescriptor`
- `NseLibraryVersion`
- `NseLibraryConflict`

Support built-in inspection, approved native library registration, explicit overrides, compatibility validation, per-runtime isolation, dependency diagnostics, and registry identity for checkpoints.

Python callbacks, if supported, remain local-only provisional extensions with strict timeouts and exception mapping.

## Workstream 7 — NSE execution and structured output

### Proposed types

- `NseExecutionRequest`
- `NseExecutionResult`
- `NseScriptResult`
- `NseOutputValue`

Support one script or script sets, category filters, dependency ordering, bounded concurrency, partial results, script/runtime timeouts, cancellation, diagnostics, structured output trees, compatibility text rendering, finding conversion, and artifact references.

Refactor `nse_run` to delegate through this runtime rather than preserving separate one-shot semantics.

## Workstream 8 — NSE validation fixtures

Add fixtures for metadata, rule match/non-match, dependency chains, missing dependencies, instruction limits, memory limits, timeouts, cancellation, structured output, findings, malformed scripts, and secret arguments. Document supported NSE compatibility and intentional deviations from Nmap.

## Workstream 9 — Interception proxy architecture boundary

Document listener lifecycle, upstream connections, CONNECT, TLS interception, certificate issuance, request/response capture, WebSocket upgrades, mutation hooks, replay, HAR, limits, filters, and CLI assumptions.

Deliver `docs/python/INTERCEPTION_PROXY_ARCHITECTURE.md`.

## Workstream 10 — Interception session lifecycle

### Proposed API

- `InterceptConfig`
- `InterceptSession`
- `AsyncInterceptSession`
- `InterceptState`
- `InterceptStats`
- `InterceptEndpoint`

Support bind/start/stop, sync and async context managers, actual bound port, upstream configuration, proxy chaining, cancellation, graceful drain, forced stop, connection/exchange counts, byte counts, bounded storage, deterministic cleanup, and structured bind/privilege errors.

## Workstream 11 — Captured exchange streaming

### Proposed types

- `CapturedExchange`
- `CapturedRequest`
- `CapturedResponse`
- `CapturedBody`
- `ExchangeTiming`
- `ExchangeTlsMetadata`
- `ExchangeStream`

Support sync/async iteration, bounded queues, backpressure, host/path/method/status/content filters, raw ordered headers, normalized lookup, body truncation metadata, artifact externalization, WebSocket upgrade metadata, monotonic sequence, and default redaction.

## Workstream 12 — Filtering and mutation

### Proposed types

- `InterceptFilter`
- `RequestMutation`
- `ResponseMutation`
- `MutationDecision`
- `MutationError`

Support declarative header, query, and bounded body mutation, synthetic responses, pass-through, explicit drop/deny, mutation timeout, and callback error handling.

Prefer portable declarative mutations. Python callbacks are local-only provisional extensions and must not be represented as daemon-portable.

## Workstream 13 — CA and certificate management

### Proposed API

- `CertificateAuthorityConfig`
- `CertificateAuthority`
- `IssuedCertificate`
- `CertificateStore`

Support CA initialization/loading, safe paths and permissions, leaf issuance, hostname/IP SANs, cache/expiry, metadata, and deterministic temporary cleanup. Private keys must never appear in repr, events, reports, checkpoints, or default JSON. Do not modify system trust automatically.

## Workstream 14 — HTTP replay and comparison

### Proposed types

- `ReplayRequest`
- `ReplayResult`
- `ResponseComparison`
- `ComparisonRule`
- `HarDocument`

Support captured-request replay, scoped destination edits, explicit auth preservation/stripping, status/header/body/JSON/timing comparison, volatile-header normalization, redaction, HAR import/export, artifact-backed bodies, timeout, and cancellation.

Consider canonical operations such as `proxy_capture_assess` and `http_replay_compare` only after full operation graduation.

## Workstream 15 — WebSocket interception

Capture upgrade metadata, text/binary messages, direction, close state, bounded sizes, transcript redaction, and optional declarative message filters. Malformed-frame generation remains experimental.

## Workstream 16 — Database driver registry

### Proposed types

- `DatabaseDriverRegistry`
- `DatabaseDriverDescriptor`
- `DatabaseCapabilities`
- `DatabaseTarget`
- `DatabaseSessionConfig`

Expose driver ID, protocol, ports, auth modes, TLS, schema/privilege/transaction support, parameter style, cancellation support, and dependency/platform requirements. Feature discovery must match compiled drivers.

## Workstream 17 — Database session lifecycle

### Proposed API

- `DatabaseSession`
- `AsyncDatabaseSession`
- `DatabaseSessionState`
- `DatabaseConnectionMetadata`
- `DatabaseSessionStats`

Support connect/authenticate, TLS metadata, sync/async context managers, reconnect policy, idle/statement timeouts, cancellation, transaction state, read-only default, deterministic close, audit events, and secret-safe credentials.

## Workstream 18 — Credential provider protocol

### Proposed API

- `DatabaseCredentialProvider`
- `StaticCredentialProvider`
- `EnvironmentCredentialProvider`
- `CallbackCredentialProvider`
- `CredentialRequest`
- `CredentialResult`

Use explicit secret types, no secret serialization, bounded callback execution, provider identity without secret material, refresh behavior, and structured failures. Callback providers remain local-only provisional extensions.

## Workstream 19 — Controlled query execution

### Proposed types

- `DatabaseQuery`
- `DatabaseQueryResult`
- `DatabaseColumn`
- `DatabaseRowStream`
- `DatabaseQueryPlan`

Support parameterized queries, read-only enforcement, row/byte limits, statement timeout, cancellation, streamed rows, truncation metadata, transactions, query evidence, and redacted audit representations.

Writes, DDL, and administrative operations require separate elevated-risk request types and remain disabled by default.

## Workstream 20 — Schema, version, and privilege inspection

Expose server version, databases, schemas, tables/views, columns, indexes, current user, roles, effective privileges, security settings, and supported extension/plugin inventories using driver-neutral DTOs with namespaced driver-specific metadata.

## Workstream 21 — Database evidence and finding conversion

Convert version, schema, privilege, and configuration observations into evidence and explicit findings. Never store credentials. Externalize large inventories. Refactor `db_probe` to use the common driver registry and session implementation.

## Workstream 22 — Common session events

Add or reuse events for session creation/start/close, connections, authentication, scripts, exchanges, replay, queries, findings, artifacts, cancellation, and failures. Preserve monotonic sequence, correlations, reliable terminal events, backpressure accounting, and secret exclusion.

## Workstream 23 — Pipeline and checkpoint integration

Allow portable pipelines to compose bounded NSE, proxy, replay, and database workflows. Persist definitions, identities, results, and artifact references—not native handles, open sockets, private keys, or credentials. Resume must reconstruct sessions.

## Workstream 24 — Typing and ergonomics

Provide complete stubs, sync/async context-manager protocols, iterators, async iterators, typed output unions, callback protocols, `pathlib.Path`, datetime timestamps, secret-safe repr, mypy/pyright examples, and deliberate namespace exports.

## Workstream 25 — Packaging profiles

Validate `nse`, `web-proxy`, `db-pentest`, and compatible combined profiles through wheel builds, installed-wheel smoke tests, feature manifests, binary-size reports, dependency diagnostics, and platform metadata.

## Workstream 26 — Documentation and examples

Required guides:

- NSE runtime, scripts, rules, libraries, and sandboxing;
- proxy lifecycle, CA handling, mutation, replay, HAR, and WebSocket capture;
- database sessions, credential providers, read-only queries, schemas, and privileges;
- cancellation, cleanup, packaging, and maturity.

Required executable examples include one bounded workflow for each major subsystem and evidence-to-finding conversion.

## Validation plan

### NSE

Validate parsing, metadata, rules, dependencies, runtime reuse, timeout, limits, cancellation, secret arguments, structured results, and installed wheels.

### Proxy

Validate listener lifecycle, HTTP capture, CONNECT, TLS interception, issuance, limits, redaction, mutations, replay, HAR, WebSocket upgrades, slow consumers, cancellation, and installed wheels.

### Database

Validate driver discovery, connection success/failure, TLS, auth, secret redaction, read-only behavior, parameterized queries, limits, timeout, cancellation, schema/privilege inspection, transaction cleanup, and installed wheels.

## Performance and resource budgets

Track NSE parse/runtime memory, proxy throughput/latency and queue memory, replay throughput, certificate cache behavior, database connection/query overhead, row-stream memory, cancellation latency, and repeated lifecycle leaks.

## Recommended implementation sequence

1. NSE architecture and runtime boundary;
2. NSE metadata, rules, arguments, libraries;
3. NSE execution and `nse_run` convergence;
4. proxy architecture and lifecycle;
5. capture streaming, CA, mutations, replay, HAR, WebSocket capture;
6. database driver registry and session lifecycle;
7. credential providers and controlled queries;
8. schema/version/privilege inspection;
9. evidence, events, pipelines, checkpoints;
10. typing, packaging, documentation, and closure.

## Release acceptance criteria

Release 3 is complete only when:

- NSE runtime lifecycle and programmable script inspection/execution are complete;
- `nse_run` delegates through the common runtime;
- proxy sessions can start, capture, filter, mutate, replay, export, and stop safely;
- CA material is secret-safe;
- HTTP/WebSocket capture is bounded and redacted;
- database sessions are reusable, cancellable, typed, and read-only by default;
- driver discovery, credentials, queries, schema, and privilege inspection are available;
- `db_probe` delegates through common session primitives;
- all session APIs emit governed events and integrate with evidence/artifacts;
- pipelines and checkpoints compose subsystem workflows without persisting live handles or secrets;
- feature-specific installed wheels pass;
- maturity documentation remains accurate;
- Release 1/2 guarantees remain intact.