# Slapper Improvement Plan - Consolidated

**Date**: 2026-04-29
**Status**: IN PROGRESS
**Priority**: High

---

## Executive Summary

**Current State** (verified 2026-04-29):
- **1,115** passing tests (base library)
- **1,364** passing tests (with full features - ai-integration)
- **7** pre-existing AI test failures (ai::planner, ai::waf_bypass, ai::client)
- **~21** clippy warnings (TUI-specific acceptable)
- **503** source files
- **30** payload types
- **29** TUI tabs
- **34** WAF products detected
- **31** reconnaissance modules

**Completed (2026-04-28-29 sessions)**:
- All Waves A-G items (121 verified complete)
- AI-integration compilation fixes (portfolio_path, AttackSurface, SkillMetadata, AlertRouter)
- AuthTab rewrite with FocusArea enum
- D.4 help overlay fix, D.6/D.10 error handling
- E.7 subdomain enumeration (removed stub alexa source)

---

## Wave Organization Overview

| Wave | Focus | Items | Status | Est. Time |
|------|-------|-------|--------|-----------|
| **1** | Critical Fixes | 8 | Pending | ~8 hours |
| **2** | Code Quality | 13 | Pending | ~16-20 hours |
| **3** | Performance | 7 | Pending | ~8-12 hours |
| **4** | TUI Improvements | 5 | Pending | ~8 hours |
| **5** | Feature Enhancements | 47 | Pending | ~100+ hours |
| **6** | New Capabilities | 11 | Pending | ~12+ months |
| **7** | Documentation | 3 phases | Pending | ~8 hours |

**Total pending items**: ~91 items (~150+ hours)

---

## WAVE 1: Critical Fixes

### 1.1 Compilation Blockers

#### 1.1.1 k8s-openapi Version Gate
- **Status**: ALREADY FIXED
- **Files**: `crates/slapper/Cargo.toml:175-178`
- **Note**: k8s-openapi version is now 0.22, `container` feature enables both `kube` and `k8s-openapi` correctly
- **Verification**: `cargo check --lib -p slapper --features full` (should compile)
- **Est. Time**: N/A (complete)

#### 1.1.2 Commands Enum Box Optimization
- **Issue**: `Commands` enum has ~776-byte `FuzzArgs` variant (40+ fields)
- **Files**: `crates/slapper/src/cli/mod.rs:80-95`
- **Fix**: Change `Fuzz(FuzzArgs)` to `Fuzz(Box<FuzzArgs>)`
- **Verification**: `cargo clippy --lib -p slapper 2>&1 | grep -i "large_enum_variant"`
- **Est. Time**: 30 minutes

#### 1.1.3 Spoofed Scanner Missing Import
- **Issue**: `get_service_name` called at `spoofed.rs:419` but not imported
- **File**: `crates/slapper/src/scanner/ports/spoofed.rs:419`
- **Fix**: Add `use crate::scanner::ports::get_service_name;` or `use crate::utils::service_detection::get_service_name;`
- **Verification**: `cargo check --lib -p slapper --features stress-testing`
- **Est. Time**: 5 minutes

---

### 1.2 Security Vulnerabilities (Critical - C1-C4 from Plan 3)

#### 1.2.1 Plaintext Passwords in Auth Results (C1)
- **Severity**: CRITICAL
- **Files**:
  - `recon/ssh_auth.rs:30-35`
  - `recon/ftp_auth.rs:19`
  - `recon/smtp_auth.rs:27-28`
  - `auth/credential_stuffing.rs:17-22`
- **Issue**: Authentication structs store passwords as `String` instead of `SensitiveString`
- **Fix**: Replace `password: String` with `password: SensitiveString`, custom serialization
- **Verification**: `rg "password: String" crates/slapper/src/recon/`
- **Est. Time**: 2 hours

#### 1.2.2 WebSocket Authentication Bypass (C2)
- **Severity**: CRITICAL
- **File**: `tool/protocol/rest.rs:224-250`
- **Issue**: `/ws` endpoint doesn't call `require_auth()`
- **Fix**: Add authentication check in `ws_handler` (similar to `health_check` at lines 335-353 which checks `if state.api_key.is_some()`)
- **Verification**: `curl -i -N -H "Upgrade: websocket" http://localhost:8080/ws` (should return 401 when API key configured)
- **Est. Time**: 30 minutes

#### 1.2.3 Template Marketplace Verification Silent Failure (C3)
- **Severity**: CRITICAL
- **File**: `scanner/templates/marketplace.rs:133-167`
- **Issue**: On verification error, code only logs warning and continues - does NOT return error
- **Fix**: Treat verification errors as fatal - return `Err` instead of just logging warning
- **Verification**: Test with invalid signature template (should reject, not continue)
- **Est. Time**: 1 hour

#### 1.2.4 Template SSRF / Private IP Blocking
- **Status**: PARTIALLY PROTECTED - template loader operates on pre-validated YAML files
- **File**: `scanner/templates/executor.rs:106-115`
- **Note**: TemplateExecutor::execute_on_target operates on templates from disk via TemplateLoader::load_all()
- **Recommendation**: Verify if additional private IP validation is needed at template execution layer
- **Verification**: Test internal URL access in template execution
- **Est. Time**: 1 hour (investigation)

---

### 1.3 gRPC Implementation

- **Issue**: gRPC service has infrastructure (tonic, prost) but all RPC methods are stubs
- **Files**: `crates/slapper/src/tool/protocol/grpc.rs:166-205`
- **Methods to implement**:
  | Method | Status | Description |
  |--------|--------|-------------|
  | `list_tools` | STUB | List available tools |
  | `get_tool` | STUB | Get tool details |
  | `execute_tool` | STUB | Execute a tool |
  | `stream_execute_tool` | STUB | Streaming execution |
  | `get_capabilities` | STUB | Get server capabilities |
