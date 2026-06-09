# ADR-002: Feature Flag Design Rationale

## Status

Accepted

## Context

Eggsec has multiple optional features that increase binary size and compilation time. We needed a strategy to manage these features while maintaining usability and build flexibility.

## Decision

We use Cargo feature flags with the following design principles:

1. **Granular Features**: Each optional capability is a separate feature flag (e.g., `stress-testing`, `nse`)

2. **Composite Features**: The `full` feature enables all features except those with known issues:
   ```toml
   full = ["stress-testing", "packet-inspection",
           "rest-api", "grpc-api", "nse", "ai-integration"]
   ```

3. **Explicit Exclusions**: Two features are intentionally excluded from `full`:
   - `grpc-api`: Requires additional system dependencies
   - `nse-sandbox`: Security feature that may break some NSE scripts

4. **Feature Gating**: Code uses `#[cfg(feature = "...")]` to conditionally compile:
   - Entire module declarations when a module is only available with a feature
   - Function implementations when they depend on optional dependencies
   - Test code when tests require specific features

5. **Documentation**: AGENTS.md documents all feature flags and their interactions

## Consequences

- Positive: Users can build minimal binaries with only needed features
- Positive: CI can test with different feature combinations
- Positive: Optional dependencies don't affect users who don't need them
- Negative: Feature interactions can be complex to debug
- Negative: Some features have hidden dependencies on others

## Feature Flag Reference

| Feature | Description | Default |
|---------|-------------|---------|
| `stress-testing` | ICMP probing, IP spoofing, raw sockets | off |
| `packet-inspection` | Packet capture features | off |
| `rest-api` | REST API server | off |
| `grpc-api` | gRPC API server (NOT in `full`) | off |
| `nse` | Nmap NSE script support | off |
| `nse-sandbox` | NSE sandbox mode (NOT in `full`) | off |
| `ai-integration` | AI/LLM features | off |
| `full` | All features except grpc-api, nse-sandbox | off |

## References

- `crates/eggsec/Cargo.toml` - Feature definitions
- `AGENTS.md` - Feature flag documentation
