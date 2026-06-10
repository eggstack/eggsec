> **Status**: This is a historical planning document. The NSE crate has been extracted to `crates/eggsec-nse/`. Some paths reference the original `src/nse/` location before extraction.

# Tokio Migration Plan for NSE

## Overview

This document outlines the plan for migrating the NSE (Nmap Scripting Engine) implementation from blocking I/O to async I/O using Tokio.

## Current State

### Dependencies Already Available
- `tokio` (v1) - Already in Cargo.toml with full features
- `reqwest` (v0.12) - Already supports async
- `mlua` (v0.11) - Already has async support via `async` feature

### Blocking I/O Locations Identified (227 occurrences)

**Protocol Libraries using `std::net::TcpStream`:**
- ftp, smtp, mysql, postgres, mssql, redis, mongodb, ldap, snmp, smb, rdp, vnc
- ntp, memcached, imap, pop3, netbios, oracle, winrm, radius, dhcp
- ssh, telnet, sftp, whois, finger, elasticsearch, kafka, mqtt, websocket, http2
- ajp, afp, amqp, tns, sip, tftp, upnp, tns, ncp, ndmp, nrpc, citrixxml

**HTTP Libraries:**
- `http.rs` - Uses `reqwest::blocking::Client`
- `httpspider.rs` - Uses `reqwest::blocking::Client`

**UDP Libraries:**
- dhcp, dhcp6, ntp, radius, stun, tftp - Use `std::net::UdpSocket`

**File I/O:**
- `io.rs` - Uses `std::fs::File`

---

## Migration Strategy

### Phase 1: Core Runtime Infrastructure

#### 1.1 Update mlua Configuration
```toml
# Cargo.toml
mlua = { version = "0.11", features = ["async", "tokio"] }
```

#### 1.2 Create Async Executor Wrapper
```rust
// src/nse/async_executor.rs

use tokio::runtime::Runtime;
use std::sync::Arc;

pub struct AsyncNseExecutor {
    lua: Lua,
    runtime: Runtime,
    // ...
}

impl AsyncNseExecutor {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let runtime = Runtime::new()?;
        
        let lua = Lua::async_builder()
            .runtime(Arc::new(runtime.clone()))
            .build()?;
        
        Ok(Self { lua, runtime })
    }
}
```

#### 1.3 Design Async Function Pattern
```rust
// Each library function will return Future
// Example for HTTP:
http.set(
    "get",
    lua.create_async_function(
        |lua, (host, port, path, options): (String, u16, String, Option<Table>)| {
            async move {
                let url = build_url(&host, port, &path);
                let client = get_async_client().await;
                
                match client.get(&url).send().await {
                    Ok(resp) => build_response(lua, resp).await,
                    Err(e) => error_response(lua, e),
                }
            }
        }
    )?,
)?;
```

---

### Phase 2: HTTP Library Migration

#### 2.1 Update HTTP Library
```rust
// src/nse/libraries/http.rs

use reqwest::Client;
use once_cell::sync::Lazy;

// Replace blocking client with async
static ASYNC_HTTP_CLIENT: Lazy<Client> = Lazy::new(|| {
    Client::builder()
        .timeout(Duration::from_secs(30))
        .danger_accept_invalid_certs(true)
        .pool_max_idle_per_host(20)
        .http2_adaptive_window(true)
        .build()
        .expect("Failed to create async HTTP client")
});

pub fn register_http_library(lua: &Lua) -> LuaResult<()> {
    let http = lua.create_table()?;
    
    // Convert all functions to async
    http.set(
        "get",
        lua.create_async_function(
            |lua, (host, port, path, options): (String, u16, String, Option<Table>)| {
                async move {
                    let url = build_url(&host, port, &path);
                    let response = ASYNC_HTTP_CLIENT.get(&url).send().await
                        .map_err(|e| mlua::Error::RuntimeError(e.to_string()))?;
                    
                    build_response(lua, response).await
                }
            }
        )?,
    )?;
    
    // Similar for post, put, delete, request, etc.
}
```

