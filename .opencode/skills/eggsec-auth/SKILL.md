# Eggsec Auth Skill

Authentication security testing module.

## Module Location
`crates/eggsec/src/auth/`

## Tab
Auth tab is one of the 29 TUI tabs - see `eggsec-tui/SKILL.md` for TUI patterns.

## Key Types

- `AuthEngine` - Main authentication testing engine
- `BruteForceTester` - Credential brute force testing
- `CredentialStuffer` - Breach credential testing
- `LockoutDetector` - Account lockout detection
- `MfaTester` - MFA bypass testing
- `RateLimitTester` - Rate limit testing
- `PasswordPolicyTester` - Password policy testing
- `SessionTester` - Session management testing
- `TimingTester` - Timing attack testing
- `ProtocolAuthTester` - Multi-protocol authentication testing

## Patterns

### Brute Force Testing
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
- `brute_force.rs` - Brute force testing
- `credential_stuffing.rs` - Credential stuffing
- `lockout.rs` - Lockout detection
- `mfa.rs` - MFA bypass
- `rate_limit.rs` - Rate limit testing
- `timing.rs` - Timing attack testing

## Module Notes
See `architecture/auth.md` for architecture documentation.