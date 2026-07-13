# Eggsec Python API Completion Roadmap

## Purpose

This roadmap begins after closure of the scoped Python stable-core release candidate. `eggsec-python` now has a credible execution kernel: ten stable local operations, typed request and payload models, structured errors, policy and audit enforcement, governed events, cancellation, checkpoints, type stubs, wheel validation, and explicit maturity metadata.

The remaining problem is not lack of bindings. The package already exposes a large portion of Eggsec through direct functions and PyO3 classes. The remaining work is to turn that broad but uneven surface into a coherent Python library in which major Eggsec capabilities are discoverable, composable, policy-governed, observable, cancellable, streamable, and available through one canonical execution model.

This roadmap therefore prioritizes API convergence and reusable primitives over adding more isolated top-level wrappers.

## Current baseline

The stable local `Engine` and `AsyncEngine` operation registry currently contains:

- `scan_ports`
- `scan_endpoints`
- `fingerprint_services`
- `recon_dns`
- `inspect_tls`
- `detect_technology`
- `detect_waf`
- `validate_waf`
- `fuzz_http`
- `load_test`

The repository also contains provisional or experimental Python bindings for consolidated reconnaissance, GraphQL, OAuth/OIDC, authentication testing, browser assessment, advanced hunting, Git-secret scanning, SBOM generation, database assessment, NSE, containers, mobile analysis, packet inspection, interception proxy support, wireless, evasion, post-exploitation, C2, distributed execution, notifications, compliance, external integrations, and AI post-processing.

Importability is not treated as completion. A domain is complete only when its intended public surface is correctly classified and, where appropriate, integrated into the common execution contract.

## Target architecture

The completed Python package should have four deliberate layers.

### Stable operation API

`Engine` and `AsyncEngine` are the canonical execution path for supported assessment operations. Stable operations share:

- canonical operation IDs;
- typed request DTOs;
- typed result payloads;
- structured errors;
- operation metadata and feature requirements;
- mandatory policy and scope enforcement;
- audit decisions;
- lifecycle and progress events;
- timeout and cancellation behavior;
- deterministic fixtures;
- sync/async semantic equivalence;
- type-stub and wheel coverage.

### Reusable low-level primitives

Python developers can construct custom assessment workflows using Eggsec transport, protocol, parsing, capture, session, evidence, and artifact primitives without invoking CLI-shaped wrappers or replacing Eggsec internals with unrelated Python libraries.

### Provisional domain API

Useful but not yet compatibility-stable domains remain available with explicit maturity metadata and documented limitations. Provisional APIs must not be described as stable merely because they compile into a wheel.

### Experimental lab API

Hazardous, platform-sensitive, provider-dependent, or rapidly evolving capabilities live under an explicit `eggsec.experimental` namespace or equivalent feature-scoped modules. The top-level namespace must not become a flat export of every Rust binding.

## Architectural principles

- Rust owns networking, protocol parsing, concurrency, enforcement, performance-sensitive execution, and deterministic cleanup.
- Python owns composition, orchestration, application integration, and user-facing workflow logic.
- Async execution is canonical; sync execution is a complete façade over the same request/result path.
- Convenience functions delegate through the common engine whenever the domain is an engine operation.
- Public Python types are durable DTOs and protocols rather than projections of internal Rust implementation details.
- Long-running operations expose progress, cancellation, timeouts, partial results, streaming, bounded buffering, and terminal-state guarantees.
- Feature-gated capabilities are discoverable at runtime and fail with structured capability errors.
- Stable operation semantics are transport-independent. Local and daemon execution should differ only where transport behavior is explicitly documented.
- Stable-core request, result, event, checkpoint, report, and artifact paths remain secret-safe.
- New functionality does not bypass the existing policy, scope, audit, or operation metadata architecture.

## Release sequence

## Release 1 — API convergence

Release 1 converts the most mature existing direct-function domains into canonical engine operations and upgrades pipelines to compose them reliably.

Primary scope:

- authoritative capability inventory and graduation manifest;
- promotion of mature provisional domains into the operation registry;
- canonical request and payload variants;
- direct-function delegation through the engine;
- generalized pipelines;
- native async pipeline execution;
- typed inter-step references;
- retries, failure policy, bounded parallelism, events, and checkpoints;
- namespace cleanup and typing improvements required by the promoted domains.

Initial promotion candidates:

- `run_consolidated_recon`
- `scan_git_secrets`
- `generate_sbom`
- `graphql_test`
- `oauth_test`
- `auth_test`
- `db_probe`
- `nse_run`
- `scan_docker_image`
- `scan_kubernetes`
- `analyze_apk`
- `analyze_ipa`

Browser and advanced-hunting operations should be promoted only where deterministic fixtures, cleanup, and platform requirements can meet the same contract.

Release 1 is complete when these domains no longer require separate orchestration conventions and all declared stable operations can participate in the same pipeline, event, policy, cancellation, serialization, and checkpoint framework.

## Release 2 — Network programmability

Release 2 exposes reusable network and protocol primitives so Python users can build custom security tooling on Eggsec internals rather than merely invoke predefined assessments.

Primary scope:

- transport/session primitives;
- TCP, UDP, DNS, TLS, HTTP, and banner probes;
- a security-oriented HTTP session substrate;
- complete WebSocket bindings;
- packet capture lifecycle and streaming;
- protocol-layer packet decoding;
- flow aggregation;
- controlled active probing;
- consistent evidence, timing, cancellation, and artifact behavior.

Release 2 is complete when a Python developer can implement a custom protocol or web assessment using Eggsec-owned transports, parsers, policy gates, events, and artifact models.

## Release 3 — Major subsystem completion

Release 3 completes the programmable surfaces for major subsystems that currently expose high-level or partial APIs.

Primary scope:

- NSE runtime lifecycle, script inspection, rule evaluation, library registration, limits, diagnostics, and structured output;
- interception proxy lifecycle, capture streaming, mutation hooks, replay, certificate management, and HAR export;
- stateful database sessions, driver capabilities, authentication, controlled queries, schema enumeration, privilege inspection, and evidence collection.

Release 3 is complete when NSE, proxy, and database functionality can be used as reusable Python subsystems instead of one-shot assessment wrappers.

## Release 4 — Stateful and remote execution

Release 4 stabilizes session-oriented domains, daemon execution, persistence, and reporting.

Primary scope:

- mobile device and dynamic-analysis sessions;
- browser session lifecycle and incremental artifacts;
- local/daemon operation parity;
- reconnect, replay, result retrieval, cancellation, timeout, and artifact semantics;
- persistent finding and assessment repositories;
- content-addressed artifact stores;
- reporting parity and streaming output.

Release 4 is complete when the same operation contract can execute locally or remotely and long-running assessments can persist, reconnect, resume, compare, and report without application-specific infrastructure.

## Release 5 — Ecosystem integration and public stabilization

Release 5 closes tool-schema integration, namespace governance, packaging, documentation, compatibility, and performance gates.

Primary scope:

- bindings for reusable `eggsec-tool-core` descriptors and schemas;
- operation-to-tool conversion for Python agent frameworks;
- explicit experimental namespace isolation;
- broad Python ergonomics and typing closure;
- supported wheel matrix;
- feature-profile governance;
- executable documentation examples;
- release performance budgets;
- stable API compatibility policy and migration tooling.

Release 5 is complete when Python is a first-class Eggsec host API rather than a broad experimental binding layer.

## Detailed workstreams

### Workstream A — Capability inventory and graduation governance

Create a machine-readable capability manifest that records, for every native and Python-facing domain:

- owning Rust crate and module;
- Cargo feature requirements;
- Python exports;
- sync and async entry points;
- request DTO;
- payload DTO;
- operation descriptor;
- policy and audit coverage;
- event coverage;
- cancellation and timeout behavior;
- fixture coverage;
- type-stub coverage;
- wheel-profile coverage;
- maturity classification;
- intentional non-binding decisions.

