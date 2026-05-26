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

### Architecture Index

Use these sections as the canonical reference points when updating guidance or skills:

- `architecture/tui.md` - TUI event loop, key handling, overlays, tab routing, session persistence, and quick switch behavior
- `architecture/config.md` - config loading, scope enforcement, and TUI settings save semantics
- `architecture/output.md` - report formatting, exports, and rendering integration

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
- **TUI Settings Tab**: The settings editor applies exposed fields on top of an existing config and preserves non-exposed sections such as `profiles`, `schedule`, `remote`, `ai`, `search`, and `alert_channels`. See `architecture/config.md` for the current save semantics.

### Key Patterns

- **Division by zero guard**: Always check `if self.stages.is_empty()` before division
- **Scroll offset bounds**: Use `self.lines.is_empty()` check before calculating scroll_offset
- **Option checkbox array bounds**: Use `.get()` with fallback when accessing checkbox arrays by index
- **Arc::try_unwrap**: Use `map_err` instead of `.expect()` to avoid panic
- **LazyLock regex**: Use `.expect()` with descriptive message instead of `.unwrap()`
- **FxHashMap/FxHashSet**: Always use for performance in new code
- **Vec removal in loop**: Use `swap_remove` instead of `remove` when order doesn't matter - `swap_remove` is O(1) vs `remove` which is O(n)

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

The implementation plan is in `plans/plan.md`. All 20 implementation items have been completed and verified. See the plan for details on completed items and future considerations.

## Current Focus

The codebase is in a healthy state with all major planned fixes implemented. Ongoing work includes:
- Maintaining and improving existing modules
- Adding new security testing capabilities
- Documentation updates as needed

### TUI Bug Fixes (2026-05-26)

- **Bounds check in scan_endpoints.rs render()**: Fixed `scan_endpoints.rs:294-299` to use `input_chunks.get(i)` pattern instead of direct indexing `input_chunks[i]` which could panic if chunks < fields. Also fixed `input_chunks.get(4)` for the checkbox render.
- **Bounds check in fingerprint.rs render()**: Fixed `fingerprint.rs:249-251` to use `input_chunks.get(i)` pattern instead of direct indexing.
- **Bounds check in waf_stress.rs render()**: Fixed `waf_stress.rs:186-188` to use `input_chunks.get(i)` pattern instead of direct indexing.
- **Bounds check in packet.rs render()**: Fixed `packet.rs:596-598` to use `input_chunks.get(i)` pattern instead of direct indexing.
- **to_lowercase() redundancy in input.rs**: Fixed `input.rs:96` to cache `self.value.to_lowercase()` before the filter closure instead of calling it repeatedly for each completion candidate.
- **to_lowercase() redundancy in security.rs**: Fixed `security.rs:122-127` to cache `target.to_lowercase()` before the three `contains()` checks.
- **Worker success log level in network.rs**: Changed `network.rs:172` from `tracing::debug!` to `tracing::info!` for packet capture task completion.

### Fuzzer Bug Fixes (2026-05-26)

- **Division by zero in filters.rs**: Fixed `filters.rs:138,146,154,162` to use `result.response_length.unwrap_or(1).saturating_sub(1)` instead of `unwrap_or(0)` to prevent division by zero when calculating word/line counts from response length.

### TUI Bug Fixes (2026-05-29)

- **popup.rs render() bounds**: Fixed `popup.rs:129-167` to use `if let Some(chunk) = chunks.get(0)` and `if let Some(button_area) = chunks.get(1)` instead of direct indexing.
- **api.rs double map_err**: Fixed `api.rs:339` - removed duplicate `??` after `.map_err()` which caused unreachable error handling code.
- **recon.rs division guard**: Fixed `recon.rs:133` - added `total_stages.max(1)` guard for progress calculation.
- **Bounds check for checkbox arrays**: Added bounds check in `waf.rs:519` for `technique_checkboxes` access to prevent panic. The waf tab now properly guards against out-of-bounds index when toggling technique checkboxes, matching the pattern used in `recon.rs:588-590`.
- **Slice bounds for InputGroup fields**: Fixed `integrations.rs:329-338` to use `.get()` with fallback for slicing `issue_inputs.fields` instead of direct slice syntax `fields[..4]` which could panic if fewer than 4 fields exist.
- **Bounds check in hunt.rs get_config()**: Fixed `hunt.rs:89-93` to use `.get(index).map(|cb| cb.checked).unwrap_or(false)` for `option_checkboxes` access instead of direct indexing.
- **Bounds check in browser.rs get_config()**: Fixed `browser.rs:87-89` to use `.get(index).map(|cb| cb.checked).unwrap_or(false)` for `option_checkboxes` access instead of direct indexing.
- **Mutable bounds check in waf.rs reset()**: Fixed `waf.rs:311-316` to use `.get_mut()` for mutable access when setting default checkbox states in reset(), since `.get()` returns `&` reference which cannot be assigned to.
- **Bounds check in vuln.rs edge detection**: Fixed `vuln.rs:602,613` to use `.first().map(...).unwrap_or(true)` pattern for `is_at_left_edge()` and `is_at_right_edge()` instead of direct `fields[0]` indexing which could panic if fields is empty.
- **ScrollableText scroll_down empty lines**: Fixed `scrollable.rs:57-59` to handle empty lines case explicitly. Previously `saturating_sub(1)` on empty len would result in `usize::MAX`, causing incorrect scroll offset.
- **Worker error logging levels**: Fixed `api.rs:57,134` to use `tracing::warn!` instead of `tracing::debug!` for GraphQL request failures. Operational errors should be logged at warn level for proper visibility.
- **Worker error logging in security.rs**: Fixed `security.rs:227,235` to use `tracing::warn!` for finding list operation failures.
- **Bounds check in load.rs reset()**: Fixed `load.rs:367-374` to use `if self.inputs.fields.len() > 5` before direct field access to fields[1-4].
- **Bounds check in fuzz.rs reset()**: Fixed `fuzz.rs:404-413` to use `if self.inputs.fields.len() > 6` before direct field access to fields[1,3-6].
- **Bounds check in scan.rs render()**: Fixed `scan.rs:306-307` to use `if self.inputs.fields.len() >= 2` before direct field access to fields[0-1].
- **Bounds check in input.rs can_move helpers**: Fixed `input.rs:680-694` to add `idx < self.fields.len()` check in `can_move_left()` and `can_move_right()`.
- **Vec::swap_remove exception**: VecDeque does not have `swap_remove` - use `remove` for VecDeque or when the collection type is not Vec (`history.rs:145` uses VecDeque).

