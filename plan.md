# Slapper Codebase Improvement Plan

Consolidated plan from code reviews (2026-03-28). Merges items from plan2–5, deduplicates, corrects inaccuracies.

**Codebase Rating: 8.5/10** — Well-engineered with clear separation of concerns and solid test coverage.

---

## Already Addressed (No Action Needed)

| Item | Location | Status |
|------|----------|--------|
| Regex pre-compilation | `crates/slapper/src/recon/secrets.rs` (uses `once_cell::sync::Lazy`) | Done |
| Serialization unwraps in tests | `crates/slapper/src/waf/detector.rs`, `crates/slapper/src/scanner/fingerprint.rs`, `crates/slapper/src/scanner/endpoints.rs` — all in `#[cfg(test)]` blocks | Acceptable |

---

## Critical Priority

### 1. TLS Fallback to Plaintext Exposes PSK (Security)

**File:** `crates/slapper/src/distributed/remote.rs:79-102`

`CoordinatorServer::with_tls()` and `WorkerClient::with_tls()` silently fall back to plaintext when TLS initialization fails. The PSK is transmitted in `AuthMessage` — if TLS fails, credentials go over cleartext.

**Fix:**
- Change `CoordinatorServer::with_tls()` return type to `anyhow::Result<Self>`, return `Err` on TLS failure
- `WorkerClient::with_tls()` already returns `Result` — propagate the error instead of swallowing it
- Add separate `new_plaintext()` constructors with clear naming and docs for intentional plaintext

**Verify:** `cargo test --lib -p slapper -- distributed`

---

### 2. HTTP Flood Missing Feature Gate (Security)

**File:** `crates/slapper/src/stress/mod.rs:147-149`

`StressType::Http` / `StressType::Tcp` runs `http::run_http_flood()` without any `#[cfg(feature = "stress-testing")]` gate. Inconsistent with SYN/UDP/ICMP which are gated.

**Fix:**
```rust
StressType::Http | StressType::Tcp => {
    #[cfg(feature = "stress-testing")]
    { http::run_http_flood(&self.config, &self.metrics).await? }
    #[cfg(not(feature = "stress-testing"))]
    { anyhow::bail!("HTTP/TCP flood requires 'stress-testing' feature"); }
}
```

**Verify:** `cargo check --lib -p slapper` (no features) and `cargo test --lib -p slapper --features stress-testing`

---

### 3. IP Allowlist Bypass via Imprecise Prefix Matching (Security)

**File:** `crates/slapper/src/distributed/remote.rs:179-183`

String prefix matching allows bypass. E.g., allowlist entry `"10.0.0"` matches IP `"10.0.01"` because `"10.0.01".starts_with("10.0.0")` is true.

**Fix:** Use proper CIDR matching via the existing `ipnetwork` crate, or exact dot-boundary segment comparison.

**Verify:** `cargo test --lib -p slapper -- distributed`

---

## High Priority

### 4. SensitiveString Converted to Plain String in Notify (Security)

**File:** `crates/slapper/src/notify/mod.rs:73`

`from_settings()` converts `SensitiveString` webhook secrets to plain `String` via `expose_secret().to_string()`, defeating zeroization.

**Fix:**
- Change `WebhookConfig.secret` from `Option<String>` to `Option<SensitiveString>` in `crates/slapper/src/notify/webhook.rs`
- Update `from_settings()` to clone the `SensitiveString`
- Update code reading `webhook_config.secret` to use `expose_secret()`

**Verify:** `cargo test --lib -p slapper -- notify`

---

### 5. Proxy Passwords Stored as Plain String (Security)

**File:** `crates/slapper/src/proxy/config.rs:71`

`ProxyEntry.password` is `Option<String>`, not `SensitiveString`. The `to_url()` method embeds credentials in URLs that may be logged.

**Fix:**
- Change to `Option<SensitiveString>`
- Update `with_auth()`, `to_url()`, `from_str()` accordingly
- Serde handles `SensitiveString` transparently

**Verify:** `cargo test --lib -p slapper -- proxy`

---

### 6. Silent Error Swallowing in Fuzzer Concurrent Mode (Correctness)

**File:** `crates/slapper/src/fuzzer/engine/execution.rs:108-110`

Failed requests in `run_concurrent()` are silently dropped. Cannot distinguish "no vulnerability" from "request failed."

**Fix:**
```rust
match result {
    Ok(r) => { results.lock().await.push(r); }
    Err(e) => { tracing::debug!("Fuzz request failed: {e}"); }
}
```

**Verify:** `cargo test --lib -p slapper -- fuzzer`

---

### 7. Fragile String-Based PayloadType Dispatch (Correctness)

