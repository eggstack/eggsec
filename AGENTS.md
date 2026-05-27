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

### TUI Bug Fixes (2026-05-30)

- **Integer underflow in fingerprint.rs:292**: Fixed `handle_focus_prev()` to check `is_empty()` before `fields.len() - 1` to prevent panic when fields is empty.
- **Integer underflow in scan_endpoints.rs:334**: Fixed `handle_focus_prev()` to use `focus_prev()` with `is_empty()` guard instead of direct subtraction.
- **Bounds check in fuzz.rs render():485-497**: Added `config_chunks.len() >= 7` guard before accessing `config_chunks[3-6]`.
- **Bounds check in fuzz.rs render_overlays():583-589**: Added `config_chunks.len() >= 6` guard and `.get()` pattern for dropdown info access.
- **Bounds check in plugin.rs:251**: Added `input_chunks.first()` check before accessing `input_chunks[0]`.
- **Bounds check in graphql.rs:297-300**: Added `options_chunks.len() >= 4` guard for checkbox renders.
- **Bounds check in oauth.rs:342-345**: Added `options_chunks.len() >= 4` guard for checkbox renders.
- **Logic error in nse.rs:396-398**: Fixed `is_at_left_edge()` to use `== 0` instead of `<=` for left edge detection.
- **Redundant identity map in integrations.rs:334**: Removed `.map(|s| s)` identity map.
- **Inconsistent bounds in workflow.rs:326,330**: Added `field_chunks.get(i)` bounds check for `idx==5` and `idx==6` branches.
- **Dead code in scan_ports.rs:167-171**: Moved `is_empty()` check outside loop - was unreachable since targets() returns non-empty for non-empty input.
- **stress.rs:195-206**: Fixed missing bounds check in reset() - added individual `if len > N` guards for fields[1-3].
- **scan_ports.rs:172-186**: Fixed validation to check ALL targets (not just first) and validate port range per target.
- **waf.rs:598-606**: Fixed `is_at_right_edge()` for empty checkboxes - added `is_empty()` guard + `saturating_sub(1)`.
- **waf.rs:588-596**: Fixed `is_at_left_edge()` for empty checkboxes - added `is_empty()` guard.
- **fuzz.rs:128-134**: Refactored redundant `match` to `let...else` syntax for session None check.
- **vuln.rs:419-423**: Fixed `field_chunks[i]` bounds - use `if let Some(chunk) = field_chunks.get(i)`.
- **recon.rs:677-687**: Fixed `is_at_right_edge()` for Options - added `is_empty()` guard + `saturating_sub(1)`.
- **oauth.rs:400-404**: Added `!self.is_running()` guard to `handle_backspace()`.

### TUI Bug Fixes (2026-05-31 - Deep Dive Session)

Fixed direct array access patterns without bounds checks:

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `compliance.rs` | 232 | `input_chunks[2]` direct access | Wrapped in `if let Some(framework_area) = input_chunks.get(2)` |
| `recon.rs` | 404 | `input_chunks[2]` direct access | Wrapped in `let Some(options_area) = input_chunks.get(2) else { return; }` |
| `browser.rs` | 247 | `input_chunks[2]` direct access | Wrapped in `let Some(cb_area) = input_chunks.get(2) else { return; }` |
| `hunt.rs` | 282 | `input_chunks[3]` direct access | Wrapped in `let Some(cb_area) = input_chunks.get(3) else { return; }` |

Added missing `!self.is_running()` guards on input handlers:

| File | Lines | Handlers |
|------|-------|-----------|
| `packet.rs` | 675-695 | handle_char, handle_backspace, handle_paste |
| `cluster.rs` | 327-358 | handle_char, handle_backspace, handle_paste |
| `proxy.rs` | 508-528 | handle_char, handle_backspace, handle_paste |
| `nse.rs` | 246-262 | handle_char, handle_backspace, handle_paste |
| `plugin.rs` | 301-317 | handle_char, handle_backspace, handle_paste |
| `report.rs` | 368-399 | handle_char, handle_backspace, handle_paste |

Added `is_empty()` guards to `is_at_left_edge()`/`is_at_right_edge()` for selectors:

| File | Lines | Fix |
|------|-------|-----|
| `nse.rs` | 393-409 | Added `self.script_selector.items.is_empty() ||` before selector checks |
| `storage.rs` | 575-593 | Added `self.mode_selector.items.is_empty() ||` before selector checks |
| `integrations.rs` | 553-572 | Added `self.tracker_selector.items.is_empty() ||` before selector checks |

