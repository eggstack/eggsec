# Policy Handler Audit

## Date: 2026-06-09

## Summary

All command handlers were audited for adoption of the shared policy evaluation
path (`evaluate_and_enforce_operation` / `evaluate_operation_policy`). High-risk
handlers have been migrated to use `OperationDescriptor`-based policy checks.

## Migration Status

### Migrated Handlers

| Handler | File | Operation | Risk | IntendedUse |
|---------|------|-----------|------|-------------|
| `handle_stress` | `handlers/stress.rs` | `stress` | StressTest | DistributedSystemStress |
| `handle_proxy` (Add) | `handlers/stress.rs` | `proxy-add` | ExploitAdjacent | WebAssessment |
| `handle_proxy` (Test) | `handlers/stress.rs` | `proxy-test` | ExploitAdjacent | WebAssessment |
| `handle_waf_stress` | `handlers/fuzz.rs` | `waf-stress` | Intrusive | WafRegression |
| `handle_fuzz` | `handlers/fuzz.rs` | `fuzz` | Intrusive | WebAssessment |
| `handle_waf` | `handlers/fuzz.rs` | `waf-detect` | Intrusive | WafRegression |
| `handle_packet` (Send) | `handlers/network.rs` | `packet-send` | RawPacket | ProtocolEdgeValidation |
| `handle_packet` (Traceroute) | `handlers/network.rs` | `packet-traceroute` | RawPacket | ProtocolEdgeValidation |
| `handle_icmp` | `handlers/network.rs` | `icmp` | SafeActive | ProtocolEdgeValidation |
| `handle_traceroute` | `handlers/network.rs` | `traceroute` | RawPacket | ProtocolEdgeValidation |
| `handle_exec` | `handlers/cluster.rs` | `exec` | RemoteExecution | DistributedSystemStress |
| `handle_nse` | `handlers/scan.rs` | `nse` | Intrusive | WebAssessment |
| `handle_load` | `handlers/load.rs` | `load` | LoadTest | WebAssessment |

### Unchanged Handlers (Scope-Only or No Target)

These handlers still use `ensure_scope`/`ensure_scope_url` only:

- `handle_scan` — pipeline scan, lower risk (SafeActive), uses scope check
- `handle_scan_ports` — port scanning, SafeActive
- `handle_scan_endpoints` — endpoint discovery, SafeActive
- `handle_fingerprint` — service fingerprinting, Passive/SafeActive
- `handle_resume` — resumes existing session, target already validated
- `handle_recon` — reconnaissance, SafeActive
- `handle_auth_test` — auth testing, CredentialTesting
- `handle_hunt` — advanced hunting, Intrusive (behind `advanced-hunting` feature)
- `handle_wireless` — WiFi scanning, SafeActive
- `handle_browser` — headless browser, SafeActive
- `handle_cluster` (AddTask) — distributed task, uses scope on target
- `handle_grpc_server` — gRPC server, SafeActive
- `handle_serve` / `handle_mcp_serve` — API servers, no external target
- `handle_remote` — remote listener, no external target
- `handle_agent` — autonomous agent, manages its own portfolio

### Not Target-Bearing

- `handle_config`, `handle_doctor`, `handle_notify`, `handle_report`,
  `handle_sbom`, `handle_storage`, `handle_vuln`, `handle_ci`, `handle_ai_analyze`

## Notes

- The `CommandContext::evaluate_and_enforce_operation` method wraps
  `evaluate_operation_policy` and bails with structured output (JSON or
  human-readable) on denial.
- All migrated handlers build an `OperationDescriptor` with correct mode,
  risk, intended use, and target.
- The MCP module (`tool/protocol/mcp/`) has its own policy enforcement via
  `McpProfilePolicy::validate_tool_call` and `validate_target`, which are
  wired into the `tools/call` JSON-RPC handler. MCP denials now include
  structured `PolicyDecision` data in the error response.
