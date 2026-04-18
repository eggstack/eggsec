---
name: performance_optimization
description: "Rust performance optimization patterns for Slapper security toolkit"
triggers:
  - performance
  - optimization
  - lazy
  - cache
  - concurrent
  - hashmap
  - mutex
  - lock-free
metadata:
  category: optimization
  tools: []
  scope: codebase
---

## Overview

Performance optimization is critical for security tools that process large volumes of network data. This skill covers common optimization patterns used in Slapper's hot paths.

## Performance Patterns

### 1. DashMap for Lock-Free Concurrent Aggregation

**Problem**: `Arc<Mutex<Vec<T>>>` causes contention when many tasks append results concurrently.

**Solution**: Use `Arc<DashMap<K, T>>` for lock-free append:
```rust
use dashmap::DashMap;
let results: Arc<DashMap<u16, PortResult>> = Arc::new(DashMap::new());
// In spawned task:
results.insert(port, result);  // No lock needed
// Collect at end:
Arc::try_unwrap(results).map(|dm| dm.into_iter().map(|(_, v)| v).collect()).unwrap_or_default()
```

**Files**: `scanner/ports/mod.rs`, `scanner/endpoints.rs`, `scanner/fingerprint.rs`, `fuzzer/engine/execution.rs`

### 2. AtomicU64 for Simple Counters

**Problem**: `Arc<Mutex<u64>>` for counters adds unnecessary synchronization overhead.

**Solution**: Use `Arc<AtomicU64>` for lock-free counter operations:
```rust
use std::sync::atomic::{AtomicU64, Ordering};
let counter = Arc::new(AtomicU64::new(0));
// Increment:
counter.fetch_add(1, Ordering::Relaxed);
```

**Files**: `scanner/ports/mod.rs` (scanned_count)

### 3. Broadcast Channel for Event Notification

**Problem**: Polling loops (e.g., `sleep(50ms)`) waste CPU cycles.

**Solution**: Use `tokio::sync::broadcast` channel for event notification:
```rust
use tokio::sync::broadcast;
let (tx, mut rx) = broadcast::channel(100);
// In worker task when event occurs:
let _ = tx.send(port_number);
// In listener:
while let Some(port) = rx.recv().await {
    // Process event
}
```

**Files**: `scanner/ports/spoofed.rs`

### 4. Pre-Allocation with String::with_capacity

**Problem**: String concatenation without pre-allocation causes multiple heap reallocations.

**Solution**: Pre-allocate based on expected output size:
```rust
// For URL encoding: output can be up to 3x input size
let mut output = String::with_capacity(input.len() * 3);
// For escape functions: estimate based on input
let mut buf = String::with_capacity(s.len() + 10);
write!(buf, "\"{}\"", s.replace('"', "\\\"")).unwrap();
```

**Files**: `utils/urlencoding.rs`, `waf/bypass/evasion.rs`, `output/escape.rs`

### 2. FxHashMap for Hot Paths

**Problem**: `std::collections::HashMap` is slower than necessary for high-traffic lookups.

**Solution**: Use `rustc_hash::FxHashMap` (2-3x faster):
```rust
use rustc_hash::FxHashMap;
let cache: FxHashMap<String, Value> = FxHashMap::default();
```

**Files**: `fuzzer/state.rs` (session/cookie storage), `recon/techdetect.rs` (headers)

### 3. LazyLock for Static Data

**Problem**: Creating data on every function call wastes CPU and memory.

**Solution**: Pre-compute static data with `std::sync::LazyLock`:
```rust
use std::sync::LazyLock;
static WAF_SIGNATURES: LazyLock<HashMap<String, WafSignature>> = LazyLock::new(|| {
    // ... populate once at startup
});
pub fn get_waf_signatures() -> HashMap<String, WafSignature> {
    WAF_SIGNATURES.clone()
}
```

**Files**: `waf/waf_patterns.rs`, `recon/js.rs`, `recon/email.rs`, `fuzzer/payloads/mod.rs`

### 4. Pre-Compiled Regex at Module Level

**Problem**: `Regex::new()` inside functions causes repeated compilation overhead.

**Solution**: Pre-compile with LazyLock:
```rust
static ENDPOINTS_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"...").unwrap()
});
// Use: ENDPOINTS_REGEX.find_iter(content)
```

