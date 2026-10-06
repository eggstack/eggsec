# Autonomous Security Agent

The Eggsec autonomous agent provides continuous security monitoring, scheduled assessments, and AI-guided security testing for your infrastructure.

## Overview

The agent system consists of several components:

| Component | Purpose |
|-----------|---------|
| **Agent** | Main event loop that orchestrates all operations |
| **TargetPortfolio** | Manages configured targets and their schedules |
| **LongitudinalMemory** | Persistent storage of scan history and patterns |
| **AlertRouter** | Routes alerts to configured channels (webhooks) |
| **SkillRegistry** | Indexes and matches skills for agent behavior |

## Longitudinal Memory

The agent persists scan history through `agent::memory::LongitudinalMemory`
(`crates/eggsec/src/agent/memory.rs`). It creates two subdirectories under the
configured `--memory-dir`:

```
~/.config/eggsec/memory/
├── targets/
│   ├── example.com.json      # Scan history per target
│   └── api.example.com.json
└── patterns/
    └── detected.json         # Pattern analysis across targets
```

The memory layer also derives a per-target snapshot path and an
"already alerted findings" path from the same root. There is no separate
`runtime/` tree, `agent-state.json`, or `scans/` directory.

### Atomic Writes

Every memory file is written through an atomic temporary-file-and-rename flow:
each write targets a concrete JSON file, creates a sibling `*.tmp` file, flushes
it, and then renames it over the destination. Paths that do not resolve to a
file name are rejected with context instead of panicking, so persistence
failures are reported cleanly by the agent.

## Graceful Shutdown

The agent handles shutdown signals (`SIGTERM`, `SIGINT`) gracefully:

1. **Stop accepting new scans** - No new targets are picked up
2. **Wait for active scans** - Running scans complete or hit their timeout
3. **Persist state** - All runtime state is flushed to disk
4. **Flush alerts** - Pending alerts are sent before exit
5. **Close connections** - HTTP clients, database connections, and file handles are closed

On restart, the agent:
- Loads persisted memory (per-target scan history and detected patterns)
- Re-derives cooldown state from the last recorded scan timestamp

## Scan Budgets and Cooldowns

### Operational Constraints

Per-run throttling is expressed as `agent::constraints::OperationalConstraints`
(`crates/eggsec/src/agent/constraints.rs:167`), not as a per-scan budget block:

```rust
pub struct OperationalConstraints {
    pub off_peak_config: OffPeakConfig,
    pub alert_routing: AlertRoutingRules,
    pub do_not_do_list: DoNotDoList,
    pub rate_limit_budget: Option<usize>,
    pub require_approval_for: Vec<String>,
    pub max_concurrent_scans: Option<usize>,
    pub per_target_cooldown_secs: Option<u64>,
}
```

Every field is `Option`-or-defaulted; the builder methods (`with_off_peak_config`,
`with_alert_routing`, `with_do_not_do_list`, `with_max_concurrent_scans`,
`with_per_target_cooldown`) are the intended construction path.

Per-scan resource budgets (max duration, max findings, max payloads) live with
the scan depth / probe risk configuration, not in the agent constraints -- see
`architecture/runtime.md`.

**Enforcement (2026-06-28, Phase 3):** `handle_agent()` requires explicit scope manifest (`LoadedScope::is_explicit_manifest()`) and refuses to run without it. As defense-in-depth, `handle_agent` defensively rebuilds `EnforcementContext::agent_strict` from the current policy and loaded scope instead of trusting the incoming `CommandContext` enforcement. `Agent::new()` validates that `config.enforcement` is `AgentStrict` and rejects `ManualPermissive`, `ManualGuarded`, or other non-agent profiles (the `None` case is allowed for test-only construction). Per-scan `enforcement.evaluate` (central boundary: provenance, DenialClass downgrade for ManualPermissive only, positive capability checks) is re-evaluated immediately before dispatch in `execute_scan_with_depth` (in addition to startup gating). If `enforced_dispatcher` is present but `ApprovedOperation` is missing at dispatch time, agent returns a hard invariant error (no raw dispatch fallback). Manual override flags are never honored by agent execution.

