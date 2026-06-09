# Agent Observability & Hot-Reload Skills

## Overview

These skills cover the agent observability system and configuration hot-reloading capabilities.

## Skills

### 1. Agent Observability (`logging/init.rs`)

**Purpose**: Non-blocking, rotating JSON logs for security compliance and debugging.

**Key Features**:
- Uses `tracing-appender` for non-blocking writes
- Daily rotating logs at `memory_dir/logs/agent.log` (when agent subcommand is active)
- Composed alongside console layer — both file and stdout active simultaneously
- Thread-safe with worker guard pattern
- Rich formatting with target, thread IDs, file/line numbers

**Usage**:
```rust
// init_logging() is called once in main.rs with an optional log_dir
// When the agent subcommand is used, the log directory is derived from
// the agent's memory_dir and passed to enable file-based logging:

let log_dir = agent_log_dir(&cli);
let _guard = init_logging(
    if cli.json { LogFormat::Json } else { LogFormat::Pretty },
    log_dir,
);
```

**When to use**:
- TUI swallows stdout, need file-based audit trail
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

- `crates/eggsec/src/logging/init.rs` - Centralized logging initialization with composed layers
- `crates/eggsec/src/agent/config_watcher.rs` - ConfigWatcher implementation
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