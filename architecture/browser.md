# Browser Module

## Role & Responsibilities

Headless Chrome integration for browser-based security testing. Provides:

- **DOM XSS detection** — source/sink tracing across 8 sources × 10 sinks (80 combinations), severity-scored with CVSS
- **SPA route discovery** — DOM/JS link extraction, XHR/Fetch interception, regex route-pattern parsing
- **Client-side security checks** — localStorage sensitive data, CORS misconfiguration, CSP weaknesses, debug mode, exposed source maps
- **Request corpus capture** — forms, scripts, GraphQL candidates from page DOM

**Non-responsibilities**: Does not perform active exploitation, credential injection, or authenticated crawling. Does not persist results to database (no `database` feature dependency). Does not integrate with MCP/agent/pipeline surfaces (standalone CLI module).

## Location & Feature Gating

| Item | Location | Gate |
|------|----------|------|
| Module declaration | `crates/eggsec/src/lib.rs:86-87` | `#[cfg(feature = "headless-browser")]` |
| Real `run_browser_scan()` | `crates/eggsec/src/browser/mod.rs:41` | `#[cfg(feature = "headless-browser")]` |
| Error-stub `run_browser_scan()` | `crates/eggsec/src/browser/mod.rs:211` | `#[cfg(not(feature = "headless-browser"))]` |
| `capture_requests()` | `crates/eggsec/src/browser/mod.rs:112` | `#[cfg(feature = "headless-browser")]` |
| Submodules (xss_dom, spa_discovery, client_checks, corpus) | `crates/eggsec/src/browser/mod.rs:12-15` | Gated by parent module |
| CLI handler | `crates/eggsec/src/commands/handlers/browser.rs:5` | `#[cfg(feature = "cli")]` |
| Feature flag | `crates/eggsec/Cargo.toml:336` | `headless-browser = ["headless_chrome"]` |
| `headless_chrome` dep | `crates/eggsec/Cargo.toml:192-194` | `version = "1"`, optional |

When `headless-browser` is disabled, `run_browser_scan()` returns `EggsecError::Config("headless-browser feature not enabled")`. The module itself does not exist (no stub modules).

## Architecture

### Files (5 total)

| File | Lines | Description |
|------|-------|-------------|
| `browser/mod.rs` | 237 | `BrowserConfig`, `BrowserReport`, `run_browser_scan()` entry point, XHR/Fetch interceptor injection, `capture_requests()` |
| `browser/xss_dom.rs` | 323 | `DomXssFinding`, `XssSource` (8 variants), `XssSink` (10 variants), `scan_dom_xss()`, `calculate_severity()`, `get_remediation()` |
| `browser/spa_discovery.rs` | 259 | `SpaRoute`, `DiscoveryMethod` (4 variants), `discover_routes()`, `extract_parameters()` |
| `browser/client_checks.rs` | 346 | `ClientIssue`, `ClientIssueType` (6 variants), `check_client_security()`, `get_remediation()` |
| `browser/corpus.rs` | 187 | `CorpusEntry`, `CorpusHeader`, `BodyShape`, `BodyField`, `RequestSource` (7 variants), `RequestCorpus`, `FormInfo` |

### Key Types

