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
| `pipeline/` | `crates/slapper/src/pipeline/AGENTS.override.md` |
| `nse/` | `crates/slapper-nse/` (Lua VM, NSE libraries, sandbox, CVE integration) |

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
| `distributed/queue.rs:13` | `Task.payload` used `HashMap` instead of `FxHashMap` | Changed to `FxHashMap` for performance |
| `distributed/command.rs:36` | `CommandMessage::Execute.env` used `HashMap` instead of `FxHashMap` | Changed to `FxHashMap` for performance |
| `distributed/remote.rs:30` | `RemoteListener.rate_limits` used `HashMap` instead of `FxHashMap` | Changed to `FxHashMap` for performance |
| `distributed/queue.rs:57` | `dequeue()` ignored `worker_id` and didn't set `assigned_at_secs` | Now tracks which worker owns task and when assigned |
| `distributed/worker.rs:132-161` | Heartbeat used HTTP POST to non-existent REST API | Changed to use `RemoteClient::send_heartbeat()` via TCP |
| `ai/waf_bypass.rs:107` | Loop missing `continue` caused incorrect fallthrough to AI query when entry had `failed_attempts < 3` | Added `continue` after `failed_attempts >= 3` check |
| `ai/planner.rs:456` | `ExecutionStage` has `name` field, not `target` | Changed to `s.name.to_lowercase().contains()` |
| `ai/cache.rs` | `HashMap` used instead of `FxHashMap` | Changed to `FxHashMap` for performance |
| `ai/planner.rs` | `HashMap` used instead of `FxHashMap` | Changed to `FxHashMap` for performance in learning_cache and PlanOutcome |
| `agent/alerts/routing.rs:81` | `expect()` on fallback HTTP client could panic | Propagate error via `?` instead |
| `agent/alerts/routing.rs:107-112` | Race condition in `cleanup_stale_entries` | Inline cleanup under single lock scope |
| `agent/memory.rs:137` | `unwrap()` on `file_stem()` could panic for hidden files | Added fallback hash-based name |
| `agent/mod.rs:657` | Silent error suppression with `unwrap_or_default()` | Log warning with `unwrap_or_else()` |
| `commands/handlers/auth_test.rs:10` | Missing scope validation for auth-test command | Added `ctx.ensure_scope_url(&args.target)?` |
| `commands/handlers/cluster.rs:348` | `unwrap_or(22)` in parse could panic | Changed to `unwrap_or_else(\|_\| 22)` |
| `commands/handlers/mod.rs:155-169` | Hardcoded command list in `handle_no_command` | Replaced with guidance to use `slapper --help` |
| `config/scope.rs:209-226` | Direct IP addresses bypassed private IP checks | Added loopback and private IP validation in `TargetScope::parse()` |
| `config/settings.rs, http.rs, scan.rs` | `HashMap` used instead of `FxHashMap` | Changed to `FxHashMap` for performance in AlertChannelsConfig, WebhookConfigEntry, HttpConfig, ScanConfig |
| `config/api.rs:8` | `maxmind.data_dir` used wrong qualifier | Changed to use `PROJECT_QUALIFIER` consistently |
| `fuzzer/engine/execution.rs:75-79` | Unused `_update_session` parameter in `run_concurrent_inner` | Removed parameter; refactored callers |
| `fuzzer/detection/analyzer.rs:168,206` | `unwrap_or(Ordering::Equal)` on f64 `partial_cmp` could panic on NaN | Added explicit NaN handling with `is_nan()` checks |
| `fuzzer/api_schema/mod.rs:310` | `unwrap_or_default()` silenced body read errors | Changed to explicit match with tracing debug |
| `fuzzer/engine/utils.rs:249` | WAF status codes (403, 406, 429) hardcoded | Extracted to `WAF_BLOCKED_STATUS_CODES` constant |
| `fuzzer/engine/types.rs:176` | `BaselineResponse.headers` used `std::collections::HashMap` | Changed to `FxHashMap` for performance |
| `fuzzer/redos_detect.rs:276` | `PayloadReDosChecker.vulnerable_payloads` used `HashMap` | Changed to `FxHashMap` for performance |
| `loadtest/runner.rs:116-122` | `from_args_with_config()` used `new()` instead of `new_with_tui_mode()`, bypassing validation | Changed to use `new_with_tui_mode()` with `false` for tui_mode to ensure validation is applied |
| `loadtest/runner.rs:327-337` | Non-success HTTP response bodies not consumed, leaving connection pool in inconsistent state | Now consumes response body for non-success before recording metrics |
| `loadtest/runner.rs:300-307` | Rate limiting interval calculation could drift due to using `next + interval` instead of `now + interval` | Changed to compute `next = now_after_sleep + interval` to maintain accurate rate |
| `packet/parse_impl.rs:649` | IP payload extraction could cause out-of-bounds access | Added bounds check before payload extraction |
| `packet/parse_impl.rs:664` | TCP payload extraction used `unwrap()` that could panic | Changed to `and_then` with bounds check |
| `packet/parse_impl.rs:644-651` | Redundant IP payload re-extraction in `ParsedPacket::parse()` | Removed; `IpPacket::parse_ipv4()` already extracts payload correctly |
| `packet/craft.rs:186-187` | IPv4 fragmentation flags byte not initialized in `Ipv4Builder` | Added `bytes[7] = 0` to properly set flags octet |
| `packet/capture.rs:47-49` | PcapWriter timestamp silently defaulted on clock error | Changed to propagate error with warning log |
| `packet/traceroute.rs:622` | `panic!` in test code | Changed to `unreachable!` |
| `packet/mod.rs` | `http_parse` module declared but not present | Removed unused module declaration |
| `output/trend.rs` | `HashMap` used instead of `FxHashMap` | Changed to `FxHashMap` for performance |
| `output/agent.rs` | `FindingSummary` used `HashMap` | Changed to `FxHashMap` for performance |
| `output/dedup.rs` | `DedupEngine::seen` used `HashMap` | Changed to `FxHashMap` for performance |
| `output/diff.rs` | DiffEngine compare used `HashMap`/`HashSet` | Changed to `FxHashMap`/`FxHashSet` for performance |
| `output/baseline.rs` | BaselineComparison compare used `HashSet` | Changed to `FxHashSet` for performance |
| `output/session.rs` | `ScanSession`, `TabSessionState` used `HashMap` | Changed to `FxHashMap` for performance |
| `output/template.rs` | `ReportTemplateEngine`, `TemplateRenderContext` used `HashMap` | Changed to `FxHashMap` for performance |
| `output/attack_graph.rs` | `GraphNode::properties` used `HashMap` | Changed to `FxHashMap` for performance |
| `output/sarif.rs` | `SarifResult::properties` used `HashMap` | Changed to `FxHashMap` for performance |
| `output/junit.rs` | `JUnitBuilder::test_suites` used `HashMap` | Changed to `FxHashMap` for performance |
| `output/escape.rs:53` | `escape_xml` had unnecessary `#[allow(dead_code)]` | Removed attribute (function is used by scanner/pipeline) |
| `output/junit.rs:243` | `to_string_lossy().to_string()` allocated unnecessarily | Changed to `into_owned()` |
| `output/pdf.rs:134` | `generate_html` had clippy warning | Added `#[allow(dead_code)]` |
| `pipeline/mod.rs:240-248` | `resume_cli()` didn't return error on failed stages | Now returns `ScanFailed` error like `run_cli()` |
| `pipeline/executor.rs:444-445` | `run_load_test()` ignored config, used default TLS settings | Changed to `LoadTestRunner::from_args_with_config()` |
| `pipeline/context.rs:12` | `PipelineContext.services` used `HashMap` instead of `FxHashMap` | Changed to `FxHashMap` for performance |
| `slapper-ruby/src/bridge.rs:83-93` | `load_plugin()` used blocking `rx.recv()` with no timeout | Changed to `recv_timeout()` with `DEFAULT_TIMEOUT_SECS` (300s) |
| `slapper-ruby/src/lib.rs:33-43` | `RubyPlugin` didn't capture `author`/`description` metadata | Added `new_with_meta()` to extract plugin metadata |
| `slapper-plugin/src/python.rs:451-475` | Python plugin result truncation silently discarded findings | Now logs count of truncated findings with check name |
| `slapper-nse/src/libraries/socket.rs:98-139` | UDP `connect_udp()` sandbox check was implemented correctly | NSE socket sandbox is fully enforced for all UDP operations |
| `slapper-nse/src/libraries/socket.rs:514-543` | `sendto()` called `connect_udp()` which validates sandbox | UDP sendto is now sandboxed via `connect_udp()` host check |
| `slapper-nse/src/libraries/os.rs:295-302` | Duplicate `getenv` registration in os library | Removed duplicate `getenv_fn2` |
| `slapper-nse/src/output.rs:31-112` | Multiple `unwrap()` on `writeln!` calls could panic | Changed to use `let _ = writeln!()` pattern |
| `slapper-nse/src/cve/mod.rs:172-183` | `CveCache` used `HashMap` instead of `FxHashMap` | Changed to `FxHashMap` for performance |
| `slapper-nse/src/cve/mod.rs:257-305` | `CveAggregator` used `HashSet` instead of `FxHashSet` | Changed to `FxHashSet` for performance |
| `slapper-nse/src/libraries/io.rs:52,225` | Path traversal check bypass via simple `..` string check | Removed string check; rely on `is_path_allowed()` canonicalization |
| `slapper-nse/src/async_executor.rs:107` | `Default` impl used `expect()` that could panic | Changed to `unwrap_or_else` with descriptive panic |
| `stress/icmp.rs:119` | IPv4 flags not set in ICMP packet builder | Added `set_flags(0x40)` for Don't Fragment in `build_icmp_packet_v4()` |
| `stress/udp.rs:244` | Mutex poisoning could cause panic in raw UDP flood | Changed `unwrap()` to `into_inner()` for graceful handling |
| `recon/cve.rs:31` | `CveMapper.cache` used `HashMap` instead of `FxHashMap` | Changed to `FxHashMap` for performance |
| `recon/geolocation.rs:27` | `LOCAL_IP_DATA` used `HashMap` instead of `FxHashMap` | Changed to `FxHashMap` for performance |
| `recon/wayback.rs:86` | `WaybackClient.endpoints` used `HashSet` instead of `FxHashSet` | Changed to `FxHashSet` for performance |
| `recon/takeover.rs:3,455-456` | `HashMap` used instead of `FxHashMap` | Changed to `FxHashMap` for performance |
| `recon/email.rs:4,132,155,174` | `HashSet` used instead of `FxHashSet` | Changed to `FxHashSet` for performance |
| `recon/js.rs:5,229,287` | `HashSet` used instead of `FxHashSet` | Changed to `FxHashSet` for performance |
| `recon/subdomain.rs:8,74,112,158` | `HashSet` used instead of `FxHashSet` | Changed to `FxHashSet` for performance |
| `recon/ssl.rs:96-98` | Unimplemented `supported_versions`/`supported_cipher_suites` fields | Removed misleading empty vector assignments |
| `fuzzer/api_schema/mod.rs:5` | `HashMap` used instead of `FxHashMap` | Changed to `FxHashMap` for performance |
| `fuzzer/payloads/grpc.rs:62` | `GrpcFuzzer.metadata` used `HashMap` | Changed to `FxHashMap` for performance |
| `fuzzer/api_schema/mod.rs:196` | Magic number for oversized payload sizes | Extracted to `OVERSIZED_PAYLOAD_SIZES` constant |
| `scanner/ports/mod.rs:595-598` | `Arc::try_unwrap(...).expect()` could panic if workers not fully joined | Changed to proper error handling with `map_err` |
| `scanner/ports/spoofed.rs:75-95` | `init_packet_trace` opened file twice - second open with `create_new()` failed when file existed | Fixed by opening file once and writing header directly to same handle |
| `scanner/ports/spoofed.rs:111` | Unused `std::collections::HashMap` import | Removed unused import |
| `scanner/ports/spoofed.rs:476-480` | `Arc::try_unwrap(...).expect()` could panic | Changed to proper error handling with `map_err` |
| `scanner/templates/models.rs:57,61` | Duplicate `HttpMatcher` struct definition + missing `DnsMatcher` | Removed duplicate, added `DnsMatcher` before `Matcher` enum |
| `scanner/templates/models.rs:8,61,111` | `HashMap` used instead of `FxHashMap` | Changed to `FxHashMap` for performance |
| `scanner/templates/matcher.rs:9,24` | `HttpResponseData.headers` used `HashMap` | Changed to `FxHashMap` for performance |
| `scanner/templates/executor.rs:165` | `std::collections::HashMap::new()` used | Changed to `FxHashMap::default()` |
| `scanner/cms/mod.rs:15,52,165,291` | `security_headers` used `HashMap` | Changed to `FxHashMap` for performance |
| `scanner/endpoints.rs:835-839` | `Arc::try_unwrap(...).expect()` could panic | Changed to proper error handling with `map_err` |
| `scanner/fingerprint.rs:319-323` | `Arc::try_unwrap(...).expect()` could panic | Changed to proper error handling with `map_err` |
| `tui/tabs/scan.rs:250-256` | Division by zero when `stages` is empty | Added `if self.stages.is_empty()` guard |
| `tui/components/scrollable.rs:135` | `scroll_offset` could be `usize::MAX` when `lines` is empty | Added `if self.lines.is_empty()` check |
| `tui/workers/recon.rs:212` | `unreachable!()` panic after retry loop | Replaced with proper `Err()` return |
| `tui/workers/api.rs:89` | Silent error suppression on HTTP response read | Changed to explicit match with `tracing::debug` |
| `tui/app/state_update.rs:58-74` | Unhandled `TaskResult` variants silently dropped | Added debug logging for unhandled variants |
| `tui/tabs/history.rs:55` | `unwrap_or_default()` silenced JSON serialization errors | Changed to explicit match with `tracing::debug` |
| `waf/detector/detect.rs:118` | IP match scoring used `COOKIE_MATCH_SCORE` instead of `IP_MATCH_SCORE` | Added `IP_MATCH_SCORE` constant (20) and fixed scoring |
| `waf/mod.rs:4` | Docstring said "26 WAF products" but 34 are supported | Updated to "34 WAF products" |
| `waf/detector/*.rs` | `unwrap_or_default()` on `response.text().await` silently suppressed errors | Changed to explicit match with `tracing::debug` |
| `cli/scan.rs` | Missing `-o` short flag on PortScanArgs, EndpointScanArgs, FingerprintArgs, NseArgs, ResumeArgs | Added `short = 'o'` to output fields |
| `cli/fuzz.rs` | Missing `-o` short flag on WafStressArgs; indentation issue after edit | Added `short = 'o'` and fixed indentation; preserved `From<WafStressArgs>` impl |
| `cli/http.rs` | Missing `-o` short flag on ReconArgs | Added `short = 'o'` to output field |

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
- `.opencode/skills/slapper-nse/` - NSE/Lua module workflows
- `.opencode/skills/slapper-packet/` - Packet capture/crafting/parsing workflows
- `.opencode/skills/slapper-loadtest/` - Loadtest module workflows
- `.opencode/skills/slapper-pipeline/` - Pipeline module workflows
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
| `architecture/output.md` | Output & reporting module |
| `architecture/plugins_nse.md` | Plugin system (Python/Ruby) and NSE integration |
