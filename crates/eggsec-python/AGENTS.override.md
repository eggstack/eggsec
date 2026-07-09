# eggsec-python AGENTS Override

Python bindings via PyO3/maturin. Host-language binding over the Rust engine.

## Key Files

| File | Purpose |
|------|---------|
| `src/lib.rs` | PyModule definition, class/function registration |
| `src/client.rs` | Sync `Client` class |
| `src/async_client.rs` | Async `AsyncClient` class (tokio-backed) |
| `src/scope.rs` | `Scope` class — target authorization |
| `src/error.rs` | Python exception hierarchy |
| `src/runtime_sync.rs` | Sync blocking wrapper |
| `src/runtime_async.rs` | Async runtime (`PyFuture`) |
| `python/eggsec/__init__.py` | Public API re-exports |
| `python/eggsec/__init__.pyi` | Type stubs |
| `pyproject.toml` | maturin build configuration |

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
