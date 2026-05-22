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
| `loadtest/` | `crates/slapper/src/loadtest/` (uses FxHashMap, hdrhistogram) |

### Feature Flags

- `stress-testing` - Raw sockets, IP spoofing
- `packet-inspection` - Packet capture
- `python-plugins` / `ruby-plugins` - Plugin language support
- `rest-api` / `grpc-api` - API server integration
- `nse` - Nmap NSE script support
- `ai-integration` - AI planner, script generator, autonomous agent skills
- `ws-api` - WebSocket pub/sub
- `full` - All features combined

### Key Types

- `SlapperConfig` - Main configuration (`config::load_config()`)
- `Severity` - Unified severity (in `types.rs`, re-exported everywhere)
- `TabError` - Structured error type with categories (Network, Auth, Config, Resource, Target, Internal, Unknown) in `tui/app/tab_error.rs`
- `SensitiveString` - Zeroized credential wrapper
- `FuzzEngine` / `FuzzResult` - Fuzzing engine
- `PayloadType` - Enum of 30 payload categories
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
- **AI Module Override**: See `crates/slapper/src/ai/AGENTS.override.md` for AI-specific patterns
- **Hash Collections**: Use `rustc_hash::FxHashMap` and `rustc_hash::FxHashSet` instead of std collections for performance
- **Error Handling**: Avoid `unwrap_or_default()` on async operations; use explicit match with tracing instead

### Codebase Health

| Metric | Value |
|--------|-------|
| Tests | 1324 base, 1469+ with full features |
| Clippy | ~33 warnings (pre-existing, none in ai module) |
| Source files | 743 |
| Payload types | 30 |
| Tabs | 29 |

### Security Notes

- **Scope Enforcement**: Direct IP addresses (e.g., `127.0.0.1`) are now blocked via private IP checks in `TargetScope::parse()`. Previously they bypassed DNS resolution and private IP blocking.
- **TUI Settings Tab**: Only exposes a subset of config fields. Saving via the TUI will cause data loss for `profiles`, `schedule`, `remote`, `ai`, `search`, `alert_channels`, and other fields not shown in the UI.

### Recent Bug Fixes (2026-05-22)

| Component | Issue | Fix |
|-----------|-------|-----|
| `distributed/queue.rs:57` | `dequeue()` ignored `worker_id` and didn't set `assigned_at_secs` | Now tracks which worker owns task and when assigned |
| `distributed/worker.rs:132-161` | Heartbeat used HTTP POST to non-existent REST API | Changed to use `RemoteClient::send_heartbeat()` via TCP |
| `ai/waf_bypass.rs:107` | Loop missing `continue` caused incorrect fallthrough to AI query when entry had `failed_attempts < 3` | Added `continue` after `failed_attempts >= 3` check |
| `ai/planner.rs:456` | `ExecutionStage` has `name` field, not `target` | Changed to `s.name.to_lowercase().contains()` |
| `agent/alerts/routing.rs:81` | `expect()` on fallback HTTP client could panic | Propagate error via `?` instead |
| `agent/alerts/routing.rs:107-112` | Race condition in `cleanup_stale_entries` | Inline cleanup under single lock scope |
| `agent/memory.rs:137` | `unwrap()` on `file_stem()` could panic for hidden files | Added fallback hash-based name |
| `agent/mod.rs:657` | Silent error suppression with `unwrap_or_default()` | Log warning with `unwrap_or_else()` |
| `commands/handlers/auth_test.rs:10` | Missing scope validation for auth-test command | Added `ctx.ensure_scope_url(&args.target)?` |
| `commands/handlers/cluster.rs:348` | `unwrap_or(22)` in parse could panic | Changed to `unwrap_or_else(\|_\| 22)` |
| `commands/handlers/mod.rs:155-169` | Hardcoded command list in `handle_no_command` | Replaced with guidance to use `slapper --help` |
| `config/scope.rs:209-226` | Direct IP addresses bypassed private IP checks | Added loopback and private IP validation in `TargetScope::parse()` |
| `config/api.rs:8` | `maxmind.data_dir` used wrong qualifier | Changed to use `PROJECT_QUALIFIER` consistently |
| `fuzzer/engine/execution.rs:75-79` | Unused `_update_session` parameter in `run_concurrent_inner` | Removed parameter; refactored callers |
| `fuzzer/detection/analyzer.rs:168,206` | `unwrap_or(Ordering::Equal)` on f64 `partial_cmp` could panic on NaN | Added explicit NaN handling with `is_nan()` checks |
| `fuzzer/api_schema/mod.rs:310` | `unwrap_or_default()` silenced body read errors | Changed to explicit match with tracing debug |
| `fuzzer/engine/utils.rs:249` | WAF status codes (403, 406, 429) hardcoded | Extracted to `WAF_BLOCKED_STATUS_CODES` constant |
| `fuzzer/engine/types.rs:176` | `BaselineResponse.headers` used `std::collections::HashMap` | Changed to `FxHashMap` for performance |
| `fuzzer/redos_detect.rs:276` | `PayloadReDosChecker.vulnerable_payloads` used `HashMap` | Changed to `FxHashMap` for performance |
| `loadtest/runner.rs:327-337` | Non-success HTTP response bodies not consumed, leaving connection pool in inconsistent state | Now consumes response body for non-success before recording metrics |
| `loadtest/runner.rs:300-307` | Rate limiting interval calculation could drift due to using `next + interval` instead of `now + interval` | Changed to compute `next = now_after_sleep + interval` to maintain accurate rate |

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
- `.opencode/skills/slapper-recon/` - Recon module workflows
- `.opencode/skills/slapper-scanner/` - Scanner module workflows
- `.opencode/skills/slapper-security/` - Security testing skill workflows
- `.opencode/skills/slapper-stress/` - Stress module workflows
- `.opencode/skills/slapper-packet/` - Packet capture/crafting/parsing workflows
- `.opencode/skills/slapper-loadtest/` - Loadtest module workflows
- `.opencode/skills/slapper-tool/` - Tool module workflows
- `.opencode/skills/slapper-tui/` - TUI module workflows
- `.opencode/skills/slapper-waf/` - WAF module workflows
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
