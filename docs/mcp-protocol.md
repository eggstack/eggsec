# MCP Protocol Reference

Eggsec's MCP (Model Context Protocol) server provides AI agents with secure, structured access to security testing tools.

## Quick Start

```bash
# Start MCP server (ops-agent profile, default)
eggsec mcp-serve --port 8081

# Or with authentication
eggsec mcp-serve --port 8081 --api-key your-secret-key

# STDIO mode for direct AI integration
eggsec mcp-serve --stdio

# Coding agent profile (stdio + restricted tools)
eggsec mcp-serve --stdio --profile coding-agent

# Shorthand for coding agent via codegg-mcp subcommand (alias: mcp-codegg)
eggsec codegg-mcp
```

## MCP Profiles

Eggsec has **one** MCP implementation with multiple profiles that control available tools, safety policies, and output schemas.

| Profile | Server Name | Description |
|---------|-------------|-------------|
| `ops-agent` | `eggsec-tool-api` | Full security testing toolkit for AI agents. All tools, unconstrained. |
| `coding-agent` | `eggsec-coding-agent-mcp` | Bounded live security validation tools for coding agents. Restricted tools, enforced safety. |

### Ops-Agent Profile

The default profile. Provides full access to all registered MCP-exposable tools.
Designed for AI agents operating in controlled environments with human oversight.

- **Target policy:** `ScopeOrLocalDevOnly` default, external network allowed
- **Concurrency:** Up to 50 concurrent scans
- **Timeout:** Up to 600 seconds per tool
- **Stress testing:** Allowed
- **Broad recon:** Allowed
- **Packet features:** Allowed
- **Explicit scope:** Required
- **Sessions:** Enabled
- **`/plan` endpoint:** Enabled

### Coding-Agent Profile

Optimized for AI coding assistants that need to validate security while writing code. Enforces strict safety defaults.

- **Target policy:** Localhost, loopback, and private IPs only (`ScopeOrLocalDevOnly`)
- **Concurrency:** Max 5 concurrent scans
- **Timeout:** Max 60 seconds per tool
- **Batch size:** Max 10
- **Stress testing:** Denied
- **Broad recon:** Denied
- **External network:** Denied (unless explicitly scoped)
- **Explicit scope:** Required
- **Sessions:** Denied (`session/*` is rejected)
- **`/plan` endpoint:** Denied

The allowlist is a hardcoded exact match on six tool IDs:
`scan`, `scan-ports`, `fingerprint`, `endpoints`, `waf-detect`, `search`. The
`stress-testing` and `load-testing` categories are explicitly denied, and
argument keys such as `stealth` are stripped.

Source: `crates/eggsec/src/tool/protocol/mcp/policy.rs` (`ops_agent()` at :100,
`coding_agent()` at :120).

**Coding-agent resources** (available via `resources/read`):

| URI | Description |
|-----|-------------|
| `eggsec://coding-agent/manifest` | Available tools and recommended workflow |
| `eggsec://coding-agent/safety-policy` | Safety defaults, hard caps, allowed targets |
| `eggsec://coding-agent/finding-schema` | Schema for structured findings output |
| `eggsec://coding-agent/workflow` | Recommended validation workflow |
| `eggsec://coding-agent/tool-contracts` | Per-tool input/output contracts |

### Startup Examples

```bash
# Ops-agent (HTTP mode)
eggsec mcp-serve --port 8081

# Ops-agent (STDIO mode)
eggsec mcp-serve --stdio

# Coding-agent (STDIO mode)
eggsec mcp-serve --stdio --profile coding-agent

# Coding-agent shorthand
eggsec codegg-mcp

# Coding-agent with scope file (global --scope flag; mcp-serve has no --scope-file)
eggsec mcp-serve --stdio --profile coding-agent --scope scope.toml

# Coding-agent with example config
# See examples/codegg-mcp.local.toml
eggsec mcp-serve --stdio --profile coding-agent
```

**Enforcement (2026-06-10):** MCP server forces `McpStrict` via `EnforcementContext::evaluate` (central boundary: LoadedScope provenance, explicit manifest required for networked ops, DenialClass/positive-capability checks, capabilities populated). Preferred production constructor: `McpServer::with_enforcement`. `tools/call` computes full `PolicyDecision` via `policy_decision_for_mcp_call_with_enforcement` (via `EnforcementContext`) and embeds it in error `data`.

