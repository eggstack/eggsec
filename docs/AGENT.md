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

## Runtime Status Tracking

The agent tracks runtime state for each target and scan execution.

### Runtime States

| State | Description |
|-------|-------------|
| `Idle` | Target is not currently being scanned |
| `Scanning` | Scan is actively running against the target |
| `Cooldown` | Scan completed, waiting before next scheduled scan |
| `Paused` | Target scanning is temporarily suspended |
| `Error` | Last scan failed, waiting for retry |

### State Transitions

```
Idle → Scanning → Cooldown → Idle
Idle → Scanning → Error → Idle
Idle → Paused → Idle (manual resume)
```

### Runtime State Persistence

Agent runtime state is persisted to `~/.config/eggsec/memory/runtime/`:

```
~/.config/eggsec/memory/runtime/
├── agent-state.json         # Global agent state (start time, last poll)
├── targets/
│   ├── example.com.json     # Per-target runtime state
│   └── api.example.com.json
└── scans/
    ├── scan-001.json        # Active scan state (progress, findings so far)
    └── scan-002.json
```

State is persisted on:
- State transitions (Idle → Scanning, Scanning → Cooldown)
- Periodic snapshots (every 60 seconds during active scans)
- Graceful shutdown

## Graceful Shutdown

The agent handles shutdown signals (`SIGTERM`, `SIGINT`) gracefully:

1. **Stop accepting new scans** - No new targets are picked up
2. **Wait for active scans** - Running scans complete or hit their timeout
3. **Persist state** - All runtime state is flushed to disk
4. **Flush alerts** - Pending alerts are sent before exit
5. **Close connections** - HTTP clients, database connections, and file handles are closed

On restart, the agent:
- Loads persisted runtime state
- Resumes cooldown timers from where they left off
- Skips targets that were mid-scan (treats them as errored for retry)

## Scan Budgets and Cooldowns

### Scan Budgets

Each target scan has resource budgets to prevent runaway executions:

| Budget | Default | Description |
|--------|---------|-------------|
| `max_duration_ms` | 300,000 (5 min) | Maximum scan duration |
| `max_findings` | 100 | Stop after N findings |
| `max_requests` | 1,000 | Maximum HTTP requests |
| `max_payloads` | 500 | Maximum fuzzing payloads |

Budgets can be set per-target in the portfolio:

```json
{
  "target": "https://api.example.com",
  "budgets": {
    "max_duration_ms": 600000,
    "max_findings": 50,
    "max_requests": 500
  }
}
```

### Cooldowns

After a scan completes, the target enters a cooldown period before the next scan is allowed:

| Scan Type | Default Cooldown |
|-----------|-----------------|
| Quick scan | 5 minutes |
| Full assessment | 1 hour |
| WAF testing | 30 minutes |
| Stress testing | 4 hours |
| Recon only | 15 minutes |

Cooldowns are configurable per-target:

```json
{
  "target": "https://api.example.com",
  "cooldowns": {
    "full_assessment_ms": 3600000,
    "quick_scan_ms": 300000,
    "stress_test_ms": 14400000
  }
}
```

## Installation

### Build Requirements

```bash
# Agent requires rest-api feature
cargo build --release --features "rest-api"

# With AI integration (recommended for smart scanning)
cargo build --release --features "rest-api ai-integration"

# Full features
cargo build --release --features "full"
```

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

Add webhook configuration to `~/.config/eggsec/config.toml`:

```toml
[agent]
memory_dir = "~/.config/eggsec/memory"
poll_interval_secs = 60

[[agent.alert_channels]]
type = "webhook"
name = "security-webhook"
url = "https://hooks.example.com/security/alerts"
secret = "your-hmac-secret"

[[agent.alert_channels]]
type = "webhook"
name = "pagerduty"
url = "https://events.pagerduty.com/v2/enqueue"
service_key = "your-pagerduty-key"
```

### 3. Run the Agent

```bash
# Continuous monitoring
./eggsec agent run --portfolio ~/.config/eggsec/portfolio.json

# With AI integration
./eggsec agent run --portfolio ~/.config/eggsec/portfolio.json --with-ai --ai-config ~/.config/eggsec/ai.toml

# Run once (useful for testing)
./eggsec agent run --portfolio ~/.config/eggsec/portfolio.json --once

# Custom memory directory
./eggsec agent run --portfolio ~/.config/eggsec/portfolio.json --memory-dir /var/lib/eggsec/memory
```

## CLI Commands

### Agent Management

```bash
# Show agent status
./eggsec agent status

# Run agent (default or explicit)
./eggsec agent run
./eggsec agent run --once
./eggsec agent run --with-ai --ai-config /path/to/ai.toml
```

### Target Management

