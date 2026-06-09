# Probe Classification

## Overview

Shared probe intent and risk vocabulary defined in `crates/eggsec/src/probe.rs`. Used across scanner, NSE, WAF, loadtest, and defense-lab profiles.

## Key Types

### ProbeIntent (7 variants)

Intent categories for security probes:

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

Risk classification for guardrails and opt-in:

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

## Serialization

All enums serialize to kebab-case JSON (e.g., `"discovery"`, `"safe-active"`).