## Endpoints

| Method | Path | Description |
|--------|------|-------------|
| POST | `/mcp` | JSON-RPC 2.0 API |
| POST | `/json-rpc` | Alias for `/mcp` |
| GET | `/mcp/stream/{request_id}` | SSE streaming (`{request_id}` or `*` for all) |
| GET | `/openapi.json` | OpenAPI 3.1 spec (JSON) |
| GET | `/openapi.yaml` | OpenAPI 3.1 spec (YAML) |
| POST | `/plan` | Execution plan generator (ops-agent only) |
| GET | `/health` | Health check (`"service": "eggsec-mcp"`) |

Routes are registered in `crates/eggsec/src/tool/protocol/mcp/routes.rs:153`.

## Authentication

API key authentication is optional and compared in constant time
(`crates/eggsec/src/tool/protocol/mcp/auth.rs`). Pass the key via:
- Header: `Authorization: Bearer your-key` (a bare key without the `Bearer `
  prefix is also accepted)
- Header: `X-API-Key: your-key`
- JSON-RPC param: `params.api_key = "your-key"`

When no `--api-key` is configured, every request is authorized.

## JSON-RPC API

### Methods

#### `initialize`
Get server capabilities.

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {}
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "protocolVersion": "2024-11-05",
    "capabilities": {
      "tools": { "listChanged": true },
      "sessions": true,
      "roots": { "listChanged": true },
      "streaming": true
    },
    "serverInfo": {
      "name": "eggsec-tool-api",
      "version": "0.1.0",
      "description": "..."
    },
    "profile": "ops-agent",
    "safety": { }
  }
}
```

There is no `toolCount` field. `serverInfo.name` is profile-dependent:
`eggsec-tool-api` for `ops-agent`, `eggsec-coding-agent-mcp` for
`coding-agent`. The `capabilities` flags are driven by the profile policy --
`sessions`, `streaming` and the `safety` block reflect the active profile, and
`sessions` is `false` under `coding-agent`.

Source: `crates/eggsec/src/tool/protocol/mcp/handlers/server.rs:348`,
`crates/eggsec/src/tool/protocol/mcp/profile.rs:17`.

#### `tools/list`
List all available tools.

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/list",
  "params": {}
}
```

#### `tools/list-by-category`
List tools filtered by category.

```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "tools/list-by-category",
  "params": {
    "category": "Recon"
  }
}
```

**Categories:** `Recon`, `Scanning`, `Fuzzing`, `Waf`, `LoadTest`, `Stress`, `Pipeline`

#### `tools/call`
Execute a specific tool.

```json
{
  "jsonrpc": "2.0",
  "id": 4,
  "method": "tools/call",
  "params": {
    "name": "recon",
    "arguments": {
      "target": "https://example.com"
    }
  }
}
```

#### `ping`
Health check.

```json
{
  "jsonrpc": "2.0",
  "id": 5,
  "method": "ping",
  "params": {}
}
```

#### `session/create`
Create a scan session. Only the `target` param is read; `scan_type` is ignored.

```json
{
  "jsonrpc": "2.0",
  "id": 6,
  "method": "session/create",
  "params": {
    "target": "https://example.com"
  }
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 6,
  "result": {
    "session_id": "abc123",
    "created_at": "2024-01-15T10:30:00Z",
    "target": "https://example.com",
    "status": "pending",
    "scopes": 0,
    "findings_count": 0
  }
}
```

#### `session/get`
Get session details.

```json
{
  "jsonrpc": "2.0",
  "id": 7,
  "method": "session/get",
  "params": {
    "session_id": "abc123"
  }
}
```

#### `session/list`
List all sessions.

```json
{
  "jsonrpc": "2.0",
  "id": 8,
  "method": "session/list",
  "params": {}
}
```

#### `rate-limit/status`
Get rate limit status.

```json
{
  "jsonrpc": "2.0",
  "id": 11,
  "method": "rate-limit/status",
  "params": {}
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 11,
  "result": {
    "client_id": "...",
    "tokens_available": 9,
    "requests_this_minute": 1,
    "requests_per_minute": 60,
    "concurrent_available": 4,
    "concurrent_limit": 5
  }
}
```

#### `resources/list`
List available resources.

```json
{
  "jsonrpc": "2.0",
  "id": 12,
  "method": "resources/list",
  "params": {}
}
```

