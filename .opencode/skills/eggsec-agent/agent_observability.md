---
name: agent-observability
description: "Agent observability, logging, and configuration hot-reload patterns - use when working with agent logging, monitoring, config watching, or stateful chained fuzzing."
---

# Agent Observability & Hot-Reload Skills

## Overview

These skills cover the agent observability system and configuration hot-reloading capabilities.

## Skills

### 1. Agent Observability (`crates/eggsec-cli/src/logging.rs`)

**Purpose**: Non-blocking, rotating JSON logs for security compliance and debugging.

**Key Features**:
- Uses `tracing-appender` for non-blocking writes
- Daily rotating logs at `memory_dir/logs/agent.log` (when agent subcommand is active)
- Composed alongside the console layer on console-enabled surfaces; file-only JSON when the TUI disables console output (`ConsoleLogging::Disabled`)
- Thread-safe with worker guard pattern
- Rich formatting with target, thread IDs, file/line numbers

**Usage**:
```rust
// init_logging_with_console() is called once in main.rs with an optional
// log_dir plus an explicit ConsoleLogging policy (Disabled for rich TUI
// launches so Ratatui owns the terminal; Enabled elsewhere). init_logging()
// remains as a compat wrapper (Enabled).
// When the agent subcommand is used, the log directory is derived from
// the agent's memory_dir and passed to enable file-based logging:

let log_dir = agent_log_dir(&cli);
let _guard = init_logging_with_console(
    if cli.json { LogFormat::Json } else { LogFormat::Pretty },
    log_dir,
    console_policy_for_launch(is_rich_tui_launch),
);
```

**When to use**:
- Rich TUI mode installs no console logger, so file-based audit trail is the durable record
- Security compliance requires persistent logging
- Debugging agent decision-making

### 2. Configuration Hot-Reloading (`agent/config_watcher.rs`)

**Purpose**: Watch `eggsec.toml` and `portfolio.json` for changes without restart.

**Key Features**:
- Uses `notify` crate with debounced events (1 second debounce)
- Uses `notify-debouncer-mini` v0.5+ callback-based API (NOT channel-based)
- `ConfigReloader` trait for custom reload callbacks
- `EggsecConfigReloader` for watching config files
- Gracefully handles missing files

**Important API Note**:
`notify-debouncer-mini` 0.5+ uses callback-based API:
```rust
use notify_debouncer_mini::{new_debouncer, DebounceEventResult, Debouncer};

let watcher = new_debouncer(Duration::from_secs(1), move |res: DebounceEventResult| {
    if let Err(e) = tx.blocking_send(res) {
        tracing::error!("Failed to send debounced event: {}", e);
    }
})?;
let mut watcher = watcher;
// Access underlying watcher via:
watcher.watcher().watch(path, RecursiveMode::NonRecursive)?;
```

**When to use**:
- Long-running agent processes
- Adding targets without restarting
- Changing agent intensity on-the-fly

### 3. Stateful/Chained Fuzzing (`fuzzer/engine/chained.rs`)

**Purpose**: Multi-step business logic fuzzing (e.g., Create → Extract ID → Unauthorized Access).

**Key Components**:
- `StatefulFuzzer` - orchestrates chained fuzz operations
- `ChainedFuzzInput` / `ChainedFuzzOutput` - chain definition and results
- `FuzzChainStep` - individual step with `FuzzArgs` and extraction rules
- Variable extraction/injection between steps

**Usage**:
```rust
use crate::fuzzer::engine::chained::{StatefulFuzzer, ChainedFuzzInput, FuzzChainStep};

let chain = ChainedFuzzInput {
    steps: vec![
        FuzzChainStep {
            name: "create".to_string(),
            args: FuzzArgs::default(),
            extract_from_response: Some("id".to_string()),
        },
        FuzzChainStep {
            name: "access".to_string(),
            args: FuzzArgs::with_variable("resource_id", "{{id}}"),
            extract_from_response: None,
        },
    ],
};

let fuzzer = StatefulFuzzer::new(client);
let results = fuzzer.run_chain(chain).await?;
```

**When to use**:
- Multi-step business logic (login → extract session → access)
- State-dependent endpoints
- OAuth flows, multi-stage APIs

## Dependencies Added

- `tracing-appender` - non-blocking file logging
- `notify` - file system watching
- `notify-debouncer-mini` - debounced file events (v0.5+)

## Related Files

- `crates/eggsec-cli/src/logging.rs` - CLI-side logging initialization with composed layers
- `crates/eggsec/src/agent/config_watcher.rs` - ConfigWatcher implementation (behind `config-watch` feature)
- `crates/eggsec/src/fuzzer/engine/chained.rs` - StatefulFuzzer implementation
- `crates/eggsec/src/agent/mod.rs` - Module exports

## Verification

```bash
cargo test --lib -p eggsec --features rest-api,ai-integration
# Should show 1472 passing tests
```

---

*Created: 2026-04-29*
*Updated: 2026-04-30*