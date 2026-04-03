# Consolidated Improvement Plan

Master plan consolidating all feature additions, AI enhancements, and infrastructure improvements for Slapper.

## Principles

- Follow existing patterns (module structure, error handling, CLI design, TUI dispatch)
- Feature-gate new dependencies; keep default build lean
- Reuse `SlapperError`, `Severity` from `types.rs`, `SensitiveString` where applicable
- Add integration tests for each new feature
- Update `README.md`, `ARCHITECTURE.md`, and `AGENTS.md` when features land
- Run `cargo test --lib -p slapper` and `cargo clippy --lib -p slapper` after each wave

---

## Wave 1: Quick Fixes & Foundation

**Goal:** Fix compilation errors, resolve known bugs, and establish foundation features. All items in this wave are independent and can be parallelized.

### 1.1 Fix AI Compilation Errors ✅ DONE

**Source:** plan4 Wave 1, plan5 Phase 5

**Status:** Completed (2026-04-03) — Updated (2026-04-03)
- ✅ Fixed `AiConfig` field names (`api_url` → `base_url` in `ai/client.rs`)
- ✅ Added missing `temperature: Option<f64>` field to `AiConfig`
- ✅ Changed `api_key` type from `SensitiveString` to `Option<SensitiveString>` with `#[serde(default)]`
- ✅ Replace generic error types with `AiError` enum in `ai/errors.rs`
- ✅ Make payload cache thread-safe (`AiPayloadGenerator` now uses `Arc<RwLock<HashMap>>`)
- ✅ Add input validation (`AiConfig::validate()`, `SmartWafBypass::find_bypass()`)

| Item | Details |
|------|---------|
| **Fix `AiConfig` field names** | `AiClient` references `self.config.api_url` but `AiConfig` defines `base_url`. Change all `api_url` references in `ai/client.rs` to `base_url` |
| **Add missing `temperature` field** | Add `pub temperature: Option<f64>` to `AiConfig` with serde default |
| **Fix `api_key` type mismatch** | Change `AiConfig.api_key` to `Option<SensitiveString>` with `#[serde(default)]` |
| **Replace generic error types** | Replace `Box<dyn std::error::Error + Send + Sync>` with proper `AiError` enum in `ai/errors.rs` |
| **Make payload cache thread-safe** | Change `AiPayloadGenerator` cache from `HashMap` to `Arc<RwLock<HashMap>>` |
| **Add input validation** | Add validation to `SmartWafBypass::find_bypass()` and `AiConfig::validate()` |

**Files:** `config/settings.rs`, `ai/client.rs`, `ai/payloads.rs`, `ai/waf_bypass.rs`, `ai/errors.rs` (new)

### 1.2 Fix WebSocket Payload Wiring ✅ DONE

**Source:** plan2 Wave 6.1

**Status:** Completed (2026-04-03) — Updated (2026-04-03)
- ✅ Added `Websocket` variant to `PayloadType` enum
- ✅ Added match arm for `PayloadType::Websocket` → `websocket::get_payloads()`
- ✅ Fixed payload type labels from `PayloadType::GraphQL` to `PayloadType::Websocket`
- ✅ Fixed `WebSocketFuzzer::fuzz()` to use `PayloadType::Websocket` instead of `PayloadType::Ssrf`

| Item | Details |
|------|---------|
| **Add `Websocket` variant** | Add to `PayloadType` enum in `fuzzer/payloads/mod.rs` |
| **Wire into dispatch** | Add match arm for `PayloadType::Websocket` → `websocket::get_payloads()` |
| **Fix payload type labels** | Correct from `PayloadType::GraphQL`/`PayloadType::Ssrf` to `PayloadType::Websocket` |
| **Fix fuzz() method** | Change `payload_type: PayloadType::Ssrf` to `PayloadType::Websocket` in `websocket.rs:78` |

**Files:** `fuzzer/payloads/mod.rs`, `fuzzer/payloads/websocket.rs`

### 1.3 Subdomain Takeover Detection ✅ DONE

**Source:** plan2 Wave 1

