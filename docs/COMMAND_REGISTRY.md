# Command Registry

Metadata-aware command registration layer for CLI/TUI dispatch. Phase 6 of the architecture extensibility effort.

## Purpose

The command registry maps command IDs to dispatch metadata, enabling:
- Static inspection of all registered commands
- Descriptor generation from `OperationMetadata` instead of inline construction
- Feature-gate and category metadata for diagnostics
- Gradual migration from the legacy `handle_command()` match dispatch

## Architecture

```
Command Registration (static, inspectable)
    │
    ├─ operation_id → OperationMetadata (canonical policy metadata)
    ├─ feature gate → compile-time / runtime feature check
    ├─ category → CommandCategory enum
    ├─ dispatch_mode → CommandDispatchMode (RegistryBacked, LegacyWrapped, etc.)
    └─ descriptor builder → OperationDescriptor from metadata
    
Dispatch Bridge (handle_command)
    │
    ├─ registry.lookup(command_id) → CommandRegistration
    │   └─ dispatch_mode == RegistryBacked → build descriptor → evaluate_and_enforce → execute
    │   └─ dispatch_mode == LegacyWrapped → legacy handle_command() path
    └─ not registered → legacy handler fallback
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

### Registry-backed (Phase 6 pilot)

| Command ID | Operation ID | Category | Feature | Interactive Only | TUI Visible | Registry Backed |
|-----------|-------------|----------|---------|:---------------:|:-----------:|:---------------:|
| `recon` | `recon` | SideEffectingNetwork | — | No | Yes | Yes |
| `scan-ports` | `scan-ports` | SideEffectingNetwork | — | No | Yes | Yes |
| `scan-endpoints` | `scan-endpoints` | SideEffectingNetwork | — | No | Yes | Yes |
| `fingerprint` | `fingerprint` | SideEffectingNetwork | — | No | Yes | Yes |

### Legacy (not yet migrated)

| Command ID | Operation ID | Category | Feature | Interactive Only | TUI Visible | Registry Backed | Notes |
|-----------|-------------|----------|---------|:---------------:|:-----------:|:---------------:|-------|
| `scan` | `scan` (alias→scan-ports) | SideEffectingNetwork | — | No | Yes | No | Pipeline orchestrator, LegacyWrapped |
| `resume` | scan-resume | SideEffectingNetwork | — | No | Yes | No | Pipeline resume, LegacyWrapped |
| `fuzz` | `fuzz` | SideEffectingNetwork | — | No | Yes | No | Complex payload engine, LegacyWrapped |
| `waf` | `waf-detect` | SideEffectingNetwork | — | No | Yes | No | WAF detection, LegacyWrapped |
| `waf-stress` | `waf-stress` | SideEffectingNetwork | — | No | Yes | No | WAF stress tier, LegacyWrapped |
| `graphql` | `graphql` | SideEffectingNetwork | — | No | Yes | No | GraphQL fuzzer, LegacyWrapped |
| `oauth` | `oauth` | SideEffectingNetwork | — | No | Yes | No | OAuth fuzzer, LegacyWrapped |
| `auth-test` | `auth-test` | SideEffectingNetwork | — | No | Yes | No | Multi-test suite, LegacyWrapped |
| `load` | load-test | SideEffectingNetwork | — | No | Yes | No | Load testing, LegacyWrapped |
| `stress` | stress-test | SideEffectingNetwork | `stress-testing` | No | Yes | No | LegacyWrapped |
| `packet` | packet | SideEffectingNetwork | `packet-inspection` | No | Yes | No | LegacyWrapped |
| `icmp` | icmp | SideEffectingNetwork | `stress-testing` | No | Yes | No | LegacyWrapped |
| `traceroute` | traceroute | SideEffectingNetwork | `stress-testing` | No | Yes | No | LegacyWrapped |
| `nse` | `nse` | SideEffectingNetwork | `nse` | No | Yes | No | LegacyWrapped |
| `hunt` | `hunt` | SideEffectingNetwork | `advanced-hunting` | No | Yes | No | LegacyWrapped |
| `evasion` | evasion | SideEffectingNetwork | `evasion` | No | Yes | No | LegacyWrapped |
| `postex` | postex | SideEffectingNetwork | `postex` | No | Yes | No | LegacyWrapped |
| `c2` | `c2` | SideEffectingNetwork | `c2` | No | Yes | No | LegacyWrapped |
| `proxy-intercept` | `proxy-intercept` | SideEffectingNetwork | `web-proxy` | No | Yes | No | LegacyWrapped |
| `wireless` | `wireless` | SideEffectingNetwork | `wireless` | No | Yes | No | LegacyWrapped |
| `browser` | `browser` | SideEffectingNetwork | `headless-browser` | No | Yes | No | LegacyWrapped |
| `mobile` | mobile-static/mobile-dynamic | LocalFileDomain | `mobile` | No | Yes | No | LegacyWrapped |
| `db` | `db-pentest` | LocalFileDomain | `db-pentest` | No | Yes | No | LegacyWrapped |
| `plan` | (none) | ConfigOutputHelper | — | Yes | No | No | HelperOnly |
| `preflight` | (uses metadata lookup) | ConfigOutputHelper | — | Yes | No | No | Advisory only, HelperOnly |
| `ci` | (none) | ConfigOutputHelper | — | Yes | No | No | Passive quality gate, HelperOnly |
| `config` | (none) | ConfigOutputHelper | — | Yes | No | No | Local file I/O, HelperOnly |
| `doctor` | (none) | ConfigOutputHelper | — | Yes | No | No | Diagnostics, HelperOnly |
| `policy-explain` | (none) | PassiveAnalytical | — | Yes | No | No | HelperOnly |
| `scope-explain` | (none) | PassiveAnalytical | — | Yes | No | No | HelperOnly |
| `ai-analyze` | (none) | PassiveAnalytical | `ai-integration` | Yes | No | No | HelperOnly |
| `serve` | (none) | FrontendServer | `rest-api` | No | No | No | ServerLifecycle |
| `mcp-serve` | (none) | FrontendServer | `rest-api` | No | No | No | ServerLifecycle |
| `agent` | (none) | FrontendServer | `rest-api` | No | No | No | ServerLifecycle |
| `grpc` | (none) | FrontendServer | `grpc-api` | No | No | No | ServerLifecycle |
| `cluster` | (none) | FrontendServer | — | No | No | No | Distributed infra, ServerLifecycle |
| `remote` | (none) | FrontendServer | — | No | No | No | Distributed infra, ServerLifecycle |
| `exec` | (none) | FrontendServer | — | No | No | No | Distributed infra, ServerLifecycle |
| `report` | (none) | LocalFileDomain | — | Yes | No | No | Output formatting, HelperOnly |
| `vuln` | (none) | ConfigOutputHelper | — | Yes | No | No | CVSS scoring, HelperOnly |
| `storage` | (none) | LocalFileDomain | `database` | Yes | No | No | HelperOnly |
| `sbom` | (none) | LocalFileDomain | `sbom` | Yes | No | No | HelperOnly |
| `notify` | (none) | ConfigOutputHelper | — | Yes | No | No | Test helper, HelperOnly |

## Migration Notes

- **Phase 6 pilot**: 4 low-risk commands (recon, scan-ports, scan-endpoints, fingerprint) use the registry for metadata lookup and descriptor generation. The legacy handler match in `handle_command()` remains the execution path.
- **Future phases**: Additional commands can be migrated incrementally. The dispatch bridge supports mixed registry/legacy dispatch.
- **No enforcement changes**: The registry is metadata and routing, not authorization. `EnforcementContext::evaluate()` remains the mandatory pre-dispatch gate.

### CommandDispatchMode

Each `CommandRegistration` carries a `dispatch_mode: CommandDispatchMode` field that classifies how the command is dispatched:

| Variant | Description |
|---------|-------------|
| `RegistryBacked` | Descriptor/execution path uses registry metadata (Phase 6 pilot commands: `recon`, `scan-ports`, `scan-endpoints`, `fingerprint`). `registry_backed = true`. |
| `LegacyWrapped` | Wraps legacy `handle_command()` dispatch (pre-migration commands). `registry_backed = false`. |
| `CatalogOnly` | Listed for discoverability but never dispatched (catalog entries). |
| `ServerLifecycle` | Server daemon lifecycle command (`serve`, `mcp-serve`, `agent`, `grpc`, `cluster`, `remote`, `exec`). |
| `HelperOnly` | Read-only helper/diagnostic (`config`, `doctor`, `plan`, `preflight`, `ci`, `report`, `vuln`, `storage`, `sbom`, `notify`, `policy-explain`, `scope-explain`, `ai-analyze`). |

The `registry_backed` boolean on `CommandRegistration` is a shorthand for `dispatch_mode == RegistryBacked`. It indicates the command uses registry metadata for descriptor generation via `build_descriptor()` rather than inline construction in the legacy handler match.

## File Locations

| File | Purpose |
|------|---------|
| `crates/eggsec/src/commands/registry.rs` | Registry types and static entries |
| `crates/eggsec/src/commands/mod.rs` | Re-exports |
| `crates/eggsec/src/commands/handlers/mod.rs` | Dispatch bridge integration |
| `crates/eggsec/tests/command_registry.rs` | Registry consistency tests |
| `crates/eggsec/src/config/policy.rs` | `OperationMetadata` (canonical source) |
