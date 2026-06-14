# Feature Matrix

Comprehensive reference for all Cargo feature flags in the `eggsec` crate.

## Summary

| Metric | Count |
|--------|-------|
| Total features | 40 |
| Features with deps | 18 |
| Marker-only features | 22 |
| In `full` | 21 |

## Feature Table

| Feature | Declared | Has deps | In `full` | Primary module | Stability |
|---------|----------|----------|-----------|----------------|-----------|
| `default` | yes | no | - | (core) | Stable |
| `tool-api` | yes | no | - | `tool/` | Stable |
| `insecure-tls` | yes | no | - | `utils/` | Testing-only |
| `rest-api` | yes | yes | yes | `tool/`, `agent/` | Stable |
| `ws-api` | yes | yes | - | (WebSocket API) | Beta |
| `grpc-api` | yes | yes | - | `tool/` | Stable |
| `stress-testing` | yes | yes | yes | `stress/`, `packet/` | Stable |
| `packet-inspection` | yes | yes | yes | `packet/` | Stable |
| `nse` | yes | yes | yes | `eggsec-nse` | Stable |
| `nse-ssh2` | yes | yes | - | `eggsec-nse` | Stable |
| `nse-sandbox` | yes | yes | - | `eggsec-nse` | Stable |
| `ai-integration` | yes | yes | yes | `ai/` | Stable |
| `websocket` | yes | yes | yes | `websocket/` | Stable |
| `headless-browser` | yes | yes | yes | `browser/` | Stable |
| `database` | yes | yes | yes | `storage/` | Stable |
| `container` | yes | yes | yes | `container/` | Stable |
| `cloud` | yes | no | - | `recon/cloud/` | Stable |
| `sbom` | yes | yes | yes | `supply_chain/` | Stable |
| `advanced-hunting` | yes | no | yes | `hunt/` | Stable |
| `compliance` | yes | no | yes | `compliance/` | Stable |
| `external-integrations` | yes | no | yes | `integrations/` | Stable |
| `finding-workflow` | yes | no | yes | `workflow/` | Stable |
| `vuln-management` | yes | no | yes | `vuln/` | Stable |
| `git-secrets` | yes | no | - | `recon/git_secrets.rs` | Stable |
| `wireless` | yes | no | yes | `wireless/` | Stable |
| `wireless-advanced` | yes | yes (`wireless`) | yes | `wireless/active/` | Stable |
| `mobile` | yes | no | yes | `mobile/` | Stable |
| `mobile-dynamic` | yes | yes (`mobile`) | yes | `mobile/dynamic.rs` | Stable |
| `db-pentest` | yes | yes (`sqlx`) | yes | `db_pentest/` | Stable |
| `db-pentest-mssql-tiberius` | yes | yes | - | `db_pentest/mssql.rs` | Stable |
| `db-pentest-mongodb` | yes | yes | - | `db_pentest/mongodb.rs` | Stable |
| `db-pentest-redis` | yes | yes | - | `db_pentest/redis.rs` | Stable |
| `db-pentest-mcp` | yes | yes (`db-pentest`) | - | `tool/implementations/db_pentest.rs` | Stable |
| `web-proxy` | yes | yes | yes | `proxy/intercept/` | Stable |
| `web-proxy-mcp` | yes | yes (`web-proxy`) | - | `proxy/intercept/mcp.rs` | Stable |
| `transparent-proxy` | yes | yes (`web-proxy`) | - | `proxy/intercept/` | Stable |
| `dynamic-plugins` | yes | yes (`web-proxy`) | - | `proxy/intercept/` | Stable |
| `api-schema` | yes | no | - | `api_schema/` | Stable |
| `pdf` | yes | yes | - | `output/` | Stable |
| `full` | yes | yes | - | (all) | Deprecated |

## Stability Levels

| Level | Description |
|-------|-------------|
| **Stable** | Fully implemented, well-tested, safe for production use |
| **Beta** | Functional but may have edge cases; additional wiring may be needed |
| **Testing-only** | For development/testing only; never use in production |
| **Deprecated** | Being phased out or currently non-functional |

## Notes

### `full` feature

The `full` feature enables 21 sub-features. It does not include `grpc-api`, `ws-api`, or `pdf`.

Note: The `container` feature pulls in `k8s-openapi` which requires a Kubernetes version feature (e.g., `v1_30`) to be enabled. This must be provided by the final binary crate.

### `api-schema`

The `api-schema` feature enables the top-level `api_schema` module for standalone
OpenAPI 3.x schema ingestion and fuzz target generation. It uses manual JSON/YAML
parsing via `serde_json` and `serde_yaml_neo` (no external OpenAPI crate dependency).
The feature is independent of the always-compiled `fuzzer/api_schema/` and
`recon/api_schema.rs` modules.

### `ws-api`

The `ws-api` feature enables `axum/ws` for WebSocket API support. It is declared but
not included in `full`. The feature is functional but may need additional wiring for
full WebSocket pub/sub support.

### Marker-only features

Features like `advanced-hunting`, `compliance`, `external-integrations`,
`finding-workflow`, `vuln-management`, `cloud`, `git-secrets`, `wireless`, `mobile`,
`api-schema`, `db-pentest-mssql-tiberius`, `db-pentest-mongodb`, `db-pentest-redis`,
`transparent-proxy`, and `dynamic-plugins` have no extra runtime dependencies beyond
optional crates. They gate module compilation via `#[cfg(feature = "...")]` in `lib.rs`.

`wireless-advanced` is a dependent feature on `wireless` and pulls in the
`wireless/active/` module (deauth/disassoc frame crafting and injection). It is
included in `full`.

`mobile-dynamic` is a dependent feature on `mobile` and includes Android ADB core,
Frida instrumentation, and correlation engine.

`db-pentest` shares `sqlx` with the `database` feature. Sub-features
`db-pentest-mssql-tiberius`, `db-pentest-mongodb`, `db-pentest-redis` pull
additional database drivers. `db-pentest-mcp` enables MCP tool exposure.

`web-proxy` pulls `tokio-tungstenite`, `h2`, `http`, `prost`, and `prost-types` for
real WebSocket/HTTP2/gRPC interception. `web-proxy-mcp`, `transparent-proxy`, and
`dynamic-plugins` are dependent marker features.

### Module gating pattern

Feature-gated modules follow a dual-declaration pattern in `lib.rs`:

```rust
#[cfg(feature = "example")]
pub mod example;
#[cfg(not(feature = "example"))]
#[allow(dead_code)]
mod example;
```

This ensures the module always compiles (for internal use) but is only publicly
exposed when the feature is enabled.
