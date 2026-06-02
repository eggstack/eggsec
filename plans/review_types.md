# Core Types Architecture Review

**Document:** architecture/types.md
**Reviewed:** 2026-06-02
**Accuracy:** High
**Lines Reviewed:** 41

## Verified Claims
- [Severity enum with variants Critical, High, Medium, Low, Info (default)]: Verified at `crates/slapper/src/types.rs:16-23`
- [parse_or_default method]: Verified at `crates/slapper/src/types.rs:29-31`
- [from_cvss(score) method with >=9.0=Critical, >=7.0=High, >=4.0=Medium, >=0.1=Low]: Verified at `crates/slapper/src/types.rs:33-42`
- [as_str() returns lowercase]: Verified at `crates/slapper/src/types.rs:44-53`
- [as_int() returns ranking (Critical=4, High=3, Medium=2, Low=1, Info=0)]: Verified at `crates/slapper/src/types.rs:55-64`
- [cvss_color() returns emoji]: Verified at `crates/slapper/src/types.rs:66-75`
- [Implements Ord based on as_int()]: Verified at `crates/slapper/src/types.rs:110-114`
- [SensitiveString with Zeroize + ZeroizeOnDrop]: Verified at `crates/slapper/src/types.rs:127-128`
- [Constant-time comparison (ct_eq)]: Verified at `crates/slapper/src/types.rs:232` (uses subtle::ConstantTimeEq)
- [Hash intentionally not implemented]: Verified - no Hash impl in types.rs
- [expose_secret() / into_secret()]: Verified at `crates/slapper/src/types.rs:148-158`
- [log_secret() / for_logging()]: Verified at `crates/slapper/src/types.rs:163-193`
- [Serializes in plaintext]: Verified at `crates/slapper/src/types.rs:208-222` (with security warning in docs)
- [OutputFormat with 8 variants (Pretty, Json, Compact, Html, Csv, Sarif, Junit, Markdown)]: Verified at `crates/slapper/src/types.rs:310-320`
- [check_config_file_permissions function]: Verified at `crates/slapper/src/types.rs:269-303`

## Discrepancies
- None

## Bugs Found
- None

## Improvement Opportunities
- [Low]: The document doesn't mention `std::str::FromStr` impl for Severity that accepts "moderate" as alias for Medium (types.rs:97) - this is a useful feature worth documenting
- [Low]: The document mentions `check_config_file_permissions` but doesn't describe its behavior (warns on world-readable or group-readable permissions)

## Stale Items
- None

## Code Interrogation Findings
- [Info]: Severity has additional impl for Display that returns uppercase (e.g., "CRITICAL") which differs from as_str() lowercase
- [Info]: SensitiveString has Debug impl that always shows "SensitiveString([REDACTED])" regardless of actual value
- [Info]: SensitiveString Display impl returns "[REDACTED]" always
- [Info]: check_config_file_permissions is Unix-only (uses std::os::unix::fs::PermissionsExt) and silently returns if metadata can't be read