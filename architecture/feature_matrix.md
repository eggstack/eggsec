# Feature Matrix

Comprehensive reference for all Cargo feature flags in the `eggsec` crate.

## Summary

| Metric | Count |
|--------|-------|
| Total features | 29 |
| Features with deps | 16 |
| Marker-only features | 13 |
| In `full` | 17 |

## Feature Table

| Feature | Declared | Has deps | In `full` | Primary module | Stability | Build command |
|---------|----------|----------|-----------|----------------|-----------|---------------|
| `default` | yes | no | - | (core) | Stable | `cargo check -p eggsec` |
| `tool-api` | yes | no | - | `tool/` | Stable | `cargo check -p eggsec --features tool-api` |
| `insecure-tls` | yes | no | - | `utils/` | Testing-only | `cargo check -p eggsec --features insecure-tls` |
| `rest-api` | yes | yes | yes | `tool/`, `agent/` | Stable | `cargo check -p eggsec --features rest-api` |
| `ws-api` | yes | yes | - | (WebSocket API) | Beta | `cargo check -p eggsec --features ws-api` |
| `grpc-api` | yes | yes | - | `tool/` | Stable | `cargo check -p eggsec --features grpc-api` |
| `stress-testing` | yes | yes | yes | `stress/`, `packet/` | Stable | `cargo check -p eggsec --features stress-testing` |
| `packet-inspection` | yes | yes | yes | `packet/` | Stable | `cargo check -p eggsec --features packet-inspection` |
| `nse` | yes | yes | yes | `eggsec-nse` | Stable | `cargo check -p eggsec --features nse` |
| `nse-ssh2` | yes | yes | - | `eggsec-nse` | Stable | `cargo check -p eggsec --features nse-ssh2` |
| `nse-sandbox` | yes | yes | - | `eggsec-nse` | Stable | `cargo check -p eggsec --features nse-sandbox` |
| `ai-integration` | yes | yes | yes | `ai/` | Stable | `cargo check -p eggsec --features ai-integration` |
| `websocket` | yes | yes | yes | `websocket/` | Stable | `cargo check -p eggsec --features websocket` |
| `headless-browser` | yes | yes | yes | `browser/` | Stable | `cargo check -p eggsec --features headless-browser` |
| `database` | yes | yes | yes | `storage/` | Stable | `cargo check -p eggsec --features database` |
| `container` | yes | yes | yes | `container/` | Stable | `cargo check -p eggsec --features container` |
| `cloud` | yes | no | - | `recon/cloud/` | Stable | `cargo check -p eggsec --features cloud` |
| `sbom` | yes | yes | yes | `supply_chain/` | Stable | `cargo check -p eggsec --features sbom` |
| `advanced-hunting` | yes | no | yes | `hunt/` | Stable | `cargo check -p eggsec --features advanced-hunting` |
| `compliance` | yes | no | yes | `compliance/` | Stable | `cargo check -p eggsec --features compliance` |
| `external-integrations` | yes | no | yes | `integrations/` | Stable | `cargo check -p eggsec --features external-integrations` |
| `finding-workflow` | yes | no | yes | `workflow/` | Stable | `cargo check -p eggsec --features finding-workflow` |
| `vuln-management` | yes | no | yes | `vuln/` | Stable | `cargo check -p eggsec --features vuln-management` |
| `git-secrets` | yes | no | - | `recon/git_secrets.rs` | Stable | `cargo check -p eggsec --features git-secrets` |
| `wireless` | yes | no | - | `wireless/` | Stable | `cargo check -p eggsec --features wireless` (passive; supports --repeat, --known-good, --dry-run, --detect-suspicious; WPS/hidden/transition/rogue heuristic). **Passive Phase 0 (2026-06-11)**; active phases gated by `wireless-advanced` per `plans/wireless-active-attacks-loadout-design-plan.md`. |
| `wireless-advanced` | yes | yes (`wireless`) | - | `wireless/active/` | Stable | `cargo check -p eggsec --features wireless-advanced` (active deauth/disassoc under `wireless <iface> deauth`; lab-only, requires `--allow-active-wireless`, monitor-mode interface, root/CAP_NET_ADMIN; dry-run default; policy gate `Intrusive` + `wireless-advanced` feature; TUI active mode with the same task/confirmation flow). **Phase 1 complete 2026-06-12**; Phase 2+ (handshake capture, etc.) per `plans/wireless-active-attacks-loadout-design-plan.md`. Not included in `full`. |
| `mobile` | yes | no | yes | `mobile/` | Stable | `cargo check -p eggsec --features mobile` **Static Phase 1 complete 2026-06-11**; dynamic loadout shipped under `mobile-dynamic` (Phase 1 + Phase 2a + final polish + close-out polish complete 2026-06-12; design per `plans/dynamic-mobile-testing-loadout-design-plan.md`; like wireless active). |
| `db-pentest` | yes | yes (`sqlx` shared) | yes | `db_pentest/` | Stable | `cargo check -p eggsec --features db-pentest` (Phase 1: direct Postgres/MySQL checks + manifest + bridge; standalone defense-lab; dry-run safe; real requires `--allow-db-pentest`; local types + optional `to_scan_report_data_db` bridge auto-wired in report convert; no TUI/pipeline/MCP in Phase 1). See `plans/database-pentesting-phase1-foundation-handoff-plan.md` (executed) + `plans/non-web-database-pentesting-loadout-design-plan.md`. |
| `pdf` | yes | yes | - | `output/` | Stable | `cargo check -p eggsec --features pdf` |
| `api-schema` | yes | no | - | `api_schema/` | Stable | `cargo check -p eggsec --features api-schema` |
| `full` | yes | yes | - | (all) | Deprecated | `cargo check -p eggsec --features full` |

## Stability Levels

| Level | Description |
|-------|-------------|
| **Stable** | Fully implemented, well-tested, safe for production use |
| **Beta** | Functional but may have edge cases; additional wiring may be needed |
| **Testing-only** | For development/testing only; never use in production |
| **Deprecated** | Being phased out or currently non-functional |

## Notes

### `full` feature

The `full` feature enables 17 sub-features. It does not include `grpc-api`, `ws-api`, or `pdf`.

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
`finding-workflow`, `vuln-management`, `cloud`, `git-secrets`, `wireless`, `mobile` (dynamic per `plans/dynamic-mobile-testing-loadout-design-plan.md`), and `db-pentest` (Phase 1: postgres/mysql + manifest + bridge; standalone defense-lab; see `plans/database-pentesting-phase1-foundation-handoff-plan.md` (executed) + `plans/non-web-database-pentesting-loadout-design-plan.md`)
have no extra runtime dependencies beyond optional crates (`zip`/`plist` for `mobile`; sqlx is shared with the `database` feature for `db-pentest`).
They gate module compilation via `#[cfg(feature = "...")]` in `lib.rs`.

`wireless-advanced` is a dependent feature on `wireless` and pulls in the
`wireless/active/` module (deauth/disassoc frame crafting and injection). It is
intentionally **not** in `full` because active attacks are lab-only and require
`--allow-active-wireless` plus a monitor-mode interface, so it is opted into
explicitly at build time.

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
