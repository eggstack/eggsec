# API Testing with OpenAPI Schemas

Import OpenAPI schemas for type-aware API testing.

## Schema Import

Parse OpenAPI 3.x JSON/YAML:

```rust
use slapper::api_schema::parse_openapi;

let schema = parse_openapi(openapi_content, false)?;
println!("Found {} endpoints", schema.endpoints.len());
```

## Fuzz Target Generation

Generate fuzz targets from schema:

```rust
use slapper::api_schema::generate_fuzz_targets;

let targets = generate_fuzz_targets(&schema);
for target in &targets {
    println!("{} {} - {}", target.method, target.path, target.parameter);
}
```

## Supported Features

- OpenAPI 3.0+ JSON and YAML
- Path/query/header/cookie/body parameters
- Request body schema extraction
- Security scheme detection (Bearer, API Key, OAuth)
- Type-aware payload generation hints

## Feature Flag

Enable with: `cargo build --features api-schema`
