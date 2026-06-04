# Browser Module

## Purpose

Headless Chrome integration for browser-based security testing including DOM XSS detection, SPA route discovery, and client-side security checks. Feature-gated behind `headless-browser`.

## Key Types

| Type | Location | Description |
|------|----------|-------------|
| `BrowserConfig` | `browser/mod.rs` | Configuration for browser scan scope and options |
| `BrowserReport` | `browser/mod.rs` | Aggregated browser scan results |
| `DomXssFinding` | `browser/xss_dom.rs` | DOM XSS vulnerability finding |
| `SpaRoute` | `browser/spa_discovery.rs` | Discovered SPA route |
| `ClientIssue` | `browser/client_checks.rs` | Client-side security issue |
| `CorpusEntry` | `browser/corpus.rs` | Browser test corpus entry |

## Files

| File | Description |
|------|-------------|
| `mod.rs` | Module root: `BrowserConfig`, `BrowserReport`, `run_browser_scan()` entry point |
| `xss_dom.rs` | DOM XSS detection via source/sink tracing (8 sources × 8 sinks) |
| `spa_discovery.rs` | Single Page App route discovery via DOM/JS analysis + XHR/Fetch interception |
| `client_checks.rs` | Client-side security checks (localStorage secrets, CORS, CSP, source maps, debug mode) |
| `corpus.rs` | Browser test corpus management with deduplication |

## Implementation Status

Implemented behind `headless-browser` feature flag. Core scanning logic is in place; returns an error when the feature is not enabled.

## CLI Usage

```
slapper browser https://example.com
slapper browser https://example.com --no-dom-xss
slapper browser https://example.com --no-spa --no-client-checks
slapper browser https://example.com --json -o results.json
```
