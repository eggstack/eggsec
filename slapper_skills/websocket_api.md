---
name: websocket_api
description: "WebSocket API for real-time updates and pub/sub"
triggers:
  - websocket
  - real-time
  - ws-api
  - streaming
metadata:
  category: api
  tools: [rest-api]
  scope: targets
---

## Overview

Slapper's REST API includes WebSocket support for real-time updates and pub/sub functionality. The WebSocket endpoint enables clients to receive live notifications about scan progress, findings, and agent status.

## Usage

### Starting WebSocket Server

```bash
# Start REST API with WebSocket support (requires rest-api,ws-api features)
cargo run --features rest-api,ws-api -- --rest-api --listen 127.0.0.1:8080

# Or via slapper CLI
slapper rest-api --listen 127.0.0.1:8080 --ws-enable
```

### Connecting WebSocket Client

```javascript
// JavaScript client example
const ws = new WebSocket('ws://127.0.0.1:8080/ws');

ws.onopen = () => {
  console.log('Connected to Slapper WebSocket');
  // Subscribe to topics
  ws.send(JSON.stringify({
    type: 'subscribe',
    topics: ['findings:example.com', 'scan_progress', 'agent_status']
  }));
};

ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  console.log('Received:', data);
};

ws.onclose = () => {
  console.log('Disconnected');
};
```

### Python client

```python
import websocket

ws = websocket.create_connection("ws://127.0.0.1:8080/ws")
ws.send(json.dumps({
    "type": "subscribe", 
    "topics": ["findings:example.com"]
}))
while True:
    data = ws.recv()
    print(json.loads(data))
```

## Message Format

### Server → Client

```json
{
  "type": "finding",
  "target": "https://example.com",
  "severity": "high",
  "title": "SQL Injection",
  "timestamp": "2026-04-24T12:00:00Z"
}

{
  "type": "scan_progress",
  "target": "https://example.com",
  "phase": "fuzzing",
  "progress": 45,
  "total": 100
}

{
  "type": "agent_status",
  "running": true,
  "targets": 3,
  "last_scan": "2026-04-24T11:00:00Z"
}
```

### Client → Server

```json
{
  "type": "subscribe",
  "topics": ["findings:example.com", "scan_progress", "agent_status"]
}

{
  "type": "unsubscribe",
  "topics": ["scan_progress"]
}

{
  "type": "ping"
}
```

## Topics

| Topic | Description | Update Frequency |
|-------|-------------|-----------------|
| `findings:{target}` | Findings for specific target | On new finding |
| `scan_progress` | All scan progress updates | On progress change |
| `agent_status` | Agent status changes | On status change |
| `alerts` | Alert notifications | On alert trigger |

## CLI Features

```bash
# List available WebSocket topics
slapper rest-api ws-topics

# Enable WebSocket logging
slapper rest-api --listen 127.0.0.1:8080 --ws-log
```

## Triggers

Keywords that activate this skill:
- websocket
- real-time
- ws-api
- streaming
- pub/sub
- live updates