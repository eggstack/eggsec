# MCP Protocol Reference

Slapper's MCP (Model Context Protocol) server provides AI agents with secure, structured access to security testing tools.

## Quick Start

```bash
# Start MCP server
slapper mcp-serve --port 8081

# Or with authentication
slapper mcp-serve --port 8081 --api-key your-secret-key

# STDIO mode for direct AI integration
slapper mcp-serve --stdio
```

## Endpoints

| Method | Path | Description |
|--------|------|-------------|
| POST | `/mcp` | JSON-RPC 2.0 API |
| GET | `/mcp/stream/:request_id` | SSE streaming |
| GET | `/openapi.json` | OpenAPI 3.1 spec (JSON) |
| GET | `/openapi.yaml` | OpenAPI 3.1 spec (YAML) |
| POST | `/plan` | Execution plan generator |
| GET | `/health` | Health check |

## Authentication

API key authentication is optional. Pass the key via:
- Header: `X-API-Key: your-key`
- Header: `Authorization: Bearer your-key`
- Query param: `?api_key=your-key`

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
    "serverInfo": {
      "name": "slapper-mcp",
      "version": "0.1.0"
    },
    "capabilities": {
      "tools": true,
      "streaming": true,
      "sessions": true
    },
    "toolCount": 10
  }
}
```

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

#### `tool/execute`
Execute a specific tool.

```json
{
  "jsonrpc": "2.0",
  "id": 4,
  "method": "tool/execute",
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
Create a scan session.

```json
{
  "jsonrpc": "2.0",
  "id": 6,
  "method": "session/create",
  "params": {
    "target": "https://example.com",
    "scan_type": "full_assessment"
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

#### `session/update`
Update session status.

```json
{
  "jsonrpc": "2.0",
  "id": 9,
  "method": "session/update",
  "params": {
    "session_id": "abc123",
    "status": "in_progress",
    "progress": 50
  }
}
```

#### `session/delete`
Delete a session.

```json
{
  "jsonrpc": "2.0",
  "id": 10,
  "method": "session/delete",
  "params": {
    "session_id": "abc123"
  }
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
    "requests_per_minute": 60,
    "concurrent_limit": 5,
    "current_usage": 1,
    "burst_remaining": 9
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
Read a specific resource.

```json
{
  "jsonrpc": "2.0",
  "id": 13,
  "method": "resources/read",
  "params": {
    "uri": "slapper://manifest"
  }
}
```

## SSE Streaming

Subscribe to real-time events for a request:

```
GET /mcp/stream/your-request-id
```

**Headers:**
- `Accept: text/event-stream`

**Events:**

```sse
event: progress
data: {"type":"progress","progress":25,"message":"Scanning ports..."}

event: finding
data: {"type":"finding","severity":"high","title":"Open Port Found","description":"Port 22 is open"}

event: complete
data: {"type":"complete","status":"success","findings":5}
```

## Execution Planning

### POST /plan

Generate an execution plan for a security assessment.

**Request:**
```json
{
  "goal": "full_assessment",
  "target": "https://example.com",
  "target_type": "Web",
  "attack_surfaces": ["Web", "Api", "Network"],
  "max_duration_ms": 3600000,
  "include_load_testing": false,
  "include_stress_testing": false
}
```

**Goals:**
- `recon` - Reconnaissance only
- `vuln_scan` - Vulnerability scanning
- `full_assessment` - Complete assessment (default)
- `api` - API security testing
- `quick` - Quick scan

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

Default configuration (Standard):
- 60 requests per minute
- 5 concurrent scans
- 10 burst allowance

Configurable via TOML:

```toml
[rate_limit]
enabled = true
requests_per_minute = 60
concurrent_scans = 5
burst_allowance = 10
```

Presets: `standard` (60/5/10), `relaxed` (120/10/20), `strict` (30/2/5)

## Session Persistence

Sessions are stored on disk at `~/.slapper/sessions/` with:
- 1-hour TTL (configurable)
- Automatic cleanup
- Max 100 sessions (configurable)

## Tool Categories

| Category | Description | Example Tools |
|----------|-------------|---------------|
| Recon | Reconnaissance | DNS, subdomain, tech detection |
| Scanning | Port & endpoint discovery | Port scan, fingerprinting |
| Fuzzing | Vulnerability testing | SQL injection, XSS, SSRF |
| Waf | WAF detection/bypass | Detection, stress testing |
| LoadTest | Performance testing | HTTP load, stress |
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
- `-32001` - Unauthorized
- `-32002` - Rate limit exceeded
- `-32003` - Session not found

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
    "method": "tool/execute",
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
  -d '[{"jsonrpc":"2.0","id":1,"method":"tool/execute","params":{"name":"recon","arguments":{"target":"https://example.com"}}}]'
```
