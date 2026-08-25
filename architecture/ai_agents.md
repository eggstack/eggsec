# AI & Agents Deep Dive

Eggsec's AI/agent subsystem spans five tightly coupled components: the AI client library for LLM-powered analysis, the engine-side autonomous agent, the extracted agent-coordination crate, protocol-neutral tool-core DTOs, and the tool registry that binds everything together through enforced dispatch.

```
┌─────────────────────────────────────────────────────────────────────┐
│                     Surfaces (MCP / REST / gRPC / CLI)              │
├─────────────────────────────────────────────────────────────────────┤
│                   EnforcedDispatcher::dispatch_checked()            │
│                  (ApprovedOperation token required)                 │
├──────────┬──────────┬──────────┬───────────────┬───────────────────┤
│   Tool   │   Tool   │  Agent   │   AI Client   │   MCP / REST /   │
│ Registry │Protocol/ │ Routes   │  (4 providers)│   gRPC servers   │
│ (FxHashMap│servers  │          │               │                  │
│  +RwLock)│          │          │               │                  │
├──────────┴──────────┴──────────┴───────────────┴───────────────────┤
│   eggsec-tool-core DTOs: ToolRequest / ToolResponse / ToolError   │
│   Finding / ExecutionHistory / RateLimitConfig / CancellationToken │
├─────────────────────────────────────────────────────────────────────┤
│              eggsec-agent: AgentRegistry / TaskScheduler           │
│              LifecycleManager / MultiAgentCoordinator               │
│              ResultAggregator                                       │
├─────────────────────────────────────────────────────────────────────┤
│                     eggsec-core (Severity, SensitiveString)         │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 1. AI Integration (`crates/eggsec/src/ai/`)

### Responsibilities

Multi-provider LLM client for security analysis: payload generation, WAF-bypass suggestions, finding reassessment, adaptive scan strategy selection, and (behind `ai-integration`) AI-driven execution planning and Python script generation.

### Gating

The entire `ai` module is feature-gated at `crates/eggsec/src/lib.rs:164-165`:

```rust
#[cfg(feature = "ai-integration")]
pub mod ai;
```

Within the module, two sub-modules carry additional per-item gating (`ai/mod.rs:6-9`):

```rust
#[cfg(feature = "ai-integration")]
mod planner;
#[cfg(feature = "ai-integration")]
mod script_gen;
```

All other sub-modules (`client`, `cache`, `errors`, `types`, `payloads`, `waf_bypass`, `adaptive`) are always compiled when `ai-integration` is enabled.

### Architecture

**File inventory (11 files):**

| File | Purpose | Key type(s) |
|------|---------|-------------|
| `mod.rs:1-25` | Module root; re-exports | — |
| `client.rs:8-14` | LLM provider abstraction | `Provider` (4 variants: `OpenAI`, `Azure`, `Anthropic`, `OpenAICompatible`), `AiClient` |
| `errors.rs:6-33` | Error domain | `AiError` (9 variants) |
| `types.rs:4-33` | Shared DTOs | `AiAnalysisResult`, `AiPayloadSuggestion`, `AiWafBypassSuggestion`, `ScanFinding` |
| `cache.rs:1-16` | TTL cache with disk persistence | `AiCache`, `CacheEntry`, `CacheKeyBuilder`, `CacheStats` |
| `payloads.rs` | AI payload generator | `AiPayloadGenerator` |
| `waf_bypass.rs:23-30` | WAF bypass knowledge base + AI suggestions | `SmartWafBypass`, `WafBypassEntry` |
| `adaptive.rs` | Adaptive scan strategy engine | `AdaptiveScanEngine` |
| `planner.rs` | AI execution planner (**feature-gated**) | `AiPlanner`, `PlanOutcome` |
| `script_gen.rs` | Python security script generator (**feature-gated**) | `ScriptGenerator`, `ScriptTarget`, `PluginLanguage`, `GeneratedScript`, `ScriptMetadata` |
| `AGENTS.override.md` | Module-specific guidance | — |

**Provider enum** (`client.rs:8-14`):

```rust
pub enum Provider { OpenAI, Azure, Anthropic, OpenAICompatible }
```

Provider detection (`client.rs:17-24`): `from_str()` maps lowercase strings; Azure accepts `"azure"`, `"azureopenai"`, `"azureopenai.com"`.

**AiError variants** (`errors.rs:6-33`) — 9 total:

| # | Variant | Source |
|---|---------|--------|
| 1 | `RequestFailed(String)` | `From<reqwest::Error>`, `From<std::io::Error>` |
| 2 | `MissingApiKey` | Auth check |
| 3 | `InvalidConfig(String)` | Validation |
| 4 | `ApiError(String)` | LLM response |
| 5 | `ParseError(String)` | Deserialization |
| 6 | `Timeout` | `reqwest::Error::is_timeout()` |
| 7 | `RateLimited` | HTTP 429 |
| 8 | `InvalidResponse` | Format mismatch |
| 9 | `CircuitBreakerOpen` | Breaker state |

### Flows

**LLM request flow:**

```
AiClient::chat_completion_from_messages()
  → has_required_auth()          (reject if missing key)
  → circuit_breaker.is_available() (reject if open)
  → build request with apply_auth() (Bearer or Azure header)
  → send with 60s timeout (client.rs:76)
  → circuit_breaker.record_success/record_failure()
  → normalize Anthropic response to OpenAI format
  → return serde_json::Value