| Type | Location | Description |
|------|----------|-------------|
| `BrowserConfig` | `mod.rs:218` | `check_dom_xss`, `discover_spa_routes`, `check_client_security`, `timeout_ms`, `xss_payload` |
| `BrowserReport` | `mod.rs:22` | `target`, `dom_xss` (Vec), `spa_routes` (Vec), `client_issues` (Vec), `corpus`, `total_findings` |
| `DomXssFinding` | `xss_dom.rs:8` | `id`, `source`, `sink`, `location`, `severity`, `description`, `evidence`, `remediation`, `cvss_score` |
| `XssSource` | `xss_dom.rs:21` | 8 variants: `LocationHash`, `LocationSearch`, `DocumentCookie`, `DocumentReferrer`, `LocalStorage`, `SessionStorage`, `WebSocket`, `PostMessage` |
| `XssSink` | `xss_dom.rs:33` | 10 variants: `InnerHTML`, `OuterHTML`, `JQueryHtml`, `DocumentWrite`, `Eval`, `SetTimeout`, `SetInterval`, `FunctionConstructor`, `ScriptSrc`, `OnEventHandler` |
| `SpaRoute` | `spa_discovery.rs:6` | `path`, `method`, `parameters`, `discovered_via` |
| `DiscoveryMethod` | `spa_discovery.rs:13` | 4 variants: `Crawl`, `XhrInterception`, `FetchInterception`, `RouteParsing` |
| `ClientIssue` | `client_checks.rs:6` | `id`, `issue_type`, `severity`, `location`, `description`, `evidence`, `remediation`, `cvss_score` |
| `ClientIssueType` | `client_checks.rs:18` | 6 variants: `LocalStorageSensitive`, `CorsMisconfiguration`, `CSPSourceMap`, `DebugMode`, `SourceMapsExposed`, `CORSWildcard` |
| `RequestCorpus` | `corpus.rs:56` | `entries`, `urls`, `api_endpoints`, `forms`, `websocket_urls`, `javascript_urls`, `graphql_candidates`, `openapi_links`, `crawl_duration_ms`, `pages_visited` |
| `RequestSource` | `corpus.rs:44` | 7 variants: `Xhr`, `Fetch`, `Form`, `Navigation`, `WebSocket`, `Script`, `Other` |

### Variant Counts

| Enum | Variants | Source |
|------|----------|--------|
| `XssSource` | 8 | `xss_dom.rs:21-30` |
| `XssSink` | 10 | `xss_dom.rs:33-44` |
| `DiscoveryMethod` | 4 | `spa_discovery.rs:13-19` |
| `ClientIssueType` | 6 | `client_checks.rs:18-25` |
| `RequestSource` | 7 | `corpus.rs:44-52` |

## Behavior / Flow

### `run_browser_scan(target, config)` — `mod.rs:41-109`

1. Create `BrowserReport` with target
2. Launch headless Chrome via `headless_chrome::Browser::default()` (`:45`)
3. Create new tab, set timeout from `config.timeout_ms` (`:46-47`)
4. If `discover_spa_routes`: inject XHR/Fetch interceptor JS before navigation (`:50-82`)
5. Navigate to target, wait until loaded (`:84`)
6. If `check_dom_xss`: call `xss_dom::scan_dom_xss()` (`:86-90`)
7. If `discover_spa_routes`: call `spa_discovery::discover_routes()` (`:92-95`)
8. If `check_client_security`: call `client_checks::check_client_security()` (`:97-101`)
9. Call `capture_requests()` to collect forms/scripts/GraphQL candidates (`:103-104`)
10. Set `crawl_duration_ms` and `pages_visited` (`:105-106`)

### DOM XSS Scan — `xss_dom.rs:46-143`

Injects JS that iterates 8 sources × 10 sinks (80 pairs). For each source with a non-empty value, tests whether each sink accepts the XSS payload. Returns `DomXssFinding` for each confirmed pair. Severity calculated via `calculate_severity()` (`:145-173`): base score from sink (eval=9.0, innerHTML=7.5, document.write=8.0, etc.), multiplied by source modifier (document.cookie=1.2, localStorage=0.8), capped at 10.0.

### SPA Route Discovery — `spa_discovery.rs:32-155`

Evaluates JS that extracts routes from: `<a href>` links, `<form action>` attributes, JS route patterns (router.push, path:, url:, route:), plus the previously injected XHR/Fetch endpoint sets. Deduplicates via `HashSet`. Also extracts path parameters (`:id` and `{id}` style).

### Client Security Checks — `client_checks.rs:40-228`

Evaluates JS that checks: localStorage for sensitive patterns (token, auth, key, secret, password, credential, session, jwt, bearer), source map exposure, debug mode meta tags, CSP `unsafe-eval`, CORS wildcard/origin reflection (sends XHR with `Origin: https://evil-attacker.example.com`).

### Request Corpus — `corpus.rs:56-120`, `mod.rs:112-209`

Captures forms, script URLs, and GraphQL candidates from page DOM. `RequestCorpus` deduplicates entries by `method:url` key (`add_entry()` at `:96-101`). `api_endpoints()` filters entries containing `/api/`, `/graphql`, query strings, or non-GET methods.

## External Requirements

