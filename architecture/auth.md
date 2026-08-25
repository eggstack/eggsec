# Auth Module — Deep Dive

## Role & Responsibilities

The `auth` module provides authentication security testing: brute force resistance, credential stuffing, account lockout detection, MFA bypass testing, rate-limit analysis, session management validation, timing side-channel detection, and password policy evaluation. It also includes a multi-protocol sub-module (SSH/FTP/SMTP) gated behind the `nse-ssh2` feature.

This is a **standalone defense-lab surface** — it emits local `AuthTestReport`/`AuthFinding` types directly and is intentionally not part of the main assessment pipeline or unified reporting system.

## Location & Feature Gating

| Aspect | Detail |
|--------|--------|
| Root module | `crates/eggsec/src/auth/mod.rs` |
| Cargo feature | None — always compiled (`auth` has no dedicated feature flag) |
| Runtime gate | `OperationRisk::CredentialTesting` via `EnforcementContext::evaluate()` (default blocked by policy) |
| Multi-protocol gate | `#[cfg(feature = "nse-ssh2")]` at `auth/mod.rs:15` |
| Related module | `auth_context/` (credential injection for other modules; does not call `auth/` testers) |

## Architecture

### File Inventory (13 files)

| File | Lines | Types Defined |
|------|-------|---------------|
| `auth/mod.rs` | 309 | `AuthEngine`, `AuthTestReport`, `AuthTestType`, `AuthFinding`, `AUTH_BANNER` |
| `auth/brute_force.rs` | 181 | `BruteForceTester`, `BruteForceResult`, `WeakCredential` |
| `auth/credential_stuffing.rs` | 201 | `CredentialStuffer`, `CredentialStuffingResult`, `CompromisedAccount`, `CredentialPair` |
| `auth/lockout.rs` | 217 | `LockoutDetector`, `LockoutDetectionResult`, `LockoutType` |
| `auth/mfa.rs` | 170 | `MfaTester`, `MfaTestResult`, `MfaBypassMethod` |
| `auth/password_policy.rs` | 229 | `PasswordPolicyTester`, `PasswordPolicyResult` |
| `auth/rate_limit.rs` | 162 | `RateLimitTester`, `RateLimitResult`, `RateLimitBypassResult` |
| `auth/session.rs` | 95 | `SessionTester`, `SessionTestResult` |
| `auth/timing.rs` | 145 | `TimingTester`, `TimingTestResult`, `TimingMeasurement` |
| `auth/multi_protocol.rs` | 92 | `ProtocolAuthTester`, `AuthTestResult`, `MULTI_PROTOCOL_AUTH_BANNER` |
| `auth/multi_protocol/ssh.rs` | 144 | `test_ssh_auth()`, `check_ssh_banner()` |
| `auth/multi_protocol/ftp.rs` | 169 | `test_ftp_auth()`, `check_ftp_banner()` |
| `auth/multi_protocol/smtp.rs` | 233 | `test_smtp_auth()`, `check_smtp_banner()`, `test_login_auth()`, `test_plain_auth()` |

### Type Table

| Type | File:Line | Role |
|------|-----------|------|
| `AuthEngine` | `mod.rs:71` | Core orchestrator: max attempts, concurrency, timeout, stop flag, attempt counter, wordlists |
| `AuthTestReport` | `mod.rs:33` | Aggregated report: 8 optional result slots + findings + total attempts |
| `AuthTestType` | `mod.rs:49` | Enum of 8 test categories |
| `AuthFinding` | `mod.rs:61` | Individual finding with test_type, severity, title, description, recommendation |
| `BruteForceTester` | `brute_force.rs:24` | Wraps `AuthEngine` + `reqwest::Client`; tests password lists against a single username |
| `CredentialStuffer` | `credential_stuffing.rs:30` | Wraps `AuthEngine` + `reqwest::Client`; tests username:password pairs from breach lists |
| `LockoutDetector` | `lockout.rs:24` | Sends repeated wrong passwords; classifies lockout type from response changes |
| `MfaTester` | `mfa.rs:22` | Detects MFA presence; tests bypass via weak code (`000000`) and skip parameter |
| `PasswordPolicyTester` | `password_policy.rs:18` | Parses login page for policy text; probes with weak passwords |
| `RateLimitTester` | `rate_limit.rs:22` | Sends 50 rapid requests; tests X-Forwarded-For and similar header bypasses |
| `SessionTester` | `session.rs:14` | Sends 2 GET requests; compares Set-Cookie for fixation; checks HttpOnly/Secure/SameSite |
| `TimingTester` | `timing.rs:21` | Measures response times across 7 password inputs (10 samples each); flags >50ms variance |
| `ProtocolAuthTester` | `multi_protocol.rs:27` | Multi-protocol facade: delegates to ssh/ftp/smtp sub-modules |
| `LockoutType` | `lockout.rs:16` | Enum: `HardLockout`, `SoftLockout`, `ProgressiveDelay`, `Captcha`, `None` |

