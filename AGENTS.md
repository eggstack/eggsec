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

## Recent Bug Fixes

### 2026-05-28 (Architecture Review Wave 4)

| Component | Issue | Fix |
|-----------|-------|-----|
| WAF | `detect.rs:81` - magic number 256 for header value length | Added `HEADER_VALUE_MAX_LEN` constant |
| Scanner | `spoofed.rs:285,303` - silent errors in spoofed scan | Added `tracing::debug` for failed packet builds |

### 2026-05-28 (Architecture Review Wave 3)

| Component | Issue | Fix |
|-----------|-------|-----|
| AI | `cache.rs:276` - AiCache persist() silently failed | Added `tracing::warn` for persist failures |
| Scanner | `fingerprint.rs:347-391` - Vec allocation in hot path | Changed to static slice references |

### 2026-05-28 (Architecture Review Wave 2)

| Component | Issue | Fix |
|-----------|-------|-----|
| AI Agents | `skills.rs:202`, `portfolio.rs:112` - HashMap | Replaced with FxHashMap |
| AI | `script_gen.rs:97,141,185,272` - unwrap_or_default | Replaced with explicit error handling |
| AI | `client.rs:241` - silent fallback for Anthropic messages | Added `tracing::debug` for missing messages |
| CLI | `fuzz.rs:292` - WafStressArgs output discarded | Preserved `args.output` in From impl |
| Fuzzer | `execution.rs:267` - rate==0 causes early stop | Changed to `rate <= 1` |
| Loadtest | `runner.rs:360` - JoinSet panic handling | Added panic-aware error handling |
| Output | `diff.rs:139` - has_regressions only checked Critical | Now checks `severity >= Severity::High` |
| WAF | `patterns.rs:656` - get_waf_signatures clones | Returns `&'static FxHashMap` instead |
| WAF | `detector/mod.rs:33` - signatures clone on creation | Stores static reference |

### 2026-05-28 (Architecture Review Wave 1)

| Component | Issue | Fix |
|-----------|-------|-----|
| Distributed | `queue.rs:150` - QueueError missing traits | Added Display and Error impl |
| Distributed | `worker.rs:32`, `remote.rs:105` - capability mismatch | Unified via shared CAPABILITIES constant |
| Networking | `stress/udp.rs:98` - UDP checksum missing payload | Added `pseudo[16..].copy_from_slice(payload)` |
| Networking | `craft.rs:247` - TCP checksum set to 0 | Added `compute_tcp_checksum()` function |
| Tool | `registry.rs:2,24` - HashMap instead of FxHashMap | Replaced with FxHashMap |
| Pipeline | `executor.rs:138`, `session.rs:13` - spoof_config not persisted | Added `spoof_config` to PipelineSession |
| NSE | `vulns.rs:209,232` - duplicate CVE-2024-27956 | Added comment documenting limitation |
| Scanner | `fingerprint.rs:347` - Vec allocation per port | Changed to `&'static [&str]` slice |

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

The consolidated implementation plan is in `plans/plan.md`. It contains 26 remaining items across 3 waves:

| Wave | Items | Priority | Description |
|------|-------|----------|-------------|
| Wave 1 | 4 | Critical | Scanner UDP reuse, WAF HashMap lookup, CLI CIDR validation, Recon CveMapper cache |
| Wave 2 | 9 | High | Distributed (3), Pipeline (2), Loadtest (1), Output (1), Config (1) module fixes |
| Wave 3 | 13 | Medium/Low | Improvement opportunities across multiple modules |

---

## Knowledge Gained from Architecture Review Sessions

### Scope Validation (CLI/Config)
- Private IP check (`is_private_ip()`) occurs BEFORE scope rule evaluation in `TargetScope::parse()`. This means targets like `10.255.255.255` are rejected as private even if a scope rule like `allow 10.0.0.0/8` exists.
- Scope rejection reasons are not reported - no indication of whether rejection was due to exclude rule or no include match.
- DNS resolution failures silently return `ip: None` which can bypass CIDR rules.

### Recon Module
- `CveMapper` cache doesn't persist - each call to `map_cves()` creates a new `CveMapper` instance, so cache is lost.
- `query_alexa()` function is stubbed (returns empty) and never called.
- Secrets module (`secrets.rs`) is standalone and NOT in `FULL_RECON_PIPELINE_MODULES`.
- Dependency scan handles Ruby (Gemfile), PHP (composer.json), and Java (pom.xml) in addition to documented npm/cargo/go.
- FxHashMap count is actually 66+, not the documented 55.

### Distributed Module
- `RemoteClient` lacks `Drop` impl - connections not explicitly closed on panic.
- Heartbeat creates a new TCP connection each time via `RemoteClient::new_plaintext()`.
- DNS lookup happens every `connect()` call, not cached.
- Only `completed` queue has size limit; `pending` and `in_progress` can grow unbounded.
- Rate limit check was fixed (lock duration minimized).

### WAF Module
- `select_profile()` does O(p×s) nested linear scan through all profiles and signatures (not yet optimized).
- `BypassResult` lacks `error: Option<String>` field for network error details.
- Evasion bypass calls `get_sqli_payloads()` 7 times in loops - redundant generation.

### Pipeline Module
- Hardcoded ports duplicated in `executor.rs:276-283` and `executor.rs:534`.
- Profile mapping duplicated in `stage.rs:31-92` and `tool/implementations/pipeline.rs:64-77`.
- Stages execute sequentially (no concurrent execution mode).

### Loadtest Module
- `response.bytes().await` called inside lock at `runner.rs:342` - body consumed while holding metrics lock (still outstanding).
- Missing test coverage for TLS, streaming, chunked, redirect, rate limiting, auth, proxy, timeouts.
- Panic handling in JoinSet was fixed.

### Fuzzer Module
- `GrammarFuzzer::with_seed()` exists but undocumented.
- `KNOWN_VULNERABLE_PATTERNS` in `redos_detect.rs` creates Vec on every call (should use `LazyLock`).

### Networking/Packet Module
- IPv6 raw sockets not supported for UDP flood (returns error).
- `PacketBuilder` lacks `validate()` method.

### Output Module
- Compliance templates (`pcidss_template()`, `soc2_template()`) recreate structs every call (should use `LazyLock`).

### TUI Module
- `App.tabs` field (`FxHashMap`) is initialized but never populated or used - dead code.

(End file - 298 lines)