#### 2.2 Update httpspider Library
```rust
// src/nse/libraries/httpspider.rs

use scraper::Html;
use futures::stream::StreamExt;

pub fn register_httpspider_library(lua: &Lua) -> LuaResult<()> {
    let httpspider = lua.create_table()?;
    
    // Crawl becomes async generator
    httpspider.set(
        "crawl",
        lua.create_async_function(
            |lua, (url, options): (String, Option<Table>)| {
                async move {
                    let client = get_async_client().await;
                    let mut queue: VecDeque<String> = VecDeque::new();
                    let mut visited: HashSet<String> = HashSet::new();
                    
                    queue.push_back(url.clone());
                    
                    while let Some(current_url) = queue.pop_front() {
                        if visited.contains(&current_url) {
                            continue;
                        }
                        visited.insert(current_url.clone());
                        
                        if let Ok(resp) = client.get(&current_url).send().await {
                            if let Ok(body) = resp.text().await {
                                // Parse and queue links
                                // Yield results to Lua
                            }
                        }
                    }
                    
                    Ok(())
                }
            }
        )?,
    )?;
}
```

---

### Phase 3: Socket Library Migration

#### 3.1 Create Async Socket Wrapper
```rust
// src/nse/libraries/socket.rs

use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub fn register_socket_library(lua: &Lua) -> LuaResult<()> {
    let socket = lua.create_table()?;
    
    // TCP connect becomes async
    socket.set(
        "connect",
        lua.create_async_function(
            |lua, (host, port): (String, u16)| {
                async move {
                    let addr = format!("{}:{}", host, port);
                    let stream = TcpStream::connect(&addr).await
                        .map_err(|e| mlua::Error::RuntimeError(e.to_string()))?;
                    
                    // Store stream in connection pool
                    let fd = store_connection(stream).await;
                    
                    let result = lua.create_table()?;
                    result.set("fd", fd)?;
                    result.set("host", host)?;
                    result.set("port", port)?;
                    Ok(result)
                }
            }
        )?,
    )?;
    
    // Read becomes async
    socket.set(
        "receive",
        lua.create_async_function(
            |lua, (fd, size): (i32, usize)| {
                async move {
                    let mut stream = get_connection(fd).await
                        .ok_or_else(|| mlua::Error::RuntimeError("Invalid fd".into()))?;
                    
                    let mut buf = vec![0u8; size];
                    let n = stream.read(&mut buf).await
                        .map_err(|e| mlua::Error::RuntimeError(e.to_string()))?;
                    
                    buf.truncate(n);
                    Ok(String::from_utf8_lossy(&buf).to_string())
                }
            }
        )?,
    )?;
}
```

#### 3.2 UDP Support
```rust
// For UDP, use tokio::net::UdpSocket

socket.set(
    "send",
    lua.create_async_function(
        |lua, (host, port, data): (String, u16, String)| {
            async move {
                let socket = UdpSocket::bind("0.0.0.0:0").await
                    .map_err(|e| mlua::Error::RuntimeError(e.to_string()))?;
                
                let sent = socket.send_to(data.as_bytes(), format!("{}:{}", host, port)).await
                    .map_err(|e| mlua::Error::RuntimeError(e.to_string()))?;
                
                Ok(sent)
            }
        }
    )?,
)?;
```

---

### Phase 4: Protocol Library Migrations

#### 4.1 MySQL Example
```rust
// src/nse/libraries/mysql.rs

use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use bytes::BytesMut;

pub fn register_mysql_library(lua: &Lua) -> LuaResult<()> {
    let mysql = lua.create_table()?;
    
    mysql.set(
        "connect",
        lua.create_async_function(
            |lua, (host, port, user, password): (String, u16, String, String)| {
                async move {
                    let addr = format!("{}:{}", host, port);
                    let mut stream = TcpStream::connect(&addr).await
                        .map_err(|e| mlua::Error::RuntimeError(e.to_string()))?;
                    
                    // Perform MySQL handshake
                    // ... async read/write
                    
                    let result = lua.create_table()?;
                    result.set("host", host)?;
                    result.set("user", user)?;
                    Ok(result)
                }
            }
        )?,
    )?;
    
    mysql.set(
        "query",
        lua.create_async_function(
            |lua, (connection, query): (Table, String)| {
                async move {
                    // Get stream from connection
                    let stream = get_mysql_stream(connection).await;
                    
                    // Send query packet
                    // ... async write
                    
                    // Read response
                    let mut buf = BytesMut::new();
                    stream.read_buf(&mut buf).await
                        .map_err(|e| mlua::Error::RuntimeError(e.to_string()))?;
                    
                    // Parse and return results
                    Ok(parse_mysql_result(lua, &buf))
                }
            }
        )?,
    )?;
}
```

