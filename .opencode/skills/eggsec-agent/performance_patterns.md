---
name: performance_patterns
description: Performance optimization patterns for Eggsec codebase
triggers:
  - performance
  - optimization
  - HashMap
  - Mutex
  - cache
metadata:
  category: code_quality
  tools: [all]
  scope: implementation
---

## Overview

This skill documents performance optimization patterns implemented across the Eggsec codebase. Future agents should follow these patterns for performance-sensitive code.

## Recent Updates (2026-04-25)

- Added: Arc::try_unwrap() for DashMap results collection (replaces costly Clone)
- Added: MCP hashmap reaper documentation

## Performance Patterns Implemented

### 1. FxHashMap for Hot Paths

**Pattern**: Replace `std::collections::HashMap` with `rustc_hash::FxHashMap` for 2-3x faster lookups.

**Before**:
```rust
use std::collections::HashMap;
```

**After**:
```rust
use rustc_hash::FxHashMap;
```

**Hot paths migrated**:
- `waf/detector/mod.rs` — signatures storage
- `waf/detector/detect.rs` — detection loop
- `scanner/templates/models.rs` — templates
- `fuzzer/chain.rs` — fuzz chains
- `proxy/intercept/rules.rs` — rules
- `proxy/intercept/cert.rs` — cert cache
- `waf/data/patterns.rs` — WAF signatures

### 2. parking_lot Mutex

**Pattern**: Replace `std::sync::Mutex` with `parking_lot::Mutex` for faster lock acquisition.

**Note**: `parking_lot::Mutex::lock()` returns `MutexGuard` directly (NOT `Result`). Remove `.unwrap()` calls on lock acquisitions.

**Before**:
```rust
use std::sync::Mutex;
let guard = mutex.lock().unwrap();
```

**After**:
```rust
use parking_lot::Mutex;
let guard = mutex.lock();  // No unwrap needed
```

**Files**: `scanner/ports/spoofed.rs`, `stress/metrics.rs`, `tui/workers/recon.rs`, `agent/portfolio.rs`

### 3. DashMap for Concurrent Access

**Pattern**: Replace `RwLock<HashMap>` with `DashMap` for sharded locking in high-concurrency scenarios.

**Before**:
```rust
use parking_lot::RwLock;
tokens: RwLock<HashMap<String, TokenBucket>>,
```

**After**:
```rust
use dashmap::DashMap;
tokens: DashMap<String, TokenBucket>,
```

**File**: `tool/ratelimit.rs`

### 4. Caching to_lowercase() Results

**Issue**: Multiple `to_lowercase()` calls on the same string within loops.

**Before**:
```rust
for item in items {
    if title.to_lowercase().contains("critical") || title.to_lowercase().contains("rce") {
        // ...
    }
}
```

**After**:
```rust
let title_lower = title.to_lowercase();
for item in items {
    if title_lower.contains("critical") || title_lower.contains("rce") {
        // ...
    }
}
```

**Files**: `vuln/triage.rs`, `ai/planner.rs`

### 5. Regex Caching

**Pattern**: Cache compiled regex patterns in a HashMap to avoid repeated compilation.

**Implementation** in `fuzzer/chain.rs`:
```rust
pub struct ChainExecutor {
    regex_cache: FxHashMap<String, regex::Regex>,
}

impl ChainExecutor {
    fn get_or_compile(&mut self, pattern: &str) -> Result<&regex::Regex> {
        if !self.regex_cache.contains_key(pattern) {
            let re = RegexBuilder::new(pattern)
                .size_limit(1_000_000)
                .build()?;
            self.regex_cache.insert(pattern.to_string(), re);
        }
        Ok(self.regex_cache.get(pattern).unwrap())
    }
}
```

### 6. Watch Channel for Progress Updates

**Issue**: Polling loop with `Arc<Mutex<String>>` and sleep.

**Before**:
```rust
let progress = Arc::new(Mutex::new(String::new()));
loop {
    {
        let p = progress.lock().unwrap().clone();
        println!("Progress: {}", p);
    }
    tokio::time::sleep(Duration::from_millis(200)).await;
}
```

**After**:
```rust
use tokio::sync::watch;
let (tx, mut rx) = watch::channel::<String>("initial".to_string());
loop {
    if rx.changed().await.is_ok() {
        println!("Progress: {}", *rx.borrow());
    }
}
```

**File**: `tui/workers/recon.rs`

### 7. String Allocation Optimization

**Issue**: Allocating String to parse into SocketAddr.

**Before**:
```rust
let addr: SocketAddr = format!("{}:{}", host, port).parse().ok()?;
```

**After**:
```rust
let ip: IpAddr = host.parse().ok()?;
let addr = SocketAddr::new(ip, port);
```

**Files**: `scanner/fingerprint.rs`, `scanner/udp_fingerprint.rs`

### 8. Atomic Operations

**Pattern**: Use `AtomicU64::fetch_add()` instead of `Mutex<usize>` for counters.

**Before**:
```rust
let count = *counter.lock().await;
if count < limit {
    *counter.lock().await += 1;
    true
} else {
    false
}
```

**After**:
```rust
let old = counter.fetch_add(1, Ordering::Relaxed);
old < limit
```

**Files**: `scanner/ports/mod.rs`, `scanner/fingerprint.rs`, `scanner/endpoints.rs`

### 9. Arc::try_unwrap() for DashMap Results

**Pattern**: When all workers have completed, use `Arc::try_unwrap()` instead of `DashMap::clone()` to avoid deep copy.

**Before** (slow - deep clone):
```rust
let results: Vec<PortResult> = DashMap::clone(&results).into_iter().map(|(_, v)| v).collect();
```

**After** (fast - ownership transfer):
```rust
let results: Vec<PortResult> = Arc::try_unwrap(results)
    .expect("all workers completed")
    .into_iter()
    .map(|(_, v)| v)
    .collect();
```

**Files** (2026-04-25):
- `scanner/ports/mod.rs:593`
- `scanner/fingerprint.rs:301`
- `scanner/endpoints.rs:810`
- `fuzzer/engine/execution.rs:146`

**Prerequisite**: Must ensure all worker tasks have completed (e.g., after `join_all(handles).await`).

## Verification Commands

After implementing performance optimizations:
```bash
cargo test --lib -p eggsec
cargo clippy --lib -p eggsec
```

## Related Skills

- `security_fix_patterns` - Security vulnerability patterns
- `code_quality_patterns` - General code quality patterns