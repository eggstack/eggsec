# Python API execution contract

`eggsec-python` is a host-language binding over the Rust engine. Its scoped
pre-1.0 stable core is intentionally smaller than the importable package.

## Stable dispatch

The twenty-two stable engine operations are represented by the Rust
`StableOperation` enum in `crates/eggsec-python/src/operation_registry.rs`.
`Engine.run()` and `AsyncEngine.run()` parse the same enum before dispatch, so
unknown IDs cannot silently reach a domain executor. Historical aliases are
accepted only as compatibility inputs and resolve to the same enum variant.

## Mandatory gate

Every stable engine operation evaluates an `eggsec::config::EnforcementContext`
before entering a domain executor. The gate evaluates scope, risk, required
features, capabilities, and execution profile. It records an allow, warning,
confirmation, or deny decision in the engine’s structured audit sink. Tracing
is supplemental and is not the audit contract.

Successful results include `policy_decision=allow` and the policy schema
version in metadata. Failed results carry `OperationError`, which includes a
schema version, kind, code, operation ID, retryability, denial class, source,
details, and causes. `error_message` is a compatibility view.

## Event delivery

`EventEnvelope.sequence` is monotonic for events produced by one stream.
`BackpressureChannel` has a bounded best-effort queue for progress and a
reliable queue for planning, preflight, stage lifecycle, finding, artifact,
cancellation, failure, and completion events. `EventDeliveryStats` reports
emission, delivery, drops by event kind, maximum depth, consumer lag, and
terminal delivery failures. The guarantee is exactly-once within the in-process
queue model. Daemon replay is deliberately outside the first-release stable
contract.

## Release boundary and daemon deferral

The first public `0.x` release guarantees the twenty-two stable operations through
local `Engine` and `AsyncEngine` execution. The optional `daemon-client`
feature remains provisional. Its APIs are available for integration testing,
but no release documentation should describe daemon execution as stable until
the follow-up daemon parity milestone closes request normalization, stable
operation identity, policy/audit parity, structured errors, payload schemas,
cancellation, timeouts, reconnect/result retrieval, event replay, and artifact
metadata.

## Checkpoint contract

Stable-core pipeline checkpoints use schema version 3. A checkpoint records
the operation schema, target-set hash, scope hash, execution profile, enabled
feature-set hash, pipeline-definition hash, and artifact-store identity. A
resume rejects any mismatch with a structured `checkpoint_incompatible` error.
Persisted checkpoints are written to a sibling temporary file, flushed, and
atomically renamed. Sensitive-key fields are redacted before in-memory or
on-disk serialization; secrets are never required to resume a pipeline.

## Secret handling

The stable-core request DTOs do not accept credentials. Secret-bearing
provisional domains must use `SensitiveString` at their public boundary and
must not place raw values in repr, event envelopes, reports, or checkpoints.
Checkpoint redaction recursively covers keys such as `authorization`,
`password`, `token`, `secret`, `client_secret`, and `api_key`; release tests
assert that unique sentinels are absent from serialized JSON and reloaded
checkpoints. `SensitiveString.expose_secret()` is an explicit manual escape
hatch and is not used by stable-core dispatch.

## Domain boundary

Use Python `domain_maturity()` for whole-domain state and `api_surface()` for
individual symbol state. A Cargo feature only controls compilation. It does
not promote a domain to stable-core. See
[`docs/python/domain-maturity.md`](../docs/python/domain-maturity.md).

## Validation and Evidence Infrastructure

Release 1-4 closure introduced a profile-based validation system that provides
structured evidence for maturity classifications. The 20 validation profiles
(each defined in `crates/eggsec-python/validation/profiles.json`) produce
evidence JSON with test counts, skip budgets, wheel metadata, platform info,
and toolchain versions.

Maturity classifications are now derived from profile evidence rather than
hand-maintained checklists. Skip budget enforcement prevents silent test suite
erosion: each profile declares minimum test counts, maximum allowed
skips/xfails, and per-reason skip budgets (e.g., `feature_gate`,
`network_error`).

Key scripts:
- `scripts/run_python_profile.py` — runs a single profile end-to-end
- `scripts/build_python_release_evidence.py` — aggregates profile results into
  a release evidence bundle
- `scripts/python_skip_budget.py` — standalone skip budget enforcement
- `scripts/validate_python_profiles.py` — validates the profile manifest

## Release 4: Common Session Contract and Daemon Parity

Release 4 establishes a common managed-session contract for mobile and browser
subsystems, advances daemon parity, and introduces repository and artifact
storage abstractions.

### Common session contract

Mobile and browser subsystems share a unified `SessionState` lifecycle and
`SessionIdentity` metadata model. `MobileSession` and `BrowserSession` both
implement the common contract, providing consistent state transitions, event
delivery, and cleanup semantics.

### Daemon parity protocol

Release 4 closes gaps between local and daemon execution:

- **Idempotent request submission** with deduplication keys for safe retries.
- **Reconnect and replay** semantics for sessions interrupted by transport
  failures.
- **Cancellation propagation** across transport boundaries with structured
  cancellation events.

### Repository abstraction

`SessionRepository` provides a content-addressed storage interface for session
state. Implementations include `SQLiteSessionRepository` (persistent) and
`InMemorySessionRepository` (ephemeral). The repository abstraction decouples
session lifecycle from transport and storage backends.

### Content-addressed artifact stores

`ArtifactStore` and `DirectoryArtifactStore` provide content-addressed storage
for scan artifacts. Artifacts are addressed by content hash, enabling
deduplication and integrity verification.

### Streaming reporting

`StreamingReporter` supports incremental report generation during long-running
sessions. `ReportDiff` compares report snapshots for delta reporting and
trend analysis.

