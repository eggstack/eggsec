# Auth Control Validation Lab Guide

**Defense-lab framing**: `eggsec auth-test` exists for **controlled validation of authentication defenses** (lockout policies, rate limiting, MFA enforcement, session handling, timing side-channels, brute-force resistance) in authorized lab or private environments only. It is **not** a credential attack or exploitation tool.

## Explicit Requirements

- `allow_credential_testing = true` under `[execution_policy]` in your config (default: `false`).
- Explicit scope manifest (`--scope` or config) that covers both the target endpoint **and** any test accounts/usernames used.
- **Dedicated test accounts only**. Never use production or real-user credentials.
- Rate/attempt caps: use `--max-attempts`, `--concurrency`, `--timeout`, and scope-level `max_requests_per_second`. Start very low (e.g. 50 attempts total).

High-risk operations are blocked by default. ManualPermissive (default CLI/TUI) surfaces `RequireConfirmation` for `CredentialTesting`; strict profiles (MCP, agent, CI, `--strict-scope`) treat it as hard Deny. Use `--yes` (narrow) or `--allow-high-risk` (audited) only when you have explicit authorization.

## Example Command

```bash
eggsec auth-test https://lab-target.example.com/login \
  --all \
  --wordlist /path/to/lab-test-credentials.txt \
  --max-attempts 50 \
  --concurrency 2 \
  --yes
```

Or with explicit high-risk allowance:

```bash
eggsec auth-test ... --all --wordlist ... --max-attempts 50 --allow-high-risk --manual-override-reason "authorized lab control validation"
```

## Example Config Snippet

```toml
[execution_policy]
require_explicit_scope = true
allow_credential_testing = true
# allow_intrusive_fuzzing = false
# ... other gates ...

# Scope must explicitly authorize the lab target + test accounts
[[allowed_targets]]
host = "lab-target.example.com"
description = "Auth control validation lab target"
```

See `examples/configs/eggsec.toml` for the commented template.

## Safety Warnings

- **Account lockout**: Even small attempt counts can lock legitimate test accounts. Coordinate with the team that manages auth for the target.
- **Legal / authorization**: You must have explicit written authorization that covers credential control validation. Document scope, accounts used, attempt budgets, and time window.
- **Never production credentials**: Only synthetic or dedicated lab accounts. Breach lists or real-user data are out of scope and prohibited.
- **Monitor and rollback**: Watch target logs, auth service health, and have a kill-switch. Expect to reset test accounts after runs.
- Multi-protocol testers (SSH/FTP/SMTP) additionally require the `nse-ssh2` feature.

## Output Model (Local Findings Only)

`eggsec auth-test` is deliberately designed as a **standalone defense-lab CLI command**. It produces and emits only local `AuthTestReport` / `AuthFinding` types (defined in the `auth/` module). These are output directly as human-readable text or structured `--json` from the command handler.

**It does not:**

- Convert results to `ScanReportData`, `StoredFinding`, or any canonical finding types from the `eggsec-output` crate.
- Provide a `to_scan_report_data()` bridge, `FindingData` mapping, or similar conversion helper.
- Participate in pipeline profiles, or produce SARIF, JUnit, CSV, HTML, etc. reports via the standard unified output system.

**Why local findings only (by design):**

- `auth-test` is a high-risk operation (`OperationRisk::CredentialTesting`) intended **exclusively for controlled validation of authentication defenses** (brute-force resistance, account lockout policies, MFA enforcement, rate limiting effectiveness, timing side-channels, session handling) using dedicated lab test accounts only.
- It operates completely outside the main assessment pipeline. Its results are specialized (per-test-type observations, attempt counts, observed behaviors like lockout thresholds, MFA step-up responses, etc.) and are not intended for unification into general vulnerability reports.
- The safety model (runtime policy gating via `allow_credential_testing`, explicit scope manifest covering both target endpoint **and** test accounts, high-risk `RequireConfirmation` for ManualPermissive, hard Deny for strict/automated profiles) is enforced at the handler boundary using the central `EnforcementContext`. Canonical finding conversion was intentionally not implemented to preserve the narrow "lab-only control validation" framing and reduce risk of misuse or scope creep.
- This is the adopted model (post-2026-06-10 policy alignment): standalone CLI surface, local-only types, direct emit. No conversion path exists or is planned.