```

**Circuit breaker** (configured at `client.rs:74`): 5 failures to open, 3 successes in half-open to close, 60-second reset timeout. Implemented in `utils/circuit_breaker.rs`.

**CacheKeyBuilder convention** — used for collision-free cache keys across payloads, WAF bypass, and planner modules. Always construct via `CacheKeyBuilder::for_payload_suggestion(...)` or `CacheKeyBuilder::for_waf_bypass(...)`, never manual string concatenation.

**SmartWafBypass flow** (`waf_bypass.rs:23-30`):

```
find_bypass(waf_name, payload)
  → check knowledge_base for cached bypass (skip entries with failed_attempts >= 3)
  → if no cached bypass: query AI via client.chat_completion_from_messages()
  → record result in knowledge_base
  → evict_knowledge_base_if_needed() before insert
  → persist to waf_bypasses.json
```

Default capacity: 1000 knowledge-base entries (`waf_bypass.rs:29`). Constructor `with_config(client, max_bypasses)` is configurable.

### Integration Points

- **Engine modules**: `waf/` uses `SmartWafBypass` for bypass suggestions; `fuzzer/` uses `AiPayloadGenerator` for context-aware payloads.
- **Agent** (`agent/mod.rs:197-198`): `#[cfg(feature = "ai-integration")] ai_client: Option<AiClient>` — the agent optionally holds an AI client for adaptive scan decisions.
- **Agent skills** (`agent/skills.rs`): feature-gated `ai-integration` — loads discrete capabilities for AI assistants.
- **REST AI routes** (`protocol/ai_routes.rs:7-8`): AI endpoint state holds `Option<AiClient>` behind `ai-integration`.
- **Adaptive engine**: `AdaptiveScanEngine` wraps `Option<AiClient>` and falls back to severity-based heuristics when AI is unavailable.

### Testing

```bash
cargo test --lib -p eggsec ai::
```

### Gotchas

- `chat_completion()` is **private** — external callers must use `chat_completion_from_messages()` (`client.rs:154-168`).
- `Provider::from_str()` never fails; unknown strings become `OpenAICompatible` (`client.rs:22-23`).
- Azure provider **requires** `base_url` or construction fails with `AiError::InvalidConfig` (`client.rs:69-73`).
- Anthropic responses are normalized to OpenAI format; original lives under `provider_response`.

---

## 2. Agent Orchestration (`crates/eggsec/src/agent/`)

### Responsibilities

Engine-side autonomous security agent: event-driven polling loop, scheduled scan dispatch, enforcement-context validation (must be `AgentStrict`), longitudinal memory, target portfolio, constraint checking, alert routing to channels (Slack, PagerDuty, email, webhook), and config hot-reload.

### Gating

The entire `agent` module is feature-gated at `crates/eggsec/src/lib.rs:167-168`:

```rust
#[cfg(feature = "rest-api")]
pub mod agent;
```

Within `agent/mod.rs:19-20` and `agent/mod.rs:52-53`, the `skills` sub-module and `AiClient` import carry additional `ai-integration` gating:

```rust
#[cfg(feature = "ai-integration")]
pub mod skills;

#[cfg(feature = "ai-integration")]
use crate::ai::AiClient;
```

### Architecture

**File inventory (12 entries):**

