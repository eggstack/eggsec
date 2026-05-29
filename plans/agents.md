# Plan: Productionize Slapper Agent and MCP Profiles

## Purpose

Productionize Slapper's autonomous security agent and its MCP exposure as two separate operating profiles over one shared engine:

1. `ops-agent`: Slapper's own security-operations/autonomous-assessment profile. This profile can orchestrate scheduled assessments, portfolios, longitudinal memory, alerting, agent lifecycle, and broader security workflows under explicit scope controls.
2. `coding-agent`: Codegg-facing profile. This profile must expose only bounded, low-noise, local/dev-environment validation tools suitable for a coding agent. It must not expose broad recon, stress testing, packet crafting, destructive tooling, or high-volume testing by default.

Do not create two independent MCP implementations. The existing direction is better: one MCP core under `crates/slapper/src/tool/protocol/mcp/`, with profile-specific tool surfaces, prompts, resources, output schemas, and enforcement. The implementation task is to make that split complete and production-grade.

## Current repository state to preserve

The repo already has the main seams needed for this work.

Relevant current files:

- `crates/slapper/src/tool/protocol/mcp/profile.rs`
  - Already defines `McpProfile::{OpsAgent, CodingAgent}`.
  - Already gives separate server names/descriptions.
- `crates/slapper/src/commands/handlers/notify.rs`
  - `handle_mcp_serve` already maps `--profile coding-agent` to `McpProfile::CodingAgent` and otherwise defaults to `OpsAgent`.
  - It also supports stdio and network serving.
- `crates/slapper/src/tool/protocol/mcp/routes.rs`
  - Owns MCP HTTP/SSE/stdio routing.
  - `create_mcp_router(registry, api_key, profile)` and `run_stdio(registry, api_key, profile)` already accept a profile.
- `crates/slapper/src/tool/protocol/mcp/handlers/server.rs`
  - `McpServer` already stores `profile: McpProfile`.
  - `initialize` already returns special coding-agent safety metadata.
  - `resources/list` and `resources/read` already have coding-agent-specific resources.
  - `tools/list` currently appears to list the whole registry without filtering.
  - `tools/call` currently appears to resolve and dispatch arbitrary tools from the registry, subject mainly to auth/rate/scope.
- `crates/slapper/src/tool/protocol/mcp/prompts.rs`
  - Already separates ops-agent prompts and coding-agent prompts.
- `crates/slapper/src/tool/protocol/mcp/constraints.rs`
  - Has an initial `McpConstraintContext`, but it is not yet sufficient as the central policy engine.
- `crates/slapper/src/agent/mod.rs`
  - Slapper autonomous agent is real: scheduled scans, portfolio, memory, alerts, constraints, config watcher, logging.
- `crates/slapper/src/tool/protocol/agent_routes.rs`
  - Agent registration, task scheduling, task leasing, result submission, and callback URL validation exist.
- `crates/slapper/src/tool/agents/*`
  - Contains registry, scheduler, lifecycle, communication, delegation, aggregation.
- `docs/AGENT.md`, `docs/mcp-protocol.md`, `architecture/ai_agents.md`, `docs/CAPABILITIES.md`
  - These docs already describe agent and MCP concepts but need updating to reflect profile-specific production behavior.

## Non-goals

Do not remove the existing Slapper security-agent capability model.

Do not duplicate the MCP server into two unrelated codepaths. Use one MCP implementation and make profiles first-class.

Do not expose offensive/stress/destructive primitives to Codegg by default. The coding-agent profile is for controlled validation of code changes against local or explicitly scoped dev/test targets.

Do not solve full distributed persistence or multi-node scheduling in this pass. Use trait seams and state abstractions so persistence can be added later, but keep the first implementation bounded.

Do not change unrelated scanner/fuzzer internals except where needed to support profile filtering, deterministic output, cancellation, and safety budgets.

## Target architecture

The desired structure is:

```text
slapper engine
  tool registry
  tool dispatcher
  scan/fuzz/recon/waf/nse primitives
  agent runtime
    portfolio
    memory
    scheduler
    constraints
    alerts
    logging/events
  mcp core
    transport: http/json-rpc, sse, stdio
    auth/rate/scope/session/cancellation
    profile: ops-agent | coding-agent
    profile policy
    profile resources
    profile prompts
    profile output contracts
```