#### `resources/read`
Read a specific resource. URIs are profile-scoped.

```json
{
  "jsonrpc": "2.0",
  "id": 13,
  "method": "resources/read",
  "params": {
    "uri": "eggsec://coding-agent/manifest"
  }
}
```

**Available URIs:**

| Profile | URIs |
|---------|------|
| `coding-agent` | `eggsec://coding-agent/manifest`, `/safety-policy`, `/finding-schema`, `/workflow`, `/tool-contracts` |
| `ops-agent` | `eggsec://ops-agent/safety-policy`, `/task-schema`, `/event-schema` |
| `roots/list` | `eggsec://config`, `eggsec://tools`, `eggsec://templates`, `eggsec://payloads` |

#### `prompts/list` and `prompts/read`
List and read server prompts (`crates/eggsec/src/tool/protocol/mcp/prompts.rs`).

#### `roots/list`
List roots exposed by the server.

```json
{
  "jsonrpc": "2.0",
  "id": 14,
  "method": "roots/list",
  "params": {}
}
```

#### `shutdown`
Request a graceful server shutdown.

#### Additional tool methods

`tools/call-stream`, `tools/cancel`, `tools/history` and `tools/result` are also
dispatched alongside `tools/list`, `tools/list-by-category` and `tools/call`.

The complete dispatch table is in
`crates/eggsec/src/tool/protocol/mcp/handlers/server.rs:289-300`.

## Structured Output Schemas

The coding-agent profile returns structured JSON output for all tool executions,
typed as `CodingAgentFindingReport` / `CodingAgentFinding` / `CodingAgentEvidence`
/ `CodingAgentSummary` (`crates/eggsec/src/tool/protocol/mcp/coding_agent_output.rs`):

```json
{
  "schema_version": "1.0",
  "target": "https://localhost:3000",
  "profile": "coding-agent",
  "run_id": "req-001",
  "status": "completed",
  "findings": [
    {
      "id": "0f8f...",
      "title": "Missing security header",
      "category": "vulnerability",
      "severity": "high",
      "confidence": "high",
      "observed_behavior": "X-Content-Type-Options header is not set",
      "evidence": [
        { "type": "raw", "content": "..." }
      ],
      "patch_relevance": "..."
    }
  ],
  "summary": {
    "total_findings": 1,
    "by_severity": { "high": 1 }
  }
}
```

### Finding Schema

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Stable finding identifier (UUID) |
| `title` | string | Short human-readable title |
| `category` | string | e.g. `vulnerability`, `open_port`, `endpoint` |
| `severity` | string | `critical`, `high`, `medium`, `low`, `info` |
| `confidence` | string | Confidence assessment |
| `observed_behavior` | string | What was observed during the scan |
| `evidence` | array | `CodingAgentEvidence` items (`type` + `content`) |
| `patch_relevance` | string | How the finding relates to merge readiness |

Exploit payload dumps are omitted by default so output stays safe to embed in
issue trackers and code review comments. The report has no top-level `tool` or
`metadata` block -- the old `{profile, tool, status, findings, metadata}` shape
is not what the server emits.

## SSE Streaming

Subscribe to real-time events for a request:

```
GET /mcp/stream/your-request-id
```

**Headers:**
- `Accept: text/event-stream`

**Events:**

Server-emitted events carry the `event_type` of each `StreamEvent`
(`crates/eggsec/src/tool/protocol/mcp/streaming.rs:4`), so the exact set depends
on the in-flight tool call. The transport always adds these two:

```sse
event: heartbeat
data: {"timestamp": "alive"}

event: lagged
data: {"lagged_events": 12}
```

axum also sends a keep-alive comment every 15 seconds. Subscribing to
`GET /mcp/stream/*` delivers events for every request rather than one.

## Execution Planning

### POST /plan

Generate an execution plan for a security assessment.

**Request:**
```json
{
  "goal": "full_assessment",
  "target": "https://example.com",
  "target_type": "web",
  "attack_surfaces": ["web", "network"],
  "max_duration_ms": 3600000,
  "include_load_testing": false,
  "include_stress_testing": false
}
```

`target_type` is `web`, `api`, `network` or `mixed` (lowercase).

**Goals:**
- `recon` (aliases `reconnaissance`, `discovery`) - Reconnaissance only
- `vuln_scan` (aliases `vulnerability_scan`, `fuzz`) - Vulnerability scanning
- `full_assessment` (aliases `full`, `complete`) - Complete assessment
- `api` (alias `api_security`) - API security testing
- `quick` (alias `fast`) - Quick scan

