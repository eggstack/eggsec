# Constants

## Overview

Centralized constants in `crates/slapper/src/constants.rs` for magic numbers, strings, and default values.

## Top-Level Constants

| Constant | Type | Value | Description |
|----------|------|-------|-------------|
| `PROJECT_QUALIFIER` | `&str` | `"tools"` | Project qualifier |
| `PROJECT_NAME` | `&str` | `"slapper"` | Project name |
| `DEFAULT_EXPORT_DIR` | `&str` | `"./exports"` | Default export directory |
| `DEFAULT_REMOTE_PORT` | `u16` | `7890` | Default remote port |
| `DEFAULT_CONFIG_FILE` | `&str` | `"slapper.toml"` | Default config filename |
| `DEFAULT_MAX_RETRIES` | `u32` | `3` | Default max retries |
| `DEFAULT_RETRY_DELAY_MS` | `u64` | `1000` | Default retry delay (ms) |
| `DEFAULT_POOL_IDLE_TIMEOUT_SECS` | `u64` | `30` | Connection pool idle timeout |
| `DEFAULT_POOL_MAX_IDLE_PER_HOST` | `usize` | `20` | Max idle connections per host |
| `DEFAULT_TOOL_TIMEOUT_MS` | `u64` | `30000` | Default tool timeout (ms) |
| `DEFAULT_BROWSER_TIMEOUT_MS` | `u64` | `60000` | Browser timeout (ms) |
| `BROWSER_TIMEOUT_BUFFER_MS` | `u64` | `10000` | Browser timeout buffer (ms) |
| `DEFAULT_PROXY_TIMEOUT_MS` | `u64` | `10000` | Default proxy timeout (ms) |
| `DEFAULT_TASK_QUEUE_CAPACITY` | `usize` | `10000` | Task queue capacity |
| `DEFAULT_LEASE_DURATION_MS` | `u64` | `300000` | Agent lease duration (ms) |
| `DEFAULT_SCHEDULER_RETRY_DELAY_MS` | `u64` | `30000` | Scheduler retry delay (ms) |
| `MAX_REQUESTS_PER_SECOND_LIMIT` | `u32` | `10000` | Max requests/sec limit |
| `STATUS_RATE_LIMITED` | `u16` | `429` | HTTP 429 status |
| `STATUS_FORBIDDEN` | `u16` | `403` | HTTP 403 status |
| `STATUS_LOCKED` | `u16` | `423` | HTTP 423 status |
| `STATUS_SERVER_ERROR` | `u16` | `503` | HTTP 503 status |
| `SUPPORTED_WAF_COUNT` | `usize` | `34` | Number of WAF detectors |

## Nested Modules

### http
`DEFAULT_TIMEOUT_SECS: u64 = 30`, `DEFAULT_MAX_REDIRECTS: u32 = 10`, `DEFAULT_CONCURRENCY: usize = 10`

### scan
`DEFAULT_PORT_CONCURRENCY: usize = 100`

### cache
`DEFAULT_TTL_SECS: u64 = 3600`

### waf
WAF detection scoring constants: `MAX_REDIRECTS: usize = 5`, `HEADER_MATCH_SCORE: u16 = 25`, `COOKIE_MATCH_SCORE: u16 = 20`, `BODY_MATCH_SCORE: u16 = 15`, `IP_MATCH_SCORE: u16 = 20`, `UNKNOWN_WAF_CONFIDENCE: u16 = 30`, `LENGTH_DIFF_THRESHOLD: usize = 100`, `HIGH_CONFIDENCE_EXIT: u16 = 90`, `BLOCKED_STATUS_CODES: [u16; 4] = [403, 406, 429, 503]`, `BLOCKED_PATTERNS: [&str; 8]`, `WEAK_BLOCK_INDICATOR_PATTERNS: [&str; 4]`, `UNKNOWN_WAF_WEAK_PATTERN_THRESHOLD: usize = 2`.
