# Fuzzer Module

Deep dive: payload-driven security fuzzing engine, detection pipeline, chained/stateful fuzzing, and WAF interplay.

Parent overview: [overview.md](overview.md). Related: [waf.md](waf.md), [api_schema.md](api_schema.md).

## Role & Responsibilities

The fuzzer is Eggsec's primary vulnerability discovery engine. It sends crafted payloads to targets, analyzes responses for anomalies (timing spikes, data leaks, status changes), and reports findings. The module is **always compiled** — no feature gate.

Key capabilities:
- 40 payload categories covering injection, access control, server-side, client-side, API, and infrastructure classes
- Three execution modes: Sequential, Burst (concurrent), Adaptive (rate-limited)
- Aho-Corasick multi-pattern leak detection (database errors, stack traces, file paths, credentials)
- IQR-based timing anomaly detection
- Response diffing against a baseline
- Grammar-based (JSON/GraphQL/XML/JWT/SSTI) and mutation-based payload generation
- Multi-step chained fuzzing with variable extraction and conditional logic
- Auto-calibration for baseline filter derivation
- 17-product WAF fingerprinting with bypass suggestions
- ReDoS detection with timeout-based execution

## Location & Feature Gating

- Source: `crates/eggsec/src/fuzzer/` — 73 `.rs` files across 14 directories
- Feature gate: **none** (always compiled)
- Bidirectional type sharing with `waf`: fuzzer uses `waf::types::{OwaspCategory, Severity}` (`fuzzer/engine/types.rs:5`); WAF uses `fuzzer::config::WafConfig` (`waf/mod.rs:86`)

### Directory Structure

| Directory | Files | Purpose |
|-----------|-------|---------|
| `payloads/` | 42 | Per-type payload libraries + `PayloadType` enum + `payload_vec!` macro |
| `engine/` | 7 | Core `FuzzEngine`, execution modes, session building, advanced dispatch |
| `detection/` | 4 | Aho-Corasick leak matcher, IQR timing analyzer, raw patterns |
| `targets/` | 6 | Per-target profiles: api, apache, php, nginx, generic |
| `api_schema/` | 1 | **Internal** schema-aware fuzzer (distinct from top-level `api_schema/`) |
| (root) | 12 | config, chain, filters, grammar, mutator, rate_limit, redos_detect, diff, state, waf_fingerprint, advanced, mod |

---

## Architecture

### PayloadType Enum (40 variants)

**File:** `fuzzer/payloads/mod.rs:49-90`

Exact variant list in declaration order:

| # | Variant | Display Name | Class |
|---|---------|-------------|-------|
| 1 | `Sqli` | SQL Injection | Injection |
| 2 | `Xss` | XSS | Injection |
| 3 | `Traversal` | Path Traversal | File System |
| 4 | `Ssrf` | SSRF | Server-Side |
| 5 | `Redirect` | Open Redirect | Client-Side |
| 6 | `Redos` | ReDoS | Server-Side |
| 7 | `Headers` | Header Expansion | Infrastructure |
| 8 | `Compression` | Compression Bomb | Infrastructure |
| 9 | `GraphQL` | GraphQL | API Security |
| 10 | `OAuth` | OAuth/OIDC | API Security |
| 11 | `Jwt` | JWT | API Security |
| 12 | `Idor` | IDOR | Access Control |
| 13 | `Ssti` | SSTI | Injection |
| 14 | `Grpc` | gRPC | API Security |
| 15 | `Xxe` | XXE | Injection |
| 16 | `Ldap` | LDAP Injection | Injection |
| 17 | `Cmd` | Command Injection | Injection |
| 18 | `Deser` | Deserialization | Server-Side |
| 19 | `Host` | Host Header Injection | Infrastructure |
| 20 | `Cache` | Cache Poisoning | Infrastructure |
| 21 | `Csv` | CSV Injection | Client-Side |
| 22 | `Soap` | SOAP/XML | API Security |
| 23 | `Websocket` | WebSocket | API Security |
| 24 | `Nosql` | NoSQL Injection | Injection |
| 25 | `Xpath` | XPath Injection | Injection |
| 26 | `Expression` | Expression Injection | Injection |
| 27 | `Prototype` | Prototype Pollution | Client-Side |
| 28 | `Race` | Race Condition | Server-Side |
| 29 | `MassAssign` | Mass Assignment | Access Control |
| 30 | `Oast` | OAST | Infrastructure |
| 31 | `Saml` | SAML | Access Control |
| 32 | `HtmlInject` | HTML Injection | Injection |
| 33 | `CssInject` | CSS Injection | Client-Side |
| 34 | `Ssi` | SSI Injection | Injection |
| 35 | `DomClobber` | DOM Clobbering | Client-Side |
| 36 | `Xslt` | XSLT Injection | Injection |
| 37 | `Viewstate` | ViewState Deserialization | Server-Side |
| 38 | `DepConfusion` | Dependency Confusion | Infrastructure |
| 39 | `XsLeak` | XS-Leak | Client-Side |
| 40 | `Latex` | LaTeX Injection | Injection |