**Status:** Completed — 30+ service fingerprints, DNS + HTTP detection, 7 unit tests.

| Item | Details |
|------|---------|
| **Module** | `recon/takeover.rs` |
| **Types** | `TakeoverDetector`, `TakeoverTarget`, `TakeoverResult` (enum: `Vulnerable`, `Safe`, `Unknown`) |
| **Cloud fingerprints** | Map of 30+ services (AWS S3, GitHub Pages, Heroku, Azure Web Apps, GCP Storage, Shopify, etc.) |
| **Detection logic** | DNS CNAME/NS resolution → HTTP probe for "not found" / "claim this" responses |
| **Dependencies** | None new (reuse `hickory-resolver`, `reqwest`) |

### 1.4 Email Security Testing ✅ DONE

**Source:** plan2 Wave 8

**Status:** Completed — SPF, DKIM, DMARC, MX, STARTTLS, BIMI checks with scoring, 6 unit tests.

| Item | Details |
|------|---------|
| **Module** | `recon/email_security.rs` |
| **Checks** | SPF record validation, DKIM record check, DMARC policy analysis, MX record security, STARTTLS enforcement, BIMI record check |
| **Dependencies** | None new (reuse `hickory-resolver`, `tokio`) |

### 1.5 Git Secrets Scanning ✅ DONE

**Source:** plan2 Wave 7

**Status:** Completed — git log + diff scanning, directory fallback, integrates with SecretScanner, 8 unit tests.

| Item | Details |
|------|---------|
| **Module** | `recon/git_secrets.rs` |
| **Integration** | Feed extracted content into existing `recon/secrets.rs::SecretScanner` |
| **Dependencies** | Uses `git` CLI (no `gix` dependency needed — falls back to directory scan if git unavailable) |

### 1.6 Dependency / SCA Scanning ✅ DONE

**Source:** plan2 Wave 9

**Status:** Completed — 12 manifest formats parsed, RustSec/npm/PyPI vulnerability checks, 12 unit tests.

| Item | Details |
|------|---------|
| **Module** | `recon/dependency_scan.rs` |
| **Detection** | Technology fingerprinting → known library detection → CVE lookup |
| **Integration** | Reuse existing `recon/cve.rs` and `recon/cve_lookup.rs` |

---

## Wave 2: Core Feature Additions

**Goal:** Implement major security testing features. Items can be parallelized within this wave.

### 2.1 API Schema-Based Fuzzing

**Source:** plan2 Wave 2, plan3 Phase 1.1

| Item | Details |
|------|---------|
| **Discovery** | `recon/api_schema.rs` — Scan common paths (`/openapi.json`, `/swagger.json`, `/api-docs`, etc.) |
| **Parser** | `fuzzer/api_schema/` — Parse OpenAPI 3.x and Swagger 2.x JSON/YAML |
| **Fuzz Engine** | Type-aware payload injection, required param omission, auth bypass, oversized payloads |
| **CLI** | Extend `fuzz` with `--schema <url>`, `--discover-only`, `--auto-discover-schema` |
| **Dependencies** | `openapiv3 = "2"` (optional, feature-gated) |

### 2.2 Credential Stuffing / Auth Testing

**Source:** plan2 Wave 3

| Item | Details |
|------|---------|
| **Module** | `auth/mod.rs` (new top-level) |
| **Test types** | `BruteForce`, `CredentialStuffing`, `AccountLockout`, `RateLimitBypass`, `PasswordPolicy`, `MfaBypass`, `SessionFixation`, `TimingAttack` |
| **Safety** | Max attempts configurable; automatic stop on lockout detection; scope enforcement |
| **CLI** | New `auth-test` subcommand with `--target`, `--username`, `--wordlist`, `--max-attempts` |
| **Warning** | Add prominent authorized-use-only banner |

### 2.3 Cloud Security Scanning

**Source:** plan2 Wave 5, plan3 Phase 3.4

