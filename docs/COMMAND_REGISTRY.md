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
    └─ descriptor builder → OperationDescriptor from metadata
    
Dispatch Bridge (handle_command)
    │
    ├─ registry.lookup(command_id) → CommandRegistration
    │   └─ found → build descriptor → evaluate_and_enforce → execute
    └─ not found → legacy handler fallback
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

| Command ID | Operation ID | Category | Feature | Manual Only | TUI Visible |
|-----------|-------------|----------|---------|:-----------:|:-----------:|
| `recon` | `recon` | SideEffectingNetwork | — | No | Yes |
| `scan-ports` | `scan-ports` | SideEffectingNetwork | — | No | Yes |
| `scan-endpoints` | `scan-endpoints` | SideEffectingNetwork | — | No | Yes |
| `fingerprint` | `fingerprint` | SideEffectingNetwork | — | No | Yes |

### Legacy (not yet migrated)

| Command ID | Operation ID | Category | Feature | Notes |
|-----------|-------------|----------|---------|-------|
| `scan` | `scan` (alias→scan-ports) | SideEffectingNetwork | — | Pipeline orchestrator |
| `resume` | scan-resume | SideEffectingNetwork | — | Pipeline resume |
| `fuzz` | `fuzz` | SideEffectingNetwork | — | Complex payload engine |
| `waf` | `waf-detect` | SideEffectingNetwork | — | WAF detection |
| `waf-stress` | `waf-stress` | SideEffectingNetwork | — | WAF stress tier |
| `graphql` | `graphql` | SideEffectingNetwork | — | GraphQL fuzzer |
| `oauth` | `oauth` | SideEffectingNetwork | — | OAuth fuzzer |
| `auth-test` | `auth-test` | SideEffectingNetwork | — | Multi-test suite |
| `load` | load-test | SideEffectingNetwork | — | Load testing |
| `stress` | stress-test | SideEffectingNetwork | `stress-testing` | |
| `packet` | packet | SideEffectingNetwork | `packet-inspection` | |
| `icmp` | icmp | SideEffectingNetwork | `stress-testing` | |
| `traceroute` | traceroute | SideEffectingNetwork | `stress-testing` | |
| `nse` | `nse` | SideEffectingNetwork | `nse` | |
| `hunt` | `hunt` | SideEffectingNetwork | `advanced-hunting` | |
| `evasion` | evasion | SideEffectingNetwork | `evasion` | |
| `postex` | postex | SideEffectingNetwork | `postex` | |
| `c2` | `c2` | SideEffectingNetwork | `c2` | |
| `proxy-intercept` | `proxy-intercept` | SideEffectingNetwork | `web-proxy` | |
| `wireless` | `wireless` | SideEffectingNetwork | `wireless` | |
| `browser` | `browser` | SideEffectingNetwork | `headless-browser` | |
| `mobile` | mobile-static/mobile-dynamic | LocalFileDomain | `mobile` | |
| `db` | `db-pentest` | LocalFileDomain | `db-pentest` | |
| `plan` | (none) | ConfigOutputHelper | — | Local config |
| `preflight` | (uses metadata lookup) | ConfigOutputHelper | — | Advisory only |
| `ci` | (none) | ConfigOutputHelper | — | Passive quality gate |
| `config` | (none) | ConfigOutputHelper | — | Local file I/O |
| `doctor` | (none) | ConfigOutputHelper | — | Diagnostics |
| `policy-explain` | (none) | PassiveAnalytical | — | |
| `scope-explain` | (none) | PassiveAnalytical | — | |
| `ai-analyze` | (none) | PassiveAnalytical | `ai-integration` | |
| `serve` | (none) | FrontendServer | `rest-api` | |
| `mcp-serve` | (none) | FrontendServer | `rest-api` | |
| `agent` | (none) | FrontendServer | `rest-api` | |
| `grpc` | (none) | FrontendServer | `grpc-api` | |
| `cluster` | (none) | FrontendServer | — | Distributed infra |
| `remote` | (none) | FrontendServer | — | Distributed infra |
| `exec` | (none) | FrontendServer | — | Distributed infra |
| `report` | (none) | LocalFileDomain | — | Output formatting |
| `vuln` | (none) | ConfigOutputHelper | — | CVSS scoring |
| `storage` | (none) | LocalFileDomain | `database` | |
| `sbom` | (none) | LocalFileDomain | `sbom` | |
| `notify` | (none) | ConfigOutputHelper | — | Test helper |

## Migration Notes

- **Phase 6 pilot**: 4 low-risk commands (recon, scan-ports, scan-endpoints, fingerprint) use the registry for metadata lookup and descriptor generation. The legacy handler match in `handle_command()` remains the execution path.
- **Future phases**: Additional commands can be migrated incrementally. The dispatch bridge supports mixed registry/legacy dispatch.
- **No enforcement changes**: The registry is metadata and routing, not authorization. `EnforcementContext::evaluate()` remains the mandatory pre-dispatch gate.

## File Locations

| File | Purpose |
|------|---------|
| `crates/eggsec/src/commands/registry.rs` | Registry types and static entries |
| `crates/eggsec/src/commands/mod.rs` | Re-exports |
| `crates/eggsec/src/commands/handlers/mod.rs` | Dispatch bridge integration |
| `crates/eggsec/tests/command_registry.rs` | Registry consistency tests |
| `crates/eggsec/src/config/policy.rs` | `OperationMetadata` (canonical source) |
