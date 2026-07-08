# Utils Module

## Overview

Common utility functions and helpers used throughout the codebase for HTTP handling, URL parsing, scope checking, and output formatting.

## Submodules (23 files, 21 declared in mod.rs)

| Module | Purpose |
|--------|---------|
| `auth` | `constant_time_eq()` for timing-safe comparison |
| `cache` | Generic TTL cache utilities |
| `circuit_breaker` | `CircuitBreaker`, `CircuitState` - fault tolerance pattern |
| `client_pool` | `ClientPool`, `OptimizedClientPool` - HTTP client reuse |
| `error` | Error message sanitization utilities |
| `formatting` | `strip_controls()`, `preserve_all()` - string truncation |
| `http` | `create_http_client()` family - HTTP client creation with proxy, TLS options |
| `logging` | Logging utility helpers |
| `network` | TCP connection utilities with Nagle's algorithm disabled |
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
- `constant_time_eq` from `auth`
- `sanitize_for_logging` from `logging`

### HTTP Client Functions (`http`)

| Function | Description |
|----------|-------------|
| `create_http_client(timeout_secs)` | Create HTTP client with default options |
| `create_http_client_with_options(...)` | Create HTTP client with full configuration |
| `create_http_client_with_proxy(proxy, timeout)` | Create HTTP client with proxy |
| `create_insecure_client_with_options(...)` | Create HTTP client that skips TLS verification |
| `create_insecure_http_client(timeout_secs)` | Create HTTP client that skips TLS verification (simplified) |
| `get_shared_http_client(timeout_secs)` | Get or create a shared HTTP client singleton |
| `get_shared_insecure_http_client(timeout_secs)` | Get or create a shared insecure HTTP client singleton |

### Network (`network`)

| Function | Description |
|----------|-------------|
| `connect_with_nodelay(addr)` | TCP connect with Nagle's algorithm disabled |
| `connect_with_nodelay_timeout(addr, timeout)` | TCP connect with Nagle's algorithm disabled and timeout |

### Output (`output`)

| Function | Description |
|----------|-------------|
| `print_error(msg)` | Print error-styled message to stderr |
| `print_info(msg)` | Print info-styled message to stdout |
| `print_json(value)` | Print pretty-printed JSON |
| `print_json_compact(value)` | Print compact JSON |
| `print_success(msg)` | Print success-styled message to stdout |
| `print_warning(msg)` | Print warning-styled message to stderr |

### Parsing (`parsing`)

| Function | Description |
|----------|-------------|
| `contains_ignore_case(haystack, needle)` | Case-insensitive substring search |
| `parse_headers(headers)` | Parse header strings into key-value pairs |
| `parse_url_validated(url)` | Parse and validate a URL string |

### Scope (`scope`)

| Function | Description |
|----------|-------------|
| `check_scope(target, scope)` | Check if a target is within scope |
| `check_scope_from_url(url, scope)` | Check if a URL's host is within scope |

### Target (`target`)

| Function | Description |
|----------|-------------|
| `extract_domain(target)` | Extract domain from a target string |
| `extract_host_port(target)` | Extract host and port from a target |
| `extract_target_from_url(url)` | Extract target from a URL |
| `is_ip_address(target)` | Check if a target is an IP address |
| `normalize_url(url)` | Normalize a URL |
| `parse_host_port(host_port)` | Parse a "host:port" string |
| `parse_socket_addr(addr)` | Parse a socket address |
| `strip_url_protocol(url)` | Strip protocol prefix from a URL |

### Validation (`validation`)

| Function | Description |
|----------|-------------|
| `validate_concurrency(value)` | Validate concurrency setting (1..=1000) |
| `validate_git_repo_path(path)` | Validate a git repository path |
| `validate_path(path)` | Validate a file system path exists |
| `validate_path_string(path)` | Validate a path string |
| `validate_rate_limit(value)` | Validate rate limit setting |
| `validate_timeout(value)` | Validate timeout setting |
| `validate_url(url)` | Validate a URL string |

### Privilege (`privilege`, feature-gated: `stress-testing` or `packet-inspection`)

| Function | Description |
|----------|-------------|
| `check_privileged()` | Check if running with elevated privileges |
| `is_root()` | Check if running as root (Unix) |
| `require_root()` | Require root, return error if not |