### Enum Variant Counts

| Enum | Variants | File:Line |
|------|----------|-----------|
| `AuthTestType` | 8 | `mod.rs:49-58` |
| `LockoutType` | 5 | `lockout.rs:16-22` |

## Behavior & Flow

### AuthEngine Orchestration

`AuthEngine::run_full_test()` (`mod.rs:124-223`) executes all 8 test types sequentially:

1. **BruteForce** — Uses first username from wordlist + all passwords
2. **CredentialStuffing** — Cartesian product of usernames × passwords as `CredentialPair`s
3. **AccountLockout** — Uses first username (or `"admin"` fallback at `mod.rs:182`)
4. **RateLimitBypass** — Standalone, no credentials needed
5. **MfaBypass** — Standalone
6. **SessionFixation** — Standalone
7. **TimingAttack** — Standalone
8. **PasswordPolicy** — Standalone

Each tester is wrapped in `if let Ok(result) = ...` — errors are silently swallowed (testers are optional; missing results become `None` in the report).

`AuthEngine` uses `AtomicBool` stop flag + `AtomicUsize` attempt counter (`mod.rs:78-79`) for thread-safe coordination. `increment_attempts()` (`mod.rs:106-114`) returns `false` when max attempts reached or stop flag set.

### Brute Force Strategies

`BruteForceTester::test()` (`brute_force.rs:36-115`):

- Sends `POST` with `application/x-www-form-urlencoded` body (`username={}&password={}`)
- **Success detection** (`brute_force.rs:75-85`): HTTP 200/302 AND body does NOT contain: `"invalid"`, `"incorrect"`, `"wrong"`, `"failed"`, `"denied"`, `class="error"`, `class='error'`, `id="error"`, `id='error'`
- **Response analysis** (`brute_force.rs:117-132`): Checks for `"welcome"`, `"dashboard"`, `"session"`, `"token"`, `"jwt"` indicators
- **Lockout detection** (`brute_force.rs:98-106`): Checks `STATUS_LOCKED` (423), `"locked"`, `"too many attempts"` — triggers `engine.stop()` if `stop_on_lockout` is true
- **Rate-limit detection** (`brute_force.rs:95-97`): Checks `STATUS_RATE_LIMITED` (429)

### Credential Stuffing Wordlist Handling

`CredentialStuffer::test()` (`credential_stuffing.rs:42-115`):

- Sends `POST` with JSON body (`{"username": ..., "password": ...}`)
- Success detection (`credential_stuffing.rs:82-84`): HTTP 302 OR (HTTP 200 AND body lacks `"invalid"` and `"error"`)
- `load_breach_list()` (`credential_stuffing.rs:131-146`): Parses `username:password` format (split on first `:`), trims whitespace
- Same lockout/rate-limit detection as brute force

### Lockout Detection Heuristics

`LockoutDetector::detect()` (`lockout.rs:34-96`):

- Sends up to `max_attempts` wrong passwords
- Detects lockout when HTTP status **changes** from previous attempt OR body matches `is_lockout_response()` (`lockout.rs:98-105`): `"too many attempts"`, `"account locked"`, `"try again later"`, `"rate limit"`, `"captcha"`
- **Classification** (`lockout.rs:107-127`):
  - `HardLockout`: Status 423 OR body contains `"locked"`
  - `SoftLockout`: Status 429
  - `Captcha`: Body contains `"captcha"`
  - `ProgressiveDelay`: Body contains `"try again"` or `"wait"`
  - `None`: No lockout indicators
