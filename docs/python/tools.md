# Tool Abstraction Layer

Release 5 Phase A exposes `eggsec-tool-core` types to Python, providing a
deterministic tool abstraction for all 22 stable operations. The tool layer
gives every operation a unified request/response contract, JSON Schema
generation, and a registry-driven invocation path.

> **Phase B Note:** The underlying dispatch architecture was refactored in
> Release 5 Phase B. `ToolRegistry` descriptors are now generated from the
> authoritative `OperationExecutorDescriptor` registry (not hand-maintained).
> The `SchemaGenerator` produces JSON Schema from the same registry source.
> See `docs/python/REGISTRY_ARCHITECTURE.md` for the full dispatch flow.

## Overview

The tool abstraction layer sits between the Python API surface and the engine
dispatch layer. It provides:

- **Unified request/response types** — `ToolRequest`, `ToolResponse`,
  `ToolFinding`, `ToolError` for all operations.
- **Deterministic tool descriptors** — each of the 22 stable operations has a
  `ToolDescriptor` with ID, label, target types, parameter schema, and risk
  classification.
- **JSON Schema generation** — `SchemaGenerator` produces JSON Schema for any
  operation's request/response types.
- **Tool invocation** — `Engine.invoke_tool()` and
  `AsyncEngine.async_invoke_tool()` execute a `ToolRequest` through the
  policy gate.

## Tool Types

All types are frozen pyclasses with `to_dict()`, `to_json()`, `__repr__`, and
`__str__` methods. They follow the existing eggsec-python conventions.

### Enums

| Python Type | Rust Source | Values |
|-------------|-------------|--------|
| `ToolTargetType` | `TargetType` | `url`, `domain`, `ip`, `cidr`, `file` |
| `ToolAuthType` | `AuthType` | `none`, `basic`, `bearer`, `api_key`, `oauth2` |
| `ToolResponseType` | `ResponseStatus` | `success`, `partial_success`, `failed`, `timeout`, `scope_violation`, `cancelled` |
| `ToolFindingType` | `FindingType` | `vulnerability`, `information`, `weakness`, `configuration`, `misconfiguration`, `sensitive_data`, `banner`, `technology`, `service`, `endpoint`, `subdomain`, `open_port` |
| `ToolSeverity` | `ResponseSeverity` | `critical`, `high`, `medium`, `low`, `info`, `none` |
| `ToolErrorType` | `ToolErrorType` | `validation`, `authentication`, `authorization`, `rate_limit`, `network`, `timeout`, `scope_violation`, `not_found`, `configuration`, `internal`, `tool_not_found` |
| `ToolPortState` | `PortState` | `open`, `closed`, `filtered` |
| `ToolStreamEventType` | `StreamEventType` | `progress`, `finding`, `result`, `error` |

### Structs

| Python Type | Rust Source | Description |
|-------------|-------------|-------------|
| `ToolScope` | `Scope` | Allowed/excluded patterns and IPs |
| `ToolTarget` | `Target` | Target type + value + optional scope |
| `ToolRequestOptions` | `RequestOptions` | Timeout, concurrency, proxy, stealth, SSL |
| `ToolAuthConfig` | `AuthConfig` | Auth type + credentials (redacted in repr) |
| `ToolRequest` | `ToolRequest` | Execution request (tool, target, params, options) |
| `ToolResponseMetadata` | `ResponseMetadata` | Timing, counts, duration |
| `ToolFinding` | `Finding` | Security finding with type, severity, location |
| `ToolError` | `ToolError` | Structured error with code, type, retry info |
| `ToolResponse` | `ToolResponse` | Full response (status, results, metadata, errors, findings) |
| `ToolProgressUpdate` | `ProgressUpdate` | Streaming progress (stage, percentage, items) |
| `ToolStreamEvent` | `StreamEvent` | Typed event (progress, finding, result, error) |
| `ToolPortData` | `PortData` | Port scan result for a single port |
| `ToolEndpointData` | `EndpointData` | Discovered endpoint |
| `ToolTechnologyData` | `TechnologyData` | Detected technology |
| `ToolRateLimitConfig` | `RateLimitConfig` | Rate limit configuration |
| `ToolRateLimitStatus` | `RateLimitStatus` | Current rate limit state |
| `ToolExecutionEntry` | `ExecutionEntry` | Execution history record |

### Cancellation

`CancellationToken` maps to the existing `eggsec.CancellationToken` type. It
is shared between the tool-core layer and the engine dispatch layer.

## Tool Descriptors

`ToolDescriptor` describes a single tool (operation) for registry-driven
invocation. It is generated from `OperationMetadata` and contains:

| Field | Type | Description |
|-------|------|-------------|
| `tool_id` | `str` | Canonical tool identifier (e.g., `"scan_ports"`) |
| `label` | `str` | Human-readable label |
| `description` | `str` | Operation description |
| `target_types` | `list[ToolTargetType]` | Supported target types |
| `parameter_schema` | `dict` | JSON Schema for parameters |
| `result_schema` | `dict` | JSON Schema for results |
| `risk` | `str` | Risk level (`safe_active`, `moderate`, `intrusive`) |
| `required_features` | `list[str]` | Cargo features required |
| `supported_surfaces` | `list[str]` | Execution surfaces |

## ToolRegistry

`ToolRegistry` provides lookup and enumeration of all registered tool
descriptors:

```python
from eggsec import ToolRegistry

# List all registered tools
tools = ToolRegistry.all_tools()
for t in tools:
    print(f"{t.tool_id}: {t.label} (risk={t.risk})")

# Find by tool ID
desc = ToolRegistry.find("scan_ports")
print(desc.description)

# Find by operation ID (alias)
desc = ToolRegistry.find_by_operation("scan-ports")
```

## Schema Generation

