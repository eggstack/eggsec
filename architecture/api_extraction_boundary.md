# API Extraction Boundary Note

## Current Owner

All API/agent modules currently live in the `slapper` crate:

- `crates/slapper/src/tool/protocol/` - REST, MCP, gRPC, OpenAI, OpenResponses adapters
- `crates/slapper/src/tool/agents/` - Agent registry, task scheduler, lifecycle management
- `crates/slapper/src/agent/` - Autonomous agent (portfolio, memory, alerts, skills)
- `crates/slapper/src/nse_tool.rs` - NSE tool implementation

Feature gates:
- `rest-api` = `["tool-api", "axum", "tower", "tower-http", "async-stream"]`
- `grpc-api` = `["tool-api", "tonic", "prost", "prost-types", "tonic-prost", "tonic-reflection", "prost-build", "tonic-prost-build", "async-stream", "tokio-stream"]`
- `ai-integration` = `["tool-api", "eventsource-stream", "semver"]`
- `ws-api` = `["axum/ws"]`

---

## Candidate slapper-api modules

These modules are server adapters that translate between wire protocols and the tool abstraction layer. They have no engine-internal logic.

### REST adapter
- **File:** `crates/slapper/src/tool/protocol/rest.rs`
- **Dependencies:** `axum`, `tower-http`, `subtle`, `crate::tool::{ToolDispatcher, ToolRegistry, ToolRequest, ToolResponse}`, `crate::config::Scope`, `crate::distributed::TlsConfig`, `crate::tool::ratelimit::*`
- **Notes:** Depends on `crate::distributed::TlsConfig` - would need to be passed as a trait object or moved to a shared types crate.

### MCP adapter
- **Files:**
  - `crates/slapper/src/tool/protocol/mcp/mod.rs`
  - `crates/slapper/src/tool/protocol/mcp/routes.rs` - axum HTTP + stdio transport
  - `crates/slapper/src/tool/protocol/mcp/handlers/server.rs` - McpServer (1686 lines, deeply coupled)
  - `crates/slapper/src/tool/protocol/mcp/handlers/helpers.rs`
  - `crates/slapper/src/tool/protocol/mcp/auth.rs`
  - `crates/slapper/src/tool/protocol/mcp/types.rs` - McpRequest, McpResponse, McpError, McpTool, McpResource, McpRoot
  - `crates/slapper/src/tool/protocol/mcp/streaming.rs` - StreamEvent
  - `crates/slapper/src/tool/protocol/mcp/policy.rs` - McpProfilePolicy, TargetPolicy, ToolSelector
  - `crates/slapper/src/tool/protocol/mcp/profile.rs` - McpProfile enum
  - `crates/slapper/src/tool/protocol/mcp/constraints.rs`
  - `crates/slapper/src/tool/protocol/mcp/coding_agent_output.rs`
  - `crates/slapper/src/tool/protocol/mcp/prompts.rs` (feature-gated `rest-api`)
- **Dependencies:** `axum`, `async-stream`, `tokio`, `crate::tool::{ToolDispatcher, ToolRegistry, ...}`, `crate::ai::AiClient` (optional), `crate::config::Scope`
- **Blocker:** `McpServer` directly holds `ToolRegistry`, `ToolDispatcher`, `SessionManager`, `AiClient`. It's the most deeply coupled adapter. Would need a trait-based decoupling to move.

### gRPC adapter
- **Files:**
  - `crates/slapper/src/tool/protocol/grpc.rs`
  - `crates/slapper/src/tool/protocol/grpc.proto` (proto definition)
- **Dependencies:** `tonic`, `prost`, `prost-types`, `tonic-reflection`, `async-stream`, `tokio-stream`, `crate::tool::{ToolDispatcher, ToolRegistry, ToolRequest}`
- **Notes:** Self-contained service implementation. Cleanest candidate for extraction.