- Connection failure → `HardLockout` (`lockout.rs:83-91`)

### MFA Bypass Vectors

`MfaTester::test()` (`mfa.rs:32-51`):

1. **MFA detection** (`mfa.rs:53-86`): POSTs `admin/admin` credentials; checks response body for `"two-factor"`, `"mfa"`, `"verification code"`, `"authenticator"`, `"totp"`, `"2fa"`, `"enter code"`, `"enter token"`
2. **Bypass testing** (`mfa.rs:88-136`):
   - **Weak MFA code**: Sends `mfa_code=000000` — if 200/302 returned, reports Critical finding
   - **MFA skip parameter**: Sends `mfa_skip=true` — if 200/302 returned, reports Critical finding

### Rate-Limit Analysis

`RateLimitTester::test()` (`rate_limit.rs:32-84`):

- Sends up to 50 rapid POST requests with wrong passwords
- Detects rate limiting via `STATUS_RATE_LIMITED` (429)
- Extracts `X-RateLimit-Limit` or `RateLimit-Limit` header values (`rate_limit.rs:60-69`)
- **Bypass techniques** (`rate_limit.rs:86-127`): Tests 5 X-Forwarded-For-style headers: `X-Forwarded-For`, `X-Real-IP`, `X-Client-IP`, `X-Originating-IP`, `X-Remote-IP` (all set to `1.1.1.1`)

### Session Tests

`SessionTester::test()` (`session.rs:24-71`):

- Sends 2 sequential GET requests to target
- **Fixation detection** (`session.rs:40-46`): Compares `Set-Cookie` headers — identical values across requests indicate fixation
- **Cookie flag analysis** (`session.rs:49-67`): Checks first `Set-Cookie` for `HttpOnly`, `Secure`, `SameSite` attributes

### Timing Attack Statistics

`TimingTester::test()` (`timing.rs:31-75`):

- Tests 7 password inputs of varying length: `"a"`, `"aa"`, `"aaa"`, `"aaaa"`, `"aaaaa"`, `"wrong"`, `"wrongpassword"`
- Each input measured with 10 samples (`timing.rs:43-48`)
- `measure_timing()` (`timing.rs:77-112`): Records `Instant::now()` before/after each POST; averages successful request times only
- **Vulnerability threshold** (`timing.rs:60`): `diff > 50.0ms` between max and min average response times
- Analysis string reports max, min, and variance

### Multi-Protocol Testing (gated `nse-ssh2`)

`ProtocolAuthTester` (`multi_protocol.rs:27-71`) provides:

- **SSH** (`multi_protocol/ssh.rs`): Uses `ssh2::Session` for password auth (`ssh_auth_attempt` at `:80-98`); reads SSH banner via `check_ssh_banner()` (`:101-123`)
- **FTP** (`multi_protocol/ftp.rs`): Raw TCP with `USER`/`PASS` commands; checks for `230` response (`ftp_auth_attempt` at `:80-124`); banner check via `check_ftp_banner()` (`:126-148`)
- **SMTP** (`multi_protocol/smtp.rs`): EHLO negotiation → AUTH LOGIN (base64) or AUTH PLAIN (`smtp_auth_attempt` at `:81-123`); checks for `235` response; banner check via `check_smtp_banner()` (`:190-212`)

All multi-protocol functions use `tokio::time::timeout()` wrapping blocking `std::net::TcpStream` I/O.

## Public API