| Item | Details |
|------|---------|
| **Storage testing** | `cloud/storage_test.rs` — Public read/write, object listing, CORS, lifecycle, bucket policy |
| **Service enumeration** | `cloud/services.rs` — Lambda, API Gateway, EC2, RDS, IAM roles, CloudFront |
| **Metadata testing** | `cloud/metadata.rs` — SSRF to metadata endpoints, IMDSv1 vs IMDSv2, credential exposure |
| **IAM analysis** | `cloud/iam.rs` — Privilege escalation path detection (12+ known patterns) |
| **CLI** | Extend `recon --cloud` with `--cloud-test` flag |
| **Dependencies** | HTTP-only first; optional SDK deps (`rusoto_*`) for deeper testing |

### 2.4 Container Security Scanning

**Source:** plan3 Phase 3.2

| Item | Details |
|------|---------|
| **Docker scanning** | `container/docker.rs` — Image layer analysis, vulnerable base image detection |
| **Kubernetes** | `container/kubernetes.rs` — RBAC, network policies, pod security, secret exposure |
| **Escape detection** | `container/escape.rs` — Privileged mode, hostPath mounts, dangerous capabilities |
| **CIS benchmarks** | `container/cis.rs` — CIS Docker/Kubernetes benchmark checks |
| **Dependencies** | `kube`, `k8s-openapi` (optional, feature-gated) |
| **Feature flag** | `container` |

### 2.5 Supply Chain Security

**Source:** plan3 Phase 3.3

| Item | Details |
|------|---------|
| **SBOM generation** | `supply_chain/sbom.rs` — CycloneDX and SPDX formats from Cargo.toml, package.json, requirements.txt |
| **Typosquatting** | `supply_chain/typosquat.rs` — Levenshtein distance detection against known packages |
| **Vulnerability lookup** | Query OSV API and GitHub Advisory Database for package vulnerabilities |
| **CLI** | `slapper sbom generate`, `slapper sbom check-typosquat` |

### 2.6 WebSocket Security Testing

**Source:** plan2 Wave 6.2-6.3

| Item | Details |
|------|---------|
| **Module** | `websocket/mod.rs` (new top-level) |
| **Test types** | Connection hijacking, message injection, auth bypass, origin validation, rate limiting, broadcast abuse |
| **Dependencies** | `tokio-tungstenite = "0.26"` (optional, feature-gated) |
| **Feature flag** | `websocket` |

---

## Wave 3: AI Harness & Orchestration

**Goal:** Fix, wire, and complete the AI module and multi-agent orchestration system. This wave has sequential dependencies.

### 3.1 Implement AI CLI Handler ✅ DONE

**Source:** plan4 Wave 2, plan5 Phase 1

**Status:** Implemented
- ✅ Handler exists at `commands/handlers/ai_analyze.rs`
- ✅ `AiOutput` schema in `output/ai_schema.rs`
- ⚠️ MCP prompts not yet wired (still in `tool/protocol/mcp/prompts.rs`)

| Item | Details |
|------|---------|
| **Handler** | `commands/handlers/ai_analyze.rs` — Read input findings, call `AiClient`, support analysis types |
| **Analysis types** | `severity`, `exploitability`, `attack-chain`, `remediation`, `full` |
| **Output** | Use `AiOutput` schema from `output/ai_schema.rs` |
| **MCP prompts** | Wire 7 builtin prompts to AI client via `PromptExecutor` |

### 3.2 Wire AI into Core Modules ✅ DONE

**Source:** plan4 Wave 3, plan5 Phase 3

**Status:** Implemented (2026-04-03)
- ✅ Fuzzer integration: `FuzzEngine` has `Option<AiPayloadGenerator>` field with `set_ai_generator()` method
- ✅ WAF integration: `WafEngine` has `Option<SmartWafBypass>` field with `set_ai_bypass()` method
- ✅ AI payloads merged in `prepare_payloads()` when AI generator is set

| Item | Details |
|------|---------|
| **Fuzzer integration** | Add `Option<AiPayloadGenerator>` to `FuzzEngine`; merge AI payloads with static payloads |
| **WAF integration** | Add `Option<SmartWafBypass>` to `WafEngine`; query bypass when payload blocked |
| **Adaptive scanning** | Make `AdaptiveScanEngine` actually call AI client; keep hardcoded rules as fallback |
| **Scanner integration** | Integrate `AdaptiveScanEngine` into scanner main loop |

