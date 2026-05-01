---
name: mcp_protocol
description: "MCP (Model Context Protocol) server integration for AI agents"
triggers:
  - mcp
  - model context protocol
  - ai agent tool
  - mcp server
  - mcp client
metadata:
  category: api
  tools: [rest-api, tool-api]
  scope: targets
---

## Overview

⚠️ **Note**: The `mcp-server` feature has been removed. Use `rest-api` instead for API integrations. This skill is retained for reference only.

The MCP (Model Context Protocol) server provides a JSON-RPC 2.0 API for AI agents to interact with Slapper's security tools. It's designed for integration with LLM-powered assistants that need to execute security testing workflows.

## Architecture

The MCP server exposes security tools via JSON-RPC 2.0 with:
- Tool discovery and listing
- Synchronous and streaming tool execution
- Session management
- Rate limiting
- Authentication via API key

## Usage

### Starting MCP Server

```bash
# Start with default settings
slapper serve

# With API key authentication
slapper serve --api-key your-key-here

# With scope restrictions
slapper serve --api-key your-key-here --scope-file scope.toml

# With custom port
slapper serve --port 8080
```

### MCP Client Integration

```python
import json
import requests

# Example Python client
def call_mcp_tool(tool_name: str, target: str, **kwargs):
    """Call an MCP tool via JSON-RPC"""
    response = requests.post(
        "http://localhost:8080/mcp",
        json={
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": tool_name,
                "arguments": {
                    "target": target,
                    **kwargs
                }
            }
        },
        headers={"Authorization": "Bearer your-api-key"}
    )
    return response.json()

# List available tools
def list_tools():
    response = requests.post(
        "http://localhost:8080/mcp",
        json={
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list",
            "params": {}
        }
    )
    return response.json()
```

## Available Methods

| Method | Description |
|-------|-------------|
| `initialize` | Initialize MCP protocol connection |
| `tools/list` | List all available security tools |
| `tools/list-by-category` | Tools grouped by category |
| `tools/call` | Execute a security tool |
| `tools/cancel` | Cancel running tool execution |
| `tools/result` | Get tool execution result |
| `tools/history` | Get execution history |
| `session/create` | Create assessment session |
| `session/get` | Get session details |
| `session/list` | List all sessions |
| `resources/list` | List available resources |
| `resources/read` | Read resource content |
| `prompts/list` | List builtin prompts |
| `prompts/read` | Get prompt template |
| `ping` | Health check |

## Tool Categories

- **Recon**: DNS, SSL/TLS, technology detection
- **Scanner**: Port scanning, endpoint discovery
- **Fuzzer**: SQLi, XSS, command injection
- **Auth**: Brute force, credential stuffing
- **Proxy**: HTTP interception
- **Load**: HTTP stress testing

## Triggers

Keywords that activate this skill:
- "mcp"
- "ai agent"
- "tool execution"
- "json-rpc"
- "agent integration"