- **Dependencies**: `ToolRegistry` (tool/registry.rs:23-194), `SecurityTool` trait (tool/traits.rs:143-205)
- **Verification**: `cargo check --lib -p slapper --features grpc-api`; `grpcurl -plaintext localhost:50051 list`
- **Est. Time**: 8 hours

---

### 1.4 Stress Module Security Fixes

#### 1.4.1 No Spoof Range Validation
- **Issue**: Spoof range validation doesn't check for private/reserved IPs
- **Files**:
  - `stress/authorization.rs`
  - `stress/syn.rs:234-303`
  - `stress/udp.rs:273-299`
  - `stress/icmp.rs:240-309`
- **Fix**: Add private IP validation to `parse_spoof_range()`
- **Verification**: `cargo test --lib -p slapper -- stress::authorization`
- **Est. Time**: 1 hour

#### 1.4.2 UDP Broadcast Enabled
- **Issue**: UDP flood enables `set_broadcast(true)` on socket
- **File**: `stress/udp.rs:401`
- **Fix**: Remove `set_broadcast(true)` or limit to specific interfaces
- **Verification**: `cargo test --lib -p slapper -- stress::udp`
- **Est. Time**: 15 minutes

#### 1.4.3 No Per-Source Rate Limiting
- **Issue**: Global rate limit only, no per-source tracking
- **Files**: `stress/syn.rs`, `udp.rs`, `icmp.rs`
- **Fix**: Add per-source tracking with `DashMap<IpAddr, AtomicU64>`
- **Verification**: `cargo test --lib -p slapper -- stress::metrics`
- **Est. Time**: 2 hours

---

**Wave 1 Total Estimated**: ~8 hours

---

## WAVE 2: Code Quality

### 2.1 Dead Code Removal (Quick Wins)

#### 2.1.1 Remove Unused WebSocketFuzzer Field
- **File**: `fuzzer/payloads/websocket.rs:53`
- **Issue**: `url` field in `WebSocketFuzzer` is never read
- **Fix**: Remove `url: String` field and update constructor
- **Verification**: `cargo test --lib -p slapper -- websocket`
- **Est. Time**: 15 minutes

#### 2.1.2 Remove Unused ParsedDependency Struct
- **File**: `recon/dependency_scan/mod.rs:61-64`
- **Issue**: `ParsedDependency` struct is never constructed
- **Fix**: Remove the struct definition
- **Verification**: `grep -r "ParsedDependency" crates/slapper/src/` (should only return definition)
- **Est. Time**: 10 minutes

#### 2.1.3 Remove Unused is_input_focused Method
- **File**: `tui/app/dispatch.rs:80-82`
- **Issue**: `TabDispatcher::is_input_focused` method is never called
- **Fix**: Remove the method
- **Verification**: `cargo test --lib -p slapper -- dispatch`
- **Est. Time**: 10 minutes

#### 2.1.4 Clean Up Unused Allow Attributes
- **Files**:
  - `tui/workers/recon.rs:5` - remove `#[allow(unused_variables)]`
  - `vuln/mod.rs:21-32` - remove 6 false positive `#[allow(unused_imports)]`
  - `integrations/common.rs:1` - remove false positive `#[allow(unused_imports)]`
- **Fix**: Remove false positive allow attributes
- **Verification**: `cargo clippy --lib -p slapper 2>&1 | grep -i allow`
- **Est. Time**: 20 minutes

---

### 2.2 Error Handling Improvements

#### 2.2.1 Refactor Agent Initialization
- **Files**: `agent/mod.rs:457,476,484,491,509,531,533`
- **Issue**: Uses `.unwrap()` causing CLI crashes
- **Fix**: Replace with proper error handling:
  ```rust
  let agent = Agent::new(config).await
      .map_err(|e| SlapperError::AgentError(format!("Failed to initialize agent: {}", e)))?;
  ```
- **Verification**: `cargo test --lib -p slapper -- agent`
- **Est. Time**: 1 hour

#### 2.2.2 Improve Config Unwraps in Command Handlers
- **File**: `commands/handlers/agent.rs:71`
- **Fix**: Use `if let Some(config) = ...` pattern instead of `.unwrap()`
- **Verification**: `cargo test --lib -p slapper -- commands`
- **Est. Time**: 30 minutes

#### 2.2.3 Standardize Error Types
- **Issue**: 22+ instances of `Result<T, String>` mixed with `Result<T, SlapperError>`
- **Files**: Various TUI, gRPC, reconnaissance modules
- **Fix**: Create `UserFacingError` type or document when `String` is appropriate
- **Verification**: `cargo test --lib -p slapper`
- **Est. Time**: 2-3 hours

---

### 2.3 Code Refactoring (Long-term)

#### 2.3.1 Split tool/session.rs
- **File**: `tool/session.rs` (1418 lines)
- **Split into**:
  ```
  tool/session/
  ├── mod.rs      (re-exports)
  ├── auth.rs     (AuthMethod, MfaConfig, LoginSequence, LoginStep)
  ├── csrf.rs     (CsrfToken, CsrfExtractor)
  ├── forms.rs    (FormDetector, LoginForm, FormField)
  ├── state.rs    (SessionState, LoginExecutor, SessionVerifier, AuthenticatedSessionManager)
  ```
- **Verification**: `cargo test --lib -p slapper -- session`
- **Est. Time**: 4-6 hours

#### 2.3.2 Split mcp/handlers/server.rs
- **File**: `tool/protocol/mcp/handlers/server.rs` (898 lines)
- **Split into**:
  ```
  tool/protocol/mcp/handlers/
  ├── mod.rs
  ├── server.rs    (McpServer struct only)
  ├── tools.rs     (handle_tools_* methods)
  ├── session.rs   (handle_session_* methods)
  ├── resources.rs (handle_resources_* methods)
  └── prompts.rs   (handle_prompts_* methods)
  ```
