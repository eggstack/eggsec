---
name: config_management
description: "Configuration validation and management commands"
triggers:
  - config validate
  - config show
  - config
metadata:
  category: configuration
  tools: [config]
  scope: local
---

## Overview
Slapper provides configuration validation and display commands for managing the tool's settings. These commands help verify configuration files and inspect effective settings.

## Usage

### Validate Configuration
Verify a configuration file is valid:
```bash
slapper config validate
slapper config validate --config /path/to/config.toml
```

### Show Effective Configuration
Display the current effective configuration:
```bash
slapper config show
```

## Triggers
- `config validate` - Validate configuration file
- `config show` - Display effective configuration
- Configuration management