Fixed silent error suppression on `selector.confirm()`:

| File | Line | Fix |
|------|-------|-----|
| `report.rs` | 461 | `if self.view_selector.confirm().is_none() { tracing::warn!(...) }` |
| `cluster.rs` | 436 | Same pattern |
| `packet.rs` | 749 | Same pattern |
| `load.rs` | 568 | Same pattern |
| `settings/input.rs` | 189, 213, 224 | Same pattern for proxy_rotation, severity, accent_color selectors |

Fixed session.rs silent errors:

| File | Line | Fix |
|------|-------|-----|
| `session.rs` | 109 | Changed `filter_map(|e| e.ok())` to explicit match with `tracing::debug!` |
| `session.rs` | 176 | Changed `let _ = fs::remove_file(...)` to `if let Err(e) = ... { tracing::warn!(...) }` |

Fixed redundant `to_lowercase()` in dashboard.rs:195-208 - combined into single fold() iteration.

### Pre-existing Test Fix (2026-05-31)

- **key_handler.rs:440-457**: Fixed `test_quick_switch_clamps_selection_after_filter_input` by making `clamp_quick_switch_selection()` re-fetch fresh `get_quick_switch_results()` instead of using stale results passed as parameter.

### Scanner Module Fixes (2026-05-30)

- **Silent error suppression in scanner/ports/mod.rs:582**: Changed `let _ = tx.send(...)` to proper error check with debug logging.
- **Silent error suppression in scanner/ports/spoofed.rs:450**: Changed `let _ = tx.send(...)` to proper error check with debug logging.
- **Silent error suppression in scanner/fingerprint.rs:306**: Same fix for progress channel send.
- **Silent error suppression in scanner/endpoints.rs:827**: Same fix for progress channel send.

### TUI is_running() Guards (2026-05-30 Continuation Session)

All 29 tabs now properly guard input handlers (`handle_char`, `handle_backspace`, `handle_paste`) with `!self.is_running()`:

| Tab | Status |
|-----|--------|
| stress.rs | ✅ All 3 handlers fixed |
| compliance.rs | ✅ All 3 handlers fixed |
| storage.rs | ✅ All 3 handlers fixed |
| integrations.rs | ✅ All 3 handlers fixed |
| workflow.rs | ✅ All 3 handlers fixed |
| vuln.rs | ✅ All 3 handlers fixed |
| oauth.rs | ✅ handle_char fixed |
| auth.rs | ✅ All 3 handlers fixed |
| cluster.rs | ✅ All 3 handlers fixed |
| graphql.rs | ✅ handle_char fixed |

### Core Tool Implementation Fixes (2026-05-30)

- **tool/implementations/fuzzer.rs:175-182**: Fixed `Arc::try_unwrap()` from `.expect()` panic to graceful fallback with `match`
- **tool/implementations/recon.rs:145-152**: Same fix - gracefully handles concurrent callback references
- **tool/implementations/scanner.rs:184-191**: Same fix
- **ai/cache.rs:160-183**: Fixed race condition in `with_persistence()` - now merges entries properly with async block

### Additional Fixes (2026-05-30 Continuation)

- **scanner/udp_fingerprint.rs:201-204**: Added timeout to UDP probe send operation
- **stress.rs:195-206**: Fixed bounds check from `>3` to `>1` for fields[1]
- **load.rs:367-376**: Fixed bounds check from `>5` to `>=5` for fields[4]
- **recon.rs:309-318**: Removed dead code path `visible_rows == 0` from `options_window_start()`

### Proxy Module Fixes (2026-05-30)

- **Silent error suppression in proxy/health.rs:158-162**: Changed `filter_map(|r| r.ok())` to explicit `match` with `is_panic()` detection and warn logging.

### TUI Bug Fixes (2026-05-26 Session)

