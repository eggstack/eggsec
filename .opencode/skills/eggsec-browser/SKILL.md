---
name: eggsec-browser
description: "Headless browser security testing - use when working with DOM XSS detection, SPA route discovery, client-side security checks, or browser-based vulnerability scanning."
---

# Eggsec Browser Skill

Headless browser security testing module.

## Module Location
`crates/eggsec/src/browser/`

## Tab
Browser tab is one of the 33 TUI tabs - see `eggsec-tui/SKILL.md` for TUI patterns.

## Key Types

- `BrowserConfig` - Browser configuration
- `BrowserReport` - Scan results and findings
- `run_browser_scan()` - Entry point for browser security scanning
- `DomXssFinding` - DOM XSS vulnerability findings
- `ClientIssue` - Client-side security issues (with `ClientIssueType` enum)
- `SpaRoute` - SPA route discovery results
- `RequestCorpus` - Request corpus for testing

## Patterns

### Browser Scan
```rust
let config = BrowserConfig::new(target);
let report = run_browser_scan(target, config).await?;
```

## Key Files
- `mod.rs` - Main browser engine, `BrowserConfig`, `BrowserReport`, `run_browser_scan()`
- `xss_dom.rs` - DOM XSS detection (`DomXssFinding`, `XssSource`, `XssSink`)
- `spa_discovery.rs` - SPA route discovery (`SpaRoute`, `DiscoveryMethod`)
- `client_checks.rs` - Client-side checks (`ClientIssue`, `ClientIssueType`)
- `corpus.rs` - Request corpus building (`RequestCorpus`, `CorpusEntry`)

## Module Notes
See `architecture/browser.md` for architecture documentation.