There should be exactly one MCP core. The profile chooses what the client can see and do.

## Phase 1: Audit and encode the profile contract

Create a concrete profile-policy layer rather than relying on scattered `if profile.is_coding_agent()` checks.

Suggested new files:

- `crates/slapper/src/tool/protocol/mcp/policy.rs`
- optionally `crates/slapper/src/tool/protocol/mcp/profile_policy.rs` if you prefer a narrower name

Suggested types:

```rust
pub struct McpProfilePolicy {
    pub profile: McpProfile,
    pub default_target_policy: TargetPolicy,
    pub allowed_tool_ids: ToolSelector,
    pub denied_tool_ids: ToolSelector,
    pub allowed_capabilities: ToolSelector,
    pub denied_capabilities: ToolSelector,
    pub max_concurrency: usize,
    pub max_timeout_ms: u64,
    pub max_batch_size: usize,
    pub allow_streaming: bool,
    pub allow_sessions: bool,
    pub allow_plan_endpoint: bool,
    pub require_explicit_scope: bool,
    pub allow_external_network: bool,
    pub allow_stress_testing: bool,
    pub allow_packet_features: bool,
    pub allow_broad_recon: bool,
}

pub enum TargetPolicy {
    ExplicitScopeOnly,
    LocalhostAndPrivateCidrsOnly,
    ScopeOrLocalDevOnly,
    AnyWithScopeEngine,
}

pub enum ToolSelector {
    All,
    None,
    Exact(Vec<String>),
    Category(Vec<String>),
    Capability(Vec<String>),
}
```

Keep the actual implementation idiomatic to existing code; this is a shape, not a strict API requirement.

Required behavior:

- `OpsAgent` policy may expose the broad Slapper toolkit, but still must respect configured scope, auth, rate limits, and dangerous feature gates.
- `CodingAgent` policy must start from deny-by-default and explicitly allow only bounded validation tools.
- All policy checks must happen both at discovery time and call time. Discovery filtering alone is insufficient.
- Policy violations must return structured MCP errors with sanitized messages.

Acceptance criteria:

- Unit tests prove `CodingAgent` cannot call a tool that is not visible in its `tools/list` response.
- Unit tests prove direct `tools/call` with a denied tool name returns a policy error.
- Unit tests prove `OpsAgent` still sees the normal registry, except tools gated by config or features.
- `initialize` reports profile metadata derived from the same profile policy, not duplicated hand-written booleans.

## Phase 2: Filter MCP tool discovery by profile

Update `McpServer::handle_tools_list` and `handle_tools_list_by_category` in `crates/slapper/src/tool/protocol/mcp/handlers/server.rs`.

Current issue to address:

- `tools/list` maps every `registry.list()` entry into `McpTool`.
- `tools/list-by-category` categorizes every `registry.list()` entry.
- This makes the coding-agent profile look safer in `initialize`, but still gives it the full tool catalog.

Implementation tasks:

1. Add a helper on `McpServer`:

```rust
fn visible_tools_for_profile(&self) -> Vec<ToolInfo> { ... }
```

or a policy method:

```rust
self.profile_policy().filter_tools(self.registry.list())
```

2. Replace raw `self.registry.list()` in `tools/list`, `tools/list-by-category`, `slapper://tools`, and `slapper://manifest` with the filtered list for the active profile.

3. Ensure coding-agent-specific resources do not claim availability for tools the profile cannot actually call.

4. Add profile metadata to tool descriptors where useful:

```json
{
  "x-slapper-profile": "coding-agent",
  "x-slapper-safety": {
    "requires_scope": true,
    "external_network_default": false,
    "max_concurrency": 3,
    "max_timeout_ms": 30000
  }
}
```

Avoid nonstandard data if it breaks MCP clients. If needed, include this under capabilities or resource manifests instead of `McpTool`.

Coding-agent default allowlist should initially be conservative. Suggested categories/capabilities:

- Basic HTTP validation of a single explicit URL.
- Header/security-header inspection.
- CORS validation against a provided URL.
- TLS/certificate validation for explicitly scoped targets.
- Endpoint validation where the target URL is provided directly.
- WAF regression checks only when target is localhost/private CIDR or an explicit scope file allows it.
- CVE/technology mapping only when based on user-provided dependency/service evidence or explicitly scoped target, not broad internet recon.
- Retest of a known finding ID against a provided target.

Default deny for coding-agent:

- Stress testing, load testing, flood primitives.
- Packet capture/crafting/sending.
- Broad subdomain enumeration.
- WHOIS/ASN/threat-intel enrichment by default.
- Cloud asset enumeration by default.
- SSRF payloads that target metadata, localhost, or private network unless the target is explicitly a local lab and the payload profile is safe.
- Command injection, deserialization exploit-generation, destructive payload classes, intrusive NSE categories.
- Anything requiring root privileges.
- Anything requiring stealth/evasion mode.

Acceptance criteria:

- `slapper mcp-serve --stdio --profile coding-agent` returns a small, bounded tool list.
- `slapper mcp-serve --stdio --profile ops-agent` returns the broader operational tool list.
- The coding-agent manifest exactly matches the callable tools.
- Existing tests for `tools/list` continue to pass after updating expectations for profile-specific behavior.

## Phase 3: Enforce profile policy in `tools/call`

Update `McpServer::handle_tools_call`.

Add explicit call-time checks after `resolve_tool_id(name)` and before building `ToolRequest`:

1. Validate the resolved tool ID is allowed for the active profile.
2. Validate the selected capability, if any, is allowed for the active profile.
3. Validate target policy.
4. Clamp or reject timeout/concurrency/request budgets.
5. Reject denied arguments such as stealth, proxy rotation, excessive mutation counts, raw packet mode, root-required modes, or stress modes in coding-agent profile.
6. Enforce feature gates and runtime flags.

Suggested helper shape:

```rust
fn validate_profile_call(
    &self,
    tool_id: &str,
    capability: Option<&str>,
    arguments: &serde_json::Value,
    target_value: &str,
) -> Result<ValidatedMcpCall, McpError>
```

`ValidatedMcpCall` can include sanitized/clamped arguments and request options.

Important safety detail:

Discovery filtering is for the client. Call-time enforcement is the boundary. Assume a malicious or confused agent can call hidden tools directly.

Acceptance criteria:

- Coding-agent direct calls to denied tools fail even if the client guesses the tool name.
- Coding-agent requests with `concurrency` above policy fail or are clamped deterministically. Prefer fail-closed for the first implementation unless there is already a clear clamping convention.
- Coding-agent requests with `timeout_ms` above policy fail or are clamped deterministically.
- Coding-agent requests to external public internet targets fail unless an explicit scope file/policy permits them.
- Ops-agent behavior remains compatible with existing tests and docs.

## Phase 4: Formalize target scope for coding-agent

The coding-agent profile must have a target policy appropriate for Codegg workflows.

Default target behavior:

- Allow loopback: `localhost`, `127.0.0.0/8`, `::1`.
- Allow private lab networks only if explicitly enabled or scoped: RFC1918 IPv4 ranges, ULA IPv6, Docker bridge ranges if represented as private CIDRs.
- Deny public internet by default.
- Deny link-local metadata endpoints by default, especially `169.254.169.254`, cloud metadata hostnames, and equivalent IPv6/alternate encodings.
- Require explicit `--scope-file` or config entry for any external host.

Implementation tasks:

1. Reuse existing `Scope` / `ScopeRule` machinery where possible.
2. Add a profile-level target validator in MCP policy code.
3. Make DNS resolution behavior explicit. Avoid TOCTOU where a hostname resolves differently after validation.
4. Normalize hostnames and IPs before comparison.
5. Add tests for:
   - `localhost`
   - `127.0.0.1`
   - `::1`
   - `10.0.0.5`
   - `192.168.1.10`
   - `172.16.0.5`
   - `169.254.169.254`
   - `example.com`
   - decimal/octal/hex IPv4 variants if parser supports them
   - hostnames resolving to private or metadata IPs

