# Eggsec Python API High-Value Roadmap

## Purpose

This roadmap moves `eggsec-python` from a broad collection of PyO3 wrappers into a coherent, stable, Python-native security assessment framework. Existing bindings already cover many individual tools; the highest-value remaining work is architectural: orchestration, policy, lifecycle, eventing, persistence, and extensibility.

The target is not one Python wrapper per CLI command. The target is a single execution model under which Eggsec capabilities can be discovered, authorized, composed, executed, observed, cancelled, resumed, persisted, and integrated.

## Architectural principles

- Rust remains responsible for networking, concurrency, parsing, enforcement, and performance-sensitive execution.
- Python owns composition, configuration, orchestration, integration, and user-facing application logic.
- Async is canonical; sync is a complete façade over the same request/result path.
- Scope and authorization semantics remain equivalent across Rust library, CLI/TUI, daemon, MCP/agent, and Python.
- Public Python types are stable DTOs and protocols rather than direct mirrors of internal Rust structures.
- Long-running operations support progress, cancellation, timeout, partial results, deterministic cleanup, and resumability.
- Feature-gated capabilities are discoverable at runtime and fail with structured capability errors.
- All domains converge on common findings, evidence, artifacts, status, timing, and audit primitives.

## Milestone sequence

### Milestone A — Unified engine and operation model

Introduce `Engine` and `AsyncEngine`, typed operation requests, common results, execution handles, cancellation, progress events, pipelines, planning, preflight hooks, and checkpoints. Existing convenience functions should delegate through this path.

### Milestone B — Policy, configuration, and execution context

Expose complete configuration loading, scope explanations, operation metadata, execution context, authorization policy, override grants, preflight decisions, audit events, credential providers, and secret-safe serialization.

### Milestone C — Core missing assessment domains

Bind consolidated reconnaissance, GraphQL, OAuth/OIDC, authentication testing, headless-browser assessment, and advanced hunting through the engine/policy model.

### Milestone D — Complete partial domain bindings

Complete NSE runtime access, live packet inspection, ICMP/traceroute/raw packet operations, interception proxy lifecycle, mobile dynamic analysis, daemon task APIs, and database extensibility.

### Milestone E — Findings lifecycle, reporting, storage, and integrations

Finalize the finding/evidence/artifact schema, CVSS and vulnerability management, workflow state, persistence, baseline comparison, reporting formats, compliance mappings, and external integrations.

### Milestone F — Specialized and lab-oriented domains

Expose wireless, evasion validation, post-exploitation simulation, C2 simulation, distributed scanning, remote execution, notifications, and structured AI post-processing under explicit policy gates.

### Milestone G — Extensibility and API stabilization

Finalize the operation registry, event protocol, callback/sink contracts, Python ergonomics, buffer efficiency, public API governance, documentation, wheel matrix, compatibility checks, and 1.0 release hardening.

## Delivery tranches

### Tranche 1: framework foundation

1. Engine/AsyncEngine.
2. Common requests/results/status.
3. Planning and preflight.
4. Execution context and authorization.
5. Cancellation and event streaming.
6. Pipeline composition and checkpoints.
7. Migration of existing convenience functions.

### Tranche 2: ordinary assessment coverage

1. Consolidated recon.
2. GraphQL.
3. OAuth/OIDC.
4. Authentication.
5. Browser testing.
6. Daemon task execution.

### Tranche 3: deep domain parity

1. NSE completion.
2. Interception proxy.
3. Live packet capture and probing.
4. Mobile dynamic analysis.
5. Database extensibility.
6. Finding lifecycle and reporting.

### Tranche 4: platform completion and stabilization

1. Storage and integrations.
2. Wireless and lab domains.
3. Distributed and remote execution.
4. AI analysis.
5. Extensibility contracts.
6. API stabilization and 1.0 readiness.

## Cross-cutting test strategy

Every operation must satisfy contract tests for request validation, planning, preflight, scope denial, feature-unavailable behavior, sync execution, async execution, cancellation, timeout, result serialization, finding schema compliance, audit emission, and cleanup.

Policy-equivalence tests should compare Rust library, Python local engine, daemon execution, strict CLI, MCP, and agent decisions for equivalent inputs. Feature-matrix tests should confirm compiled exports, type stubs, `features()`, `has_feature()`, and operation availability remain synchronized.

Dedicated fixtures should cover HTTP, TLS, GraphQL, OAuth/OIDC, authentication, WebSocket, databases, containers, NSE, proxy interception, and daemon transport. Privileged tests should run only in explicit CI jobs.

## Completion definition

The roadmap is substantially complete when Python applications can load configuration and scope, inspect capabilities, plan and preflight work, execute locally or through the daemon, compose pipelines, stream progress and findings, cancel and resume work, preserve frontend-equivalent authorization, persist and compare assessments, generate standard reports, and access all major Eggsec domains through consistent request/result models.