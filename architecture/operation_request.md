# Operation Request Contracts

## Overview

Canonical, dependency-light execution-parameter contracts shared across every frontend. Defaults, validation, and normalization live once in `eggsec-tool-core::operation_request`; the engine facade at `crates/eggsec/src/operation_request.rs` (589 lines) only adapts frontend parse shapes (Clap args, runtime `TaskKind` params, `ToolRequest` JSON, Python DTOs) into those canonical types.

The module is **policy-free** — it produces a normalized request but never authorizes it. Callers must still build an `OperationDescriptor` via `OperationMetadata::try_descriptor_for_target` and evaluate through `EnforcementContext` before execution. See [config.md](config.md) and [overview.md](overview.md).

## Role & Responsibilities

- **Single owner for defaults**: `DEFAULT_*` consts (ports, concurrency, timeouts, fuzz/GraphQL/OAuth settings) live in `eggsec-tool-core`, not in CLI/TUI/runtime duplicates.
- **Pure normalization**: `parse_port_spec`, `resolve_load_test_counts`, `parse_scan_type`, `normalize_timeout_*`, `normalize_concurrency`, `parse_scan_profile`, `normalize_target_value` — pure functions with bounds enforcement.
- **Typed requests per family**: `PortScanRequest`, `EndpointScanRequest`, `FingerprintRequest`, `FuzzRequest`, `WafDetectRequest`, `WafStressRequest`, `LoadTestRequest`, `ReconRequest`, `GraphQlRequest`, `OAuthRequest`, `AuthTestRequest`, `PipelineRequest`, … each with serde support and a `normalize()` returning a validated `Normalized*` value.
- **Shallow frontend adapters**: `operation_request::cli_adapters` (Clap args → canonical), `operation_request::runtime_adapters` (`TaskKind` params → canonical), plus `ToolRequest`/Python adapters — no policy logic, no defaults re-declared.

## Location & Feature Gating

| Component | Path | Feature Gate |
|-----------|------|-------------|
| Canonical contracts + defaults | `crates/eggsec-tool-core/src/operation_request.rs` (~1260 lines) | Always |
| Engine facade + re-export | `crates/eggsec/src/operation_request.rs` | Always |
| `cli_adapters` | `operation_request.rs:cli_adapters` | `cli` |
| `runtime_adapters` | `operation_request.rs:runtime_adapters` | `cli` |

No authorization, no I/O, no network in this layer.

## Architecture

### Canonical defaults (single owner)

`eggsec-tool-core::operation_request` owns:

- Ports: `DEFAULT_PORT_SCAN_PORTS = "1-1024"`, `DEFAULT_FINGERPRINT_PORTS = "80,443,22,…"`, `MAX_PORT_COUNT = 65_535`.
- Concurrency per family: port-scan 100, endpoint/fingerprint 20, fuzz/GraphQL/OAuth/WAF 10, WAF-stress 20, load 10, auth 1.
- Timeouts: port-scan 2s, endpoint/fuzz/auth 10s, WAF/GraphQL/OAuth 15s, load 30s, hunt 30s; bounds `1–600s`, `100–600_000ms`, concurrency `1–1000`.
- Load: `DEFAULT_LOAD_REQUESTS = 100`. Fuzz: payload `all`, mode `sequential`, method `GET`, mutations 3. GraphQL introspection/depth-bypass/alias-overload default `true`.

Mirrors CLI `timeout.rs`/arg defaults by design — CLI must not re-declare these.

### Adapter layers (engine facade)

1. **CLI adapters** (`cli_adapters`): `port_scan_from_cli`, `endpoint_scan_from_cli`, `fingerprint_from_cli`, `fuzz_from_cli`, `waf_detect_from_cli`, `waf_stress_from_cli`, `load_test_from_cli`, `recon_from_cli`, `graphql_from_cli`, `oauth_from_cli`, `auth_test_from_cli` — field-by-field copies, `Option`-wrapped so canonical `normalize()` applies defaults.
2. **Runtime adapters** (`runtime_adapters`): `operation_id_for_task_kind` / `target_for_task_kind` thinly delegate to `TaskKind::operation_id()` / `TaskKind::canonical_target()` (single wire-side match lives in `eggsec-runtime`; no parallel table), plus `*_from_runtime` converters per family.
3. **Tool/Python adapters**: `ToolRequest.params` JSON and Python DTOs convert through the same canonical structs, so REST/MCP/gRPC/Python schemas stay consistent.