- **Silent error suppression in scan.rs:518,528**: Changed `let _ = selector.confirm()` to proper error check with `is_none()` and warn logging.
- **Dead code in scan_ports.rs:166-171**: Removed unreachable `is_empty()` check - fields are guaranteed non-empty after InputGroup construction.
- **Silent error suppression in fuzz.rs:713,722,731**: Changed `let _ = selector.confirm()` to proper error check.
- **Options edge detection in graphql.rs:490-502**: Added explicit `GraphQlFocusArea::Options` handling with `is_empty()` guard - was swallowed by `_ => true`.
- **handle_paste guard in oauth.rs:406-409**: Added missing `!self.is_running()` guard to match `handle_backspace()`.
- **Options edge detection in oauth.rs:534-546**: Added explicit `OAuthFocusArea::Options` handling with `is_empty()` guard.
- **One-directional bounds in report.rs:299-303**: Changed to `if let Some(chunk) = input_chunks.get(i)` pattern.
- **to_lowercase redundancy in history.rs:194-206**: Pre-compute `details_lower` vector before filter closure to avoid calling `to_lowercase()` per detail per entry.
- **Missing is_empty guard in hunt.rs:514,524**: Added `is_empty()` guard to `is_at_left_edge()` and `is_at_right_edge()`.
- **Missing is_empty guard in browser.rs:473,483**: Same fix as hunt.rs.
- **Missing is_empty guard in compliance.rs:417,428**: Added `framework_selector.items.is_empty()` guard.
- **Inconsistent guard in scan_endpoints.rs:333**: Added same `!fields.is_empty()` guard to `handle_focus_next()` for consistency.
- **FormBuilder bounds in input.rs:784-788**: Changed direct `chunks[i]` indexing to `if let Some(chunk) = chunks.get(i)` pattern.

### TUI Components Fixes (2026-05-26 Session)

- **scrollable.rs:150**: Fixed use of unbounded `self.scroll_offset` - now uses bounded `scroll_offset` variable calculated with explicit empty lines check.
- **selector.rs:247**: Added `!self.items.is_empty()` guard to `handle_left()` for consistency with `handle_right()`.

---

## Current Focus

The codebase is in a healthy state with all major planned fixes implemented. Ongoing work includes:
- Maintaining and improving existing modules
- Adding new security testing capabilities
- Documentation updates as needed

### TUI Bug Fixes (2026-05-26 Evening Session)

Fixed bounds/edge detection issues across multiple files:

| File | Lines | Issue | Fix |
|------|-------|-------|-----|
| `plugin.rs` | 419-435 | Missing `is_empty()` guards on `PluginSelector` edge detection | Added `self.plugin_selector.items.is_empty() \|\|` before selector checks |
| `input.rs` | 684-698 | `can_move_left/right()` missing `is_empty()` guard | Wrapped in `if !self.fields.is_empty()` check |
| `key_handler.rs` | 48 | `Ctrl+x` (quick switch) missing `is_running()` guard | Added `if !app.has_active_task()` guard |

### Non-TUI Module Fixes (2026-05-26 Evening Session)

Fixed silent error suppression and logging level issues:

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `tool/protocol/rest.rs` | 260 | Silent WS channel send | Changed to `if let Err(e) = tx.send(...).await` with `tracing::warn!` |
| `tool/agents/lifecycle.rs` | 341 | Silent event send | Same fix pattern |
| `distributed/remote.rs` | 116 | Silent shutdown send | Same fix pattern |
| `scanner/ports/mod.rs` | 580 | `debug!` instead of `warn!` for progress dropped | Changed to `tracing::warn!` |
| `scanner/fingerprint.rs` | 306 | Same logging level issue | Changed to `tracing::warn!` |
| `scanner/endpoints.rs` | 828 | Same logging level issue | Changed to `tracing::warn!` |
| `scanner/ports/spoofed.rs` | 451 | Same logging level issue | Changed to `tracing::warn!` |
| `scanner/templates/marketplace.rs` | 208-209 | Silent `filter_map(\|e\| e.ok())` | Changed to explicit match with `tracing::debug!` |
| `recon/git_secrets.rs` | 287 | Same silent filter_map issue | Same fix pattern |

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

### TUI Bug Fixes (2026-05-29 Evening)

