# API Schema Module

Deep dive: standalone OpenAPI 3.0 parser for tooling/report integration.

Parent overview: [overview.md](overview.md). Related: [fuzzer.md](fuzzer.md).

## Role & Responsibilities

Standalone OpenAPI 3.0 (JSON/YAML) document parser that produces a lightweight schema model and derives concrete fuzz targets — independent of any specific fuzzing engine. Designed for report/tooling integration, not live fuzzing.

Key capabilities:
- Parse OpenAPI 3.0 JSON and YAML documents
- Extract endpoints, parameters (path/query/header/cookie), request bodies, and security schemes
- Generate `FuzzTarget` values (method + URL + injectable parameter positions)
- Deliberately non-interoperable with `fuzzer::api_schema` (different type hierarchy, different purpose)

## Location & Feature Gating

- Source: `crates/eggsec/src/api_schema/mod.rs` (single file, ~457 lines incl. tests)
- Feature gate: `api-schema` (`lib.rs:81`)
- Always-declared module; without the feature it compiles as an empty/private stub

## Architecture

### Public Types

| Type | File:Line | Fields |
|------|-----------|--------|
| `ApiSchema` | `mod.rs:40` | `title: Option<String>`, `version: Option<String>`, `base_url: Option<String>`, `endpoints: Vec<ApiEndpoint>`, `security_schemes: Vec<SecurityScheme>` |
| `ApiEndpoint` | `mod.rs:4` | `path`, `method`, `operation_id: Option<String>`, `summary: Option<String>`, `parameters: Vec<ApiParameter>`, `request_body: Option<ApiRequestBody>`, `tags: Vec<String>` |
| `ApiParameter` | `mod.rs:15` | `name`, `location: ParameterLocation`, `required: bool`, `schema: Option<serde_json::Value>`, `description: Option<String>` |
| `ParameterLocation` | `mod.rs:25` | `Query`, `Header`, `Path`, `Cookie` (4 variants, `serde` renamed to snake_case) |
| `ApiRequestBody` | `mod.rs:33` | `content_type: Option<String>`, `schema: Option<serde_json::Value>`, `required: bool` |
| `SecurityScheme` | `mod.rs:49` | `name`, `scheme_type: String`, `location: Option<String>` |
| `FuzzTarget` | `mod.rs:284` | `path`, `method`, `parameter`, `location` (string repr of ParameterLocation), `schema_hint: Option<String>` |

### Public Functions

| Function | File:Line | Signature |
|----------|-----------|-----------|
| `parse_openapi()` | `mod.rs:55` | `(content: &str, is_yaml: bool) -> anyhow::Result<ApiSchema>` |
| `generate_fuzz_targets()` | `mod.rs:255` | `(schema: &ApiSchema) -> Vec<FuzzTarget>` |

### Internal Helpers

| Function | File:Line | Purpose |
|----------|-----------|---------|
| `parse_operation()` | `mod.rs:105` | Parses a single OpenAPI operation into `ApiEndpoint` |
| `parse_parameter()` | `mod.rs:160` | Parses a parameter object; defaults location to `Query` on unknown `in` value |
| `parse_request_body()` | `mod.rs:192` | Parses `requestBody` with first content-type and its schema |
| `parse_security_schemes()` | `mod.rs:218` | Extracts from `components.securitySchemes`; handles `apiKey` and `http` types |

### Parsing Logic

**JSON/YAML detection** (`mod.rs:56-61`): If `is_yaml`, deserializes via `serde_yaml_neo` then converts to `serde_json::Value`. Otherwise directly parses JSON.

**Endpoint extraction** (`mod.rs:83-92`): Iterates `paths` object; for each path, checks methods `get`, `post`, `put`, `delete`, `patch`, `options`, `head`.

**Security scheme extraction** (`mod.rs:218-253`): Reads `components.securitySchemes`; for `apiKey` type extracts `in` field, for `http` type extracts `scheme` field (defaults to `"bearer"`).

**Fuzz target generation** (`mod.rs:255-281`): For each endpoint parameter, creates a `FuzzTarget` with parameter name and location. For request bodies, creates a target with `parameter: "request_body"` and `location: "body"`.

---

## Relationship to `fuzzer::api_schema`

There are deliberately **two independent OpenAPI models** in the codebase:

| | `api_schema` (this module) | `fuzzer::api_schema` |
|--|---------------------------|----------------------|
| **Gate** | `api-schema` feature | Always compiled |
| **Focus** | Parsing + target generation | Schema-aware payload selection inside the fuzz engine |
| **Endpoint fields** | `operation_id`, `summary`, `tags` | `security` field, no display metadata |
| **Parameter fields** | `name`, `location`, `required`, `schema` (raw JSON), `description` | `name`, `location`, `required`, `param_type`, `format`, `example`, `min_value`, `max_value`, `pattern`, `enum_values` |
| **Security** | `SecurityScheme` enum (name, type, location) | `Vec<String>` of security scheme names on endpoint |
| **Request body** | `ApiRequestBody` (content_type, schema, required) | `RequestBody` (content_type, schema, required) — same shape |
| **Has fuzzer** | No (generates targets only) | Yes (`ApiSchemaFuzzer` with `fuzz_endpoint()`, type-aware payloads, auth bypass, oversized payloads) |
| **Parameter location enum** | `ParameterLocation` (Query, Header, Path, Cookie) | `ParamLocation` (Path, Query, Header, Cookie, Body) — includes `Body` |
| **Dependency on fuzzer types** | None | Depends on `fuzzer::payloads::{Payload, PayloadType}` |