Verified at `crates/eggsec/src/commands/handlers/agent.rs:35` (profile check),
`:54` (explicit manifest), `:61` (rebuild), and
`crates/eggsec/src/agent/mod.rs:244` (`Agent::new`).

> For MCP and autonomous-agent execution, `EnforcementContext::evaluate()` is the mandatory pre-dispatch gate. Scope provenance must come from `LoadedScope`; raw `Scope` is not sufficient for automated execution.

### Cooldowns

Cooldown is a single per-target interval, not a per-scan-type table. When
`per_target_cooldown_secs` is set, the scheduler skips a target whose elapsed
time since the last scan is still below the threshold
(`crates/eggsec/src/agent/mod.rs:805`):

```rust
if let Some(cooldown) = self.constraints.as_ref().and_then(|c| c.per_target_cooldown_secs) {
    // "Skipping {target} - per-target cooldown ({n}s remaining)"
}
```

## Installation

### Build Requirements

```bash
# The workspace root is a virtual manifest -- build the CLI package.
# Agent requires the rest-api feature
cargo build --release -p eggsec-cli --features rest-api

# With AI integration (recommended for smart scanning)
cargo build --release -p eggsec-cli --features "rest-api ai-integration"

# Install from source
cargo install --path crates/eggsec-cli --features rest-api
```

`--features` must be applied to the `-p eggsec-cli` package; a bare
`cargo build --features ...` at the workspace root does not resolve. `full`
aggregates are curated rather than exhaustive -- see `docs/FEATURE_MATRIX.md`.

### Directory Setup

```bash
# Create config directory
mkdir -p ~/.config/eggsec

# Create memory directory (for longitudinal storage)
mkdir -p ~/.config/eggsec/memory

# Create skills directory
mkdir -p ~/.config/eggsec/skills
```

## Quick Start

### 1. Create a Portfolio

A portfolio defines the targets to monitor:

```json
{
  "version": "1.0",
  "targets": {
    "my-api": {
      "target": "https://api.example.com",
      "target_type": "url",
      "priority": "high",
      "schedule": "0 0 * * *",
      "alert_channels": ["security-webhook"],
      "enabled": true
    },
    "internal-dashboard": {
      "target": "https://dashboard.internal.example.com",
      "target_type": "url",
      "priority": "critical",
      "schedule": "*/15 * * * *",
      "alert_channels": ["security-webhook", "pagerduty"],
      "enabled": true
    }
  }
}
```

### 2. Configure Alerts

Add webhook configuration to `~/.config/eggsec/config.toml`. `alert_channels`
is a top-level table keyed by channel name:

```toml
[alert_channels.security-webhook]
type = "webhook"
url = "https://hooks.example.com/security/alerts"
secret = "your-hmac-secret"

[alert_channels.pagerduty]
type = "pagerduty"
url = "https://events.pagerduty.com/v2/enqueue"
service_key = "your-pagerduty-key"
```

Per-run agent options such as `--portfolio`, `--memory-dir` and `--poll-interval`
are CLI flags (defaulting to `~/.config/eggsec/memory` and 60 seconds), not
config.toml keys.

### 3. Run the Agent

```bash
# Continuous monitoring
eggsec agent --portfolio ~/.config/eggsec/portfolio.json run

# With AI integration
eggsec agent --portfolio ~/.config/eggsec/portfolio.json --with-ai \
  --ai-config ~/.config/eggsec/ai.toml run

# Run once (useful for testing)
eggsec agent --portfolio ~/.config/eggsec/portfolio.json run --once

# Custom memory directory and poll interval
eggsec agent --portfolio ~/.config/eggsec/portfolio.json \
  --memory-dir /var/lib/eggsec/memory \
  --poll-interval 300 run
```

`--portfolio`, `--memory-dir`, `--poll-interval`, `--with-ai` and `--ai-config`
are **flags on `eggsec agent` itself**, not on the `run` subcommand
(`crates/eggsec/src/cli/agent.rs:19`). The `run` subcommand only accepts
`--once`.

