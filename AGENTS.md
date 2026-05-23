# AGENTS.md

Guidelines for AI agents working on this codebase.

## Project Overview

Slapper is a Rust-based security testing toolkit. See `README.md` for features and `ARCHITECTURE.md` for design details.

## Quick Reference

### Build & Test Commands

```bash
cargo check --lib -p slapper
cargo test --lib -p slapper
cargo test --test negative_tests -p slapper
cargo test --test scanner_tests -p slapper
cargo clippy --lib -p slapper
cargo build --release -p slapper
```

### Ruby Plugin Build Note

For `all-plugins` or `ruby-plugins` builds on macOS, prefer Homebrew Ruby over system Ruby:

```bash
RUBY=/usr/local/opt/ruby/bin/ruby RB_SYS_STABLE_API_COMPILED_FALLBACK=1 cargo check --lib -p slapper --features all-plugins
```

Reason: system Ruby (2.6) can fail to provide symbols expected by `magnus`/`rb-sys` during Rust compilation.

### Module Override Files

For specialized guidance on specific modules, see `AGENTS.override.md` in each module directory:

| Module | Override File |
|--------|---------------|
| `agent/` | `crates/slapper/src/agent/AGENTS.override.md` |
| `ai/` | `crates/slapper/src/ai/AGENTS.override.md` |
| `fuzzer/` | `crates/slapper/src/fuzzer/AGENTS.override.md` |
| `scanner/` | `crates/slapper/src/scanner/AGENTS.override.md` |
| `tui/` | `crates/slapper/src/tui/AGENTS.override.md` |
| `waf/` | `crates/slapper/src/waf/AGENTS.override.md` |
| `recon/` | `crates/slapper/src/recon/AGENTS.override.md` |
| `tool/` | `crates/slapper/src/tool/AGENTS.override.md` |
| `config/` | `crates/slapper/src/config/AGENTS.override.md` |
| `output/` | `crates/slapper/src/output/AGENTS.override.md` |
| `proxy/` | `crates/slapper/src/proxy/AGENTS.override.md` |
| `stress/` | `crates/slapper/src/stress/AGENTS.override.md` |
| `distributed/` | `crates/slapper/src/distributed/AGENTS.override.md` |
| `packet/` | `crates/slapper/src/packet/` (uses pnet, pnet_packet for raw sockets) |
| `loadtest/` | `crates/slapper/src/loadtest/AGENTS.override.md` |
| `pipeline/` | `crates/slapper/src/pipeline/AGENTS.override.md` |
| `nse/` | `slapper-nse/` (Lua VM, NSE libraries, sandbox, CVE integration) |

### Feature Flags

- `stress-testing` - Raw sockets, IP spoofing
- `packet-inspection` - Packet capture
- `python-plugins` / `ruby-plugins` - Plugin language support
- `rest-api` / `grpc-api` - API server integration
- `nse` - Nmap NSE script support
- `ai-integration` - AI planner, script generation, autonomous agent skills
- `ws-api` - WebSocket pub/sub
- `full` - All features combined

### Key Types

- `SlapperConfig` - Main configuration (`config::load_config()`)
- `Severity` - Unified severity (in `types.rs`, re-exported everywhere)
- `TabError` - Structured error type with categories (Network, Auth, Config, Resource, Target, Internal, Unknown) in `tui/app/tab_error.rs`
- `SensitiveString` - Zeroized credential wrapper
- `FuzzEngine` / `FuzzResult` - Fuzzing engine
- `PayloadType` - Enum of 31 payload categories
- `AiClient` / `Provider` - AI LLM client and provider enum
- `AiCache` / `CacheKeyBuilder` - TTL cache for AI responses
- `SmartWafBypass` - WAF bypass with knowledge base
- `AiPlanner` - AI-driven execution planning (requires `ai-integration`)

### Important Patterns

- **Severity Enum**: Single canonical definition in `types.rs`. Re-export, don't recreate.
- **TabError Enum**: Structured error handling for tabs with `is_recoverable()` method for auto-recovery logic
- **Tool Abstraction**: `tool/traits.rs` has `SecurityTool` trait, `tool/registry.rs` has `ToolRegistry`
- **Regex Caching**: Use `lru = "0.18"` with cache size 100 (NonZeroUsizer)
- **Circuit Breaker**: `utils/circuit_breaker.rs` - `CircuitBreaker` + `CircuitBreakerRegistry`
- **Truncation**: `utils/formatting.rs` - `strip_controls` (recommended) and `preserve_all`
- **Visual Regression Testing**: Use `TestBackend` + `Terminal::new()` with `terminal.backend().buffer()` to verify rendered content
- **AI Cache Keys**: Always use `CacheKeyBuilder` for cache keys in AI module to avoid collisions
- **Hash Collections**: Use `rustc_hash::FxHashMap` and `rustc_hash::FxHashSet` instead of std collections for performance
- **Error Handling**: Avoid `unwrap_or_default()` on async operations; use explicit match with tracing instead

