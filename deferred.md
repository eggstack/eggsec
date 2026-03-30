# Deferred Items

Items that are known but not yet addressed, referenced by AGENTS.md.

## 1. Ruby Plugin Thread Safety

`RubyPluginAdapter` requires `Plugin` trait to be `Send + Sync`. `RubyBridge` may not be thread-safe. Consider message-passing wrapper or thread-local Ruby VM.

**Tracking:** Phase 7 of plan.md

## 2. TUI Plugin Integration

TUI `App` struct is missing a `plugin` field. Cannot be added until thread safety is resolved.

**Tracking:** Phase 7 of plan.md

## 3. `Arc<Mutex>` Usage Review

Review all `Arc<Mutex>` usage patterns for potential simplification or lock-free alternatives.

**Tracking:** TBD

## 4. PyO3/Python 3.14 Forward Compatibility

Review PyO3 version in `crates/slapper-plugin/Cargo.toml`. Update when Python 3.14 is released.

**Tracking:** Phase 7 of plan.md
