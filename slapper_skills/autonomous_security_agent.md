---
name: autonomous_security_agent
description: "Configuration and operation of the Slapper autonomous security agent"
triggers:
  - agent
  - autonomous
  - scheduled scan
  - monitoring
  - portfolio
  - target management
  - cron
  - schedule
  - alert
  - webhook
  - memory
  - longitudinal
metadata:
  category: agent
  tools: [agent]
  scope: targets
---

## Overview

The autonomous security agent continuously monitors configured targets, executes scheduled security scans, maintains longitudinal memory of findings, and routes alerts to configured channels.

## Architecture Components

| Component | Purpose |
|-----------|---------|
| Agent | Main event loop and orchestration |
| TargetPortfolio | Multi-target configuration and scheduling |
| LongitudinalMemory | Persistent scan history and pattern detection |
| AlertRouter | Route alerts to configured channels |
| EventHandler | Custom event processing hooks |

## Usage

### Run Agent (continuous)

```bash
slapper agent run
slapper agent run --portfolio /path/to/portfolio.json
```

### Run Once (single scan)

```bash
slapper agent run --once
```

### With AI Integration

```bash
slapper agent run --with-ai --ai-config /path/to/ai.toml
```

### Target Management

```bash
slapper agent targets list
slapper agent targets add mytarget --target https://example.com --schedule "0 0 * * *"
slapper agent targets remove mytarget
slapper agent targets enable mytarget
slapper agent targets disable mytarget
```

### Skill Management

```bash
slapper agent skills list
slapper agent skills load /path/to/skills/
slapper agent skills show dns_reconnaissance
```

### Agent Status

```bash
slapper agent status
```

## Portfolio Configuration

Create `portfolio.json`:

```json
{
  "version": "1.0",
  "targets": {
    "example-com": {
      "target": "https://example.com",
      "target_type": "url",
      "priority": "high",
      "schedule": "0 0 * * *",
      "alert_channels": ["webhook"],
      "enabled": true
    }
  }
}
```

## Alert Configuration

```toml
[agent]
memory_dir = "~/.config/slapper/memory"
poll_interval_secs = 60

[[agent.alert_channels]]
type = "webhook"
url = "https://hooks.example.com/security"
secret = "your-secret-key"
```

## Cron Schedule Format

Uses standard cron expression: `minute hour day month weekday`

| Example | Schedule |
|---------|----------|
| `0 0 * * *` | Daily at midnight |
| `0 */6 * * *` | Every 6 hours |
| `0 0 * * 0` | Weekly on Sunday |
| `*/15 * * * *` | Every 15 minutes |

## Triggers

Keywords: agent, autonomous, scheduled, monitor, portfolio, target, alert, webhook, memory, cron, schedule, run, continuous, security monitoring

## Best Practices

1. Start with `--once` to test configurations
2. Use appropriate poll intervals (not too frequent)
3. Configure alert channels before enabling monitoring
4. Set up baselines for longitudinal comparison
5. Review scan history regularly for patterns