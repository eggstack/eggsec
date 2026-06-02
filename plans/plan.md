# Slapper Implementation Plan

**Created:** 2026-05-30
**Last Updated:** 2026-06-02
**Status:** In Progress (148 items from architecture review)

---

## Summary

| Priority | Count |
|----------|-------|
| HIGH | 13 |
| MEDIUM | 55 |
| LOW | 80 |
| **Total** | **148** |

---

## Non-Goals

- Do NOT add new offensive capability
- Do NOT reintroduce Python/Ruby plugin runtimes
- Do NOT publish crates or flip visibility unless instructed
- Do NOT invent domains/organizations/support contacts
- Do NOT claim production maturity for experimental features
- Do NOT remove NSE support
- Do NOT perform large architectural rewrites in single passes

---

## Completed Waves (Historical)

### Wave 0: Critical Bug Fixes
- Clock skew panic prevention (`routing.rs`)
- `spoof_ip` rename
- `unwrap_or` clarity improvements

### Wave 1: Architecture Documentation (52 items)
- 5 sub-waves (1A-1E): counts, structure, AI/MCP, recon, stub modules

### Wave 2: Agent & MCP Profile Productionization (12 phases)
- Phase 7: `CodingAgentFindingReport` typed struct (new)

### Wave 3: Output Module Documentation

### Wave 4: Critical Bug Fixes
- 4.1 SLA calculation bug fix (`workflow/mod.rs`)
- 4.2 Discord notify dispatch bug (`notify/mod.rs`)
- 4.3 Storage module stubs documented (`storage/postgres.rs`)
- 4.4 Feature matrix math error (`architecture/feature_matrix.md`)
- 4.5 Findings architecture doc wrong type (`architecture/findings.md`)
- 4.6 Lib.rs stale docstrings
- 4.7 ~~Output AttackGraph feature gate~~ — Removed (not a bug)

### Wave 5: Type Name & Count Corrections (15 items)
- All type names, counts, and descriptions corrected across architecture docs

### Wave 6: Documentation Gaps (14 items)
- **Group A:** Error From impls, Recon counts, Proxy completeness, Output report_summary.rs
- **Group B:** TUI completeness, Pipeline executor fields, Distributed completeness, Loadtest completeness, Findings completeness
- **Group C:** Networking completeness, Container missing types, Compliance report.rs, Vuln completeness, Hunt feature gate

### Wave 7: Uncovered Module Documentation (9 items)
- Created `architecture/` docs for: stress, utils, types, constants, probe, auth_context, logging, macros, generated

---

## HIGH Priority Items (Action Required)

### Container
- **Docker Scanner Shell Injection Risk:** `crates/slapper/src/container/docker.rs:208-209` uses `std::process::Command::new("docker")` with `args(["inspect", _image_name])`. If `_image_name` contains special characters, this could lead to command injection. Validate image names before passing to shell; reject or sanitize special characters.

### Defense Lab
- **Verify RunManifest Location:** Document references `crates/slapper/src/output/run_manifest.rs` as defining RunManifest, but this was not verified during review. Read `output/run_manifest.rs` to verify it exists and contains the RunManifest struct definition.

### Loadtest
- **Semaphore Unwrap Could Panic:** At `runner.rs:315`, the semaphore acquire uses `.unwrap()` which could panic if the semaphore is closed: `let _permit = sem.acquire().await.unwrap();`. Handle the error explicitly instead of unwrapping - use match or map_err with tracing.

### Networking
- **PcapWriter Write Result Silently Dropped:** At `capture.rs:209`, the PcapWriter `write_packet` result is silently dropped with `let _ = writer.write_packet(&packet)`. While PcapWriter itself handles errors properly, the caller ignores the result which could hide write failures. Log a warning when write_packet fails instead of silently dropping the result.

### NSE
- **Bug Fix Implementations Unverified:** Historical bug fixes listed in documentation cannot be verified without reading each file. Add unit test coverage to prevent regressions; verify each listed fix was actually applied.
- **CveCache/FxHashMap Migration:** If not migrated, using HashMap instead of FxHashMap could be a performance issue. Verify CveCache uses FxHashMap and add migration if needed.
- **Missing Sandbox Integration Tests:** No visible test coverage for NSE sandbox enforcement (network and filesystem restrictions). Add integration tests for sandbox enforcement, particularly around network and filesystem restrictions.

### Output
- **Format Count Mismatch:** Document header says "8 output formats" but table at lines 11-18 lists only 7 (JSON, SARIF, HTML, Markdown, PDF, CSV, JUnit XML). The `OutputFormat` enum actually has 8 variants: `Pretty`, `Json`, `Compact`, `Html`, `Csv`, `Sarif`, `Junit`, `Markdown`. The table includes PDF (not in enum) and omits `Pretty` and `Compact` (which ARE in enum). Correct the documentation to accurately reflect the enum variants.

