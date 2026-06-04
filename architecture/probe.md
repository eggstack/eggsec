# Probe Classification

## Overview

Shared probe intent and risk vocabulary defined in `crates/slapper/src/probe.rs`. Used across scanner, NSE, WAF, loadtest, and defense-lab profiles.

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

### ProbeMetadata

Struct combining `id`, `name`, `intent`, `risk`, `requires_explicit_scope`, `requires_budget`, `compatibility_source`.

## Serialization

All enums serialize to kebab-case JSON (e.g., `"discovery"`, `"safe-active"`).
