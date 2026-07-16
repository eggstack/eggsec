# Python API Release 5 Phase A — Tool-Core and Schema Integration

## Objective

Expose Eggsec's protocol-neutral tool abstraction to Python without duplicating the existing operation API or importing framework-specific agent behavior into the stable package. The result must let Python callers inspect, validate, serialize, and invoke Eggsec capabilities through durable tool descriptors while preserving Eggsec scope, policy, audit, cancellation, and rate-limit semantics.

## Current problem

`eggsec-tool-core` already owns generic request, response, error, stream, rate-limit, history, authentication, scope, target, port, endpoint, and technology data. The main Rust tool layer adds registry metadata, registrations, OpenAPI/schema generation, planning, dispatch, orchestration, and protocol adapters. The Python crate currently exposes a separate operation-oriented model but does not provide a first-class bridge to these reusable tool primitives.

Without this bridge, Python integrations must reconstruct schemas, capability requirements, errors, and invocation metadata manually. That creates semantic drift and encourages adapters to bypass Eggsec policy and validation.

## Workstream A1 — Dependency and boundary audit

1. Add a direct dependency from `eggsec-python` to `eggsec-tool-core` using the workspace path.
2. Audit every public type in `eggsec-tool-core` and classify it as:
   - bind directly;
   - map into an existing Python type;
   - expose through a Python-native DTO;
   - internal and intentionally omitted.
3. Document overlap with existing Python types such as `Scope`, `TargetPy`, `CancellationToken`, `Finding`, `OperationResult`, `ExecutionEvent`, and network DTOs.
4. Avoid two public Python classes for the same concept unless a compatibility adapter is unavoidable.
5. Record conversion direction and losslessness for every mapped type.

Deliverable: `docs/python/TOOL_CORE_BINDING_MAP.md` and a machine-readable map under `crates/eggsec-python/validation/`.

## Workstream A2 — Core tool DTOs

Implement Python classes or adapters for:

- `ToolRequest`;
- `ToolResponse`;
- `ToolError` and `ToolErrorType`;
- `RequestOptions`;
- `AuthConfig` and `AuthType`;
- `Target` and `TargetType`;
- response status and metadata;
- progress updates and stream events;
- rate-limit configuration and status;
- execution-history entries;
- generic endpoint, port, technology, and finding data.

Requirements:

- all DTOs support deterministic `to_dict()` and `to_json()`;
- enums expose stable string values and reject unknown values clearly;
- secret-bearing fields use `SensitiveString` or equivalent redaction wrappers;
- repr and error messages never reveal credentials, tokens, cookies, authorization headers, or private keys;
- schema versions are explicit;
- conversion tests prove round trips between Rust and Python representations;
- pickling is disabled unless a safe, versioned contract is explicitly implemented.

## Workstream A3 — Tool descriptors and registrations

Expose a stable Python descriptor model containing:

- canonical tool name and version;
- human-readable title and description;
- category and capabilities;
- input schema;
- output schema;
- required Cargo feature;
- maturity;
- risk classification;
- intended-use metadata;
- confirmation requirement;
- target and scope requirements;
- streaming support;
- cancellation support;
- timeout support;
- artifact and finding behavior;
- local and daemon availability.

Expose registry functions or classes equivalent to:

```python
registry = eggsec.tools.registry()
registry.list()
registry.get("scan_ports")
registry.schema("scan_ports")
registry.validate("scan_ports", payload)
```

The exact spelling may change, but descriptors must be framework-neutral and generated from authoritative Rust metadata.

## Workstream A4 — Operation-to-tool conversion

Create a canonical adapter from every stable operation descriptor to a tool descriptor. It must:

1. preserve the operation ID;
2. generate a deterministic JSON Schema for the typed request;
3. describe the typed result payload and common `OperationResult` envelope;
4. carry feature, maturity, risk, policy, and confirmation metadata;
5. preserve structured errors;
6. expose whether progress/events are available;
7. validate supplied dictionaries before dispatch;
8. delegate invocation through `Engine` or `AsyncEngine`, never through a second implementation path.

Add APIs such as `operation_as_tool()` or an equivalent registry view. Stable operation tools and legacy Rust tool registrations must have explicit identity and alias rules; do not silently conflate differently shaped requests.

## Workstream A5 — Invocation API

Provide synchronous and asynchronous invocation that accepts either a typed `ToolRequest` or validated mapping:

```python
response = engine.invoke_tool(request)
response = await async_engine.invoke_tool(request)
```

Invocation must preserve:

- feature-unavailable errors;
- scope and authorization enforcement;
- audit event generation;
- cancellation and timeout behavior;
- rate-limit decisions;
- progress and stream events;
- typed findings and artifact references;
- correlation IDs and execution history.

Do not permit a tool descriptor to call internal functions directly around the engine gate.

## Workstream A6 — Schema generation

Generate JSON Schema Draft 2020-12 or another explicitly pinned dialect for tool inputs and outputs. Requirements:

- deterministic field ordering;
- stable `$id` and schema version conventions;
- enum constraints;
- numeric bounds;
- path and URL formats where appropriate;
- feature and maturity annotations under namespaced extension keys;
- secret fields marked write-only and excluded from examples;
- no Rust implementation names such as `Py` suffixes in public schema names;
- schema snapshots checked into test fixtures;
- compatibility tests detect breaking schema changes.

Provide an OpenAPI conversion only as a derived adapter; JSON Schema remains the canonical tool contract.

## Workstream A7 — Optional framework adapters

Add no framework dependency to the core wheel. Instead define a small adapter protocol and optional modules for converting tool descriptors to common external formats. Initial adapters may include generic OpenAI-compatible function schemas and MCP-compatible tool definitions, but must:

- remain optional;
- contain no agent prompts or orchestration policy;
- preserve Eggsec risk and policy metadata;
- invoke through the canonical engine/tool path;
- reject unsupported streaming or callback semantics explicitly.

## Workstream A8 — Validation

Add tests for:

- all stable operations convert to valid tool descriptors;
- schema generation is deterministic across runs;
- typed request, mapping request, and JSON request normalization are equivalent;
- invalid fields, unknown fields, enum errors, and numeric bounds fail before execution;
- feature-gated operations remain discoverable but report structured unavailability;
- scope and policy denials are identical through operation and tool invocation;
- sync and async tool responses normalize identically;
- secret sentinels do not appear in descriptors, schemas, repr, errors, events, history, or artifacts;
- rate-limit metadata survives round trip;
- installed wheels expose the same descriptor inventory as source builds for the same profile.

## Documentation

Add:

- `docs/python/tools.md`;
- a tool discovery example;
- a typed invocation example;
- a mapping/JSON invocation example;
- an async streaming example;
- an optional framework-adapter example;
- migration guidance for users currently building schemas from `api_surface()`.

## Acceptance criteria

Phase A is complete when:

- `eggsec-tool-core` has a deliberate, tested Python binding map;
- every stable operation has one deterministic tool descriptor and request schema;
- tool invocation delegates through `Engine`/`AsyncEngine`;
- tool errors, rate limits, events, history, findings, and artifacts remain structured;
- no framework-specific dependency is required by the core wheel;
- schema and descriptor compatibility are guarded in CI;
- no policy, scope, confirmation, or redaction behavior can be bypassed through the tool API.