**Files**: `recon/js.rs` (4 static patterns), `recon/email.rs` (4 static patterns)

### 5. Single-Buffer String Escape Functions

**Problem**: Chained `.replace()` creates N intermediate String allocations.

**Solution**: Use `write!` with single buffer:
```rust
pub fn escape_html(s: &str) -> String {
    let mut buf = String::with_capacity(s.len() * 6);
    for c in s.chars() {
        match c {
            '&' => buf.push_str("&amp;"),
            '<' => buf.push_str("&lt;"),
            // ...
            _ => buf.push(c),
        }
    }
    buf
}
```

**Files**: `output/escape.rs`

### 6. HTTP Connection Pooling

**Problem**: Creating new HTTP clients for each request adds latency.

**Solution**: Configure connection pooling:
```rust
Client::builder()
    .pool_max_idle_per_host(20)
    .pool_idle_timeout(Duration::from_secs(30))
    .tcp_nodelay(true)
    .build()
```

**Files**: `utils/http.rs`, `agent/alerts.rs`, `tool/implementations/search.rs`

### 7. SmallVec for Fixed-Size Buffers

**Problem**: `Vec<u8>` heap-allocates even for small fixed buffers.

**Solution**: Use `SmallVec<[u8; N]>` for stack allocation:
```rust
use smallvec::SmallVec;
let mut buf = SmallVec::<[u8; 256]>::new();
buf.resize(256, 0);
```

**Files**: `scanner/fingerprint.rs` (banner parsing)

### 8. contains_ignore_case Helper

**Problem**: Calling `path.to_lowercase()` inside loops allocates repeatedly.

**Solution**: Call `to_lowercase()` once before loop:
```rust
fn contains_ignore_case(haystack: &str, needle: &str) -> bool {
    haystack.to_lowercase().contains(&needle.to_lowercase())
}
```

**Files**: `utils/parsing.rs`, `waf/detector/types.rs`, `scanner/fingerprint.rs`

### 9. Report Generation with writeln!

**Problem**: String concatenation with `push_str(&format!())` is inefficient.

**Solution**: Use `writeln!` macro:
```rust
use std::fmt::Write;
let mut output = String::new();
writeln!(output, "# Heading").unwrap();
writeln!(output, "Content: {}", value).unwrap();
```

**Files**: `output/markdown.rs`, `output/html.rs`, `output/csv.rs`

### 10. Watch Channel for Progress Updates

**Problem**: TUI progress polling every 200ms with mutex instead of efficient signaling.

**Solution**: Use `tokio::sync::watch` channel for progress updates:
```rust
use tokio::sync::watch;

let (tx, rx) = watch::channel::<String>("initial".to_string());

// In worker (send progress):
tx.send("Processing step 1".to_string())?;

// In UI (receive progress):
while rx.changed().await.is_ok() {
    println!("Progress: {}", *rx.borrow());
}
```

**Files**: `tui/workers/recon.rs`, `recon/runner.rs`, `recon/spinner.rs`

### 11. HTTP Client Connection Pooling

**Problem**: Creating new HTTP clients for each request adds latency.

**Solution**: Configure connection pooling with proper settings:
```rust
Client::builder()
    .pool_max_idle_per_host(20)
    .pool_idle_timeout(Duration::from_secs(30))
    .tcp_nodelay(true)
    .build()
```

**Files**: `utils/http.rs`, `ai/client.rs`, `tool/implementations/search.rs`

## Dependency Additions

When adding new optimization dependencies:
```toml
# Workspace Cargo.toml
rustc-hash = "2"

# slapper/Cargo.toml
rustc-hash = { workspace = true }
smallvec = "1"
```

## Common Pitfalls

1. **Borrow checker with self reference**: When a method takes `&mut self`, you cannot simultaneously borrow `&self.field`. The `String::clone()` is cheap (pointer+len+cap copy).

2. **RegexBuilder size_limit**: Always use `.size_limit(100_000)` when building regexes from untrusted input to prevent ReDoS.

3. **LazyLock in tests**: Tests may fail if LazyLock panics during initialization. Use `.unwrap()` or handle gracefully.

## Verification Commands

```bash
cargo test --lib -p slapper
cargo clippy --lib -p slapper
cargo check --lib -p slapper
```