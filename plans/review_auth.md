# Auth Module Architecture Review

**Document:** architecture/auth.md
**Reviewed:** 2026-06-02
**Accuracy:** High
**Lines Reviewed:** 42

## Verified Claims
- [AuthEngine]: Verified at `crates/slapper/src/auth/mod.rs:65`
- [AuthTestReport]: Verified at `crates/slapper/src/auth/mod.rs:28`
- [AuthTestType enum with 8 variants]: Verified at `crates/slapper/src/auth/mod.rs:42-52`
- [AuthFinding]: Verified at `crates/slapper/src/auth/mod.rs:55`
- [BruteForceTester, CredentialStuffer, LockoutDetector, MfaTester, RateLimitTester, SessionTester, TimingTester]: All re-exported at `crates/slapper/src/auth/mod.rs:19-25`
- [multi_protocol.rs with ssh.rs, ftp.rs, smtp.rs]: Verified at `crates/slapper/src/auth/multi_protocol.rs:6-8`
- [Safety mechanisms (stop_on_lockout, max_attempts, stop_flag)]: Verified at `crates/slapper/src/auth/mod.rs:66-74`

## Discrepancies
- None significant. All file listings and type locations are accurate.

## Bugs Found
- None found.

## Improvement Opportunities
- [Incomplete run_full_test()]: The `run_full_test()` method only runs 3 of 8 test types (RateLimitBypass, TimingAttack, SessionFixation). Missing: BruteForce, CredentialStuffing, AccountLockout, MfaBypass, PasswordPolicy. Consider documenting this partial implementation or completing it (priority: medium)
- [Hardcoded stop_on_lockout=true]: The `AuthEngine::new()` always sets `stop_on_lockout: true` (line 80), ignoring any parameter passed. This should be configurable (priority: low)

## Stale Items
- None.

## Code Interrogation Findings
- [Missing BruteForceTester usage]: `AuthEngine::run_full_test()` does not invoke `BruteForceTester` even though the struct is exported. The brute force module may be unused by the engine.
- [SSH/FTP/SMTP protocols require nse-ssh2 feature]: The multi_protocol.rs uses `ssh2::Session` which requires the `nse-ssh2` feature. If not enabled, these will fail to compile.
- [Missing protocol testers in engine]: The auth engine only orchestrates HTTP-based tests. Multi-protocol testers (ssh, ftp, smtp) exist but are not integrated into `run_full_test()`.