**File:** `crates/slapper/src/fuzzer/engine/core.rs:207-209`

Uses `format!("{:?}", pt).to_lowercase()` to match against hardcoded string array. If `PayloadType` Debug repr changes, dispatch breaks silently.

**Fix:** Add `PayloadType::is_advanced()` or `PayloadType::advanced_name()` method. Replace string dispatch with method call.

**Verify:** `cargo test --lib -p slapper -- fuzzer`

---

### 8. Remove Unused `dispatch_blocking()` Methods (Cleanup)

**Files:** `crates/slapper/src/tool/registry.rs:154-164`, `crates/slapper/src/tool/dispatcher.rs:79-93`

Both methods are defined but never called. They use `rt.block_on()` which would panic if called from within an async context. Dead code that could mislead.

**Fix:** Remove both methods entirely.

**Verify:** `cargo test --lib -p slapper`

---

### 9. Excessive `unwrap()` in Production Code

**Files:**
- `crates/slapper/src/proxy/mod.rs:171` — `"0.0.0.0:0".parse::<SocketAddr>().unwrap()`
- `crates/slapper/src/scanner/ports/mod.rs:284` — `ProgressStyle::template().unwrap()`
- `crates/slapper/src/stress/metrics.rs:153` — `self.last_refill.lock().unwrap()`

**Fix:** Replace with `.expect("descriptive message")` or proper error propagation via `context()` / `map_err()`.

**Verify:** `cargo clippy --lib -p slapper`

---

### 10. Excessive Clones in Recon Module

**File:** `crates/slapper/src/recon/mod.rs:276-284`

Creates 9 clones of `resolved_ip` and 5 clones of `domain` for `tokio::join!` futures.

**Fix:** Wrap in `Arc<String>` and clone the `Arc` instead.

**Verify:** `cargo test --lib -p slapper -- recon`

---

### 11. Adaptive Mode Is No-Op Alias (Correctness)

**File:** `crates/slapper/src/fuzzer/engine/execution.rs:177-182`

`run_adaptive_with_session()` delegates directly to `run_sequential_with_session()`. The advertised "adaptive" mode does not adapt.

**Fix:** Wire the existing `AdaptiveRateLimiter` (from `fuzzer/rate_limit.rs`) into the method. The infrastructure already exists.

**Verify:** `cargo test --lib -p slapper -- fuzzer`

---

## Medium Priority

### 12. Payload Cloning in FuzzResult

**File:** `crates/slapper/src/fuzzer/engine/execution.rs:214,228`

Clones entire payload on every fuzz result, including error paths.

**Fix:** Use `Arc<Payload>` in `FuzzResult`, or move payload on success and only clone on error.

**Verify:** `cargo test --lib -p slapper -- fuzzer`

---

### 13. Fuzzer Concurrency Has Minimum Floor of 100

**File:** `crates/slapper/src/fuzzer/engine/core.rs:87`

`args.concurrency.max(100)` sets a **minimum** of 100, not a cap. A user setting `--concurrency 50` silently gets 100 with no warning.

**Fix:** Log a warning when the floor is applied: `if args.concurrency < 100 { tracing::warn!(...); }`. Or validate at the CLI layer.

**Verify:** `cargo test --lib -p slapper -- fuzzer`

---

### 14. Redundant Unwrap on Just-Assigned Option

**File:** `crates/slapper/src/tui/workers/runner.rs:195-196` and `:878-879`

Assigns `Some(e)` then immediately calls `.unwrap()` on it.

**Fix:** Use `e.to_string()` directly before wrapping in `Some(e)`.

**Verify:** `cargo check --lib -p slapper`

---

### 15. Unnecessary Clone of Results Vec

**File:** `crates/slapper/src/scanner/ports/mod.rs:327`

`results.lock().await.clone()` clones the entire vector just to sort it.

**Fix:** Sort in-place while holding the lock, or restructure to avoid the clone.

**Verify:** `cargo test --lib -p slapper -- scanner`

---

### 16. Config Clone in ProxyPool

**File:** `crates/slapper/src/proxy/mod.rs:38`

Clones entire config just to create proxy pool.

**Fix:** Accept only necessary fields or use `Arc`.

**Verify:** `cargo test --lib -p slapper -- proxy`

---

### 17. Dead Code Annotations Hide Unused Code

**Files:** `crates/slapper/src/recon/wayback.rs`, `recon/subdomain.rs`, `recon/ssl.rs`, `proxy/socks.rs`, `proxy/pool.rs`, `waf/bypass/smuggling.rs`, `fuzzer/redos_detect.rs`, `utils/rate_limiter.rs`

