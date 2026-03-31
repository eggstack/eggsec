# Deferred Items

All previously deferred items have been completed (2026-03-31).

## 1. Ruby Plugin Thread Safety ✅ COMPLETED

Replaced `unsafe impl Send + Sync` on `RubyBridge` with message-passing wrapper.
`RubyPluginClient` owns a dedicated Ruby VM thread and communicates via `std::sync::mpsc`.
`RubyPluginAdapter` now uses `Arc<RubyPluginClient>` — naturally `Send + Sync` without unsafe code.

## 2. TUI Plugin Integration ✅ COMPLETED

Extended the `#[cfg(feature = "python-plugins")]` gate to `#[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]` across all TUI files:
- `tui/tabs/mod.rs` — module declaration and re-export
- `tui/app.rs` — `App` struct field and constructor
- `tui/ui.rs` — rendering, breadcrumbs, status bar
- `tui/components/popup.rs` — help text

## 3. `Arc<Mutex>` Usage Review ✅ COMPLETED

Audited all 16 `Arc<Mutex<T>>` instances. All are justified:
- **Concurrent task results** (Vec shared across spawned tasks): `scanner/endpoints.rs`, `scanner/fingerprint.rs`, `fuzzer/engine/execution.rs`
- **Shared mutable state** (HashMap, PipelineContext): `pipeline/executor.rs`, `utils/rate_limiter.rs`, `tool/protocol/mcp.rs`
- **Timing/state shared across async boundaries**: `fuzzer/engine/core.rs`, `fuzzer/rate_limit.rs`
- **TUI shared state**: `tui/state/mod.rs`
- **Thread coordination** (spinner stage text): `recon/mod.rs`
- **Fixed**: `slapper-ruby/src/loader.rs` — replaced `Arc<Mutex<RubyBridge>>` with message-passing `Arc<RubyPluginClient>`

## 4. PyO3/Python 3.14 Forward Compatibility ✅ COMPLETED

Upgraded PyO3 from 0.24 to 0.25 in `crates/slapper-plugin/Cargo.toml`.
PyO3 0.25 adds Python 3.14 support (tested against 3.14.0b1 and 3.14 final release).
No API changes needed — existing `Python::with_gil` calls are compatible with 0.25.