**Count verified:** 40 variants at `payloads/mod.rs:49-90`. Each variant maps to a `get_payloads()` dispatch arm at `payloads/mod.rs:182-224` (40 arms).

**Advanced check** (`payloads/mod.rs:140-150`): `is_advanced()` returns `true` for exactly **6** variants: `GraphQL`, `OAuth`, `Jwt`, `Idor`, `Ssti`, `Grpc`. Note: `Websocket` has a dedicated fuzzer implementation (`advanced.rs:416-612`) but is **excluded** from `is_advanced()`.

### Payload Structure

```rust
// payload.rs:159
pub struct Payload {
    pub payload_type: PayloadType,
    pub payload: String,
    pub description: String,
    pub severity: Severity,     // re-exported from waf::types::Severity
    pub tags: Vec<String>,
}
```

Payloads are cached via `LazyLock` maps (`payloads/mod.rs:170-180`): `PAYLOAD_CACHE` (per-type) and `ALL_PAYLOADS_CACHE` (flattened). The `get_payloads_cached()` function returns `&'static Vec<Payload>`.

### Engine Components

| Component | File:Line | Purpose |
|-----------|-----------|---------|
| `FuzzEngine` | `engine/core.rs:105` | Main struct: HTTP client, timing analyzer, pattern matcher, grammar fuzzer, session, differ, filter chain, auth context |
| `FuzzEngine::new()` | `engine/core.rs:128` | Constructor (tui_mode=false) |
| `FuzzEngine::new_with_tui_mode()` | `engine/core.rs:140` | Constructor with explicit TUI mode; clamps concurrency to 1..=500 |
| `FuzzEngine::run()` | `engine/core.rs:282` | CLI entry point: runs session, prints output |
| `FuzzEngine::run_return_session()` | `engine/core.rs:383` | Core loop: parse payload types → dispatch per-type → apply filters → build session |
| `FuzzEngine::run_all_types()` | `engine/core.rs:463` | WAF stress: all payloads across all types |
| `FuzzEngine::run_advanced_fuzzer()` | `engine/advanced.rs:14` | Dispatches to GraphQL/JWT/OAuth/IDOR/SSTI/WebSocket/gRPC fuzzers |
| `FuzzEngine::parse_payload_types()` | `engine/advanced.rs:115` | Parses comma-separated payload type strings with aliases |
| `FuzzMode` | `config.rs:9` | `Sequential` (default), `Burst`, `Adaptive` |
| `send_payload_async()` | `engine/utils.rs:218` | HTTP request + timing + leak detection + WAF block check → `FuzzResult` |
| `compute_severity()` | `engine/utils.rs:342` | Severity escalation: ReDoS→Critical, WAF+leak→Critical, leak→High, WAF→Medium |
| `FuzzResult` | `engine/types.rs:10` | Per-payload result: status, timing, anomalies, leaks, WAF blocked, OWASP category |
| `FuzzSession` | `engine/types.rs:152` | Aggregate: counts, OWASP summary, baseline, results |
| `OwaspSummary` | `engine/types.rs:36` | OWASP Top 10 (2021+2023) mapping from `FuzzResult` set |

