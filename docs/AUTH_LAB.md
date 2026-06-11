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

## Implementation Notes

- There is **no dedicated Cargo feature** named `credential-testing`. Auth testing is always compiled; control is exclusively via runtime policy (`allow_credential_testing` + `CredentialTesting` risk tier in the central `EnforcementContext`).
- Multi-protocol support lives under the `nse-ssh2` feature (libssh2-backed).
- Distinct from the pipeline `auth` profile (`ScanProfile::Auth`), which performs PortScan + Fingerprint + EndpointScan + Fuzz focused on JWT/OAuth/IDOR issues (see `architecture/pipeline.md` and `architecture/auth.md`). The CLI `auth-test` command invokes the `auth/` module testers directly and does not use the pipeline.

## See Also

- `docs/SAFETY.md` — risk tiers, execution profiles, `EnforcementContext`
- `docs/lab-safety.md` — dedicated "Authentication Testing" section with lab requirements
- `architecture/auth.md` — module types, CLI surface, policy integration, TUI status
- `crates/eggsec/src/commands/handlers/auth_test.rs` — handler, `evaluate_and_enforce_operation`, `AUTH_BANNER`, wordlist loading, reporting
- `crates/eggsec/src/config/policy.rs` — `OperationRisk::CredentialTesting`, `allow_credential_testing`
- `docs/CAPABILITIES.md`, `README.md`, `AGENTS.md` (key types + security notes)
