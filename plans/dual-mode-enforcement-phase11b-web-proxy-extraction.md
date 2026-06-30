# Phase 11b Plan: eggsec-web-proxy Domain Crate Extraction

## Goal

Extract the web proxy module into a standalone `eggsec-web-proxy` domain crate, following the `eggsec-db-lab` pattern. The main `eggsec` crate becomes a thin adapter for enforcement, CLI/TUI/MCP/REST integration, and re-exports.

## Scope

**Extract to domain crate:**
- Proxy pool management: config, pool, rotator, health, socks, http_connect
- Intercept/MITM: ProxyServer, CertGenerator, InterceptProxy, rules engine, protocol handlers (WebSocket/HTTP2/gRPC), correlation engine, narrative generation, plugin system, evidence bundles, redteam tests
- MCP tool schema types (marker feature)
- Transparent proxy (feature-gated)
- Dynamic plugins (feature-gated)

**Keep in main crate:**
- Adapter layer (`proxy/mod.rs`): re-exports + ProxyManager facade + enforcement wiring
- CLI handlers, TUI tabs, tool implementation, pipeline integration
- All policy/enforcement code

## Extraction Principles

1. Enforcement stays central in main crate.
2. Domain crate owns domain execution logic, types, and tests.
3. Domain crate does NOT parse global CLI flags or own `ExecutionSurface`/`ExecutionProfile`.
4. Feature flags remain explicit and narrow.
5. Existing CLI/TUI/MCP/REST behavior preserved (TUI paths unchanged via re-exports).

## File Inventory

### Files to Move (23 Rust files, ~15,311 lines)

| Source | Destination | Lines |
|--------|-------------|-------|
| `proxy/config.rs` | `eggsec-web-proxy/src/config.rs` | 624 |
| `proxy/pool.rs` | `eggsec-web-proxy/src/pool.rs` | 595 |
| `proxy/rotator.rs` | `eggsec-web-proxy/src/rotator.rs` | 418 |
| `proxy/health.rs` | `eggsec-web-proxy/src/health.rs` | 382 |
| `proxy/socks.rs` | `eggsec-web-proxy/src/socks.rs` | 584 |
| `proxy/http_connect.rs` | `eggsec-web-proxy/src/http_connect.rs` | 325 |
| `proxy/mcp.rs` | `eggsec-web-proxy/src/mcp.rs` | 484 |
| `proxy/intercept/types.rs` | `eggsec-web-proxy/src/intercept/types.rs` | 1039 |
| `proxy/intercept/rules.rs` | `eggsec-web-proxy/src/intercept/rules.rs` | 1464 |
| `proxy/intercept/interceptor.rs` | `eggsec-web-proxy/src/intercept/interceptor.rs` | 251 |
| `proxy/intercept/cert.rs` | `eggsec-web-proxy/src/intercept/cert.rs` | 180 |
| `proxy/intercept/bridge.rs` | `eggsec-web-proxy/src/intercept/bridge.rs` | 493 |
| `proxy/intercept/mod.rs` | `eggsec-web-proxy/src/intercept/mod.rs` | 1246 |
| `proxy/intercept/protocols.rs` | `eggsec-web-proxy/src/intercept/protocols.rs` | 1868 |
| `proxy/intercept/correlation.rs` | `eggsec-web-proxy/src/intercept/correlation.rs` | 1254 |
| `proxy/intercept/narrative.rs` | `eggsec-web-proxy/src/intercept/narrative.rs` | 498 |
| `proxy/intercept/plugins.rs` | `eggsec-web-proxy/src/intercept/plugins.rs` | 1013 |
| `proxy/intercept/bundle.rs` | `eggsec-web-proxy/src/intercept/bundle.rs` | 779 |
| `proxy/intercept/redteam.rs` | `eggsec-web-proxy/src/intercept/redteam.rs` | 651 |
| `proxy/intercept/transparent.rs` | `eggsec-web-proxy/src/intercept/transparent.rs` | 425 |
| `proxy/intercept/dynamic_plugins.rs` | `eggsec-web-proxy/src/intercept/dynamic_plugins.rs` | 381 |

### Files to Keep in Main Crate (adapter layer)