- **Verification**: `cargo test --lib -p slapper -- mcp`
- **Est. Time**: 3-4 hours

#### 2.3.3 Extract DEFAULT_ENDPOINTS
- **File**: `scanner/endpoints.rs`
- **Issue**: 225-line static wordlist
- **Fix**: Extract to `scanner/wordlists/endpoints.rs`
- **Verification**: `cargo test --lib -p slapper -- endpoints`
- **Est. Time**: 1-2 hours

---

### 2.4 Infrastructure Improvements

#### 2.4.1 Document Feature Matrix
- **File**: `README.md`
- **Issue**: 29 features without clear documentation
- **Fix**: Add feature matrix table showing core, optional, and pre-built feature sets
- **Est. Time**: 1 hour

#### 2.4.2 Clean Up Storage Module
- **File**: `storage/postgres.rs`
- **Issue**: Stub implementation without documentation
- **Fix**: Add documentation explaining this is stub code
- **Est. Time**: 30 minutes

#### 2.4.3 Add Feature Validation CI
- **Fix**: Add CI workflow testing feature combinations: `rest-api`, `rest-api,ai-integration`, `full`
- **Est. Time**: 2 hours

---

**Wave 2 Total Estimated**: ~16-20 hours

---

## WAVE 3: Performance

### 3.1 Quick Wins

#### 3.1.1 Eliminate to_string() in Hot Path
- **File**: `fuzzer/engine/utils.rs:211,237`
- **Issue**: `payload_type.to_string()` called twice per fuzz request
- **Impact**: ~7.2 million allocations/hour at 1000 req/s
- **Fix**: Use `payload_type.as_str()` or pass `&str` through call chain
- **Verification**: `cargo test --lib -p slapper -- fuzzer::engine::utils`
- **Est. Time**: 15 minutes

#### 3.1.2 Migrate std::sync::Mutex to parking_lot
- **File**: `stress/udp.rs:202`
- **Fix**: Replace `std::sync::Mutex` with `parking_lot::Mutex`
- **Verification**: `cargo check --features stress-testing`
- **Est. Time**: 10 minutes

---

### 3.2 Medium Effort

#### 3.2.1 FxHashMap Migration (Priority Files)

| Priority | File | Usage Context |
|----------|------|---------------|
| 1 | `fuzzer/redos_detect.rs` | ReDoS detection hot path |
| 2 | `tool/session.rs` | Every HTTP session |
| 3 | `agent/alerts/routing.rs` | Alert routing |
| 4 | `scanner/cms/mod.rs` | CMS detection |
| 5 | `ai/planner.rs` | AI planning |
| 6 | `recon/cve.rs` | CVE lookups |
| 7 | `utils/cache.rs` | Cache operations |
| 8 | `tool/state.rs` | Tool state |
| 9 | `tool/ratelimit.rs` | Rate limiting |
| 10 | `config/settings.rs` | Config storage |

- **Pattern**: `use rustc_hash::FxHashMap; let mut map = FxHashMap::default();`
- **Verification**: `rg "std::collections::HashMap" crates/slapper/src | wc -l` (should reduce from 131)
- **Est. Time**: 1-2 hours

#### 3.2.2 TUI Render Optimization

##### 3.2.2.1 Progress Update Throttling
- **File**: `tui/app/state_update.rs:8-19`
- **Issue**: Every progress update triggers full redraw (dozens/second)
- **Fix**: Add 100ms debounce with `last_progress_update: Option<Instant>`

##### 3.2.2.2 Breadcrumb Caching
- **File**: `tui/ui.rs:312-458`
- **Issue**: Breadcrumb recomputed every frame
- **Fix**: Cache on tab change with `cached_breadcrumb: Option<Line>`

##### 3.2.2.3 Command Palette Virtualization
- **File**: `tui/app/runner.rs:280-295`
- **Issue**: Renders all results without pagination
- **Fix**: Use existing `scroll_offset` with `skip().take(14)`

- **Verification**: `cargo test --lib -p slapper -- tui`
- **Est. Time**: 2-3 hours

---

### 3.3 High Effort

#### 3.3.1 Fuzzer Clone Reduction
- **File**: `fuzzer/engine/execution.rs:101-113`
- **Issue**: 13 clones per concurrent request
- **Solution - FuzzWorker pattern**:
  ```rust
  pub struct FuzzWorker {
      client: Client,
      args: FuzzArgs,
      timing_analyzer: Arc<Mutex<TimingAnalyzer>>,
      pattern_matcher: PatternMatcher,
      user_agent: String,
  }

  impl FuzzWorker {
      async fn execute(self: Arc<Self>, payload: Payload) -> Result<FuzzResult> {
          tokio::spawn(async move { self.run_fuzz(payload).await }).await?
      }
  }
  ```
- **Files to modify**: `fuzzer/engine/execution.rs`, `fuzzer/engine/core.rs`
- **Verification**: `cargo test --lib -p slapper -- fuzz`
- **Est. Time**: 4-6 hours

---

### 3.4 Security Performance Items

#### 3.4.1 ReDoS Prevention (H2)
- **Files**:
  - `fuzzer/filters.rs:91-95`
  - `tool/session.rs:564,577,870,882,1004,1012`
- **Issue**: User-provided regex patterns not validated, no `size_limit()`
- **Fix**: Use `RegexBuilder` with `size_limit(100_000)` everywhere
- **Verification**: `cargo test --lib redos_filter`
- **Est. Time**: 2 hours

