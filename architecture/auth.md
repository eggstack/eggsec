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
| `ssh.rs` | SSH-specific authentication testing |
| `ftp.rs` | FTP-specific authentication testing |
| `smtp.rs` | SMTP-specific authentication testing |

## Implementation Status

Fully implemented. All sub-modules contain working test logic with `AuthEngine` orchestrating execution. Includes safety mechanisms (`stop_on_lockout`, `max_attempts`, `stop_flag`) and protocol-specific testers.