Many wired modules have file-level `#![allow(dead_code)]` suppressing warnings for genuinely unused items.

**Fix:** Remove file-level annotations one module at a time. Wire in, document, or remove each flagged item.

**Verify:** `cargo check --lib -p slapper` with zero file-level `#![allow(dead_code)]` remaining.

---

### 18. `deny.toml` Missing License and Ban Configuration

**File:** `deny.toml`

Only advisory checks configured. No license compliance or duplicate crate banning.

**Fix:** Add `[licenses]` allowlist (MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC) and `[bans]` section.

**Verify:** `cargo deny check`

---

### 19. No Pinned Rust Toolchain

No `rust-toolchain.toml`. CI uses `dtolnay/rust-toolchain@stable` with no version pin.

**Fix:** Create `rust-toolchain.toml` with channel = "stable" and components = ["rustfmt", "clippy"].

**Verify:** `cargo check --lib -p slapper`

---

### 20. Clippy Configuration

No clippy deny configuration for unwrap/expect in production code.

**Fix:** Add `#![deny(clippy::unwrap_used)]` and `#![deny(clippy::expect_used)]` to `lib.rs`, or use `clippy.toml`.

**Verify:** `cargo clippy --lib -p slapper`

---

## Low Priority

### 21. Standardize Error Types

Mixed usage of `anyhow::Result` (76 files), `SlapperError::Result<T>` (tool API), and `std::io::Result` (e.g., `utils/privilege.rs`). Callers cannot pattern-match on error types.

**Fix:** Migrate core library modules from `anyhow::Result` to `crate::error::Result<T>` incrementally. Keep `anyhow` for CLI command handlers and TUI. Add missing `SlapperError` variants for Proxy, Recon, Fingerprint, LoadTest.

**Verify:** `anyhow` usage limited to `main.rs`, command handlers, and TUI after migration.

---

### 22. Dual TLS Backends

Both `native-tls` (distributed) and `rustls` (HTTP via reqwest) are compiled. Increases binary size and attack surface.

**Fix:** Migrate `distributed/io.rs` from `tokio-native-tls` to `tokio-rustls`. Requires certificate format migration.

**Verify:** `cargo tree -p slapper | grep native-tls` returns nothing.

---

### 23. Pipeline Stages Execute Sequentially Only

**File:** `crates/slapper/src/pipeline/executor.rs`

Independent stages (e.g., PortScan + Recon) could run in parallel but execute sequentially.

**Fix:** Add dependency graph; run independent stages concurrently via `tokio::join!`.

---

### 24. Stub Encoder Implementations in Ruby API

**File:** `crates/slapper-ruby/src/api.rs:934-949`

`encoder_encode()` and `encoder_compatible_payloads()` return `Err("not yet implemented")`.

**Fix:** Implement via MSF RPC delegation or remove from API surface.

---

### 25. Untracked Spawned Task

**File:** `crates/slapper/src/distributed/worker.rs:98-119`

Heartbeat task spawns with no `JoinHandle` captured.

**Fix:** Store `JoinHandle` and ensure cleanup on shutdown.

---

## Larger Refactoring (Ongoing / Future)

### 26. Split Large Files

| File | Lines | Proposed Split |
|------|-------|----------------|
| `crates/slapper/src/tui/app.rs` | 2193 | `state.rs`, `events.rs`, `layout.rs` |
| `crates/slapper/src/tool/protocol/mcp.rs` | 1710 | `handlers.rs`, `types.rs`, `server.rs` |
| `crates/slapper/src/tui/workers/runner.rs` | 1192 | `scan_worker.rs`, `fuzz_worker.rs` |
| `crates/slapper/src/packet/parse.rs` | 1111 | `headers.rs`, `payload.rs` |
| `crates/slapper/src/tui/tabs/settings.rs` | 783 | `form.rs`, `validation.rs` |
| `crates/slapper/src/fuzzer/payloads/jwt.rs` | 766 | `algorithms.rs`, `claims.rs` |
| `crates/slapper/src/waf/waf_patterns.rs` | 743 | Split by vendor |

### 27. Documentation

- Add `# Examples` and `# Errors` to all public functions
- Add module-level docs to `crates/slapper/src/distributed/`, `pipeline/`, `notify/`, `proxy/`
- Update `ARCHITECTURE.md` with sequence diagrams and feature flag dependencies

### 28. Testing

- Add CIDR boundary and IPv6 scope tests for `crates/slapper/src/config/scope.rs`
- Add WireMock fixtures for common WAF responses
- Expand property-based testing (proptest) for port parser, URL normalization
- Benchmark WAF detection, payload generation, scanner throughput