#### 3.4.2 Unbounded Regex Caches (H3)
- **Files**:
  - `fuzzer/chain.rs:81`
  - `scanner/templates/matcher.rs:23`
  - `slapper-nse/src/libraries/re.rs:11`
  - `slapper-nse/src/libraries/pcre.rs:11`
- **Issue**: All regex caches are unbounded `FxHashMap`
- **Fix**: Replace with `LruCache::new(1000)`
- **Verification**: Test with many patterns - memory should stay bounded
- **Est. Time**: 2 hours

---

**Wave 3 Total Estimated**: ~8-12 hours

---

## WAVE 4: TUI Improvements

### 4.1 LoadTest Module Fixes

#### 4.1.1 LoadTest Connection Pooling
- **File**: `loadtest/runner.rs:237-254`
- **Issue**: Doesn't configure explicit HTTP connection pooling
- **Fix**: Add `pool_max_idle_per_host(20)`, `pool_idle_timeout`, `tcp_nodelay(true)`
- **Verification**: `cargo test --lib -p slapper -- loadtest`
- **Est. Time**: 30 minutes

#### 4.1.2 Metrics Mutex Contention
- **File**: `loadtest/runner.rs:304`, `loadtest/metrics.rs`
- **Issue**: Every load test request locks `Arc<Mutex<Metrics>>`
- **Fix**: Use per-thread aggregation with atomic counters
- **Verification**: `cargo test --lib -p slapper -- loadtest`
- **Est. Time**: 2 hours

#### 4.1.3 Histogram Record Silent Failure
- **File**: `loadtest/metrics.rs:86`
- **Issue**: `self.histogram.record(latency_ms).ok()` swallows errors
- **Fix**: Log error instead of silently ignoring
- **Verification**: `cargo test --lib -p slapper -- loadtest`
- **Est. Time**: 15 minutes

#### 4.1.4 Real-Time Load Test Streaming
- **File**: `loadtest/runner.rs:332`
- **Issue**: Results only after all requests complete
- **Fix**: Implement progress updates using watch channel
- **Verification**: `cargo test --lib -p slapper -- loadtest`
- **Est. Time**: 3 hours

---

### 4.2 AuthTab Refinement
- **File**: `tui/tabs/auth.rs`
- **Status**: Rewrite complete, verify edge cases:
  1. Error display formatting
  2. OAuth flow state handling
  3. Test coverage
- **Verification**: `cargo test --lib -p slapper -- auth`
- **Est. Time**: 2 hours

---

### 4.3 High Priority Security (H4, H5, H7)

**Note**: H6 (Intercept Proxy TLS) was removed - it's by design, not a bug.

#### 4.3.1 CSRF Token Expiration Not Set (H4)
- **File**: `tool/session.rs:712,727,755`
- **Issue**: `expires_at` always set to `None`
- **Fix**: Set reasonable default TTL (30 minutes)
- **Verification**: Test token expiration - after 30 minutes should be marked expired
- **Est. Time**: 1 hour

#### 4.3.2 CSRF Token No Regeneration After Auth (H5)
- **File**: `tool/session.rs`, `tool/state.rs:84-94`
- **Issue**: Session IDs regenerated but CSRF tokens not cleared
- **Fix**: Clear CSRF tokens on authentication:
  ```rust
  pub fn set_authenticated(&mut self, auth: bool) {
      if auth && !self.authenticated {
          self.regenerate_session_id();
          self.csrf_tokens.clear();
      }
      self.authenticated = auth;
  }
  ```
- **Verification**: Test CSRF token cleared after login
- **Est. Time**: 1 hour

#### 4.3.3 Intercept Proxy TLS Validation
- **Status**: NOT A BUG - by design
- **File**: `proxy/intercept/mod.rs:186-189`
- **Explanation**: This is an intercepting proxy. Client connects via TLS (line 197: `tls_acceptor.accept(stream)`), proxy makes raw TCP upstream connection to inspect decrypted traffic. This is intentional architecture. Private IP validation exists at line 184 via `validate_target`.
- **Est. Time**: N/A (no fix needed)

#### 4.3.4 MCP OpenAPI/Health Unauthenticated (H7)
- **File**: `tool/protocol/mcp/routes.rs:179`
- **Issue**: Health, OpenAPI, Plan endpoints don't check auth
- **Fix**: Add auth requirement or document security implications
- **Verification**: `curl http://localhost:8080/health` without auth header
- **Est. Time**: 1 hour

---

### 4.4 Formula Injection Prevention (H1)
- **File**: `pipeline/report.rs:9-21`
- **Issue**: Duplicate `escape_csv()` missing NFKC normalization - fullwidth bypass possible
- **Fix**: Use `crate::output::escape::escape_csv()` instead of local implementation
- **Verification**: `cargo test --lib formula`
- **Est. Time**: 1 hour

---

**Wave 4 Total Estimated**: ~8 hours

---

## WAVE 5: Feature Enhancements

### 5.1 Agent Enhancement (Plan 6)

**10 Enhancement Areas**:

| # | Area | Priority | Effort | Key Files |
|---|------|----------|--------|-----------|
| 1 | Adaptive Scan Strategy | High | Medium | `agent/mod.rs`, `agent/memory.rs` |
| 2 | False Positive Learning | High | Medium | `agent/memory.rs`, `tool/protocol/rest.rs` |
| 3 | Remediation Tracking | High | Medium | `agent/portfolio.rs` |
| 4 | Threat Intelligence Integration | High | Medium | `agent/threat_intel.rs` (new) |
| 5 | Resource-Aware Scheduling | Medium | Low | `agent/system.rs` (new) |
| 6 | Asset Discovery Automation | High | High | `agent/discovery.rs` (new) |
| 7 | Distributed Coordination | Medium | High | `tool/agents/registry.rs` |
| 8 | Exploitation Verification | High | High | `fuzzer/verification/*.rs` (new) |
| 9 | External Integration | Medium | Medium | `agent/alerts/channels.rs` |
| 10 | Kill Chain Coverage | Medium | Medium | `kill_chain/mod.rs` (new) |

