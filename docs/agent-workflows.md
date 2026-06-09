# Agent Workflows

Eggsec is designed as a **controlled assessment backend** for automated and agent-driven security testing. It is not an autonomous scanner - it operates within explicit scope boundaries, produces structured output, and respects human approval gates.

## Why Agents Use Eggsec

| Property | Benefit |
|----------|---------|
| **Structured output** | JSON, SARIF, JUnit formats machine-parseable by CI and dashboards |
| **Repeatable** | Same command + same scope = same results across runs |
| **Scope-enforced** | Cannot target out-of-scope systems even if instructed |
| **Risk-tiered** | Operations classified by risk; high-risk ops blocked by default |
| **Rate-limited** | Built-in `--rate-limit` and scope-level `max_requests_per_second` |

## Tool / API / MCP Surfaces

Agents interact with Eggsec through three interfaces:

| Surface | Use Case |
|---------|----------|
| **CLI** | Direct invocation from scripts, Makefiles, CI jobs |
| **REST API** | Programmatic control with `rest-api` feature flag |
| **MCP Protocol** | Tool-use integration with LLM agents |

All three surfaces enforce the same scope rules. There is no bypass path.

```bash
# CLI invocation from an agent script
eggsec scan "$TARGET" \
  --profile full \
  --scope /etc/eggsec/scope.toml \
  --output json \
  --output-dir /tmp/scan-results

# REST API invocation
curl -X POST http://localhost:8080/api/v1/scan \
  -H 'Content-Type: application/json' \
  -d '{"target": "web.example.com", "scope": "/etc/eggsec/scope.toml"}'
```

## Scope-First Execution

Every agent workflow must define scope **before** execution:

1. **Load scope file** - Agent reads and validates the scope configuration
2. **Verify targets** - All intended targets must appear in `allowed_targets`
3. **Execute within bounds** - Eggsec rejects any target not matching scope rules
4. **Report scope violations** - Failed scope checks are logged and returned as errors

This is not optional. Even if an agent attempts to scan `evil.com`, the scope check rejects it and returns a structured error.

## CI / Regression Usage

Eggsec integrates into CI pipelines for continuous security regression testing:

```yaml
# GitHub Actions example
- name: Security scan
  run: |
    eggsec scan "$DEPLOYED_URL" \
      --profile quick \
      --scope .eggsec/scope.toml \
      --output sarif \
      --output-dir security-results/

- name: Upload SARIF
  uses: github/codeql-action/upload-sarif@v3
  with:
    sarif_file: security-results/
```

```makefile
# Makefile integration
.PHONY: security-scan
security-scan:
	eggsec scan $(TARGET_URL) \
		--profile full \
		--scope scopes/$(ENV).toml \
		--output json \
		--output-dir reports/security-$(CI_COMMIT_SHA)/
```

Key CI properties:
- Deterministic - same inputs produce same output structure
- Exit codes - non-zero on findings above severity threshold
- SARIF output - integrates with GitHub, GitLab, and other code scanning dashboards

## Scheduled Defensive Assessments

For ongoing monitoring of production-like environments:

```bash
# Run agent with a portfolio of targets
eggsec agent run \
  --portfolio ~/.config/eggsec/portfolio.json \
  --scope /etc/eggsec/scope.toml \
  --once
```

The agent respects scan budgets (max duration, max requests, max findings) and cooldowns between scans. See [AGENT.md](AGENT.md) for full agent configuration.

## Coding-Agent Defense-Lab Usage

Eggsec serves as a controlled backend for coding agents building security tooling:

```python
# Pseudocode: coding agent using Eggsec
target = deploy_test_container()
scope = create_scope_file(allowed=[target.hostname])

result = run_eggsec(
    command="scan",
    target=target.hostname,
    scope=scope.path,
    output="json"
)

findings = parse_json(result.stdout)
assert_no_critical(findings)
teardown(target)
```

Coding agents should:
- Deploy isolated test infrastructure (Docker, VMs)
- Generate scope files dynamically for each target
- Parse structured output for assertions
- Tear down infrastructure after testing

## Output Formats for Agents

| Format | Extension | Use Case |
|--------|-----------|----------|
| **JSON** | `.json` | Programmatic parsing, dashboards |
| **SARIF** | `.sarif` | Code scanning integration (GitHub, GitLab) |
| **JUnit** | `.xml` | CI test result aggregation |
| **Text** | `.txt` | Human review, log archives |

```bash
# JSON for programmatic use
eggsec scan example.com --output json -o results.json

# SARIF for code scanning dashboards
eggsec scan example.com --output sarif -o results.sarif

# JUnit for CI test results
eggsec scan example.com --output junit -o results.xml
```

## Human Approval Boundaries

Eggsec enforces human-in-the-loop at critical decision points:

| Operation | Risk Tier | Approval Required |
|-----------|-----------|-------------------|
| Port scanning | ActiveScan | Allowed by default |
| Fuzzing | IntrusiveFuzz | Must be enabled in config |
| Stress testing | StressTest | Must be enabled in config |
| Raw packet ops | RawPacket | Must be enabled in config |
| Auth testing | CredentialTesting | Must be enabled in config |
| Agent autonomous | AgentAutonomous | Must be enabled in config |

High-risk operations require explicit opt-in via the execution policy in the config file:

```toml
[execution_policy]
allow_intrusive_fuzzing = true
allow_stress_testing = true
```

Agents cannot bypass this. The policy is loaded from disk, not passed as a runtime argument.

## See Also

- [SAFETY.md](SAFETY.md) - Risk tiers and authorization requirements
- [scope.md](scope.md) - Scope model and enforcement details
- [lab-safety.md](lab-safety.md) - Safe use of high-risk features
- [AGENT.md](AGENT.md) - Autonomous agent configuration and operation