### Detection Pipeline

| Component | File:Line | Purpose |
|-----------|-----------|---------|
| `PatternMatcher` | `detection/aho_corasick.rs:60` | Wraps static Aho-Corasick; `scan()` returns `Vec<LeakMatch>` sorted by severity |
| `LeakMatch` | `detection/aho_corasick.rs:6` | Matched pattern + category + severity + context |
| `LeakCategory` | `detection/aho_corasick.rs:14` | 7 variants: `DatabaseError`, `StackTrace`, `FilePath`, `SensitiveData`, `DebugInfo`, `Configuration`, `Credentials` |
| `LeakSeverity` | `detection/aho_corasick.rs:24` | 4 levels: `Critical`, `High`, `Medium`, `Low` |
| Static patterns | `detection/aho_corasick.rs:46-57` | LazyLock-compiled Aho-Corasick from `patterns.rs` |
| `TimingAnalyzer` | `detection/analyzer.rs:13` | IQR baseline with atomic stats (lock-free counters, `&mut self` recording) |
| `TimingResult` | `detection/analyzer.rs:5` | `response_time_ms`, `is_anomaly`, `is_redos_suspected`, `anomaly_factor` |
| Constants | `detection/analyzer.rs:27-29` | `DEFAULT_SPIKE_THRESHOLD=3.0`, `DEFAULT_REDOS_THRESHOLD_MS=5000`, `DEFAULT_MIN_SAMPLES_FOR_BASELINE=20` |

Pattern categories in `detection/patterns.rs`:
- **Database errors** (17 patterns): SQL syntax, mysql_fetch, ORA-, PLS-, PostgreSQL, ODBC, SQLSTATE, etc.
- **Stack traces** (19 patterns): Java/Python/PHP/.NET/General
- **File paths** (18 patterns): /etc/passwd, /var/log, .env, wp-config, etc.
- **Sensitive data** (30 patterns): passwords, API keys, tokens, private keys, AWS keys, connection strings

### Response Diffing

**File:** `fuzzer/diff.rs`

| Type | Purpose |
|------|---------|
| `ResponseDiffer` | Baseline capture + comparison; default ignores `date`, `content-length`, `connection`, `keep-alive` |
| `ResponseSnapshot` | Baseline: status, headers (HeaderSnapshot), body SHA-256 hash, body length, content type, timing |
| `DiffResult` | Weighted anomaly score: status change (+0.3), content-type change (+0.2), body length >1000 (+0.2), new/removed headers (+0.1 each), header value changes (+0.05), new cookies (+0.15), timing >1000ms (+0.2) |
| `min_anomaly_threshold` | 0.3 (default) |

### Chained Fuzzing

**File:** `fuzzer/chain.rs`

| Type | Purpose |
|------|---------|
| `ChainExecutor` | Multi-step execution with LRU regex cache (size 100, `chain.rs:9`), variable interpolation `${var}`, dual HTTP clients (follow/no-redirect) |
| `ChainAction` | `Request`, `ExtractVar`, `Conditional`, `Sleep` |
| `ExtractRule` | From: `ResponseBody`, `ResponseHeader(name)`, `ResponseStatus`, `Cookie(name)` |
| `ConditionCheck` | `StatusCode`, `StatusCodeRange`, `Contains`, `RegexMatch`, `VariableExists`, `VariableEquals` |
| `AutoExploiter` | Automated SSRF/SQLi exploitation chain generation |
| Sleep clamp | Max 60,000ms (`chain.rs:172`) |
| Variable interpolation | `LazyLock<Regex>` at `chain.rs:425` (cached, `\$\{(\w+)\}`) |