| File | Purpose |
|------|---------|
| `mod.rs:1-3575` | Agent runtime, config, polling loop, `Agent::new()` requires `AgentStrict` enforcement |
| `alerts/` | Alert routing, aggregation, channel delivery (Slack, PagerDuty, email, webhook) |
| `channels.rs` | Channel implementations (`WebhookConfig`, `SlackTemplate`, `PagerDutyTemplate`, etc.) |
| `constraints.rs` / `constraints/` | `ConstraintChecker`: `evaluate_action()`, `evaluate_target()`, `evaluate_scan_depth()`, `evaluate_rate_limit()`, `evaluate_payload()`, `evaluate_off_peak()`, `evaluate_approval()`, `evaluate_all()` |
| `enforcement.rs:1-307` | Per-scan enforcement helpers: `risk_for_agent_scan_depth()`, `capabilities_for_agent_scan()`, `operation_descriptor_for_agent_scan()` |
| `events.rs` | `EventHandler` trait, `SecurityEvent` types |
| `memory.rs` | `LongitudinalMemory`: baseline-aware finding comparisons, target lock tracking |
| `portfolio.rs` | `TargetPortfolio`: target configs, schedules, scan history, `Priority`, `ScanRecord`, `ScanDepth` |
| `skills.rs` | `Skill`, `SkillRegistry`, `SkillLoader` (**feature-gated `ai-integration`**) |
| `config_watcher.rs` | `ConfigWatcher`, `ConfigReloader`, `EggsecConfigReloader` |
| `AGENTS.override.md` | Module-specific guidance |

**Key types** (`agent/mod.rs:119-219`):

- `AgentConfig` — portfolio path, memory dir, poll interval, AI config, operational constraints, enforcement context.
- `AgentRuntimeStatus` — runtime status reportable via `agent status` (14 fields: running, started_at, scans_completed/failed, alerts_sent, last_preflight_denial, etc.).
- `AgentRuntimePersisted` — persisted metadata written to disk at start/scan/shutdown.
- `AgentPreflightDenial` — recorded enforcement denial with operation, target, timestamp, reasons.
- `Agent` struct (`mod.rs:190-219`) — holds `ToolRegistry`, `ConstraintScanner`, `EnforcedDispatcher`, optional `AiClient`, `CronScheduler`, `TargetPortfolio`, `LongitudinalMemory`, `AlertRouter`, event handlers, runtime status counters.

### Flows

**Agent startup** (`Agent::new()` at `mod.rs:222-299`):

```
Agent::new(config)
  → validate enforcement context is AgentStrict (rejects ManualPermissive/ManualGuarded)
  → create_default_registry()
  → ToolDispatcher + EnforcedDispatcher
  → load TargetPortfolio from disk (or empty)
  → load LongitudinalMemory, warm cache
  → load AlertRouter, register channels from EggsecConfig
```

**Scan execution** (single pass via `Agent::run_once()`):

```
run_once()
  → poll portfolio for due targets
  → for each target:
      → ConstraintChecker::evaluate_all()
      → preflight_operation() via EnforcementContext
      → risk_for_agent_scan_depth() + capabilities_for_agent_scan()
      → operation_descriptor_for_agent_scan()
      → dispatch via EnforcedDispatcher::dispatch_checked()
      → record_policy_denial() on failure
  → update memory
  → send alerts via AlertRouter
```

### Integration Points

- **AI module**: Optional `AiClient` for adaptive scan decisions (`agent/mod.rs:197-198`).
- **Tool registry**: Creates its own `create_default_registry()` instance (`agent/mod.rs:239`).
- **Enforcement**: Requires `AgentStrict` profile; per-scan enforcement via `enforcement.rs` helpers.
- **REST API**: Agent routes (`protocol/agent_routes.rs`) expose agent/task CRUD over HTTP.

### Testing

```bash
cargo test --lib -p eggsec agent::
```

### Gotchas

- `Agent::new()` **panics** if `config.enforcement` is `None` or not `AgentStrict` (`mod.rs:226-237`). Use `Agent::new_for_test()` for test construction.
- `config_watcher` field is `#[allow(dead_code)]` (`mod.rs:207`) — hot-reload is wired but not yet consumed by the polling loop.
- `memory.warm_cache().await.ok()` silently ignores warm-cache errors (`mod.rs:252`).

---

## 3. Agent Coordination Crate (`crates/eggsec-agent/`)

### Responsibilities

Standalone agent coordination primitives extracted from the engine. Provides registry, scheduling, lifecycle management, inter-agent communication, task delegation, and result aggregation. Designed for multi-agent topologies where agents discover each other, delegate work, and aggregate results.

### Gating

No feature gates — always compiled as a standalone crate.

### Architecture

**File inventory (7 files):**

