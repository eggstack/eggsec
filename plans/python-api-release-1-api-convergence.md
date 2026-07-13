# Eggsec Python API Release 1 — API Convergence

## Handoff objective

Release 1 converts mature but isolated Python bindings into canonical Eggsec operations and makes the pipeline layer capable of composing the enlarged operation registry.

This is an integration and convergence release, not a feature-count release. The implementation must reduce parallel execution paths, preserve the stable-core guarantees already established, and make every promoted domain behave like an existing stable operation.

The release must not weaken the current local `Engine`/`AsyncEngine` contract, policy gate, audit model, structured errors, event guarantees, checkpoint compatibility rules, secret handling, installed-wheel validation, or maturity metadata.

## Current state

The Python package has a stable ten-operation local engine boundary:

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

Many additional domains are importable through direct functions and domain-specific DTOs, but do not participate fully in the canonical operation registry, `OperationRequest`, `OperationPayload`, common policy dispatch, governed event lifecycle, pipeline composition, checkpointing, or generic introspection.

The resulting package is broad but semantically uneven. Release 1 closes that gap for the most mature domains.

## Release outcome

At completion:

- the capability inventory is authoritative and machine-readable;
- mature provisional domains use canonical operation IDs;
- promoted domains have typed requests and typed payloads;
- direct functions delegate through `Engine` or `AsyncEngine` rather than maintaining separate semantics;
- pipelines support all stable operations;
- async pipelines are natively asynchronous;
- pipelines support dependencies, retries, timeouts, bounded parallelism, failure policy, events, and checkpoint/resume;
- public exports, type stubs, feature metadata, documentation, and maturity classification remain synchronized;
- no promoted operation bypasses policy, audit, cancellation, structured errors, or secret handling.

## Scope

### Mandatory promotion candidates

The first implementation pass should evaluate and promote the following domains in this order:

1. `scan_git_secrets`
2. `generate_sbom`
3. `run_consolidated_recon`
4. `graphql_test`
5. `oauth_test`
6. `auth_test`
7. `db_probe`
8. `nse_run`
9. `scan_docker_image`
10. `scan_kubernetes`
11. `analyze_apk`
12. `analyze_ipa`

The ordering intentionally starts with artifact/local-analysis domains and ordinary web-assessment domains before platform-sensitive domains.

### Conditional candidates

The following may be promoted during Release 1 only if they can meet all graduation gates without conditional skips or platform-dependent ambiguity:

- `browser_test`
- `hunt_test`

If they cannot meet the gates, leave them provisional and record the exact blockers in the capability manifest. Do not weaken the graduation criteria to increase the operation count.

### Explicit non-goals

Release 1 does not attempt to complete:

- low-level HTTP, TCP, UDP, TLS, DNS, or packet primitives;
- WebSocket session bindings;
- full NSE runtime programmability;
- interception proxy lifecycle;
- stateful database sessions;
- mobile dynamic-analysis lifecycle;
- browser automation as a general API;
- daemon parity;
- public 1.0 stabilization;
- new offensive capabilities unrelated to operation convergence.

Those belong to later releases.

## Workstream 1 — Authoritative capability inventory

### Goal

Create one machine-readable source of truth for native capabilities, Python exports, operation maturity, and validation status.

### Required data

For each domain or operation, record:

- domain ID;
- public display name;
- owning Rust crate;
- owning Rust module;
- Cargo feature;
- default-wheel availability;
- Python module/export names;
- sync direct function;
- async direct function;
- canonical operation ID;
- request DTO;
- payload DTO;
- operation descriptor;
- operation risk;
- operation mode;
- intended use;
- policy/scope support;
- audit support;
- lifecycle event support;
- cancellation support;
- timeout support;
- deterministic fixture status;
- serialization status;
- type-stub status;
- wheel-profile status;
- maturity classification;
- known blockers;
- intentionally unbound primitives.

### Suggested files

