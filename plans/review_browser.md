# Browser Module Architecture Review

**Document:** architecture/browser.md
**Reviewed:** 2026-06-02
**Accuracy:** High
**Lines Reviewed:** 30

## Verified Claims
- [BrowserConfig]: Verified at `crates/slapper/src/browser/mod.rs:69`
- [BrowserReport]: Verified at `crates/slapper/src/browser/mod.rs:21`
- [DomXssFinding]: Verified at `crates/slapper/src/browser/xss_dom.rs:9`
- [SpaRoute]: Verified at `crates/slapper/src/browser/spa_discovery.rs:8`
- [ClientIssue]: Verified at `crates/slapper/src/browser/client_checks.rs:8`
- [CorpusEntry]: Verified at `crates/slapper/src/browser/corpus.rs:11`
- [run_browser_scan() entry point]: Verified at `crates/slapper/src/browser/mod.rs:39` (feature-gated) and line 63 (error fallback)
- [Feature-gated behind headless-browser]: Verified at `crates/slapper/src/browser/mod.rs:38` and `mod.rs:62`

## Discrepancies
- None significant.

## Bugs Found
- [Hardcoded test payload in XSS scanner]: `crates/slapper/src/browser/xss_dom.rs:98` uses a static payload `<img src=x onerror=alert(1)>`. This is easily detected by WAFs and should be configurable/parameterized (priority: medium)

## Improvement Opportunities
- [Incomplete client_checks.rs coverage]: The `ClientIssueType` enum defines 8 variants (LocalStorageSensitive, CorsMisconfiguration, CSPSourceMap, DebugMode, SourceMapsExposed, CORSWildcard, WeakCiphers, CertificateIssues), but the JavaScript detection only handles 3 (LocalStorageSensitive, SourceMapsExposed, DebugMode). CORS, WeakCiphers, and CertificateIssues are not actually detected (priority: medium)
- [SPA route discovery limited]: The `discover_routes()` function only parses DOM links/forms and inline JS. It doesn't actually crawl pages or handle client-side routing libraries (React Router, Vue Router, etc.) beyond pattern matching (priority: medium)

## Stale Items
- [corpus.rs not integrated]: The `RequestCorpus` and `CorpusEntry` types exist but are not used by `run_browser_scan()`. The corpus functionality appears unused (priority: low)

## Code Interrogation Findings
- [Browser connection without error handling]: In `xss_dom.rs:69-74`, `Browser::default()` and `browser.new_tab()` can fail, but the error is propagated via `?` without specific handling. Consider adding retries or better error messages.
- [SPA route parameters limited]: Parameter extraction in `spa_discovery.rs:166-179` only handles `{param}` and `:param` patterns. Doesn't handle React Router v6 `*` catch-all routes or other framework-specific patterns.