CI must detect drift between the capability manifest, operation registry, PyO3 exports, type stubs, documentation, and runtime feature matrix.

### Workstream B — Operation registry expansion

For each promoted operation:

1. define a canonical operation ID;
2. define or normalize the request DTO;
3. define the typed payload variant;
4. register the operation descriptor and feature requirements;
5. implement sync and async execution through the common dispatcher;
6. enforce scope, policy, audit, and risk metadata;
7. emit governed lifecycle events;
8. implement timeout and cancellation semantics;
9. convert direct functions into engine delegates;
10. add deterministic contract fixtures and installed-wheel tests.

No promoted domain may retain a second semantic implementation path.

### Workstream C — Pipeline completion

Expand pipelines from sequential stable-core composition to a general declarative workflow model:

- native async execution;
- dependencies and typed output references;
- conditional steps;
- bounded parallel groups;
- fan-out and fan-in;
- retry and backoff policy;
- per-step timeout;
- stop, continue, skip-dependent, and compensation behavior;
- progress and partial-result events;
- artifact references between steps;
- portable schema serialization;
- compatibility-validated checkpoint/resume.

Portable pipelines must not require arbitrary Python callbacks. Local callback extensions may remain provisional.

### Workstream D — Network and protocol primitives

Expose durable DTOs and managed sessions for resolution, TCP, UDP, TLS, DNS, HTTP, retries, timing, proxy routing, transcript capture, and evidence collection.

The API should provide the primitives required by Eggsec assessments without attempting to become a general replacement for every Python network library.

### Workstream E — WebSocket completion

Close the current mismatch between the declared WebSocket Cargo feature and the lack of a complete Python WebSocket API.

Provide connection/session classes, handshake metadata, text and binary messages, frame-level inspection where supported, ping/pong/close controls, transcripts, sync and async iteration, and a policy-governed `websocket_assess` operation.

### Workstream F — Packet capture and active probing

Provide explicit capture lifecycle, streaming packet access, protocol decoding, filters, PCAP/PCAPNG handling, flow aggregation, bounded buffering, drop accounting, packet replay, and controlled active probes.

Raw injection remains feature-gated, privilege-aware, scope-enforced, and experimental until hardened.

### Workstream G — NSE runtime completion

Expose script parsing, compilation, metadata, categories, dependencies, hostrule/portrule/postrule evaluation, script arguments, target contexts, direct script execution, runtime reuse, library registration, sandbox limits, diagnostics, event streaming, and structured output.

### Workstream H — Interception proxy completion

Expose start/stop lifecycle, captured exchange streams, filtering, request and response mutation, upstream routing, TLS interception, CA and certificate handling, WebSocket upgrade capture, replay, HAR import/export, body limits, storage limits, and redaction.

### Workstream I — Database sessions

Expose driver registry, capabilities, managed sessions, authentication, credential-provider protocols, controlled query execution, schema enumeration, version fingerprinting, privilege inspection, transactions, timeout, cancellation, safe result limits, and evidence capture.

Stable database behavior remains read-only by default.

### Workstream J — Mobile and browser sessions

Expose deterministic lifecycle primitives for devices, emulators, packages, applications, logs, network capture, artifact extraction, browser navigation, DOM snapshots, routes, console/network events, cookies, storage, screenshots, and cleanup.

Dynamic instrumentation remains experimental until platform behavior is repeatable and cleanup is reliable.

### Workstream K — Daemon parity

Unify local and daemon request normalization, payload schemas, errors, policy decisions, audit events, cancellation, timeouts, operation IDs, feature discovery, progress events, checkpoint identity, and artifact metadata.

Define explicit guarantees for submission idempotency, reconnect, result retrieval, event replay, duplicate suppression, ordering, cancellation races, daemon restart behavior, and terminal persistence.

### Workstream L — Persistence and reporting

