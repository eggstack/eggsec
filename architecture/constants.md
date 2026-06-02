# Constants

## Overview

Centralized constants in `crates/slapper/src/constants.rs` for magic numbers, strings, and default values.

## Top-Level Constants

| Constant | Value | Description |
|----------|-------|-------------|
| `PROJECT_QUALIFIER` | `"tools"` | Project qualifier |
| `PROJECT_NAME` | `"slapper"` | Project name |
| `DEFAULT_EXPORT_DIR` | `"./exports"` | Default export directory |
| `DEFAULT_REMOTE_PORT` | `7890` | Default remote port |
| `DEFAULT_TRACEROUTE_PORT` | `33434` | Default traceroute port |
| `DEFAULT_PROXY_TIMEOUT_MS` | `10000` | Proxy timeout (ms) |
| `DEFAULT_HEALTH_CHECK_INTERVAL_SECS` | `60` | Health check interval |
| `DEFAULT_MAX_HEALTH_CHECK_FAILURES` | `3` | Max health check failures |
| `WAYBACK_SNAPSHOT_LIMIT` | `100` | Wayback Machine snapshot limit |
| `DEFAULT_ICMP_PAYLOAD_SIZE` | `56` | ICMP payload size |
| `DEFAULT_CONFIG_FILE` | `"slapper.toml"` | Default config filename |
| `DEFAULT_WORDLIST` | `"wordlists/directories.txt"` | Default wordlist |
| `SUPPORTED_WAF_COUNT` | `34` | Number of WAF detectors |

## Nested Modules

### http
`DEFAULT_TIMEOUT_SECS: 30`, `DEFAULT_MAX_REDIRECTS: 10`, `DEFAULT_CONCURRENCY: 10`

### scan
`DEFAULT_PORT_RANGE: "1-1024"`, `DEFAULT_PORT_CONCURRENCY: 100`, `DEFAULT_ENDPOINT_CONCURRENCY: 20`

### cache
`DEFAULT_TTL_SECS: 3600`, `DEFAULT_MAX_ENTRIES: 10000`

### nvd
`DEFAULT_RATE_LIMIT_DELAY_MS: 6000`

### ui
`CHECK_MARK: "✓"`, `CROSS_MARK: "✗"`, `ARROW: "→"`, `WIDTH_DEFAULT: 58`

### waf
WAF detection scoring constants: `HEADER_MATCH_SCORE: 25`, `COOKIE_MATCH_SCORE: 20`, `BODY_MATCH_SCORE: 15`, `IP_MATCH_SCORE: 20`, `UNKNOWN_WAF_CONFIDENCE: 30`, `LENGTH_DIFF_THRESHOLD: 100`, `HIGH_CONFIDENCE_EXIT: 90`, `BLOCKED_STATUS_CODES: [403, 406, 429, 503]`, and pattern arrays.