| File | Action |
|------|--------|
| `proxy/mod.rs` | **Rewrite**: thin adapter with re-exports + ProxyManager facade |
| `proxy/AGENTS.override.md` | Keep (update paths) |

### Tests to Move

| Source | Destination | Lines |
|--------|-------------|-------|
| `tests/proxy_cert_tests.rs` | `eggsec-web-proxy/tests/cert_tests.rs` | 101 |
| `tests/proxy_integration_tests.rs` | `eggsec-web-proxy/tests/integration_tests.rs` | 340 |
| `tests/proxy_stress_tests.rs` | `eggsec-web-proxy/tests/stress_tests.rs` | 362 |
| `tests/proxy_tests.rs` | `eggsec-web-proxy/tests/proxy_tests.rs` | 182 |

### Tests to Keep in Main Crate

| File | Reason |
|------|--------|
| `tests/benchmark_tests.rs` | Mixed proxy/scanner benchmarks; keep as adapter smoke test |

### New Files to Create

| File | Purpose |
|------|---------|
| `eggsec-web-proxy/Cargo.toml` | Domain crate manifest |
| `eggsec-web-proxy/src/lib.rs` | Domain entry points + re-exports |
| `eggsec-web-proxy/src/error.rs` | Domain error type (replaces EggsecError::Proxy usage) |
| `eggsec-web-proxy/tests/adapter_smoke.rs` | Smoke test proving main crate delegates to domain crate |

## Dependency Graph

```text
eggsec -> eggsec-web-proxy
eggsec-web-proxy -> eggsec-core (types, constants)
eggsec-web-proxy -> eggsec-output (bridge types)
```

Never:
```text
eggsec-web-proxy -> eggsec
```

## Step 1: Create Domain Crate Structure

Create `crates/eggsec-web-proxy/`:

```
crates/eggsec-web-proxy/
  Cargo.toml
  src/
    lib.rs
    error.rs
    config.rs
    pool.rs
    rotator.rs
    health.rs
    socks.rs
    http_connect.rs
    mcp.rs
    intercept/
      mod.rs
      types.rs
      rules.rs
      interceptor.rs
      cert.rs
      bridge.rs
      protocols.rs
      correlation.rs
      narrative.rs
      plugins.rs
      bundle.rs
      redteam.rs
      transparent.rs
      dynamic_plugins.rs
  tests/
    cert_tests.rs
    integration_tests.rs
    stress_tests.rs
    proxy_tests.rs
    adapter_smoke.rs
```

### Cargo.toml

```toml
[package]
name = "eggsec-web-proxy"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true
description = "Web proxy and MITM interception domain crate for Eggsec defense-lab security assessment"
authors.workspace = true
homepage.workspace = true
documentation.workspace = true
keywords = ["security", "proxy", "mitm", "intercept", "defense-lab"]
categories = ["command-line-utilities", "development-tools::testing"]

[dependencies]
# Shared types from core
eggsec-core = { path = "../eggsec-core" }
# Output bridge (for ScanReportData, FindingData)
eggsec-output = { path = "../eggsec-output" }

# TLS / certificate generation
tokio-rustls = "0.26"
rustls = "0.23"
rustls-pki-types = "1"
rcgen = "0.13"

# WebSocket interception
tokio-tungstenite = { version = "0.26", optional = true }

# HTTP/2 interception
h2 = { version = "0.4", optional = true }
http = { version = "1", optional = true }

# gRPC protobuf decoding
prost = { version = "0.13", optional = true }
prost-types = { version = "0.13", optional = true }

# Async runtime
tokio = { workspace = true, features = ["rt-multi-thread", "net", "fs", "time", "macros", "io-util", "sync", "signal", "process"] }

# HTTP client (health checks)
reqwest = { workspace = true, features = ["json"] }

# Concurrent collections
dashmap = "6"

# Random selection
rand = "0.8"

# Serialization
serde = { workspace = true }
serde_json.workspace = true
serde_yaml_neo = "1"

# Time
chrono.workspace = true

# Base64 (proxy auth)
base64 = "0.22"

# Async utilities
futures = "0.3"

# Bytes (HTTP/2 body)
bytes = "1"

# Performance
rustc_hash = "2"

# Tracing
tracing.workspace = true

# Error handling
anyhow.workspace = true

# Compression (evidence bundles)
flate2.workspace = true

[features]
default = []

# Core web proxy feature (enables intercept, protocols, rules)
web-proxy = ["dep:tokio-tungstenite", "dep:h2", "dep:http", "dep:prost", "dep:prost-types"]

# MCP tool schema types (marker only)
web-proxy-mcp = ["web-proxy"]

# Transparent proxy (Linux iptables/nftables)
transparent-proxy = ["web-proxy"]

# Dynamic plugin loading
dynamic-plugins = ["web-proxy"]
```

