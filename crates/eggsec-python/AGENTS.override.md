# eggsec-python AGENTS Override

Python bindings via PyO3/maturin. Host-language binding over the Rust engine.

## Key Files

| File | Purpose |
|------|---------|
| `src/lib.rs` | PyModule definition, class/function registration |
| `src/engine.rs` | Sync `Engine` class — primary entry point |
| `src/async_engine.rs` | Async `AsyncEngine` class (tokio-backed) |
| `src/client.rs` | Sync `Client` class — wraps Engine internally |
| `src/async_client.rs` | Async `AsyncClient` class — wraps AsyncEngine |
| `src/scope.rs` | `Scope` class — target authorization |
| `src/status.rs` | `ExecutionStatus`, `ExecutionStats`, `Artifact`, `OperationResult` |
| `src/requests.rs` | `OperationRequest` + 10 typed request DTOs, `RequestBuilder` |
| `src/handles.rs` | `ExecutionHandle`, `ExecutionEvent`, `EventLog` |
| `src/cancellation.rs` | `CancellationToken` — atomic cancellation |
| `src/pipeline.rs` | `Pipeline`, `AsyncPipeline`, `PipelineStep`, `StepResult`, `PipelineResult` |
| `src/planning.rs` | `PlanStep`, `ScanPlan` — heuristic scan plan generation |
| `src/checkpoint.rs` | `Checkpoint`, `CheckpointStore` — pipeline resumption |
| `src/error.rs` | Python exception hierarchy |
| `src/runtime_sync.rs` | Sync blocking wrapper |
| `src/runtime_async.rs` | Async runtime (`PyFuture`) |
| `python/eggsec/__init__.py` | Public API re-exports |
| `python/eggsec/__init__.pyi` | Type stubs |
| `pyproject.toml` | maturin build configuration |

## Engine/Operation Model

The canonical entry point is `Engine` (sync) or `AsyncEngine` (async). `Client`/`AsyncClient` wrap these internally for backward compatibility.

```python
from eggsec import Engine, Scope, PortScanRequest, PortRange

scope = Scope.allow_hosts(["example.com"])
engine = Engine(scope)

# Typed request
req = PortScanRequest("example.com", port=PortRange.top_1000(), timing="normal")
result = engine.run_port_scan(req)

# Dispatch by name
result = engine.run("port_scan", target="example.com")

# Plan and execute
plan = engine.plan("example.com")
for step in plan.steps:
    result = engine.run(step.operation, target=step.target)

# Pipeline
from eggsec import Pipeline
pipe = Pipeline(engine)
pipe.add_step("recon_dns", "example.com")
pipe.add_step("fingerprint", "example.com")
pipe_result = pipe.run()

# Async
from eggsec import AsyncEngine
async with AsyncEngine(scope) as aengine:
    result = await aengine.run_port_scan(req)
```

## Build & Test

```bash
cd crates/eggsec-python
maturin develop                           # dev build into active venv
maturin develop --features <feature>      # with specific features
maturin build --release                   # release wheel
cargo test -p eggsec-python               # Rust-side tests
pytest crates/eggsec-python/tests/        # Python-side tests
```

## Conventions

- Register new classes in `src/lib.rs` via `m.add_class::<T>()`.
- Register new functions via `m.add_function(wrap_pyfunction!(...)?)`.
- Re-export all public API in `python/eggsec/__init__.py`.
- Add type stubs in `python/eggsec/*.pyi` for every public class/function.
- Add tests in `tests/` for Python-side validation.
- GIL is released during network I/O; use `py.allow_threads()` for blocking calls.
- Feature-gated engine modules require explicit `--features` at build time.
- All result types follow `#[pyclass(frozen)]` with `from_engine()`, `to_dict()`, `to_json()`, `__repr__`, `__str__`.
- Engine methods return `OperationResult` (common protocol). `Client`/`AsyncClient` unwrap to domain-specific types for backward compat.
- `Client` wraps `Engine` internally; `AsyncClient` wraps `AsyncEngine`. All scope enforcement delegated to engine helpers.
- New code should use `Engine`/`AsyncEngine` + typed request DTOs. `Client`/`AsyncClient` retained for backward compatibility.