| Function/Method | File:Line | Signature |
|-----------------|-----------|-----------|
| `AuthEngine::new()` | `mod.rs:83` | `(max_attempts, concurrency, timeout_secs, stop_on_lockout) -> Result<Self>` |
| `AuthEngine::load_wordlists()` | `mod.rs:101` | `(&mut self, usernames: Vec<String>, passwords: Vec<String>)` |
| `AuthEngine::increment_attempts()` | `mod.rs:106` | `(&self) -> bool` |
| `AuthEngine::should_stop()` | `mod.rs:116` | `(&self) -> bool` |
| `AuthEngine::stop()` | `mod.rs:120` | `(&self)` |
| `AuthEngine::run_full_test()` | `mod.rs:124` | `(&self, target: &str) -> Result<AuthTestReport>` |
| `BruteForceTester::new()` | `brute_force.rs:30` | `(max_attempts, concurrency, timeout_secs) -> Result<Self>` |
| `BruteForceTester::test()` | `brute_force.rs:36` | `(&self, target, username, passwords) -> Result<BruteForceResult>` |
| `CredentialStuffer::new()` | `credential_stuffing.rs:36` | `(max_attempts, concurrency, timeout_secs) -> Result<Self>` |
| `CredentialStuffer::test()` | `credential_stuffing.rs:42` | `(&self, target, credentials) -> Result<CredentialStuffingResult>` |
| `CredentialStuffer::load_breach_list()` | `credential_stuffing.rs:131` | `(&self, path: &str) -> Result<Vec<CredentialPair>>` |
| `LockoutDetector::new()` | `lockout.rs:29` | `(timeout_secs) -> Result<Self>` |
| `LockoutDetector::detect()` | `lockout.rs:34` | `(&self, target, username, max_attempts) -> Result<LockoutDetectionResult>` |
| `MfaTester::new()` | `mfa.rs:27` | `(timeout_secs) -> Result<Self>` |
| `MfaTester::test()` | `mfa.rs:32` | `(&self, target) -> Result<MfaTestResult>` |
| `PasswordPolicyTester::new()` | `password_policy.rs:23` | `(timeout_secs) -> Result<Self>` |
| `PasswordPolicyTester::test()` | `password_policy.rs:28` | `(&self, target) -> Result<PasswordPolicyResult>` |
| `RateLimitTester::new()` | `rate_limit.rs:27` | `(timeout_secs) -> Result<Self>` |
| `RateLimitTester::test()` | `rate_limit.rs:32` | `(&self, target) -> Result<RateLimitResult>` |
| `SessionTester::new()` | `session.rs:19` | `(timeout_secs) -> Result<Self>` |
| `SessionTester::test()` | `session.rs:24` | `(&self, target) -> Result<SessionTestResult>` |
| `TimingTester::new()` | `timing.rs:26` | `(timeout_secs) -> Result<Self>` |
| `TimingTester::test()` | `timing.rs:31` | `(&self, target) -> Result<TimingTestResult>` |
| `ProtocolAuthTester::new()` | `multi_protocol.rs:33` | `(timeout_secs) -> Result<Self>` |
| `ProtocolAuthTester::test_ssh()` | `multi_protocol.rs:45` | `(&self, target, port, credentials) -> Result<Vec<AuthTestResult>>` |
| `ProtocolAuthTester::test_ftp()` | `multi_protocol.rs:54` | `(&self, target, port, credentials) -> Result<Vec<AuthTestResult>>` |
| `ProtocolAuthTester::test_smtp()` | `multi_protocol.rs:63` | `(&self, target, port, credentials) -> Result<Vec<AuthTestResult>>` |

## Integration Points

### CLI

- **Command**: `eggsec auth-test <target>` — handler at `commands/handlers/auth_test.rs:9`
- **CLI args**: `AuthTestArgs` defined in `cli/auth.rs`
- **Policy gate**: `evaluate_and_enforce_operation()` with `OperationRisk::CredentialTesting` (`auth_test.rs:13-27`)
- **Banner**: `AUTH_BANNER` printed to stderr before tests (`auth_test.rs:29`)
- **Selective dispatch**: Handler checks `args.all` or individual flags (`args.brute_force`, `args.rate_limit_bypass`, etc.) to select which testers to run
- **Wordlist loading**: `load_passwords()` (`auth_test.rs:290-317`) with path traversal protection; falls back to 10 common passwords

### Dispatch

- `TaskKind::AuthTest(AuthTestParams)` dispatched at `dispatch/mod.rs:240-251`
- Calls `auth::run_auth_task()` with max_attempts=100, concurrency=1, timeout=30

### Pipeline

- `ScanProfile::Auth` exists but does **not** invoke `auth/` module testers — it runs PortScan + Fingerprint + EndpointScan + Fuzz (for JWT/OAuth/IDOR) (`pipeline/stage.rs:98-103`)
- The `auth/` module is entirely standalone

### Auth Context

