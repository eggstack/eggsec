# Python API Milestone B — Policy, Configuration, and Execution Context

## Goal

Expose Eggsec’s complete configuration, scope, policy, authorization, audit, and secret-handling model to Python. This milestone prevents Python from becoming a lower-governance execution path and establishes prerequisites for hazardous and interception-oriented domains.

## Dependencies

- Milestone A engine/request/result model.
- Rust `EggsecConfig`, scope evaluation, operation metadata, preflight, enforcement, audit, and authentication-context primitives.

## Workstream B1 — Configuration model

Expose `EggsecConfig` and stable nested configuration DTOs for runtime, network, resolver, HTTP, output, audit, storage, and domain settings.

Support `from_file`, `from_toml`, `from_yaml`, `from_dict`, `to_dict(redacted=True)`, and validation diagnostics with precise field paths. Decide between immutable validated objects or explicit builder/freeze semantics. Unknown-field and environment-expansion behavior must match Rust.

## Workstream B2 — Scope evaluation and explanation

Expand the current `Scope` wrapper with `ScopeRule`, `ScopeMatch`, `ScopeDecision`, `ScopeExplanation`, `ResolvedTarget`, and `TargetExpansion`.

Expose `scope.evaluate(target)` and `scope.explain(target)` without sending assessment traffic. Report matched allow/deny/exclusion rules, hostname/CIDR source, canonicalization, resolved addresses, private/loopback status, redirect boundaries, and required overrides.

## Workstream B3 — Operation metadata and capabilities

Expose `OperationDescriptor`, `RiskLevel`, `IntendedUse`, `OperationMode`, `CapabilityRequirement`, and `PrivilegeRequirement`.

Provide `engine.operations()`, `engine.operation(id)`, and `engine.can_execute(request)`. Metadata should include stable ID, domain, risk, required features, privilege/platform requirements, supported backends, credential/interception behavior, target-expansion behavior, and availability reason.

## Workstream B4 — Execution context

Add `ExecutionContext` containing operation mode, intended use, scope, authorization policy, actor/client/session identity, correlation ID, audit metadata, environment classification, and justification.

The engine should carry this context. Per-request overrides must be narrow, explicit, and rejected where frontend policy forbids them.

## Workstream B5 — Authorization policy

Add structured `AuthorizationPolicy`, `AuthorizationDecision`, `OverrideRequest`, `OverrideGrant`, and `OverrideReason` types.

Represent existing controls independently: out-of-scope, explicit exclusion, high risk, database pentesting, traffic interception, nonbaseline capabilities, private resolution, cross-host redirects, raw packet/stress, remote execution, and credential-bearing operations.

Manual grants must be rejected or ignored in strict, CI, MCP, and agent contexts according to Rust policy.

## Workstream B6 — Preflight

Implement `engine.preflight(request_or_assessment)`. Return allowed/denied/confirmation-required state, scope decision, risk decision, capability and privilege checks, missing features, required grants, resolution/redirect concerns, audit implications, and remediation guidance.

Ensure preflight is serializable and deterministic enough for CI policy gates.

## Workstream B7 — Audit APIs

Expose `AuditEvent`, `EnforcementAuditEvent`, `ScopeAudit`, `ManualOverrideAudit`, `AuditOutcome`, and an `AuditSink` protocol.

Provide null, in-memory, JSONL, Python logging, and guarded callback sinks. Specify callback threading, ordering, exception handling, and shutdown semantics. Do not place Python callbacks in packet/request hot paths.

## Workstream B8 — Credentials and secrets

Add `CredentialProvider`, `SecretValue`, `SecretReference`, and `AuthenticationContext`.

Secret-bearing objects must redact values in `repr`, logs, exceptions, JSON, audit events, checkpoints, and daemon requests. Results must never serialize raw credentials. Define sync/async credential-provider contracts and safe GIL behavior.

## Workstream B9 — Policy equivalence

Build shared fixtures that compare equivalent decisions across Rust library calls, Python local engine, daemon execution, strict CLI, MCP, and agent paths. Differences must be explicit and documented.

## Testing

- Config parsing/round-trip tests for TOML, YAML, dict, and file paths.
- Unknown-field and invalid-path diagnostics.
- Scope matching for hostname, CIDR, exclusion, private resolution, and redirects.
- Authorization matrices across operation modes.
- Preflight determinism and serialization.
- Audit emission for allow, deny, override, cancellation, and failure.
- Secret-redaction property tests.
- Policy-equivalence integration tests.

## Acceptance criteria

- Python loads and validates all stable Eggsec configuration sections.
- Scope and policy can be explained without network execution.
- Every engine operation has discoverable metadata and capability status.
- High-risk operations require the same grants as Rust and CLI paths.
- Audit events cover all enforcement outcomes.
- Secrets cannot leak through standard object or serialization surfaces.
- Preflight output is suitable for CI use.

## Risks

- Policy drift: derive bindings from canonical Rust metadata where possible.
- Boolean explosion: centralize grants in typed policy objects.
- Secret leakage: add redaction tests to every serialization path.
- Callback deadlocks: isolate and bound callback delivery.

## Handoff notes

Implement configuration and metadata first, then execution context and scope decisions, then authorization/preflight, then audit and secret-provider interfaces. Do not bind additional hazardous domains before this milestone’s enforcement tests pass.