`SchemaGenerator` produces JSON Schema from tool descriptors:

```python
from eggsec import SchemaGenerator

# Generate request schema for an operation
schema = SchemaGenerator.request_schema("scan_ports")
print(schema)  # JSON Schema dict

# Generate response schema
schema = SchemaGenerator.response_schema("scan_ports")
print(schema)

# Generate full tool manifest (all operations)
manifest = SchemaGenerator.full_manifest()
for tool_id, schemas in manifest.items():
    print(f"{tool_id}: request={bool(schemas['request'])}, response={bool(schemas['response'])}")
```

## Tool Invocation

### Engine.invoke_tool()

Synchronous tool invocation through the policy gate:

```python
from eggsec import Engine, Scope, ToolRequest, ToolTarget, ToolRequestOptions

scope = Scope.allow_hosts(["127.0.0.1"])
engine = Engine(scope)

# Build a tool request
target = ToolTarget.ip("127.0.0.1")
request = ToolRequest.new(
    tool="scan_ports",
    target=target,
    params={"ports": [22, 80, 443]},
    options=ToolRequestOptions.new(timeout_ms=5000),
)

# Invoke through the policy gate
response = engine.invoke_tool(request)
if response.is_success():
    for finding in response.findings:
        print(f"[{finding.severity}] {finding.title}")
else:
    for error in response.errors:
        print(f"Error: {error.code} - {error.message}")
```

### AsyncEngine.async_invoke_tool()

Asynchronous tool invocation:

```python
import asyncio
from eggsec import AsyncEngine, Scope, ToolRequest, ToolTarget

async def main():
    scope = Scope.allow_hosts(["127.0.0.1"])
    engine = AsyncEngine(scope)

    target = ToolTarget.ip("127.0.0.1")
    request = ToolRequest.new(
        tool="scan_ports",
        target=target,
        params={"ports": [22, 80, 443]},
    )

    response = await engine.async_invoke_tool(request)
    print(response.status)

asyncio.run(main())
```

## Async Streaming Example

For operations that support streaming progress events:

```python
import asyncio
from eggsec import AsyncEngine, Scope

async def stream_example():
    scope = Scope.allow_hosts(["127.0.0.1"])
    engine = AsyncEngine(scope)

    # Run a scan with progress tracking
    future = engine.async_invoke_tool("scan_ports", "127.0.0.1", timeout_ms=10000)
    result = await future

    if result.error:
        print(f"Error: {result.error.kind}: {result.error.message}")
    else:
        print(f"Status: {result.status}")
        print(f"Target: {result.target}")

asyncio.run(stream_example())
```

## Framework Adapters

The tool abstraction layer enables framework adapters for MCP, REST, gRPC, and
agent surfaces. Each adapter:

1. Receives a tool invocation request in its native format.
2. Converts it to a `ToolRequest`.
3. Calls `Engine.invoke_tool()` or `AsyncEngine.async_invoke_tool()`.
4. Converts the `ToolResponse` back to the framework's response format.

This eliminates per-surface dispatch logic and ensures all surfaces share the
same policy gate and audit contract.

### OpenAPI Adapter

Generate OpenAPI 3.0 specifications from tool descriptors:

```python
from eggsec import OpenApiAdapter, ToolRegistry

# Generate OpenAPI for a single tool
path_item = OpenApiAdapter.tool_to_openapi("scan_ports")
print(path_item["post"]["summary"])  # "Port Scan"

# Generate a full OpenAPI spec for all tools
spec = OpenApiAdapter.full_openapi_spec()
print(f"OpenAPI version: {spec['openapi']}")
print(f"Paths: {len(spec['paths'])}")

# Use in a REST framework
for tool_id, path_item in spec["paths"].items():
    op = path_item["post"]
    print(f"{tool_id}: risk={op.get('x-eggsec-risk', 'unknown')}")
```

### Custom Framework Adapter Pattern

```python
from eggsec import Engine, Scope, ToolRequest, ToolTarget, ToolRegistry

def mcp_tool_handler(tool_name: str, arguments: dict) -> dict:
    """Convert an MCP tool invocation to an eggsec tool call."""
    desc = ToolRegistry.get(tool_name)
    if desc is None:
        return {"error": f"Unknown tool: {tool_name}"}

    # Build a ToolRequest from MCP arguments
    target_value = arguments.pop("target", "")
    target = ToolTarget.url(target_value) if target_value.startswith("http") else ToolTarget.ip(target_value)
    request = ToolRequest.new(tool=tool_name, target=target, params=arguments)

    # Invoke through the policy gate
    engine = Engine(Scope.allow_hosts(["*"]))
    result = engine.invoke_tool_request(request)

    # Convert result to MCP format
    return {
        "content": [{"type": "text", "text": result.status}],
        "isError": result.error is not None,
    }
```

## Migration: api_surface() to ToolRegistry

`api_surface()` remains available for stability introspection. For operation
discovery and invocation, use `ToolRegistry`:

```python
# Before: api_surface() for stability info
surface = eggsec.api_surface()
print(surface["scan_ports"]["stability"])  # "stable"

# After: ToolRegistry for tool discovery + invocation
from eggsec import ToolRegistry, Engine, ToolRequest, ToolTarget

desc = ToolRegistry.find("scan_ports")
print(desc.risk)  # "safe_active"

# Build and invoke
engine = Engine(Scope.allow_hosts(["127.0.0.1"]))
request = ToolRequest.new(
    tool=desc.tool_id,
    target=ToolTarget.ip("127.0.0.1"),
    params={"ports": [80, 443]},
)
response = engine.invoke_tool(request)
```

`api_surface()` and `ToolRegistry` are complementary: `api_surface()` reports
stability classifications, while `ToolRegistry` provides the operational tool
descriptor and invocation path.