### Grammar-Based Fuzzing

**File:** `fuzzer/grammar.rs`

| Type | Purpose |
|------|---------|
| `GrammarFuzzer` | Generates payloads from `Grammar` rules; `with_seed()` for deterministic output |
| `GrammarKind` | `Json`, `GraphQL`, `Xml`, `Jwt`, `Ssti` — each maps to a `PayloadType` and `Severity` |
| `Grammar` | `start` rule + `Vec<GrammarRule>` with named alternatives |

Built-in grammars: JSON (null/true/false/strings/numbers), GraphQL (introspection/mutation queries), XML (XXE entities), JWT (none/algorithm variants), SSTI (Jinja2/ERB templates).

### Response Filtering

**File:** `fuzzer/filters.rs`

`FilterChain` applies sequential filters — any match excludes the result. **13** `PayloadFilter` variants:

| # | Variant | Criterion |
|---|---------|-----------|
| 1 | `StatusCode(Vec<u16>)` | Status code in set |
| 2 | `ResponseSize(Vec<u64>)` | Exact size in set |
| 3 | `ResponseSizeRange { min, max }` | Size within range |
| 4 | `WordCount(Vec<u64>)` | Word count in set |
| 5 | `WordCountRange { min, max }` | Word count within range |
| 6 | `LineCount(Vec<u64>)` | Line count in set |
| 7 | `LineCountRange { min, max }` | Line count within range |
| 8 | `ResponseTimeMax(u64)` | Response time ≤ threshold |
| 9 | `ResponseTimeRange { min, max }` | Response time within range |
| 10 | `Regex(Regex)` | Regex match on body |
| 11 | `SizeGreaterThan(u64)` | Size > threshold |
| 12 | `SizeLessThan(u64)` | Size < threshold |

Note: Word/line count filters use response body when available; otherwise estimate from `response_length` (words ≈ size/5, lines ≈ size/30).

### Calibration

**File:** `fuzzer/calibration.rs`

| Type | Purpose |
|------|---------|
| `Calibrator` | Sends 10 dummy payloads (`FUZZ0`..`FUZZ9`), computes `BaselineStats`, derives `FilterChain` |
| `CalibrationResult` | `filter_chain`, `baseline_stats`, `samples_taken` |
| `BaselineStats` | `status_codes`, avg/min/max size, avg words, avg lines, avg/min/max time |

### WAF Fingerprinting

**File:** `fuzzer/waf_fingerprint.rs`

`WafFingerprinter` detects **17 WAF products** via headers, cookies, status codes, and body patterns with confidence scoring (0.0–1.0):

Cloudflare (0.9), Akamai (0.85), AWS WAF (0.7), Imperva/Incapsula (0.9), F5 ASM (0.8), Azure WAF (0.75), FortiWeb (0.8), ModSecurity (0.7), Sucuri (0.85), Barracuda (0.8), DenyAll (0.75), Radware (0.7), Safe3 (0.65), dotDefender (0.7), StackPath (0.8), Fastly (0.75), CloudFront (0.65).

Detection weights: header pattern match (+0.3), header presence (+0.2), cookie match (+0.25), status code match (+0.2), body pattern match (+0.15). Confidence capped at fingerprint's `confidence` field.

### ReDoS Detection

**File:** `fuzzer/redos_detect.rs`

| Type | Purpose |
|------|---------|
| `RegexExecutor` | Timeout-based detection (default 1000ms, max 100k iterations); tests against 10 default strings |
| `ReDosDetector` | Checks against 15 known vulnerable patterns (`KNOWN_VULNERABLE_PATTERNS` at `redos_detect.rs:10`) before running executor |
| `PayloadReDosChecker` | Extracts regex patterns from payload descriptions and tests them |