**How its output differs from normal pipeline scans:**

Normal pipeline scans (`eggsec scan <target> --profile <profile>`) execute one or more chained stages from the assessment pipeline and always produce a full `ScanReportData` structure. This contains unified findings (with canonical severity, category, evidence, remediation, etc.) that flow through the `eggsec-output` crate for consistent, loadable, diffable, and exportable reports (JSON, SARIF, JUnit, HTML, CSV, Markdown, etc.).

`ScanProfile::Auth` (the profile named "auth") is itself a **pipeline profile**: it runs PortScan + Fingerprint + EndpointScan + Fuzz, focused on application-level authentication/authorization issues detectable via fuzzing payloads (JWT algorithm confusion, weak secrets, null signatures; OAuth/OIDC redirect_uri / state / grant-type flaws; IDOR via object enumeration, etc.). It does **not** invoke any of the credential testers (BruteForce, LockoutDetector, MfaTester, etc.) from the `auth/` module.

**When to use `auth-test` vs. `ScanProfile::Auth`:**

- Use `eggsec auth-test <target> ...` (with `--all` or specific flags like `--brute-force`/`--lockout`/`--test-mfa`, a wordlist of **synthetic lab credentials only**, conservative `--max-attempts`/`--concurrency`, and the required policy allowances such as `allow_credential_testing = true` + `--allow-high-risk` or equivalent) when your goal is to **validate the effectiveness of authentication controls themselves** in an explicitly authorized lab environment.

  Examples: "Does the account lockout policy actually trigger and stay enforced after N consecutive failures?", "Is step-up MFA reliably enforced on risky login patterns?", "Are rate limits + timing protections sufficient to make brute-force impractical?", "Do session tokens behave correctly after failed attempts?"

  This requires an explicit scope that authorizes both the login endpoint **and** the test accounts being used, plus the high-risk policy gate.

- Use `eggsec scan <target> --profile auth` (or a custom profile that includes the "auth" stage) when performing a broader **web or API security assessment** that needs to probe for flaws in how authentication and authorization mechanisms are implemented in the application (e.g., JWT mis-issuance or weak verification, OAuth implementation errors, IDOR, session management issues detectable without direct credential testing).

  This flows through the standard pipeline, produces canonical `ScanReportData`, and can be exported/loaded like any other scan profile.

See also the short "Distinct from..." bullet in Implementation Notes below.

## Implementation Notes

- There is **no dedicated Cargo feature** named `credential-testing`. Auth testing is always compiled; control is exclusively via runtime policy (`allow_credential_testing` + `CredentialTesting` risk tier in the central `EnforcementContext`).
- Multi-protocol support lives under the `nse-ssh2` feature (libssh2-backed).
- Distinct from the pipeline `auth` profile (`ScanProfile::Auth`), which performs PortScan + Fingerprint + EndpointScan + Fuzz focused on JWT/OAuth/IDOR issues (see `architecture/pipeline.md` and `architecture/auth.md`). The CLI `auth-test` command invokes the `auth/` module testers directly and does not use the pipeline.

## See Also

- `docs/SAFETY.md` — risk tiers, execution profiles, `EnforcementContext`
- `docs/SAFETY.md` — "Authentication Testing" section with lab requirements
- `architecture/auth.md` — module types, CLI surface, policy integration, TUI status, local-findings-only decision
- `crates/eggsec/src/commands/handlers/auth_test.rs` — handler, `evaluate_and_enforce_operation`, `AUTH_BANNER`, wordlist loading, reporting
- `crates/eggsec/src/config/policy.rs` — `OperationRisk::CredentialTesting`, `allow_credential_testing`
- `docs/CAPABILITIES.md`, `README.md`, `AGENTS.md` (key types + security notes + standalone defense-lab surfaces)