- **Bounds check in scan_ports.rs:333**: Fixed `input_chunks[4]` direct indexing to use `.get(4)` for UDP checkbox render.
- **Bounds check in recon.rs:399**: Fixed field render loop to use `input_chunks.get(i)` pattern.
- **Bounds check in fuzz.rs:477**: Added `config_chunks.len() >= 3` check before accessing config_chunks[0-2].
- **Bounds check in hunt.rs:277**: Fixed field render loop to use `input_chunks.get(i)` pattern.
- **Bounds check in browser.rs:242**: Fixed field render loop to use `input_chunks.get(i)` pattern.
- **Bounds check in storage.rs:320,337**: Fixed `config_chunks[i+1]` and `query_chunks[i+1]` to use `.get(i+1)` pattern.
- **Bounds check in integrations.rs:344**: Fixed `field_chunks[i]` to use `.get(i)` pattern.
- **Bounds check in workflow.rs:333**: Fixed `field_chunks[i]` to use `.get(i)` pattern (only fields[idx] was checked).
- **to_lowercase() redundancy in dashboard.rs:198-206**: Cached `e.summary.to_lowercase()` per entry instead of calling twice.
- **to_lowercase() redundancy in history.rs:197-202**: Cached lowercased fields per entry in search().
- **to_lowercase() redundancy in app/mod.rs:696-698**: Cached tab title/stable_id/description lowercase in get_quick_switch_results().
- **to_lowercase() redundancy in components/input.rs:97**: Cached `s.to_lowercase()` per candidate instead of calling twice.
- **Division guard in scan.rs:259**: Added `.max(1)` guard for progress calculation.
- **Identity map in integrations.rs:331**: Removed redundant `.map(|s| s)`.
- **Empty if body in workers/security.rs:121**: Fixed empty if block to push `Severity::Info` finding for no-cache/no-store directives.
- **Task error logging in app/task_runtime.rs:74**: Changed from silent `let _ = error_tx.send()` to proper error check with warn logging.
- **Unreachable code pattern in app/task_runtime.rs:68-79**: Refactored from `match runner.run()` with empty Ok arm to `if let Err(e)` pattern.
- **Log level in app/state_update.rs:68**: Changed from `tracing::debug!` to `tracing::warn!` for unhandled TaskResult variants.

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

### Load Tab Fixes (2026-05-26 Evening Session)

#### TUI Tab Edge Detection is_empty() Guards

| File | Lines | Selector/Field |
|------|-------|----------------|
| `load.rs` | 652-665, 667-681 | `test_type_selector` (is_empty guards added) |

#### load.rs update_progress validation

- **load.rs:321-324**: Fixed validation in `update_progress()` - added `completed.min(total)` and `total.max(1)` guards to prevent invalid progress values.

#### network.rs Worker Timeout Wrappers (2026-05-26 Evening)

- **network.rs:9-46**: Added timeout wrapper (300s) to `run_load_test()` - `tokio::time::timeout()` with proper error handling
- **network.rs:85-98**: Added timeout wrapper (600s) to `run_stress_test()` - `tokio::time::timeout()` with proper error handling
- **network.rs:22-37**: Added initial progress send `(0, requests)` at start of load test
- **network.rs:87-96**: Restructured error handling to convert `SlapperError` to `anyhow::Error` for compatibility

### Deep Dive Session Fixes (2026-05-31 Evening)

#### TUI Tab Edge Detection is_empty() Guards

| File | Lines | Selector/Field |
|------|-------|----------------|
| `stress.rs` | 455, 464 | `type_selector` |
| `workflow.rs` | 524, 533 | `mode_selector` |
| `packet.rs` | 840, 855 | `view_selector` |
| `proxy.rs` | 649, 663 | `view_selector` |
| `cluster.rs` | 552, 570 | `view_selector` |
| `scan_ports.rs` | 491, 500 | InputGroup delegation |
| `scan_endpoints.rs` | 458, 467 | InputGroup delegation |
| `fingerprint.rs` | 405, 414 | InputGroup delegation |

#### settings/input.rs is_running() Guards

All 8 input handlers now properly guard with `!self.is_running()`:
handle_char (36), handle_backspace (53), handle_paste (70), handle_enter (165), handle_up (269), handle_down (316), handle_left (364), handle_right (396)

#### history.rs is_running() Guard

- **history.rs:431**: Added `is_running()` guard to `handle_char` for hotkeys 'd' and 'C'

#### components/input.rs InputGroup Edge Guards

- **input.rs:668-682**: Fixed `is_at_left_edge()` and `is_at_right_edge()` to add `!self.fields.is_empty() && idx < self.fields.len()` guards

#### key_handler.rs Ctrl+V Guard

- **key_handler.rs:65-72**: Added `!app.has_active_task()` guard to Ctrl+V paste handler

#### Worker Silent Error Suppression (88 occurrences fixed)

