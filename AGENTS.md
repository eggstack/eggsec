# AGENTS.md

Guidelines for AI agents working on this codebase.

## Project Overview

Slapper is a Rust-based security testing toolkit. See `README.md` for features and `ARCHITECTURE.md` for design details.

## Implementation Plan

**`plans/plan.md`** contains the consolidated implementation plan. Currently 148 items from architecture review are pending implementation across 6 waves.

| Status | Items |
|--------|-------|
| Completed (Waves 0-7) | Bug fixes, documentation corrections, new architecture docs |
| In Progress (Wave 1+) | 13 HIGH, 55 MEDIUM, 80 LOW priority items from architecture review |

## Quick Reference

### Build & Test Commands

```bash
cargo check --lib -p slapper
cargo check -p slapper-nse
cargo test --lib -p slapper
cargo test --test negative_tests -p slapper
cargo test --test scanner_tests -p slapper
cargo clippy --lib -p slapper
cargo build --release -p slapper
```

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
| `packet/` | `crates/slapper/src/packet/AGENTS.override.md` (uses pnet, pnet_packet for raw sockets) |
| `loadtest/` | `crates/slapper/src/loadtest/AGENTS.override.md` |
| `pipeline/` | `crates/slapper/src/pipeline/AGENTS.override.md` |
| `nse/` | `slapper-nse/AGENTS.override.md` (Lua VM, NSE libraries, sandbox, CVE integration) |
| `container/` | `crates/slapper/src/container/AGENTS.override.md` |

### Architecture Index

Use these sections as the canonical reference points when updating guidance or skills:

- `architecture/tui.md` - TUI event loop, key handling, overlays, tab routing, session persistence, and quick switch behavior
- `architecture/config.md` - config loading, scope enforcement, and TUI settings save semantics
- `architecture/output.md` - report formatting, exports, and rendering integration

### Feature Flags

- `stress-testing` - Raw sockets, IP spoofing
- `packet-inspection` - Packet capture
- `rest-api` / `grpc-api` - API server integration
- `nse` - Nmap NSE script support
- `nse-ssh2` - NSE with SSH2/libssh2 support
- `nse-sandbox` - Restrict dangerous Lua operations
- `ai-integration` - AI planner, script generation, autonomous agent skills
- `ws-api` - WebSocket pub/sub
- `api-schema` - API schema support (marker-only, no additional deps)
- `full` - All features combined (16 sub-features, does not include `grpc-api` or `ws-api`)

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
- `McpProfile` - MCP agent profile (`OpsAgent`, `CodingAgent`) in `tool/protocol/mcp/profile.rs`
- `McpProfilePolicy` - 18-field policy struct enforcing tool visibility and call restrictions per profile in `tool/protocol/mcp/policy.rs`
- `TargetPolicy` - Target scope enforcement policy in `tool/protocol/mcp/policy.rs`
- `CodingAgentFindingReport` - Typed output schema for coding-agent findings in `tool/protocol/mcp/coding_agent_output.rs`
- `ProbeIntent` / `ProbeRisk` - Probe classification in `probe.rs`

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
- **MCP Profile Policy**: Use `McpProfilePolicy` struct in `tool/protocol/mcp/policy.rs` to enforce tool visibility and call restrictions per profile

### Codebase Health

| Metric | Value |
|--------|-------|
| Tests | 1324 base, 1469+ with full features |
| Clippy | ~33 warnings (pre-existing, none in ai module) |
| Source files | 742 (.rs files in crates/) |
| Payload types | 30 |
| Tabs | 27 (28 with conditional feature tabs) |
| WAF products | 34 |
| NSE libraries | 169 |
| Modules | 39 |
| Output formats | 8 (Pretty, Json, Compact, Html, Csv, Sarif, Junit, Markdown) |
| CLI commands | ~29 base, ~40 with all features |

### Codebase Issues (Known Stub Implementations)

These modules exist but are stubs or have known limitations:

| Module | Issue | Reference |
|--------|-------|-----------|
| `storage/postgres.rs` | All CRUD methods return empty values - no SQLx integration | `plans/review_storage.md` |
| `vuln/mod.rs:37-40` | `VulnAssessment` only has `mode`, `results`, `assessed_at` - cannot hold structured findings | `plans/review_vuln.md` |
| `supply_chain/sbom.rs` | SBOM generators return empty `vulnerabilities` vectors - no CVE lookup | `plans/review_supply_chain.md` |

### Security Notes

- **Scope Enforcement**: Direct IP addresses (e.g., `127.0.0.1`) are blocked via private IP checks in `TargetScope::parse()`. However, scope rule evaluation happens AFTER private IP check - so targets like `10.255.255.255` are rejected even with scope rules like `allow 10.0.0.0/8`.
- **TUI Settings Tab**: The settings editor applies exposed fields on top of an existing config and preserves non-exposed sections such as `profiles`, `schedule`, `remote`, `ai`, `search`, and `alert_channels`. See `architecture/config.md` for the current save semantics.
- **MCP Coding Agent**: Default deny posture; stress/load/packet tools are hidden from coding-agent profile
- **Docker Shell Injection**: `container/docker.rs:208-209` uses unsanitized image names in `docker inspect` command - validate image names before passing to shell
- **Silent Error Suppression**: Multiple modules use `let _ =` pattern that silently ignores errors:
  - `notify/mod.rs:114,140-143,219-222,293-296` - notification failures
  - `loadtest/runner.rs:315` - semaphore acquire
  - `packet/capture.rs:209` - pcap write
  - `kubernetes.rs:65,104,163,195,254` - API errors (uses `.ok()`)

### Key Patterns (Lessons Learned)