Known vulnerable patterns: `(.+)+`, `(.*)*`, `(a+)+`, `(a*)a*`, `([a-zA-Z]+)*`, `(x+x+)+y`, `(x?x?)+`, `(a{1,100})+`, `(a|a|a)+`, `(.*a){10}`, `(\d+\.?\\d*)+`, `(\w+\s?)*`, `(a+)+b`, `(?:a|a)+`, `^(.*?)*$`.

### Mutator

**File:** `fuzzer/mutator.rs`

`generate_mutations(payload, count)` — public API. 12 `MutationType` variants:

`CaseToggle`, `UrlEncode`, `DoubleUrlEncode`, `NullByte`, `Duplicate`, `Truncate`, `Prefix`, `Suffix`, `Comment`, `Whitespace`, `Reverse`, `Swap`.

### Rate Limiting

**File:** `fuzzer/rate_limit.rs`

| Type | Purpose |
|------|---------|
| `AdaptiveRateLimiter` | Adjusts request rate: backs off on 429/500+ (threshold=3 errors, multiplier=0.5), recovers on success (multiplier=1.25 after 10 consecutive successes) |
| `RateLimiterTokenBucket` | Token bucket with CAS-based atomic acquire/refill |

### Session Management

**File:** `fuzzer/state.rs`

| Type | Purpose |
|------|---------|
| `HttpSession` | Cookies, tokens, headers (SerializableHeaderMap), state data |
| `SessionManager` | Async session storage with `Arc<RwLock<FxHashMap>>` |
| `AuthHandler` | Supports `None`, `Basic`, `Bearer`, `ApiKey`, `OAuth2`, `JWT` auth types |

### Internal Schema Fuzzer

**File:** `fuzzer/api_schema/mod.rs`

`ApiSchemaFuzzer` — always compiled, lives inside the fuzzer crate. **Distinct** from the top-level `api_schema` module.

| Type | Purpose |
|------|---------|
| `ApiSchemaFuzzer` | OpenAPI 3.0 parser + type-aware payload generator |
| `ApiEndpoint` | path, method, parameters, request_body, **security** field |
| `ApiParameter` | name, location, type, format, example, min/max, pattern, enum_values |
| `ParamLocation` | `Path`, `Query`, `Header`, `Cookie`, `Body` |
| `SchemaFuzzTarget` | endpoint + base_url |
| `SchemaFuzzResult` | endpoint, method, test_type, status_code, vulnerable, details |

Type-aware payloads (`generate_type_aware_payloads`):
- **string**: SQLi, XSS, SSTI, pattern bypass
- **integer/number**: SQLi (overflow, negative, float)
- **boolean**: true/false/1 injection
- **array**: empty, numeric, XSS in array
- **object**: empty, prototype pollution, constructor pollution

Auth bypass via `X-Original-URL`, `X-Override-URL`, `X-Rewrite-URL` headers. Required parameter omission. Oversized payloads at sizes `[1_000, 10_000, 100_000, 1_000_000]` (`OVERSIZED_PAYLOAD_SIZES` at `fuzzer/api_schema/mod.rs:7`).

### Target Profiles

**File:** `fuzzer/targets/mod.rs`

5 target types: `Api`, `Nginx`, `Apache`, `PHP`, `Generic`. Each provides server-specific payloads (paths, misconfigurations, default files).

---

## Behavior / Flow

### Fuzz Session Lifecycle