| File | Count | Lines |
|------|-------|-------|
| `api.rs` | 15 | 22,63,140,146,159,160,200,220,231,245,292,293,310,341,351 |
| `security.rs` | 27 | 20,22,23,36,38,39,53,179,182,183,200,205-208,213,217,222,245-247,257-261,263-267,272-274,279,282,301,305,316-318,320-324,328-330,333-338,342,356,362-364,378,397,400-402,406 |
| `recon.rs` | 12 | 58,62,63,78,107,135,139,159,162,178,179,204 |
| `network.rs` | 13 | 23,24,68-74,124,137-139,176-181,226,244-246,335,344,348-353 |
| `plugin.rs` | 10 | 14,32,51,58,78,79,90,93,113,114 |
| `scanner.rs` | 9 | 14,19,36,37,51,80,96,113,114 |
| `fuzzer.rs` | 8 | 91,92,142,147,149,152,178,179 |

#### network.rs Log Level Fix

- **network.rs:172**: Changed `tracing::info!` to `tracing::debug!` for successful packet capture completion

### Additional Investigation Fixes (2026-05-26 Evening)

#### TUI Components Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `selector.rs` | 228 | Silent `let _ =` on confirm() | Changed to `if .is_none() { warn }` pattern |
| `palette.rs` | 60 | Direct array access | Changed to `.get()` with bounds check |

#### TUI session.rs Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `session.rs` | 113 | `debug!` instead of `warn!` | Changed to `tracing::warn!` |
| `session.rs` | 174 | Silent `filter_map(\|e\| e.ok())` | Changed to explicit match with warn |

#### tool/agents/lifecycle.rs Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `lifecycle.rs` | 337 | Silent `update_status` | Added `if let Err(e) = ...` with warn |
| `lifecycle.rs` | 381,416,429,434,447 | Silent `event_tx.send()` | Added `if let Err(e) = ...` with warn |

#### tool/protocol/mcp/routes.rs Fixes

| File | Lines | Issue | Fix |
|------|-------|-------|-----|
| `routes.rs` | 216-252 | Silent write/flush errors | Added `if let Err(e) = ...` with warn |

#### FxHashMap Performance Fixes

| File | Lines | Issue | Fix |
|------|-------|-------|-----|
| `orchestrator/mod.rs` | 21,50,84,89,302 | HashMap/HashSet | Changed to FxHashMap/FxHashSet |
| `tool/session.rs` | 288,316,461,465,1076 | HashMap | Changed to FxHashMap |
| `tool/state.rs` | 124,136 | HashMap | Changed to FxHashMap |
| `recon/mod.rs` | 221,253 | std HashMap | Changed to FxHashMap |

### Tab Bug Fixes (2026-05-26 Deep Dive)

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `graphql.rs` | 490-502 | Options edge detection missing | Added explicit Options case |
| `oauth.rs` | 534-546 | Options edge detection missing | Added explicit Options case |
| `report.rs` | 457 | Missing is_running guard | Added guard to handle_enter |
| `nse.rs` | 311 | Missing is_running guard | Added guard to handle_enter |
| `plugin.rs` | 356 | Missing is_running guard | Added guard to handle_enter |
| `vuln.rs` | 618-619 | Missing is_empty() guard | Added `items.is_empty() \|\|` guard |
| `workflow.rs` | 411 | Missing is_running guard | Added guard to handle_copy |
| `workflow.rs` | 257 | reset() doesn't clear mode | Added `current_mode` reset |
| `integrations.rs` | 280 | reset() doesn't clear selector | Added selector reset |
| `storage.rs` | 250 | reset() doesn't clear fields | Added fields.clear() loop |

## Bug Fixes (2026-06-01 Session)

### TUI Tab Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `vuln.rs` | 603-614 | `is_at_left_edge()` missing `is_empty()` guard | Added `items.is_empty() \|\|` guard for Mode selector |
| `nse.rs` | 311-343 | `handle_enter()` inverted guard logic | Removed early `return` when not running |
| `plugin.rs` | 356-369 | `handle_enter()` didn't stop/start properly | Rewrote to `if is_running() { stop } else { start }` |
| `proxy.rs` | 660-669 | `is_at_right_edge()` missing `is_open()` guard | Added `is_open()` check matching `is_at_left_edge()` |
| `proxy.rs` | 624-640 | `handle_left/right()` missing `is_open()` check | Added `is_open()` guard before selector methods |
| `load.rs` | 377 | `reset()` missing selector reset | Added `test_type_selector.select(0)` |
| `stress.rs` | 206 | `reset()` missing selector reset | Added `type_selector.select(0)` |
| `storage.rs` | 253 | `reset()` missing config_inputs clear | Added `for field in config_inputs.fields` loop |