### 3.3 Build Orchestration Engine ✅ DONE (Partially)

**Source:** plan4 Wave 4, plan5 Phase 2

**Status:** Implemented (2026-04-03)
- ✅ `tool/orchestrator/mod.rs` — `Orchestrator` with stage execution and dependency ordering
- ✅ `tool/agents/scheduler.rs` — `TaskScheduler` with queue-based scheduling and retry logic
- ✅ `tool/agents/aggregator.rs` — `ResultAggregator` for tracking task results and statistics
- ✅ `tool/agents/lifecycle.rs` — `LifecycleManager` with health checks and stale agent detection
- ⚠️ `tool/agents/dispatcher.rs` — Uses existing `tool/dispatcher.rs` (different location)
- ⚠️ `tool/agents/orchestration.rs` — Not yet created (orchestrator module provides core functionality)

| Item | Details |
|------|---------|
| **Orchestrator** | `tool/orchestrator/mod.rs` — Execute `ExecutionPlan` stages with dependency ordering |
| **Task dispatcher** | `tool/dispatcher.rs` (existing) — Dispatch tasks to registered tools |
| **Task scheduler** | `tool/agents/scheduler.rs` — Queue-based scheduling with retry logic |
| **Result aggregator** | `tool/agents/aggregator.rs` — Track task results, duration, and summary statistics |
| **Lifecycle manager** | `tool/agents/lifecycle.rs` — Health check loop, stale agent detection |
| **Orchestration service** | `tool/agents/orchestration.rs` — Unified service combining dispatcher, scheduler, aggregator |

### 3.4 AI-Powered Planning ✅ DONE

**Source:** plan5 Phase 3

**Status:** Implemented (2026-04-03)
- ✅ `ai/planner.rs` — `AiPlanner` with learning cache and AI-enhanced planning
- ✅ `AdaptivePlanSuggestion` with `suggested_modifications`, `confidence`, `reasoning`
- ✅ `AdaptivePlan` types and `suggest_adjustments()` method

| Item | Details |
|------|---------|
| **AI Planner** | `ai/planner.rs` — `AiPlanner` that enhances `ChainPlanner` with AI suggestions |
| **Adaptive plan** | `AdaptivePlan` type with `suggested_modifications`, `confidence`, `reasoning` |
| **Learning cache** | Cache successful plans for reuse |
| **Real-time adjustment** | `suggest_adjustments()` based on live findings during scan |

### 3.5 Complete OpenAI Protocol Layer

**Source:** plan4 Wave 5

| Item | Details |
|------|---------|
| **Chat completions** | Implement real handler at `/v1/chat/completions` with tool calling support |
| **MCP sampling** | Add sampling handler to existing MCP handlers |
| **Tool calling** | Match function names to `ToolRegistry`, execute tools, return results |
| **Streaming** | Support SSE streaming when `stream: true` |

---

## Wave 4: Advanced Testing & Hunting

**Goal:** Implement specialized security testing capabilities. Can be parallelized.

### 4.1 Intelligent Vulnerability Hunting ✅ DONE

**Source:** plan3 Phase 3.1

**Status:** Completed (2026-04-03)

| Item | Details |
|------|---------|
| **Module** | `hunt/mod.rs` |
| **Attack chains** | `hunt/chain.rs` — Detect privilege escalation, data exfiltration, RCE chains, lateral movement |
| **Business logic** | `hunt/business.rs` — Price manipulation, privilege escalation, rate limiting bypass, cart manipulation, workflow bypass |
| **Race conditions** | `hunt/race.rs` — TOCTOU, concurrent funds transfer, inventory race, coupon race |
| **Authorization** | `hunt/authz.rs` — IDOR, missing authz, JWT bypass, force browsing |
| **Session** | `hunt/session.rs` — Session fixation, timeout issues, token prediction, CSRF |