#### 4.2 Apply to All Protocols
- **Strategy**: Migrate one protocol family at a time
- **Order**: Start with most commonly used (http, mysql, redis, smb)
- **Pattern**: Each protocol follows same async pattern

---

### Phase 5: File I/O Migration

#### 5.1 Async File Operations
```rust
// src/nse/libraries/io.rs

use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub fn register_io_library(lua: &Lua) -> LuaResult<()> {
    let io = lua.create_table()?;
    
    io.set(
        "open",
        lua.create_async_function(
            |lua, (filename, mode): (String, Option<String>)| {
                async move {
                    let file = tokio::fs::File::open(&filename).await
                        .map_err(|e| mlua::Error::RuntimeError(e.to_string()))?;
                    
                    // Store in async file handle pool
                    let fd = store_file(file).await;
                    
                    let result = lua.create_table()?;
                    result.set("fd", fd)?;
                    Ok(result)
                }
            }
        )?,
    )?;
}
```

---

## Implementation Checklist

### Phase 1: Infrastructure
- [ ] Update Cargo.toml for async mlua
- [ ] Create AsyncNseExecutor wrapper
- [ ] Design async function patterns
- [ ] Test basic async Lua execution

### Phase 2: HTTP
- [ ] Migrate http.rs to async
- [ ] Migrate httpspider.rs to async
- [ ] Add connection pooling with tokio
- [ ] Test HTTP scripts

### Phase 3: Sockets
- [ ] Create async socket wrapper
- [ ] Implement TCP connection pool
- [ ] Implement UDP async operations
- [ ] Test socket scripts

### Phase 4: Protocols
- [ ] Migrate MySQL
- [ ] Migrate Redis
- [ ] Migrate SMB
- [ ] Migrate remaining protocols (~40 libraries)

### Phase 5: File I/O
- [ ] Migrate io.rs to async
- [ ] Test file operations

---

## Breaking Changes

### API Changes
1. **Function Signatures**: All network functions become async
   ```lua
   -- Before (blocking)
   local response = http.get(host, port, path)
   
   -- After (async)
   local response = http.get(host, port, path) -- Still works, but internally async
   ```

2. **Return Types**: May need adjustment for async iterators

### Performance Impact
- **Positive**: 10-100x better concurrency for I/O-bound scripts
- **Negative**: Slightly higher memory overhead per connection

---

## Testing Strategy

1. **Unit Tests**: Test each async function individually
2. **Integration Tests**: Run actual NSE scripts
3. **Benchmarking**: Compare blocking vs async performance
4. **Compatibility Tests**: Ensure NSE script compatibility

---

## Rollout Plan

1. **Internal Testing**: 2-3 weeks
2. **Beta Release**: With opt-in flag `--async-nse`
3. **Stable Release**: Default enabled after 2 releases

---

## Dependencies Required

```toml
# Additional dependencies for async migration
tokio = { version = "1", features = ["rt-multi-thread", "net", "sync", "time", "macros", "fs", "io-util"] }
reqwest = { version = "0.12", features = ["json", "rustls-tls", "socks", "http2"] }  # Remove "blocking"
mlua = { version = "0.11", features = ["async", "tokio"] }
bytes = "1"
futures = "0.3"
```

---

## Notes

- mlua 0.11 already supports async via `async` and `tokio` features
- The existing tokio dependency can be leveraged
- Connection pooling should use `dashmap` for async-safe maps
- Consider using `tokio-postgres`, `redis` crates for mature async protocol implementations