```bash
# List all targets
./eggsec agent targets list

# Add a new target
./eggsec agent targets add mytarget \
  --target https://example.com \
  --schedule "0 0 * * *" \
  --priority high

# Remove a target
./eggsec agent targets remove mytarget

# Enable/disable a target
./eggsec agent targets enable mytarget
./eggsec agent targets disable mytarget
```

### Skills Management

```bash
# List available skills
./eggsec agent skills list

# Load skills from directory
./eggsec agent skills load ~/.config/eggsec/skills/

# Show skill details
./eggsec agent skills show dns_reconnaissance
./eggsec agent skills show sql_injection_fuzzing
./eggsec agent skills show waf_detection_bypass
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
      "enabled": true
    }
  }
}
```

### Cron Schedule Format

| Expression | Description |
|------------|-------------|
| `0 0 * * *` | Daily at midnight |
| `0 */6 * * *` | Every 6 hours |
| `0 0 * * 0` | Weekly on Sunday |
| `*/15 * * * *` | Every 15 minutes |
| `0 9-17 * * 1-5` | Business hours, weekdays |

### Alert Channel Types

```toml
# Webhook alert
[[agent.alert_channels]]
type = "webhook"
name = "my-webhook"
url = "https://hooks.example.com/alerts"
secret = "hmac-secret"

# Email alert (future)
[[agent.alert_channels]]
type = "email"
name = "security-team"
smtp_host = "smtp.example.com"
to = ["security@example.com"]
```

## Memory Structure

The agent stores scan history in `~/.config/eggsec/memory/`:

```
~/.config/eggsec/memory/
├── targets/
│   ├── example.com.json      # Scan history per target
│   └── api.example.com.json
├── patterns/
│   └── detected.json         # Pattern analysis
├── baselines/
│   └── example.com.json      # Baseline findings
└── cache/
    └── ai_cache.json         # AI analysis cache
```

## Skills

Skills define agent capabilities using YAML frontmatter + Markdown. See `eggsec_skills/` for all available skills.

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
| **API Testing** | graphql_security, oauth_oidc_testing, cors_security, authentication_security |
| **WAF** | waf_detection_bypass |
| **Load Testing** | http_load_testing |
| **Compliance** | security_compliance_checks |
| **Pipeline** | security_assessment_pipeline |
| **Agent** | autonomous_security_agent |

## AI Integration

### Configuration

Create `~/.config/eggsec/ai.toml`:

```toml
provider = "openai"           # or "ollama" for local
model = "gpt-4"              # or "llama3" for Ollama
base_url = "https://api.openai.com/v1"

# Optional for Ollama
# provider = "ollama"
# model = "llama3"
# base_url = "http://localhost:11434/v1"

[output]
format = "json"
path = "./reports"
```

### Usage with AI

```bash
# Run with AI analysis
./eggsec agent run --with-ai --ai-config ~/.config/eggsec/ai.toml

# AI features:
# - Adaptive scan strategy based on findings
# - Smart payload selection
# - WAF bypass recommendations
# - Vulnerability prioritization
```

## Webhook Alerts

### Alert Format

When an alert is triggered, the agent sends:

```json
{
  "version": "1.0",
  "alert_id": "uuid",
  "timestamp": "2024-01-15T10:30:00Z",
  "severity": "critical | high | medium | low | info",
  "title": "Critical finding on example.com",
  "message": "SQL injection vulnerability detected",
  "target": "https://example.com/api",
  "finding_ids": ["sqli-001", "sqli-002"],
  "recommended_actions": [
    "Review and patch vulnerable code",
    "Implement input validation",
    "Use parameterized queries"
  ]
}
```

### HMAC Verification

Webhook requests include HMAC signature for verification:

```http
X-Eggsec-Signature: sha256=<hmac-sha256>
X-Eggsec-Timestamp: <unix-timestamp>
```

Verify in your webhook handler:

```python
import hmac
import hashlib

def verify_signature(payload: bytes, signature: str, secret: str) -> bool:
    expected = hmac.new(
        secret.encode(),
        payload,
        hashlib.sha256
    ).hexdigest()
    return hmac.compare_digest(f"sha256={expected}", signature)
```

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
./eggsec agent run --portfolio /path/to/portfolio.json --once

# Verify config
./eggsec agent status
```

### Memory errors

```bash
# Check memory directory permissions
ls -la ~/.config/eggsec/memory/

# Recreate if corrupted
rm -rf ~/.config/eggsec/memory
mkdir ~/.config/eggsec/memory
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
./eggsec --help
./eggsec agent --help

# Subcommand help
./eggsec agent run --help
./eggsec agent targets --help
./eggsec agent skills --help
```

## See Also

- [.opencode/skills/](../.opencode/skills/) - All available skills
- [AGENTS.md](../AGENTS.md) - Developer documentation for the codebase