### Additional TUI Bug Fixes (2026-05-25 Session)

- **Tokio spawn error handling in network.rs:159-170**: Replaced double-unwrap pattern with proper `match` on `handle_result` (Timeout, JoinError, Ok). Added `tracing::warn!` for task failures and `tracing::debug!` for success.
- **Tokio spawn error handling in recon.rs:176-215**: Added `progress_handle.await` checks with `is_panic()` detection in all match arms after `progress_handle.abort()`.
- **Worker error logging in history.rs:58**: Changed `tracing::debug!` to `tracing::warn!` for history export serialization failures.
- **Bounds check in fuzz.rs:128-134**: Replaced `.expect()` with `if let Some(s) = ...` pattern + warn logging.
- **Bounds check in fuzz.rs:471-473**: Added `if self.inputs.fields.len() > 2` guard before rendering fields[0-2].
- **Bounds check in scan_ports.rs:167-192**: Added `is_empty()` and `len() < 2` guards + `.get(i)` for chunks access.
- **Bounds check in scan_endpoints.rs, fingerprint.rs, waf_stress.rs, packet.rs**: Added `if len > N` guards in reset() methods.
- **Bounds check in settings/main.rs:267-325, 431-446**: Replaced direct field indexing with `.get().map().unwrap_or()` pattern.
- **Bounds check in workflow.rs:332 and vuln.rs:420**: Added `if idx < self.inputs.fields.len()` guard.
- **Duplicate import in integrations.rs:3**: Removed duplicate `use crate::tc;`.
- **Redundant to_lowercase() calls**: Fixed `security.rs:115-121` and `history.rs:168-186` to cache lowercase values.

### Plugin/NSE Module

- PluginManager uses `FxHashMap` at `lib.rs:296-297`
- CVE-2024-27956 uses Vec-based storage for multiple entries per CVE
- Socket sandbox DNS rebinding protection is validated at connection time

### AI Module

- `CacheKeyBuilder` uses null byte separator (`\x00`) - no colon collision risk
- AI Agents files use `FxHashMap`: `alerts/mod.rs`, `constraints/checker.rs`, `portfolio.rs`
- `SmartWafBypass` knowledge base eviction sorts by failed_attempts and last_accessed (fixed)

### AI Module Fixes (2026-05-29)

- **cache.rs serialization losing entries**: Fixed `cache.rs:122-130` - `From<AiCache> for AiCacheSerialized` now correctly copies entries instead of using empty `FxHashMap::default()`. Cache entries are now properly persisted to disk.
- **planner.rs cache key collision**: Fixed `planner.rs:63-71` - `request_cache_key()` now sanitizes input by removing null bytes to prevent collisions when goal/target contain colons.

### TUI Components Fixes (2026-05-29)

- **popup.rs render() bounds**: Fixed `popup.rs:129-167` to use `if let Some(chunk) = chunks.get(0)` and `if let Some(button_area) = chunks.get(1)` instead of direct indexing `chunks[0]` and `chunks[1]`.

### TUI Workers Fixes (2026-05-29)

- **api.rs double map_err**: Fixed `api.rs:339` - removed duplicate `??` after `.map_err()` which caused unreachable error handling code. Now uses single `?` properly.
- **recon.rs division guard**: Fixed `recon.rs:133` - added `total_stages.max(1)` guard for progress calculation to prevent division by zero if stages collection were empty.

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