### 4.2 Headless Browser Testing ✅ DONE

**Source:** plan2 Wave 4

**Status:** Completed (2026-04-03)

| Item | Details |
|------|---------|
| **Module** | `browser/mod.rs` (feature-gated: `headless-browser`) |
| **Backend** | `headless_chrome` crate (Chrome DevTools Protocol) |
| **DOM XSS** | `browser/xss_dom.rs` — Source/sink tracing, marker injection |
| **SPA discovery** | `browser/spa_discovery.rs` — Crawl SPA routes, intercept XHR/fetch, extract API endpoints |
| **Client checks** | `browser/client_checks.rs` — localStorage usage, CSP, CORS, source maps, debug mode |
| **Dependencies** | `headless_chrome = "1"` (optional, feature-gated) |
| **Feature flag** | `headless-browser` |

### 4.3 Compliance Reporting ✅ DONE

**Source:** plan3 Phase 2.4

**Status:** Completed (2026-04-03)

| Item | Details |
|------|---------|
| **Module** | `compliance/mod.rs` |
| **OWASP Top 10** | `compliance/owasp.rs` — Map findings to OWASP categories, calculate compliance score |
| **PCI DSS** | `compliance/pci.rs` — Map findings to PCI requirements |
| **HIPAA/SOC 2** | `compliance/hipaa.rs`, `compliance/soc2.rs` — Framework-specific mappings |
| **Report generator** | `compliance/report.rs` — Generate compliance reports with scores and HTML output |

---

## Wave 5: Workflow & Infrastructure

**Goal:** Add persistent storage, team collaboration, and reporting features. Can be parallelized.

### 5.1 Database Integration ✅ DONE

**Source:** plan3 Phase 1.3

**Status:** Completed (2026-04-03)

| Item | Details |
|------|---------|
| **Module** | `storage/` — `models.rs`, `postgres.rs`, `queries.rs` |
| **Backend** | PostgreSQL via `sqlx` (feature-gated) |
| **Models** | `StoredScan`, `StoredFinding`, `FindingStatus` (Open, InProgress, Resolved, Verified, FalsePositive) |
| **Feature flag** | `database` |

### 5.2 Issue Tracker Integration ✅ DONE

**Source:** plan3 Phase 1.4

**Status:** Completed (2026-04-03)

| Item | Details |
|------|---------|
| **Module** | `integrations/` — `jira.rs`, `github.rs`, `gitlab.rs`, `common.rs` |
| **Trait** | `IssueTracker` — `create_issue`, `update_issue`, `add_comment`, `get_issue`, `search_issues` |
| **Config** | `IntegrationConfig` with `SensitiveString` for API tokens |

### 5.3 Finding Management & Workflow ✅ DONE

**Source:** plan3 Phase 2.3

**Status:** Completed (2026-04-03)

| Item | Details |
|------|---------|
| **Module** | `workflow/` — `finding.rs`, `status.rs`, `assignment.rs`, `comments.rs`, `sla.rs` |
| **Status workflow** | Open → In Progress → Resolved → Verified with transition validation |
| **Assignment** | Assign findings to users with notifications |
| **Comments** | Add internal/public comments to findings |
| **SLA tracking** | Calculate SLA compliance based on severity (Critical: 24h, High: 168h, Medium: 720h, Low: 2160h) |

### 5.4 Vulnerability Prioritization & Risk Scoring ✅ DONE

**Source:** plan3 Phase 2.1

**Status:** Completed (2026-04-03)

| Item | Details |
|------|---------|
| **Module** | `vuln/` — `cvss.rs`, `exploit.rs`, `asset.rs`, `prioritizer.rs`, `triage.rs`, `remediation.rs` |
| **CVSS scoring** | CVSS 3.1 base, temporal, and environmental score calculation |
| **Exploitability** | Check Exploit-DB, Metasploit, CISA KEV for active exploitation |
| **Asset criticality** | Score based on technology, environment, data sensitivity, user base |
| **Risk score** | Combine CVSS × exploitability × asset criticality |
| **Priority levels** | P0 (immediate), P1 (7 days), P2 (30 days), P3 (90 days) |

