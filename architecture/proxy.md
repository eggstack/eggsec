# Proxy Module

## Purpose

Proxy pool management supporting SOCKS4/5, HTTP CONNECT, and HTTPS proxies with health checking, rotation strategies, and chain proxying.

## Key Types

| Type | Location | Description |
|------|----------|-------------|
| `ProxyManager` | `proxy/mod.rs` | Central proxy orchestrator (pool + rotator + health checker) |
| `ProxiedConnection` | `proxy/mod.rs` | Connection routed through proxy chain |
| `ProxyPool` | `proxy/pool.rs` | Thread-safe proxy pool with statistics tracking |
| `ProxyRotator` | `proxy/rotator.rs` | Rotation strategy selector (round-robin, random, weighted, least-used, lowest-latency) |
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
| `intercept/mod.rs` | Intercepting proxy server with dynamic SSL cert generation |
| `intercept/cert.rs` | `CertGenerator` — on-the-fly SSL certificate generation for HTTPS interception |
| `intercept/interceptor.rs` | `InterceptProxy`, `InterceptConfig`, `InterceptMode` — request/response interception with pause/modify/continue |
| `intercept/rules.rs` | `InterceptRule`, `RuleAction`, `RuleSet` — configurable interception rules (Allow, Block, Intercept, Monitor, Modify) |

### Intercept Submodule (`intercept/`)

An HTTP/HTTPS intercepting proxy for security testing. Key components:

- **`ProxyServer`** (`intercept/mod.rs`): Binds a TCP listener, handles CONNECT tunneling and plain HTTP requests, applies rule evaluation, and generates dynamic TLS certificates for HTTPS interception.
- **`CertGenerator`** (`intercept/cert.rs`): Generates per-host SSL certificates using `rcgen` with a 24-hour validity cache.
- **`InterceptProxy`** (`intercept/interceptor.rs`): Higher-level proxy with `InterceptMode` (Monitor, Intercept, Allow) and channel-based request/response modification.
- **`RuleSet`** (`intercept/rules.rs`): Evaluates host/path patterns against rules with `RuleAction` variants: `Allow`, `Block`, `Intercept`, `Monitor`, `Modify`.

## Key Methods on `ProxyManager`

| Method | Location | Description |
|--------|----------|-------------|
| `create_chained_connection()` | `mod.rs:156-218` | Creates a connection through a chain of proxies. Validates chain length against healthy pool size, selects a chain via `ProxyRotator::select_chain()`, and enforces SOCKS5/Tor-only chains for multi-hop. Falls back to single-proxy connection for length 1. |
| `start_background_health_check()` | `mod.rs:224-266` | Spawns a tokio task that periodically checks all proxies in the pool. Runs on a configurable interval, marks proxies healthy/unhealthy via `HealthChecker::check_all()`, and logs results. Returns a `JoinHandle<()>` for lifecycle management. |

## Implementation Status

Fully implemented. Complete proxy management with pool, rotation, health checking, SOCKS/HTTP CONNECT, proxy chaining, background health checks, and an intercepting proxy submodule with dynamic TLS. Includes an `AGENTS.override.md` for specialized guidance.