### error.rs

Domain error type replacing `EggsecError::Proxy` usage:

```rust
use std::fmt;

#[derive(Debug)]
pub enum WebProxyError {
    Proxy(String),
    Network(String),
    Config(String),
    Io(std::io::Error),
    Tls(String),
    Intercept(String),
    Rule(String),
    Cert(String),
    Protocol(String),
    Timeout { timeout_ms: u64, operation: String },
}

impl fmt::Display for WebProxyError { ... }
impl std::error::Error for WebProxyError { ... }
impl From<std::io::Error> for WebProxyError { ... }

pub type Result<T> = std::result::Result<T, WebProxyError>;
```

### lib.rs

Domain entry points + re-exports:

```rust
pub mod error;
pub mod config;
pub mod pool;
pub mod rotator;
pub mod health;
pub mod socks;
pub mod http_connect;
#[cfg(feature = "web-proxy-mcp")]
pub mod mcp;
pub mod intercept;

pub use config::{HealthCheckConfig, ProxyConfig, ProxyEntry, ProxyType};
pub use health::{HealthChecker, ProxyHealth};
pub use pool::ProxyPool;
pub use rotator::ProxyRotator;
pub use error::{WebProxyError, Result};

/// ProxyManager: central proxy orchestrator (pool + rotator + health checker).
pub struct ProxyManager { ... }

/// ProxiedConnection: connection result routed through proxy chain.
pub struct ProxiedConnection { ... }

impl ProxyManager {
    pub fn new(config: ProxyConfig) -> Result<Self> { ... }
    pub async fn add_proxy(&self, proxy: ProxyEntry) -> Result<()> { ... }
    // ... all methods from current proxy/mod.rs ProxyManager
}
```

## Step 2: Fix Imports in Domain Crate

All `crate::` imports in moved files must be updated:

| Old Import | New Import |
|------------|------------|
| `crate::error::{EggsecError, Result}` | `crate::error::{WebProxyError, Result}` |
| `crate::types::SensitiveString` | `eggsec_core::types::SensitiveString` |
| `crate::utils::connect_with_nodelay_timeout` | `eggsec_core::utils::connect_with_nodelay_timeout` |
| `crate::utils::create_insecure_client_with_options` | `eggsec_core::utils::create_insecure_client_with_options` |
| `crate::constants::DEFAULT_PROXY_TIMEOUT_MS` | `eggsec_core::constants::DEFAULT_PROXY_TIMEOUT_MS` |
| `crate::output::convert::{FindingData, ScanReportData}` | `eggsec_output::convert::{FindingData, ScanReportData}` |
| `crate::proxy::config::ProxyType` | `crate::config::ProxyType` |
| `crate::proxy::intercept::types::*` | `crate::intercept::types::*` |
| `crate::proxy::intercept::correlation::*` | `crate::intercept::correlation::*` |
| `crate::proxy::intercept::protocols::*` | `crate::intercept::protocols::*` |
| `crate::proxy::intercept::rules::*` | `crate::intercept::rules::*` |
| `crate::proxy::intercept::cert::*` | `crate::intercept::cert::*` |
| `crate::proxy::intercept::plugins::*` | `crate::intercept::plugins::*` |

### Error Mapping

Replace `EggsecError::Proxy(msg)` with `WebProxyError::Proxy(msg)` in all domain files.

Replace `Err(EggsecError::...)` with `Err(WebProxyError::...)`.

Update return types from `crate::error::Result<T>` to `crate::error::Result<T>`.

### Cross-Module Imports Within Domain

Files within the domain crate that reference `crate::proxy::...` need updating:

| File | Old Path | New Path |
|------|----------|----------|
| `intercept/types.rs` | `crate::proxy::intercept::correlation::*` | `crate::intercept::correlation::*` |
| `intercept/types.rs` | `crate::proxy::intercept::protocols::*` | `crate::intercept::protocols::*` |
| `intercept/bridge.rs` | `crate::proxy::intercept::correlation::*` | `crate::intercept::correlation::*` |
| `intercept/bridge.rs` | `crate::proxy::intercept::protocols::*` | `crate::intercept::protocols::*` |
| `intercept/bundle.rs` | `crate::proxy::intercept::correlation::*` | `crate::intercept::correlation::*` |
| `intercept/bundle.rs` | `crate::proxy::intercept::rules::*` | `crate::intercept::rules::*` |
| `intercept/bundle.rs` | `crate::proxy::intercept::types::*` | `crate::intercept::types::*` |
| `intercept/narrative.rs` | `crate::proxy::intercept::types::*` | `crate::intercept::types::*` |
| `intercept/narrative.rs` | `crate::proxy::intercept::correlation::*` | `crate::intercept::correlation::*` |
| `intercept/mod.rs` (tests) | `crate::proxy::intercept::rules::*` | `crate::intercept::rules::*` |
| `intercept/mod.rs` (tests) | `crate::proxy::intercept::cert::*` | `crate::intercept::cert::*` |
| `health.rs` (tests) | `crate::proxy::config::*` | `crate::config::*` |
| `rotator.rs` (tests) | `crate::proxy::config::*` | `crate::config::*` |

## Step 3: Create Adapter Layer in Main Crate

Rewrite `crates/eggsec/src/proxy/mod.rs` as thin adapter:

```rust
// Re-export everything from the domain crate
pub use eggsec_web_proxy::*;
pub use eggsec_web_proxy::{
    HealthCheckConfig, ProxyConfig, ProxyEntry, ProxyType,
    HealthChecker, ProxyHealth, ProxyPool, ProxyRotator,
    ProxiedConnection, ProxyManager,
};

// Re-export intercept module
pub use eggsec_web_proxy::intercept;
```

**No domain logic remains in this file.** The adapter:
1. Re-exports all types at `crate::proxy::...` paths (TUI/CLI compatibility)
2. Forwards any convenience functions
3. Handles error mapping (if needed for CLI handlers)

### Remove old files from main crate

Delete these files from `crates/eggsec/src/proxy/`:
- `config.rs`, `pool.rs`, `rotator.rs`, `health.rs`, `socks.rs`, `http_connect.rs`, `mcp.rs`
- `intercept/types.rs`, `intercept/rules.rs`, `intercept/interceptor.rs`, `intercept/cert.rs`
- `intercept/bridge.rs`, `intercept/mod.rs`, `intercept/protocols.rs`, `intercept/correlation.rs`
- `intercept/narrative.rs`, `intercept/plugins.rs`, `intercept/bundle.rs`
- `intercept/redteam.rs`, `intercept/transparent.rs`, `intercept/dynamic_plugins.rs`

Keep:
- `proxy/mod.rs` (rewritten as adapter)
- `proxy/AGENTS.override.md`

## Step 4: Update Cargo.toml Files

### Root `Cargo.toml`

Add workspace member:
```toml
members = [
    ...,
    "crates/eggsec-web-proxy",
]
```

### `crates/eggsec/Cargo.toml`

Add dependency:
```toml
[dependencies]
eggsec-web-proxy = { path = "../eggsec-web-proxy", optional = true }
```

Update feature:
```toml
[features]
web-proxy = ["dep:eggsec-web-proxy", "dep:tokio-tungstenite", "dep:h2", "dep:http", "dep:prost", "dep:prost-types"]
web-proxy-mcp = ["web-proxy", "eggsec-web-proxy/web-proxy-mcp"]
transparent-proxy = ["web-proxy", "eggsec-web-proxy/transparent-proxy"]
dynamic-plugins = ["web-proxy", "eggsec-web-proxy/dynamic-plugins"]
```

### `crates/eggsec-cli/Cargo.toml`

No change needed (already re-exports `web-proxy` feature).

### `crates/eggsec-tui/Cargo.toml`

No change needed (already re-exports `web-proxy` feature).

## Step 5: Move Tests

Move these test files from `crates/eggsec/tests/` to `crates/eggsec-web-proxy/tests/`:

| File | Action |
|------|--------|
| `proxy_cert_tests.rs` | Move, update imports: `eggsec::proxy::intercept::*` -> `eggsec_web_proxy::intercept::*` |
| `proxy_integration_tests.rs` | Move, update imports similarly |
| `proxy_stress_tests.rs` | Move, update imports similarly |
| `proxy_tests.rs` | Move, update imports: `eggsec::proxy::*` -> `eggsec_web_proxy::*` |

### Keep in main crate

| File | Action |
|------|--------|
| `benchmark_tests.rs` | Keep as-is (mixed proxy/scanner benchmarks; adapter smoke test) |

### Add adapter smoke test

Create `crates/eggsec/tests/proxy_adapter_smoke.rs`:

```rust
#![cfg(feature = "web-proxy")]

#[test]
fn proxy_types_reexported_from_main_crate() {
    use eggsec::proxy::{ProxyEntry, ProxyType, ProxyConfig};
    use eggsec::proxy::intercept::types::{ProxyFlow, WebProxySessionReport};
    // Verify adapter re-exports work
    let _ = ProxyEntry::default();
    let _ = ProxyFlow::default();
}
```

## Step 6: Fix TUI Imports

The TUI imports proxy types via `eggsec::proxy::...` paths. Since the main crate re-exports everything, **TUI paths should remain unchanged**. However, verify:

1. `crates/eggsec-tui/src/tabs/proxy.rs` - imports `eggsec::proxy::{HealthCheckConfig, HealthChecker, ProxyEntry, ProxyType}` → re-exports work
2. `crates/eggsec-tui/src/tabs/intercept/mod.rs` - imports `eggsec::proxy::intercept::...` → re-exports work
3. `crates/eggsec-tui/src/tabs/intercept/types.rs` - imports `eggsec::proxy::intercept::types::ProxyFlowDirection` → re-exports work
4. `crates/eggsec-tui/src/tabs/intercept/render.rs` - inline `eggsec::proxy::intercept::...` paths → re-exports work
5. `crates/eggsec-tui/src/tabs/intercept/tests.rs` - inline `eggsec::proxy::intercept::...` paths → re-exports work
6. `crates/eggsec-tui/src/workers/intercept_worker.rs` - `eggsec::proxy::intercept::types::InterceptSession` → re-exports work
7. `crates/eggsec-tui/src/workers/runner.rs` - `eggsec::proxy::intercept::types::InterceptSession` → re-exports work

**No TUI changes needed** if re-exports are complete.

## Step 7: Fix CLI/Tool/Pipeline/Config Imports

These modules import proxy types via `crate::proxy::...`:

| Module | Imports | Action |
|--------|---------|--------|
| `tool/implementations/proxy.rs` | `crate::proxy::intercept::types::{ProxyFlow, WebProxySessionReport}` | No change (re-exports) |
| `commands/proxy.rs` | `crate::proxy::{ProxyEntry, ProxyType}` | No change (re-exports) |
| `commands/handlers/web_proxy.rs` | `crate::proxy::intercept::correlation::*`, `crate::proxy::intercept::protocols::*` | No change (re-exports) |
| `commands/handlers/stress.rs` | `crate::proxy::{HealthCheckConfig, HealthChecker, ProxyEntry}` | No change (re-exports) |
| `config/settings.rs` | `crate::proxy::ProxyType` | No change (re-exports) |
| `stress/http.rs` | `crate::proxy::{ProxyEntry, ProxyManager, ProxyType}` | No change (re-exports) |
| `pipeline/executor.rs` | `crate::proxy::intercept::types::WebProxySessionReport` | No change (re-exports) |

**No changes needed** in these modules.

## Step 8: Fix Benchmarks

Update `crates/eggsec/benches/proxy_benchmarks.rs`:

```rust
// Old
use eggsec::proxy::intercept::types::{FlowBuffer, ProxyFlow, WebProxySessionReport};
use eggsec::proxy::intercept::{EnhancedRule, EnhancedRuleSet, ...};

// New (still works via re-exports)
use eggsec::proxy::intercept::types::{FlowBuffer, ProxyFlow, WebProxySessionReport};
use eggsec::proxy::intercept::{EnhancedRule, EnhancedRuleSet, ...};
```

No change needed if re-exports are complete.

## Step 9: Update Documentation

### Files to Update

