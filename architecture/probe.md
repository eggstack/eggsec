# Probe Classification

## Overview

Shared probe intent and risk vocabulary defined in `crates/eggsec/src/probe.rs`. Used across scanner, NSE, WAF, loadtest, and defense-lab profiles.

## Key Types

### ProbeIntent (7 variants)

Intent categories for security probes. Derives `Hash` and `Copy` in addition to standard traits.

| Variant | Description |
|---------|-------------|
| `Discovery` | Port/service discovery |
| `Fingerprint` | Service version fingerprinting |
| `ServiceValidation` | Validate detected services |
| `WafEvaluation` | WAF detection and evaluation |
| `EvasionResistance` | Test WAF evasion techniques |
| `LoadBearing` | Load testing |
| `Stress` | Stress testing |

### ProbeRisk (6 variants)

Risk classification for guardrails and opt-in. Derives `Hash` and `Copy` in addition to standard traits.

| Variant | Description |
|---------|-------------|
| `Passive` | Passive observation only |
| `SafeActive` | Safe active probing |
| `Intrusive` | Intrusive testing |
| `Credentialed` | Requires credentials |
| `Stress` | Stress/load testing |
| `ExploitAdjacent` | Near-exploitation testing |

### ProbeRisk → OperationRisk Mapping

`ProbeRisk::to_operation_risk()` converts probe-level risk to the shared `OperationRisk` enum used by the policy evaluator:

| ProbeRisk | OperationRisk | Policy Gate |
|-----------|---------------|-------------|
| `Passive` | `Passive` | Always allowed |
| `SafeActive` | `SafeActive` | Always allowed |
| `Intrusive` | `Intrusive` | `allow_intrusive_fuzzing` |
| `Credentialed` | `CredentialTesting` | `allow_credential_testing` |
| `Stress` | `StressTest` | `allow_stress_testing` |
| `ExploitAdjacent` | `ExploitAdjacent` | `allow_exploit_adjacent` |

### ProbeRisk Methods

- `risk_level()` - Returns a numeric risk level (`u8`, 0-5). Higher values indicate higher risk. Used to enforce risk budgets: a stage is skipped if its risk level exceeds the profile's budget. Order: Passive(0) < SafeActive(1) < Intrusive(2) < Credentialed(3) < Stress(4) < ExploitAdjacent(5).
- `requires_opt_in()` - Returns `true` if this risk level requires explicit user opt-in. Returns true for: `Credentialed`, `Intrusive`, `Stress`, `ExploitAdjacent`. Returns false for: `Passive`, `SafeActive`.

## Serialization

All enums serialize to kebab-case JSON (e.g., `"discovery"`, `"safe-active"`).
