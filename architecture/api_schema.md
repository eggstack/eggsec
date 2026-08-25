# API Schema Module

Deep dive: standalone OpenAPI-driven test-target generation.

Parent overview: [overview.md](overview.md). Related: [fuzzer.md](fuzzer.md).

## Location & Gating

- Source: `crates/eggsec/src/api_schema/mod.rs` (single module, ~460 lines incl. tests)
- Feature gate: `api-schema` (`lib.rs:81`)
- Always-declared module; without the feature it compiles as an empty/private stub

## Purpose

Parse OpenAPI 3.0 documents (JSON or YAML) into a lightweight schema model and derive concrete fuzz targets from it — independent of any specific fuzzing engine.

## Public Surface

| Item | Kind | Notes |
|------|------|-------|
| `ApiSchema` | struct | Document root: `title`, `version`, `base_url`, endpoints |
| `ApiEndpoint` | struct | Path + method + `operation_id` + `summary` + `tags` + parameters |
| `ApiParameter` | struct | Name, location (path/query/header/body), type info, required flag |
| `SecurityScheme` | enum | Auth scheme classification for the document |
| `parse_openapi()` | fn | JSON/YAML detection and parsing into `ApiSchema` |
| `generate_fuzz_targets()` | fn | Produces `FuzzTarget` values (method + URL + injectable parameter positions) |

## Relationship to `fuzzer/api_schema`

There are deliberately **two independent OpenAPI models**:

| | `api_schema` (this doc) | `fuzzer::api_schema` |
|--|------------------------|------------------------|
| Gate | `api-schema` feature | always compiled |
| Focus | parsing + target generation | schema-aware payload selection inside the fuzz engine |
| Endpoint fields | `operation_id`, `summary`, `tags` | `security` field, no display metadata |
| Types | own hierarchy | depends on `fuzzer::payloads::{Payload, PayloadType}` |

They are not interoperable by design; converting between them would couple the standalone parser to fuzzer internals. Consumers choose based on need: report/tooling integration uses this module; live fuzzing uses the fuzzer-internal one.

## Testing

Unit tests cover JSON/YAML parsing round-trips, parameter extraction, security-scheme detection, and fuzz-target derivation (see `#[cfg(test)]` block in `mod.rs`).
