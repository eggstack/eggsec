# Logging Module Architecture Review

**Document:** architecture/logging.md
**Reviewed:** 2026-06-02
**Accuracy:** Medium
**Lines Reviewed:** 25

## Verified Claims
- [Defined in crates/slapper/src/logging/]: Verified
- [Files: mod.rs and init.rs]: Verified - both files exist
- [mod.rs re-exports init_logging, LogFormat, LogLevel]: Verified at `crates/slapper/src/logging/mod.rs:1-3`
- [init_logging function]: Verified at `crates/slapper/src/logging/init.rs:53`
- [LogFormat enum]: Verified at `crates/slapper/src/logging/init.rs:8-14` (Pretty, Json, Compact)
- [LogLevel enum]: Verified at `crates/slapper/src/logging/init.rs:16-24` (Info, Debug, Trace, Warn, Error)

## Discrepancies
- [Architecture doc doesn't mention logging macros]: `init.rs` defines 4 additional macros that are not documented: `log_request!`, `log_scan_progress!`, `log_finding!`, `log_error_context!` (init.rs:83-131)

## Bugs Found
- None

## Improvement Opportunities
- [Medium]: Missing documentation for 4 macros defined in init.rs:
  - `log_request!` - logs HTTP request completion
  - `log_scan_progress!` - logs scan progress with percentage
  - `log_finding!` - logs security findings as warnings
  - `log_error_context!` - logs errors with context
- [Low]: The `init_logging` function signature shows 3 parameters (level, format, json_output) but the document doesn't describe the json_output boolean parameter

## Stale Items
- None

## Code Interrogation Findings
- [Info]: init_logging takes 3 parameters: `level: LogLevel`, `format: LogFormat`, `json_output: bool` (init.rs:53)
- [Info]: LogLevel implements Display (returns lowercase string), FromStr (accepts "trace", "debug", "info", "warn"/"warning", "error"), Default
- [Info]: LogFormat only implements Debug and Default (Pretty is default)
- [Info]: The log macros use `expr_2021` syntax suggesting they require Rust 2021 edition