### Pipeline
- **CSV Escape in Pipeline Module Lacks NFKC:** `pipeline/report.rs:10-22` has its own `escape_csv()` without NFKC normalization, unlike `output/escape.rs:16-35`. Use the NFKC-normalized `escape_csv()` from output module in pipeline.
- **Defense-Lab Profile Stage Counts Incorrect:** Stage counts in `pipeline.md:136-142` may be incorrect for defense-lab profiles. Verify against actual implementation in `stage.rs:92-107`.
- **Missing Defense-Lab Profiles from Available Stages Table:** "Available Stages" table at `pipeline.md:23-34` lists only 11 profiles, missing all 5 defense-lab profiles. Add all 5 defense-lab profiles to the Available Stages table with their correct stage counts.

### Recon
- **Module Count Accurate:** ~~Document says 17 modules but `FULL_RECON_PIPELINE_MODULES` at `mod.rs:350-368` has 18 entries~~ - VERIFIED: The count IS 17 entries (reverse_dns, geolocation, threatintel, ssl, whois, subdomain, dns_records, techdetect, js, wayback, cloud, content, cors, email, takeover, cve, secrets). The architecture doc is correct. This item can be removed.

### Storage
- **Stub Implementation Misleading Documentation:** The `Database` struct explicitly states "WARNING: Stub implementation - not connected to a real database". All methods return empty results. Architecture document does not mention this.
- **No Actual SQLx Integration:** Despite being described as a "SQLx-based persistence layer", there is no SQLx dependency used. All database methods are stubs returning empty results.

### Stress
- **StressConfig Documentation Field Names Wrong:** Documentation says `rate_limit` but actual field is `rate_pps`; says `threads` but actual is `concurrency`. Update architecture/stress.md to use correct field names.
- **StressConfig Missing Fields in Documentation:** Several StressConfig fields not documented: `spoof_range`, `random_source_port`, `payload_size`, `use_proxies`, `proxy_pool`. Add missing fields to architecture/stress.md StressConfig documentation.

### Supply Chain
- **No Actual Vulnerability Lookup in SBOM:** `SbomReport` has a `vulnerabilities: Vec<SbomVulnerability>` field but `generate_from_cargo()`, `generate_from_npm()`, etc. all return empty vectors. No actual CVE lookup implemented.

### Vuln
- **VulnAssessment Is a Stub That Can't Hold Structured Findings:** The `VulnAssessment` struct at `mod.rs:37-40` only has `mode: String`, `results: Vec<String>`, and `assessed_at: DateTime`. It cannot store actual structured findings.

---

## MEDIUM Priority Items

### Auth
- **Incomplete `run_full_test()` Implementation:** The `run_full_test()` method only runs 3 of 8 test types (RateLimitBypass, TimingAttack, SessionFixation). Missing: BruteForce, CredentialStuffing, AccountLockout, MfaBypass, PasswordPolicy.
- **BruteForceTester Not Invoked by Engine:** `AuthEngine::run_full_test()` does not invoke `BruteForceTester` even though the struct is exported. The brute force module may be unused.
- **Multi-Protocol Testers Not Integrated:** SSH/FTP/SMTP testers exist in `multi_protocol.rs` but are not integrated into `run_full_test()`. The auth engine only orchestrates HTTP-based tests.
- **SSH/FTP/SMTP Require `nse-ssh2` Feature:** `multi_protocol.rs` uses `ssh2::Session` which requires the `nse-ssh2` feature. If not enabled, compilation will fail.

### Auth Context
- **Cookies Interpolation Not Documented:** Document only mentions headers interpolation, but cookies are also interpolated (mod.rs:44-46). Update architecture/auth_context.md to document that cookie values also support environment variable interpolation.

### Browser
- **Hardcoded XSS Test Payload:** XSS scanner uses static payload `<img src=x onerror=alert(1)>` which is easily detected by WAFs. Make payload configurable/parameterized to allow customization.
- **Incomplete Client Checks Coverage:** `ClientIssueType` enum defines 8 variants but only 3 are detected (LocalStorageSensitive, SourceMapsExposed, DebugMode); CORS, WeakCiphers, CertificateIssues are not implemented.
- **SPA Route Discovery Limited:** `discover_routes()` only parses DOM links/forms and inline JS; doesn't crawl pages or handle client-side routing libraries beyond pattern matching. Expand to handle React Router v6, Vue Router, and other framework-specific patterns.

### Compliance
- **Compliance Framework Modules Return Mock Data:** The owasp.rs, pci.rs, hipaa.rs, soc2.rs modules likely return simplified/mock compliance reports rather than actual framework-specific checks.