| File | Purpose | Key type(s) |
|------|---------|-------------|
| `lib.rs:1-29` | Crate root; re-exports | — |
| `registry.rs:1-125` | Agent registration and lookup | `AgentRegistry` (FxHashMap<Uuid, AgentInfo> + tokio::RwLock), `AgentInfo`, `AgentStatus` |
| `scheduler.rs:1-400` | Task queue with priority, leasing, retry | `TaskScheduler`, `TaskQueue`, `ScheduledTask`, `TaskStatus` (5 variants), `TaskPriority` (4 variants: Critical/High/Normal/Low) |
| `lifecycle.rs:1-881` | Health checking, stale detection, graceful shutdown | `LifecycleManager`, `AgentHealth`, `HealthIssue` (5 variants), `LifecycleEvent`, `LifecycleConfig` |
| `delegation.rs:1-19` | Task delegation DTOs | `DelegationRequest`, `DelegationResponse` |
| `aggregator.rs:1-291` | Multi-stage result aggregation | `ResultAggregator`, `AggregatedResult`, `StageSummary`, `ToolSummary`, `AggregatedError` |
| `communication.rs:1-630` | Inter-agent messaging, capability advertisement | `MultiAgentCoordinator`, `AgentCapability`, `HealthMetrics`, `HealthStatus` (4 variants), `InterAgentChannel` |

**Dependencies** (`Cargo.toml:17-27`):

```toml
eggsec-core = { path = "../eggsec-core" }
reqwest = { version = "0.13", features = ["rustls-no-provider"], default-features = false }
rustls = { version = "0.23", default-features = false, features = ["ring", "std", "tls12"] }
```

Internal deps: `eggsec-core` only. External: `reqwest` + `rustls` (for `LifecycleManager` callback health checks), `tokio`, `uuid`, `chrono`, `serde`/`serde_json`, `rustc-hash`, `tracing`.

**TaskStatus** (`scheduler.rs:9-15`): `Pending` → `Leased` → `Completed` / `Failed` / `Cancelled`.

**HealthIssue** (`lifecycle.rs:40-46`): `MissedHeartbeat`, `CallbackUnhealthy(String)`, `HighLatency(u64)`, `TaskTimeout`, `ResourceExhaustion(String)`.

**HealthStatus** (`communication.rs:52-57`): `Healthy`, `Degraded`, `Unhealthy`, `Unknown`.

### Flows

**Agent registration flow:**

```
AgentRegistry::register(agent_info)
  → insert into FxHashMap<Uuid, AgentInfo>
  → LifecycleManager monitors via heartbeat checks
```

**Task scheduling flow:**

```
TaskQueue::submit(task)
  → Pending state, scheduled_for timestamp
TaskScheduler::next_task()
  → returns Pending tasks where scheduled_for <= now
TaskScheduler::lease_task(task_id, agent_id, timeout)
  → Pending → Leased
TaskScheduler::submit_result(task_id, outcome)
  → Leased → Completed or Failed
```

**Health check flow** (`lifecycle.rs`):

```
LifecycleManager::start()
  → periodic interval (default 30s)
  → for each registered agent:
      → check heartbeat staleness (default 120s threshold)
      → probe callback URL via reqwest
      → update AgentHealth with issues
      → if consecutive_failures > max (default 5): mark Offline
```

### Integration Points

- **Engine agent** (`eggsec/src/agent/`): Uses `eggsec-agent` re-exported through `tool/agents` compatibility facade (`tool/mod.rs:51-57`).
- **REST agent_routes** (`protocol/agent_routes.rs`): Exposes `AgentRegistry`, `TaskScheduler` over HTTP.
- **Aggregator**: `ResultAggregator` is used by pipeline/orchestrator to merge multi-tool execution results.

### Testing

```bash
cargo test -p eggsec-agent
```

### Gotchas

- `LifecycleManager` health checks use `reqwest` with `rustls` — requires `libssl-dev` at build time if TLS is needed (currently ring-only via `rustls-no-provider`).
- `now_ms()` in scheduler uses `SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default()` (`scheduler.rs:17-22`) — handles clock skew gracefully.
- `AgentInfo.last_heartbeat` is `u64` (epoch seconds), not `DateTime<Utc>`.

---

## 4. Tool Core DTOs (`crates/eggsec-tool-core/`)

### Responsibilities

Protocol-neutral, engine-free data types for the tool abstraction layer. These are pure DTOs with no dispatch logic, no enforcement, and no engine dependencies — enabling `eggsec-agent`, `eggsec-daemon-protocol`, and `eggsec-python` to share types without pulling in the full engine.

