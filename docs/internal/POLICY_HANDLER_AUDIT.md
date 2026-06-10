# Policy Handler Audit

## Date: 2026-06-10

## Summary

All command handlers and protocol execution paths were audited for adoption of
the shared policy evaluation path (`evaluate_and_enforce_operation` /
`evaluate_operation_policy`). High-risk handlers have been migrated to use
`OperationDescriptor`-based policy checks.

## Migration Status

### Migrated Handlers (using `evaluate_and_enforce_operation`)

| Path | Entry point | Target-bearing? | Current policy path | Risk tier | Status | Notes |
|------|-------------|-----------------|---------------------|-----------|--------|-------|
| `handlers/stress.rs` | `handle_stress` | yes | `evaluate_and_enforce_operation` | StressTest | migrated | requires `stress-testing` feature |
| `handlers/stress.rs` | `handle_proxy` (Add) | yes | `evaluate_and_enforce_operation` | ExploitAdjacent | migrated | requires `stress-testing` feature |
| `handlers/stress.rs` | `handle_proxy` (Test) | yes | `evaluate_and_enforce_operation` | ExploitAdjacent | migrated | requires `stress-testing` feature |
| `handlers/fuzz.rs` | `handle_waf_stress` | yes | `evaluate_and_enforce_operation` | Intrusive | migrated | requires `stress-testing` feature |
| `handlers/fuzz.rs` | `handle_fuzz` | yes | `evaluate_and_enforce_operation` | Intrusive | migrated | |
| `handlers/fuzz.rs` | `handle_waf` | yes | `evaluate_and_enforce_operation` | Intrusive | migrated | |
| `handlers/network.rs` | `handle_packet` (Send) | yes | `evaluate_and_enforce_operation` | RawPacket | migrated | requires `packet-inspection` feature |
| `handlers/network.rs` | `handle_packet` (Traceroute) | yes | `evaluate_and_enforce_operation` | RawPacket | migrated | requires `packet-inspection` feature |
| `handlers/network.rs` | `handle_icmp` | yes | `evaluate_and_enforce_operation` | SafeActive | migrated | requires `stress-testing` feature |
| `handlers/network.rs` | `handle_traceroute` | yes | `evaluate_and_enforce_operation` | RawPacket | migrated | requires `stress-testing` feature |
| `handlers/cluster.rs` | `handle_exec` | yes | `evaluate_and_enforce_operation` | RemoteExecution | migrated | |
| `handlers/cluster.rs` | `handle_remote` (Start) | yes | `evaluate_and_enforce_operation` | RemoteExecution | migrated | mode: HazardousLab |
| `handlers/scan.rs` | `handle_nse` | yes | `evaluate_and_enforce_operation` | Intrusive | migrated | requires `nse` feature |
| `handlers/load.rs` | `handle_load` | yes | `evaluate_and_enforce_operation` | LoadTest | migrated | |
| `handlers/scan.rs` | `handle_scan_ports` | yes | `evaluate_and_enforce_operation` | SafeActive | migrated | |
| `handlers/scan.rs` | `handle_scan_endpoints` | yes | `evaluate_and_enforce_operation` | SafeActive | migrated | |
| `handlers/scan.rs` | `handle_fingerprint` | yes | `evaluate_and_enforce_operation` | SafeActive | migrated | |
| `handlers/scan.rs` | `handle_scan` | yes | `evaluate_and_enforce_operation` | SafeActive | migrated | |
| `handlers/scan.rs` | `handle_resume` | yes | `evaluate_and_enforce_operation` | SafeActive | migrated | |
| `handlers/fuzz.rs` | `handle_graphql` | yes | `evaluate_and_enforce_operation` | Intrusive | migrated | |
| `handlers/fuzz.rs` | `handle_oauth` | yes | `evaluate_and_enforce_operation` | Intrusive | migrated | |
| `handlers/recon.rs` | `handle_recon` | yes | `evaluate_and_enforce_operation` | SafeActive | migrated | |
| `handlers/hunt.rs` | `handle_hunt` | yes | `evaluate_and_enforce_operation` | Intrusive | migrated | requires `advanced-hunting` feature |
| `handlers/auth_test.rs` | `handle_auth_test` | yes | `evaluate_and_enforce_operation` | CredentialTesting | migrated | |
| `handlers/wireless.rs` | `handle_wireless` | yes | `evaluate_and_enforce_operation` | SafeActive | migrated | requires `wireless` feature |
| `handlers/browser.rs` | `handle_browser` | yes | `evaluate_and_enforce_operation` | SafeActive | migrated | requires `headless-browser` feature |
| `handlers/grpc.rs` | `handle_grpc_server` | yes | `evaluate_and_enforce_operation` | SafeActive | migrated | requires `grpc-api` feature |

### Handlers Using `evaluate_operation_policy` Directly (no bail on deny)

These handlers call `evaluate_operation_policy` to read policy decisions for
preview/output purposes rather than to enforce denials:

| Path | Entry point | Target-bearing? | Current policy path | Risk tier | Status | Notes |
|------|-------------|-----------------|---------------------|-----------|--------|-------|
| `handlers/plan.rs` | `handle_plan` | yes | `evaluate_operation_policy` (direct) | variable per stage | deferred | planning-only tool, no enforcement |
| `handlers/explain.rs` | `handle_policy_explain` | yes | delegates to CLI helper | variable | deferred | explain-only, no enforcement |
| `handlers/explain.rs` | `handle_scope_explain` | yes | `evaluate_operation_policy` (direct) | Passive | deferred | explain-only, no enforcement |

### Scope-Only Handlers (use `ensure_scope`/`ensure_scope_url`)

None remaining. All target-bearing handlers now use `evaluate_and_enforce_operation`.

### No Policy Evaluation (no target or delegated enforcement)

| Path | Entry point | Target-bearing? | Current policy path | Risk tier | Status | Notes |
|------|-------------|-----------------|---------------------|-----------|--------|-------|
| `handlers/cluster.rs` | `handle_cluster` (Worker/Coordinator/Status/AddTask) | no | none | no-target | no-target | infrastructure management |
| `handlers/agent.rs` | `handle_agent` | no | none | no-target | no-target | agent management; tool ops enforced by runtime |
| `handlers/serve.rs` | `handle_serve` | no | none | no-target | no-target | server startup |
| `handlers/serve.rs` | `handle_mcp_serve` | no | none | no-target | no-target | MCP server startup |
| `handlers/config.rs` | `handle_config` | no | none | no-target | no-target | config validation |
| `handlers/doctor.rs` | `handle_doctor` | no | none | no-target | no-target | dependency check |
| `handlers/notify.rs` | `handle_notify` | no | none | no-target | no-target | webhook testing |
| `handlers/report.rs` | `handle_report` | no | none | no-target | no-target | report generation |
| `handlers/sbom.rs` | `handle_sbom` | no | none | no-target | no-target | requires `sbom` feature |
| `handlers/storage.rs` | `handle_storage` | no | none | no-target | no-target | database operations |
| `handlers/vuln.rs` | `handle_vuln` | no | none | no-target | no-target | CVSS scoring |
| `handlers/ci.rs` | `handle_ci` | no | none | no-target | no-target | CI gate |
| `handlers/ai_analyze.rs` | `handle_ai_analyze` | no | none | no-target | no-target | requires `ai-integration` feature |

## Protocol Layer Audit

### MCP (`tool/protocol/mcp/handlers/server.rs`)

- **Entry point:** `handle_tools_call()` (JSON-RPC `tools/call`)
- **Dispatches tool calls:** Yes (`dispatcher.dispatch()`)
- **Policy enforcement:** Yes — `McpProfilePolicy::validate_tool_call()` + `validate_target()` + `policy_decision_for_mcp_call()`
- **Structured denials:** Yes — `PolicyDecision` serialized in MCP error `data` field
- **Status:** fully migrated

### REST API (`tool/protocol/rest.rs`)

- **Entry point:** `execute_tool()` (POST `/api/v1/tools/{tool_id}/execute`)
- **Dispatches tool calls:** Yes (`dispatcher.dispatch()`)
- **Policy enforcement:** Scope check only (`scope.is_target_allowed()`)
- **Structured denials:** No — plain `EggsecError` -> HTTP status
- **Status:** scope-only (profile-based restrictions not applicable; REST is a direct API)

### gRPC (`tool/protocol/grpc.rs`)

- **Entry point:** `execute_tool()` via `ToolServiceImpl` (gRPC `ToolService`)
- **Dispatches tool calls:** Yes (`dispatcher.dispatch()`)
- **Policy enforcement:** API key auth only
- **Structured denials:** No — `Status::internal`
- **Status:** no policy enforcement beyond auth

### OpenAI-Compatible (`tool/protocol/openai/handlers.rs`)

- **Entry point:** `chat_completions()` (POST `/v1/chat/completions`)
- **Dispatches tool calls:** Yes (via streaming or non-streaming paths)
- **Policy enforcement:** Scope check only (`scope.is_target_allowed()`)
- **Structured denials:** No
- **Status:** scope-only

### OpenResponses (`tool/protocol/openresponses/handlers.rs`)

- **Entry point:** `create_response()` (POST `/v1/responses`)
- **Dispatches tool calls:** Yes
- **Policy enforcement:** None
- **Structured denials:** No
- **Status:** no policy enforcement

## Notes

- The `CommandContext::evaluate_and_enforce_operation` method wraps
  `evaluate_operation_policy` and bails with structured output (JSON or
  human-readable) on denial.
- All target-bearing handlers now build an `OperationDescriptor` with correct
  mode, risk, intended use, and target.
- 18 regression tests cover policy denial behavior for all risk tiers
  (SafeActive, Intrusive, StressTest, RawPacket, LoadTest, RemoteExecution,
  ExploitAdjacent, CredentialTesting) plus scope enforcement, JSON-mode
  structured denial output, and human-mode denial readability.
- The MCP module (`tool/protocol/mcp/`) has its own policy enforcement via
  `McpProfilePolicy::validate_tool_call` and `validate_target`, which are
  wired into the `tools/call` JSON-RPC handler. MCP denials now include
  structured `PolicyDecision` data in the error response.
- REST and gRPC protocol paths are scope-only or no-policy. These are
  considered acceptable for their use case: REST has auth + rate limiting +
  scope; gRPC has auth. Neither has profile-based restrictions (those are
  MCP-specific).
