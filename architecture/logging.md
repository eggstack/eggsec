# Logging Module

## Overview

Eggsec's logging has two layers:

1. **Engine-internal** (`crates/eggsec/src/logging/`) — conditionally compiled re-export behind the `logging-subscriber` feature flag. Provides `LogFormat` (3 variants) and `init_logging()` for process-host crates that link the engine with this feature enabled.
2. **Process-host** (`crates/eggsec-cli/src/logging.rs`) — the CLI crate owns the subscriber/appender dependencies unconditionally and provides an identical `init_logging()` implementation.

Both implementations are functionally identical (same code, different compilation units). The CLI's version is always available; the engine's version requires the `logging-subscriber` feature.

The engine core code (`crates/eggsec/src/`) uses the `tracing` facade only — it never configures subscribers or appenders. Subscriber/appender configuration is owned by the frontend that starts the process.

Every number in this document was verified against source on 2026-08-25.

## Files

| File | Feature Gate | Purpose |
|------|-------------|---------|
| `crates/eggsec/src/logging/mod.rs` | `logging-subscriber` | Re-exports `LogFormat` and `init_logging` from `init.rs` |
| `crates/eggsec/src/logging/init.rs` | `logging-subscriber` | Subscriber/appender setup (engine copy) |
| `crates/eggsec-cli/src/logging.rs` | Always (CLI binary) | Subscriber/appender setup (CLI copy) |

**Note**: The daemon (`eggsec-daemon`) does not have its own logging module. It inherits subscriber configuration from the process host that embeds it.

## Feature Gate: `logging-subscriber`

Declared in `crates/eggsec/Cargo.toml:318`:
```toml
logging-subscriber = ["dep:tracing-subscriber", "dep:tracing-appender"]
```

This pulls in `tracing-subscriber` (with `env-filter` + `json` features) and `tracing-appender`. Python and headless consumers do not link these crates by default.

## Key Types

### `LogFormat` (`init.rs:10-16`)

```rust
#[derive(Debug, Clone, Copy, Default)]
pub enum LogFormat {
    #[default]
    Pretty,
    Json,
    Compact,
}
```

| Variant | Console Layer | Thread IDs | Line Numbers | Span Events |
|---------|--------------|------------|--------------|-------------|
| `Pretty` (default) | `.pretty()` | No | Yes | None |
| `Json` | `.json()` | Yes | Yes (via thread names) | `FmtSpan::CLOSE` |
| `Compact` | `.compact()` | No | Yes | None |

## Behavior/API

### `init_logging(format, log_dir) -> Option<WorkerGuard>` (`init.rs:18-121`)

**Signature**:
```rust
pub fn init_logging(
    format: LogFormat,
    log_dir: Option<PathBuf>,
) -> Option<tracing_appender::non_blocking::WorkerGuard>
```

**Behavior**:
1. Reads `RUST_LOG` env var via `EnvFilter::try_from_default_env()`; defaults to `"info"` if unset
2. Creates a `tracing_subscriber::registry()` with the env filter
3. **Without `log_dir`** (console-only): Registers a single console layer matching the `format` variant
4. **With `log_dir`** (console + file):
   - Creates the log directory (`create_dir_all`)
   - Sets up a daily rolling file appender (`agent.log`, `Rotation::DAILY`)
   - Creates a non-blocking writer with `tracing_appender::non_blocking()`
   - Registers a **JSON file layer** (always JSON, regardless of console format) with: no ANSI, target enabled, thread IDs, file + line numbers
   - Registers the console layer matching the `format` variant
   - Returns the `WorkerGuard` (must be held for process lifetime)
5. Logs error to stderr if subscriber initialization fails (non-fatal)

**File layer** always uses JSON format with: `with_ansi(false)`, `with_target(true)`, `with_thread_ids(true)`, `with_file(true)`, `with_line_number(true)`.

### `sanitize_for_logging()` (`utils/logging.rs:59-61`)

**Not part of the subscriber** — lives in `utils/logging.rs`, re-exported from `utils`:
```rust
pub fn sanitize_for_logging(input: &str) -> String {
    sanitize_bytes(input, 500)
}
```

Strips ANSI CSI escape sequences (`\x1B[...`), control chars (preserving tabs), truncates to 500 chars. Used across scanner, fuzzer, pipeline, recon, stress, and waf modules.

## Integration Points

| Consumer | How Used |
|----------|----------|
| `eggsec-cli/src/main.rs` | Calls `init_logging(format, log_dir)` once at startup; format driven by `--json` flag |
| `eggsec-cli/src/commands/agent.rs` | Passes agent's `memory_dir` as `log_dir` for file-based logging |
| Engine modules (scanner, fuzzer, etc.) | Use `tracing::{info!, warn!, error!}` facade only — never configure subscribers |
| `utils/logging.rs` | `sanitize_for_logging()` used before logging user-controlled strings |

**Dependency boundary**: `tracing-subscriber` and `tracing-appender` are optional engine dependencies behind `logging-subscriber`. The CLI crate owns these dependencies unconditionally since it is the process host. See `Cargo.toml:64-65`:
```toml
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"], optional = true }
tracing-appender = { version = "0.2", optional = true }
```

## Usage

Called once during application startup:
```rust
// In CLI main.rs:
let _guard = init_logging(LogFormat::Pretty, None);  // console only
// or
let _guard = init_logging(LogFormat::Json, Some(log_dir));  // console + file
```

The `WorkerGuard` **must** be held for the process lifetime when `log_dir` is `Some`. Dropping it flushes and shuts down the non-blocking writer.

## Invariants & Gotchas

1. **Two identical copies**: `crates/eggsec/src/logging/init.rs` and `crates/eggsec-cli/src/logging.rs` contain the same implementation. The engine copy is behind `logging-subscriber`; the CLI copy is unconditional. They are maintained independently — changes to one must be replicated to the other.
2. **File layer always JSON**: Even when console format is `Pretty` or `Compact`, the file layer outputs JSON. This ensures machine-parseable log files regardless of console preference.
3. **`WorkerGuard` lifetime**: If the guard is dropped before process exit, log messages may be lost (non-blocking writer flush is tied to guard drop).
4. **`EnvFilter` default is `info`**: Only changed by setting `RUST_LOG` env var. No programmatic override is provided.
5. **Daemon has no logging module**: `eggsec-daemon` does not own logging configuration. The process host that embeds the daemon is responsible for subscriber setup.

## Related

- [utils.md](utils.md) — `utils/logging.rs` provides `sanitize_for_logging()` for stripping ANSI escapes and control characters from log output
- [config.md](config.md) — Configuration system may set log-related options

*Last verified against source: 2026-08-25*