### OpenAI-compatible adapter
- **Files:**
  - `crates/slapper/src/tool/protocol/openai/mod.rs`
  - `crates/slapper/src/tool/protocol/openai/handlers.rs`
  - `crates/slapper/src/tool/protocol/openai/models.rs`
  - `crates/slapper/src/tool/protocol/openai/types.rs`
- **Dependencies:** `axum`, `crate::tool::registry::ToolRegistry`, `crate::config::Scope`

### OpenResponses adapter
- **Files:**
  - `crates/slapper/src/tool/protocol/openresponses/mod.rs`
  - `crates/slapper/src/tool/protocol/openresponses/handlers.rs`
  - `crates/slapper/src/tool/protocol/openresponses/types.rs`
- **Dependencies:** `axum`, `crate::tool::registry::ToolRegistry`

### AI REST routes
- **File:** `crates/slapper/src/tool/protocol/ai_routes.rs`
- **Dependencies:** `axum`, `subtle`, `crate::ai::AiClient` (optional), `crate::utils::circuit_breaker::*`

### Agent REST routes
- **File:** `crates/slapper/src/tool/protocol/agent_routes.rs`
- **Dependencies:** `axum`, `uuid`, `subtle`, `crate::tool::agents::{AgentRegistry, TaskScheduler, ...}`, `crate::constants::*`

---

## Candidate slapper-agent modules

These modules implement autonomous agent scheduling, memory, and orchestration. They are tightly coupled to the tool engine but conceptually separate from the protocol adapters.

### Agent core
- **Files:**
  - `crates/slapper/src/agent/mod.rs` - Agent struct, run loop, scan execution
  - `crates/slapper/src/agent/portfolio.rs` - TargetPortfolio, TargetConfig, ScanRecord
  - `crates/slapper/src/agent/memory.rs` - LongitudinalMemory
  - `crates/slapper/src/agent/alerts/` - AlertRouter, AlertChannel, EmailChannel, SlackChannel, PagerDutyChannel, WebhookConfig
  - `crates/slapper/src/agent/channels.rs`
  - `crates/slapper/src/agent/events.rs` - SecurityEvent, EventHandler
  - `crates/slapper/src/agent/constraints/` - ConstraintChecker, OperationalConstraints, DoNotDoList
  - `crates/slapper/src/agent/config_watcher.rs` - ConfigWatcher, ConfigReloader
  - `crates/slapper/src/agent/skills.rs` (feature-gated `ai-integration`) - SkillLoader, SkillRegistry
- **Dependencies:** `crate::tool::{create_default_registry, ToolDispatcher, ToolRegistry, ToolRequest, ToolResponse}`, `crate::config::SlapperConfig`, `crate::output::schedule::CronScheduler`, `crate::ai::AiClient` (optional)
- **Blocker:** `Agent::new()` calls `create_default_registry()` directly and holds a `ToolDispatcher`. Would need the tool registry injected as a dependency.

### Tool agent coordination
- **Files:**
  - `crates/slapper/src/tool/agents/mod.rs`
  - `crates/slapper/src/tool/agents/registry.rs` - AgentRegistry, AgentInfo, AgentStatus
  - `crates/slapper/src/tool/agents/scheduler.rs` - TaskScheduler, ScheduledTask, TaskStatus, TaskPriority
  - `crates/slapper/src/tool/agents/lifecycle.rs` - LifecycleManager, AgentHealth
  - `crates/slapper/src/tool/agents/communication.rs` - MultiAgentCoordinator, InterAgentChannel
  - `crates/slapper/src/tool/agents/delegation.rs` - DelegationRequest, DelegationResponse
  - `crates/slapper/src/tool/agents/aggregator.rs` - ResultAggregator
- **Dependencies:** `uuid`, `tokio`, `serde_json`, `crate::constants::*`
- **Notes:** These modules have minimal coupling to slapper internals (only `crate::constants`). Good candidates for early extraction.

### Agent CLI handler
- **File:** `crates/slapper/src/commands/handlers/agent.rs`
- **Dependencies:** `crate::agent::*`, `crate::cli::agent::*`

---

## Must remain in slapper for now