Acceptance criteria:

- Coding-agent default policy can validate a local dev server without a scope file.
- Coding-agent denies public hosts unless scoped.
- Ops-agent continues to respect existing scope semantics.
- Error messages are actionable but do not leak internal parser details.

## Phase 5: Split profile-specific resource manifests

Improve `resources/list` and `resources/read`.

Required resources for `ops-agent`:

- `slapper://manifest`
- `slapper://tools`
- `slapper://vulnerabilities`
- `slapper://ops-agent/safety-policy`
- `slapper://ops-agent/task-schema`
- `slapper://ops-agent/event-schema`

Required resources for `coding-agent`:

- `slapper://coding-agent/manifest`
- `slapper://coding-agent/safety-policy`
- `slapper://coding-agent/finding-schema`
- `slapper://coding-agent/workflow`
- `slapper://coding-agent/tool-contracts`

The coding-agent manifest should be optimized for Codegg, not for a human pentester. It should include:

- Stable tool names.
- Exact input schema.
- Output schema.
- Safety policy.
- Expected latency class.
- Whether the tool is deterministic.
- Whether the tool may make network requests.
- Whether it requires a running local service.
- Whether it can be used during normal coding flow or only explicit security-review flow.
- Recommended summarization strategy for the calling agent.

Acceptance criteria:

- `resources/list` for coding-agent does not expose irrelevant operational resources.
- `resources/read` denies profile-mismatched resources.
- Resource texts are generated from the same policy structures used for enforcement where practical.
- Add tests for profile-specific resources.

## Phase 6: Productionize MCP transport behavior

Update `crates/slapper/src/tool/protocol/mcp/routes.rs` and stdio handling.

HTTP mode tasks:

1. Support both single JSON-RPC request objects and batched arrays if not already supported.
   - Current HTTP handler takes `Json<Vec<McpRequest>>`; this can reject standard single-object clients.
   - Implement an enum or custom deserializer:

```rust
#[serde(untagged)]
enum McpIncoming {
    Single(McpRequest),
    Batch(Vec<McpRequest>),
}
```

2. Make max batch size profile-policy driven, not a hardcoded `100`.
3. Apply auth once per request or once per batch consistently.
4. Use request IDs in tracing spans.
5. Return `Content-Type: application/json` consistently.
6. Ensure panic-free error handling for malformed JSON.

STDIO mode tasks:

1. Support single-object JSON-RPC messages, not only arrays.
2. Do not emit normal logs to stdout in stdio mode. Logs must go to stderr or structured file logging.
3. Flush each response line.
4. Preserve JSON-RPC IDs.
5. Add tests for malformed line, single request, batch request, and unknown method.

SSE mode tasks:

1. Ensure stream events include request ID, event type, progress percent if available, and sanitized messages.
2. Add bounded backlog behavior and clear lagged event semantics.
3. Ensure dropped/lost events do not block completion polling via `tools/result`.

Acceptance criteria:

- MCP stdio works with a minimal client sending a single JSON-RPC object per line.
- MCP HTTP works with a single JSON-RPC object and a batch array.
- Batch limits are different by profile if configured.
- No operational logs appear on stdout in stdio mode.

## Phase 7: Add stable coding-agent output schemas

Codegg needs low-noise, machine-readable outputs. Do not return only pretty-printed `ToolResponse` text for coding-agent.

Current issue:

- `tools/call` wraps `ToolResponse` as a JSON string in a text content block.
- This is compatible with basic MCP clients, but it is suboptimal for deterministic agent workflows.

Implementation tasks:

1. Keep the standard MCP content block for compatibility.
2. Add structured JSON content or a stable embedded JSON object when supported by the existing type conventions.
3. For coding-agent tools, normalize output into a `CodingAgentFindingReport`:

```rust
pub struct CodingAgentFindingReport {
    pub schema_version: String,
    pub target: String,
    pub profile: String,
    pub run_id: String,
    pub started_at: String,
    pub completed_at: String,
    pub status: String,
    pub summary: CodingAgentSummary,
    pub findings: Vec<CodingAgentFinding>,
    pub evidence: Vec<CodingAgentEvidence>,
    pub recommended_next_steps: Vec<String>,
    pub patch_relevance: Vec<PatchRelevanceHint>,
    pub limits: AppliedRuntimeLimits,
}
```