### Container
- **Kubernetes Scanner Silently Fails:** API calls use `.ok()` on results (lines 65, 104, 163, 195, 254), silently ignoring network errors and returning empty results, making debugging difficult.
- **Docker Socket Access Not Checked:** The escape detection in `escape.rs` checks for docker.sock in config strings but doesn't actually verify if the container has access to the Docker socket.
- **CIS Benchmark Checks Are Simplistic:** CIS checks in `cis.rs` use simple string matching (e.g., `lower.contains("privileged")`) which can produce false positives/negatives.

### Diff
- **DiffEngine/BaselineComparison Locations Unverified:** Document references `output/diff.rs` for DiffEngine and `output/baseline.rs` for BaselineComparison, but these were not verified. Verify `output/diff.rs` and `output/baseline.rs` files exist and contain the expected types.

### Error
- **Lossy `From<anyhow::Error>` Conversion:** The `From<anyhow::Error>` impl maps all anyhow errors to `RequestFailed` variant with method="UNKNOWN" and url="unknown". This is lossy and may make debugging harder.

### Findings
- **Non-Cryptographic Fingerprint Computation:** `compute_fingerprint()` uses `std::collections::hash_map::DefaultHasher` (SipHash). For security-sensitive deduplication, a cryptographically secure hash (SHA-256) is more appropriate.
- **FindingStore Lacks Deduplication:** `store_finding()` appends to JSONL file without checking for duplicates. Document claims "deduplication" but implementation does not deduplicate.

### Fuzzer
- **Silent Error Suppression in fuzz_endpoint:** `fuzz_endpoint` silently continues on request failure without proper error propagation.
- **RegexExecutor Long-Running Tasks:** `max_iterations: 100000` in `check_pattern_async` could lead to long-running tasks on complex regexes.
- **TimingAnalyzer Clone Divergence:** `Clone` implementation for `TimingAnalyzer` may diverge during parallel fuzzing.
- **Endpoint Silent Error Continue:** `fuzz_endpoint` at `fuzzer/api_schema/mod.rs:291-306` silently continues on request failure. Log at warn level or add failed requests to a counter.

### Hunt
- **Missing Timeout Enforcement Per Check:** While `HuntConfig` has a `timeout_ms` field, the actual enforcement of this timeout per sub-module check is not visible in `run_hunt()`.
- **No Error Handling in run_hunt():** The function returns `Result<HuntReport>` but none of the sub-module calls have error handling. If any sub-module returns an error, it will abort the entire hunt.

### Integrations
- **IssueTracker Trait Should Be Async:** The IssueTracker trait methods (`create_issue`, `update_issue`, `add_comment`, `get_issue`, `search_issues`) are synchronous and return `Result`. Convert to `async fn` signatures to support proper async API client implementations.

### Kubernetes
- **Scanner Silent Failures:** API calls use `.ok()` on results at `kubernetes.rs:65, 104, 163, 195, 254`, silently ignoring errors. Log network errors instead of silently ignoring them.

### Logging
- **Missing Logging Macros Documentation:** 4 macros (`log_request!`, `log_scan_progress!`, `log_finding!`, `log_error_context!`) at `logging/init.rs:83-131` not documented. Document these 4 macros or note them as internal.
- **json_output Parameter Not Documented:** The `init_logging` function takes 3 parameters (level, format, json_output) but the document doesn't describe the `json_output` boolean parameter.

### NSE
- **TOCTOU Vulnerability in lfs Path Traversal:** `is_path_allowed()` could be bypassed via symlinks or race conditions.
- **DNS Rebinding Attack Vector:** `is_host_allowed()` DNS resolution could be vulnerable to DNS rebinding if `allowed_networks` changes between check and connect.

### Notify
- **Silent Error Suppression with `let _`:** `let _ = notifier.notify(&payload).await;` silently ignores notification failures at `notify/mod.rs:114, 140-143, 219-222, 293-296`. Replace with `tracing::warn` or similar error logging to avoid silent failures.
- **No Retry Logic for Failed Notifications:** `WebhookNotifier::notify()` does not implement retry logic; failed webhooks are not retried. Implement retry logic with backoff for failed webhook deliveries.

### Pipeline
- **Bug Fixes Table May Be Stale:** "Recent Bug Fixes" table at lines 150-165 references 2026-05-22 fixes but does not mention the defense-lab profile stage count discrepancy. Review and update the bug fixes table to include the defense-lab stage count issue, or archive if no longer relevant.