### Codebase Health

| Metric | Value |
|--------|-------|
| Tests | 1324 base, 1469+ with full features |
| Clippy | ~33 warnings (pre-existing, none in ai module) |
| Source files | 743 |
| Payload types | 31 |
| Tabs | 29 |

### Security Notes

- **Scope Enforcement**: Direct IP addresses (e.g., `127.0.0.1`) are blocked via private IP checks in `TargetScope::parse()`. However, scope rule evaluation happens AFTER private IP check - so targets like `10.255.255.255` are rejected even with scope rules like `allow 10.0.0.0/8`.
- **TUI Settings Tab**: Only exposes a subset of config fields. Saving via the TUI will cause data loss for `profiles`, `schedule`, `remote`, `ai`, `search`, `alert_channels`, and other fields not shown in the UI.

### Key Patterns

- **Division by zero guard**: Always check `if self.stages.is_empty()` before division
- **Scroll offset bounds**: Use `self.lines.is_empty()` check before calculating scroll_offset
- **Arc::try_unwrap**: Use `map_err` instead of `.expect()` to avoid panic
- **LazyLock regex**: Use `.expect()` with descriptive message instead of `.unwrap()`
- **FxHashMap/FxHashSet**: Always use for performance in new code

## Skills Directory

Skills are located in:
- `.opencode/skills/slapper-agent/` - Agent-specific workflows
- `.opencode/skills/slapper-ai/` - AI module workflows
- `.opencode/skills/slapper-cli/` - CLI parsing, command dispatch, handler patterns
- `.opencode/skills/slapper-config/` - Config module workflows
- `.opencode/skills/slapper-distributed/` - Distributed module workflows
- `.opencode/skills/slapper-fuzzer/` - Fuzzer module workflows
- `.opencode/skills/slapper-output/` - Output module workflows
- `.opencode/skills/slapper-proxy/` - Proxy module workflows
- `.opencode/skills/slapper-recon/` - Reconnaissance module workflows
- `.opencode/skills/slapper-scanner/` - Scanner module workflows
- `.opencode/skills/slapper-security/` - Security testing skill workflows
- `.opencode/skills/slapper-stress/` - Stress module workflows
- `.opencode/skills/slapper-nse/` - NSE/Lua module workflows
- `.opencode/skills/slapper-packet/` - Packet capture/crafting/parsing workflows
- `.opencode/skills/slapper-loadtest/` - Loadtest module workflows
- `.opencode/skills/slapper-pipeline/` - Pipeline module workflows
- `.opencode/skills/slapper-tool/` - Tool module workflows
- `.opencode/skills/slapper-tui/` - TUI module workflows
- `.opencode/skills/slapper-waf/` - WAF module workflows
- `.opencode/skills/slapper-architecture-review/` - Architecture document review workflows
- `.opencode/skills/slapper-wave-implementation/` - Multi-wave plan execution patterns
- `.opencode/skills/tui-testing/` - TUI testing patterns and guides

Use the `skill` tool to load relevant skills when tackling tasks in their domain.

## Architecture Documentation

Detailed architecture documentation is in the `architecture/` directory:

| File | Module |
|------|--------|
| `architecture/cli_commands.md` | CLI parsing, command dispatch, handler patterns |
| `architecture/ai_agents.md` | AI/LLM integration and autonomous agents |
| `architecture/config.md` | Configuration system, scope enforcement |
| `architecture/scanner.md` | Port scanning and endpoint discovery |
| `architecture/fuzzer.md` | Fuzzing engine and payload generation |
| `architecture/waf.md` | WAF detection and bypass |
| `architecture/recon.md` | Reconnaissance module |
| `architecture/pipeline.md` | Security assessment pipeline |
| `architecture/distributed.md` | Distributed coordinator/worker architecture |
| `architecture/loadtest.md` | HTTP load testing and benchmarking |
| `architecture/networking.md` | Networking & packets module |
| `architecture/output.md` - Output & reporting module |
| `architecture/plugins_nse.md` | Plugin system (Python/Ruby) and NSE integration |
| `architecture/tui.md` | Terminal User Interface (TUI) module, 29 tabs, event loop, components |

---

## Implementation Notes

- **NSE module** (`slapper-nse/`) is a separate crate - use `cargo check -p slapper-nse` for validation
- **Test code** can use `.unwrap()` and `.expect()` - the architecture guidelines about these apply only to production code
- **Networking DNS parsing** is in `packet/parse_impl.rs` (packet module), not `networking/` module

## Implementation Plan

The consolidated implementation plan is in `plans/plan.md`. It contains 24 items across 3 waves:

| Wave | Items | Priority | Status |
|------|-------|----------|--------|
| Wave 1 | 6 | High | COMPLETED (2026-05-29) |
| Wave 2 | 8 | Medium | COMPLETED (2026-05-29) |
| Wave 3 | 10 | Low | COMPLETED - 6/10 implemented, 4 deferred |

---

## Recently Implemented Fixes (2026-05-29)

### Wave 1 (All Completed)
- PluginManager: HashMap → FxHashMap (`slapper-plugin/src/lib.rs`)
- Ruby timeout: Clarified parameter usage (`slapper-ruby/src/bridge.rs`)
- CMS scanner: unwrap_or_default → explicit error handling
- AI CacheKeyBuilder: Colon → null byte separator
- AI Agents: All HashMaps → FxHashMap (verified already correct)
- NSE CVE-2024-27956: Vec-based storage for multiple entries

### Wave 2 (All Completed)
- WAF HTTP/2 smuggling: Documented limitation
- Scope.validate(): Added validation method
- Scanner progress bar: Added catch_unwind wrapper
- Distributed TaskResult: Implemented collection
- Distributed heartbeat: Uses resolve_cached()
- Pipeline session save: Errors propagated via checkpoint_error
- Pipeline fingerprint: Uses EXTENDED_SCAN_PORTS constant
- Pipeline WAF: Uses self.common for TLS settings

### Wave 3 (6/10 Completed, 4 Deferred)
- TUI dispatcher: Cached to reduce 4 calls to 1
- CLI output flag: Added `-o`/`--output` to commands
- Docs module counts: Updated recon.md (17 modules)
- Fuzzer progress bar: Added to run_sequential_with_session
- CSV streaming: Async BufWriter implementation
- TUI theme restore: Implemented in restore_session
- Recon secrets: Now invoked in pipeline
- Deferred: TUI unwrap_or_default, NSE DNS rebinding, NSE OSV/CISA KE |

---

## Verified as Already Fixed (Reference)

These items were verified during architecture review and do NOT need implementation:

| Item | Evidence |
|------|----------|
| Loadtest error list cap 1000 | `metrics.rs:101,109` uses 1000 |
| Cloud parallelization | `cloud/mod.rs:66` uses `tokio::join!` |
| Distributed queue lock acquisition | `queue.rs:78-79` acquires both locks upfront |
| Fuzzer LazyLock per-type init | `payloads/mod.rs:140-150` uses LazyLock correctly |
| WAF HEADER_VALUE_MAX_LEN | `waf/detector/detect.rs:10` at module level |
| Config private IP check | `scope.rs:226,280` uses `is_private_ip()` |
| Fuzzer rate < 1 | `execution.rs:267` uses `rate < 1` |
| NSE library count 164 | Correct count (no discrepancy) |

---

## Key Implementation Patterns (2026-05-29)

### Division by zero guard
```rust
// Always check before division
if self.stages.is_empty() {
    return 0.0;
}
```

### Scroll offset bounds
```rust
// Check empty before calculating offset
if self.lines.is_empty() {
    return 0;
}
```

### Arc::try_unwrap error handling
```rust
// Use map_err instead of expect()
Arc::try_unwrap(arc).map_err(|_| MyError::TooManyOwners)?
```

### LazyLock regex initialization
```rust
// Use unwrap_or_else for descriptive panic
static REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(pattern).unwrap_or_else(|e| panic!("Invalid regex: {}", e))
});
```

### Error handling pattern
```rust
// Instead of unwrap_or_default()
let body = match response.text().await {
    Ok(text) => text,
    Err(e) => {
        tracing::debug!("Failed to read response body: {}", e);
        String::new()
    }
};
```

### Plugin/NSE Module
- PluginManager still uses std `HashMap` at `lib.rs:296-297` - needs FxHashMap conversion
- CVE-2024-27956 appears twice in vulns.rs (AutomateWoo and WooCommerce) but HashMap only stores one
- Socket sandbox DNS rebinding protection should be verified

### AI Module
- `CacheKeyBuilder` uses `:` separator - if payload contains colon, cache keys may collide
- Three AI Agents files still use std HashMap: `alerts/mod.rs`, `constraints/checker.rs`, `portfolio.rs`
- `SmartWafBypass` knowledge base eviction may incorrectly wipe all failures on size limit

---

## Verification Commands

```bash
cargo check --lib -p slapper
cargo check --lib -p slapper-plugin
cargo check --lib -p slapper-ruby
cargo check -p slapper-nse
cargo test --lib -p slapper
cargo test --test negative_tests -p slapper
cargo test --test scanner_tests -p slapper
cargo clippy --lib -p slapper
cargo clippy --lib -p slapper-plugin
cargo clippy --lib -p slapper-ruby
```

(End file - ends around line 290)