A finding should include:

- Stable ID.
- Severity.
- Confidence.
- CWE/CAPEC when known.
- Affected endpoint or route.
- Evidence summary.
- Reproduction note, bounded and safe.
- Likely code area, if inferable.
- Remediation hint.
- Retest command/tool suggestion.

Do not include exploit payload dumps by default in coding-agent output. Include redacted/hashed payload identifiers or short safe excerpts where necessary.

Acceptance criteria:

- Coding-agent `tools/call` returns a stable schema that a coding harness can parse.
- Existing ops-agent output remains compatible.
- Tests validate schema presence and no raw secrets/headers/tokens are included.

## Phase 8: Harden Slapper autonomous agent runtime

The agent runtime is already meaningful. Production work should focus on lifecycle, persistence, status, and failure behavior.

Files to inspect/update:

- `crates/slapper/src/agent/mod.rs`
- `crates/slapper/src/agent/events.rs`
- `crates/slapper/src/agent/logging.rs`
- `crates/slapper/src/agent/memory.rs`
- `crates/slapper/src/agent/portfolio.rs`
- `crates/slapper/src/agent/constraints/*`
- `crates/slapper/src/agent/config_watcher.rs`
- `crates/slapper/src/tool/agents/scheduler.rs`
- `crates/slapper/src/tool/agents/lifecycle.rs`
- `crates/slapper/src/tool/agents/aggregator.rs`

Implementation tasks:

1. Add an explicit `AgentRuntimeStatus` model.

Suggested fields:

```rust
pub struct AgentRuntimeStatus {
    pub running: bool,
    pub started_at: Option<DateTime<Utc>>,
    pub last_tick_at: Option<DateTime<Utc>>,
    pub next_tick_at: Option<DateTime<Utc>>,
    pub portfolio_targets_total: usize,
    pub portfolio_targets_enabled: usize,
    pub last_scan_started_at: Option<DateTime<Utc>>,
    pub last_scan_completed_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub scans_completed: u64,
    pub scans_failed: u64,
    pub alerts_sent: u64,
}
```

2. Wire `slapper agent status` to real state where possible.
   - If the agent is not a daemon yet, status can inspect memory/log/portfolio state and report that no live runtime is attached.
   - Do not fake live status.

3. Persist enough runtime metadata for post-crash inspection.
   - Use a small JSON state file under the configured memory/log directory.
   - Use atomic write/rename.

4. Add graceful shutdown behavior.
   - Stop scheduling new work.
   - Cancel or allow current scan based on config.
   - Flush logs/memory/portfolio.

5. Add controlled scan budgets.
   - Per-target timeout.
   - Per-agent concurrency cap.
   - Per-profile depth cap.
   - Per-target cooldown.

6. Ensure config watcher failures do not crash the agent loop.

7. Ensure one bad target does not block all scheduled scans.

Acceptance criteria:

- `agent run --once` writes deterministic runtime metadata.
- `agent status` reports last run state without panicking.
- Scheduled scan failure records an error and continues to other targets.
- Agent shutdown path is covered by tests using cancellation tokens or test seams.

## Phase 9: Make agent API routes production-safe

The routes in `crates/slapper/src/tool/protocol/agent_routes.rs` are useful, but they need stronger semantics before being considered production grade.

Implementation tasks:

1. Authentication
   - Current API key comparison should support `Authorization: Bearer <token>` and `X-API-Key: <token>` consistently.
   - Avoid treating the full `Authorization` header as the token if it includes `Bearer `.
   - Use constant-time comparison after parsing.

2. Agent registration
   - Validate agent name length and character set.
   - Validate capabilities against a known registry or configured allowlist.
   - Reject duplicate names unless explicitly allowed.
   - Add optional `profile` or `kind`: `ops-worker`, `coding-validator`, `reporter`, etc.

