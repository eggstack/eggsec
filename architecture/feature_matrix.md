# Feature Matrix

Comprehensive reference for all Cargo feature flags in the `slapper` crate.

## Summary

| Metric | Count |
|--------|-------|
| Total features | 28 |
| Features with deps | 18 |
| Marker-only features | 12 |
| In `full` | 16 |

## Feature Table

| Feature | Declared | Has deps | In `full` | Primary module | Stability | Build command |
|---------|----------|----------|-----------|----------------|-----------|---------------|
| `default` | yes | no | - | (core) | Stable | `cargo check -p slapper` |
| `tool-api` | yes | no | - | `tool/` | Stable | `cargo check -p slapper --features tool-api` |
| `insecure-tls` | yes | no | - | `utils/` | Testing-only | `cargo check -p slapper --features insecure-tls` |
| `rest-api` | yes | yes | yes | `tool/`, `agent/` | Stable | `cargo check -p slapper --features rest-api` |
| `ws-api` | yes | yes | - | (WebSocket API) | Beta | `cargo check -p slapper --features ws-api` |
| `grpc-api` | yes | yes | - | `tool/` | Stable | `cargo check -p slapper --features grpc-api` |
| `stress-testing` | yes | yes | yes | `stress/`, `packet/` | Stable | `cargo check -p slapper --features stress-testing` |
| `packet-inspection` | yes | yes | yes | `packet/` | Stable | `cargo check -p slapper --features packet-inspection` |
| `nse` | yes | yes | yes | `slapper-nse` | Stable | `cargo check -p slapper --features nse` |
| `nse-ssh2` | yes | yes | - | `slapper-nse` | Stable | `cargo check -p slapper --features nse-ssh2` |
| `nse-sandbox` | yes | yes | - | `slapper-nse` | Stable | `cargo check -p slapper --features nse-sandbox` |
| `ai-integration` | yes | yes | yes | `ai/` | Stable | `cargo check -p slapper --features ai-integration` |
| `websocket` | yes | yes | yes | `websocket/` | Stable | `cargo check -p slapper --features websocket` |
| `headless-browser` | yes | yes | yes | `browser/` | Stable | `cargo check -p slapper --features headless-browser` |
| `database` | yes | yes | yes | `storage/` | Stable | `cargo check -p slapper --features database` |
| `container` | yes | yes | yes | `container/` | Stable | `cargo check -p slapper --features container` |
| `cloud` | yes | no | - | `recon/cloud/` | Stable | `cargo check -p slapper --features cloud` |
| `sbom` | yes | yes | yes | `supply_chain/` | Stable | `cargo check -p slapper --features sbom` |
| `advanced-hunting` | yes | no | yes | `hunt/` | Stable | `cargo check -p slapper --features advanced-hunting` |
| `compliance` | yes | no | yes | `compliance/` | Stable | `cargo check -p slapper --features compliance` |
| `external-integrations` | yes | no | yes | `integrations/` | Stable | `cargo check -p slapper --features external-integrations` |
| `finding-workflow` | yes | no | yes | `workflow/` | Stable | `cargo check -p slapper --features finding-workflow` |
| `vuln-management` | yes | no | yes | `vuln/` | Stable | `cargo check -p slapper --features vuln-management` |
| `git-secrets` | yes | no | - | `recon/git_secrets.rs` | Stable | `cargo check -p slapper --features git-secrets` |
| `wireless` | yes | no | - | `wireless/` | Stable | `cargo check -p slapper --features wireless` |
| `pdf` | yes | yes | - | `output/` | Stable | `cargo check -p slapper --features pdf` |
| `full` | yes | yes | - | (all) | Deprecated | `cargo check -p slapper --features full` |

## Stability Levels

| Level | Description |
|-------|-------------|
| **Stable** | Fully implemented, well-tested, safe for production use |
| **Beta** | Functional but may have edge cases; additional wiring may be needed |
| **Testing-only** | For development/testing only; never use in production |
| **Deprecated** | Being phased out or currently non-functional |

## Notes

### `full` feature (Deprecated)

The `full` feature is **Deprecated** and currently fails to compile due to a
pre-existing `k8s-openapi` issue: the `container` feature pulls in `k8s-openapi` which
requires a Kubernetes version feature (e.g., `v1_30`) to be enabled. This is not set
in `Cargo.toml` and must be provided by the final binary crate.

It enables 16 sub-features.

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
`finding-workflow`, `vuln-management`, `cloud`, `git-secrets`, and `wireless` have no
extra dependencies. They gate module compilation via `#[cfg(feature = "...")]` in
`lib.rs`.

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
