# Safety and Scope Enforcement

Eggsec is for authorized security testing only. You are responsible for getting
permission, defining the assessment boundary, and avoiding impact to other
systems or users. A scope file helps enforce a boundary, but it does not grant
authorization by itself.

## Start with scope

Create a scope file with the targets and ports approved for your assessment.
For a preview that does not send traffic, run:

```bash
eggsec plan --scope scope.toml --target https://staging.example.com
```

Run only after reviewing the plan and confirming authorization:

```bash
eggsec scan staging.example.com --profile quick --scope scope.toml
```

See the [scope guide](scope.md) for the file format and matching rules.

Hostname patterns match the requested hostname. They do not constrain the IP
addresses returned by DNS. If a hostname may resolve outside your approved
boundary, do not rely on a broad wildcard alone. Define address exclusions.
Run `eggsec scope-explain --help` for the syntax to inspect a target's decision.

Loopback targets are treated separately by scope policy. Some scanner paths
also require the explicit `EGGSEC_ALLOW_LOOPBACK_FIXTURE=1` opt-in. Use it only
for a local fixture or lab, together with an explicit scope file. It is not a
substitute for authorization.

## Enforcement modes

CLI and TUI commands evaluate policy before dispatch. The default manual mode
may ask an operator to confirm some decisions. Use `--strict-scope` when you
want the CLI to deny unresolved or out-of-scope operations without overrides.
Automated surfaces such as CI, REST, MCP, and agents use strict enforcement and
do not accept manual override flags. Networked automated operations require an
explicit scope manifest.

Some commands also require optional build features. A command being present
does not mean policy allows it. Risk policy and feature requirements still
apply. See [enforcement modes](ENFORCEMENT_MODES.md) and
[build features](BUILD.md).

## Higher-risk operations

Fuzzing, load testing, credential testing, raw packet operations, traffic
interception, stress testing, and autonomous execution can disrupt services,
trigger security controls, or affect user accounts. Use these only with
specific authorization and appropriate safeguards:

- Confirm the targets, permitted techniques, time window, and rate limits with
  the system owner.
- Use isolated labs and dedicated test accounts for intrusive or account-based
  checks.
- Start with conservative request rates and concurrency. Monitor for errors or
  degradation, and stop if unexpected impact occurs.
- Coordinate with operations and security teams when testing production-like
  environments.
- Keep credentials and tokens out of shell history, logs, and shared reports.

High-risk capabilities are restricted by execution policy and may require
explicit configuration or runtime confirmation. Strict automated modes fail
closed rather than accepting manual overrides.

## Further reading

- [Scope model](scope.md)
- [Enforcement modes](ENFORCEMENT_MODES.md)
- [Authentication testing](AUTH_LAB.md)
- [Platform prerequisites](PLATFORM.md)