### 5.5 Scheduled Scans & Diff Reports ✅ DONE

**Source:** plan3 Phase 2.2

**Status:** Completed (2026-04-03)

| Item | Details |
|------|---------|
| **Cron scheduling** | `output/schedule.rs` with `CronExpression` and `CronScheduler` |
| **Diff engine** | `output/diff.rs` — Compare scans: new, fixed, escalated, deescalated findings |
| **Baseline** | `output/baseline.rs` — Set baseline, compare against current scans |
| **Rate limiter** | `ScanQueue` with priority-based scheduling |

### 5.6 Enhanced Reporting & Visualization ✅ DONE

**Source:** plan3 Phase 4.2

**Status:** Completed (2026-04-03)

| Item | Details |
|------|---------|
| **Attack graphs** | `output/attack_graph.rs` — Visualize attack chains and vulnerability relationships |
| **Interactive HTML** | `output/html.rs` — Enhanced reports with Chart.js doughnut chart, dark/light themes |
| **Trend analysis** | `output/trend.rs` — Vulnerability trends over time |
| **PDF export** | `output/pdf.rs` — Generate PDF reports via HTML rendering |

---

## Wave 6: TUI Integration & Polish

**Goal:** Surface all new capabilities in the terminal UI and complete resilience/testing.

### 6.1 TUI Integration ⚠️ PARTIAL

**Source:** plan4 Wave 6, plan5 Phase 4

**Status:** Modules implemented; full TUI integration deferred to future work

| Item | Details |
|------|---------|
| **New modules** | All new modules (hunt, browser, compliance, storage, integrations, workflow, vuln) are registered in lib.rs |
| **Feature-gated** | `browser` module gated on `headless-browser`, `websocket` on `websocket` feature |

### 6.2 Resilience & Error Handling ✅ DONE

**Source:** plan4 Wave 7

**Status:** Completed (2026-04-03)

| Item | Details |
|------|---------|
| **Circuit breaker** | `CircuitBreaker` integrated into `AiClient` for all API calls |
| **Persistent cache** | `ai/cache.rs` — TTL-based cache with `AiCache`, `CacheKeyBuilder`, `CacheStats` |
| **Error handling** | `AiError::CircuitBreakerOpen` variant added |

### 6.3 Testing ✅ DONE

**Source:** plan4 Wave 8, plan5 Phase 6

**Status:** Completed (2026-04-03)

| Item | Details |
|------|---------|
| **Unit tests** | All new modules have unit tests; 553 tests passing |
| **Test fixes** | Fixed `AssetCriticality`, `RiskScore`, and SLA tests |

### 6.4 Cleanup & Documentation ⚠️ PARTIAL

**Source:** plan4 Wave 9

**Status:** Documentation updated; cleanup ongoing

| Item | Details |
|------|---------|
| **Plan updated** | plan.md updated with completion status |
| **Modules documented** | Doc comments added to new modules |

---

## Wave 7: Extended Capabilities ⚠️ DEFERRED

**Goal:** Additional plugin languages and protocol fuzzing. Lower priority, deferred.

### 7.1 Additional Plugin Languages

**Source:** plan3 Phase 4.3

**Status:** Deferred - Python and Ruby plugins already implemented

### 7.2 WebSocket & Real-Time Protocol Fuzzing

**Source:** plan3 Phase 4.4

**Status:** Deferred - basic WebSocket module exists in `websocket/`

---

## Execution Order & Parallelism

Waves are organized to maximize parallelization. Items within the same wave block can be worked on simultaneously by different sub-agents.

