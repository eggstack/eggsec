---
name: code_quality_patterns
description: "Code quality patterns, error handling, testing best practices, and documentation standards for Slapper"
triggers:
  - code quality
  - error handling
  - From impl
  - error conversion
  - doc test
  - test gating
  - feature flag
  - clippy
  - unwrap audit
  - documentation
  - ADR
  - architecture
metadata:
  category: maintenance
  tools: [general]
  scope: internal
---

## Overview

This skill documents code quality patterns, error handling best practices, and testing conventions for working on the Slapper security toolkit codebase.

## Error Type Conversion Patterns

### Adding From Implementations

When adding error type conversions to `error/mod.rs`, always gate with appropriate feature flags:

```rust
#[cfg(feature = "ai-integration")]
impl From<crate::ai::AiError> for SlapperError {
    fn from(e: crate::ai::AiError) -> Self {
        match e {
            crate::ai::AiError::Timeout => {
                SlapperError::Timeout { timeout_ms: 0, operation: "ai-request".to_string() }
            }
            // ... other variants
        }
    }
}
```

### Pattern: Match Enum Variants Explicitly

Instead of using `#[from]` attribute, explicitly match each variant to control mapping:

```rust
impl From<ThirdPartyError> for SlapperError {
    fn from(e: ThirdPartyError) -> Self {
        match e {
            ThirdPartyError::Timeout => SlapperError::Timeout { /* ... */ },
            ThirdPartyError::AuthFailed => SlapperError::Config("Auth failed".to_string()),
            ThirdPartyError::RateLimited => SlapperError::RateLimited("rate limit".to_string()),
        }
    }
}
```

## Feature Gating Patterns

### Integration Test Feature Gating

Always gate integration tests with the appropriate feature flag:

```rust
// Wrong: Test will fail to compile without feature
use slapper::stress::authorization::{create_example_stress_config, StressScope};

// Correct: Test only compiles when feature is enabled
#![cfg(feature = "stress-testing")]

use slapper::stress::authorization::{create_example_stress_config, StressScope};
```

### Module Feature Gating

Gate module declarations, not just uses:

```rust
// Wrong: Dead code warning when feature disabled
pub mod stress;  // Module exists unconditionally

// Correct: Module only exists when feature enabled
#[cfg(feature = "stress-testing")]
pub mod stress;
```

## Doc Test Patterns

### Valid Doc Test Structure

Doc tests must use correct types and signatures:

```rust
/// # Examples
///
/// ```rust,no_run
/// use slapper::scanner::{scan_ports, SpoofConfig};
/// use std::time::Duration;
///
/// # async fn example() -> slapper::error::Result<()> {
/// let results = scan_ports(
///     "example.com",
///     vec![80, 443],
///     100,  // concurrency
///     Duration::from_secs(5),
///     false,  // tui_mode
///     SpoofConfig::default(),
///     None,  // progress_tx
/// ).await?;
/// # Ok(())
/// # }
/// ```
```

### Common Doc Test Errors

1. **Wrong field names**: Use actual struct field names, not similar ones
2. **Missing Default**: Many structs don't implement Default - construct explicitly
3. **Missing arguments**: Ensure all required arguments are provided
4. **Async/sync mismatch**: Don't use `.await` on sync functions
5. **Private types**: Use only public API in doc examples

## URL Encoding Pattern

Always encode user-provided query parameters:

```rust
// Wrong: Special characters break URL
let url = format!("https://api.github.com/search/issues?q={}", query);

// Correct: Special characters are safely encoded
let url = format!(
    "https://api.github.com/search/issues?q={}",
    urlencoding::encode(query)
);
```

## Dead Code Security Pattern

Code after an early return that can never execute is a security risk - remove it:

```rust
// Wrong: Dead code after early return
if env.is_some() {
    return Err("Custom environment variables are not allowed".to_string());
}

