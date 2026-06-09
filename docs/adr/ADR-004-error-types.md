# ADR-004: Error Type Separation

## Status

Accepted

## Context

We needed to handle errors across different layers of the application:
- Library code returning structured errors
- CLI/TUI code using `anyhow::Result` for ergonomic error handling
- Configuration errors with context
- External library errors

## Decision

We use three error handling patterns, each serving different purposes:

### 1. `EggsecError` (Core Library)

`crate::error::EggsecError` is a structured error enum for the core library:

```rust
pub enum EggsecError {
    Timeout { message: String },
    Network { message: String },
    Config(String),
    // ... 23 variants total
}
```

Used in core modules with `crate::error::Result<T>` (which is `Result<T, EggsecError>`).

**Rationale**: Structured errors allow callers to match on specific error types and handle them appropriately.

### 2. `ConfigError` (Configuration)

`crate::config::ConfigError` wraps std::io::Error for file operations:

```rust
pub enum ConfigError {
    FileRead(String, std::io::Error),
    Parse(String, String),
    // ...
}
```

**Rationale**: Configuration errors have specific variants for different failure modes.

### 3. `anyhow::Result` (CLI/TUI/Tests)

Commands and tests use `anyhow::Result<T>` which is `Result<T, anyhow::Error>`.

**Rationale**: Ergonomic error handling without explicit error type, allowing rapid prototyping and simple error propagation in application code.

## Consequences

- Positive: Core library has structured, actionable errors
- Positive: CLI code is ergonomic with `anyhow`
- Positive: Configuration errors have context
- Negative: Error conversion requires `From` implementations
- Negative: Mixed error types can be confusing

## Conversion Between Types

```rust
// EggsecError implements From<ConfigError>
impl From<ConfigError> for EggsecError {
    fn from(e: ConfigError) -> Self {
        EggsecError::Config(e.to_string())
    }
}
```

Feature-gated conversions exist for:
- `AiError` (from `ai/errors.rs`)
- `CaptureError` (from `packet/capture.rs`)
- `TracerouteError` (from `packet/traceroute.rs`)

## References

- `crates/eggsec/src/error/mod.rs` - `EggsecError` definition
- `crates/eggsec/src/config/mod.rs` - `ConfigError` definition
- `AGENTS.md` - Error handling guidelines