```
FuzzConfig → FuzzEngine::new()
  ├─ Build reqwest::Client (TLS, proxy, redirect policy)
  ├─ Optionally init GrammarFuzzer, HttpSession, ResponseDiffer
  ├─ Load AuthContextEntry (YAML + env-var interpolation)
  └─ Init FilterChain

FuzzEngine::run_return_session()
  ├─ [optional] capture_baseline_for_diffing() → ResponseDiffer baseline
  ├─ parse_payload_types() → Vec<PayloadType>
  │   └─ "all" → PayloadType::all_variants() (40 types)
  │   └─ comma-separated with aliases (30 mapped aliases)
  ├─ For each PayloadType:
  │   ├─ is_advanced()? → run_advanced_fuzzer()
  │   │   └─ Dispatch to GraphQLFuzzer/JwtFuzzer/OAuthFuzzer/IdorFuzzer/SstiFuzzer/WebSocketFuzzer/GrpcFuzzer
  │   │   └─ Each returns Vec<FuzzResult> via AdvancedFuzzer::fuzz()
  │   └─ else → prepare_payloads() → run_payload_batch()
  │       ├─ get_payloads(pt) → base payloads
  │       ├─ [optional] mutate_payloads() → generate_mutations()
  │       ├─ [optional] GrammarFuzzer::generate_batch()
  │       ├─ [optional] AiPayloadGenerator::generate_payloads()
  │       └─ run_sequential / run_burst / run_adaptive
  │           └─ send_payload_async() per payload:
  │               ├─ Build URL (param injection or path append)
  │               ├─ HTTP request with auth context
  │               ├─ TimingAnalyzer::record()
  │               ├─ PatternMatcher::scan() → Vec<LeakMatch>
  │               ├─ WAF blocked check (BLOCKED_STATUS_CODES)
  │               ├─ compute_severity()
  │               └─ → FuzzResult
  ├─ [optional] Target-specific payloads (api/apache/php/nginx/generic)
  ├─ Apply FilterChain (exclude matching results)
  └─ build_session() → FuzzSession
      ├─ Count successes/failures/bypasses/leaks/anomalies/ReDoS
      ├─ OwaspSummary::from_results()
      └─ Return FuzzSession
```

### Chaining Flow

```
ChainExecutor::execute(actions)
  ├─ Reverse actions → pop() for declared order
  ├─ For each action:
  │   ├─ Request: interpolate URL/headers/body, send, store _last_status/_last_body/_header_*/_cookie_*
  │   ├─ ExtractVar: regex extract from stored variables → named variable
  │   ├─ Conditional: check condition → enqueue then/else actions
  │   └─ Sleep: clamp to 60s max
  └─ → ChainExecutionResult
```

### Severity Escalation (`engine/utils.rs:342`)

```
compute_severity(base, waf_blocked, redos, has_leak):
  ReDoS → Critical
  WAF blocked + leak → Critical
  Leak only → High
  WAF blocked only → Medium
  None → base severity
```

---

## Public API

**Module re-exports** (`fuzzer/mod.rs:104-133`):

| Symbol | Source |
|--------|--------|
| `FuzzEngine`, `FuzzResult`, `FuzzSession`, `OwaspSummary`, `StatefulFuzzer`, `ChainedFuzzInput/Output`, `FuzzChainStep`, `StepResults` | `engine` |
| `Payload`, `PayloadType`, `Severity`, `get_payloads`, `get_payloads_cached`, `get_all_payloads_cached` | `payloads` |
| `FilterChain`, `PayloadFilter` | `filters` |
| `GrammarFuzzer`, `Grammar` | `grammar` |
| `generate_mutations` | `mutator` |
| `ResponseDiffer`, `ResponseDiff`, `DiffResult` | `diff` |
| `Calibrator`, `CalibrationResult`, `BaselineStats` | `calibration` |
| `ChainExecutor`, `ChainAction`, `AutoExploiter`, `ChainExecutionResult`, `ChainedFuzzResult` | `chain` |
| `AdaptiveRateLimiter`, `RateLimiterTokenBucket` | `rate_limit` |
| `RegexExecutor`, `ReDosDetector`, `ReDosResult`, `PayloadReDosChecker` | `redos_detect` |
| `HttpSession`, `SessionManager`, `AuthHandler`, `AuthCredentials`, `AuthType` | `state` |
| `TargetPayload`, `TargetType`, `get_target_payloads` | `targets` |
| `WafFingerprinter`, `WafFingerprint`, `WafDetectionResult` | `waf_fingerprint` |
| `AdvancedFuzzer`, `GraphQLFuzzer`, `JwtFuzzer`, `OAuthFuzzer`, `IdorFuzzer`, `SstiFuzzer`, `WebSocketFuzzer`, `GrpcFuzzer` | `advanced` |

