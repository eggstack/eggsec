# Safety and Scope Enforcement

Eggsec is a security testing toolkit designed for **authorized testing only**.

## Scope Enforcement

All target-bearing operations go through scope validation:
- Direct IP addresses (e.g., `127.0.0.1`) are blocked by default
- Scope rules define allowed targets
- Operations outside scope are rejected

## Operation Risk Tiers

Eggsec classifies operations by risk level:

| Risk Level | Description | Default |
|------------|-------------|---------|
| Passive | Read-only operations | Allowed |
| ActiveScan | Port scanning, fingerprinting | Allowed |
| IntrusiveFuzz | Fuzzing, injection testing | Blocked |
| LoadTest | Load testing | Blocked |
| StressTest | Stress testing | Blocked |
| RawPacket | Raw packet operations | Blocked |
| CredentialTesting | Auth testing | Blocked |
| RemoteExecution | Remote command execution | Blocked |
| AgentAutonomous | Agent-driven operations | Blocked |

High-risk operations must be explicitly enabled in your config file.

## Authorization Requirements

Before using Eggsec:
1. Ensure you have explicit authorization to test the target
2. Understand the scope of your testing engagement
3. Review and configure operation policies appropriately
4. Never test production systems without authorization

## Configuration

Operation policies are configured in your config file:

```toml
[execution_policy]
require_explicit_scope = true
allow_intrusive_fuzzing = false
allow_stress_testing = false
```

See `architecture/feature_matrix.md` for feature flags.

## Operating Modes

Eggsec operates in three modes:

- **standard-assessment**: Ordinary scoped scanning, fuzzing, API testing, WAF detection
- **defense-lab**: Local/private WAF regression, Synvoid validation, protocol edge testing
- **hazardous-lab**: Raw packets, flood stress, proxy rotation, distributed stress

Each CLI command's help text indicates its mode. Use `eggsec policy-explain` to inspect decisions before running traffic-generating operations.

## Policy Decision Records

Every target-bearing operation produces a structured policy decision with:
- Unique decision ID
- Operation mode and risk level
- Target normalization and scope matching
- Required features and policy flags
- Denial reasons (when blocked)

Use `eggsec policy-explain --json` to view a policy decision without executing.