- `crates/eggsec-python/python/eggsec/_capabilities.json` or generated equivalent
- `docs/python/API_CAPABILITY_MATRIX.md`
- `docs/python/domain-maturity.md`
- `crates/eggsec-python/src/domains.rs`
- `crates/eggsec-python/src/features.rs`
- `crates/eggsec-python/src/operation_registry.rs`
- `scripts/check-python-capability-matrix.py`
- `scripts/check-architecture-guards.sh`

### Implementation requirements

- Prefer generation from Rust-owned metadata where practical.
- Do not maintain several manually duplicated lists without drift checks.
- `domain_maturity()`, `api_surface()`, `feature_matrix()`, and `Engine.list_operations()` must be derivable from or validated against the same metadata.
- Every provisional domain must identify the unmet graduation gates.
- Every internal or intentionally unbound subsystem must be recorded so absence is deliberate rather than accidental.

### Validation

Add CI checks that fail when:

- an engine operation lacks capability metadata;
- capability metadata references an absent executor;
- a PyO3 export lacks stub coverage;
- a stub symbol is absent from the built extension;
- a Cargo feature is missing from feature metadata;
- a stable domain lacks all mandatory contract statuses;
- documentation names an operation not present in the registry.

## Workstream 2 — Operation request and payload expansion

### Goal

Add canonical request and result representations for every promoted operation.

### Request model requirements

Each promoted operation must have a dedicated request DTO that:

- validates required fields at construction or dispatch;
- uses stable Python-friendly field names;
- supports JSON serialization where appropriate;
- excludes raw secrets from `repr` and serialization;
- uses `SensitiveString` for secret-bearing fields;
- identifies target and scope-relevant fields explicitly;
- records feature requirements;
- carries timeout or execution options consistently;
- converts to the native Rust domain configuration without lossy translation.

Extend:

- `OperationRequest`
- `RequestBuilder`
- request normalization helpers
- `.pyi` definitions
- serialization schemas

### Payload model requirements

Each promoted operation must have a typed `OperationPayload` variant and Python conversion path.

The result must preserve native domain data. Do not flatten rich results into strings, generic dictionaries, or report-only blobs merely to fit the common engine.

Each payload should define:

- payload type name;
- stable schema version where persistence is expected;
- Python DTO conversion;
- JSON conversion;
- artifact references for large data;
- finding conversion where applicable;
- secret redaction behavior;
- equality/representation behavior needed by tests.

### Likely files

- `crates/eggsec-python/src/requests.rs`
- `crates/eggsec-python/src/status.rs`
- `crates/eggsec-python/src/engine.rs`
- `crates/eggsec-python/src/async_engine.rs`
- domain-specific binding modules
- `crates/eggsec-python/python/eggsec/__init__.pyi`

### Acceptance criteria

- No promoted operation returns an empty or placeholder payload.
- `OperationResult.payload_type_name` is deterministic.
- Sync and async calls produce equivalent payload types.
- Payload serialization round-trips for persistable domains.
- Large outputs use artifact references or lazy containers where required.

## Workstream 3 — Registry and executor integration

### Goal

Route every promoted domain through the authoritative operation registry and shared engine state.

### Required implementation

For each operation:

- add a stable operation constant;
- add an `OperationDescriptor`;
- record feature requirements;
- record risk, mode, intended use, target policy, and confirmation behavior;
- register a sync executor;
- register an async executor or canonical async bridge;
- call `pre_dispatch_validate()` or its successor;
- emit the dispatch audit event;
- populate execution statistics;
- map native errors to `OperationError`;
- preserve cancellation reason;
- apply timeout semantics;
- emit lifecycle events through the governed event system.

### Executor architecture

Avoid a large monolithic match statement becoming the permanent architecture.

Prefer a registry entry that binds:

- operation metadata;
- request validation/conversion;
- sync executor;
- async executor;
- payload conversion;
- feature predicate.

The registry must continue to support:

- `list_operations()`;
- `has_operation()`;
- descriptor introspection;
- feature-unavailable errors;
- unknown-operation suggestions;
- stable operation ordering where documented.

### Direct-function migration