**New Types**:
```rust
// ScanIntensity (agent/portfolio.rs)
pub enum ScanIntensity { Minimal, Standard, Thorough, Aggressive }

// FpPattern (agent/memory.rs)
pub enum FpReason { LegitimateBehavior, Duplicate, TestData, ... }
pub enum FpMatcherType { UrlRegex, ParameterPattern, Fingerprint, FindingType }
pub struct FpPattern { id, matcher_type, pattern, source_tool, ... }

// RemediationStatus (agent/portfolio.rs)
pub enum RemediationStatus { Open, InProgress, VerifiedFixed, RiskAccepted, FalsePositive }

// KillChainPhase (kill_chain/mod.rs)
pub enum KillChainPhase { Reconnaissance, Weaponization, Delivery, Exploitation, Installation, CommandAndControl, ActionsOnObjectives }
```

**Implementation Phases**:
| Phase | Duration | Focus | Items |
|-------|----------|-------|-------|
| 1 | Weeks 1-4 | Foundation | Threat Intel, False Positive Learning, Remediation Tracking |
| 2 | Weeks 5-8 | Intelligence | Adaptive Scan, Resource Scheduling, Kill Chain |
| 3 | Weeks 9-14 | Automation | Asset Discovery, Exploitation Verification |
| 4 | Weeks 15-18 | Scale | Distributed Coordination, External Integration |

**Verification**: `cargo test --lib -p slapper adaptive` etc.

---

### 5.2 Plugin System Enhancement (Plan 7)

**7 Waves**:

| Wave | Description | Files | Est. Lines |
|------|-------------|-------|-----------|
| P1 | Ruby API Expansion | `slapper-ruby/src/api.rs`, `slapper-ruby/src/lib.rs` | ~450 |
| P2 | ProcessPluginRunner Integration | `slapper-plugin/src/lib.rs`, `slapper-plugin/src/python.rs` | ~140 |
| P3 | Network Restrictions | `slapper-plugin/src/config.rs` | ~100 |
| P4 | Plugin Signing | `slapper-plugin/src/signer.rs` (new), `slapper-plugin/src/verify.rs` (new) | ~210 |
| P5 | Categories/Tags | `slapper-plugin/src/lib.rs`, CLI commands | ~90 |
| P6 | Community Infrastructure | `slapper-plugins/` (future), `specs/plugin-index.schema.json` | ~250 |
| P7 | Documentation Update | `docs/PLUGIN_DEVELOPMENT.md`, `docs/API_REFERENCE.md` | ~550 |

**Ruby API Functions to Add**:
1. `Slapper::Http.request()` - HTTP client
2. `Slapper::Scanner.scan()` - Scanner integration
3. `Slapper::Fuzzer.fuzz()` - Fuzzer integration
4. `Slapper::Recon` functions - DNS, subdomain enum
5. `Slapper::Target` - Target management
6. `Slapper::Events` - Event subscriptions

**ProcessPluginRunner Decision**:
```
Plugin Loading:
├── Trusted source (~/.config/slapper/plugins/) → Embedded (PyO3)
├── Untrusted source (marketplace download) → Process + Sandboxed
└── Config override (plugin_isolation = "sandboxed") → Force subprocess
```

**Verification**: `cargo check --lib -p slapper --features python-plugins,ruby-plugins`

---

### 5.3 Fuzzer Gaps

#### 5.3.1 HTTP Request Smuggling Payloads
- **Issue**: No dedicated smuggling payloads (CL.TE, TE.CL)
- **New module**: `fuzzer/payloads/smuggling.rs`
- **Payload types**:
  ```rust
  pub enum SmugglingTechnique {
      ClTe,           // Content-Length vs Transfer-Encoding
      TeCl,           // Transfer-Encoding takes precedence
      ChunkedMalformed,
      DoubleContentLength,
      MultipartMixed,
  }
  ```
- **Verification**: `cargo test --lib -p slapper -- smuggling`
- **Est. Time**: 3 hours

#### 5.3.2 Business Logic Fuzzing
- **Issue**: No payloads for integer overflow, logic bypass
- **Payloads**: `"999999999999999999"`, `"-999999999999999999"`, `"0x7FFFFFFFFFFFFFFF"`, `"18446744073709551615"`
- **Verification**: `cargo test --lib -p slapper -- fuzz`
- **Est. Time**: 2 hours

#### 5.3.3 Grammar Weight Support
- **File**: `fuzzer/payloads/macros.rs`
- **Issue**: `payload_vec!` macro doesn't support weights
- **Fix**: Extend macro with weight parameter
- **Verification**: `cargo test --lib -p slapper -- payload`
- **Est. Time**: 2 hours

---

### 5.4 WAF Detection Gaps

#### 5.4.1 Missing WAF Products
- **File**: `waf/bypass/profiles.rs`
- **Missing WAFs**:
  | WAF | Priority | Status |
  |-----|---------|--------|
  | Reblaze | Medium | Partial |
  | OpenResty | Medium | Missing |
  | HAProxy WAF | Medium | Missing |
  | Palo Alto Advanced | Low | Bypass incomplete |
- **Verification**: `cargo test --lib -p slapper -- waf`
- **Est. Time**: 4 hours

---

### 5.5 CLI Consistency

#### 5.5.1 Derive Macro Inconsistencies
- **Files**: `cli/ci.rs:3-4`, `cli/plan.rs:3-4`, `cli/ai_analyze.rs:3-4`
- **Issue**: Some use `#[derive(Parser)]` instead of `#[derive(clap::Args)]`
- **Fix**: Normalize to `#[derive(clap::Args)]` with `#[command(group = ...)]`
- **Verification**: `cargo check --lib -p slapper`
- **Est. Time**: 1 hour

