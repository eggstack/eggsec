# Python API Milestone D — Complete Partial Domain Bindings

## Goal

Bring already-bound domains to meaningful parity with their Rust implementations. This milestone focuses on NSE, packet inspection and network probing, interception proxying, mobile dynamic analysis, daemon task execution, and database extensibility.

## Dependencies

- Milestones A and B are mandatory.
- Milestone C is not strictly required, but shared namespace and documentation conventions should be reused.
- Feature and privilege discovery must be available before live capture, raw packet, proxy interception, mobile dynamic, or daemon full-executor work lands.

## Workstream D1 — NSE runtime completion

Expose `NseRuntime`, `NseScript`, `NseScriptMetadata`, categories, arguments, target context, rule evaluations, execution events, and sandbox policy.

Support script discovery, metadata inspection, category filtering, custom search paths, argument validation, host/port context construction, host/port rule evaluation, library resolution, per-script timeout, cancellation, multi-target scheduling, structured logs, sandbox selection, and SSH2 capability reporting.

Ensure custom script execution follows scope/policy and does not bypass the canonical NSE runtime safeguards.

## Workstream D2 — Live packet inspection

Add `CaptureSession`, `PacketStream`, `PacketFilter`, `FlowRecord`, protocol-decoder metadata, and `LiveCaptureResult`.

Support lifecycle management, bounded async iteration, filters, backpressure, dropped-packet statistics, flow aggregation, optional PCAP output, cancellation, and interface/privilege diagnostics.

Avoid per-packet Python callbacks by default. Prefer Rust-side filtering/aggregation and bounded sampled delivery.

## Workstream D3 — Network probing and packet operations

Bind structured APIs for ICMP echo, traceroute, packet construction, transmission, response collection, rate controls, and privilege checks.

Raw transmission and hazardous rate settings require explicit high-risk grants. Planning/preflight must state required OS capabilities and provide actionable failure diagnostics.

## Workstream D4 — Interception proxy

Separate proxy-pool rotation from traffic interception. Add `InterceptProxy`, `AsyncInterceptProxy`, configuration, certificate authority, session, captured exchange, WebSocket frame, proxy event, mutation hook, and replay request types.

Support HTTP/HTTPS/WebSocket lifecycle, CA generation/loading, captured traffic streams, bounded mutation hooks, replay, export, transparent-proxy feature discovery, and dynamic-plugin discovery where supported.

Traffic interception requires explicit authorization and complete audit records. Ensure listeners, temporary certificates, and streams shut down deterministically.

## Workstream D5 — Mobile dynamic analysis

Feature-gated types should include `MobileDevice`, `MobileRuntimeSession`, `AdbSession`, `LogcatStream`, `FridaSession`, dynamic request/result types, and runtime artifacts.

Support device discovery, install/launch/stop, log streaming, runtime filesystem/network observations, Frida script lifecycle, artifact collection, cancellation, and cleanup. Distinguish static and dynamic capabilities clearly in feature introspection.

## Workstream D6 — Daemon task API

Expand the daemon client with capabilities, session, task request, task handle, status, result, event stream, artifact retrieval, attach/detach, cancellation, reconnection, and transport metadata.

Local and daemon execution should share operation request/result schemas. Callers should be able to select backend without rewriting assessment definitions.

Expose conservative/full/no-op capability profiles and reject unsupported task kinds before submission.

## Workstream D7 — Database extensibility

Add driver enumeration, capability descriptors, backend-specific configuration, test selection, read-only/transaction guards, credential-provider integration, custom checks, structured query evidence, and backend detail types.

Do not expose raw driver objects. Keep a stable Eggsec-level interface across PostgreSQL, MySQL, MSSQL, MongoDB, and Redis.

## Workstream D8 — Cross-domain lifecycle standards

All session-oriented APIs must implement context managers, explicit `close()`, idempotent shutdown, cancellation propagation, status inspection, and finalizer warnings for leaked resources.

## Testing

- NSE discovery, metadata, sandbox, timeout, and cancellation fixtures.
- Packet backpressure/drop accounting and bounded-memory tests.
- Privilege-diagnostic tests on supported and unsupported platforms.
- Proxy TLS/interception/replay fixtures with cleanup validation.
- Mobile dynamic mocked tests plus dedicated hardware/emulator CI where available.
- Daemon local transport, reconnect, cancellation, profile, and artifact tests.
- Database driver capability and credential-redaction tests.

## Acceptance criteria

- NSE scripts can be discovered, configured, executed, observed, cancelled, and sandboxed.
- Live capture supports bounded async iteration without unbounded memory growth.
- Interception proxy and mobile sessions clean up reliably under exceptions and cancellation.
- Daemon tasks use the same request/result models as local execution.
- Privileged operations provide preflight diagnostics before execution.
- Hazardous capabilities require explicit policy grants and emit audit events.

## Risks

- GIL pressure from high-volume streams: aggregate/filter in Rust and bound delivery.
- Platform-specific packet behavior: maintain explicit capability matrices.
- Proxy mutation callbacks can block traffic: define timeouts and fail-open/fail-closed policy.
- Mobile tooling is externally stateful: design idempotent cleanup and recovery.
- Daemon schema drift: share canonical DTO definitions or generated schemas.

## Handoff notes

Land daemon schema alignment early because it influences request/result design. Implement NSE and database extensions as lower-risk validation tracks, then packet capture/probing, proxy interception, and mobile dynamic analysis in dedicated feature-gated passes.