Provide official in-memory, SQLite, JSONL, directory-backed, and content-addressed stores as appropriate. Add querying, pagination, deduplication, migration, retention, pruning, resumption, baseline comparison, artifact integrity, and import/export.

Align Python reporting with reusable native output infrastructure where doing so prevents semantic divergence.

### Workstream M — Tool and agent integration

Expose machine-readable tool descriptors, input/output schemas, capability requirements, policy metadata, invocation validation, operation-to-tool conversion, and structured tool results.

Keep framework-specific adapters optional and keep internal prompts and agent orchestration outside the stable core.

### Workstream N — Namespace and maturity isolation

Move hazardous or rapidly changing capabilities under an explicit experimental namespace. Maintain temporary top-level aliases only where migration compatibility requires them.

Every domain must have a deliberate outcome: stable, provisional, experimental, or internal.

### Workstream O — Python ergonomics and typing

Standardize `pathlib.Path`, `datetime`, enum behavior, context managers, async context managers, sequence/mapping protocols, iterators, async iterators, equality, hashing, representation, safe pickling, JSON conversion, buffer support, lazy artifacts, and secret-safe representations.

Validate `.pyi` parity with both mypy and pyright. Prefer typed convenience overloads and payload narrowing over broad `Any` results.

### Workstream P — Packaging and release profiles

Ship predictable wheels for supported CPython and platform combinations. Embed feature and maturity metadata in each wheel. Test built artifacts rather than only source trees.

The initial preference is a broad wheel containing non-system-dependent capabilities, with source-build features or separately justified profiles for platform-sensitive domains.

### Workstream Q — Documentation and examples

Document workflows rather than merely enumerating extension symbols. Every stable operation and major primitive should have executable examples covering sync, async, policy, errors, cancellation, pipelines, checkpoints, events, artifacts, and persistence where relevant.

### Workstream R — Release closure

Every release tranche must close correctness, compatibility, security, resource, and performance gates. Stable operations cannot rely on skipped integration tests or documentation-only claims.

## Cross-cutting validation requirements

Every stable operation must satisfy:

- request validation;
- feature discovery;
- scope denial;
- policy confirmation or denial where required;
- audit emission;
- sync execution;
- async execution;
- structured error mapping;
- timeout;
- cancellation;
- event ordering;
- payload serialization;
- secret-sentinel exclusion;
- deterministic cleanup;
- type-stub parity;
- installed-wheel execution.

Every long-running or streaming primitive must additionally validate:

- bounded queues;
- backpressure accounting;
- consumer cancellation;
- partial-result behavior;
- terminal event delivery;
- resource cleanup after exceptions;
- large-artifact externalization.

Every daemon-stable operation must run the same contract suite against local and daemon engines.

## Scope boundaries

The roadmap does not bind implementation details merely because they exist in Rust. The following remain outside the public Python API unless a specific durable use case is approved:

- TUI widget state;
- terminal rendering;
- CLI parser internals;
- daemon server implementation details;
- Tokio channels and task types;
- UI-specific view models;
- agent prompt implementation;
- internal runtime ownership objects.

The Python API exposes durable security concepts: operations, requests, results, findings, evidence, artifacts, sessions, events, policies, transports, parsers, stores, and descriptors.

## Completion definition

This roadmap is complete when:

- all major Eggsec domains have a deliberate public maturity classification;
- stable assessment domains execute through one canonical engine contract;
- Python users can build custom network and protocol assessments using Eggsec primitives;
- NSE, proxy, database, mobile, browser, and packet subsystems expose managed programmable lifecycles where intended;
- local and daemon execution are semantically equivalent for declared stable operations;
- assessments can stream, cancel, checkpoint, resume, persist, compare, and report;
- tool schemas can be consumed by Python orchestration systems without bypassing policy;
- wheels, typing, documentation, compatibility metadata, and performance gates support a credible public release.

The key metric is not the percentage of Rust structures wrapped by PyO3. It is the percentage of intended Eggsec capabilities that can be composed predictably through a single governed Python execution model.