### Gating

No feature gates — always compiled.

### Architecture

**File inventory (7 files):**

| File | Purpose | Key type(s) |
|------|---------|-------------|
| `lib.rs:1-26` | Crate root; re-exports at crate level | — |
| `request.rs:1-356` | Tool invocation request | `ToolRequest` (id, tool, target, params, options, cancel_token), `Target`, `TargetType`, `AuthConfig`, `AuthType`, `Scope`, `RequestOptions`, `CancellationToken`, `CancellationTokenHandle` |
| `response.rs:1-321` | Tool execution response | `ToolResponse` (request_id, tool_id, status, results, metadata, errors, findings), `ResponseStatus` (6 variants: Success/Failed/Partial/Timeout/Cancelled/RateLimited), `ResponseMetadata`, `StreamEvent`, `StreamEventType`, `ProgressUpdate`, `PortData`, `PortState`, `EndpointData`, `TechnologyData` |
| `tool_error.rs:1-97` | Structured error type | `ToolError` (code, message, details, target, recoverable, error_type, retry_after_ms), `ToolErrorType` (11 variants) |
| `finding.rs:1-177` | Security finding DTO | `Finding` (id, finding_type, severity, title, description, location, evidence, cve_ids, remediation, references, metadata), `FindingType` (12 variants), `ResponseSeverity` |
| `history.rs:1-153` | Execution history ring buffer | `ExecutionHistory` (parking_lot::RwLock<Vec<ExecutionEntry>>, max 1000 entries default), `ExecutionEntry` |
| `ratelimit.rs:1-141` | Rate-limit configuration and status | `RateLimitConfig` (standard/relaxed/strict presets), `EndpointLimit`, `RateLimitStatus`, `GlobalRateLimitStatus` |

**Dependencies** (`Cargo.toml:17-24`):

```toml
eggsec-core = { path = "../eggsec-core" }
serde, serde_json, chrono, rustc-hash, parking_lot, uuid, toml
```

No network dependencies. No engine dependencies.

**ToolErrorType** (`tool_error.rs:50-63`) — 11 variants:

`Validation`, `Authentication`, `Authorization`, `RateLimit`, `Network`, `Timeout`, `ScopeViolation`, `NotFound`, `Configuration`, `Internal`, `ToolNotFound`.

Recoverable types (`tool_error.rs:66-74`): `RateLimit`, `Timeout`, `Network`, `Internal`.

**CancellationToken** (`request.rs:8-48`): AtomicBool-backed cooperative cancellation. `CancellationTokenHandle` wraps it with an optional `request_id` for serialization.

### Flows

```
ToolRequest → EnforcedDispatcher::dispatch_checked()
  → ToolDispatcher::dispatch()
      → ToolRegistry::get(tool_id)
      → SecurityTool::validate(&request)
      → SecurityTool::execute(request)
  → ToolResponse
```

### Integration Points

- **Engine `tool/` module**: Re-exports all types as sub-modules (`tool/mod.rs:29-33`) for backward compatibility (`crate::tool::tool_error::ToolError`).
- **eggsec-agent**: Uses `ToolRequest`/`ToolResponse` indirectly through engine integration.
- **eggsec-daemon-protocol**: Could depend on `eggsec-tool-core` for IPC types (currently depends on `eggsec-runtime`).
- **eggsec-python**: PyO3 bindings use these types for Python-facing API.

### Testing

```bash
cargo test -p eggsec-tool-core
```

### Gotchas

- `ExecutionHistory` uses `parking_lot::RwLock` (not tokio) — blocking reads in async context are fine because the critical section is tiny (clone).
- `Finding.metadata` uses `FxHashMap` for performance, not `std::collections::HashMap`.
- `RateLimitConfig::default()` = 60 req/min, 5 concurrent, 10 burst.

---

## 5. Tool Registry & Protocol Layer (`crates/eggsec/src/tool/`)

### Responsibilities

Centralized tool management: registry (FxHashMap-backed), tool trait abstraction, registration derivation from `OperationMetadata` + `DomainDescriptor`, protocol servers (REST/MCP/gRPC/agent/AI routes/OpenAI-compatible), and enforced dispatch with fail-closed binding validation.

### Gating

The `tool` module is gated at `lib.rs:161-162`:

```rust
#[cfg(any(feature = "tool-api", feature = "rest-api", feature = "grpc-api"))]
pub mod tool;
```

