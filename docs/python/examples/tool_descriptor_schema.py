#!/usr/bin/env python3
"""Tool descriptor discovery and JSON Schema generation.

Demonstrates listing registered tools, inspecting individual tool
descriptors, and generating JSON Schema for request/response types.
All introspection -- no network required.

Requirements:
    - eggsec (default features)

Usage:
    python3 docs/python/examples/tool_descriptor_schema.py
"""

import json

from eggsec import ToolRegistry, SchemaGenerator


def main():
    # List all registered tools
    tools = ToolRegistry.list()
    print(f"Registered tools: {len(tools)}")

    for t in tools[:5]:
        print(f"  {t.tool_id}: {t.title} (risk={t.risk})")
    if len(tools) > 5:
        print(f"  ... and {len(tools) - 5} more")

    # Find a specific tool descriptor
    desc = ToolRegistry.get("scan_ports")
    if desc:
        print(f"\n--- scan_ports descriptor ---")
        print(f"  tool_id:      {desc.tool_id}")
        print(f"  title:        {desc.title}")
        print(f"  description:  {desc.description[:60]}...")
        print(f"  risk:         {desc.risk}")
        print(f"  target_policy:{desc.target_policy}")
        print(f"  maturity:     {desc.maturity}")
        print(f"  surfaces:     {desc.supported_surfaces}")

    # Generate JSON Schema for a tool (returned as JSON string)
    req_schema_str = SchemaGenerator.generate_input_schema("scan_ports")
    req_schema = json.loads(req_schema_str)
    print(f"\n--- scan_ports request schema ---")
    print(json.dumps(req_schema, indent=2)[:300])

    resp_schema_str = SchemaGenerator.generate_output_schema("scan_ports")
    resp_schema = json.loads(resp_schema_str)
    print(f"\n--- scan_ports response schema ---")
    print(json.dumps(resp_schema, indent=2)[:300])

    # Full manifest
    manifest = SchemaGenerator.all_schemas()
    print(f"\n--- Full manifest ---")
    print(f"Tools with schemas: {len(manifest)}")
    for tool_id, schemas in list(manifest.items())[:3]:
        has_req = bool(schemas.get("input"))
        has_resp = bool(schemas.get("output"))
        print(f"  {tool_id}: input={has_req}, output={has_resp}")


if __name__ == "__main__":
    main()