Existing direct functions remain for compatibility but must delegate through the canonical engine path.

Requirements:

- no duplicate scope or policy implementation;
- no duplicate timeout semantics;
- no direct-function-only result shape;
- no direct-function-only exception behavior;
- compatibility tests compare direct and engine invocation;
- deprecation is not required during Release 1 unless an existing function is unsafe or irreconcilably inconsistent.

## Workstream 4 — Policy, scope, audit, and risk equivalence

### Goal

Prevent newly promoted operations from becoming policy bypasses.

### Requirements

For each promoted operation:

- identify all target-bearing request fields;
- normalize targets before scope evaluation;
- classify local artifact operations separately from remote target operations;
- define feature and privilege requirements;
- define confirmation classes;
- define denial classes;
- emit a complete audit event for allow, deny, confirmation-required, cancellation, timeout, and failure outcomes;
- preserve manual override metadata where permitted;
- verify strict and manual-permissive profiles;
- ensure operation descriptors match actual executor behavior.

### Domain-specific considerations

#### Git secrets and SBOM

These are local artifact operations. Scope policy should validate allowed paths or artifact sources rather than pretending they are network targets.

#### Database probing

Credential-bearing fields must use `SensitiveString`. Read-only defaults and destructive-operation exclusions must be explicit.

#### NSE

Script categories, target scope, sandbox policy, and any intrusive script classifications must influence policy decisions.

#### Container and mobile artifacts

Distinguish local artifact analysis from live daemon, cluster, device, or runtime access.

### Validation

Add policy-equivalence tests comparing:

- engine preflight;
- sync engine execution;
- async engine execution;
- direct compatibility function;
- operation descriptor metadata.

## Workstream 5 — Event and lifecycle integration

### Goal

Make promoted operations observable through the same governed event system as stable-core operations.

### Required lifecycle

Operations should emit, as applicable:

- planning event;
- preflight event;
- stage started;
- progress;
- finding;
- artifact;
- cancellation;
- failure;
- completion.

### Requirements

- sequence numbers remain monotonic per execution;
- terminal events are reliable;
- backpressure policy is explicit;
- progress events may be lossy only where documented;
- finding and artifact events preserve structured references;
- direct functions use the same event path when an event sink is supplied;
- event payloads do not contain secrets;
- callback failures cannot silently corrupt operation state.

### Validation

For every promoted operation, assert:

- exactly one terminal outcome;
- no completion after cancellation or failure;
- stage events are ordered;
- event stats account for drops;
- secret sentinels are absent;
- async consumers can cancel cleanly.

## Workstream 6 — Cancellation and timeout closure

### Goal

Give every promoted operation predictable interruption semantics.

### Requirements

- accept the common cancellation token or execution handle;
- check cancellation before expensive stages;
- propagate cancellation into native async work where possible;
- define behavior for blocking native libraries;
- enforce operation timeout without orphaning resources;
- preserve partial findings and artifacts where safe;
- record cancellation reason in the structured result and terminal event;
- distinguish timeout from explicit cancellation;
- clean up files, sockets, processes, runtimes, and temporary directories.

Operations that cannot be safely interrupted must remain provisional until the limitation is resolved or explicitly bounded.

## Workstream 7 — Pipeline model expansion

### Goal

Make the pipeline API useful for the enlarged operation registry and real assessment workflows.

### Required features

#### Native async execution

Replace any sync delegation inside `AsyncPipeline` with native asynchronous scheduling.

#### Step dependencies

Allow a step to depend on one or more prior steps.

#### Typed output references

Introduce a serializable reference model such as:

```python
OutputRef(step_id="recon", path="payload.resolved_hosts")
```

References must validate against completed step results and fail with structured pipeline errors.

#### Conditional execution

Support declarative conditions based on:

- prior step status;
- presence or absence of findings;
- payload field values;
- feature availability.

Avoid arbitrary Python lambdas in portable pipeline schemas.

#### Parallel groups

Support bounded parallel execution for independent steps. Concurrency must honor engine-wide and operation-specific limits.

