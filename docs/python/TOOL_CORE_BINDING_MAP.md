# Tool-Core Binding Map

Machine-readable binding map for `eggsec-tool-core` types exposed to Python.
Every type in `eggsec-tool-core` is documented with its Python equivalent,
binding strategy, and conversion properties.

## Binding Strategies

| Strategy | Meaning |
|----------|---------|
| **Direct** | Python type wraps the Rust type 1:1 with no data loss |
| **Direct (aliased)** | Python type wraps Rust type but is exported under a different name |
| **Direct (renamed via name attr)** | `#[pyclass(name = "...")]` renames the Python-visible class |
| **Mapped** | Python type maps to an existing engine type (not a new binding) |
| **Omitted** | Not exposed to Python (internal only) |

## Enums

| eggsec-tool-core Type | Python Type | Binding | Conversion |
|---|---|---|---|
| `TargetType` | `ToolTargetType` | Direct (aliased) | Lossless; 1:1 variant mapping |
| `AuthType` | `ToolAuthType` | Direct (aliased) | Lossless; 1:1 variant mapping |
| `ResponseStatus` | `ToolResponseType` | Direct (aliased) | Lossless; 1:1 variant mapping |
| `FindingType` | `ToolFindingType` | Direct (renamed via `name` attr) | Lossless; 1:1 variant mapping |
| `ResponseSeverity` | `ToolSeverity` | Direct (renamed via `name` attr) | Lossless; 1:1 variant mapping |
| `ToolErrorType` | `ToolErrorType` | Direct | Lossless; 1:1 variant mapping |
| `PortState` | `ToolPortState` | Direct (aliased) | Lossless; 1:1 variant mapping |
| `StreamEventType` | `ToolStreamEventType` | Direct (aliased) | Lossless; 1:1 variant mapping |

## Structs

| eggsec-tool-core Type | Python Type | Binding | Conversion |
|---|---|---|---|
| `Scope` | `ToolScope` | Direct (aliased) | Lossless; fields preserved |
| `Target` | `ToolTarget` | Direct (renamed via `name` attr) | Lossless; factory methods preserved |
| `RequestOptions` | `ToolRequestOptions` | Direct (aliased) | Lossless; all fields mapped |
| `AuthConfig` | `ToolAuthConfig` | Direct (aliased) | Lossless; credentials redacted in repr/to_dict/to_json |
| `ToolRequest` | `ToolRequest` | Direct | Lossless; params converted via serde_json |
| `ResponseMetadata` | `ToolResponseMetadata` | Direct (aliased) | Lossless; DateTime converted to RFC 3339 string |
| `Finding` | `ToolFinding` | Direct | Lossless; all fields mapped |
| `ToolError` | `ToolError` | Direct | Lossless; all fields mapped |
| `ToolResponse` | `ToolResponse` | Direct | Lossless; results converted via serde_json |
| `ProgressUpdate` | `ToolProgressUpdate` | Direct (aliased) | Lossless; all fields mapped |
| `StreamEvent` | `ToolStreamEvent` | Direct (aliased) | Lossless; typed factory methods preserved |
| `PortData` | `ToolPortData` | Direct (aliased) | Lossless; all fields mapped |
| `EndpointData` | `ToolEndpointData` | Direct (aliased) | Lossless; DateTime set at construction |
| `TechnologyData` | `ToolTechnologyData` | Direct (aliased) | Lossless; all fields mapped |
| `RateLimitConfig` | `ToolRateLimitConfig` | Direct (aliased) | Lossless; preset constructors preserved |
| `RateLimitStatus` | `ToolRateLimitStatus` | Direct (aliased) | Lossless; all fields mapped |
| `ExecutionEntry` | `ToolExecutionEntry` | Direct (aliased) | Lossless; DateTime converted to RFC 3339 string |

## Special Types

| eggsec-tool-core Type | Python Type | Binding | Conversion |
|---|---|---|---|
| `CancellationToken` | `CancellationToken` (existing) | Mapped to existing | Maps to `eggsec.CancellationToken` already in the API surface |
| `CancellationTokenHandle` | N/A (internal) | Omitted | Internal serialization wrapper; not exposed |
| `ExecutionHistory` | N/A (internal) | Omitted | Internal bookkeeping; not exposed |

## Conversion Notes

### Lossless Conversions

All Direct bindings are lossless. The Python wrapper holds the Rust type
internally and converts via `From`/`Into` impls. Serialization uses
`serde_json` for struct fields that are JSON values (e.g., `ToolRequest.params`,
`ToolResponse.results`).

### Redacted Fields

`ToolAuthConfig` (wrapping `AuthConfig`) redacts credential values in
`to_dict()`, `to_json()`, `__repr__`, and `__str__`. The credential keys are
preserved but values are replaced with `[REDACTED]`. This is a deliberate
safety measure and is documented in the class docstring.

### DateTime Handling

`ResponseMetadata.started_at` and `ResponseMetadata.completed_at` are
`chrono::DateTime<Utc>` in Rust. In Python they are represented as RFC 3339
strings. The constructor accepts both ISO 8601 and naive datetime formats.

### JSON Value Fields

`ToolRequest.params` and `ToolResponse.results` are `serde_json::Value` in
Rust. In Python they are converted to native Python objects via `json.loads()`
on the serialized JSON. Construction accepts Python dicts/lists which are
converted to `serde_json::Value` via `json.dumps()`.

## Reference

- Rust source: `crates/eggsec-tool-core/src/`
- Python binding: `crates/eggsec-python/src/tool_core.rs`
- Re-exports: `crates/eggsec-python/python/eggsec/__init__.py`
- Type stubs: `crates/eggsec-python/python/eggsec/__init__.pyi`