#### 5.5.2 Help Text Inconsistencies
- **File**: `cli/http.rs:138,163,194`
- **Fix**: Add explicit `#[arg(long, help = "...")]` wrappers
- **Verification**: `cargo check --lib -p slapper`
- **Est. Time**: 2 hours

#### 5.5.3 Boolean Flag Handling
- **File**: `cli/http.rs`
- **Issue**: `pub json: bool` without wrapper
- **Fix**: Add `#[arg(long)]` wrapper to all boolean flags
- **Verification**: `cargo check --lib -p slapper`
- **Est. Time**: 1 hour

---

### 5.6 Configuration

#### 5.6.1 Environment Variable Override
- **File**: `config/loader.rs`
- **Issue**: Env var override documented but not implemented
- **Fix**:
  ```rust
  pub fn load_config(config_path: Option<&str>) -> Result<SlapperConfig> {
      let mut config = load_from_file(config_path)?;
      if let Ok(timeout) = std::env::var("SLAPPER_HTTP_TIMEOUT") {
          config.http.timeout_secs = timeout.parse()?;
      }
      Ok(config)
  }
  ```
- **Verification**: `SLAPPER_HTTP_TIMEOUT=60 cargo test --lib -p slapper -- config`
- **Est. Time**: 2 hours

#### 5.6.2 Config Migration System
- **File**: `config/settings.rs`
- **Issue**: No config version field for migration
- **Fix**: Add version field and migration handlers
- **Verification**: `cargo test --lib -p slapper -- config`
- **Est. Time**: 3 hours

---

### 5.7 AI Cache Optimization

#### 5.7.1 AI Cache Serialization Bug
- **File**: `ai/cache.rs:90-127`
- **Issue**: `AiCacheSerialized` loses all entries - `persist_path` not serialized
- **Fix**:
  ```rust
  impl From<AiCache> for AiCacheSerialized {
      fn from(cache: AiCache) -> Self {
          Self {
              // ... other fields
              persist_path: cache.persist_path.clone(),  // ADD THIS
          }
      }
  }
  ```
- **Verification**: `cargo test --lib -p slapper -- ai::cache`
- **Est. Time**: 30 minutes

#### 5.7.2 AI Cache Write Batching
- **File**: `ai/cache.rs:180`
- **Issue**: `persist().await` called on every `set()` - disk I/O bottleneck
- **Fix**: Batch writes with debounce using `last_persist` atomic counter
- **Verification**: `cargo test --lib -p slapper -- ai::cache`
- **Est. Time**: 2 hours

---

**Wave 5 Total Estimated**: ~100+ hours (Agent: ~18 weeks, Plugin: ~8 weeks, Fuzzer/WAF/CLI: ~20 hours)

---

## WAVE 6: New Capabilities

### 6.1 Exploitation Framework (Priority 1)
- **New Directory**: `src/exploit/`, `src/shellcode/`, `src/session/`, `src/pivot/`, `src/payload/`, `src/post_ex/`
- **Core Traits**:
  ```rust
  pub trait ExploitModule: Send + Sync {
      fn info(&self) -> &ExploitInfo;
      fn check(&self, target: &Target) -> Result<CheckResult>;
      fn exploit(&self, target: &Target, context: &ExploitContext) -> Result<ExploitResult>;
  }

  pub trait Session: Send + Sync {
      fn id(&self) -> SessionId;
      fn session_type(&self) -> SessionType;
      fn execute(&self, command: &str) -> Result<CommandOutput>;
      fn upload(&self, data: &[u8], path: &Path) -> Result<()>;
      fn download(&self, path: &Path) -> Result<Vec<u8>>;
  }
  ```
- **Timeline**: 6-9 months

### 6.2 Hash Cracking Integration (Priority 1)
- **File**: `src/cracking/mod.rs`
- **Tools**: hashcat/John the Ripper integration
- **Supported Types**: MD5 (0), SHA1 (100), SHA256 (1400), NTLM (1000), LM (3000), Kerberos (18200)
- **Timeline**: 2-3 months

### 6.3 AD/LDAP Attack Toolkit (Priority 1)
- **File**: `src/ad/mod.rs`
- **Attacks**: AS-REP Roasting, Kerberoasting, Golden Ticket, NTLM Relay
- **Timeline**: 3-4 months

### 6.4 Network MITM Capabilities (Priority 1)
- **File**: `src/mitm/mod.rs`
- **Components**: ArpSpoof, DnsSpoof, SslStrip
- **Timeline**: 2-3 months

### 6.5 Cloud Security Enhancement (Priority 2)
- **Files**: `src/recon/cloud/aws.rs`, `src/recon/cloud/azure.rs`, `src/recon/cloud/gcp.rs`
- **Capabilities**: S3 bucket enumeration, IAM user discovery, metadata endpoint testing
- **Timeline**: 3-4 months

### 6.6 External Tool Integration (Priority 2)
- **Tools**: SQLmap, Nuclei, Exploit-DB
- **Integration**: API + local CSV lookup
- **Timeline**: 1-2 months

### 6.7 Enhanced GUI (Priority 2)
- **New Crate**: `slapper-web` (React + TypeScript)
- **Stack**: React 18+, TypeScript, Zustand, Tailwind CSS, Socket.io-client, Recharts
- **Timeline**: 6-9 months

### 6.8 Mobile Security (Priority 2)
- **File**: `src/mobile/mod.rs`
- **Capabilities**: APK analysis, Frida instrumentation, SSL pinning bypass
- **Timeline**: 3-4 months