These modules depend on engine-internal types and cannot be extracted without significant refactoring:

- `tool/traits.rs` - SecurityTool trait definition (defines the contract everything else depends on)
- `tool/registry.rs` - ToolRegistry (holds `Arc<dyn SecurityTool>`)
- `tool/dispatcher.rs` - ToolDispatcher (validates + executes tools)
- `tool/implementations/` - All concrete tool implementations (recon, scanner, fuzzer, waf, loadtest, pipeline, search)
- `tool/finding.rs` - `From` impls that depend on scanner/fuzzer/recon types
- `tool/session.rs` - SessionManager, AuthenticatedSessionManager
- `tool/state.rs` - AgentSession, ScanContext
- `tool/planner.rs` - ChainPlanner, ExecutionPlan
- `tool/orchestrator/` - Orchestrator, StageResult
- `tool/openapi.rs` - OpenApiGenerator
- `tool/convert.rs` - conversion utilities
- `tool/scripting.rs` - scripting utilities
- `nse_tool.rs` - NseTool (depends on `slapper_nse::NseExecutor`)

---

## DTOs already in slapper-tool-core

The `slapper-tool-core` crate already contains protocol-neutral DTOs:

| Module | Types |
|--------|-------|
| `request.rs` | `ToolRequest`, `Target`, `TargetType`, `Scope`, `RequestOptions`, `AuthConfig`, `AuthType`, `CancellationToken`, `CancellationTokenHandle` |
| `response.rs` | `ToolResponse`, `ResponseMetadata`, `ResponseStatus`, `ProgressUpdate`, `StreamEvent`, `StreamEventType`, `EndpointData`, `PortData`, `PortState`, `TechnologyData` |
| `finding.rs` | `Finding`, `FindingType`, `ResponseSeverity` |
| `tool_error.rs` | `ToolError`, `ToolErrorType` |
| `history.rs` | `ExecutionEntry`, `ExecutionHistory` |
| `ratelimit.rs` | `RateLimitConfig`, `RateLimiter`, `RateLimitStatus`, `EndpointLimit`, `GlobalRateLimitStatus` |

These types have no dependencies on the main slapper engine (only `slapper-core` for `Severity`, `SensitiveString`).

---

## Dependency targets to isolate

To enable extraction of the server adapters into `slapper-api`:

1. **ToolRegistry trait interface** - Currently `ToolRegistry` holds `Arc<dyn SecurityTool>`. A `slapper-api` crate would need to depend on `slapper-tool-core` for DTOs and accept a `ToolRegistry` via dependency injection (trait object or `Arc<dyn ToolRegistryInterface>`).

2. **TlsConfig** - `rest.rs` imports `crate::distributed::TlsConfig`. Either move to a shared crate or pass as a trait.

3. **AiClient** - `ai_routes.rs` and `McpServer` optionally hold `AiClient`. Pass as `Option<Arc<dyn AiClientTrait>>` or keep ai-integration feature-gated in slapper-api.

4. **SessionManager** - `McpServer` holds `SessionManager`. Could be made optional or trait-based.

5. **Scope** - `config::Scope` is used by REST and MCP adapters. Move to slapper-core or accept as `Arc<dyn ScopeEnforcer>`.

6. **Constants** - `crate::constants::*` is used by agent_routes and agents. Move shared constants to slapper-core.

---

## Known blockers

1. **McpServer deep coupling** - `handlers/server.rs` is 1686 lines and directly uses `ToolRegistry`, `ToolDispatcher`, `SessionManager`, `AiClient`, `Scope`, `CancellationToken`, and many other slapper-internal types. Extracting this requires either: (a) trait-based decoupling of all dependencies, or (b) keeping McpServer in slapper and only extracting the HTTP transport layer.

2. **`create_default_registry()` coupling** - Both `Agent::new()` and test code call `create_default_registry()` which hardcodes all tool implementations. The agent module would need the registry injected.

3. **Feature gate alignment** - `rest-api` enables axum/tower/async-stream, while `grpc-api` enables tonic/prost. A `slapper-api` crate would need to re-export these as optional features.

