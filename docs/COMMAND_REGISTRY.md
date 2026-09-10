# Command Registry

Metadata-aware command registration layer for CLI/TUI dispatch.

## Purpose

The command registry maps command IDs to dispatch metadata, enabling:
- Static inspection of all registered commands
- Descriptor generation from `OperationMetadata` instead of inline construction
- Feature-gate and category metadata for diagnostics
- All operation-backed commands use a single `RegistryBacked` dispatch path

## Architecture

```
Command Registration (static, inspectable)
    │
    ├─ operation_id → OperationMetadata (canonical policy metadata)
    │   └─ display_name, feature derived from metadata (construction test enforced)
    ├─ command-specific fields (category, dispatch_mode, cli_interactive_only)
    └─ descriptor builder → OperationDescriptor from metadata

Routing Contract (handle_command, Phase 1 single owner)
    │
    ├─ route_for_commands(cmd) classify once → CommandRoute
    │   (Operation | Multiplexer | Helper | Lifecycle; aliases resolved)
    ├─ operation-backed → canonical request → evaluate_and_enforce → execute_approved
    │   (same single executor owner as TUI/daemon)
    └─ helper/lifecycle → explicit non-operation paths (never forced into catalog)
```

## Command Categories

| Category | Description |
|----------|-------------|
| `SideEffectingNetwork` | Network operations requiring enforcement (scans, fuzz, stress) |
| `LocalFileDomain` | Local file or domain-specific operations (DB, mobile, reports) |
| `PassiveAnalytical` | Read-only analysis (explain, AI analyze) |
| `ConfigOutputHelper` | Configuration, help, diagnostics (config, doctor, plan) |
| `FrontendServer` | Server daemons (REST, MCP, gRPC, agent) |
| `LegacySpecial` | Commands with no metadata or unique dispatch needs |

## Registered Commands

### Registry-backed (operation-backed)

All operation-backed commands use `RegistryBacked` dispatch: `OperationMetadata` → `describe_from_registry()` → `EnforcementContext` → canonical dispatcher. There is no permanent `LegacyWrapped` dispatch mode.

| Command ID | Operation ID | Category | Feature | CLI Interactive Only | TUI Visible |
|-----------|-------------|----------|---------|:--------------------:|:-----------:|
| `recon` | `recon` | SideEffectingNetwork | — | No | Yes |
| `scan-ports` | `scan-ports` | SideEffectingNetwork | — | No | Yes |
| `scan-endpoints` | `scan-endpoints` | SideEffectingNetwork | — | No | Yes |
| `fingerprint` | `fingerprint` | SideEffectingNetwork | — | No | Yes |
| `scan` | `pipeline` | SideEffectingNetwork | — | No | Yes |
| `resume` | `pipeline` | SideEffectingNetwork | — | No | Yes |
| `fuzz` | `fuzz` | SideEffectingNetwork | — | No | Yes |
| `waf` | `waf-detect` | SideEffectingNetwork | — | No | Yes |
| `waf-stress` | `waf-stress` | SideEffectingNetwork | — | No | Yes |
| `graphql` | `graphql` | SideEffectingNetwork | — | No | Yes |
| `oauth` | `oauth` | SideEffectingNetwork | — | No | Yes |
| `auth-test` | `auth-test` | SideEffectingNetwork | — | No | Yes |
| `load` | `load-test` | SideEffectingNetwork | — | No | Yes |
| `stress` | `stress-test` | SideEffectingNetwork | `stress-testing` | No | Yes |
| `packet` | `packet` | SideEffectingNetwork | `packet-inspection` | No | Yes |
| `icmp` | `packet` | SideEffectingNetwork | `packet-inspection` | No | Yes |
| `traceroute` | `packet` | SideEffectingNetwork | `packet-inspection` | No | Yes |
| `nse` | `nse` | SideEffectingNetwork | `nse` | No | Yes |
| `hunt` | `hunt` | SideEffectingNetwork | `advanced-hunting` | No | Yes |
| `evasion` | `evasion` | SideEffectingNetwork | `evasion` | No | Yes |
| `postex` | `postex` | SideEffectingNetwork | `postex` | No | Yes |
| `c2` | `c2` | SideEffectingNetwork | `c2` | No | Yes |
| `proxy-intercept` | `proxy-intercept` | SideEffectingNetwork | `web-proxy` | No | Yes |
| `wireless` | `wireless` | SideEffectingNetwork | `wireless` | No | Yes |
| `wireless-deauth` | `wireless-deauth` | SideEffectingNetwork | `wireless-advanced` | No | Yes |
| `browser` | `browser` | SideEffectingNetwork | `headless-browser` | No | Yes |
| `mobile` | `mobile-static` | LocalFileDomain | `mobile` | No | Yes |
| `mobile-dynamic` | `mobile-dynamic` | LocalFileDomain | `mobile-dynamic` | No | Yes |
| `db` | `db-pentest` | LocalFileDomain | `db-pentest` | No | Yes |

### CLI-helper interactive only (not TUI, not programmatic)

