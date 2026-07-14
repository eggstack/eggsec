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
