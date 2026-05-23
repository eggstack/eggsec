# Pipeline Module Override

## Key Files

| File | Purpose |
|------|---------|
| `crates/slapper/src/pipeline/mod.rs` | Module entry, CLI entry points (`run_cli`, `resume_cli`) |
| `crates/slapper/src/pipeline/stage.rs` | `Stage` enum, profiles, aliases, parsing |
| `crates/slapper/src/pipeline/executor.rs` | `Pipeline` struct, sequential execution, stage dispatch |
| `crates/slapper/src/pipeline/context.rs` | `PipelineContext` for inter-stage data sharing |
| `crates/slapper/src/pipeline/session.rs` | `PipelineSession` for pause/resume via JSON snapshots |
| `crates/slapper/src/pipeline/report.rs` | `PipelineReport`, HTML/CSV output |
| `crates/slapper/src/tool/implementations/pipeline.rs` | `PipelineTool` implementing `SecurityTool` |

## Performance: Use FxHashMap

For performance-critical code, use `rustc_hash::FxHashMap` instead of `std::collections::HashMap`:

```rust
use rustc_hash::FxHashMap;

let mut services: FxHashMap<u16, ServiceFingerprint> = FxHashMap::default();
```

## Recent Bug Fixes (2026-05-22)

| File | Issue | Fix |
|------|-------|-----|
| `context.rs:12` | `PipelineContext.services` used `HashMap` instead of `FxHashMap` | Changed to `FxHashMap` for performance |
| `mod.rs:240-248` | `resume_cli()` didn't return error on failed stages | Now returns `ScanFailed` error like `run_cli()` |
| `executor.rs:444-445` | `run_load_test()` ignored config, used default TLS settings | Changed to `LoadTestRunner::from_args_with_config()` |

## Additional Fixes (2026-05-27 through 2026-05-29)

| File | Issue | Fix |
|------|-------|-----|
| `mod.rs:77-238` | `run_cli()` and `run_cli_with_callback()` had duplicated output writing code | Extracted to `write_output()` helper function |
| `executor.rs:19-24` | `StageResult.duration_ms` was serialized to JSON (unnecessary, causes bloat) | Added `#[serde(skip)]` to `duration_ms` field |
| `executor.rs:19-30` | `StageResult` lacked constructor for cleaner object creation | Added `StageResult::new()` constructor |
| `executor.rs:157` | Progress bar created even for empty stage list | Changed condition to `self.tui_mode \|\| self.stages.is_empty()` to skip progress for empty runs |
| `stage.rs` | Hardcoded ports duplicated in executor.rs | Added `DEFAULT_SCAN_PORTS` and `EXTENDED_SCAN_PORTS` constants |
| `stage.rs` | Profile mapping duplicated between stage.rs and tool/implementations/pipeline.rs | Created `profile_from_str()` shared function |
| `executor.rs:575-577` | `get_extended_ports()` hardcoded string | Now uses `EXTENDED_SCAN_PORTS.to_string()` constant |

## Key Patterns

1. **Sequential execution** via simple `match` in `execute_stage()` - no trait abstraction
2. **Context sharing** via `Arc<Mutex<PipelineContext>>`
3. **Session persistence** only when output path is session-like (`*.session` or `*.session.json`)
4. **Checkpointing** happens after each stage in `Pipeline::run()`
5. **No verify_tls in FuzzArgs** - use `common.insecure` flag instead

## Testing

```bash
cargo test --lib -p slapper pipeline::
cargo check --lib -p slapper
cargo clippy --lib -p slapper
```