### Proxy
- **Pool::add Signature Inconsistency:** `ProxyPool::add()` at `pool.rs:74` takes `&mut self` but `ProxyManager::add_proxy()` at `proxy/mod.rs:48-52` calls it through `RwLockWriteGuard` which provides interior mutability. This is inconsistent API design but not a runtime bug.
- **Rotation Strategies Documentation Incomplete:** Document describes `ProxyRotator` as having "round-robin, least-used, random" strategies (3 strategies) but actual implementation has 5: RoundRobin, Random, Weighted, LeastUsed, LowestLatency.
- **SOCKS4 Health Check May Not Work Correctly:** `HealthChecker::check_proxy()` at `proxy/health.rs:78-104` uses `socks5` for all SOCKS proxies (Socks4 and Socks5) and for Tor. Socks4 support may not work correctly with this approach.

### Recon
- **Secret Detection Pattern Count Unverified:** Document claims "25+ regex patterns" but count was not verified.
- **IAM Privilege Escalation Patterns Count Unverified:** Document claims "12 known patterns" but count was not verified.
- **Sensitive Files Count Should Be Verified:** Document claims "80+ sensitive files" but exact number should be verified.

### Scanner
- **Silent Error Suppression Verification:** The silent error suppression change at `endpoints.rs:768` may not actually log instead of silently dropping errors.
- **UDP Fingerprinting Timeout Handling:** UDP probes may hang indefinitely on closed ports.
- **cms/joomla.rs Bounds Check Edge Cases:** The bounds check fix at `cms/joomla.rs:88-89` should be verified for empty strings and malformed XML.

### Storage
- **Sensitive Passwords Not Encrypted at Rest:** `StorageConfig` stores `password: SensitiveString`, but there's no encryption at rest. Database credentials may be logged in plain text if Debug trait is used.

### Stress
- **Authorization Behavior Simplified in Docs:** Documentation describes authorization as "displaying warnings" but actual implementation enforces scope validation, rate limits, and duration limits. Update architecture/stress.md to accurately describe StressAuthorization's enforcement behavior.

### Supply Chain
- **SBOM Generation Limited to 3 Ecosystems:** SBOM generator only supports Cargo, npm, and Python (requirements.txt). Missing: Go modules, Ruby gems, Java Maven/Gradle, .NET NuGet.
- **TyposquatDetector Has Hardcoded Package List:** `typosquat.rs:42-86` has a static list of 45 "well known packages" that will become stale.
- **Type Duplication in SupplyChainFinding:** `scanner.rs:46` defines its own `SupplyChainFinding` struct which is different from `supply_chain/mod.rs:23`. This causes confusion.

### Utils
- **Module Count Mismatch (23 vs 21):** Document incorrectly states "23 files" in subtitle but only lists 21 modules in the table. Correct the subtitle count to match actual module count or add missing modules to table.
- **Missing Serialization Module:** The utils directory has `serialization.rs` but the architecture document's table does not include it. Add `serialization` module to the architecture document table.

### Vuln
- **ExploitInfo::assess() Uses Year-Based Heuristics:** `exploit.rs:16-17` determines exploit availability based on whether the CVE ID contains "2021" or "2022". This will become increasingly inaccurate as time passes.
- **Triage Uses Simple Keyword Matching:** `triage.rs:43-55` uses simple keyword arrays for duplicate/false positive detection, resulting in high false positive/negative rates.
- **Remediation Steps Are Generic Templates:** `remediation.rs:25-78` returns hardcoded remediation steps based only on severity. Real remediation guidance should be tailored to the specific vulnerability/CVE.
- **CVSS Calculation May Have Bugs:** The CVSS 3.1 implementation in `cvss.rs` has a custom `min!` macro and complex calculations. The `#![allow(clippy::too_many_arguments)]` at line 53 suggests known issues.
- **No CVSS Vector Parsing Validation:** `calculate_base_score_from_vector()` at line 147 silently ignores invalid vector components. Malformed vectors produce incorrect scores without warning.

### WAF
- **BLOCKED_STATUS_CODES Verification:** Document claims 503 should be checked but need to verify `BLOCKED_STATUS_CODES` constant includes it.
- **No Timeout on Bypass Attempts:** `BypassEngine::run_bypasses` lacks explicit timeouts per bypass technique.

### WebSocket
- **Silent Error Suppression in close():** In `websocket/injection.rs:95`, `websocket/connection.rs:58`, and `websocket/fuzz.rs:85,124,152`, WebSocket streams are closed with `let _ = ws_stream.close(None).await;` which silently ignores close failures. Replace `let _ =` pattern with explicit error logging using `tracing`.
- **Unverified Test File Reference:** Document claims 7 tests exist in `fuzzer/payloads/websocket.rs:349-411` but this file was not verified. Verify the correct test file location or update the reference to point to actual test locations.

### Workflow
- **No Actual Persistence in Assignment/Comment:** `Assignment::new()` and `Comment::new()` create in-memory structs but don't persist to a database. If the application restarts, all assignments and comments are lost.
- **SLA Policies Hardcoded:** `sla.rs:11-34` defines default policies with hardcoded hour values (Critical=24h, High=168h, Medium=720h, Low=2160h, Info=8760h).

