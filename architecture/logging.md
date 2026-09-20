# Logging Module

## Overview

Eggsec's logging has two layers:

1. **Engine-internal** (`crates/eggsec/src/logging/`) — conditionally compiled re-export behind the `logging-subscriber` feature flag. Provides `LogFormat` (3 variants), `ConsoleLogging` (console emission policy), and `init_logging()` / `init_logging_with_console()` for process-host crates that link the engine with this feature enabled.
2. **Process-host** (`crates/eggsec-cli/src/logging.rs`) — the CLI crate owns the subscriber/appender dependencies unconditionally and provides an identical `init_logging()` / `init_logging_with_console()` implementation.

Both implementations are functionally identical (same code, different compilation units). The CLI's version is always available; the engine's version requires the `logging-subscriber` feature.

The engine core code (`crates/eggsec/src/`) uses the `tracing` facade only — it never configures subscribers or appenders. Subscriber/appender configuration is owned by the frontend that starts the process.

The process host chooses the logging destination by execution surface: rich TUI mode disables console formatting so Ratatui/Crossterm is the only writer to the controlling terminal, while CLI/CI/daemon-console surfaces keep console output. `tracing` remains the required diagnostic facade in all surfaces.

Every number in this document was verified against source on 2026-08-25; console-policy behavior verified against source on 2026-09-20 (Phase A).

## Files

| File | Feature Gate | Purpose |
|------|-------------|---------|
| `crates/eggsec/src/logging/mod.rs` | `logging-subscriber` | Re-exports `LogFormat`, `ConsoleLogging`, `init_logging`, `init_logging_with_console`, policy helpers from `init.rs` |
| `crates/eggsec/src/logging/init.rs` | `logging-subscriber` | Subscriber/appender setup + console emission policy (engine copy) |
| `crates/eggsec-cli/src/logging.rs` | Always (CLI binary) | Subscriber/appender setup + console emission policy (CLI copy) |
| `crates/eggsec-cli/src/main.rs` | `tui` (TUI branch) | Resolves rich-TUI launch intent before subscriber construction (`rich_tui_launch_requested`, `console_policy_for_launch`) |

**Note**: The daemon (`eggsec-daemon`) does not have its own logging module. It inherits subscriber configuration from the process host that embeds it.

## Feature Gate: `logging-subscriber`

Declared in `crates/eggsec/Cargo.toml:318`:
```toml
logging-subscriber = ["dep:tracing-subscriber", "dep:tracing-appender"]
```

This pulls in `tracing-subscriber` (with `env-filter` + `json` features) and `tracing-appender`. Python and headless consumers do not link these crates by default.

## Key Types

### `LogFormat`

```rust
#[derive(Debug, Clone, Copy, Default)]
pub enum LogFormat {
    #[default]
    Pretty,
    Json,
    Compact,
}
```

| Variant | Console Layer (when enabled) | Thread IDs | Line Numbers | Span Events |
|---------|------------------------------|------------|--------------|-------------|
| `Pretty` (default) | `.pretty()` | No | Yes | None |
| `Json` | `.json()` | Yes | Yes (via thread names) | `FmtSpan::CLOSE` |
| `Compact` | `.compact()` | No | Yes | None |

### `ConsoleLogging` — console emission policy

```rust
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ConsoleLogging {
    #[default]
    Enabled,
    Disabled,
}
```

| Policy | Meaning |
|--------|---------|
| `Enabled` | Non-TUI surfaces (CLI, CI, daemon console): console formatter is constructed. |
| `Disabled` | Rich TUI mode: no stdout/stderr formatter is constructed. Tracing call sites remain valid but emit no terminal bytes. |

Pure helpers (tested without touching the global subscriber):

- `resolve_console_logging(is_rich_tui_launch: bool) -> ConsoleLogging` — TUI launch maps to `Disabled`, all else to `Enabled`.
- `console_layer_enabled(console: ConsoleLogging) -> bool` — the single predicate production consults before constructing any console `fmt::layer()`.

## Behavior/API

### `init_logging(format, log_dir) -> Option<WorkerGuard>`

Compatibility wrapper that delegates with `ConsoleLogging::Enabled`. Existing callers remain source- and behavior-compatible.

### `init_logging_with_console(format, log_dir, console) -> Option<WorkerGuard>`

**Signature**:
```rust
pub fn init_logging_with_console(
    format: LogFormat,
    log_dir: Option<PathBuf>,
    console: ConsoleLogging,
) -> Option<tracing_appender::non_blocking::WorkerGuard>
```