## CLI Commands

### Agent Management

```bash
# Show agent status
eggsec agent status

# Run agent (default or explicit)
eggsec agent
eggsec agent run --once
eggsec agent --with-ai --ai-config /path/to/ai.toml run
```

### Target Management

```bash
# List all targets
eggsec agent targets list

# Add a new target
eggsec agent targets add mytarget https://example.com \
  --schedule "0 0 * * *" \
  --priority high

# Update a target
eggsec agent targets update mytarget --priority critical --scan-depth deep

# Remove a target
eggsec agent targets remove mytarget

# Enable/disable a target
eggsec agent targets enable mytarget
eggsec agent targets disable mytarget
```

For `targets add`, the target URL is a **positional** argument (`target:`), not
`--target`; the accepted flags are `--target-type` (default `url`), `--schedule`
and `--priority` (default `normal`).

### Skills Management

```bash
# List available skills
eggsec agent skills list

# Load skills from directory
eggsec agent skills load ~/.config/eggsec/skills/

# Show skill details
eggsec agent skills show dns_reconnaissance
eggsec agent skills show sql_injection
eggsec agent skills show waf_detection_bypass
```

## Configuration Reference

### Portfolio Schema

```json
{
  "version": "1.0",
  "targets": {
    "<target-id>": {
      "target": "https://example.com",
      "target_type": "url | host | cidr",
      "priority": "low | normal | high | critical",
      "schedule": "<cron-expression>",
      "alert_channels": ["<channel-name>"],
      "last_scan": "<ISO8601-timestamp>",
      "scan_history": [],
      "baseline_findings": ["<finding-id>"],
      "enabled": true,
      "scan_depth": "shallow | deep",
      "off_peak_window": { "start_hour": 0, "end_hour": 6, "timezone": "UTC" },
      "scope": { "allowed_targets": [] }
    }
  }
}
```

`scan_depth`, `off_peak_window` and `scope` are the optional fields the old
schema omitted; `off_peak_window` is only honoured when the matching
`OffPeakConfig` constraint is set (`crates/eggsec/src/agent/portfolio.rs:117`).

### Cron Schedule Format

| Expression | Description |
|------------|-------------|
| `0 0 * * *` | Daily at midnight |
| `0 */6 * * *` | Every 6 hours |
| `0 0 * * 0` | Weekly on Sunday |
| `*/15 * * * *` | Every 15 minutes |
| `0 9-17 * * 1-5` | Business hours, weekdays |

### Alert Channel Types

`alert_channels` is a **top-level** key in `config.toml` holding a map of named
channels, each tagged by `type`
(`AlertChannelsConfig { channels: FxHashMap<String, AlertChannelConfigEntry> }`,
`crates/eggsec/src/config/settings.rs:20`). There is no `[agent]` section.
Supported `type` values are `webhook`, `email`, `slack` and `pagerduty`.

```toml
# Webhook alert
[alert_channels.my-webhook]
type = "webhook"
url = "https://hooks.example.com/alerts"
secret = "hmac-secret"

[alert_channels.security-team]
type = "email"
smtp_host = "smtp.example.com"
smtp_port = 587
from = "eggsec@example.com"
to = ["security@example.com"]
```

Channel URLs are validated at config load (`http://`/`https://` for webhooks;
non-zero port and non-empty `from`/`to` for email).

## Memory Structure