---

## LOW Priority Items

### AI Agents
- **CodingAgent Tool List Mismatch:** The architecture document lists `"scan", "scan-ports", "fingerprint", "endpoints", "waf-detect", "search"` as CodingAgent allowed tools, but the test at `policy.rs:498-522` shows `endpoints` is allowed but not documented.
- **Missing Clone Derive Documentation:** AiClient implements Clone manually (verified at `client.rs:602-608`) but not as a derived trait. Documentation could clarify this is intentional for internal Arc fields.
- **Feature-Gated Skills Module Not Documented:** The `skills.rs` file is conditionally compiled with `#[cfg(feature = "ai-integration")]`, correctly noted in `agent/mod.rs:19-20` but the architecture document doesn't explicitly call out this gating.

### Auth
- **Hardcoded `stop_on_lockout=true`:** `AuthEngine::new()` always sets `stop_on_lockout: true`, ignoring any parameter passed.

### Auth Context
- **ServiceValidation Serialization Format Not Documented:** Architecture doc mentions ProbeRisk has 6 variants but doesn't document that ProbeIntent::ServiceValidation serializes as "service-validation". Add documentation noting that ProbeIntent::ServiceValidation serializes as "service-validation" (kebab-case).

### Browser
- **Browser Connection Without Error Handling:** `Browser::default()` and `browser.new_tab()` can fail but error is propagated via `?` without specific handling. Add retries or better error messages for browser connection failures.
- **Corpus Module Not Integrated:** `RequestCorpus` and `CorpusEntry` types exist but are not used by `run_browser_scan()`. Either integrate corpus functionality into browser scan or remove as dead code.
- **SPA Route Parameters Limited:** Parameter extraction only handles `{param}` and `:param` patterns; doesn't handle React Router v6 `*` catch-all routes. Add support for React Router v6 `*` catch-all and other framework-specific patterns.

### Compliance
- **OWASP Report Framework Name Mismatch:** In `compliance/mod.rs:82`, the test expects `report.framework == "OWASP Top 10"` but ComplianceReport only stores `framework: String` without a standardized naming convention.
- **Score Thresholds Hardcoded:** RiskLevel thresholds (90, 70, 50) are hardcoded in `report.rs:22-27`.

### Constants
- **Missing WAF Constants Documentation:** Document lists `BLOCKED_PATTERNS` array constant but doesn't mention `WEAK_BLOCK_INDICATOR_PATTERNS` and `UNKNOWN_WAF_WEAK_PATTERN_THRESHOLD`. Add documentation for these constants.

### Container
- **Node/Namespace Count Always None:** `ClusterInfo::node_count` and `namespace_count` are always `None` despite being part of the struct definition.

### Diff
- **Documentation Says JSONL but Code Uses JSON:** Document states `load_findings_from_file()` loads from "JSONL file" but code uses `serde_json::from_str` which parses standard JSON. Update architecture/diff.md to say "JSON file" instead of "JSONL file".
- **Evidence Change Detection:** When finding severity changes, old/new evidence content is not stored in `FindingChange`. Store old and new evidence content in `FindingChange` struct.
- **Fingerprint Collision Possible:** Diff logic uses `fingerprint` as key; hash collisions could cause findings to be lost. Use `HashMap<Fingerprint, Vec<Vec<Finding>>>` to handle collisions properly.
- **format_diff_text() Not Documented:** `format_diff_text()` function at line 110 is not mentioned in architecture document. Document `format_diff_text()` utility function in architecture/diff.md.
- **Uses std HashMap Instead of FxHashMap:** Module imports `std::collections::HashMap` rather than `rustc_hash::FxHashMap`. Replace with `FxHashMap` for better performance with large finding sets.

### Distributed
- **CommandExecutor Env Var Security Measure Not Documented:** `CommandExecutor::execute()` method (command.rs:162-171) explicitly rejects custom environment variables with a security comment, but this deliberate security measure is not documented in the architecture.
- **Worker Capabilities Documentation Gap:** Worker registration sends all `CAPABILITIES` (mod.rs:83-91) as string slices, but this detail is not documented in architecture.

### Error
- **Documentation Line Number Stale:** Line number reference in documentation for `std::io::Error` variant should be updated from `mod.rs:56` to `mod.rs:82`.

### Findings
- **EvidenceKind Display Names Not Human-Readable:** The `EvidenceKind` enum Display impl uses underscores (e.g., "http_request" instead of "HTTP Request").
- **JSONL Format Limitations:** FindingStore rewrites entire file on updates (`store.rs:138-145`). For large finding sets, this could be slow.

### Fuzzer
- **Missing PayloadType Variant Catch-All:** The `get_payloads` match at `payloads/mod.rs:152-185` could benefit from a catch-all for unknown types.
- **AdaptiveRateLimiter Not Integrated:** `AdaptiveRateLimiter` is defined but not visibly integrated into main fuzzing loop.