| File | Change |
|------|--------|
| `architecture/overview.md` | Add `eggsec-web-proxy` to workspace crate layout |
| `architecture/proxy.md` | Add domain crate extraction status, split Files section |
| `architecture/web_proxy.md` | Add domain crate extraction status, update Files section |
| `AGENTS.md` | Add `eggsec-web-proxy` build/test commands |

### New Documentation

No new architecture docs needed — existing `proxy.md` and `web_proxy.md` cover the domain.

## Step 10: Validation

Run after extraction:

```bash
cargo fmt --all
cargo check -p eggsec-web-proxy
cargo check -p eggsec --features web-proxy
cargo check -p eggsec-tui --features web-proxy
cargo test -p eggsec-web-proxy --lib
cargo test -p eggsec-web-proxy
cargo test -p eggsec --features web-proxy --lib
cargo test -p eggsec --features web-proxy --test proxy_adapter_smoke
cargo test -p eggsec-tui --features web-proxy
cargo clippy -p eggsec-web-proxy
cargo clippy -p eggsec --features web-proxy
```

## Risk Assessment

| Risk | Mitigation |
|------|------------|
| TUI import breakage | Re-export everything at same paths; verify with `cargo check -p eggsec-tui --features web-proxy` |
| Cyclic dependency | Domain crate depends only on `eggsec-core` + `eggsec-output`; never on `eggsec` |
| Feature flag regression | Test all feature combinations: `web-proxy`, `web-proxy-mcp`, `transparent-proxy`, `dynamic-plugins` |
| Error type incompatibility | Domain crate has its own `WebProxyError`; adapter maps to `EggsecError` where needed |
| Test breakage | Move tests to domain crate; keep adapter smoke test in main crate |

## Acceptance Criteria

- [ ] `eggsec-web-proxy` crate exists and is workspace member
- [ ] Main crate depends on it optionally behind `web-proxy` feature
- [ ] No cyclic dependency
- [ ] All existing CLI/TUI/MCP/REST behavior preserved
- [ ] Enforcement still occurs in main crate before domain execution
- [ ] Domain-specific tests pass in new crate
- [ ] Main integration tests pass
- [ ] `cargo clippy -p eggsec-web-proxy` clean
- [ ] `cargo clippy -p eggsec --features web-proxy` clean (or same pre-existing warnings)
- [ ] Compile-time dependency footprint improves for default builds (web-proxy opt-in)

## Non-Goals

- Do not change user-visible behavior
- Do not extract enforcement into domain crate
- Do not change TUI import paths (re-exports preserve compatibility)
- Do not extract CLI handlers or TUI tabs (stay in main crate)

## Estimated Effort

| Step | Effort |
|------|--------|
| Create crate structure + Cargo.toml | Small |
| Move 23 source files | Medium (git mv) |
| Fix imports in domain crate | Large (many `crate::` path changes) |
| Create adapter layer | Small |
| Update Cargo.toml files | Small |
| Move 4 test files + fix imports | Medium |
| Add adapter smoke test | Small |
| Verify TUI compatibility | Small (cargo check) |
| Fix CLI/tool/pipeline imports | Small (re-exports handle it) |
| Update documentation | Small |
| Run validation suite | Medium |

Total: Larger than db-pentest extraction due to scale (15K lines vs 2K) and import density.

## Implementation Order

1. Create `crates/eggsec-web-proxy/Cargo.toml`
2. Create `crates/eggsec-web-proxy/src/error.rs`
3. `git mv` all 23 source files from main crate to domain crate
4. Create `crates/eggsec-web-proxy/src/lib.rs` (entry points + re-exports)
5. Fix all `crate::` imports in domain crate files
6. Rewrite `crates/eggsec/src/proxy/mod.rs` as adapter
7. Delete old files from main crate
8. Update root `Cargo.toml` (add workspace member)
9. Update `crates/eggsec/Cargo.toml` (add dependency, update features)
10. `git mv` test files to domain crate
11. Fix test imports
12. Create adapter smoke test
13. `cargo check -p eggsec-web-proxy` (fix remaining import issues)
14. `cargo check -p eggsec --features web-proxy` (verify adapter)
15. `cargo check -p eggsec-tui --features web-proxy` (verify TUI compatibility)
16. Run full test suite
17. Update documentation
18. Commit and push
