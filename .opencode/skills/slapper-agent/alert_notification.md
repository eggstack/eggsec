---
name: alert_notification
description: "Quick alert and webhook notification commands"
triggers:
  - alert send
  - notify
  - webhook
metadata:
  category: notifications
  tools: [notify, alert]
  scope: external
---

## Overview
Slapper provides quick alert commands for sending notifications via webhooks. This skill covers the alert command for immediate notifications.

## Usage

### Send Quick Alert
Send a quick alert message:
```bash
slapper alert "Found XSS vulnerability" --severity high
slapper alert "Scan complete: 5 findings" --severity medium
```

### Webhook Notifications
Configure and test webhook notifications:
```bash
slapper notify send --webhook https://hooks.example.com/notify --message "Alert"
slapper notify test --webhook https://hooks.example.com/test
```

## Triggers
- `alert` - Quick alert command
- `notify send` - Send webhook notification
- `notify test` - Test webhook configuration
- Webhook alert routing