### Generated
- **Protobuf Package/Namespace Not Documented:** Architecture doc doesn't specify the protobuf package/namespace (slapper.tool.v1) or purpose of generated types. Update architecture/generated.md to document the protobuf package namespace and purpose.
- **Regeneration Process Not Documented:** No documentation on how to regenerate the protobuf code. Update architecture/generated.md to document how to regenerate (protoc command or build.rs process).

### Hunt
- **Potential TOCTOU in AttackChain Step Counting:** At `hunt/mod.rs:44`, `total_findings += chain.steps.len()` could be inconsistent if `chain.steps` is modified between the `len()` call and the `push()` call at line 45.
- **No Aggregation of Concurrent Results:** Results are processed sequentially after each check completes rather than collecting all concurrent tasks and processing together.
- **Empty Report Handling:** If all checks return empty vectors, the report will have `total_findings: 0`. This is valid but could be confusing.
- **Unbounded Vector Growth:** Each sub-module returns a `Vec` which is appended to the report. For targets with many findings, this could lead to significant memory usage without limits.
- **No Priority Ordering:** Findings are processed in a fixed order (attack_chains first, then business_logic, etc.) rather than by severity or exploitability.

### Integrations
- **No Error Handling Strategy Documented:** The trait returns `Result<String>` for `create_issue` and `Result<()>` for other methods, but there's no documentation on error cases or retry logic. Add documentation to the trait methods explaining error cases and consider adding retry logic for transient failures.

### Loadtest
- **Rate Limit Semaphore Comment Misleading:** The comment "Semaphore starts with `rate` permits, preventing initial burst" at line 272 is slightly misleading. It actually starts with `rate` permits which allows `rate` requests through immediately before the first interval tick.
- **set_common_with_config Auth Gap:** `set_common_with_config()` method at `runner.rs:149-170` properly merges config settings but does not call `apply_auth_headers()` with merged config's auth if `common.auth` is `None`.

### Macros
- **Confusing Macro Signature Documentation:** The macro signatures in the documentation show simplified syntax that doesn't fully capture the actual macro patterns. Improve documentation format for `run_if_enabled!` macro signature.
- **Missing run_if_enabled! Return Type Documentation:** Documentation doesn't mention that `run_if_enabled!` returns `Option<...>` and uses `$crate::recon::set_stage`. Document these details.

### Networking
- **CaptureBuilder Missing `#[must_use]`:** `CaptureBuilder::build()` at `capture.rs:501` clones config into `PacketCapture::new(self.config)` but doesn't use `#[must_use]` attribute on the builder.
- **parse_impl.rs Documentation Clarity:** Document references `parse_impl.rs` for DNS/TLS/HTTP parsing, but these implementations are actually in `types.rs`. `parse_impl.rs` contains the `ParsedPacket::parse()` orchestration method.

### NSE
- **LazyLock Initialization Contention:** `WAF_SIGNATURES` LazyLock may have thread contention during first access in multi-threaded context.

### Notify
- **Duplicate Payload Construction:** `notify_scan_complete()` method constructs the same `NotificationPayload` multiple times for webhooks, Slack, Discord, and Teams. Construct payload once and clone for each platform.
- **No Timeout on Webhook Requests:** `create_http_client(10)` has 10s timeout but individual webhook requests don't have explicit timeouts and could hang. Add explicit timeout wrappers to individual webhook requests.
- **RateLimited Event Never Dispatched:** `WebhookEvent::RateLimited` variant exists but `NotifyManager` has no `notify_rate_limited()` method. Add `notify_rate_limited()` method to `NotifyManager` or remove unused variant.
- **Teams Webhook Not Integrated:** `NotifyManager` stores `teams_webhook: Option<String>` but has no `notify_teams()` method call, unlike Slack and Discord. Add `notify_teams()` call in dispatch methods or remove unused `teams_webhook` field.

### Output
- **HashMap Migration Incomplete in report_summary.rs:** `report_summary.rs` uses `std::collections::HashMap` for `by_severity`, `by_confidence`, `by_type`, and `asset_counts` instead of FxHashMap.
- **Feature-Gated Exports Not Clearly Documented:** `AttackGraphBuilder::to_html()` is only available with `advanced-hunting` feature, but documentation doesn't clearly indicate this.
- **CSV Formula Injection Test Coverage Incomplete:** Test at `output/escape.rs:42-49` verifies fullwidth character bypasses, but no test for primary formula injection vectors (=, +, -, @).
- **RunManifest Excludes Non-Interesting Endpoints:** `populate_findings_from_report()` only creates findings for `interesting` endpoints; non-interesting are excluded.