### Tool Module Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `tool/agents/lifecycle.rs` | 178-181, etc. | SystemTime `unwrap()` panic risk | Changed to `unwrap_or_else(\|_\| Duration::from_secs(0))` |
| `tool/agents/lifecycle.rs` | 3, 73, 95, 171 | HashMap performance | Changed to `FxHashMap` |
| `tool/agents/communication.rs` | 7, 150, 164 | HashMap performance | Changed to `FxHashMap` |
| `tool/session.rs` | 519 | HashMap performance | Changed to `FxHashMap` for response headers |
| `tool/state.rs` | 216 | Silent file remove error | Changed to `if let Err(e) = ...` with debug logging |
| `tool/protocol/mcp/handlers/server.rs` | 758 | Silent session update error | Changed to `if let Err(e) = ...` with debug logging |

### Scanner Module Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `scanner/ports/spoofed.rs` | 281, 348, 384 | Silent packet send errors | Changed to check send result and log warning |

### AI Module Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `ai/cache.rs` | 277 | Silent directory creation error | Changed to `if let Err(e) = ...` with warn logging |
| `ai/planner.rs` | 473 | fallback_key cache key collision | Added `.replace('\x00)', "")` sanitization |

---

## Bug Fixes (2026-06-01 Session - Deep Dive)

### TUI Tab Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `settings/main.rs` | 311-347 | `apply_to_config()` unsafe direct field access | Changed to safe `.get()` pattern with bounds checks |
| `settings/main.rs` | 400,523,595 | Silent file write errors | Added `if let Err(e) = ...` with status_message |
| `report.rs` | 457-487 | `handle_enter()` returns early when not running | Restructured to allow selector interaction when idle |
| `nse.rs` | 311-340 | `handle_enter()` logic issue with Results + is_running | Restructured to properly handle blur/selector |
| `plugin.rs` | 356-388 | Missing `start()` method | Added `start()` method, restructured `handle_enter()` |
| `graphql.rs` | 415-432 | Missing `is_running()` guard on `handle_enter()` | Added `!self.is_running()` guard |
| `oauth.rs` | 459-476 | Missing `is_running()` guard on `handle_enter()` | Added `!self.is_running()` guard |
| `recon.rs` | 591-596 | Missing `is_running()` guard on Options toggle | Added `!self.is_running()` guard |
| `recon.rs` | 670-671 | Missing `is_empty()` guard on `is_at_left_edge()` | Added `self.option_checkboxes.is_empty() \|\|` |
| `stress.rs` | 263-267 | Direct array access `input_chunks[i]` | Changed to `.get(i)` pattern |
| `stress.rs` | 390-404 | `handle_enter()` uses `handle_enter()` result not captured | Changed to `confirm().is_none()` pattern |
| `storage.rs` | 339 | Direct array access `query_chunks[0]` | Changed to `.get(0)` pattern |
| `integrations.rs` | 335 | Suspicious fallback `&[]` in slice access | Changed to `&self.issue_inputs.fields` |
| `vuln.rs` | 495-505 | `handle_copy()` missing `is_running()` guard | Added `!self.is_running()` guard |
| `history.rs` | 441,443 | Empty handlers missing `is_running()` guards | Added `!self.is_running()` guards |
| `auth.rs` | 227-229 | `fields.len() - 1` underflow risk | Added `!self.inputs.fields.is_empty()` guard |

### TUI Component Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `scrollable.rs` | 99-106 | `is_at_left_edge/is_at_right_edge` inconsistent with empty lines | Added `is_empty()` guards to both methods |

### TUI Worker Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `plugin.rs` | 100 | Silent `discover_plugins()` call with `let _` | Changed to `if let Err(e) = ...` with debug logging |

### TUI App Module Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `task_runtime.rs` | 72-76 | Silent error suppression with `Err(_e)` | Changed to `if let Err(e) = ...` using actual error value |

### Tool Module Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `session.rs` | 525 | Silent error suppression `unwrap_or_default()` | Changed to `unwrap_or_else(\|e\| { warn!; String::new() })` |
| `session.rs` | 1016 | Silent error suppression `unwrap_or_default()` | Same fix pattern |
| `state.rs` | 217 | `debug!` instead of `warn!` for file removal | Changed to `tracing::warn!` |

### AI Module Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `cache.rs` | 278 | `debug!` instead of `warn!` for cache dir creation | Changed to `tracing::warn!` |

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

(End of file - total 527 lines)
