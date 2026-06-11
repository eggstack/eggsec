# Auth Module

## Purpose

Authentication security testing module providing brute force, credential stuffing, lockout detection, MFA bypass testing, and multi-protocol authentication analysis with built-in safety mechanisms.

## Key Types

| Type | Location | Description |
|------|----------|-------------|
| `AuthEngine` | `auth/mod.rs` | Core engine managing test execution, concurrency, and stop conditions |
| `AuthTestReport` | `auth/mod.rs` | Aggregated report of all authentication tests |
| `AuthTestType` | `auth/mod.rs` | Enum of 8 test categories (BruteForce, CredentialStuffing, etc.) |
| `AuthFinding` | `auth/mod.rs` | Individual authentication finding with severity |
| `BruteForceTester` | `auth/brute_force.rs` | Brute force login testing |
| `CredentialStuffer` | `auth/credential_stuffing.rs` | Credential stuffing attack simulation |
| `LockoutDetector` | `auth/lockout.rs` | Account lockout policy detection |
| `MfaTester` | `auth/mfa.rs` | Multi-factor authentication bypass testing |
| `RateLimitTester` | `auth/rate_limit.rs` | Rate limiting detection and bypass |
| `SessionTester` | `auth/session.rs` | Session management security testing |
| `TimingTester` | `auth/timing.rs` | Timing-based user enumeration detection |

## Files

| File | Description |
|------|-------------|
| `mod.rs` | Module root: `AuthEngine`, `AuthTestReport`, `AuthFinding` types, engine methods |
| `brute_force.rs` | Brute force testing with configurable wordlists and concurrency |
| `credential_stuffing.rs` | Credential stuffing using leaked credential databases |
| `lockout.rs` | Account lockout detection and threshold identification |
| `mfa.rs` | MFA bypass testing (replay, fallback, brute force) |
| `rate_limit.rs` | Rate limit detection and bypass techniques |
| `session.rs` | Session fixation, token leakage, and session management flaws |
| `timing.rs` | Timing side-channel for user enumeration |
| `multi_protocol.rs` | Multi-protocol authentication testing (SSH, FTP, SMTP) |
| `multi_protocol/ssh.rs` | SSH-specific authentication testing |
| `multi_protocol/ftp.rs` | FTP-specific authentication testing |
| `multi_protocol/smtp.rs` | SMTP-specific authentication testing |

## CLI Surface

Primary surface is the `eggsec auth-test <target>` CLI command (defense-lab / high-risk credential control validation).

- CLI args: `crates/eggsec/src/cli/auth.rs`
- Handler: `crates/eggsec/src/commands/handlers/auth_test.rs` (selective tester dispatch, wordlist loading, `AUTH_BANNER` print)
- Uses `AuthEngine` for orchestration of selected `AuthTestType`s
- Policy enforcement: `evaluate_and_enforce_operation(OperationDescriptor { risk: CredentialTesting, ... })` (via `CommandContext`)

## Policy & Enforcement

- `OperationRisk::CredentialTesting` (high-risk tier; default blocked)
- `Capability::CredentialTesting`
- `allow_credential_testing` flag in `ExecutionPolicy` (default `false`)
- Routed through central `EnforcementContext::evaluate()` (post-2026-06-10 handler policy alignment)
- Handler regression tests + `enforcement_tests.rs` cover credential_testing paths
- No `credential-testing` Cargo feature (auth always compiled; gated at runtime by policy + scope). See `docs/AUTH_LAB.md`.
- Multi-protocol testers (SSH/FTP/SMTP) gated on `nse-ssh2` feature

## Findings & Output

- Local types only: `AuthTestReport`, `AuthFinding` (defined in `auth/mod.rs`)
- No conversion to `StoredFinding`, `ScanReportData`, or `eggsec-output` canonical types (adopted model)
- No `to_scan_report_data()` bridge or `FindingData` mapping exists or is planned
- Standard output formats supported via handler (JSON/text; where applicable) — direct emit only

**Why local findings only (standalone defense-lab design):** `eggsec auth-test` is a narrow, high-risk CLI surface (`OperationRisk::CredentialTesting`) for **controlled validation of authentication defenses** (lockout policies, MFA enforcement, rate limiting, brute-force resistance, timing side-channels, session handling) using dedicated lab test accounts only. It is intentionally **not** part of the main assessment pipeline or unified reporting system. Results are specialized per-test observations (attempt counts, observed lockout thresholds, MFA step-up responses, rate-limit behaviors, etc.) and are kept local to preserve the "lab-only control validation" framing and avoid scope creep or misuse as a general credential attack tool.

This is the adopted model. Compare to `ScanProfile::Auth` (pipeline profile: PortScan + Fingerprint + EndpointScan + Fuzz for JWT/OAuth/IDOR issues; does not invoke `auth/` module testers).

See `docs/AUTH_LAB.md` (especially the new "Output Model (Local Findings Only)" section) for full usage guidance, when to choose `auth-test` vs. pipeline `--profile auth`, safety requirements, and examples. Also see `architecture/cli_commands.md` (Special Cases) and `architecture/output.md` for how this differs from pipeline scans and the wireless/mobile optional bridges.

## TUI Status

- Full `AuthTab` implementation exists at `crates/eggsec-tui/src/tabs/auth.rs` (TabState + TabRender + TabInput)
- Explicitly **not** part of the `Tab` enum (CLI-only surface; see `architecture/tui.md`)

## Implementation Status

**Status as of 2026-06-11**: Feature complete under runtime policy model. No dedicated `credential-testing` Cargo feature (runtime `allow_credential_testing` + `CredentialTesting` risk only). See `docs/AUTH_LAB.md` for defense-lab usage.

Core module fully implemented (testers, `AuthEngine`, safety controls, `AUTH_BANNER`, multi-protocol under `nse-ssh2`). CLI command + handler + policy integration complete and tested (17 wiremock `auth_tests` + enforcement/policy contract tests green).

Gaps vs. original design: no subcommand hierarchy (`auth test`/`validate`/`regression`), no dedicated pipeline profiles (`auth-validation`/`credential-regression`), no `AuthOperation` enum or sub-caps, no `credential-testing` feature flag, no `AuthFinding` → canonical findings conversion. `ScanProfile::Auth` exists but is pipeline-focused (PortScan+Fingerprint+EndpointScan+Fuzz for JWT/OAuth/IDOR); it does not invoke `auth/` module testers. Safety is via central `EnforcementContext` + `CredentialTesting` risk at the handler boundary. All tests pass; no code changes required under the adopted model.