**Entry points:**
- `run_cli(args)` — CLI fuzzing (`fuzzer/mod.rs:144`)
- `run_cli_with_callback(args, callback)` — pipeline streaming (`fuzzer/mod.rs:160`, feature `tool-api`)
- `run_waf_stress(args)` — WAF stress testing (`fuzzer/mod.rs:188`)

---

## Integration Points

### Dispatch

`FuzzExecutor` in `dispatch/executors/` calls `FuzzEngine::run_return_session()` or `run_cli()`. The TUI path uses `FuzzEngine::new_with_tui_mode(args, true)`.

### CLI Handlers

- `handle_fuzz()` in `commands/handlers/` → `fuzzer::run_cli()`
- `handle_graphql()` → specialized GraphQL fuzzing
- `handle_oauth()` → specialized OAuth testing
- `handle_waf_stress()` → `fuzzer::run_waf_stress()`

### WAF Interplay

- Fuzzer reads `crate::constants::waf::BLOCKED_STATUS_CODES` (`engine/utils.rs:18`) to detect WAF blocks
- Fuzzer reads `crate::constants::waf::LENGTH_DIFF_THRESHOLD` (`engine/utils.rs:165`) for body length diffing
- Fuzzer imports `waf::types::{OwaspCategory, Severity}` (`engine/types.rs:5`)
- WAF module imports `fuzzer::config::WafConfig` (`waf/mod.rs:86`)
- `WafFingerprinter` in fuzzer detects WAF presence; `WafDetector` in waf module provides more comprehensive detection

### Pipeline

Pipeline stages invoke fuzzer via `FuzzExecutor`. The `ScanProfile` enum determines which payload types to run.

### Tool Registry

`ToolRegistry` registers fuzzer operations. `EnforcedDispatcher` validates scope before dispatch.

---

## Configuration

**File:** `fuzzer/config.rs`

### FuzzConfig (37 fields)

Core: `url`, `payload_type`, `mode`, `method`, `param`, `concurrency`, `timeout`

Payload control: `mutate`, `mutation_count`, `grammar_fuzz`, `grammar_type`

Feature toggles: `session`, `diffing`, `capture_baseline`, `enhanced_redos`, `waf_fingerprint`, `chaining`, `chain_file`, `adaptive_rate`

Advanced fuzzers: `graphql_introspection`, `graphql_depth_bypass`, `graphql_alias_overload`, `jwt_token`, `oauth_client_id`, `oauth_client_secret`, `oauth_redirect`, `oauth_scope`, `oauth_state`, `oauth_grant`, `oauth_issuer`, `idor_base_id`, `idor_user_ids`, `ssti_param`

Schema: `schema`, `discover_only`, `auto_discover_schema`, `calibrate`

Filters: `fc` (status codes), `fs` (sizes), `fw` (words), `fl` (lines), `ft` (time), `fr` (regex)

Output: `json`, `output`, `verbose`, `quiet`, `format`

Common: `common: CommonHttpArgs` (proxy, bearer, user_agent, insecure, auth_context, auth_role)

### FuzzMode

`Sequential` (default) → one-at-a-time; `Burst` → concurrent with semaphore; `Adaptive` → rate-limited with backoff/recovery.

### WafConfig (15 fields)

`url`, `detect_only`, `bypass`, `header_bypass`, `smuggling`, `evasion`, `profile`, `test_type`, `concurrency`, `timeout`, `json`, `verbose`, `quiet`, `output`, `common`

### WafStressConfig

`url`, `concurrency`, `timeout`, `json`, `verbose`, `quiet`, `output`, `common`

Conversions: `FuzzArgs` → `FuzzConfig` (feature `cli`), `WafStressArgs` → `WafStressConfig` → `FuzzConfig`, `WafArgs` → `WafConfig`.