### Stability classification

All Release 4 types are **provisional**. They follow engine conventions but
do not yet satisfy the graduation checklist in
`docs/python/domain-maturity.md` for stable-core promotion. Releases 1-3
stable-core guarantees remain intact.

## Shared Async Runtime Ownership

All async Python operations share a single process-global Tokio runtime
(`OnceLock<Runtime>` in `runtime_async.rs`) with a 2-worker-thread pool.
This replaces the previous per-`PyFuture` runtime pattern where each
`spawn_async` call created and destroyed its own `new_current_thread()`
runtime on a dedicated thread.

### Guarantees

- Stateful resources created in one awaited call remain valid for subsequent calls.
- `connect()` → `write()` → `read()` → `close()` chains work on the same session.
- Dropping a `PyFuture` does not drop the session runtime (shared).
- Concurrent sessions share the runtime without global serialization.
- Sync wrappers (`block_on`) use the separate `OnceLock` runtime in
  `runtime_sync.rs` and release the GIL during I/O.

### Affected APIs

`AsyncTcpSession`, `AsyncUdpSocket`, `AsyncHttpClient`, `AsyncWebSocketSession`,
`AsyncCaptureSession`, `AsyncMobileSession`, `AsyncBrowserSession`, daemon-backed
`AsyncEngine`, async proxy and database sessions, and all one-shot async
functions (recon, scanning, WAF, etc.) returning `PyFuture`.

## Release 5 Phase A: Tool Integration

Release 5 Phase A bridges `eggsec-tool-core` types to Python, establishing a
deterministic tool abstraction layer for all 22 stable operations.

### eggsec-tool-core exposure

`eggsec-tool-core` is a standalone crate with no engine dependencies. Its
types are pure data DTOs (request, response, finding, error, rate-limit,
scope, target). Release 5 Phase A binds all public types to Python via
`crates/eggsec-python/src/tool_core.rs`.

The binding follows existing conventions: `#[pyclass(frozen)]`, `to_dict()`,
`to_json()`, `__repr__`, `__str__`, `__hash__`. Credentials in `AuthConfig`
are redacted in all repr and serialization paths.

### Tool descriptor generation from OperationMetadata

Each of the 22 stable operations has a `ToolDescriptor` generated from
`OperationMetadata`. The descriptor captures:

- Canonical tool ID and human-readable label
- Supported target types (IP, domain, URL, CIDR, file)
- Parameter and result JSON Schema (generated from Rust types)
- Risk classification (from `OperationMetadata.default_risk`)
- Required features and supported execution surfaces

`ToolRegistry` provides static lookup: `find(tool_id)`,
`find_by_operation(operation_id)`, `all_tools()`.

### invoke_tool dispatch flow

```
Python: Engine.invoke_tool(ToolRequest)
  → ToolRegistry.find(request.tool)
  → EnforcementContext.evaluate(descriptor)
  → ToolRequest → OperationDescriptor conversion
  → eggsec::dispatch::dispatch_inner()
  → ToolResponse construction from OperationResult
```

The `invoke_tool` path is identical for all operations. The tool ID resolves
to an operation via `ToolRegistry`, and the engine dispatches through the
standard `EnforcementContext` → `EnforcedDispatcher` path. The response is
wrapped in `ToolResponse` with status, findings, errors, and metadata.

### Schema generation architecture

`SchemaGenerator` produces JSON Schema from Rust type metadata. Request
schemas describe the parameter structure; response schemas describe the
result payload. The full manifest covers all 22 stable operations and is
useful for API documentation, validation, and code generation tools.

Schema generation uses `schemars` (or equivalent) at compile time or
runtime to derive schemas from the Rust struct definitions.

## Release 2: Network Programmability

Release 2 introduces Python bindings for low-level network primitives. These
types live in `eggsec.network`, `eggsec.transport`, `eggsec.probes`,
`eggsec.http_client`, and `eggsec.websocket`.

### Enforcement posture

All Release 2 network operations pass through the engine's enforcement model:

- **Low-level network primitives** (`TargetPy`, `ConnectionConfigPy`,
  `RetryPolicyPy`, etc.) are scope-checked at construction time. Target
  resolution validates against `LoadedScope` before DNS or TCP contact.
- **TCP/UDP sessions** (`TcpSessionPy`, `UdpSocketPy`) use the existing
  `EnforcementContext` via the transport layer. Session creation evaluates
  scope and risk; connection attempts to out-of-scope targets raise
  `EnforcementError`.
- **HTTP client requests** (`HttpClientPy`, `AsyncHttpClientPy`) pass through
  the policy gate for each request. The client enforces scope on the target
  authority, records TLS metadata for audit, and redacts sensitive headers
  from transcripts and reports.
- **WebSocket assessments** (`websocket_assess()`) are canonical operations
  with a policy check, structured events, and cooperative cancellation. They
  follow the same dispatch contract as stable-core operations but are
  classified provisional until graduation criteria are met.
- **Raw packet injection** remains experimental (feature: `packet-inspection`).
  It is not exposed through the Python bindings in the default wheel and
  requires an explicit feature flag and elevated scope.

### Stability classification

Release 2 network types are **provisional**. The public API shape is useful
and follows engine conventions (frozen pyclasses, `to_dict`/`to_json`,
context managers for sessions), but they do not yet satisfy the graduation
checklist in `docs/python/domain-maturity.md` for stable-core promotion.
The 2026-07-14 release closure pass (1977 passed, 89 skipped) verified that
all Release 2 network/transport/probe configuration types are properly
registered in the API surface with provisional stability. Use
`api_surface()` to verify stability before relying on these types in
compatibility-sensitive automation.