Protocol sub-modules carry per-feature gating (`protocol/mod.rs:1-16`):

| Sub-module | Feature gate |
|------------|-------------|
| `agent_routes`, `ai_routes`, `auth`, `mcp`, `openai`, `openresponses`, `rest` | `rest-api` |
| `grpc` | `grpc-api` |

### Architecture

**Top-level modules (17 entries):**

| Module | Purpose |
|--------|---------|
| `registry.rs:23-25` | `ToolRegistry` — `FxHashMap<String, Arc<dyn SecurityTool>>` behind `parking_lot::RwLock` |
| `traits.rs:1-319` | `SecurityTool` trait, `ToolCategory` (7 variants), `ToolCapability`, `ToolInfo` |
| `dispatcher.rs:1-288` | `ToolDispatcher` (raw) + `EnforcedDispatcher` (requires `ApprovedOperation`) |
| `registration.rs:1-367` | `ToolRegistration` derivation from `OperationMetadata` + `DomainDescriptor`; filter functions for each surface |
| `mod.rs:99-157` | `create_default_registry()` — registers 11 base tools + 3 gated tools |
| `metadata.rs` | Operation metadata lookup helpers |
| `finding.rs` | Engine-side finding enrichment |
| `convert.rs` | DTO conversion between engine and tool-core types |
| `openapi.rs` | OpenAPI spec generation from registry |
| `planner.rs` | `ChainPlanner` — sequential/parallel execution plan generation |
| `orchestrator/` | Parallel and sequential tool execution orchestration |
| `session.rs` | Session management (cookies, CSRF, MFA) |
| `state.rs` | Scan context, session manager |
| `scripting.rs` | Script execution helpers |
| `implementations/` | Concrete `SecurityTool` implementations (recon, scanner, fuzzer, waf, loadtest, pipeline, search, proxy, db-pentest, c2) |
| `protocol/` | Protocol server implementations |
| `AGENTS.override.md` | Module-specific guidance |

**ToolRegistry internals** (`registry.rs:23-25`):

```rust
pub struct ToolRegistry {
    tools: Arc<RwLock<FxHashMap<String, Arc<dyn SecurityTool>>>>,
}
```

Methods: `register()`, `unregister()`, `get()`, `list()`, `list_by_category()`, `categories()`, `find_by_capability()`, `find_by_keyword()`.

**create_default_registry** (`mod.rs:99-157`) — 11 base tools + 3 gated:

| # | Tool ID | Source | Feature gate |
|---|---------|--------|-------------|
| 1 | `recon` | `ReconTool::new()` | — |
| 2 | `scan-ports` | `ScannerTool::ports()` | — |
| 3 | `fingerprint` | `ScannerTool::fingerprint()` | — |
| 4 | `scan-endpoints` | `ScannerTool::endpoints()` | — |
| 5 | `fuzz` | `FuzzerTool::new()` | — |
| 6 | `load-test` | `LoadTestTool::new()` | — |
| 7 | `waf-detect` | `WafTool::detect()` | — |
| 8 | `waf-bypass` | `WafTool::bypass()` | — |
| 9 | `waf-stress` | `WafTool::stress()` | — |
| 10 | `pipeline` | `PipelineTool::new()` | — |
| 11 | `search` | `SearchTool::new(None)` | — |
| 12 | proxy | `ProxyTool::new()` | `web-proxy-mcp` |
| 13 | db-pentest | `DbPentestTool::new()` | `db-pentest-mcp` |
| 14 | c2 | `C2Tool::new()` | `c2-mcp` |

**ToolRegistration** (`registration.rs:11-27`):

```rust
pub struct ToolRegistration {
    pub tool_id: &'static str,
    pub operation_id: &'static str,
    pub display_name: &'static str,
    pub source: ToolRegistrationSource,
    pub feature: Option<&'static str>,
    pub required_mcp_feature: Option<&'static str>,
    pub mcp_metadata_exposable: bool,
    pub mcp_default_visible: bool,
    pub rest_exposable: bool,
    pub grpc_exposable: bool,
    pub agent_exposable: bool,
    pub category: ToolCategory,
}
```

`ToolRegistrationSource` (`registration.rs:31-38`): `Base`, `FeatureGated(&'static str)`, `Domain(&'static str)`.

**Registration filter functions** (`registration.rs:140-201`):

