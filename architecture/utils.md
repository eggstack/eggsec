# Utils Module

## Overview

Common utility functions and helpers used throughout the codebase for HTTP handling, URL parsing, scope checking, and output formatting.

## Submodules (23 files)

| Module | Purpose |
|--------|---------|
| `auth` | `constant_time_eq()` for timing-safe comparison |
| `cache` | Generic TTL cache utilities |
| `circuit_breaker` | `CircuitBreaker`, `CircuitState` - fault tolerance pattern |
| `client_pool` | `ClientPool`, `OptimizedClientPool` - HTTP client reuse |
| `error` | Error utility helpers |
| `formatting` | `strip_controls()`, `preserve_all()` - string truncation |
| `http` | `create_http_client()` family - HTTP client creation with proxy, TLS options |
| `logging` | Logging utility helpers |
| `network` | Network utility functions |
| `output` | Terminal output helpers (colors, icons) |
| `parsing` | URL and header parsing utilities |
| `progress` | Progress bar helpers |
| `rate_limiter` | Rate limiting utilities |
| `redaction` | Secret redaction helpers |
| `scope` | `check_scope()` - target scope validation |
| `service_detection` | Service detection utilities |
| `stealth` | Stealth/scanning evasion utilities |
| `target` | Target extraction and normalization |
| `urlencoding` | URL encoding/decoding helpers |
| `validation` | Input validation utilities |
| `privilege` | Privilege escalation checks (feature-gated: `stress-testing` or `packet-inspection`) |

## Key Re-exports

- `CircuitBreaker`, `CircuitState` from `circuit_breaker`
- `ClientPool`, `OptimizedClientPool` from `client_pool`
- `strip_controls`, `preserve_all` from `formatting`
- `create_http_client` family from `http`