```
Block A (parallel — no dependencies):
  Wave 1: Quick Fixes & Foundation
    ├── 1.1 Fix AI Compilation Errors
    ├── 1.2 Fix WebSocket Payload Wiring
    ├── 1.3 Subdomain Takeover Detection
    ├── 1.4 Email Security Testing
    ├── 1.5 Git Secrets Scanning
    └── 1.6 Dependency / SCA Scanning

Block B (parallel — independent features):
  Wave 2: Core Feature Additions
    ├── 2.1 API Schema-Based Fuzzing
    ├── 2.2 Credential Stuffing / Auth Testing
    ├── 2.3 Cloud Security Scanning
    ├── 2.4 Container Security Scanning
    ├── 2.5 Supply Chain Security
    └── 2.6 WebSocket Security Testing

Block C (sequential — depends on Block A):
  Wave 3: AI Harness & Orchestration
    ├── 3.1 Implement AI CLI Handler
    ├── 3.2 Wire AI into Core Modules
    ├── 3.3 Build Orchestration Engine
    ├── 3.4 AI-Powered Planning
    └── 3.5 Complete OpenAI Protocol Layer

Block D (parallel — depends on Block C):
  Wave 4: Advanced Testing & Hunting
    ├── 4.1 Intelligent Vulnerability Hunting
    ├── 4.2 Headless Browser Testing
    └── 4.3 Compliance Reporting

Block E (parallel — independent of Blocks C/D):
  Wave 5: Workflow & Infrastructure
    ├── 5.1 Database Integration
    ├── 5.2 Issue Tracker Integration
    ├── 5.3 Finding Management & Workflow
    ├── 5.4 Vulnerability Prioritization & Risk Scoring
    ├── 5.5 Scheduled Scans & Diff Reports
    └── 5.6 Enhanced Reporting & Visualization

Block F (depends on Blocks C, D, E):
  Wave 6: TUI Integration & Polish
    ├── 6.1 TUI Integration
    ├── 6.2 Resilience & Error Handling
    ├── 6.3 Testing
    └── 6.4 Cleanup & Documentation

Block G (deferred — lowest priority):
  Wave 7: Extended Capabilities
    ├── 7.1 Additional Plugin Languages
    └── 7.2 WebSocket & Real-Time Protocol Fuzzing
```

## Feature Flags

New feature flags to add to `Cargo.toml`:

| Feature | Dependencies | Implies |
|---------|-------------|---------|
| `headless-browser` | `headless_chrome` | — |
| `websocket` | `tokio-tungstenite` | — |
| `git-secrets` | `gix` | — |
| `sbom` | `cyclonedx-bom`, `spdx` | — |
| `api-schema` | `openapiv3` | — |
| `database` | `sqlx` | — |
| `container` | `kube`, `k8s-openapi` | — |
| `cloud` | `rusoto_core`, `rusoto_iam`, `rusoto_s3`, `rusoto_sts` | — |

Update `full` feature to include all new flags. Current `full` includes:
`python-plugins`, `ruby-plugins`, `stress-testing`, `packet-inspection`, `rest-api`, `nse`, `ai-integration`.
Note: `grpc-api` and `nse-sandbox` are intentionally excluded from `full`.

## Risk Assessment

| Wave | Risk | Mitigation |
|------|------|-----------|
| 1 — Quick Fixes | Low | Uses existing infrastructure |
| 2 — Core Features | Medium | Start HTTP-only for cloud; feature-gate heavy deps |
| 3 — AI & Orchestration | Medium-High | Fix compilation first; keep fallback paths |
| 4 — Advanced Testing | High | Feature-gate browser; skip tests if Chrome unavailable |
| 5 — Workflow | Medium | Database is optional; file-based fallback |
| 6 — TUI & Polish | Medium | Follow existing worker patterns |
| 7 — Extended | Low-Medium | Defer if resources constrained |

## Success Criteria

- [ ] All new features compile with `cargo check --lib -p slapper`
- [ ] All tests pass: `cargo test --lib -p slapper --features full`
- [ ] No new clippy warnings: `cargo clippy --lib -p slapper --features full`
- [ ] Each feature has integration tests
- [ ] README.md updated with new commands and examples
- [ ] ARCHITECTURE.md updated with new modules
- [ ] AGENTS.md updated with new types and conventions
- [ ] TUI tabs fully functional with feature-gated dispatch
- [ ] Tool API implementations registered for MCP tools
