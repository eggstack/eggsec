# Proxy Module

## Purpose

Proxy pool management supporting SOCKS4/5, HTTP CONNECT, and HTTPS proxies with health checking, rotation strategies, and chain proxying.

## Key Types

| Type | Location | Description |
|------|----------|-------------|
| `ProxyManager` | `proxy/mod.rs` | Central proxy orchestrator (pool + rotator + health checker) |
| `ProxiedConnection` | `proxy/mod.rs` | Connection routed through proxy chain |
| `ProxyPool` | `proxy/pool.rs` | Thread-safe proxy pool with statistics tracking |
| `ProxyRotator` | `proxy/rotator.rs` | Rotation strategy selector (round-robin, least-used, random) |
| `HealthChecker` | `proxy/health.rs` | Async health checking for proxy endpoints |
| `ProxyHealth` | `proxy/health.rs` | Health status of a proxy |
| `ProxyConfig` | `proxy/config.rs` | Proxy pool configuration |
| `ProxyEntry` | `proxy/config.rs` | Individual proxy definition (host, port, type, auth) |
| `ProxyType` | `proxy/config.rs` | Enum: Http, Https, Socks4, Socks5 |
| `HealthCheckConfig` | `proxy/config.rs` | Health check configuration |

## Files

| File | Description |
|------|-------------|
| `mod.rs` | Module root: `ProxyManager`, `ProxiedConnection`, proxy connection logic |
| `config.rs` | `ProxyConfig`, `ProxyEntry`, `ProxyType`, file loading |
| `pool.rs` | `ProxyPool` with stats tracking and health filtering |
| `rotator.rs` | `ProxyRotator` with multiple rotation strategies |
| `health.rs` | `HealthChecker` with periodic health checks |
| `socks.rs` | SOCKS4/5 connection implementation |
| `http_connect.rs` | HTTP CONNECT tunnel implementation |
| `intercept/` | Proxy interception submodule |

## Implementation Status

Fully implemented. Complete proxy management with pool, rotation, health checking, and SOCKS/HTTP CONNECT support. Includes an `AGENTS.override.md` for specialized guidance.