See [Longitudinal Memory](#longitudinal-memory) above -- the agent writes only
`targets/` and `patterns/` under the memory directory.

## Skills

Skills define agent capabilities using YAML frontmatter + Markdown. They are
loaded from `~/.config/eggsec/skills` (the default in
`crates/eggsec/src/commands/handlers/agent.rs:360`) or from an explicit path
passed to `eggsec agent skills load <path>`.

### Skill Format

```yaml
---
name: skill_name
description: "Brief description of the skill"
triggers:
  - trigger keyword
  - another trigger
metadata:
  category: recon | scanning | fuzzing | api_testing | agent
  tools: [tool1, tool2]
  scope: targets
---

## Overview
Detailed description of what this skill does.

## Capabilities
- Capability 1
- Capability 2

## Usage
```bash
example command
```

## Triggers
Keywords that activate this skill
```

### Available Skills

| Category | Skills |
|----------|--------|
| **Reconnaissance** | dns_reconnaissance, ssl_tls_analysis, subdomain_enumeration, web_search_integration |
| **Scanning** | port_scanning, endpoint_discovery |
| **Fuzzing** | sql_injection, cross_site_scripting, path_traversal, ssrf, command_injection, ldap_injection |
| **API Testing** | graphql_security, oauth_oidc_testing, cors_security, authentication_security (CLI `auth-test` defense-lab only; agents/MCP subject to strict `CredentialTesting` enforcement + explicit `Capability::CredentialTesting`; results local only; see architecture/auth.md) |
| **WAF** | waf_detection_bypass |
| **Load Testing** | http_load_testing |
| **Compliance** | security_compliance_checks |
| **Pipeline** | security_assessment_pipeline |
| **Agent** | autonomous_security_agent |

## AI Integration

### Configuration

Create `~/.config/eggsec/ai.toml`. The schema is `AiConfig`
(`crates/eggsec/src/config/settings.rs:204`):

```toml
provider = "openai"           # or "ollama" for local
model = "gpt-4"              # or "llama3" for Ollama
base_url = "https://api.openai.com/v1"
api_key = "sk-..."
max_tokens = 4096
temperature = 0.2
max_payloads = 50             # default 50
max_bypasses = 10             # default 10

# Optional for Ollama
# provider = "ollama"
# model = "llama3"
# base_url = "http://localhost:11434/v1"
```

### Usage with AI

```bash
# Run with AI analysis
eggsec agent --with-ai --ai-config ~/.config/eggsec/ai.toml run

# AI features:
# - Adaptive scan strategy based on findings
# - Smart payload selection
# - WAF bypass recommendations
# - Vulnerability prioritization
```

## Webhook Alerts

### Alert Format

When a webhook alert is triggered, the agent POSTs a payload with the alert
nested under a single `alert` key (`crates/eggsec/src/agent/alerts/routing.rs:240`):

```json
{
  "alert": {
    "severity": "critical | high | medium | low | info",
    "title": "Critical finding on example.com",
    "message": "SQL injection vulnerability detected",
    "target": "https://example.com/api",
    "finding_ids": ["sqli-001", "sqli-002"],
    "recommended_actions": [
      "Review and patch vulnerable code",
      "Implement input validation",
      "Use parameterized queries"
    ],
    "timestamp": "2024-01-15T10:30:00Z"
  }
}
```

There is no `version` or `alert_id` field; `timestamp` is generated at send time
as RFC 3339.

### HMAC Verification

When the channel sets `secret`, the request carries an HMAC-SHA256 signature
header:

```http
X-Signature-256: sha256=<hex-hmac-sha256>
```

There is no separate timestamp header, and **no timestamp is mixed into the
signature** -- the MAC is computed over the canonical JSON serialization of the
`{"alert": {...}}` payload object (the same `serde_json::json!` value that is
sent), not over the raw request body
(`crates/eggsec/src/agent/alerts/routing.rs:253-261`).

Verify in your webhook handler by re-serializing the received JSON canonically:

```python
import hmac
import hashlib
import json

def verify_signature(payload: dict, signature: str, secret: str) -> bool:
    canonical = json.dumps(payload, separators=(",", ":"))
    expected = hmac.new(
        secret.encode(),
        canonical.encode(),
        hashlib.sha256
    ).hexdigest()
    return hmac.compare_digest(f"sha256={expected}", signature)
```

Any `headers` configured on the webhook channel are appended after the
signature header.

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         CLI / TUI / API                              │
└─────────────────────────────────────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────┐
│                        Agent Core (agent/)                           │
│  ┌─────────────┐  ┌──────────────┐  ┌─────────────┐  ┌────────────┐ │
│  │   Agent     │  │ TargetPortfolio│ │ Longitudinal │ │   Alert    │ │
│  │   EventLoop │  │              │  │   Memory     │ │   Router   │ │
│  └─────────────┘  └──────────────┘  └─────────────┘  └────────────┘ │
└─────────────────────────────────────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────┐
│                     Tool Layer (tool/)                              │
│  ┌────────┐  ┌──────────┐  ┌────────┐  ┌───────┐  ┌────────────┐  │
│  │ Recon  │  │ Scanner  │  │ Fuzzer │  │  WAF  │  │   Search   │  │
│  └────────┘  └──────────┘  └────────┘  └───────┘  └────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
```

## Troubleshooting

### Agent won't start

```bash
# Check portfolio file syntax
eggsec agent --portfolio /path/to/portfolio.json run --once

# Verify config
eggsec agent status
```

### Memory errors

```bash
# Check memory directory permissions
ls -la ~/.config/eggsec/memory/

# Recreate if corrupted
rm -rf ~/.config/eggsec/memory
mkdir -p ~/.config/eggsec/memory
```

### AI integration fails

```bash
# Verify AI config
cat ~/.config/eggsec/ai.toml

# Test AI provider connectivity
curl https://api.openai.com/v1/models
```

## Defense-Lab Agent Runs

When the agent runs defense-lab profiles, it:
- Uses the profile's operation mode and risk budget
- Records policy decisions with unique IDs
- Enforces per-target cooldowns and execution budgets
- Produces structured reports with budget consumption data

## Best Practices

1. **Start with `--once`** to verify configuration before running continuously
2. **Set appropriate poll intervals** - Don't scan too frequently (15min minimum recommended)
3. **Configure alert channels** before enabling monitoring
4. **Review scan history** regularly to establish baselines
5. **Use AI integration** for adaptive scanning based on findings
6. **Store portfolios in version control** for reproducibility

## Getting Help

```bash
# General help
eggsec --help
eggsec agent --help

# Subcommand help
eggsec agent run --help
eggsec agent targets --help
eggsec agent skills --help
```

## CI/CD Integration

Eggsec integrates into CI pipelines for continuous security regression testing:

```yaml
# GitHub Actions example
- name: Security scan
  run: |
    eggsec scan "$DEPLOYED_URL" \
      --profile quick \
      --scope .eggsec/scope.toml \
      --format sarif \
      -o security-results/results.sarif

- name: Upload SARIF
  uses: github/codeql-action/upload-sarif@v3
  with:
    sarif_file: security-results/results.sarif
```

```makefile
# Makefile integration
.PHONY: security-scan
security-scan:
	eggsec scan $(TARGET_URL) \
		--profile full \
		--scope scopes/$(ENV).toml \
		--format json \
		-o reports/security-$(CI_COMMIT_SHA).json
```

`--format` accepts `json`, `html`, `csv`, `sarif` and `junit`
(`crates/eggsec/src/cli/scan.rs:371`). `-o`/`--output` is a single **file path**,
not a format name, and there is no `--output-dir` flag. `--scope` is the global
scope-file flag.

Key CI properties:
- Deterministic - same inputs produce same output structure
- Exit codes - non-zero on findings above severity threshold
- SARIF output - integrates with GitHub, GitLab, and other code scanning dashboards

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

## See Also

- [.opencode/skills/](../.opencode/skills/) - All available skills
- [AGENTS.md](../AGENTS.md) - Developer documentation for the codebase
- [SAFETY.md](SAFETY.md) - Risk tiers and authorization requirements
- [scope.md](scope.md) - Scope model and enforcement details

## Dependency Injection (Phase D, 2026-09-09)

`Agent::with_engine_services(config, services, alert_router)` injects `agent::services::AgentExecutionService` (checked-only, `AgentStrict` by construction). `Agent::new` is the composition-root shim. Tests use fakes without building the default registry. The injected executor exposes no raw dispatch.
