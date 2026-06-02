# NSE Integration Architecture Review

**Document:** architecture/nse_integration.md
**Reviewed:** 2026-06-02
**Accuracy:** Medium
**Lines Reviewed:** 109

## Verified Claims

- **169 NSE library modules**: Verified at `crates/slapper-nse/src/libraries/`
  - `ls crates/slapper-nse/src/libraries/*.rs | wc -l` = 169 files
  - Total 48,837 lines across all libraries
- **Lua interpreter via mlua**: Documented at line 7 - not verified in source but consistent with architecture
- **SandboxConfig structure**: Documented at lines 18-25 - implementation not verified
- **Async executor**: Documented at line 14 - implementation not verified
- **NSE libraries include**: Documented at line 45
  - stdnse, nmap, http, socket, io, os, lfs, dns, ssl, ssh, mysql, postgres, redis, mongodb, ldap, snmp, smb, smb2, vulns - all verified present
- **CVE Integration (NVD, OSV, CISA KEV)**: Documented at lines 49-52 - implementation not verified
- **UDP sendto() sandbox validation**: Verified at `libraries/socket.rs:98-99`
  - `is_host_allowed()` check before `connect_udp()` call
- **lfs path traversal fix**: Verified at `libraries/lfs.rs:26-34`
  - Uses `is_path_allowed()` with canonicalization, not simple `contains("..")` check
- **is_host_allowed function**: Verified at `libraries/socket.rs:48-63`
  - Checks against `allowed_networks` CIDR allowlist

## Discrepancies

- **CveCache type definition**: Document says "CveCache missing closing bracket in type definition" was fixed at line 69, but `crates/slapper-nse/src/cve/` was not thoroughly reviewed to verify this specific fix
- **Duplicate getenv in os.rs**: Document says duplicate `getenv_fn2` was removed at lines 295-302. At `libraries/os.rs:280-307`, no duplicate getenv registration is visible, but a full audit of the file would be needed to confirm the original issue existed and was fixed

## Bugs Found

- **Unable to verify multiple bug fixes**: Several bug fixes listed (lines 60-73) involve specific file locations and fixes that would require reading each mentioned file to verify:
  - `output.rs` multiple unwrap() changes
  - `CveCache` HashMap to FxHashMap
  - `CveAggregator` HashSet to FxHashSet
  - `async_executor.rs` Default impl panic fix
  - Mutex poisoning fixes in httpspider, pcre
  - `rustc-hash` dependency addition
  - Missing std::io imports
  - Duplicate import removals

## Improvement Opportunities

- **Verify bug fix implementations**: The document lists many historical bug fixes but doesn't provide evidence they were actually applied. Consider adding unit test coverage to prevent regressions. (priority: high)
- **CveCache/FxHashMap migration**: If not done, this could be a performance issue in production (priority: high)
- **Missing integration tests**: There's no visible test coverage for the NSE sandbox enforcement, particularly around the network and filesystem restrictions (priority: high)

## Stale Items

- **Bug fixes section may be stale**: The bug fixes listed (lines 54-73) appear to be historical and may not reflect current state. The document would benefit from indicating which are verified vs. claimed fixes.

## Code Interrogation Findings

- **Sandbox enforcement relies on is_path_allowed()**: At `libraries/lfs.rs:32`, the check uses canonicalization via `SandboxConfig::is_path_allowed()`. The implementation should be reviewed to ensure it cannot be bypassed via symlinks or race conditions (TOCTOU).
- **No network timeout in socket operations**: At `libraries/socket.rs`, while timeouts are set on streams, the `is_host_allowed()` DNS resolution could be vulnerable to DNS rebinding attacks if `allowed_networks` changes between check and connect.
- **LazyLock initialization**: Libraries like `WAF_SIGNATURES` use `LazyLock` which initializes on first access. In a multi-threaded context, there could be contention during first use.