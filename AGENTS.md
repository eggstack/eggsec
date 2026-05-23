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

- **Scope Enforcement**: Direct IP addresses (e.g., `127.0.0.1`) are now blocked via private IP checks in `TargetScope::parse()`. Previously they bypassed DNS resolution and private IP blocking.
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
| `architecture/output.md` | Output & reporting module |
| `architecture/plugins_nse.md` | Plugin system (Python/Ruby) and NSE integration |
| `architecture/tui.md` | Terminal User Interface (TUI) module, 29 tabs, event loop, components |

---

## Recent Bug Fixes

### 2026-05-28 (Implementation Session)

| Component | Issue | Fix |
|-----------|-------|-----|
| NSE | `public_api/api.rs` - 8 std::HashMap instances | Replaced with FxHashMap for performance |
| Networking | `packet/parse_impl.rs:531,551` - DNS parsing bounds | Added `new_offset >= data.len()` check before byte access |
| Distributed | `worker.rs:115-123` - hardcoded capabilities | Created `worker_capabilities()` helper from TaskType enum |
| AI | `waf_bypass.rs:44` - silent knowledge base load failure | Changed to `unwrap_or_else()` with `tracing::warn` |
| NSE | `libraries/http.rs, datafiles.rs, creds.rs` - 4 more HashMap/HashSet | Replaced with FxHashMap/FxHashSet for performance |
| Distributed | `command.rs:146-149` - env field rejected without explanation | Added clarifying comment for intentional security rejection |
| Recon | 20 instances of `unwrap_or_default()` | Replaced with explicit match with `tracing::debug` across 12 files |
| Fuzzer | `analyzer.rs:188-190` - IQR division by zero | Added `if iqr_samples.is_empty()` check |
| Loadtest | `metrics.rs:76` - imprecise panic message | Changed to "Failed to create hdrhistogram" |
| Config | `settings.rs` - no AlertChannelsConfig validation | Added validation for all 4 channel types (Webhook, Email, Slack, PagerDuty) |
| Docs | `architecture/*.md` - outdated counts and notes | Updated TUI payload count (30→31), recon FxHashMap count (13→55), added DNS bounds note |

### 2026-05-28 (WAF Review)

| Component | Issue | Fix |
|-----------|-------|-----|
| `waf/mod.rs:4` | Docstring listed only 25 WAF products | Updated to "34 WAF products" |
| `waf/bypass/profiles.rs:21,37` | `get_waf_profiles()` recreated profiles every call | Changed to `LazyLock` static for caching |
| `waf/detector/detect.rs:45,71` | Score accumulator `u8` could overflow | Changed to `u16` with proper constant types |
| `constants.rs:69-90` | WAF scoring constants were `u8` | Changed to `u16` to prevent overflow |

### 2026-05-23

| Component | Issue | Fix |
|-----------|-------|-----|
| Distributed | `queue.rs:57` dequeue() error handling | Returns `Result<Option<Task>, QueueError>` instead of silently dropping errors |
| Distributed | `worker.rs:132-161` heartbeat | Uses `RemoteClient::send_heartbeat()` via TCP instead of HTTP POST |
| Recon | `geolocation.rs:308` CIDR mask | Fixed to `u32::MAX << (32 - prefix)` |
| Recon | `smtp_auth.rs:248,256,285` base64 API | Changed to `base64::engine::general_purpose::STANDARD.encode(...)` |
| Recon | `subdomain.rs:111,151` unwrap_or_default | Changed to explicit match with `tracing::debug` |
| Recon | `api_schema.rs:115` silent error | Changed to explicit match with `tracing::debug` |

---

## Implementation Notes

- **NSE module** (`slapper-nse/`) is a separate crate - use `cargo check -p slapper-nse` for validation
- **Test code** can use `.unwrap()` and `.expect()` - the architecture guidelines about these apply only to production code
- **Networking DNS parsing** is in `packet/parse_impl.rs` (packet module), not `networking/` module

## Implementation Plan

The consolidated implementation plan is in `plans/plan.md`. It contains 3 waves of work:

| Wave | Items | Description |
|------|-------|-------------|
| Wave 1 | 4 items | Production safety - NSE HashMap, DNS parsing, worker capabilities, AI knowledge base |
| Wave 2 | 5 items | Performance & correctness - NSE HashMap, distributed env, recon unwrap_or_default, fuzzer IQR, loadtest message |
| Wave 3 | 2 items | Documentation & polish - config validation, architecture doc updates |