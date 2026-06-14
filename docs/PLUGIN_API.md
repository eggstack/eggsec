# Web Proxy Plugin API Documentation

## Overview

The web proxy plugin system allows you to extend protocol handling beyond the built-in HTTP/1.1, HTTP/2, WebSocket, and gRPC support. Plugins are Rust types implementing the `ProtocolHandler` trait.

## Plugin Trait

```rust
pub trait ProtocolHandler: Send + Sync {
    /// Return metadata about this plugin.
    fn info(&self) -> PluginInfo;

    /// Detect whether this plugin should handle the given connection.
    fn detect(
        &self,
        host: &str,
        path: &str,
        headers: &HashMap<String, String>,
    ) -> DetectionResult;

    /// Handle a detected protocol session.
    fn handle(
        &self,
        host: &str,
        path: &str,
        headers: &HashMap<String, String>,
        body: Option<&str>,
    ) -> HandleResult;
}
```

## Plugin Info

```rust
pub struct PluginInfo {
    pub id: String,           // Unique identifier (e.g., "my-protocol")
    pub name: String,         // Human-readable name
    pub version: String,      // SemVer version
    pub description: String,  // What the plugin handles
}
```

## Detection Result

```rust
pub enum DetectionResult {
    Detected {
        confidence: f64,           // 0.0 - 1.0
        protocol_name: String,     // Detected protocol name
        context: HashMap<String, String>,  // Protocol-specific context
    },
    NotDetected,
}
```

## Handle Result

```rust
pub struct HandleResult {
    pub findings: Vec<PluginFinding>,  // Security findings
    pub metadata: HashMap<String, String>,  // Session metadata
}
```

## Example Plugin

```rust
use eggsec::proxy::intercept::plugins::*;

struct MyProtocolHandler;

impl ProtocolHandler for MyProtocolHandler {
    fn info(&self) -> PluginInfo {
        PluginInfo {
            id: "my-proto".to_string(),
            name: "My Protocol".to_string(),
            version: "0.1.0".to_string(),
            description: "Handles custom binary protocol".to_string(),
        }
    }

    fn detect(
        &self,
        _host: &str,
        _path: &str,
        headers: &HashMap<String, String>,
    ) -> DetectionResult {
        if headers.get("x-protocol").map(|v| v.as_str()) == Some("my-proto") {
            DetectionResult::Detected {
                confidence: 0.95,
                protocol_name: "my-proto".to_string(),
                context: HashMap::new(),
            }
        } else {
            DetectionResult::NotDetected
        }
    }

    fn handle(
        &self,
        _host: &str,
        _path: &str,
        _headers: &HashMap<String, String>,
        body: Option<&str>,
    ) -> HandleResult {
        // Custom handling logic
        let findings = vec![PluginFinding {
            plugin_id: "my-proto".to_string(),
            title: "Custom Protocol Detected".to_string(),
            description: "Protocol handling complete".to_string(),
            severity: 0,
            metadata: HashMap::new(),
        }];

        HandleResult {
            findings,
            metadata: HashMap::new(),
        }
    }
}
```

## Plugin Registry

```rust
use eggsec::proxy::intercept::plugins::*;

let mut registry = PluginRegistry::new();
registry.register(Box::new(MyProtocolHandler)).unwrap();

// List plugins
let plugins = registry.list();

// Detect protocol
let (handler, result) = registry.detect("example.com", "/", &headers).unwrap();
```

## Sandbox Security

Plugins run in a sandboxed environment with capability-based restrictions:

```rust
use eggsec::proxy::intercept::plugins::*;

// Restricted sandbox (minimal capabilities)
let sandbox = PluginSandbox::restricted();

// Permissive sandbox (full capabilities)
let sandbox = PluginSandbox::permissive();

// Check capabilities
sandbox.check_capability(&PluginCapability::ReadMetadata)?;
sandbox.check_memory(1024 * 1024)?;
sandbox.check_execution_time(100)?;
```

## Available Capabilities

| Capability | Description |
|-----------|-------------|
| `ReadMetadata` | Read connection metadata (host, path, headers) |
| `ReadBodies` | Read request/response bodies |
| `WriteData` | Modify request/response data |
| `NetworkAccess` | Access network connections |
| `FileSystem` | Access file system |
| `SpawnTasks` | Spawn background tasks |
| `CryptoAccess` | Access cryptographic operations |
| `RegisterProtocols` | Register new protocol handlers |

## Built-in Plugins

### NonStandardPortHandler

Detects services running on non-standard ports (not 80, 443, 8080, 8443, 3000, 5000, 9090).

```rust
let handler = NonStandardPortHandler;
let result = handler.detect("example.com:9999", "/", &headers);
// Returns Detected { confidence: 0.6, ... }
```