#### Retry policy

Support:

- maximum attempts;
- retryable error classes;
- fixed or exponential backoff;
- maximum delay;
- jitter where deterministic testing can control it.

#### Failure policy

Support:

- stop pipeline;
- continue;
- skip dependent steps;
- mark partial;
- execute declared compensation step where supported.

#### Per-step timeout

A pipeline step may override the pipeline default timeout without bypassing operation limits.

#### Partial results

Preserve completed steps and safe partial outputs when later steps fail or cancel.

#### Pipeline events

Emit pipeline and step lifecycle events through the governed event protocol.

#### Serialization

Pipeline definitions must serialize to a versioned schema without arbitrary code objects.

### Suggested files

- `crates/eggsec-python/src/pipeline.rs`
- `crates/eggsec-python/src/planning.rs`
- `crates/eggsec-python/src/checkpoint.rs`
- `crates/eggsec-python/src/checkpoint_store.rs`
- `crates/eggsec-python/src/event_protocol.rs`
- Python façade modules and stubs

## Workstream 8 — Checkpoint and resume integration

### Goal

Preserve the existing checkpoint compatibility guarantees while supporting promoted operations and richer pipelines.

### Requirements

- include pipeline schema version;
- include operation schema version;
- include registry or capability identity;
- include target-set hash;
- include scope hash;
- include execution profile;
- include enabled feature-set hash;
- include pipeline-definition hash;
- include artifact-store identity;
- redact secret-bearing keys recursively;
- write atomically;
- restore typed step results;
- reject incompatible resumes with `checkpoint_incompatible`;
- do not require secrets to be persisted for resume.

### Pipeline compatibility

Changing any of the following must invalidate or explicitly migrate the checkpoint:

- operation ID;
- request schema;
- step graph;
- output reference path;
- feature requirements;
- scope identity;
- artifact-store identity.

## Workstream 9 — Namespace and typing convergence

### Goal

Make newly promoted operations discoverable without worsening top-level namespace sprawl.

### Requirements

- expose stable operations through `Engine`, `AsyncEngine`, and deliberate convenience functions;
- retain domain DTOs in logical modules or façade namespaces;
- avoid adding every internal class to the top-level namespace;
- update `api_surface()` and `domain_maturity()`;
- update `.pyi` files;
- add `Literal` operation IDs where practical;
- add typed overloads for engine convenience methods;
- support payload narrowing in static typing;
- run mypy and pyright over examples;
- retain secret-safe `repr` behavior.

### Experimental isolation

Release 1 may introduce the namespace scaffolding for `eggsec.experimental`, but broad migration of all experimental domains is not required until the later stabilization release.

## Workstream 10 — Documentation and executable examples

### Required documentation

- updated domain maturity table;
- capability matrix;
- operation registry reference;
- migration guide from direct functions to engine operations;
- pipeline schema guide;
- retries and failure policy;
- checkpoint/resume guide;
- cancellation and timeout guide;
- feature-profile behavior;
- structured error reference.

### Required examples

At minimum:

- Git-secret scan through `Engine`;
- SBOM generation through `AsyncEngine`;
- consolidated recon pipeline;
- GraphQL assessment with preflight;
- OAuth assessment with cancellation;
- database probe using `SensitiveString`;
- NSE execution with policy metadata;
- container artifact assessment;
- pipeline fan-out and fan-in;
- checkpoint and resume.

Examples should run in CI where their feature profile is available.

## Validation plan

### Rust validation

Run at minimum:

```bash
cargo fmt --all -- --check
cargo check -p eggsec-python
cargo check -p eggsec-python --features full-no-system
cargo test -p eggsec-python
bash scripts/check-architecture-guards.sh
```

Add feature-specific jobs for promoted domains not included in `full-no-system`.

### Python validation

Build the extension or wheel and run:

```bash
pytest crates/eggsec-python/tests/ crates/eggsec-python/python/tests/ -v --timeout=30
```

Add contract suites for every promoted operation covering:

