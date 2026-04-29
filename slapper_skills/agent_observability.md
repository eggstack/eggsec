# Agent Observability & Hot-Reload Skills

## Overview

These skills cover the agent observability system and configuration hot-reloading capabilities implemented in Wave 5.

## Skills

### 1. Agent Observability (`agent/logging.rs`)

**Purpose**: Non-blocking, rotating JSON logs for security compliance and debugging.

**Key Features**:
- Uses `tracing-appender` for non-blocking writes
- Daily rotating logs at `~/.config/slapper/logs/agent.log`
- Thread-safe with worker guard pattern
- Rich formatting with target, thread IDs, file/line numbers

**Usage**:
```rust
use crate::agent::logging::AgentLogger;

let logger = AgentLogger::new();
logger.init("target.com");
tracing::info!("Agent started scanning");
```

**When to use**:
- TUI swallows stdout, need file-based audit trail
- Security compliance requires persistent logging
- Debugging agent decision-making

### 2. Configuration Hot-Reloading (`agent/config_watcher.rs`)

**Purpose**: Watch `slapper.toml` and `portfolio.json` for changes without restart.

**Key Features**:
- Uses `notify` crate with debounced events (1 second debounce)
- `ConfigReloader` trait for custom reload callbacks
- `SlapperConfigReloader` for watching config files

**Usage**:
```rust
use crate::agent::config_watcher::{ConfigWatcher, SlapperConfigReloader};

let watcher = ConfigWatcher::new();
watcher.watch_config("slapper.toml", Box::new(SlapperConfigReloader::new()));
watcher.start().await;
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
- `notify-debouncer-mini` - debounced file events

## Related Files

- `crates/slapper/src/agent/logging.rs` - AgentLogger implementation
- `crates/slapper/src/agent/config_watcher.rs` - ConfigWatcher implementation
- `crates/slapper/src/fuzzer/engine/chained.rs` - StatefulFuzzer implementation
- `crates/slapper/src/agent/mod.rs` - Module exports

## Verification

```bash
cargo check --lib -p slapper
cargo test --lib -p slapper -- agent
```

---

*Created: 2026-04-29*
*Wave: 5 - Feature Enhancements*