| Function | Filter |
|----------|--------|
| `mcp_tool_registrations("ops-agent")` | All tools with `mcp_metadata_exposable = true` |
| `mcp_tool_registrations("coding-agent")` | Hardcoded narrow allowlist (scan, scan-ports, fingerprint, scan-endpoints, endpoints, waf-detect, search) |
| `mcp_tool_registrations_default_visible()` | Tools with `mcp_default_visible = true` (passive/safe-active, no feature gate) |
| `rest_tool_registrations()` | `rest_exposable = true` |
| `grpc_tool_registrations()` | `grpc_exposable = true` |
| `agent_tool_registrations()` | `agent_exposable = true` |

**MCP exposure model** (`registration.rs:43-49`):

```
default_mcp_visible_for_operation(meta) =
    (meta.risk == Passive || meta.risk == SafeActive)
    && meta.mcp_exposable
    && meta.required_features.is_empty()
```

### Protocol Servers

**Protocol file inventory (`tool/protocol/`):**

| Path | Purpose | Feature |
|------|---------|---------|
| `rest.rs:1-1377` | Axum REST server with rate limiting, CORS, API-key auth, max payload 10MB | `rest-api` |
| `auth.rs:1-138` | Constant-time API key validation, X-API-Key/Bearer token extraction | `rest-api` |
| `agent_routes.rs:1-1573` | Agent/task CRUD endpoints, SSRF-protected callback URL validation | `rest-api` |
| `ai_routes.rs:1-598` | AI payload/WAF-bypass suggestion endpoints | `rest-api` |
| `openai/mod.rs` | OpenAI-compatible chat completions adapter | `rest-api` |
| `openai/handlers.rs` | Request/response handlers | `rest-api` |
| `openai/models.rs` | Model definitions | `rest-api` |
| `openai/types.rs` | OpenAI-specific types | `rest-api` |
| `openresponses/mod.rs` | OpenAI Responses API adapter | `rest-api` |
| `openresponses/handlers.rs` | Handlers | `rest-api` |
| `openresponses/types.rs` | Types | `rest-api` |
| `mcp/mod.rs:1-963` | MCP server module root (11 sub-files) | `rest-api` |
| `mcp/handlers/server.rs` | MCP request handler, enforcement boundary | `rest-api` |
| `mcp/handlers/helpers.rs` | Helper functions | `rest-api` |
| `mcp/handlers/mod.rs` | Handler module root | `rest-api` |
| `mcp/routes.rs` | MCP stdio/HTTP transport | `rest-api` |
| `mcp/policy.rs:1-1554` | `McpProfilePolicy`, `TargetPolicy`, `ToolSelector`, risk classification | `rest-api` |
| `mcp/profile.rs` | `McpProfile` enum (OpsAgent, CodingAgent) | `rest-api` |
| `mcp/types.rs` | MCP protocol types | `rest-api` |
| `mcp/auth.rs` | MCP authentication | `rest-api` |
| `mcp/constraints.rs` | `McpConstraintContext` | `rest-api` |
| `mcp/coding_agent_output.rs` | `CodingAgentFindingReport` typed output | `rest-api` |
| `mcp/prompts.rs` | MCP prompt templates | `rest-api` |
| `mcp/streaming.rs` | Stream events | `rest-api` |
| `grpc.rs:1-1145` | tonic gRPC service, checked-in proto-generated code | `grpc-api` |
| `grpc.proto` | Protobuf service definition | `grpc-api` |
| `mod.rs:1-16` | Protocol module root with per-feature gates | — |

### Enforced Dispatch Flow

```
Surface (REST/MCP/gRPC/Agent)
  → EnforcementContext::evaluate(descriptor)
  → produces ApprovedOperation token (or denial)
  → EnforcedDispatcher::dispatch_checked(approval, request)
      → validate_request_binding(approval, request)
          → operation_matches_tool_id(request.tool, approval.operation)
          → target normalization comparison
          → typed-vs-parameter target agreement
          → fail-closed on any mismatch
      → ToolDispatcher::dispatch(request)
          → ToolRegistry::get(tool_id)
          → SecurityTool::validate(&request)
          → SecurityTool::execute(request)
      → record in ExecutionHistory (if configured)
  → ToolResponse
```

### Testing

```bash
cargo test --lib -p eggsec tool::
```

### Gotchas

