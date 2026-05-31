# Browser Architecture Review
**Document:** architecture/browser.md
**Reviewed:** 2026-05-31
**Accuracy:** High
**Lines Reviewed:** 30

## Verified Claims
- Feature-gated behind `headless-browser`: Verified at `crates/slapper/src/browser/mod.rs:38,62`
- `BrowserConfig` struct: Verified at `crates/slapper/src/browser/mod.rs:70` with fields `check_dom_xss`, `discover_spa_routes`, `check_client_security`, `crawl_depth`, `timeout_ms`
- `BrowserReport` struct: Verified at `crates/slapper/src/browser/mod.rs:21` with fields `target`, `dom_xss`, `spa_routes`, `client_issues`, `total_findings`
- `DomXssFinding` struct: Verified at `crates/slapper/src/browser/xss_dom.rs:9` with fields `id`, `source`, `sink`, `location`, `severity`, `description`, `evidence`, `remediation`, `cvss_score`
- `SpaRoute` struct: Verified at `crates/slapper/src/browser/spa_discovery.rs:8` with fields `path`, `method`, `parameters`, `discovered_via`
- `ClientIssue` struct: Verified at `crates/slapper/src/browser/client_checks.rs:8` with fields `id`, `issue_type`, `severity`, `location`, `description`, `evidence`, `remediation`, `cvss_score`
- `CorpusEntry` struct: Verified at `crates/slapper/src/browser/corpus.rs:11` with fields `url`, `method`, `headers`, `body_shape`, `content_type`, `source`, `timestamp`
- `run_browser_scan()` entry point: Verified at `crates/slapper/src/browser/mod.rs:39` (feature-enabled) and line 63 (feature-disabled error)
- Error when feature not enabled: Verified at `crates/slapper/src/browser/mod.rs:64-66`
- All files present: `mod.rs`, `xss_dom.rs`, `spa_discovery.rs`, `client_checks.rs`, `corpus.rs` - verified

## Discrepancies
- None. All documented types, files, and feature-gate behavior match the actual codebase.

## Bugs Found
- None

## Improvement Opportunities
- The document does not mention `XssSource` enum (`crates/slapper/src/browser/xss_dom.rs:22`) or `DiscoveryMethod` enum (`crates/slapper/src/browser/spa_discovery.rs:16`) or `ClientIssueType` enum (`crates/slapper/src/browser/client_checks.rs:20`). These are significant types used in the sub-modules.
- The document does not mention `CorpusHeader`, `BodyShape`, `RequestSource` types in `corpus.rs`. Consider adding these to the Key Types table.

## Stale Items
- None