### 29. Performance

- Cache frequently-used payloads with `once_cell::Lazy` (return `&'static [Payload]`)
- Use `Cow<str>` for WAF header comparisons
- Implement work-stealing for port scanning
- Use `bytes::BytesMut` pool for HTTP response buffers

### 30. CI/CD

- Matrix build for feature combinations
- Add `cargo-deny`, `cargo-machete`, typos checker
- Automate changelog generation
- Track test coverage with `cargo-tarpaulin`

---

## Execution Order

```
Critical (do first):
  1.  TLS fallback to plaintext       (security)
  2.  HTTP flood feature gate          (security)
  3.  IP allowlist bypass              (security)

High:
  4.  Notify SensitiveString leak      (security)
  5.  Proxy password SensitiveString   (security)
  6.  Fuzzer silent error swallowing   (correctness)
  7.  Fragile PayloadType dispatch     (correctness)
  8.  Remove unused dispatch_blocking  (cleanup)
  9.  Fix unwrap() in production code  (reliability)
  10. Reduce clones in recon           (performance)
  11. Wire adaptive mode               (correctness)

Medium:
  12. Payload cloning in FuzzResult    (performance)
  13. Concurrency floor warning        (correctness)
  14. Redundant unwrap in TUI runner   (code quality)
  15. Unnecessary results clone        (performance)
  16. Config clone in ProxyPool        (performance)
  17. Dead code annotations cleanup    (maintainability)
  18. deny.toml license/ban config     (compliance)
  19. Pinned Rust toolchain            (CI stability)
  20. Clippy configuration             (quality gates)

Low:
  21. Standardize error types          (architecture)
  22. Unify TLS backends               (architecture)
  23. Pipeline parallel stages         (performance)
  24. Ruby encoder stubs               (correctness)
  25. Untracked spawned task           (cleanup)

Ongoing:
  26. Split large files                (maintainability)
  27. Documentation improvements       (developer experience)
  28. Testing enhancements             (quality)
  29. Performance optimizations        (performance)
  30. CI/CD improvements               (quality gates)
```

---

## Verification Commands

After each item:

```bash
cargo check --lib -p slapper
cargo test --lib -p slapper
cargo clippy --lib -p slapper -- -D warnings
```

For feature-gated items:

```bash
cargo check --lib -p slapper --features full
cargo test --lib -p slapper --features full
```

For integration tests:

```bash
cargo test --test negative_tests -p slapper
cargo test --test scanner_tests -p slapper
cargo test --test waf_tests -p slapper
```

---

## Success Criteria

| Criterion | Current | Target |
|-----------|---------|--------|
| TLS fallback | Silent plaintext | Error on TLS failure |
| HTTP flood feature gate | None | Gated behind `stress-testing` |
| Webhook secret storage | Plain `String` | `SensitiveString` |
| Proxy password storage | Plain `String` | `SensitiveString` |
| Fuzzer concurrent errors | Silent drop | Logged via `tracing::warn!` |
| PayloadType dispatch | String-based | Compile-time method |
| `dispatch_blocking` | Unused, can deadlock | Removed |
| IP allowlist | String prefix | Proper CIDR matching |
| Adaptive mode | No-op alias | Real adaptive logic |
| Dead code annotations | File-level suppressions | All removed |
| `deny.toml` | Advisory only | + licenses + bans |
| Toolchain | Unpinned | `rust-toolchain.toml` |
| Production unwrap() | 3 locations | 0 (all have context/error) |
| All tests | Passing | Still passing |
| Clippy warnings | 0 | 0 |

---

## Risk Assessment

### High-Risk (require extensive testing)

| Change | Risk | Mitigation |
|--------|------|------------|
| TLS fallback change | Breaking distributed connections | Keep `new_plaintext()` fallback, add integration tests |
| Payload caching | Stale payload data | Ensure `Lazy` is truly static |
| Adaptive mode wiring | Behavioral change in fuzzer | Add unit tests for rate adjustment |
| Dead code removal | Removing needed code | Do one module at a time, run full test suite |

### Medium-Risk (standard review)

| Change | Risk | Mitigation |
|--------|------|------------|
| SensitiveString migration | Breaking serde deserialization | Serde handles transparently, test config loading |
| IP allowlist change | Breaking legitimate CIDR patterns | Test with existing allowlist configs |
| Error type migration | Breaking call sites | Incremental, one module at a time |

### Low-Risk (safe to implement)

| Change | Risk |
|--------|------|
| Documentation additions | None |
| New test cases | None |
| Clippy configuration | None |
| `deny.toml` additions | None |
| `rust-toolchain.toml` creation | None |
