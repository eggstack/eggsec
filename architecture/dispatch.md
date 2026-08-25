# Dispatch Layer

Deep dive: how requests become engine executions once policy has approved them.

Parent overview: [overview.md](overview.md). Related: [runtime_bridge.md](runtime_bridge.md), [cli_commands.md](cli_commands.md), [config.md](config.md).

## Role

`crates/eggsec/src/dispatch/` is the engine's **frontend-neutral execution layer**. It converts a `TaskKind` request into a call to the right engine module and returns a typed `TaskResult`. It performs **no authorization of its own** — every executor receives an already-evaluated `OperationDescriptor`, and all scope/policy decisions were made upstream by `EnforcementContext::evaluate()` (see [config.md](config.md)).

```
CLI handler ─────────────┐
TUI (via TuiTaskDispatcher) ──┤→ dispatch::dispatch_inner() → per-domain worker → engine module → TaskResult
REST/MCP/gRPC/Agent (EnforcedDispatcher::dispatch_checked) ─┤
Daemon/Runtime (runtime_bridge::approve_run_request → ApprovedRunRequest) ─┘
```

## File Layout

| File | Contents |
|------|----------|
| `mod.rs` | `dispatch_task()` (`mod.rs:63`) — creates progress + result channels, calls `dispatch_inner()` and forwards the `TaskResult`; `dispatch_inner()` (`mod.rs:101–388`) — the `TaskKind` router (29 match arms covering all task kinds, including feature-gated kinds that fall through to a "task kind unavailable" error when compiled out) |
| `types.rs` | `TaskResult` enum — typed result variants mirroring `TaskKind` plus `Error`; feature-gated variants compile out with their features |
| `executor.rs` | `OperationExecutor` trait |
| `executors/` | Registry-backed adapter layer (see below) |
| Domain workers | `scanner.rs`, `recon.rs`, `fuzzer.rs`, `network.rs`, `auth.rs`, `security.rs`, `api.rs`, `intercept.rs`, `nse` (gated), `db_pentest.rs` (gated), `c2.rs` (gated) |

## Executor Adapters

`executors/mod.rs` registers adapters used by the tool/registry path:

| Executor | Feature | Maps to |
|----------|---------|---------|
| `ScannerExecutor` | always | `scan-ports`, `scan-endpoints`, `fingerprint` |
| `ReconExecutor` | always | full recon pipeline ([recon.md](recon.md)) |
| `WafExecutor` | always | detect / bypass / stress ([waf.md](waf.md)) |
| `FuzzExecutor` | always | fuzz engine ([fuzzer.md](fuzzer.md)) |
| `NetworkExecutor` | always | load test / GraphQL / OAuth / auth-test |
| `NseExecutor` | `nse` | NSE runs ([nse_integration.md](nse_integration.md)) |
| `DbPentestExecutor` | `db-pentest` | database assessment ([database_pentest.md](database_pentest.md)) |

## Interaction With Enforcement

Three distinct paths reach engine functions, all post-authorization:

1. **Manual surfaces** — CLI/TUI handlers in `crates/eggsec/src/commands/handlers/` call `ctx.evaluate_and_enforce_operation(descriptor)` themselves, then invoke engine functions directly (not via `dispatch_inner`). See [cli_commands.md](cli_commands.md).
2. **Strict protocol surfaces** — REST/MCP/gRPC/agent route through `EnforcedDispatcher::dispatch_checked()` in `tool/dispatcher.rs`, which validates the `ApprovedOperation` token binding (tool ↔ canonical operation, target agreement) and fails closed before any executor runs.
3. **Runtime/daemon surfaces** — `runtime_bridge::approve_run_request()` converts `RunRequest` → descriptor, obtains an `ApprovedOperation`, and the resulting `ApprovedRunRequest` executes through this dispatch layer. See [runtime_bridge.md](runtime_bridge.md).

## Invariants

- Executors are policy-free: no `LoadedScope`/`EnforcementContext` access below this layer.
- Every spawned task path is expected to carry timeout wrappers per workspace convention.
- Unsupported feature-gated task kinds fail with an explicit `TaskResult::Error`, never silently no-op.
- The TUI must not host its own dispatch code — tab UIs call into this module (architecture guard enforced).
