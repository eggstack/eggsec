---
name: config_management
description: "Configuration validation and management commands"
triggers:
  - config validate
  - config show
  - config
metadata:
  category: configuration
  tools: [config]
  scope: local
---

## Overview
Slapper provides configuration validation and display commands for managing the tool's settings. These commands help verify configuration files and inspect effective settings.

## CLI Commands

### Validate Configuration
Verify a configuration file is valid:
```bash
slapper config validate
slapper config validate --config /path/to/config.toml
```

### Show Effective Configuration
Display the current effective configuration:
```bash
slapper config show
```

## Agent Config Hot-Reloading

For autonomous agent mode, the `ConfigWatcher` provides file watching with hot-reload:

**Key Components** (`agent/config_watcher.rs`):
- `ConfigWatcher` - watches config files using `notify-debouncer-mini`
- `ConfigReloader` trait - custom reload logic via callback
- `SlapperConfigReloader` - reloader that handles portfolio + main config paths

**Wiring** (`agent/mod.rs:198-207`):
```rust
// ConfigWatcher is stored in Agent struct as field:
pub struct Agent {
    ...
    config_watcher: Option<ConfigWatcher>,
}

// In Agent::new(), watcher is created and stored:
let config_paths = std::iter::once(config.portfolio_path.clone())
    .flatten()
    .chain(crate::config::SlapperConfig::default_path())
    .collect::<Vec<_>>();
let reloader = Arc::new(SlapperConfigReloader::new(
    Some(portfolio.clone()),
    config.portfolio_path.clone(),
    crate::config::SlapperConfig::default_path(),
));
let config_watcher = Some(ConfigWatcher::new(config_paths, reloader)?);
```

**Important API Note**:
`notify-debouncer-mini` 0.5+ uses callback-based API:
```rust
use notify_debouncer_mini::{new_debouncer, DebounceEventResult};

let watcher = new_debouncer(Duration::from_secs(1), move |res: DebounceEventResult| {
    if let Err(e) = tx.blocking_send(res) {
        tracing::error!("Failed to send debounced event: {}", e);
    }
})?;
```

## Triggers
- `config validate` - Validate configuration file
- `config show` - Display effective configuration
- Configuration management