They are not interoperable by design; converting between them would couple the standalone parser to fuzzer internals. Consumers choose based on need: report/tooling integration uses this module; live fuzzing uses the fuzzer-internal one.

---

## Behavior / Flow

### Parse → Generate Pipeline

```
OpenAPI content (JSON/YAML string)
  │
  ├─ is_yaml? → serde_yaml_neo::from_str() → serde_json::to_value()
  └─ else     → serde_json::from_str()
  │
  ▼
serde_json::Value
  │
  ├─ Extract info.title, info.version
  ├─ Extract servers[0].url → base_url
  ├─ Extract paths → for each path × method:
  │   ├─ parse_operation() → ApiEndpoint
  │   │   ├─ operationId, summary, tags
  │   │   ├─ parameters → parse_parameter() for each
  │   │   └─ requestBody → parse_request_body()
  │   └─ push to endpoints
  ├─ Extract components.securitySchemes → parse_security_schemes()
  │
  ▼
ApiSchema { title, version, base_url, endpoints, security_schemes }
  │
  ▼
generate_fuzz_targets(&schema) → Vec<FuzzTarget>
  │
  ├─ For each endpoint × parameter → FuzzTarget { path, method, parameter, location, schema_hint }
  └─ For each endpoint × request_body → FuzzTarget { path, method, "request_body", "body", schema_hint }
```

---

## Public API

### parse_openapi

```rust
pub fn parse_openapi(content: &str, is_yaml: bool) -> anyhow::Result<ApiSchema>
```

Parses an OpenAPI 3.0 document. JSON auto-detection is caller-controlled via `is_yaml` flag. Returns `anyhow::Result` (uses `anyhow` not `EggsecError`).

### generate_fuzz_targets

```rust
pub fn generate_fuzz_targets(schema: &ApiSchema) -> Vec<FuzzTarget>
```

Derives injectable positions from parsed schema. Each parameter becomes a target; request bodies become a single `request_body` target.

---

## Integration Points

### Dispatch

Used by the tool registry to generate fuzz targets from OpenAPI specs provided via REST/MCP/gRPC. The `handle_fuzz` command handler can accept `--schema <path>` to load an OpenAPI document and generate targets.

### Pipeline

Pipeline stages can use `parse_openapi()` + `generate_fuzz_targets()` to auto-discover endpoints before running targeted fuzzing.

### Report Generation

The `FuzzTarget` output is compatible with the findings store and output report formats (JSON/SARIF/HTML).

---

## Testing

6 unit tests covering:

| Test | What it verifies |
|------|------------------|
| `parse_openapi_json` | JSON parsing: title, version, base_url, endpoints count (2), security_schemes count (1) |
| `parse_openapi_yaml` | YAML parsing: same fields, different values |
| `generate_fuzz_targets_from_schema` | Targets include `limit` param and `request_body` |
| `parse_parameters_correctly` | Parameter fields: name, location, required, operation_id, summary, tags |
| `parse_request_body_correctly` | Body: required=true, content_type=application/json, schema present |
| `parse_security_schemes_correctly` | Bearer auth: name=bearerAuth, type=http, scheme=bearer |
| `parse_minimal_schema` | Empty paths → no endpoints, no title/version/base_url |

---

## Invariants & Gotchas

1. **Feature-gated**: Module compiles as empty stub without `api-schema` feature — consumers must `cfg(feature = "api-schema")`
2. **`anyhow::Result` not `EggsecError`**: This module uses `anyhow` for error handling, unlike most engine modules that use `EggsecError`. This is deliberate for the standalone parsing use case.
3. **No validation of OpenAPI version**: Accepts any document with `paths` object — does not verify `openapi: "3.0.x"` field
4. **Single server only**: `base_url` takes only `servers[0].url` — ignores additional servers
5. **First content-type wins**: `parse_request_body()` takes the first entry from the `content` object
6. **Security scheme `location`**: For `apiKey` type, `location` is the `in` field; for `http` type, `location` is the `scheme` field (e.g., "bearer"); for other types, `location` is `None`
7. **No `$ref` resolution**: Does not follow `$ref` pointers — inline schemas only
8. **Parameter location fallback**: Unknown `in` values default to `Query` (`mod.rs:171`)
9. **Deliberate non-interoperability**: `api_schema::FuzzTarget` and `fuzzer::api_schema::SchemaFuzzTarget` are unrelated types — converting requires manual mapping

*Last verified against source: 2026-08-25*