`cli_interactive_only: true` — intended for direct CLI/operator invocation
only. Hidden from TUI tabs and not exposed via MCP/REST/gRPC/agent. The flag
does **not** mean "all human interactive surfaces"; manual operator actions
with TUI tabs use `tui_visible`.

| Command ID | Operation ID | Category | Feature | CLI Interactive Only | TUI Visible |
|-----------|-------------|----------|---------|:--------------------:|:-----------:|
| `plan` | (none) | ConfigOutputHelper | — | Yes | No |
| `preflight` | (uses metadata lookup) | ConfigOutputHelper | — | Yes | No |
| `ci` | (none) | ConfigOutputHelper | — | Yes | No |
| `config` | (none) | ConfigOutputHelper | — | Yes | No |
| `doctor` | (none) | ConfigOutputHelper | — | Yes | No |
| `policy-explain` | (none) | PassiveAnalytical | — | Yes | No |
| `scope-explain` | (none) | PassiveAnalytical | — | Yes | No |
| `ai-analyze` | (none) | PassiveAnalytical | `ai-integration` | Yes | No |
| `serve` | (none) | FrontendServer | `rest-api` | No | No |
| `mcp-serve` | (none) | FrontendServer | `rest-api` | No | No |
| `agent` | (none) | FrontendServer | `rest-api` | No | No |
| `grpc` | (none) | FrontendServer | `grpc-api` | No | No |
| `cluster` | (none) | FrontendServer | — | No | No |
| `remote` | (none) | FrontendServer | — | No | No |
| `exec` | (none) | FrontendServer | — | No | No |
| `report` | (none) | LocalFileDomain | — | Yes | No |
| `vuln` | (none) | ConfigOutputHelper | — | Yes | No |
| `storage` | (none) | LocalFileDomain | `database` | Yes | No |
| `sbom` | (none) | LocalFileDomain | `sbom` | Yes | No |
| `notify` | (none) | ConfigOutputHelper | — | Yes | No |

The full server-lifecycle group (`serve`, `mcp-serve`, `agent`, `grpc`,
`cluster`, `remote`, `exec`) is `cli_interactive_only: false` because the CLI
operator uses these to launch the daemon, not to interactively invoke them in
the helper sense.

## Visibility & Surface Fields

Each `CommandRegistration` carries visibility and dispatch fields that
classify how it integrates with the CLI/TUI/programmatic surfaces. These are
**metadata, not authorization**; all side-effecting commands still flow
through `EnforcementContext::evaluate()` before execution.

| Field | Type | Meaning |
|-------|------|---------|
| `cli_visible` | `bool` | Whether this command appears as a CLI subcommand or in help output. |
| `tui_visible` | `bool` | Whether this command appears as a TUI tab action. |
| `programmatic_visible` | `bool` | Whether this command may be exposed through MCP/REST/gRPC/agent. |
| `cli_interactive_only` | `bool` | Whether the command is intended for **direct CLI/operator invocation only**. CLI helper/config/report-style commands (e.g. `doctor`, `plan`, `preflight`, `config`, `report`) are `cli_interactive_only: true`; they are not TUI-visible and not programmatic. **This flag does not apply to all human-interactive surfaces** — TUI manual actions use `tui_visible`, not `cli_interactive_only`. |
| `dispatch_mode` | `CommandDispatchMode` | See below. |

Invariants enforced by `crates/eggsec/tests/command_registry.rs`:
- `cli_interactive_only → !programmatic_visible`
- `cli_interactive_only → !tui_visible`
- `HelperOnly → cli_interactive_only`
- `ServerLifecycle → !tui_visible && !cli_interactive_only`
- `RegistryBacked → operation_id.is_some()`

## CommandDispatchMode

Each `CommandRegistration` carries a `dispatch_mode: CommandDispatchMode` field that classifies how the command is dispatched:

| Variant | Description |
|---------|-------------|
| `RegistryBacked` | Descriptor/execution path uses registry metadata via `OperationMetadata` → `describe_from_registry()` → `EnforcementContext` → canonical dispatcher. All operation-backed commands. |
| `CatalogOnly` | Listed for discoverability but never dispatched (catalog entries). |
| `ServerLifecycle` | Server daemon lifecycle command (`serve`, `mcp-serve`, `agent`, `grpc`, `cluster`, `remote`, `exec`). |
| `HelperOnly` | Read-only helper/diagnostic (`config`, `doctor`, `plan`, `preflight`, `ci`, `report`, `vuln`, `storage`, `sbom`, `notify`, `policy-explain`, `scope-explain`, `ai-analyze`). |

## File Locations

| File | Purpose |
|------|---------|
| `crates/eggsec/src/commands/registry.rs` | Registry types and static entries |
| `crates/eggsec/src/commands/mod.rs` | Re-exports |
| `crates/eggsec/src/commands/handlers/mod.rs` | Dispatch bridge integration |
| `crates/eggsec/tests/command_registry.rs` | Registry consistency tests |
| `crates/eggsec/src/config/policy_catalog.rs` | `OperationMetadata` (canonical source) |