- **TUI bounds checking**: Always use `.get(i)` pattern instead of direct `chunks[i]` indexing
- **TUI is_running() guards**: All input/navigation handlers must check `!self.is_running()` before processing
- **TUI reset() methods**: Must reset all state (selectors, checkboxes, fields, focus areas)
- **TUI edge detection**: `is_at_left_edge()`/`is_at_right_edge()` need `is_empty()` guards
- **Silent error suppression**: Never use `let _ =` or `filter_map(|e| e.ok())` - always log with tracing
- **Timeout wrappers**: All spawned tokio tasks should have timeout wrappers (30-300s depending on operation)
- **FxHashMap migration**: Replace `std::collections::HashMap` with `rustc_hash::FxHashMap` in performance-critical paths
- **Distributed results**: Workers must send `CommandMessage::Result` back to coordinator via channel
- **Verification before claims**: Always verify line numbers, file paths, and whether issues still exist before including in plans
- **File path conventions**: Use `commands/handlers/` not `cli/handlers/` - the latter directory does not exist
- **Dead code detection**: Check if `#![allow(dead_code)]` is at file top - many items flagged in reviews may already be resolved
- **Rate limiter patterns**: Use `tokio::time::sleep()` not spin loops; check if rate limiter is actually used (some are dead code)
- **Bounds check patterns**: Check for existing `if let Some(idx)` or `if len() > N` guards before claiming missing bounds checks
- **Wave plan verification**: When verifying plan claims, use subagents to check actual codebase state - plans may contain stale assertions that no longer match reality
- **Count verification**: Always verify statistical claims (file counts, enum variants, match arms) against actual source. Source file counts can vary by 200+ depending on whether nested crates are included
- **TUI stale detection**: TUI styling fixes may already be applied in a previous pass - always verify before re-implementing. Check actual `.rs` files, not just plan descriptions
- **PayloadType location**: `PayloadType` enum is in `fuzzer/payloads/mod.rs`, not `types.rs`. `types.rs` contains `OutputFormat`, `Severity`, etc.
- **Fabricated claims**: Always verify module/file existence before documenting dead code. The `auth/multi_protocol/` directory was claimed to exist but doesn't.
- **Proxy features exist**: `Tor` ProxyType and `Weighted`/`LowestLatency` rotation strategies already exist in code — verify before claiming they're missing.
- **Feature matrix math**: When verifying feature counts, sum the sub-counts to check for arithmetic errors (e.g., 18+12=30≠28). Correct counts: 16 features-with-deps + 12 marker-only = 28.

## Skills Directory

Skills are located in `.opencode/skills/`:

| Skill | Purpose |
|-------|---------|
| `slapper-agent/` | Agent-specific workflows |
| `slapper-ai/` | AI module workflows |
| `slapper-architecture-review/` | Architecture document review methodology |
| `slapper-auth/` | Authentication security testing workflows |
| `slapper-browser/` | Headless browser security testing |
| `slapper-cli/` | CLI parsing, command dispatch, handler patterns |
| `slapper-config/` | Config module workflows |
| `slapper-distributed/` | Distributed module workflows |
| `slapper-fuzzer/` | Fuzzer module workflows |
| `slapper-hunt/` | Vulnerability hunting workflows |
| `slapper-loadtest/` | Loadtest module workflows |
| `slapper-nse/` | NSE/Lua module workflows |
| `slapper-output/` | Output module workflows |
| `slapper-packet/` | Packet capture/crafting/parsing workflows |
| `slapper-pipeline/` | Pipeline module workflows |
| `slapper-proxy/` | Proxy module workflows |
| `slapper-recon/` | Reconnaissance module workflows |
| `slapper-scanner/` | Scanner module workflows |
| `slapper-security/` | Security testing skill workflows |
| `slapper-stress/` | Stress module workflows |
| `slapper-tool/` | Tool module workflows |
| `slapper-tui/` | TUI module workflows |
| `slapper-waf/` | WAF module workflows |
| `tui-testing/` | TUI testing guidance and visual regression patterns |

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
| `architecture/nse_integration.md` | NSE integration |
| `architecture/tui.md` | Terminal User Interface (TUI) module, 28 tabs (+ conditional feature tabs), event loop, components |
| `architecture/defense_lab.md` | Defense-lab mode and regression validation |
| `architecture/stress.md` | Stress testing module (raw sockets, IP spoofing) |
| `architecture/utils.md` | Utility functions (23 submodules) |
| `architecture/types.md` | Core types (Severity, SensitiveString, OutputFormat) |
| `architecture/constants.md` | Centralized constants |
| `architecture/probe.md` | Probe classification (ProbeIntent, ProbeRisk) |
| `architecture/auth_context.md` | Auth context YAML parsing |
| `architecture/logging.md` | Logging configuration |
| `architecture/macros.md` | Exported macros |
| `architecture/generated.md` | Auto-generated protobuf code |

## Verification Commands

```bash
cargo check --lib -p slapper
cargo check -p slapper-nse
cargo test --lib -p slapper
cargo test --test negative_tests -p slapper
cargo test --test scanner_tests -p slapper
cargo clippy --lib -p slapper
```

## Planning Notes for Future Agents

When implementing items from `plans/plan.md`:

1. **Verify before implementing**: Some items in review files may be stale or incorrect. Always verify file paths, line numbers, and whether issues still exist before implementing.

2. **Use subagents for parallel work**: Items are grouped into waves (1-6) that can be implemented in parallel by different agents.

3. **High priority items**: Focus on security issues first (Wave 1: Docker shell injection, Loadtest semaphore panic, Networking pcap silent drop, NSE sandbox tests).

4. **Documentation accuracy**: Many items are documentation fixes. Verify actual code behavior matches what documentation claims.

5. **Stub modules**: Storage, VulnAssessment, and SBOM generators are known stubs - do not try to "fix" them without explicit instruction as they are intentional placeholders.