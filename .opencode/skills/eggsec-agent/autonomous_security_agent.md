---
name: autonomous_security_agent
description: "Configuration and operation of the Eggsec autonomous security agent"
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
  - task scheduling
  - task status
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
| Enforcement | Per-scan risk/capability mapping (`agent/enforcement.rs`); `EnforcementContext::evaluate()` is the mandatory pre-dispatch gate |
| TargetPortfolio | Multi-target configuration and scheduling |
| LongitudinalMemory | Persistent scan history and pattern detection |
| AlertRouter | Route alerts to configured channels |
| EventHandler | Custom event processing hooks |
| TaskScheduler | Multi-agent task queue (`crates/eggsec-agent/src/scheduler.rs`) |
| LifecycleManager | Agent health and lifecycle (`crates/eggsec-agent/src/lifecycle.rs`) |

## Agent Run Modes

### Continuous Mode
```bash
eggsec agent run
eggsec agent run --portfolio /path/to/portfolio.json
```

### Single Pass Mode
```bash
eggsec agent run --once
```
Runs one scheduled scan pass over all due targets, then exits.

### With AI Integration
```bash
eggsec agent run --with-ai --ai-config /path/to/ai.toml
```

## Task Scheduling (Multi-Agent)

The tool/agents system provides task scheduling for distributed agents:

**TaskStatus Lifecycle:**
- `Pending` - Task available for agents to claim
- `Leased` - Task claimed by an agent (includes `assigned_agent_id`, `leased_until`)
- `Completed` - Task completed successfully
- `Failed` - Task failed (may retry based on `retry_count`)
- `Cancelled` - Task cancelled (cannot be leased)

**REST API Endpoints:**
- `POST /api/v1/tasks` - Create task
- `GET /api/v1/tasks` - List all tasks with status
- `POST /api/v1/tasks/{id}/lease` - Claim a task
- `POST /api/v1/tasks/{id}/result` - Submit task result

## Target Management

```bash
eggsec agent targets list
eggsec agent targets add mytarget --target https://example.com --schedule "0 0 * * *"
eggsec agent targets remove mytarget
eggsec agent targets enable mytarget
eggsec agent targets disable mytarget
```

All target commands use consistent portfolio loading (not `TargetPortfolio::new()` which would discard state).

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
memory_dir = "~/.config/eggsec/memory"
poll_interval_secs = 60

[[agent.alert_channels]]
type = "webhook"
url = "https://hooks.example.com/security"
secret = "your-secret-key"
```

## Callback URL Security

Agent callback URLs are validated for SSRF protection:
- Only `http` and `https` schemes allowed
- No embedded credentials (`user:pass@`)
- Rejects loopback (127.x.x.x), private (10.x, 172.16-31.x, 192.168.x), link-local (169.254.x), multicast, unspecified IPs
- DNS resolution checked for hostname-based URLs

## Cron Schedule Format

Uses standard cron expression: `minute hour day month weekday`

| Example | Schedule |
|---------|----------|
| `0 0 * * *` | Daily at midnight |
| `0 */6 * * *` | Every 6 hours |
| `0 0 * * 0` | Weekly on Sunday |
| `*/15 * * * *` | Every 15 minutes |

## Triggers

Keywords: agent, autonomous, scheduled, monitor, portfolio, target, alert, webhook, memory, cron, schedule, run, continuous, security monitoring, task, lease, multi-agent

## Best Practices

1. Start with `--once` to test configurations
2. Use appropriate poll intervals (not too frequent)
3. Configure alert channels before enabling monitoring
4. Set up baselines for longitudinal comparison
5. Review scan history regularly for patterns
6. Use task leasing for multi-agent coordination (not direct dispatch)