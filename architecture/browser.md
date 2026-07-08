# Browser Module

## Purpose

Headless Chrome integration for browser-based security testing including DOM XSS detection, SPA route discovery, and client-side security checks. Feature-gated behind `headless-browser`.

## Key Types

| Type | Location | Description |
|------|----------|-------------|
| `BrowserConfig` | `browser/mod.rs` | Configuration for browser scan scope and options. Fields: `check_dom_xss`, `discover_spa_routes`, `check_client_security`, `timeout_ms`, `xss_payload` |
| `BrowserReport` | `browser/mod.rs` | Aggregated browser scan results |
| `DomXssFinding` | `browser/xss_dom.rs` | DOM XSS vulnerability finding. Fields: `id`, `source`, `sink`, `location`, `severity`, `description`, `evidence`, `remediation`, `cvss_score` |
| `XssSource` | `browser/xss_dom.rs` | XSS source enum (8 variants: location.hash, location.search, document.cookie, document.referrer, localStorage, sessionStorage, WebSocket, postMessage) |
| `XssSink` | `browser/xss_dom.rs` | XSS sink enum (10 variants: innerHTML, outerHTML, jQuery.html, document.write, eval, setTimeout, setInterval, Function, scriptSrc, onerror) |
| `SpaRoute` | `browser/spa_discovery.rs` | Discovered SPA route with discovery method |
| `DiscoveryMethod` | `browser/spa_discovery.rs` | How a route was found (Crawl, XhrInterception, FetchInterception, RouteParsing) |
| `ClientIssue` | `browser/client_checks.rs` | Client-side security issue. Fields: `id`, `issue_type`, `severity`, `location`, `description`, `evidence`, `remediation`, `cvss_score` |
| `ClientIssueType` | `browser/client_checks.rs` | Issue type enum (6 variants: LocalStorageSensitive, CorsMisconfiguration, CSPSourceMap, DebugMode, SourceMapsExposed, CORSWildcard) |
| `CorpusEntry` | `browser/corpus.rs` | Browser test corpus entry. Fields: `url`, `method`, `headers`, `body_shape`, `content_type`, `source`, `timestamp` |
| `CorpusHeader` | `browser/corpus.rs` | Request header entry: `name`, `value`, `redacted` |
| `BodyShape` | `browser/corpus.rs` | Request body shape: `content_type`, `fields` |
| `BodyField` | `browser/corpus.rs` | Body field descriptor: `name`, `field_type`, `required` |
| `RequestSource` | `browser/corpus.rs` | How request was observed: `Xhr`, `Fetch`, `Form`, `Navigation`, `WebSocket`, `Script`, `Other` |
| `RequestCorpus` | `browser/corpus.rs` | Complete corpus from a browser crawl session |
| `FormInfo` | `browser/corpus.rs` | Discovered form: `action`, `method`, `fields` |

## Files

| File | Description |
|------|-------------|
| `mod.rs` | Module root: `BrowserConfig`, `BrowserReport`, `run_browser_scan()` entry point, XHR/Fetch interceptor injection |
| `xss_dom.rs` | DOM XSS detection via source/sink tracing (8 sources × 10 sinks). `calculate_severity()` computes `base_score * modifier`, capped at 10.0 |
| `spa_discovery.rs` | Single Page App route discovery via DOM/JS analysis + XHR/Fetch interception |
| `client_checks.rs` | Client-side security checks (localStorage secrets, CORS, CSP, source maps, debug mode) |
| `corpus.rs` | Browser test corpus management with deduplication. Types: `CorpusEntry`, `CorpusHeader`, `BodyShape`, `BodyField`, `RequestSource`, `RequestCorpus`, `FormInfo` |

## Implementation Status

Implemented behind `headless-browser` feature flag. Core scanning logic is in place; returns an error when the feature is not enabled.

## Key Functions

- **`run_browser_scan(target, config)`** — Main entry point: opens headless Chrome tab, injects XHR/Fetch interceptors, runs DOM XSS scan, SPA discovery, client checks, and captures request corpus
- **`capture_requests(tab)`** — Captures forms, scripts, and GraphQL candidates from the page DOM and returns a `RequestCorpus`

## CLI Usage

```
eggsec browser https://example.com
eggsec browser https://example.com --no-dom-xss
eggsec browser https://example.com --no-spa --no-client-checks
eggsec browser https://example.com --json -o results.json
```