### Overview
- **Command Count Imprecision:** `overview.md:156` says "37+" and `cli_commands.md:9` says "35+" but actual is ~29 without features, ~40 with all. Clarify base command count vs. feature-gated count in documentation.
- **Test Count May Be Stale:** `overview.md:581` claims "1324 base, 1469+ with full features" but counts may have changed. Verify with actual test run and update if necessary.

### Pipeline
- **DEFAULT_ENDPOINTS Static Array (Binary Size):** At `scanner/endpoints.rs:34`, static array means all 261 endpoints always compiled into binary. Consider making lazy-loaded from config file for binary size optimization.

### Probe
- **ServiceValidation Serialization Format:** The architecture doc mentions ProbeRisk has 6 variants but doesn't document that ProbeIntent::ServiceValidation serializes as "service-validation". Add documentation noting that ProbeIntent::ServiceValidation serializes as "service-validation" (kebab-case).

### Proxy
- **ProxyRotator Callback Design Not Documented:** `ProxyRotator::select_with_stats()` at `rotator.rs:40-56` takes a closure for stats callback, but documentation doesn't mention this callback-based design.

### Scanner
- **Static DEFAULT_ENDPOINTS Array:** All 261 endpoints are always compiled into binary even if unused.
- **No Endpoint Deduplication:** Custom wordlist may overlap with DEFAULT_ENDPOINTS causing duplicate scans.

### Storage
- **queries.rs Unused:** The `QueryBuilder` struct in `queries.rs` exists but is never used by the Database struct.
- **Missing Connection Pooling Configuration:** `StorageConfig` has `max_connections: u32` field but `Database::new()` ignores it since it's a stub.
- **No Transaction Support:** The Database stub doesn't have transaction methods (begin, commit, rollback).

### Stress
- **ESSID Parsing Fragile:** ESSID parsing could behave unexpectedly with inputs like `ESSID:"test" with extra`. Improve parsing robustness to handle edge cases in iwlist output format.
- **Limited Vulnerability Analysis Scope:** `analyze_networks()` only generates vulnerabilities for Open, WEP, and WPA networks; does not flag WPA2 Enterprise or other weak configurations. Expand detection to include WPA2 Enterprise and other potentially weak configurations.
- **Simple Parser May Miss Networks:** `parse_scan_output()` is a simple line-by-line parser that doesn't handle all possible iwlist output formats. Add more robust parsing to handle variant iwlist output formats across system versions.
- **Platform Dependency Not Documented:** Module uses `iwlist` which is Linux-specific but is not explicitly documented as Linux-only. Add explicit documentation that wireless scanning requires Linux with iwlist tool.
- **Signal Strength Fallback May Hide Errors:** Signal parsing uses `unwrap_or(-100)` which could mask parsing errors. Consider logging parsing failures instead of silently defaulting.

### Supply Chain
- **walkdir Performance on Large Repos:** Repo scanning uses `walkdir::WalkDir` without filtering by file size or depth limit, could be slow on large repositories.

### TUI
- **Tab::all() Ordering Inconsistent with Enum Discriminants:** When NSE feature is enabled, Nse tab is appended to end of `all()` but has enum discriminant 17. This causes `from_discriminant(17)` to return Nse but `all()[17]` to return Settings.
- **Bug Patterns Section May Be Outdated:** "Bug Patterns to Avoid" section (lines 327-884) documents patterns that may now be enforced via lints or already fixed.
- **Feature-Gated Tab Count Not Documented:** Document says "28 tabs" but this only applies when all 8 conditional features are enabled. Base count is 20 tabs.

### Types
- **Missing FromStr impl Documentation for Severity:** The document doesn't mention `std::str::FromStr` impl for Severity that accepts "moderate" as alias for Medium. Add documentation for the `FromStr` implementation that maps "moderate" → Medium.
- **Missing check_config_file_permissions Behavior Description:** The document mentions `check_config_file_permissions` but doesn't describe its behavior. Document that the function warns on world-readable or group-readable permissions.

### Vuln
- **assess_asset() Uses Exact String Matching:** Line 61-67 does exact string matching ("database", "web_server", etc.). Typos or case differences use default scoring.

### WAF
- **LazyLock Without Refresh Mechanism:** `WAF_SIGNATURES` and `WAF_PROFILES` LazyLocks cannot be refreshed at runtime.

### WebSocket
- **Consider Adding ws-api to full Feature:** The `ws-api` feature provides WebSocket support and is functionally complete, but is not included in the `full` feature. Consider adding `ws-api` to the `full` feature for completeness, or document why it is intentionally excluded.

