# Autonomous Security Agent

The Slapper autonomous agent provides continuous security monitoring, scheduled assessments, and AI-guided security testing for your infrastructure.

## Overview

The agent system consists of several components:

| Component | Purpose |
|-----------|---------|
| **Agent** | Main event loop that orchestrates all operations |
| **TargetPortfolio** | Manages configured targets and their schedules |
| **LongitudinalMemory** | Persistent storage of scan history and patterns |
| **AlertRouter** | Routes alerts to configured channels (webhooks) |
| **SkillRegistry** | Indexes and matches skills for agent behavior |

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
mkdir -p ~/.config/slapper

# Create memory directory (for longitudinal storage)
mkdir -p ~/.config/slapper/memory

# Create skills directory
mkdir -p ~/.config/slapper/skills
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

Add webhook configuration to `~/.config/slapper/config.toml`:

```toml
[agent]
memory_dir = "~/.config/slapper/memory"
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
./slapper agent run --portfolio ~/.config/slapper/portfolio.json

# With AI integration
./slapper agent run --portfolio ~/.config/slapper/portfolio.json --with-ai --ai-config ~/.config/slapper/ai.toml

# Run once (useful for testing)
./slapper agent run --portfolio ~/.config/slapper/portfolio.json --once

# Custom memory directory
./slapper agent run --portfolio ~/.config/slapper/portfolio.json --memory-dir /var/lib/slapper/memory
```

## CLI Commands

### Agent Management

```bash
# Show agent status
./slapper agent status

# Run agent (default or explicit)
./slapper agent run
./slapper agent run --once
./slapper agent run --with-ai --ai-config /path/to/ai.toml
```

### Target Management

```bash
# List all targets
./slapper agent targets list

# Add a new target
./slapper agent targets add mytarget \
  --target https://example.com \
  --schedule "0 0 * * *" \
  --priority high

# Remove a target
./slapper agent targets remove mytarget

# Enable/disable a target
./slapper agent targets enable mytarget
./slapper agent targets disable mytarget
```

### Skills Management

```bash
# List available skills
./slapper agent skills list

# Load skills from directory
./slapper agent skills load ~/.config/slapper/skills/

# Show skill details
./slapper agent skills show dns_reconnaissance
./slapper agent skills show sql_injection_fuzzing
./slapper agent skills show waf_detection_bypass
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

The agent stores scan history in `~/.config/slapper/memory/`:

```
~/.config/slapper/memory/
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

Skills define agent capabilities using YAML frontmatter + Markdown. See `slapper_skills/` for all available skills.

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

Create `~/.config/slapper/ai.toml`:

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
./slapper agent run --with-ai --ai-config ~/.config/slapper/ai.toml

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
X-Slapper-Signature: sha256=<hmac-sha256>
X-Slapper-Timestamp: <unix-timestamp>
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
./slapper agent run --portfolio /path/to/portfolio.json --once

# Verify config
./slapper agent status
```

### Memory errors

```bash
# Check memory directory permissions
ls -la ~/.config/slapper/memory/

# Recreate if corrupted
rm -rf ~/.config/slapper/memory
mkdir ~/.config/slapper/memory
```

### AI integration fails

```bash
# Verify AI config
cat ~/.config/slapper/ai.toml

# Test AI provider connectivity
curl https://api.openai.com/v1/models
```

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
./slapper --help
./slapper agent --help

# Subcommand help
./slapper agent run --help
./slapper agent targets --help
./slapper agent skills --help
```

## See Also

- [slapper_skills/README.md](../slapper_skills/README.md) - All available skills
- [AGENTS.md](../AGENTS.md) - Developer documentation for the codebase