- `auth_context/` is a separate module for credential injection (headers/cookies from YAML); it does **not** call `auth/` module testers
- `auth_context` is consumed by the fuzzer engine (`fuzzer/engine/core.rs`) and HTTP utilities

## Safety

- **Credential-testing risk tier**: `OperationRisk::CredentialTesting` — default blocked in non-interactive surfaces
- **Scope requirement**: Target must be in loaded scope for `EnforcementContext::evaluate()` to approve
- **Stop-on-lockout**: `AuthEngine.stop_on_lockout` (default `true` in testers) halts when lockout detected
- **Max attempts**: Hard cap via `AuthEngine.max_attempts` + `AtomicUsize` counter
- **AUTH_BANNER**: Printed to stderr for every `auth-test` invocation (`mod.rs:226-234`)
- **Path traversal protection**: `load_passwords()` rejects `..` in wordlist paths (`auth_test.rs:292-293`)
- **No credential storage**: Credentials are held in memory only; not persisted

## Testing

Each sub-module has `#[cfg(test)] mod tests` with structural/unit tests:

| File | Test Count | Coverage |
|------|------------|----------|
| `mod.rs` | 7 | Engine creation, stop flag, attempt counter, wordlists, finding creation, test type variants, banner |
| `brute_force.rs` | 4 | Result default, weak credential creation, analyze_response indicators (positive/negative) |
| `credential_stuffing.rs` | 5 | Result default, compromised account, credential pair, nonexistent file error, analyze indicators |
| `lockout.rs` | 7 | Detector creation, type variants, lockout response positive/negative, classify hard/soft/captcha, result creation |
| `mfa.rs` | 3 | Tester creation, result default, bypass method creation |
| `password_policy.rs` | 6 | Result defaults (3 variants), serialization roundtrip, tester creation, various timeouts |
| `rate_limit.rs` | 3 | Result default, bypass result creation, tester creation |
| `session.rs` | 2 | Tester creation, result default |
| `timing.rs` | 3 | Tester creation, result default, measurement creation |
| `multi_protocol.rs` | 1 | ProtocolAuthTester creation |

Handler-level tests in `commands/handlers/` plus 17 wiremock `auth_tests` + enforcement/policy contract tests.

## Invariants & Gotchas

1. **No Cargo feature gate**: Auth is always compiled; safety is enforced at runtime by `EnforcementContext` + `CredentialTesting` risk only.
2. **Local findings only**: `AuthTestReport`/`AuthFinding` are not converted to `StoredFinding` or `eggsec-output` canonical types. No `to_scan_report_data()` bridge exists.
3. **`ScanProfile::Auth` ≠ auth module**: The pipeline Auth profile runs fuzzing stages (JWT/OAuth/IDOR), not the `auth/` testers.
4. **Blocking I/O in multi-protocol**: `multi_protocol/ssh.rs`, `ftp.rs`, `smtp.rs` use `std::net::TcpStream` (blocking) within async functions. `tokio::time::timeout` wraps these but cannot interrupt a blocking TCP connect — the executor thread may stall during connection attempts.
5. **`unwrap_or_default()` on `resp.text().await`**: Used in brute_force.rs:72, credential_stuffing.rs:80, lockout.rs:66, mfa.rs:68, password_policy.rs:44,96. Acceptable — these unwrap already-completed HTTP response bodies, not hanging async operations.
6. **Credential stuffing success detection is narrower than brute force**: credential_stuffing.rs:82-84 checks only for `"invalid"` and `"error"`, while brute_force.rs:75-85 additionally checks `"incorrect"`, `"wrong"`, `"failed"`, `"denied"`, and error CSS classes.
7. **`password_policy.rs:69` uses `.expect("valid regex pattern")`**: Acceptable — compile-time-known regex; panics only on bug in the regex literal.
8. **`auth_engine.rs:171` uses `.unwrap_or_default()` on `Vec<CredentialPair>`**: This is a sync `Option::unwrap_or_default()`, not async — safe.
9. **Timing tester uses `std::time::Instant`** (`timing.rs:88`): Fine for wall-clock measurement but susceptible to NTP adjustments; 50ms threshold is conservative enough.

*Last verified against source: 2026-08-25*