### Workflow
- **SLA Calculation Ignores Resolved/False-Positive/Verified Findings:** `crates/slapper/src/workflow/mod.rs:38-48` only checks `FindingStatus::Open` for SLA violations. Resolved findings that were open but now resolved still had SLA violations that went untracked.
- **StatusWorkflow::can_transition() Missing FalsePositive Transitions:** The allowed transitions at `status.rs:7-18` don't include FalsePositive as a valid target from any state. A finding marked as false positive cannot be re-opened or transitioned to any other state.
- **WorkflowReport::calculate_metrics() Iterates Findings Twice:** Lines 38-47 filter for `FindingStatus::Open` then filter again for SLA violation. This is O(2n).

---

## Key Module Locations

| Module | Location |
|--------|----------|
| Agent | `crates/slapper/src/agent/` |
| AI | `crates/slapper/src/ai/` |
| Auth | `crates/slapper/src/auth/` |
| Browser | `crates/slapper/src/browser/` |
| Config | `crates/slapper/src/config/` |
| Container | `crates/slapper/src/container/` |
| Distributed | `crates/slapper/src/distributed/` |
| Findings | `crates/slapper/src/findings/` |
| Fuzzer | `crates/slapper/src/fuzzer/` |
| Hunt | `crates/slapper/src/hunt/` |
| Loadtest | `crates/slapper/src/loadtest/` |
| Networking | `crates/slapper/src/networking/` |
| NSE | `slapper-nse/` |
| Output | `crates/slapper/src/output/` |
| Pipeline | `crates/slapper/src/pipeline/` |
| Proxy | `crates/slapper/src/proxy/` |
| Recon | `crates/slapper/src/recon/` |
| Scanner | `crates/slapper/src/scanner/` |
| Storage | `crates/slapper/src/storage/` |
| Stress | `crates/slapper/src/stress/` |
| Supply Chain | `crates/slapper/src/supply_chain/` |
| TUI | `crates/slapper/src/tui/` |
| Vuln | `crates/slapper/src/vuln/` |
| WAF | `crates/slapper/src/waf/` |
| Workflow | `crates/slapper/src/workflow/` |

---

## Wave Structure (For Parallel Implementation)

### Wave 1: Critical Security Issues (Parallel: 2 agents)
**Focus:** Security risks and potential crashes

| Agent A | Agent B |
|---------|---------|
| Container Docker Shell Injection fix | Loadtest Semaphore unwrap panic fix |
| Networking PcapWriter silent drop fix | NSE Sandbox integration tests |
| - | Storage stub documentation clarification |

### Wave 2: Documentation Accuracy (Parallel: 2-3 agents)
**Focus:** Fix documentation mismatches and missing documentation

| Agent A | Agent B | Agent C |
|---------|---------|---------|
| Pipeline defense-lab stage counts | Output format count (8→7/8) | StressConfig field names correction |
| Pipeline CSV NFKC escape | Defense Lab RunManifest verification | - |
| Missing defense-lab profiles table | - | - |

### Wave 3: Error Handling Improvements (Parallel: 2 agents)
**Focus:** Address silent error suppression patterns across modules

| Agent A | Agent B |
|---------|---------|
| Notify `let _` pattern replacements | Kubernetes `.ok()` error logging |
| Scanner endpoints silent suppression | WebSocket close() error handling |
| Findings store deduplication | - |

### Wave 4: Type & Performance Issues (Parallel: 2 agents)
**Focus:** Fix type mismatches and performance concerns

| Agent A | Agent B |
|---------|---------|
| FxHashMap migrations (NSE CveCache, output report_summary, diff) | VulnAssessment struct redesign |
| Non-cryptographic fingerprint in Findings | DEFAULT_ENDPOINTS lazy loading consideration |
| Supply Chain SBOM CVE lookup decision | - |

### Wave 5: Feature Completeness (Parallel: 3 agents)
**Focus:** Complete incomplete implementations

| Agent A | Agent B | Agent C |
|---------|---------|---------|
| Auth run_full_test() all 8 test types | Browser XSS payload parameterization | Notify retry logic and Teams integration |
| Notify `let _` pattern replacements | Browser SPA route handling | IssueTracker async trait conversion |
| Compliance framework implementations | - | - |

### Wave 6: Low Priority Improvements (Ongoing - Single agent or distributed)
Items that don't block functionality but improve robustness:
- Browser SPA route handling expansion
- CVSS vector parsing validation
- SLA calculation for resolved findings
- ESSID parsing robustness
- CIS benchmark check improvements
- Various documentation updates

---

## Notes

- Items were consolidated from multiple review passes (10 batches across 43 architecture review files)
- Duplicates were merged; higher-detail version retained
- Priority designations preserved from original classifications
- Wave groupings suggest approximate parallelization; actual timing may vary based on agent availability
- **Verified items:** Docker shell injection, StressConfig field names, Output format count confirmed as documentation errors
- **Corrected items:** Recon module count (verified as 17, not 18 - documentation was correct)