| Requirement | Source | Notes |
|-------------|--------|-------|
| Chrome/Chromium binary | `headless_chrome` crate (v1) | Must be installed on the system; the crate discovers via `which` or `CHROME_PATH` env |
| No root required | — | Unlike wireless/packet modules |
| Network access to target | — | Browser connects over HTTP/HTTPS |

The `headless_chrome` crate (v1) wraps the Chrome DevTools Protocol. It requires a Chrome/Chromium binary to be installed and accessible. On Linux this is typically `chromium-browser` or `google-chrome`. The crate auto-discovers the binary path.

## Public API

| Function | Signature | Description |
|----------|-----------|-------------|
| `run_browser_scan` | `pub async fn run_browser_scan(target: &str, config: BrowserConfig) -> Result<BrowserReport>` | Main entry point (real impl gated; stub returns error) |

`BrowserConfig` and `BrowserReport` are public types. Submodule types (`DomXssFinding`, `SpaRoute`, `ClientIssue`, `RequestCorpus`, etc.) are all public.

## Integration Points

### CLI

`handle_browser()` in `commands/handlers/browser.rs:5` routes through `EnforcementContext` with `OperationRisk::SafeActive` + `IntendedUse::WebAssessment`. Wraps `run_browser_scan()` in a `tokio::time::timeout()` with `args.timeout + BROWSER_TIMEOUT_BUFFER_MS` (buffer = 10,000ms per `eggsec-core::constants.rs:24`). Defaults: `DEFAULT_BROWSER_TIMEOUT_MS` = 60,000ms (`eggsec-core::constants.rs:23`).

### Dispatch

`Browser` is a recognized `Commands` variant in `commands/handlers/mod.rs:568`. The module is standalone (not integrated into `dispatch/` workers or the tool registry).

### No MCP/Agent/Pipeline Integration

The browser module is CLI-only. It does not register as an MCP tool, does not produce `TaskResult`s, and is not wired into the pipeline orchestrator.

## Testing

- `xss_dom.rs`: 14 tests (1 integration with live Chrome, 13 unit tests for severity calculation and remediation)
- `spa_discovery.rs`: 10 tests (1 integration with live Chrome, 9 unit tests for parameter extraction and display)
- `client_checks.rs`: 12 tests (1 integration with live Chrome, 11 unit tests for remediation and type display)
- `corpus.rs`: 5 tests (dedup, API filter, JSON serialization, default, RequestSource snake_case)

Integration tests in `xss_dom.rs`, `spa_discovery.rs`, and `client_checks.rs` require a running Chrome binary and network access.

## Invariants & Gotchas

1. **Chrome binary required**: `headless_chrome::Browser::default()` returns an error if Chrome is not installed. The stub returns a config error instead.
2. **XHR interceptors injected before navigation** (`mod.rs:49`): This ensures initial-load API calls are captured.
3. **Severity capped at 10.0** (`xss_dom.rs:164`): `(base_score * modifier).min(10.0)`.
4. **Deduplication by `method:url`** (`corpus.rs:97`): Same URL with different methods produces separate entries.
5. **No timeout on JS evaluation**: `tab.evaluate()` calls in `xss_dom.rs:110`, `spa_discovery.rs:94`, `client_checks.rs:160` rely on the tab's default timeout but do not add individual timeouts.
6. **`capture_requests` deserialization fallbacks**: All JSON deserialization uses `.unwrap_or_else()` with `tracing::warn!` — no panics on malformed data.

## Bugs / Observations

| Location | Issue | Severity |
|----------|-------|----------|
| `xss_dom.rs:53` | `unwrap_or_else` on `serde_json::to_string` for payload — fallback is a hardcoded string, but the actual error path is unreachable since `xss_payload` is a plain `String` | Low |
| `mod.rs:163` | `unwrap_or_else` on `serde_json::from_value` — properly logged with `tracing::warn!`, not silent | Informational |
| `client_checks.rs:153` | CORS test sends synchronous XHR in a try-catch — silently catches errors without logging | Low |
| Browser tests (`xss_dom.rs:211`, `spa_discovery.rs:178`, `client_checks.rs:256`) | Integration tests require Chrome; will fail in CI without it. No `#[ignore]` or conditional compilation | Medium |

*Last verified against source: 2026-08-25*
