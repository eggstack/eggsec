# Slapper Auth Skill

Authentication security testing module.

## Module Location
`crates/slapper/src/auth/`

## Tab
Auth tab is one of the 29 TUI tabs - see `slapper-tui/SKILL.md` for TUI patterns.

## Key Types

- `AuthEngine` - Main authentication testing engine
- `BruteForceTester` - Credential brute force testing
- `CredentialStuffingTester` - Breach credential testing
- `LockoutDetector` - Account lockout detection
- `MfaTester` - MFA bypass testing
- `RateLimitTester` - Rate limit testing

## Patterns

### Brute Force Testing
```rust
let mut engine = AuthEngine::new();
engine.add_wordlist("rockyou.txt");
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

## AGENTS.md Override
See `crates/slapper/src/auth/AGENTS.override.md`