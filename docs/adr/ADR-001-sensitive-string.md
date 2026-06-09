# ADR-001: SensitiveString Instead of SecretString

## Status

Accepted

## Context

We needed a type to handle sensitive data (API keys, passwords, PSKs, webhook secrets) that:
- Automatically zeroizes memory on drop to prevent secrets from lingering in memory
- Provides safe logging and display without exposing secrets
- Supports constant-time equality comparison to prevent timing attacks
- Maintains serialization compatibility with existing config files

## Decision

We use `SensitiveString` (defined in `types.rs`) instead of `SecretString` from third-party crates because:

1. **Zeroization on Drop**: `SensitiveString` uses `zeroize::Zeroize` to explicitly zeroize memory when dropped, preventing secrets from remaining in memory after use.

2. **Transparency**: `SensitiveString` serializes as a plain string, maintaining backward compatibility with existing TOML/YAML config files without special handling.

3. **Explicit API**: Our `SensitiveString` provides explicit methods:
   - `expose_secret()` - borrows the inner string
   - `into_secret()` - consumes and returns the inner string
   - `log_secret()` - safely logs with optional redaction
   - `for_logging()` - creates a display-safe wrapper

4. **Constant-Time Comparison**: `PartialEq` uses `subtle::ConstantTimeEq` to prevent timing attacks during credential checking.

5. **Minimal Dependencies**: Using a custom type avoids adding a heavy dependency for what is essentially a wrapper around `String` with zeroization.

## Consequences

- Positive: Secrets are properly zeroized in memory
- Positive: Constant-time comparison prevents timing attacks
- Positive: Serialization compatibility maintained
- Negative: Custom implementation required maintenance of the type
- Negative: Developers must use accessor methods instead of direct field access

## References

- `crates/eggsec/src/types.rs` - `SensitiveString` implementation
- [zeroize crate documentation](https://docs.rs/zeroize)