- request validation;
- registry discovery;
- feature unavailable behavior;
- preflight;
- scope denial;
- policy denial or confirmation;
- audit event;
- sync success;
- async success;
- direct/engine equivalence;
- structured failure;
- timeout;
- cancellation;
- event ordering;
- payload serialization;
- secret redaction;
- installed-wheel execution.

### Pipeline validation

Add deterministic tests for:

- sequential execution;
- dependency resolution;
- invalid references;
- conditional skip;
- parallel execution limit;
- retry success;
- retry exhaustion;
- stop-on-failure;
- continue-on-failure;
- dependent-step skip;
- per-step timeout;
- cancellation propagation;
- partial result preservation;
- checkpoint restore;
- checkpoint incompatibility;
- secret sentinel exclusion.

### Performance validation

Track at minimum:

- generic engine dispatch overhead;
- direct-function compatibility overhead;
- pipeline scheduler overhead;
- event emission overhead;
- async parallel-step scaling;
- memory growth for large payloads.

Release 1 should not introduce large regressions to the stable ten operations.

## Commit sequence recommendation

A clean implementation sequence is:

1. capability manifest and drift guards;
2. shared operation-registration abstractions;
3. artifact/local operations: Git secrets and SBOM;
4. consolidated recon;
5. GraphQL, OAuth, and auth;
6. database probe;
7. NSE operation integration;
8. container and mobile static operations;
9. direct-function delegation and equivalence tests;
10. native async pipeline scheduler;
11. dependencies and typed output references;
12. retries, timeouts, failure policy, and parallel groups;
13. checkpoint schema extension;
14. namespace, typing, documentation, and release closure.

Keep commits scoped enough that registry changes, domain promotion, and pipeline changes can be reviewed independently.

## Risks and mitigations

### Risk: operation registry becomes a monolithic dispatcher

Mitigation: use registry-owned executor descriptors and domain adapters rather than growing a single giant match block.

### Risk: direct functions and engine paths diverge

Mitigation: direct functions must delegate and contract tests must compare outputs and errors.

### Risk: rich domain results are flattened

Mitigation: add explicit payload variants and artifact references rather than generic dictionaries or strings.

### Risk: policy metadata does not match behavior

Mitigation: descriptor/executor equivalence tests and domain-specific target normalization tests.

### Risk: pipeline portability is lost to Python callbacks

Mitigation: keep portable conditions and references declarative; classify callbacks as local-only provisional extensions.

### Risk: feature combinations make CI unmanageable

Mitigation: define intentional wheel/build profiles and test promotion candidates in grouped matrices without claiming untested combinations.

### Risk: checkpoint schema becomes unstable

Mitigation: version the pipeline schema and compatibility identity before adding richer workflow behavior.

## Release acceptance criteria

Release 1 is complete only when:

- the capability manifest covers all current Python domains;
- CI validates manifest, registry, exports, stubs, features, and documentation;
- all mandatory promotion candidates either graduate or have explicit, evidence-backed blockers recorded;
- every graduated operation has canonical request and payload types;
- every graduated operation passes the common policy, audit, event, timeout, cancellation, serialization, and installed-wheel contract suite;
- direct compatibility functions delegate through the engine;
- `Engine.list_operations()` and `AsyncEngine.list_operations()` expose the enlarged stable registry;
- pipelines support all stable operations;
- `AsyncPipeline` is natively asynchronous;
- dependencies, typed output references, bounded parallelism, retries, failure policy, timeouts, events, and checkpoint/resume are tested;
- secret sentinels are absent from events, errors, reports, artifacts, and checkpoints;
- mypy and pyright pass representative examples;
- documentation accurately distinguishes stable, provisional, experimental, and internal surfaces;
- the existing ten stable operations retain their release guarantees and validation coverage.

## Handoff note

Do not optimize this release for the largest possible operation count. Optimize it for removal of semantic forks. A domain should graduate only when it behaves like a first-class Eggsec operation across execution, policy, lifecycle, errors, typing, tests, and packaging.