---

## Testing

- Unit tests in every module file (`#[cfg(test)]` blocks)
- Concurrency clamping tests: 0→1, 1000→500
- Payload type parsing: 30+ test cases for aliases, whitespace, case insensitivity, invalid types
- Filter chain: status, size, word/line count, time, regex
- Chain executor: interpolation, action ordering
- Calibration: baseline stats computation
- WAF fingerprinter: Cloudflare detection
- ReDoS: known vulnerable pattern detection
- Grammar fuzzer: JSON/SSTI generation
- Mutator: count, uniqueness, property-based (proptest)
- Timing analyzer: IQR baseline, NaN handling

---

## Invariants & Gotchas

1. **Concurrency always ≥ 1**: `FuzzEngine::new_with_tui_mode()` clamps to `1..=500` (`engine/core.rs:143`)
2. **Every spawned task has a timeout**: 300s wrapper on all concurrent tasks (`engine/execution.rs:117`)
3. **Semaphore per concurrent session**: bounds parallel requests (`engine/execution.rs:96`)
4. **Regex caching**: `ChainExecutor` uses LRU (size 100) for extraction patterns (`chain.rs:9`); `PatternMatcher` uses `LazyLock` static Aho-Corasick (`detection/aho_corasick.rs:48`)
5. **NaN handling**: `TimingAnalyzer` sorts with explicit NaN ordering to prevent panics (`detection/analyzer.rs:168-178`)
6. **Sleep clamping**: Chain sleep actions max 60s (`chain.rs:172`)
7. **WebSocket in advanced check**: `PayloadType::Websocket` has a dedicated `AdvancedFuzzer` impl but is NOT in `is_advanced()` — dispatched as a regular payload type, not through `run_advanced_fuzzer()`
8. **`is_advanced()` mapping is incomplete**: `parse_payload_types()` in `engine/advanced.rs:115-166` only maps 30 of 40 payload types by string alias; `saml`, `html_inject`, `css_inject`, `ssi`, `dom_clobber`, `xslt`, `viewstate`, `dep_confusion`, `xs_leak`, and `latex` have no comma-separated string aliases (only reachable via `"all"`)
9. **`TimingAnalyzer` requires `&mut self`**: wrapped in `Arc<Mutex<>>` at call site (`engine/core.rs:108`)
10. **Payload cache is static**: `PAYLOAD_CACHE` and `ALL_PAYLOADS_CACHE` are `LazyLock` — initialized once, never invalidated
11. **Two OpenAPI models**: `fuzzer::api_schema` (always compiled, security-focused) and top-level `api_schema` (feature-gated `api-schema`, report/tooling-focused) are deliberately non-interoperable
12. **WAF block detection uses status codes only**: `BLOCKED_STATUS_CODES` check (`engine/utils.rs:260`) does not inspect response body

---

## Bug Sweep

| # | File:Line | Issue | Severity |
|---|-----------|-------|----------|
| 1 | `engine/advanced.rs:155-156` | 10 PayloadType variants (`Saml`, `HtmlInject`, `CssInject`, `Ssi`, `DomClobber`, `Xslt`, `Viewstate`, `DepConfusion`, `XsLeak`, `Latex`) have no string alias in `parse_payload_types()` — unreachable via CLI comma-separated list | Low (can use `"all"`) |
| 2 | `engine/execution.rs:88` | `ProgressStyle::template()` uses `format!` with user-controlled mode_name; invalid template characters cause `unwrap_or_else` fallback (safe but noisy) | Low |
| 3 | `calibration.rs:89` | Uses `eprintln!` directly instead of `tracing` — inconsistent with codebase logging convention | Low |
| 4 | `calibration.rs:171` | Uses `eprintln!` for calibration sample failure — same logging inconsistency | Low |
| 5 | `advanced.rs:480-489` | `AutoExploiter::try_sqli_exploitation()` always returns `None` — dead code | Low |

*Last verified against source: 2026-08-25*