4. **`From` impls in finding.rs** - The `From<FuzzResult> for Finding` etc. impls depend on engine types. These must stay in slapper, but the `Finding` type itself is already in `slapper-tool-core`.

5. **WebSocket support** - `ws-api` feature adds WebSocket handler inside `rest.rs`. Would need conditional compilation in slapper-api.

---

## Proposed next-pass order

### Phase 1: Extract slapper-tool-core DTOs (DONE)
Already complete. `slapper-tool-core` contains all protocol-neutral request/response/finding/error types.

### Phase 2: Extract tool agent coordination to slapper-agent
Extract `tool/agents/` (registry, scheduler, lifecycle, communication, delegation, aggregator) into a new `slapper-agent` crate. These modules have minimal coupling (only `crate::constants`).

**Files to move:**
- `tool/agents/registry.rs`
- `tool/agents/scheduler.rs`
- `tool/agents/lifecycle.rs`
- `tool/agents/communication.rs`
- `tool/agents/delegation.rs`
- `tool/agents/aggregator.rs`
- `tool/agents/mod.rs`

**Dependencies to resolve:**
- Move `crate::constants::DEFAULT_MAX_RETRIES` and `DEFAULT_SCHEDULER_RETRY_DELAY_MS` and `DEFAULT_LEASE_DURATION_MS` to slapper-core or slapper-agent.

### Phase 3: Extract gRPC adapter to slapper-api
The gRPC adapter is the cleanest candidate - it has a clear proto boundary and minimal coupling beyond ToolRegistry/ToolDispatcher.

**Files to move:**
- `tool/protocol/grpc.rs`
- `tool/protocol/grpc.proto` (and build.rs proto compilation)

**Dependencies to resolve:**
- Accept `ToolRegistry` and `ToolDispatcher` via constructor injection.

### Phase 4: Extract REST + OpenAI + OpenResponses adapters to slapper-api
These all use axum and share similar patterns.

**Files to move:**
- `tool/protocol/rest.rs`
- `tool/protocol/openai/`
- `tool/protocol/openresponses/`
- `tool/protocol/ai_routes.rs`

**Dependencies to resolve:**
- `TlsConfig` - pass as parameter or trait
- `Scope` - pass as `Option<Scope>` from slapper-tool-core (Scope already exists there)
- `RateLimiter` - already in slapper-tool-core

### Phase 5: Extract MCP adapter to slapper-api
Most complex extraction due to McpServer coupling.

**Files to move:**
- `tool/protocol/mcp/types.rs`
- `tool/protocol/mcp/streaming.rs`
- `tool/protocol/mcp/policy.rs`
- `tool/protocol/mcp/profile.rs`
- `tool/protocol/mcp/auth.rs`
- `tool/protocol/mcp/constraints.rs`
- `tool/protocol/mcp/coding_agent_output.rs`
- `tool/protocol/mcp/prompts.rs`
- `tool/protocol/mcp/routes.rs`
- `tool/protocol/mcp/handlers/` (after trait-based decoupling)

**Dependencies to resolve:**
- McpServer needs trait-based access to ToolRegistry, ToolDispatcher, SessionManager, AiClient
- Consider splitting McpServer into transport (move to slapper-api) and handler logic (stay in slapper)

### Phase 6: Extract agent core to slapper-agent
Move the autonomous agent runtime (portfolio, memory, alerts, constraints, skills).

**Files to move:**
- `agent/mod.rs` (Agent struct, run loop)
- `agent/portfolio.rs`
- `agent/memory.rs`
- `agent/alerts/`
- `agent/channels.rs`
- `agent/events.rs`
- `agent/constraints/`
- `agent/config_watcher.rs`
- `agent/skills.rs`

**Dependencies to resolve:**
- Agent must accept ToolRegistry via injection (not call `create_default_registry()`)
- AiClient must be optional and injected
- CronScheduler already in slapper-output, accessible via workspace dependency
