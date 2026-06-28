# ExecutionSurface

`ExecutionSurface` (Phase 2 of dual-mode enforcement) is the single source of truth for caller-origin-to-enforcement-profile mapping.

## Quick Reference

| Surface | Profile | Manual Override | Label |
|---------|---------|-----------------|-------|
| `CliManual` | `ManualPermissive` | Yes | CLI manual |
| `TuiManual` | `ManualPermissive` | Yes | TUI manual |
| `CliManualStrict` | `ManualGuarded` | No | CLI manual strict |
| `TuiManualStrict` | `ManualGuarded` | No | TUI manual strict |
| `McpServer` | `McpStrict` | No | MCP server |
| `SecurityAgent` | `AgentStrict` | No | Security agent |
| `Ci` | `CiStrict` | No | CI |
| `RestApi` | `McpStrict` (placeholder) | No | REST API |

## Usage

```rust
use eggsec::config::{ExecutionSurface, EnforcementContext};

let surface = ExecutionSurface::CliManual;
let enforcement = EnforcementContext::for_surface(surface, policy, loaded_scope);
```

## Key Methods

- `profile()` - Derive `ExecutionProfile`
- `is_manual()` - True for CLI/TUI surfaces
- `is_agent_controlled()` - True for MCP/Agent/CI/REST
- `honors_manual_override()` - True only for permissive manual surfaces
- `requires_explicit_manifest_for_networked()` - True for automated surfaces
- `label()` - Human-readable name
