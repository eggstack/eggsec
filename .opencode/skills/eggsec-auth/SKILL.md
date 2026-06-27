---
name: eggsec-auth
description: "Authentication security testing - use when working with brute force testing, credential stuffing, lockout detection, MFA bypass, rate limiting, password policy, or session testing."
---

# Eggsec Auth Skill

Authentication security testing module.

## Module Location
`crates/eggsec/src/auth/`

## Tab
`AuthTab` is fully integrated as `Tab::Auth` in the TUI (TabSpec with Intrusive risk_group, direct_launch: true; TaskConfig::Auth + TaskResult::Auth in worker system; session save/restore; copy-CLI equivalent). Primary surface is CLI `auth-test`; local findings only (no `ScanReportData` bridge).

## Key Types

- `AuthEngine` - Main authentication testing engine
- `AuthTestReport`, `AuthFinding`, `AuthTestType` (8 variants: BruteForce, CredentialStuffing, Lockout, Mfa, RateLimit, PasswordPolicy, Session, Timing)
- `BruteForceTester` - Credential brute force testing
- `CredentialStuffer` - Breach credential testing
- `LockoutDetector` - Account lockout detection
- `MfaTester` - MFA bypass testing
- `RateLimitTester` - Rate limit testing
- `PasswordPolicyTester` - Password policy testing
- `SessionTester` - Session management testing
- `TimingTester` - Timing attack testing
- `ProtocolAuthTester` (multi-protocol under `nse-ssh2`): SSH/FTP/SMTP testers in `multi_protocol/`

## CLI Integration

- Handler: `commands/handlers/auth_test.rs` (selective tester dispatch via `AuthTestType`, wordlist loading, `AUTH_BANNER`)
- CLI args: `cli/auth.rs`
- Policy: `evaluate_and_enforce_operation(OperationDescriptor { risk: CredentialTesting, ... })` (central `EnforcementContext`; post-2026-06-10)
- No dedicated Cargo feature (runtime policy gate only)

## Patterns

### Brute Force Testing (via engine in handler)
```rust
let mut engine = AuthEngine::new();
let usernames = vec!["admin".to_string(), "root".to_string()];
let passwords = vec!["password".to_string(), "123456".to_string()];
engine.load_wordlists(usernames, passwords);
engine.set_target("https://example.com/login");
engine.run_brute_force().await?;
```

### Lockout Detection
```rust
let detector = LockoutDetector::new();
let is_locked = detector.detect_lockout(&response).await?;
```

## Key Files
- `mod.rs` - `AuthEngine`, `AuthTestReport`, `AuthFinding`, `AuthTestType`, `AUTH_BANNER`
- `brute_force.rs` - Brute force testing
- `credential_stuffing.rs` - Credential stuffing
- `lockout.rs` - Lockout detection
- `mfa.rs` - MFA bypass
- `rate_limit.rs` - Rate limit testing
- `password_policy.rs` - Password policy testing
- `session.rs` - Session management testing
- `timing.rs` - Timing attack testing
- `multi_protocol.rs` + `multi_protocol/{ssh,ftp,smtp}.rs` (gated on `nse-ssh2`)
- `commands/handlers/auth_test.rs` - CLI handler (selective dispatch, policy)
- `cli/auth.rs` - CLI arg definitions

## Module Notes
See `architecture/auth.md` for architecture documentation. TUI `AuthTab` is fully integrated as `Tab::Auth` (TabSpec, task system, policy enforcement, session save/restore). Policy enforcement uses central `EnforcementContext` + `CredentialTesting` risk tier (no feature flag). All 17 wiremock auth tests + enforcement/policy contract tests pass.

Local `AuthTestReport`/`AuthFinding` only (no conversion to `StoredFinding`/`ScanReportData`/`eggsec-output` canonical types per adopted model; handler produces JSON/text directly). `auth-test` is standalone defense-lab CLI (distinct from pipeline `ScanProfile::Auth` which is JWT/OAuth/IDOR fuzzer-focused via stages + fuzzer payloads).

See `docs/AUTH_LAB.md` for defense-lab usage, requirements (`allow_credential_testing=true` + explicit scope + dedicated test accounts), and command examples. Final auth polish + overall new-modules cleanup in `plans/final-cleanup-new-modules-plan.md` (Task 1 + resolution note).