### 6.9 Enhanced Reporting (Priority 2)
- **Missing**: PDF generation, compliance templates (PCI-DSS, HIPAA, SOC2)
- **Options**: printpdf (pure Rust), latex + tectonic, html2pdf
- **Timeline**: 1-2 months

### 6.10 Session Handling UI (Priority 3)
- **New TUI tab**: "Cookies"
- **Features**: Cookie table with add/edit/delete, session rules UI
- **Timeline**: 1-2 months

### 6.11 Plugin Marketplace (Priority 3)
- **Features**: Community templates (20+), signing key infrastructure, version tracking
- **Timeline**: 3-6 months

---

**Wave 6 Total Estimated**: 12+ months

---

## WAVE 7: Documentation

### 7.1 README.md Updates (Phase 1)

#### 7.1.1 Fix Critical Numbers

| Area | Current | Correct |
|------|---------|---------|
| WAF Detection | 26 | 34 |
| Payload Types | 20 | 30 |
| Pipeline Profiles | 11 | 13 |

#### 7.1.2 Transform Core Features Table

Add "When to Use" column:

| Category | When to Use |
|----------|-------------|
| Reconnaissance | Gather target intelligence before testing |
| Web Security | Test for injection vulnerabilities |
| API Security | Test GraphQL, JWT, OAuth endpoints |
| Scanning | Find open ports and services |
| WAF | Detect and bypass web application firewalls |
| Load Testing | Measure performance under load |
| Stress Testing | Test resilience to DoS (requires explicit permission) |

#### 7.1.3 Add Missing Features
- `plan` command: Preview execution without running
- `ci` command: CI/CD integration with exit codes
- TUI capabilities (29 tabs)
- Wireless security testing

---

### 7.2 CAPABILITIES.md Updates (Phase 2)

#### 7.2.1 Fix Enumeration Numbers
- Fuzzing Payload Types: 24 → 30
- Reconnaissance Modules: 18 → 31

#### 7.2.2 Document TUI Capabilities (29 tabs)

| Tab # | Name | CLI Equivalent | Feature Gate |
|-------|------|---------------|--------------|
| 0 | Recon | slapper recon | - |
| 1 | Load | slapper load | - |
| 2 | ScanPorts | slapper scan-ports | - |
| 3 | ScanEndpoints | slapper scan-endpoints | - |
| 4 | Fingerprint | slapper fingerprint | - |
| 5 | Fuzz | slapper fuzz | - |
| 6 | Waf | slapper waf | - |
| 7 | WafStress | slapper waf-stress | - |
| 8 | Scan | slapper scan | - |
| 9 | Resume | slapper resume | - |
| 10 | Proxy | slapper proxy | - |
| 11 | Packet | slapper packet | - |
| 12 | GraphQl | slapper graphql | - |
| 13 | OAuth | slapper oauth | - |
| 14 | Cluster | slapper cluster | - |
| 15 | Stress | slapper stress | - |
| 16 | Report | slapper report | - |
| 17 | Nse | slapper nse | nse |
| 18 | Plugin | slapper plugin | python-plugins/ruby-plugins |
| 19 | Settings | - | - |
| 20 | History | - | - |
| 21 | Dashboard | - | - |
| 22 | Hunt | slapper hunt | advanced-hunting |
| 23 | Browser | slapper browser | headless-browser |
| 24 | Compliance | slapper compliance | compliance |
| 25 | Storage | slapper storage | database |
| 26 | Integrations | slapper integrations | external-integrations |
| 27 | Workflow | slapper workflow | finding-workflow |
| 28 | Vuln | slapper vuln | vuln-management |

#### 7.2.3 Add Missing Recon Modules
- git_secrets, api_schema, containers, ssl_audit
- ssh_auth, smtp_auth, ftp_auth
- takeover, email_security

---

### 7.3 ARCHITECTURE.md Updates (Phase 3)

#### 7.3.1 Remove Fictional Composite Features
Remove lines 58-103 listing non-existent features:
- api-integration, ai-capabilities, devsecops, network-analysis
- app-testing, cloud-security, nse-scripting, security-research

#### 7.3.2 Fix MCP Reference
Update: "MCP Server" requires "mcp-server feature" → MCP integrated into `rest-api`

#### 7.3.3 Update Module Map
Add missing modules: wireless, ai, agent, tool/agents

---

**Wave 7 Total Estimated**: ~8 hours

---

## False Positives / Already Fixed (Verified - Do Not Address)

These items are NOT bugs - the plan was incorrect or the issue has been fixed:

| Item | Claim | Actual State |
|------|-------|--------------|
| 1.1.1 k8s-openapi | full feature doesn't compile | Already fixed - version is 0.22, `container` feature works |
| 1.2.4 SSRF/Private IP | No blocking in executor | Partially protected via template loader; investigate further if needed |
| 4.3.3 Intercept TLS | No TLS validation | NOT A BUG - by design (intercepting proxy inspects decrypted traffic) |
| D.7 | HistoryTab search missing | Method EXISTS at `tui/tabs/history.rs` |
| D.8 | SettingsTab progress incorrect | 0.0 is CORRECT (no async work) |
| E.6 | Auto-Calibration missing | Already implemented in `fuzzer/calibration.rs` |
| E.8 | Templates missing | `scanner/templates/` EXISTS with full implementation |
| E.4.2 | AST vs regex | Current regex implementation is intentional |
| C.8 | CircuitBreaker atomic reset | Verified FALSE POSITIVE - atomic stores ARE inside lock scope |
| Execution clone count | 13 clones | Actually **12 clones** (off-by-one) |

---

## Deferred Items (Lower Priority)

These items are deferred due to complexity, lower priority, or lack of clear direction:

| Wave | Item | Reason Deferred |
|------|------|-----------------|
| C | C.1 Clone Storm Fix | Normal async pattern |
| C | C.2 FxHashMap Migration | Lower priority (4 modules) |
| D | D.1 UTF-8 Cursor Position Bug | Complex refactoring |
| D | D.2 Hardcoded Colors | Many files, lower priority |
| D | D.5 Missing Keyboard Shortcuts | Unclear what is needed |
| D | D.9 Validation Feedback | Feature not implemented |
| E | E.1 gRPC Implementation | Already in Wave 1 |
| E | E.3 Empty Feature Consolidation | No candidates found |
| E | E.5 PDF Pagination Fix | Low priority |
| F | F.4 Skills Standardization | Pending review |
| B | B.4 Remove Dead Code | ParsedDependency, is_input_focused - low priority |

---

## Pre-existing Test Failures (7 total)

These failures are pre-existing and will be addressed separately:

| # | Test | Issue |
|---|------|-------|
| 1 | `ai::client::tests::test_extract_content_valid_response` | Line count (expects 3, gets 4) |
| 2 | `ai::planner::tests::test_parse_modifications_from_text_add_stage` | Keyword extraction |
| 3 | `ai::planner::tests::test_parse_modifications_from_text_reduce_duration` | Keyword matching |
| 4 | `ai::planner::tests::test_parse_modifications_multiple_types` | Keyword matching |
| 5 | `ai::planner::tests::test_planner_cache_clear` | Cache behavior |
| 6 | `ai::planner::tests::test_record_outcome_updates_success_rate` | Cache entry creation |
| 7 | `ai::waf_bypass::tests::test_record_success_adds_to_knowledge_base` | Knowledge base state |

---

## Verification Commands

```bash
# Base compilation
cargo check --lib -p slapper

# Full features (with AI)
cargo check --lib -p slapper --features "rest-api,ai-integration"

# Full feature with k8s (requires version env var)
K8S_OPENAPI_ENABLED_VERSION=1.50 cargo check --lib -p slapper --features full

# Tests
cargo test --lib -p slapper
cargo test --lib -p slapper -- --list 2>/dev/null | grep -c "test$"  # Should be 1115

# With full features
cargo test --lib -p slapper --features "rest-api,ai-integration"

# Clippy
cargo clippy --lib -p slapper

# Module-specific tests
cargo test --lib -p slapper -- fuzzer
cargo test --lib -p slapper -- stress
cargo test --lib -p slapper -- tui
cargo test --lib -p slapper -- agent
cargo test --lib -p slapper -- mcp

# Plugin tests
cargo check --lib -p slapper --features python-plugins,ruby-plugins
cargo test --lib -p slapper-plugin
cargo test --lib -p slapper-ruby

# Count verification commands
grep -c "PayloadType::" crates/slapper/src/fuzzer/payloads/mod.rs  # Should be 30
grep -c "signatures.insert" crates/slapper/src/waf/data/patterns.rs  # Should be 34
rg "std::collections::HashMap" crates/slapper/src | wc -l  # Should reduce from 131

# TUI tab count
rg "pub enum Tab" crates/slapper/src/tui/tabs/mod.rs

# CLI commands
./target/release/slapper plan --help
./target/release/slapper ci --help
```

---

## Parallelization Strategy

**Independent items** (can run in parallel):
- Wave 1: 1.1, 1.2.2, 1.2.3, 1.2.4, 1.4 (all independent)
- Wave 2: 2.1 (dead code), 2.2 (error handling), 2.3 (refactoring) can parallelize
- Wave 3: 3.1 (quick wins), 3.2.2 (TUI), 3.4 (ReDoS) can parallelize
- Wave 4: 4.1 (LoadTest), 4.2 (AuthTab), 4.3 (H5-H7), 4.4 (Formula) can parallelize
- Wave 5: 5.3 (Fuzzer), 5.4 (WAF), 5.5 (CLI), 5.6 (Config), 5.7 (AI Cache) can parallelize
- Wave 7: All phases independent

**Sequential dependencies**:
- Wave 1: 1.3 (gRPC) depends on 1.1 (compile)
- Wave 5: 5.7.2 (AI Cache batching) depends on 5.7.1 (serialization fix)
- Wave 5: 5.6.2 (Config migration) depends on 5.7.1 (AI Cache serialization)
- Wave 6: All phases sequential within each capability

---

## Dependencies Summary

| Item | Depends On | Notes |
|------|-----------|-------|
| 1.3 (gRPC) | 1.1.1 (compile) | Requires full build |
| 5.7.2 (AI batching) | 5.7.1 (serialization) | Config references |
| 5.6.2 (Config migration) | 5.7.1 (serialization) | Version field |

---

## Implementation Order

### Immediate (This Week)
1. Wave 1: All 8 items (~8 hours)
2. Wave 7: Phase 1 README updates (~2 hours)

### Short-term (2-4 weeks)
1. Wave 2: Code Quality items (~16-20 hours)
2. Wave 3: Performance items (~8-12 hours)
3. Wave 4: TUI Improvements (~8 hours)
4. Wave 7: Phase 2-3 doc updates (~6 hours)

### Medium-term (1-3 months)
1. Wave 5: Agent Enhancement Phase 1-2
2. Wave 5: Plugin System P1-P3
3. Wave 5: Fuzzer/WAF/CLI/Config improvements

### Long-term (3-18 months)
1. Wave 5: Agent Enhancement Phase 3-4
2. Wave 6: New Capabilities (exploitation, cloud, mobile)

---

*Last updated: 2026-04-29*
*Source: plan.md (122 lines) + consolidated_1.md (916 lines) + consolidated_2.md (611 lines)*
*Total source: 1,649 lines across 3 documents*
*Status: CONSOLIDATED - Ready for implementation*