### Exhaustiveness invariant

Every conversion is exhaustive: adding a new `TaskKind` variant or canonical operation without updating the mapping is a compile error (no wildcard fallback for supported kinds). Documented at `operation_request.rs:12-17`.

## Behavior / Flow

```
CLI args / TaskKind params / ToolRequest JSON / Python DTO
        │ shallow adapter (no defaults, no policy)
        ▼
Canonical request (e.g. FuzzRequest)
        │ .normalize() → defaults + bounds + validation
        ▼
Normalized* value
        │ OperationMetadata::try_descriptor_for_target + EnforcementContext::evaluate()
        ▼
ApprovedOperation → dispatch
```

Policy-relevant and execution-relevant normalization happens here, once. Authorization happens downstream.

## Public API

```rust
// Re-exported from eggsec-tool-core (operation_request.rs:19)
pub use eggsec_tool_core::operation_request::*;

pub mod cli_adapters { /* *_from_cli() per family */ }
pub mod runtime_adapters {
    pub fn operation_id_for_task_kind(kind: &TaskKind) -> Option<&'static str>;
    pub fn target_for_task_kind(kind: &TaskKind) -> Option<String>;
    /* *_from_runtime() per family */
}
```

Canonical request structs expose `normalize()`; normalization helpers (`parse_port_spec`, `normalize_concurrency`, …) are pure and frontend-neutral.

## Integration Points

- **CLI**: `cli/` arg structs convert via `cli_adapters` before dispatch. See [cli_commands.md](cli_commands.md).
- **Runtime bridge**: `runtime_bridge/` uses `operation_id_for_task_kind` / `target_for_task_kind` to build `OperationDescriptor`s. See [runtime_bridge.md](runtime_bridge.md).
- **Dispatch**: `dispatch/` workers receive normalized requests; executors never re-normalize. See [dispatch.md](dispatch.md).
- **Tool protocols**: REST/MCP/gRPC `ToolRequest` params deserialize into canonical structs. See [ai_agents.md](ai_agents.md).
- **Python**: `eggsec-python` DTOs map into the same canonical types. See [python_api.md](python_api.md).
- **Enforcement**: `config/policy_catalog.rs` + `policy_decision.rs` consume the normalized target/operation. See [config.md](config.md).

## Testing

- Normalization unit tests live alongside `eggsec-tool-core::operation_request` (bounds, defaults, port-spec parsing, profile parsing).
- Exhaustiveness is compile-enforced: new `TaskKind`/operation variants break the build until adapters are updated.
- CLI parity tests assert adapter output matches CLI defaults (e.g. `DEFAULT_FUZZ_*`).

## Invariants & Gotchas

1. **Defaults live here, nowhere else** — do not re-declare timeouts/concurrency in CLI/TUI/runtime.
2. **No authorization here** — never call `EnforcementContext` from adapters; build the descriptor upstream.
3. **Shallow adapters only** — adapters copy fields; all validation/bounds in `normalize()`.
4. **`None` means "apply default"** — adapters pass through `Option`s; only `normalize()` fills them.
5. **Single wire-side match** — `TaskKind::operation_id()`/`canonical_target()` own identity; engine wrappers delegate, never duplicate.

## Cross-Links

- [overview.md](overview.md) — system architecture, how-an-operation-flows
- [config.md](config.md) — `OperationMetadata`, `EnforcementContext`, policy evaluation
- [dispatch.md](dispatch.md) — executor layer consuming normalized requests
- [runtime_bridge.md](runtime_bridge.md) — `TaskKind` → descriptor conversion
- [cli_commands.md](cli_commands.md) — CLI arg shapes entering the adapters
- [ai_agents.md](ai_agents.md) — tool-protocol request shapes

---

*Last verified against source: 2026-09-11*