// This code is unreachable but still compiled
if let Some(env_vars) = env {
    for (key, value) in env_vars {
        cmd.env(&key, &value);  // Security risk!
    }
}

// Correct: Removed unreachable code
if env.is_some() {
    return Err("Custom environment variables are not allowed".to_string());
}
```

## Serialization Roundtrip Testing Pattern

When testing types that implement `Serialize` + `DeserializeOwned` + `Eq`, use the helper from `tests/common/mod.rs`:

```rust
use crate::tests::common::assert_serialize_roundtrip;
use slapper::types::Severity;

// Instead of repeating serialization logic:
let json = serde_json::to_string(&value).unwrap();
let decoded: Type = serde_json::from_str(&json).unwrap();
assert_eq!(value, decoded);

// Use the helper:
assert_serialize_roundtrip(&value);
```

## Safe Serialization Helpers

For production code that serializes/deserializes JSON, use the safe helpers in `utils/serialization.rs`:

```rust
use crate::utils::serialization::{serialize_to_json, deserialize_from_json};

// Safe serialization (returns Result, not unwrap)
let json = serialize_to_json(&my_value)?;

// Safe deserialization (returns Result, not unwrap)
let decoded: MyType = deserialize_from_json(&json)?;
```

These helpers convert serde errors to `SlapperError::Parse` for consistent error handling.

## Public API Documentation Pattern

All public functions should have doc comments with `# Arguments`, `# Returns`, and `# Example` sections:

```rust
/// Creates a successful response with the given results.
///
/// # Arguments
///
/// * `request_id` - The request ID from the original ToolRequest
/// * `tool_id` - The tool identifier
/// * `results` - The tool-specific results as JSON
///
/// # Example
///
/// ```rust
/// use slapper::tool::response::ToolResponse;
///
/// let response = ToolResponse::success(
///     "req-123",
///     "scanner",
///     serde_json::json!({"ports": [80, 443]})
/// );
/// ```
pub fn success(
    request_id: impl Into<String>,
    tool_id: impl Into<String>,
    results: serde_json::Value,
) -> Self {
    // ...
}
```

## Architecture Decision Records

When making significant architectural decisions, document them in `docs/adr/`:

```markdown
# ADR-XXX: Title

## Status
Accepted

## Context
What problem are we solving?

## Decision
What is the change we're making?

## Consequences
What becomes easier or harder due to this change?

## References
Links to relevant documentation
```

See existing ADRs in `docs/adr/` for examples:
- ADR-001: SensitiveString vs SecretString
- ADR-002: Feature flag design rationale
- ADR-003: rustls over native-tls (except nse)
- ADR-004: Error type separation

## Triggers

Keywords: code quality, error handling, From impl, error conversion, doc test, test gating, feature flag, clippy, unwrap audit, feature gate, security patterns, defensive coding, documentation, ADR, architecture, serialization, roundtrip

## Common Pitfalls

1. **Enum variant mismatches**: When converting errors, ensure all variants are explicitly handled
2. **RateLimited variant**: Takes a `String` argument: `SlapperError::RateLimited("reason".to_string())`
3. **Feature-gated imports**: Imports inside `#[cfg(...)]` blocks must also be gated
4. **Iterator on reference**: `get_all_payloads_cached()` returns `&Vec<T>`, not `Vec<T>` - don't use `&` in for loops

## Verification Commands

```bash
# Check compilation
cargo check --lib -p slapper

# Run tests
cargo test --lib -p slapper

# Run clippy
cargo clippy --lib -p slapper

# Run specific integration test
cargo test --test fuzzer_tests
cargo test --test stress_tests --features stress-testing
```

## References

- [Rust Error Handling](https://doc.rust-lang.org/book/ch09-00-error-handling.html)
- [Testing in Rust](https://doc.rust-lang.org/book/ch11-00-testing.html)
- CWE-755: Improper Handling of Exceptional Conditions