**Behavior**:
1. Reads `RUST_LOG` env var via `EnvFilter::try_from_default_env()`; defaults to `"info"` if unset (`RUST_LOG` / EnvFilter semantics unchanged)
2. Creates a `tracing_subscriber::registry()` with the env filter
3. **Console `Enabled`, without `log_dir`**: Registers a single console layer matching the `format` variant (historical behavior preserved)
4. **Console `Enabled`, with `log_dir`**: File + console (historical behavior preserved):
   - Creates the log directory (`create_dir_all`)
   - Sets up a daily rolling file appender (`agent.log`, `Rotation::DAILY`)
   - Creates a non-blocking writer with `tracing_appender::non_blocking()`
   - Registers a **JSON file layer** (always JSON, regardless of console format) with: no ANSI, target enabled, thread IDs, file + line numbers
   - Registers the console layer matching the `format` variant
   - Returns the `WorkerGuard` (must be held for process lifetime)
5. **Console `Disabled`, with `log_dir`**: Registers only the JSON file layer; no console formatter is constructed. No new default persistent TUI log directory is introduced — `log_dir` still comes only from an existing caller (agent memory dir).
6. **Console `Disabled`, without `log_dir`**: Installs the filtered registry with no formatting writer, so tracing call sites remain valid but emit no terminal bytes. Both stdout *and* stderr are forbidden as side channels while the alternate screen is owned; stderr is never used as an alternate TUI writer.
7. Logs error to stderr if subscriber initialization fails (non-fatal). Initialization failures that occur before rich terminal acquisition may still report to stderr; no second subscriber, reload handle, global mutable writer switch, or TUI-specific logging crate is added.

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
| `eggsec-cli/src/main.rs` | Resolves rich-TUI launch intent (`rich_tui_launch_requested`) before logging, then calls `init_logging_with_console(format, log_dir, console_policy_for_launch(is_rich_tui_launch))` once at startup; format driven by `--json` flag. Early `--generate-config` / shell-completion paths return before subscriber construction. |
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
// In CLI main.rs (non-TUI path):
let _guard = init_logging(LogFormat::Pretty, None);  // console only (compat wrapper)
// TUI launch path:
let _guard = init_logging_with_console(
    LogFormat::Pretty,
    log_dir,
    ConsoleLogging::Disabled,
);  // file-only when log_dir is Some, silent otherwise
// or
let _guard = init_logging_with_console(
    LogFormat::Json,
    Some(log_dir),
    ConsoleLogging::Enabled,
);  // console + file
```

The `WorkerGuard` **must** be held for the process lifetime when `log_dir` is `Some`. Dropping it flushes and shuts down the non-blocking writer.

## Invariants & Gotchas

1. **Two identical copies**: `crates/eggsec/src/logging/init.rs` and `crates/eggsec-cli/src/logging.rs` contain the same implementation, including the `ConsoleLogging` policy. The engine copy is behind `logging-subscriber`; the CLI copy is unconditional. They are maintained independently — changes to one must be replicated to the other.
2. **File layer always JSON**: Even when console format is `Pretty` or `Compact`, the file layer outputs JSON. This ensures machine-parseable log files regardless of console preference.
3. **`WorkerGuard` lifetime**: If the guard is dropped before process exit, log messages may be lost (non-blocking writer flush is tied to guard drop).
4. **`EnvFilter` default is `info`**: Only changed by setting `RUST_LOG` env var. No programmatic override is provided.
5. **Daemon has no logging module**: `eggsec-daemon` does not own logging configuration. The process host that embeds the daemon is responsible for subscriber setup.
6. **Single-writer rule (Phase A)**: Rich TUI console logging is disabled. Stdout *and* stderr are both forbidden as side channels while the alternate screen is owned. `tracing` remains the required diagnostic facade; user-visible recoverable errors use TUI state (notification overlay, per-tab error, popup/status). Fatal errors are deferred until after restoration (Phase B owns teardown). No new default persistent TUI logging was introduced.

## Related

- [utils.md](utils.md) — `utils/logging.rs` provides `sanitize_for_logging()` for stripping ANSI escapes and control characters from log output
- [config.md](config.md) — Configuration system may set log-related options
- [tui.md](tui.md) — Single-terminal-writer ownership contract and in-frame error routing

*Last verified against source: 2026-08-25; console-policy section verified 2026-09-20*
