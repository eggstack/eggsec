# Probe Classification

## Overview

Shared probe intent and risk vocabulary defined in a single file: `crates/eggsec/src/probe.rs` (148 lines). Used across scanner, NSE, WAF, loadtest, and defense-lab profiles to describe what a probe is trying to achieve and what risk tier it belongs to. Profiles compile into probe plans carrying these metadata tags, enabling guardrails, budget requirements, and explicit opt-in behavior.

The module is **policy-free** — it defines vocabulary only. Policy evaluation happens upstream via `EnforcementContext::evaluate()`. See [config.md](config.md) and [overview.md](overview.md).

## Role & Responsibilities

- **Intent classification**: `ProbeIntent` tags what a probe is trying to do (discover, fingerprint, stress, etc.)
- **Risk classification**: `ProbeRisk` assigns a numeric risk tier used for budget enforcement and opt-in gating
- **Risk bridging**: `ProbeRisk::to_operation_risk()` converts probe-level risk to the shared `OperationRisk` enum used by the enforcement model
- **Cross-module vocabulary**: Same enums used by scanner (`scanner/`), NSE (`eggsec-nse`), WAF (`waf/`), loadtest (`loadtest/`), and defense-lab profiles

## Location & Feature Gating

| Component | Path | Feature Gate |
|-----------|------|-------------|
| `ProbeIntent` | `probe.rs:17` | Always |
| `ProbeRisk` | `probe.rs:33` | Always |
| Serialization (kebab-case) | `probe.rs:16,32` | Always |

No feature gating — always compiled.

## Architecture

### ProbeIntent (7 variants)

Defined at `probe.rs:17`. Derives `Hash`, `Copy`, `Clone`, `Debug`, `PartialEq`, `Eq`, `Serialize`, `Deserialize`.

| Variant | Description | Typical Consumer |
|---------|-------------|-----------------|
| `Discovery` | Port/service discovery | Scanner port scan |
| `Fingerprint` | Service version fingerprinting | Scanner fingerprint |
| `ServiceValidation` | Validate detected services | Scanner/NSE |
| `WafEvaluation` | WAF detection and evaluation | WAF module |
| `EvasionResistance` | Test WAF evasion techniques | WAF/fuzzer |
| `LoadBearing` | Load testing | Loadtest module |
| `Stress` | Stress testing | Stress module |

### ProbeRisk (6 variants)

Defined at `probe.rs:33`. Derives `Hash`, `Copy`, `Clone`, `Debug`, `PartialEq`, `Eq`, `Serialize`, `Deserialize`.

| Variant | Risk Level | Requires Opt-In | Description |
|---------|-----------|-----------------|-------------|
| `Passive` | 0 | No | Passive observation only |
| `SafeActive` | 1 | No | Safe active probing |
| `Intrusive` | 2 | Yes | Intrusive testing |
| `Credentialed` | 3 | Yes | Requires credentials |
| `Stress` | 4 | Yes | Stress/load testing |
| `ExploitAdjacent` | 5 | Yes | Near-exploitation testing |

### ProbeRisk Methods

| Method | Location | Return Type | Description |
|--------|----------|-------------|-------------|
| `risk_level()` | `probe.rs:47` | `u8` | Numeric tier 0–5 for ordering/budget comparison |
| `requires_opt_in()` | `probe.rs:59` | `bool` | `true` for Credentialed, Intrusive, Stress, ExploitAdjacent |
| `to_operation_risk()` | `probe.rs:69` | `OperationRisk` | Maps to the 15-tier `OperationRisk` enum for enforcement |

### ProbeRisk → OperationRisk Mapping

`ProbeRisk::to_operation_risk()` (`probe.rs:69`) converts the 6-tier probe vocabulary to the 15-tier enforcement vocabulary defined in `config/policy.rs:9`:

| ProbeRisk | OperationRisk | Enforcement Gate |
|-----------|---------------|-----------------|
| `Passive` | `Passive` | Always allowed |
| `SafeActive` | `SafeActive` | Always allowed |
| `Intrusive` | `Intrusive` | `allow_intrusive_fuzzing` |
| `Credentialed` | `CredentialTesting` | `allow_credential_testing` |
| `Stress` | `StressTest` | `allow_stress_testing` |
| `ExploitAdjacent` | `ExploitAdjacent` | `allow_exploit_adjacent` |

Note: `OperationRisk` has 15 variants total (`Passive` through `AgentAutonomous`). Only the 6 relevant to probe classification are mapped by this function. The remaining 9 (`LoadTest`, `RawPacket`, `DbPentest`, `TrafficInterception`, `EvasionTesting`, `PostExploitation`, `C2Operation`, `RemoteExecution`, `AgentAutonomous`) are used directly by other operation metadata entries.

## Behavior / Flow

### Risk Budget Usage

Pipeline stages carry `ProbeRisk` tags. Before execution, each stage's risk level is compared against the profile's risk budget:

```
Profile risk_budget = Intrusive (level 2)
  Stage A: Discovery (level 0) → allowed (0 ≤ 2)
  Stage B: Fingerprint (level 1) → allowed (1 ≤ 2)
  Stage C: Intrusive (level 2) → allowed (2 ≤ 2)
  Stage D: Stress (level 4) → SKIPPED (4 > 2)
```

### Opt-In Gating

When a stage's `ProbeRisk::requires_opt_in()` returns `true`, the enforcement model requires explicit user confirmation or policy allowlist entry before execution. This applies to:

- **Intrusive** testing (fuzzing, brute force)
- **Credentialed** operations (credential testing)
- **Stress** testing (DoS simulation)
- **ExploitAdjacent** operations (near-exploitation)

### Serialization

All enums serialize to kebab-case JSON:

```json
// ProbeIntent
"discovery", "fingerprint", "service-validation", "waf-evaluation",
"evasion-resistance", "load-bearing", "stress"

// ProbeRisk
"passive", "safe-active", "intrusive", "credentialed",
"stress", "exploit-adjacent"
```

## Public API

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProbeIntent {
    Discovery,
    Fingerprint,
    ServiceValidation,
    WafEvaluation,
    EvasionResistance,
    LoadBearing,
    Stress,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProbeRisk {
    Passive,
    SafeActive,
    Intrusive,
    Credentialed,
    Stress,
    ExploitAdjacent,
}

impl ProbeRisk {
    pub fn risk_level(self) -> u8;                          // probe.rs:47
    pub fn requires_opt_in(self) -> bool;                   // probe.rs:59
    pub fn to_operation_risk(self) -> OperationRisk;        // probe.rs:69
}
```

## Integration Points

- **Scanner**: Port scan stages tagged `Discovery`, fingerprint stages tagged `Fingerprint`. Risk levels determine which stages execute under a given profile.
- **NSE**: NSE scripts declare probe intent/risk for budget enforcement. `NseRunReport` includes per-run risk metadata.
- **WAF**: WAF detection tagged `WafEvaluation` (SafeActive), evasion testing tagged `EvasionResistance` (Intrusive).
- **Loadtest**: Load tests tagged `LoadBearing` (Stress level).
- **Defense-lab profiles**: Pipeline stages use `ProbeRisk` to enforce risk budgets across multi-stage assessments.
- **Enforcement model**: `ProbeRisk::to_operation_risk()` bridges to the 15-tier `OperationRisk` enum used by `EnforcementContext::evaluate()`.

## Configuration Types

The module defines only enums — no configuration structs. Risk budgets are configured at the profile level in `config/` (see [config.md](config.md)).

## Testing

- **Serialization round-trip**: All 7 `ProbeIntent` and 6 `ProbeRisk` variants verified to serialize to expected kebab-case JSON (`probe.rs:86-128`).
- **Risk ordering**: Explicit assertions that `Passive < SafeActive < Intrusive < Credentialed < Stress < ExploitAdjacent` (`probe.rs:131-137`).
- **Opt-in gating**: All 6 variants verified for `requires_opt_in()` correctness (`probe.rs:140-147`).
- **OperationRisk mapping**: Each `ProbeRisk` variant maps to the correct `OperationRisk` variant (`probe.rs:69-78`).

## Invariants & Gotchas

1. **Vocabulary only, no policy**: `probe.rs` defines types but never evaluates them. All authorization happens in `EnforcementContext`.
2. **Exact 7 intent variants**: `ProbeIntent` has exactly 7 variants. Adding a new variant requires updating all match arms in consumers.
3. **Exact 6 risk tiers**: `ProbeRisk` has exactly 6 variants with levels 0–5. The numeric ordering is used for budget comparison.
4. **Opt-in threshold**: `requires_opt_in()` returns `true` for risk level ≥ 2 (Intrusive). Levels 0–1 (Passive, SafeActive) are always allowed.
5. **Bidirectional mapping not guaranteed**: `to_operation_risk()` maps 6 of 15 `OperationRisk` variants. The reverse mapping is not defined — not every `OperationRisk` has a corresponding `ProbeRisk`.
6. **Derives Hash + Copy**: Both enums derive `Hash` and `Copy`, enabling use as map keys and cheap pass-by-value.
7. **kebab-case JSON**: Serialization uses `#[serde(rename_all = "kebab-case")]`. Consumers must handle `"safe-active"` not `"SafeActive"`.

## Cross-Links

- [overview.md](overview.md) — system architecture, enforcement model
- [scanner.md](scanner.md) — scanner module (primary consumer of ProbeIntent/ProbeRisk)
- [config.md](config.md) — enforcement context, OperationRisk enum, policy evaluation
- [stress.md](stress.md) — stress testing (ProbeRisk::Stress)
- [dispatch.md](dispatch.md) — task dispatch, executor layer

---

*Last verified against source: 2026-08-25*
