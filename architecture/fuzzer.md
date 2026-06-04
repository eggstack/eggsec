# Fuzzer Module

The Fuzzer is the most advanced part of Slapper, designed to find vulnerabilities by sending semi-random or specifically crafted data to a target and analyzing the response.

## Core Architecture (`src/fuzzer/`)

### Fuzzing Engine (`engine/`)

The core loop that manages targets, payloads, and detections.

- **Core (`core.rs`)**: `FuzzEngine` struct — main entry point, builds HTTP client, manages configuration.
- **Execution (`execution.rs`)**: Implements Sequential (one at a time), Burst (concurrent), and Adaptive (rate-limited) modes.
- **Types (`types.rs`)**: `FuzzResult`, `FuzzSession`, `OwaspSummary` and related types.
- **Utils (`utils.rs`)**: Payload mutation, session building, diffing orchestration, URL construction. Contains `WAF_BLOCKED_STATUS_CODES`.
- **Chained (`chained.rs`)**: `StatefulFuzzer` for multi-step fuzz chains.
- **Advanced (`advanced.rs`)**: Engine-level advanced fuzzing orchestration.

### Payloads (`payloads/`)

Slapper comes with a vast library of payloads for different vulnerability types. The `PayloadType` enum defines 30 categories:

- **Injection**: SQLi, XSS, Command Injection, Template Injection, LDAP, XXE, NoSQL, XPath, Expression.
- **File System**: Path Traversal.
- **Logic**: Authentication bypass (JWT, OAuth), IDOR, Prototype Pollution, Mass Assignment.
- **Server-Side**: SSRF, ReDoS, Deserialization, Race Condition.
- **Client-Side**: Open Redirect, CSV Injection.
- **API Security**: GraphQL, gRPC, WebSocket, SOAP.
- **Infrastructure**: Host Header Injection, Cache Poisoning, Compression Bombs, Header Expansion, OAST.

Each payload type has its own module (e.g., `sqli.rs`, `xss.rs`). The `payload_vec!` macro in `macros.rs` builds payload vectors from inline data.

### Detection (`detection/`)

Algorithms for identifying if a fuzzing attempt was successful.

- **Pattern Matching (`aho_corasick.rs`)**: Aho-Corasick multi-pattern matcher for leak detection (database errors, stack traces, file paths, sensitive data, credentials, debug info).
- **Timing Analysis (`analyzer.rs`)**: `TimingAnalyzer` detects response time anomalies using IQR (Interquartile Range) baselines. Internal stats (total requests, anomaly counts) use lock-free atomics, but `record()` requires `&mut self` and is wrapped in `Arc<Mutex<>>` at the call site. Handles NaN values explicitly to prevent panics.
- **Detection Patterns (`patterns.rs`)**: Raw pattern lists for SQL errors, stack traces, file paths, credentials, AWS keys, and connection strings.

### Diffing (`diff.rs`)

`ResponseDiffer` compares fuzzed responses against a baseline "clean" request. Tracks status changes, header differences, body length anomalies, cookie changes, and timing anomalies with a weighted anomaly score.

### Session Management (`state.rs`)

`HttpSession` tracks cookies, tokens, headers, and state data across requests. `SessionManager` provides async session storage. `AuthHandler` supports Basic, Bearer, API Key, and OAuth2/JWT authentication.

### Mutator (`mutator.rs`)

`generate_mutations()` is the public API entry point (re-exported from `fuzzer/mod.rs`). Internally it uses `Mutator` to apply transformations to payloads: case toggle, URL encoding, double URL encoding, null byte injection, duplication, truncation, prefix/suffix addition, comment insertion, whitespace manipulation, reversal, and swapping.

### Rate Limiting (`rate_limit.rs`)

- `AdaptiveRateLimiter`: Adjusts request rate based on consecutive errors (backs off on 429/500+, recovers on success).
- `RateLimiterTokenBucket`: Token bucket implementation for precise rate control.

### Grammar-based Fuzzing (`grammar.rs`)

`GrammarFuzzer` generates structured payloads from formal grammars supporting JSON, GraphQL, XML, JWT, and SSTI formats. Supports deterministic seeding via `with_seed()`.

### Response Filtering (`filters.rs`)

`FilterChain` applies sequential filters to exclude responses by status code, response size, word count, line count, response time, or regex patterns on the response body. Similar to ffuf's filtering.

### Chained Fuzzing (`chain.rs`)

`ChainExecutor` supports multi-step fuzz chains with variable extraction, conditional logic, and LRU regex caching (cache size 100). `AutoExploiter` automates exploitation chains.

### Calibration (`calibration.rs`)

Auto-calibration system that samples baseline responses before fuzzing to automatically configure filters. Analyzes status codes, response sizes, word counts, line counts, and timing to establish "normal" behavior.

### Targets (`targets/`)

Target-specific payload generation:
- `api.rs` - API endpoint discovery and OpenAPI spec parsing
- `apache.rs` - Apache-specific paths and misconfigurations
- `nginx.rs` - Nginx-specific paths and misconfigurations
- `php.rs` - PHP-specific payloads
- `generic.rs` - Generic target payloads