3. Task creation
   - Validate `task_type` against allowed task kinds.
   - Validate payload schema by task type.
   - Enforce max payload size.
   - Enforce profile policy for any target-bearing task.

4. Leasing
   - Set minimum and maximum lease durations.
   - Ensure only agents with matching capability can lease a task.
   - Prefer `lease_next_task` to consider capability match, priority, and due time.

5. Result submission
   - Enforce max result size.
   - Sanitize errors.
   - Track completion/failure metrics.

6. Lifecycle
   - Start `LifecycleManager` from the serving path if this API is enabled.
   - Expose health state read-only.
   - Mark stale agents offline automatically.

7. Callback URL validation
   - Keep and extend current SSRF-resistant validation.
   - Consider rejecting redirects to forbidden IP ranges.
   - Consider resolving host at send time too, not only registration time.

Acceptance criteria:

- Tests cover bearer auth and x-api-key auth.
- Tests reject task payloads with unknown task types.
- Tests reject leasing by an agent without required capability.
- Tests reject oversized results.
- Tests cover stale heartbeat state transitions.

## Phase 10: Add Codegg-specific server ergonomics

The Codegg MCP server should be easy to launch and hard to misuse.

Implementation tasks:

1. Add or document a stable invocation:

```bash
slapper mcp-serve --stdio --profile coding-agent
```

2. Consider adding an alias if appropriate:

```bash
slapper codegg-mcp --stdio
```

Only add a new CLI command if it delegates to the same MCP core and does not duplicate logic.

3. Add a Codegg sample config:

- `examples/codegg-mcp.local.toml`
- `examples/codegg-mcp.scope.toml` or JSON/YAML if scope uses another format

4. Add a Codegg MCP client snippet or README section showing:

- stdio invocation
- expected profile
- target policy
- allowed local URLs
- sample `tools/list`
- sample bounded validation request

5. Ensure `coding-agent` profile has no dependency on OpenAI/Anthropic AI features. It should be deterministic by default. AI can interpret results on the Codegg side.

Acceptance criteria:

- Codegg can launch Slapper as an MCP stdio server with no external network exposure.
- A fresh checkout can run a local-only validation example.
- Docs clearly distinguish Slapper's own agent from Codegg's MCP consumer profile.

## Phase 11: Update documentation

Update these docs:

- `docs/mcp-protocol.md`
- `docs/AGENT.md`
- `architecture/ai_agents.md`
- `docs/CAPABILITIES.md`
- optionally `.opencode/skills/slapper-agent/mcp_protocol.md`

Required doc changes:

1. Explain that Slapper has one MCP implementation with multiple profiles.
2. Document profile names exactly:
   - `ops-agent`
   - `coding-agent`
3. Document startup examples:

```bash
slapper mcp-serve --stdio --profile coding-agent
slapper mcp-serve --port 8081 --profile ops-agent --api-key "$SLAPPER_API_KEY"
```

4. Document coding-agent safety defaults:
   - local/dev target default
   - public internet denied unless scoped
   - stress/packet/root/stealth features denied by default
   - small concurrency/time budgets
5. Document ops-agent expectations:
   - explicit scope strongly recommended
   - portfolios and schedules
   - memory and alerts
   - task leasing if agent API enabled
6. Document structured output schemas.
7. Document migration from older “MCP server exposes all tools” mental model.

Acceptance criteria:

- Docs do not imply there are two separate MCP implementations.
- Docs do not imply coding-agent can use broad operational tools.
- Docs contain a minimal Codegg integration example.
- Docs contain production deployment warnings for network MCP mode.

## Phase 12: Tests and validation matrix

Add tests in the most appropriate existing test modules. Prefer unit tests for policy and route behavior, plus a small integration-style test for stdio/JSON-RPC behavior if feasible.

Minimum test matrix:

Profile tests:

- `McpProfile::default() == OpsAgent`
- `coding-agent` serde roundtrip.
- Policy for `coding-agent` denies broad categories.
- Policy for `ops-agent` preserves broad tools under scope.

Discovery tests:

- `tools/list` for coding-agent returns only allowed tools.
- `tools/list-by-category` for coding-agent returns only allowed categories.
- `resources/list` for coding-agent returns only coding-agent-safe resources.
- `resources/read` denies coding-agent resources under ops-agent if intended, or clearly handles profile mismatch.

Call tests:

- Coding-agent allowed local validation succeeds or reaches dispatcher seam.
- Coding-agent denied stress/load/packet tool fails before dispatcher.
- Coding-agent public target fails without explicit scope.
- Coding-agent excessive timeout/concurrency fails or clamps.
- Ops-agent allowed call still dispatches.

Transport tests:

- HTTP single request object.
- HTTP batch request array.
- Batch over profile max.
- STDIO single request line.
- STDIO batch request line.
- Malformed JSON-RPC line.

Agent runtime tests:

- `run_once` updates state.
- One failed target does not abort remaining targets unless configured fail-fast.
- Shutdown updates running state.
- Portfolio save failure is surfaced and logged.
- Constraint violations skip target and record event.

Agent API tests:

- Bearer auth parsing.
- X-API-Key auth parsing.
- Invalid callback URL rejection.
- Private/loopback callback rejection where appropriate.
- Capability-aware leasing.
- Oversized payload/result rejection.

Validation commands:

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo test -p slapper mcp
cargo test -p slapper agent
```

If `--all-features` is not currently clean because of unrelated existing feature interactions, document the failure and run the narrow feature sets needed for this work:

```bash
cargo test -p slapper --features rest-api
cargo test -p slapper --features "rest-api ai-integration"
cargo test -p slapper --features "rest-api mcp-server"
```

## Phase 13: Suggested implementation order for MiMo 2.5

Follow this order. Do not jump into broad refactors first.

1. Read the files listed in “Current repository state to preserve.”
2. Add MCP profile policy module with tests only. No behavior changes yet.
3. Wire `initialize` safety metadata to the policy module.
4. Filter `tools/list`, `tools/list-by-category`, and manifests by profile.
5. Add call-time policy enforcement in `tools/call`.
6. Add target policy tests for coding-agent.
7. Add single-object JSON-RPC support for HTTP and stdio.
8. Add structured coding-agent output wrapper.
9. Harden agent status/runtime metadata.
10. Harden agent API auth/task validation.
11. Update docs.
12. Run tests and fix regressions.

At each step, keep changes small and commit-ready.

## Implementation notes and guardrails

Prefer deny-by-default for coding-agent. The safe failure mode is refusing to run and explaining the required scope/profile/budget.

Do not infer authorization from the model or client name. Authorization comes from profile, scope, config, and API key.

Do not rely on prompt text as a safety mechanism. Prompts can guide clients, but enforcement belongs in Rust code.

Do not put secrets in tool outputs, logs, event payloads, or MCP errors. Use existing sanitization helpers where available and add tests for redaction.

Do not send normal logs to stdout in stdio MCP mode. It will corrupt the protocol stream.

Do not broaden the Codegg profile to “make tests pass.” Narrow tests to the intended production contract.

Do not remove NSE support. If NSE is exposed through MCP, coding-agent must default to safe/default/version/discovery categories only, and only against explicit local/private/scope-approved targets.

Do not expose stress testing to coding-agent by default. If a future workflow needs load-bearing local validation, implement a separate explicit local-lab profile or runtime confirmation gate.

## Definition of done

This initiative is complete when:

- Slapper has one MCP core with two production-grade profiles.
- `ops-agent` supports the existing Slapper security-agent workflows without losing functionality.
- `coding-agent` exposes a narrow, deterministic, local/scope-bound MCP surface suitable for Codegg.
- Tool discovery and tool execution are both profile-enforced.
- Target scope and runtime budgets are enforced at call time.
- MCP HTTP and stdio modes support standard single-request JSON-RPC objects and batches.
- Coding-agent outputs are structured enough for Codegg to consume without brittle text parsing.
- Agent runtime has real status/state persistence and safe shutdown behavior.
- Agent API routes have production-grade auth parsing, task validation, leasing rules, and callback safety.
- Docs clearly explain the profile model and Codegg integration.
- The relevant tests pass, or any unrelated existing failures are documented with exact commands and failure summaries.