- `ToolDispatcher::dispatch()` is `pub(crate)` with `#[doc(hidden)]` (`dispatcher.rs:177-178`) — strict surfaces must use `EnforcedDispatcher::dispatch_checked()`.
- `ToolRegistry::register()` rejects duplicate IDs with `EggsecError::Config` (`registry.rs:59-64`).
- `rest.rs` enforces `MAX_PAYLOAD_SIZE = 10MB` and `MAX_URL_LENGTH = 2048` (`rest.rs:28-29`).
- MCP `tools/call` enforcement uses error codes: `-32020` (tool denied), `-32021` (argument denied), `-32022` (concurrency exceeded), `-32024` (target denied), `-32025` (enforcement denial).

---

## Shared Invariants

1. **Single source of truth**: `OperationMetadata` defines all operation policy. Never build policy checks inline. Every `OperationDescriptor` derives from metadata via `metadata.descriptor_for_target()`.

2. **ApprovedOperation is the only valid dispatch token**: Strict surfaces (REST, MCP, gRPC, agent) must dispatch through `EnforcedDispatcher::dispatch_checked()`. Raw `ToolDispatcher::dispatch()` is `pub(crate)` and `#[doc(hidden)]`.

3. **EnforcementContext::evaluate() is the mandatory pre-dispatch gate**: All surfaces must call it before dispatch. Scope must come from `LoadedScope`, never raw `Scope`.

4. **Fail-closed binding**: `validate_request_binding()` rejects any mismatch between approval and request (tool name, target normalization, typed-vs-parameter agreement).

5. **MCP exposure vocabulary**: Use `mcp_metadata_exposable` for tools allowed under expanded profile listing, `mcp_default_visible` for the conservative default subset. The OpsAgent profile returns all `mcp_metadata_exposable` tools — this is profile-expanded, not the conservative default.

6. **FxHashMap everywhere**: Performance-critical paths use `rustc_hash::FxHashMap`/`FxHashSet`, not `std::collections::HashMap`. Verified in: `ToolRegistry`, `AiCache`, `AiPlanner`, `AgentRegistry`, `ResultAggregator`, `MultiAgentCoordinator`, `LongitudinalMemory`, `Finding.metadata`, `RateLimitConfig`.

7. **TLS provider**: All crates use ring-only (no aws-lc-rs). When declaring `reqwest`, use `features = ["rustls-no-provider"]`.

8. **No silent error suppression**: Never use `let _ =` or `filter_map(|e| e.ok())` without logging. All `SystemTime::now().duration_since(UNIX_EPOCH)` calls use `.unwrap_or_else(|_| ...)` or `.unwrap_or_default()` to prevent clock-skew panics.

9. **Timeout wrappers**: All outbound HTTP calls need explicit timeouts. AI client uses 60s (`client.rs:76`). MCP routes use 30s. Tool execution uses 60s.

10. **eggsec-agent dependency boundary**: The `eggsec-agent` crate depends only on `eggsec-core` (internal) plus `reqwest`/`rustls` (external). It must never depend on the engine crate.

---

## Bug Sweep

### Confirmed bugs

| File:line | Issue | Severity |
|-----------|-------|----------|
| `agent/mod.rs:252` | `memory.warm_cache().await.ok()` silently discards warm-cache errors without logging. Should use `tracing::warn!` on `Err`. | Low |

### Potential issues (verify before fixing)

| File:line | Issue | Severity |
|-----------|-------|----------|
| `tool/registry.rs:57` | `self.tools.write()` held across `tool.id()` and `insert()` — short critical section, but contention possible under heavy concurrent registration. Consider `entry` API. | Low |
| `ai/client.rs:76` | AI HTTP client timeout is 60s (`Client::builder().timeout(Duration::from_secs(60))`) — appropriate for most LLM calls but may be too short for long-context analysis. | Info |
| `agent/mod.rs:192-207` | `Agent` holds multiple `#[allow(dead_code)]` fields (`registry`, `config_watcher`) — may indicate dead code or intentional future use. | Info |

---

## Overview.md Discrepancies

None found. The overview.md references to `ai_agents.md` are consistent:
- `overview.md:181` links Tool Registry to `ai_agents.md` ✓
- `overview.md:182` links Agent to `ai_agents.md` ✓
- `overview.md:250` links AI/LLM to `ai_agents.md` ✓
- `overview.md:262` links Tool Core to `ai_agents.md` ✓
- `overview.md:575` links Orchestration deep-dive to `ai_agents.md` ✓

The overview.md correctly states `eggsec-agent` internal deps are `eggsec-core` only (line 60) and that `eggsec-tool-core` contains `ToolRequest`, `ToolResponse`, `ToolError`, finding/history/rate-limit types, cancellation tokens (line 262).

*Last verified against source: 2026-08-25*