### WAF Fingerprinting & Bypass (`waf_fingerprint.rs`)

`WafFingerprinter` detects Web Application Firewalls via headers, cookies, status codes, and body patterns. Supports 34 WAF products (Cloudflare, Akamai, AWS WAF, Imperva, F5 ASM, Azure WAF, ModSecurity, etc.) with bypass suggestions.

## Specialized Fuzzing

- **API Schema Fuzzing (`api_schema/`)**: Automatically generates tests based on OpenAPI 3.0 (Swagger) definitions. Type-aware payloads, auth bypass, required parameter omission, oversized payload generation.
- **Advanced Threat Hunting (`advanced.rs`)**: Specialized fuzzers — `GraphQLFuzzer`, `JwtFuzzer`, `OAuthFuzzer`, `IdorFuzzer`, `SstiFuzzer`, `WebSocketFuzzer`, `GrpcFuzzer`.
- **ReDoS Detection (`redos_detect.rs`)**: `RegexExecutor` with timeout-based detection (default 1000ms, max 100k iterations). `ReDosDetector` checks against known vulnerable patterns. `PayloadReDosChecker` extracts and tests regex patterns from payloads.

## Feedback Loop

The fuzzer is designed to be "smart," using feedback from the target (e.g., changes in response time or body content) to prioritize certain payloads or mutators. The `TimingAnalyzer` maintains running baselines, `PatternMatcher` detects leaks in real-time, and `FilterChain` excludes responses similar to baseline.

## Code Conventions

### Hash Collections
Use `rustc_hash::FxHashMap` and `rustc_hash::FxHashSet` instead of `std::collections::HashMap/HashSet` for performance.

### Magic Numbers
Extract magic numbers to named constants at module level:
```rust
const DEFAULT_SPIKE_THRESHOLD: f64 = 3.0;
const DEFAULT_REDOS_THRESHOLD_MS: u64 = 5000;
const BODY_LENGTH_ANOMALY_THRESHOLD: isize = 1000;
const TIMING_ANOMALY_THRESHOLD_MS: i64 = 1000;
const OVERSIZED_PAYLOAD_SIZES: [usize; 4] = [1_000, 10_000, 100_000, 1_000_000];
```

### Error Handling
Prefer explicit error handling over `unwrap_or_default()`:
```rust
// Instead of:
let body = response.text().await.unwrap_or_default();

// Use:
let body = match response.text().await {
    Ok(text) => text,
    Err(e) => {
        tracing::debug!("Failed to read response body: {}", e);
        String::new()
    }
};
```

### WAF Detection
WAF blocked status codes are defined in `crate::constants::waf::BLOCKED_STATUS_CODES` and referenced via `engine/utils.rs`:
```rust
const WAF_BLOCKED_STATUS_CODES: &[u16] = &crate::constants::waf::BLOCKED_STATUS_CODES;
```

### Timing Analysis
The `TimingAnalyzer` in `detection/analyzer.rs` uses IQR (Interquartile Range) for baseline calculation. It handles NaN values in `partial_cmp` explicitly to prevent panics:
```rust
s.sort_by(|a, b| a.partial_cmp(b).unwrap_or_else(|| {
    if a.is_nan() && b.is_nan() {
        std::cmp::Ordering::Equal
    } else if a.is_nan() {
        std::cmp::Ordering::Greater
    } else {
        std::cmp::Ordering::Less
    }
}));
```

### API Schema Fuzzing
`api_schema/mod.rs` provides OpenAPI 3.0 (JSON/YAML) parsing with type-aware fuzzing:
- `ApiSchemaFuzzer` - Generates fuzz targets from OpenAPI specs
- Type-aware payloads based on parameter types (string, integer, boolean, array, object)
- Auth bypass via headers (X-Original-URL, X-Override-URL, X-Rewrite-URL)
- Required parameter omission testing
- Oversized payload generation using `OVERSIZED_PAYLOAD_SIZES` constant

### Advanced Fuzzers
`advanced.rs` provides specialized fuzzers for:
- `GraphQLFuzzer` - Introspection, depth bypass, alias overload, batch queries
- `JwtFuzzer` - None algorithm attack, key injection, token validation
- `OAuthFuzzer` - Redirect URI, scope escalation, state parameter, grant mixing
- `IdorFuzzer` - Horizontal/vertical escalation testing
- `SstiFuzzer` - Template engine detection (Jinja2, ERB, etc.)
- `WebSocketFuzzer` - Message injection (use `PayloadType::Websocket`, not `PayloadType::Grpc`)
- `GrpcFuzzer` - Method injection

### ReDoS Detection
`redos_detect.rs` provides:
- `RegexExecutor` - Timeout-based detection (default 1000ms, max 100k iterations)
- Known vulnerable patterns: `(.+)+`, `(.*)*`, `(a+)+`, etc.
- Uses `FxHashMap` for vulnerable payload tracking
