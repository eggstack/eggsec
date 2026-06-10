# Policy Handler Audit

## Date: 2026-06-09

## Summary

All command handlers were audited for adoption of the shared policy evaluation
path (`evaluate_and_enforce_operation` / `evaluate_operation_policy`). High-risk
handlers have been migrated to use `OperationDescriptor`-based policy checks.

## Migration Status

### Migrated Handlers (using `evaluate_and_enforce_operation`)

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

### Handlers Using `evaluate_operation_policy` Directly (no bail on deny)

These handlers call `evaluate_operation_policy` to read policy decisions for
preview/output purposes rather than to enforce denials:

| Handler | File | Notes |
|---------|------|-------|
| `handle_plan` | `handlers/plan.rs` | Reads policy decisions for preview output |
| `handle_policy_explain` | `handlers/explain.rs` | Reads policy decisions for human/JSON output |
| `handle_scope_explain` | `handlers/explain.rs` | Reads policy decisions for human/JSON output |

### Scope-Only Handlers (use `ensure_scope`/`ensure_scope_url`)

| Handler | File | Risk |
|---------|------|------|
| `handle_scan` | `handlers/scan.rs` | SafeActive |
| `handle_scan_ports` | `handlers/scan.rs` | SafeActive |
| `handle_scan_endpoints` | `handlers/scan.rs` | SafeActive |
| `handle_fingerprint` | `handlers/scan.rs` | SafeActive |
| `handle_resume` | `handlers/scan.rs` | Target already validated |
| `handle_recon` | `handlers/recon.rs` | SafeActive |
| `handle_auth_test` | `handlers/auth_test.rs` | CredentialTesting |
| `handle_hunt` | `handlers/hunt.rs` | Intrusive (behind `advanced-hunting` feature) |
| `handle_wireless` | `handlers/wireless.rs` | SafeActive |
| `handle_browser` | `handlers/browser.rs` | SafeActive |
| `handle_grpc_server` | `handlers/grpc.rs` | SafeActive |

### No Policy Evaluation (no target or delegated enforcement)

| Handler | File | Notes |
|---------|------|-------|
| `handle_remote` | `handlers/cluster.rs` | Opens a network listener; no target from CLI args (listens on a bind address) |
| `handle_agent` | `handlers/agent.rs` | Autonomous agent; tool operations are enforced internally by the agent runtime |
| `handle_cluster` (Coordinator/Worker) | `handlers/cluster.rs` | Infrastructure commands; no target-bearing risk |
| `handle_cluster` (Status) | `handlers/cluster.rs` | Status query only |
| `handle_serve` / `handle_mcp_serve` | `handlers/serve.rs` | API servers, no external target |
| `handle_config` | `handlers/config.rs` | Not target-bearing |
| `handle_doctor` | `handlers/doctor.rs` | Not target-bearing |
| `handle_notify` | `handlers/notify.rs` | Not target-bearing |
| `handle_report` | `handlers/report.rs` | Not target-bearing |
| `handle_sbom` | `handlers/sbom.rs` | Not target-bearing |
| `handle_storage` | `handlers/storage.rs` | Not target-bearing |
| `handle_vuln` | `handlers/vuln.rs` | Not target-bearing |
| `handle_ci` | `handlers/ci.rs` | Not target-bearing |
| `handle_ai_analyze` | `handlers/ai_analyze.rs` | Not target-bearing |

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