**Response:**
```json
{
  "stages": [
    {
      "name": "reconnaissance",
      "tools": [
        {
          "tool_id": "recon",
          "capability": "full_recon",
          "attack_surface": ["web", "network"],
          "estimated_duration_ms": 30000
        }
      ],
      "parallel": true,
      "depends_on": []
    },
    {
      "name": "vulnerability_scanning",
      "tools": [...],
      "parallel": true,
      "depends_on": ["reconnaissance"]
    }
  ],
  "estimated_duration_ms": 120000,
  "total_tools": 5
}
```

## Rate Limiting

Default configuration (`standard`):
- 60 requests per minute
- 5 concurrent scans
- burst size 10

The full `RateLimitConfig` also supports `per_endpoint_limits`,
`global_rate_limit` and `enable_ip_based_limiting`
(`crates/eggsec-tool-core/src/ratelimit.rs:6`):

```toml
[rate_limit]
requests_per_minute = 60
concurrent_scans = 5
burst_size = 10
```

Presets: `standard` (60/5/10), `relaxed` (300/10/25), `strict` (20/2/5)

## Sessions

Sessions are backed by `eggsec_runtime::SessionManager` and are only available
under the `ops-agent` profile -- the `coding-agent` profile sets
`allow_sessions: false` and rejects the `session/*` methods.

## Tool Categories

`ToolCategory` (`crates/eggsec/src/tool/traits.rs:10`) has seven variants:

| Category | Description | Example Tools |
|----------|-------------|---------------|
| Recon | Reconnaissance | DNS, subdomain, tech detection |
| Scanning | Port & endpoint discovery | Port scan, fingerprinting |
| Fuzzing | Vulnerability testing | SQL injection, XSS, SSRF |
| Waf | WAF detection/bypass | Detection, stress testing |
| LoadTest | Performance testing | HTTP load |
| Stress | Network stress testing | SYN/UDP/ICMP flood |
| Pipeline | Orchestrated testing | Full assessment, quick scan |

## Error Responses

```json
{
  "jsonrpc": "2.0",
  "id": null,
  "error": {
    "code": -32601,
    "message": "Method not found",
    "data": "Unknown method: tools/invalid"
  }
}
```

**Error Codes:**
- `-32600` - Invalid Request
- `-32601` - Method not found
- `-32602` - Invalid params
- `-32603` - Internal error
- `-32020` - `ToolDenied` (tool not allowed by the profile policy)
- `-32021` - `ArgumentDenied` (argument key blocked by profile policy)
- `-32022` - `ConcurrencyExceeded`
- `-32023` - `TimeoutExceeded`
- `-32024` - `TargetDenied`
- `-32025` - Enforcement/policy decision failure

`tools/call` computes the full `PolicyDecision` via
`policy_decision_for_mcp_call_with_enforcement` and embeds it in the error
`data`, so a denial response carries the decision, not just a message
(`crates/eggsec/src/tool/protocol/mcp/policy.rs:407`).

## Example Usage

### Python Client

```python
import httpx
import json

client = httpx.Client(base_url="http://localhost:8081")

# Initialize
resp = client.post("/mcp", json=[{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "initialize",
    "params": {}
}])
print(resp.json())

# List tools
resp = client.post("/mcp", json=[{
    "jsonrpc": "2.0",
    "id": 2,
    "method": "tools/list",
    "params": {}
}])

# Execute recon
resp = client.post("/mcp", json=[{
    "jsonrpc": "2.0",
    "id": 3,
    "method": "tools/call",
    "params": {
        "name": "recon",
        "arguments": {"target": "https://example.com"}
    }
}])

# Generate plan
resp = client.post("/plan", json={
    "goal": "full_assessment",
    "target": "https://example.com"
})
print(resp.json())
```

### curl

```bash
# Health check
curl http://localhost:8081/health

# Get OpenAPI spec
curl http://localhost:8081/openapi.json

# List tools
curl -X POST http://localhost:8081/mcp \
  -H "Content-Type: application/json" \
  -d '[{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}]'

# Execute tool
curl -X POST http://localhost:8081/mcp \
  -H "Content-Type: application/json" \
  -d '[{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"recon","arguments":{"target":"https://example.com"}}}]'
```
