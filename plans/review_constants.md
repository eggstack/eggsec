# Constants Module Architecture Review

**Document:** architecture/constants.md
**Reviewed:** 2026-06-02
**Accuracy:** High
**Lines Reviewed:** 43

## Verified Claims
- [PROJECT_QUALIFIER = "tools"]: Verified at `crates/slapper/src/constants.rs:7`
- [PROJECT_NAME = "slapper"]: Verified at `crates/slapper/src/constants.rs:8`
- [DEFAULT_EXPORT_DIR = "./exports"]: Verified at `crates/slapper/src/constants.rs:10`
- [DEFAULT_REMOTE_PORT = 7890]: Verified at `crates/slapper/src/constants.rs:12`
- [DEFAULT_TRACEROUTE_PORT = 33434]: Verified at `crates/slapper/src/constants.rs:13`
- [DEFAULT_PROXY_TIMEOUT_MS = 10000]: Verified at `crates/slapper/src/constants.rs:14`
- [DEFAULT_HEALTH_CHECK_INTERVAL_SECS = 60]: Verified at `crates/slapper/src/constants.rs:15`
- [DEFAULT_MAX_HEALTH_CHECK_FAILURES = 3]: Verified at `crates/slapper/src/constants.rs:16`
- [WAYBACK_SNAPSHOT_LIMIT = 100]: Verified at `crates/slapper/src/constants.rs:18`
- [DEFAULT_ICMP_PAYLOAD_SIZE = 56]: Verified at `crates/slapper/src/constants.rs:19`
- [DEFAULT_CONFIG_FILE = "slapper.toml"]: Verified at `crates/slapper/src/constants.rs:21`
- [DEFAULT_WORDLIST = "wordlists/directories.txt"]: Verified at `crates/slapper/src/constants.rs:22`
- [SUPPORTED_WAF_COUNT = 34]: Verified at `crates/slapper/src/constants.rs:24` with test at lines 26-38
- [http constants (DEFAULT_TIMEOUT_SECS: 30, DEFAULT_MAX_REDIRECTS: 10, DEFAULT_CONCURRENCY: 10)]: Verified at `crates/slapper/src/constants.rs:40-44`
- [scan constants (DEFAULT_PORT_RANGE: "1-1024", DEFAULT_PORT_CONCURRENCY: 100, DEFAULT_ENDPOINT_CONCURRENCY: 20)]: Verified at `crates/slapper/src/constants.rs:46-50`
- [cache constants (DEFAULT_TTL_SECS: 3600, DEFAULT_MAX_ENTRIES: 10000)]: Verified at `crates/slapper/src/constants.rs:52-55`
- [nvd constants (DEFAULT_RATE_LIMIT_DELAY_MS: 6000)]: Verified at `crates/slapper/src/constants.rs:57-59`
- [ui constants (CHECK_MARK, CROSS_MARK, ARROW, WIDTH_DEFAULT)]: Verified at `crates/slapper/src/constants.rs:61-67`
- [waf scoring constants (HEADER_MATCH_SCORE: 25, COOKIE_MATCH_SCORE: 20, BODY_MATCH_SCORE: 15, IP_MATCH_SCORE: 20, UNKNOWN_WAF_CONFIDENCE: 30, LENGTH_DIFF_THRESHOLD: 100, HIGH_CONFIDENCE_EXIT: 90)]: Verified at `crates/slapper/src/constants.rs:69-76`
- [BLOCKED_STATUS_CODES array]: Verified at `crates/slapper/src/constants.rs:77`

## Discrepancies
- None

## Bugs Found
- None

## Improvement Opportunities
- [Low]: The document lists `BLOCKED_PATTERNS` array constant but doesn't mention `WEAK_BLOCK_INDICATOR_PATTERNS` and `UNKNOWN_WAF_WEAK_PATTERN_THRESHOLD` which are also defined in the waf module (constants.rs:78-90)

## Stale Items
- None

## Code Interrogation Findings
- [Info]: There are additional waf constants not documented: `BLOCKED_PATTERNS` (8 elements), `WEAK_BLOCK_INDICATOR_PATTERNS` (4 elements), `UNKNOWN_WAF_WEAK_PATTERN_THRESHOLD` (constants.rs:78-90)
- [Info]: There's a compile-time test ensuring `SUPPORTED_WAF_COUNT` matches actual detector count (constants.rs:30-36)