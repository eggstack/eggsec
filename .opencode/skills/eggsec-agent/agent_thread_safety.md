---
name: agent_thread_safety
description: "Thread safety patterns for the autonomous security agent modules"
triggers:
  - thread safety
  - AlertRouter
  - TargetPortfolio
  - LongitudinalMemory
  - Arc<Mutex
  - Arc<RwLock
  - Send + Sync
metadata:
  category: architecture
  tools: [agent]
  scope: internal
---

## Overview

The autonomous security agent modules require thread-safe interior mutability since they may be accessed from multiple async tasks concurrently. Three main patterns are used:

## AlertRouter Thread Safety

`agent/alerts.rs` uses `Arc<Mutex<>>` for all internal state:

```rust
pub struct AlertRouter {
    channels: Arc<Mutex<Vec<AlertChannel>>>,
    recent_alerts: Arc<Mutex<std::collections::HashMap<String, Instant>>>,
    dedup_window_secs: u64,
}
```

**Key Methods:**
- `new()` - Creates with `Arc::new(Mutex::new(...))` wrappers
- `add_channel(&self, channel: AlertChannel)` - Takes `&self` not `&mut self`
- `send(&self, alert: &Alert)` - Thread-safe async send with internal locking

**Pattern:**
```rust
pub fn add_channel(&self, channel: AlertChannel) {
    self.channels.lock().unwrap().push(channel);
}
```

## TargetPortfolio Thread Safety

`agent/portfolio.rs` uses `Arc<RwLock<>>` for read/write state:

```rust
pub struct TargetPortfolio {
    data: Arc<RwLock<PortfolioData>>,
    file_path: Option<PathBuf>,
}
```

**Key Methods:**
- `new()` - Creates with `Arc::new(RwLock::new(...))` wrapper
- `add_target(&self, id: String, config: TargetConfig)` - Takes `&self` not `&mut self`
- `get_target(&self, id: &str)` - Returns cloned data, not references
- `save(&self)` - Uses read lock for data access

**Pattern:**
```rust
pub fn get_target(&self, id: &str) -> Option<TargetConfig> {
    self.data.read().unwrap().targets.get(id).cloned()
}

pub fn add_target(&self, id: String, config: TargetConfig) {
    self.data.write().unwrap().targets.insert(id, config);
}
```

## LongitudinalMemory Thread Safety

`agent/memory.rs` takes `&self` (no interior mutability needed):

```rust
pub struct LongitudinalMemory {
    storage_dir: PathBuf,
}
```

**Key Methods:**
- Methods take `&self` for read operations
- `set_baseline(&self, target: &str, finding_ids: Vec<String>)` - No `&mut self` required

## Why These Patterns?

1. **`Arc<Mutex<T>>`** - When you need exclusive write access with shared ownership
2. **`Arc<RwLock<T>>`** - When reads are more frequent than writes (better concurrency)
3. **`&self` API** - Simpler API surface, no ` Mutex` needed when internal data doesn't change

## Important Notes

- All methods that modify state take `&self` (not `&mut self`)
- Use `.lock().unwrap()` for quick locking, or `.try_lock()` for non-blocking
- Use `RwLock::read()` for read-only operations (allows concurrent readers)
- Use `RwLock::write()` for write operations (excludes all readers)

## Verification

Run tests with:
```bash
cargo test --lib -p eggsec -- --test-threads=4
```

## Triggers

Keywords: thread safety, AlertRouter, TargetPortfolio, LongitudinalMemory, Arc<Mutex, Arc<RwLock, Send + Sync, interior mutability, concurrent access

## References

- `crates/eggsec/src/agent/alerts.rs` - AlertRouter implementation
- `crates/eggsec/src/agent/portfolio.rs` - TargetPortfolio implementation
- `crates/eggsec/src/agent/memory.rs` - LongitudinalMemory implementation