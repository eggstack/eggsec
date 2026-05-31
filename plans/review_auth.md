# Auth Architecture Review
**Document:** architecture/auth.md
**Reviewed:** 2026-05-31
**Accuracy:** Medium
**Lines Reviewed:** 42

## Verified Claims
- `AuthEngine` struct: Verified at `crates/slapper/src/auth/mod.rs:65` with fields `max_attempts`, `stop_on_lockout`, `concurrency`, `timeout_secs`, `username_list`, `password_list`, `stop_flag`, `attempt_counter`
- `AuthTestReport` struct: Verified at `crates/slapper/src/auth/mod.rs:28` with all documented fields
- `AuthTestType` enum: Verified at `crates/slapper/src/auth/mod.rs:43` with 8 variants (BruteForce, CredentialStuffing, AccountLockout, RateLimitBypass, MfaBypass, SessionFixation, TimingAttack, PasswordPolicy)
- `AuthFinding` struct: Verified at `crates/slapper/src/auth/mod.rs:54`
- `BruteForceTester` re-export: Verified at `crates/slapper/src/auth/mod.rs:19`
- `CredentialStuffer` re-export: Verified at `crates/slapper/src/auth/mod.rs:20`
- `LockoutDetector` re-export: Verified at `crates/slapper/src/auth/mod.rs:21`
- `MfaTester` re-export: Verified at `crates/slapper/src/auth/mod.rs:22`
- `RateLimitTester` re-export: Verified at `crates/slapper/src/auth/mod.rs:23`
- `SessionTester` re-export: Verified at `crates/slapper/src/auth/mod.rs:24`
- `TimingTester` re-export: Verified at `crates/slapper/src/auth/mod.rs:25`
- Safety mechanisms (`stop_on_lockout`, `max_attempts`, `stop_flag`): Verified at `crates/slapper/src/auth/mod.rs:67-72`
- Files: `mod.rs`, `brute_force.rs`, `credential_stuffing.rs`, `lockout.rs`, `mfa.rs`, `rate_limit.rs`, `session.rs`, `timing.rs` - all verified present

## Discrepancies
- **Multi-protocol file locations**: Documented as top-level files `ssh.rs`, `ftp.rs`, `smtp.rs` in the Files table. Actual location is `auth/multi_protocol/ssh.rs`, `auth/multi_protocol/ftp.rs`, `auth/multi_protocol/smtp.rs` (nested under `multi_protocol.rs` submodule)
- **`multi_protocol.rs` module declaration**: Documented as a standalone file in `auth/`. Actual: `multi_protocol.rs` exists but is **not declared as a `pub mod` in `auth/mod.rs`**, making it and its submodules (ssh, ftp, smtp) completely inaccessible from outside the `auth` crate module. The `auth/mod.rs` file only declares: `brute_force`, `credential_stuffing`, `lockout`, `mfa`, `rate_limit`, `session`, `timing` (`crates/slapper/src/auth/mod.rs:6-12`)

## Bugs Found
- **Dead code**: `multi_protocol.rs` and its submodules `ssh.rs`, `ftp.rs`, `smtp.rs` exist on disk but are never declared in `auth/mod.rs`, making them unreachable dead code. This appears to be an incomplete integration - the multi-protocol testing functionality was written but never wired into the module tree (`crates/slapper/src/auth/mod.rs:6-12`)

## Improvement Opportunities
- Add `pub mod multi_protocol;` to `auth/mod.rs` to expose the multi-protocol testing capabilities
- Add SSH, FTP, SMTP test types to `AuthTestType` enum to match the sub-module capabilities
- Wire multi-protocol tests into `AuthEngine::run_full_test()` which currently only runs rate_limit, timing, and session tests (`crates/slapper/src/auth/mod.rs:113-149`)

## Stale Items
- None
