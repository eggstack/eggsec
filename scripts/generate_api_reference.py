#!/usr/bin/env python3
"""E8: Generate API reference from the canonical operation registry and stubs.

Reads OperationRegistry, ToolRegistry, and build_info() to produce
a machine-generated API reference document. Includes maturity,
feature requirements, risk levels, policy behavior, and schema links.

Requirements:
    - eggsec installed

Usage:
    python scripts/generate_api_reference.py --output docs/python/api-reference-generated.md
"""

import argparse
import json
import sys
from datetime import datetime, timezone


def generate_reference():
    """Generate the full API reference."""
    import eggsec
    from eggsec import OperationRegistry, ToolRegistry, SchemaGenerator

    lines = []
    w = lines.append

    # Header
    w("# Eggsec Python API Reference")
    w("")
    w(f"> Auto-generated on {datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ')} — do not edit manually.")
    w("")

    # Build metadata
    info = eggsec.build_info()
    w("## Build Metadata")
    w("")
    w(f"- **Version**: {info.get('version', '?')}")
    w(f"- **Python**: {info.get('python_version', '?')}")
    w(f"- **Wheel profile**: {eggsec.wheel_profile()}")
    w(f"- **Compiled features**: {', '.join(info.get('compiled_features', []))}")
    ver = eggsec.api_surface_version()
    w(f"- **Schema version**: {ver.get('schema_version', '?')}")
    w(f"- **Protocol version**: {ver.get('protocol_version', '?')}")
    w(f"- **ABI version**: {ver.get('abi_version', '?')}")
    w("")

    # Table of contents
    w("## Table of Contents")
    w("")
    w("- [Stable Operations](#stable-operations)")
    w("- [Tool Descriptors](#tool-descriptors)")
    w("- [Policy and Enforcement](#policy-and-enforcement)")
    w("- [Feature Matrix](#feature-matrix)")
    w("- [Type Hierarchy](#type-hierarchy)")
    w("")

    # Stable operations from registry
    w("## Stable Operations")
    w("")
    w("The following operations are part of the stable-core registry.")
    w("Each operation is dispatched via `Engine.run_*()` or `AsyncEngine.run_*()`.")
    w("")

    try:
        all_ops = OperationRegistry.all_operations()
        w(f"| Operation ID | Label | Risk | Feature | Description |")
        w(f"|---|---|---|---|---|")
        for op in all_ops:
            op_id = getattr(op, "operation_id", "?")
            label = getattr(op, "label", "?")
            risk = getattr(op, "risk", "?")
            feature = getattr(op, "required_feature", "") or ""
            desc = getattr(op, "description", "")[:60]
            w(f"| `{op_id}` | {label} | {risk} | {feature} | {desc} |")
        w("")
    except Exception as e:
        w(f"<!-- OperationRegistry error: {e} -->")
        w("")

    # Tool descriptors
    w("## Tool Descriptors")
    w("")
    w("Tool descriptors provide MCP/REST/gRPC-compatible metadata for each operation.")
    w("")

    try:
        tools = ToolRegistry.list()
        w(f"| Tool ID | Title | Risk | Maturity | Target Policy |")
        w(f"|---|---|---|---|---|")
        for t in tools:
            w(f"| `{t.tool_id}` | {t.title} | {t.risk} | {t.maturity} | {t.target_policy} |")
        w("")
    except Exception as e:
        w(f"<!-- ToolRegistry error: {e} -->")
        w("")

    # Schema example
    w("### Schema Generation")
    w("")
    w("```python")
    w("from eggsec import ToolRegistry, SchemaGenerator")
    w("import json")
    w("")
    w("# List all tools")
    w("tools = ToolRegistry.list()")
    w("")
    w("# Generate input schema for a specific tool")
    w("schema_json = SchemaGenerator.generate_input_schema('scan_ports')")
    w("schema = json.loads(schema_json)")
    w("print(json.dumps(schema, indent=2))")
    w("```")
    w("")

    # Policy and enforcement
    w("## Policy and Enforcement")
    w("")
    w("| Surface | Constructor | Overrides | Scope Source |")
    w("|---|---|---|---|")
    w("| CLI/TUI | `EnforcementContext.manual_permissive(policy, scope)` | Yes | Config file |")
    w("| REST/MCP | `EnforcementContext.mcp_strict(policy, scope)` | No | Request |")
    w("| Agent | `EnforcementContext.agent_strict(policy, scope)` | No | Manifest |")
    w("| CI | `EnforcementContext.ci_strict(policy, scope)` | No | Environment |")
    w("")
    w("```python")
    w("from eggsec import (")
    w("    EnforcementContext, ExecutionPolicy, ExecutionSurface,")
    w("    LoadedScope, ManualOverride, OperationRegistry,")
    w(")")
    w("")
    w("scope = LoadedScope.default_empty()")
    w("policy = ExecutionPolicy()  # no args")
    w("ctx = EnforcementContext.manual_permissive(policy, scope)")
    w("")
    w("# Evaluate an operation")
    w("op = OperationRegistry.find('port_scan')")
    w("desc = op.descriptor_for_target('example.com')")
    w("outcome = ctx.evaluate(desc)")
    w("print(outcome.outcome_type)  # 'allow', 'confirm', or 'deny'")
    w("")
    w("# Approve (strict surfaces)")
    w("approved = ctx.approve(ExecutionSurface.MCP_SERVER, desc)")
    w("")
    w("# Manual override (CLI/TUI only)")
    w("override = ManualOverride(reason='testing', allow_high_risk=True)")
    w("approved = ctx.approve_manual(ExecutionSurface.CLI_MANUAL, desc, override)")
    w("```")
    w("")

    # Feature matrix
    w("## Feature Matrix")
    w("")
    features = eggsec.features()
    w(f"Current build has {len(features)} compiled features: {', '.join(sorted(features))}")
    w("")
    w("| Feature | Compiled | Maturity | Install Hint |")
    w("|---|---|---|---|")
    try:
        from eggsec._feature_guard import _FEATURES
        for key in sorted(_FEATURES.keys()):
            meta = _FEATURES[key]
            compiled = "+" if key in features else "-"
            maturity = meta.get("maturity", "?")
            hint = meta.get("install_hint", "")[:50]
            w(f"| `{key}` | {compiled} | {maturity} | {hint} |")
    except Exception:
        for f in sorted(features):
            w(f"| `{f}` | + | stable | |")
    w("")

    # Type hierarchy
    w("## Type Hierarchy")
    w("")
    w("### Core Types")
    w("")
    w("| Type | Purpose |")
    w("|---|---|")
    w("| `Engine` | Sync dispatch (22 operations) |")
    w("| `AsyncEngine` | Async dispatch (22 operations) |")
    w("| `Scope` | Target/port authorization (frozen) |")
    w("| `LoadedScope` | Enriched scope with source tracking |")
    w("| `CancellationToken` | Cooperative cancellation |")
    w("| `EggsecConfig` | Full configuration model |")
    w("| `SensitiveString` | Zeroized secret wrapper |")
    w("")

    w("### Policy Types")
    w("")
    w("| Type | Purpose |")
    w("|---|---|")
    w("| `ExecutionPolicy` | Risk-level policy config |")
    w("| `ManualOverride` | Override flags for manual surfaces |")
    w("| `EnforcementContext` | Policy evaluation gate |")
    w("| `ExecutionSurface` | Surface identifier (constants) |")
    w("| `ApprovedOperation` | Authorization token |")
    w("| `PreflightResult` | Pre-dispatch preview |")
    w("")

    w("### Finding Types")
    w("")
    w("| Type | Purpose |")
    w("|---|---|")
    w("| `VersionedFinding` | Schema-versioned finding |")
    w("| `VersionedEvidence` | Evidence with metadata |")
    w("| `AffectedAsset` | Asset affected by finding |")
    w("| `FindingLocation` | Location of finding |")
    w("| `FindingRepository` | In-memory finding storage |")
    w("| `FindingWorkflow` | State machine for findings |")
    w("| `BaselineComparator` | Diff between assessments |")
    w("")

    w("### Artifact Types")
    w("")
    w("| Type | Purpose |")
    w("|---|---|")
    w("| `MilestoneArtifact` | Stored artifact with hash |")
    w("| `ArtifactStore` | In-memory artifact storage |")
    w("| `ContentAddressedArtifactStore` | Deduplicating store |")
    w("")

    w("### Reporting Types")
    w("")
    w("| Type | Purpose |")
    w("|---|---|")
    w("| `FindingReporter` | Generate reports (JSON/SARIF/HTML/CSV/MD) |")
    w("| `SeveritySummary` | Severity totals and risk score |")
    w("| `ReportEnvelope` | Report metadata envelope |")
    w("| `StreamingReporter` | Streaming report generation |")
    w("")

    w("### Network Types (Provisional)")
    w("")
    w("| Type | Purpose |")
    w("|---|---|")
    w("| `TcpSession` | Managed TCP connection |")
    w("| `UdpSocket` | Managed UDP socket |")
    w("| `HttpClient` | Security-oriented HTTP client |")
    w("| `WebSocketSession` | WebSocket connection |")
    w("| `dns_query()` | One-shot DNS lookup |")
    w("| `tls_probe()` | TLS certificate inspection |")
    w("| `http_probe()` | HTTP request probe |")
    w("")

    # Exceptions
    w("## Exception Hierarchy")
    w("")
    w("```")
    w("EggsecError (base)")
    w("  ConfigError")
    w("  ScopeError")
    w("  EnforcementError")
    w("  NetworkError")
    w("  ScanError")
    w("  TimeoutError")
    w("  FeatureUnavailableError")
    w("  SerializationError")
    w("  InternalError")
    w("```")
    w("")

    # Compatibility
    w("## Compatibility Policy")
    w("")
    w("- **Stable operations** (22-operation registry): backward-compatible within 0.x")
    w("- **Provisional types** (net, sessions, storage, reporting, daemon): may change between releases")
    w("- **Experimental types** (wireless, evasion, postex, c2, stress, ai, headless-browser): unstable")
    w("- **Daemon-client API**: provisional until transport parity milestone")
    w("")

    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser(description="Generate API reference")
    parser.add_argument("--output", required=True, help="Output markdown file path")
    args = parser.parse_args()

    reference = generate_reference()

    with open(args.output, "w") as f:
        f.write(reference)

    print(f"API reference generated: {args.output}")
    print(f"  {len(reference)} bytes, {reference.count(chr(10))} lines")


if __name__ == "__main__":
    main()
