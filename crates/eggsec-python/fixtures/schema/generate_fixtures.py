#!/usr/bin/env python3
"""Generate schema snapshot fixtures for release-candidate validation.

Run from the crate root:
    python fixtures/schema/generate_fixtures.py

Produces one JSON file per stable operation under fixtures/schema/<tool_id>.json
containing the input and output schemas. These are checked into version control
and used by test_schema_snapshots.py to detect breaking schema changes.
"""

import json
import sys
from pathlib import Path

# Ensure the built extension is importable
CRATE_ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(CRATE_ROOT / "python"))

FIXTURE_DIR = Path(__file__).resolve().parent


def main() -> None:
    try:
        from eggsec import ToolRegistry, SchemaGenerator
    except ImportError:
        print("ERROR: eggsec module not importable. Run 'maturin develop' first.", file=sys.stderr)
        sys.exit(1)

    FIXTURE_DIR.mkdir(parents=True, exist_ok=True)

    tools = ToolRegistry.list()
    print(f"Generating fixtures for {len(tools)} tools...")

    for desc in tools:
        tool_id = desc.tool_id
        input_schema_str = SchemaGenerator.generate_input_schema(tool_id)
        output_schema_str = SchemaGenerator.generate_output_schema(tool_id)

        fixture = {
            "tool_id": tool_id,
            "operation_id": desc.operation_id,
            "version": desc.version,
            "input_schema": json.loads(input_schema_str) if input_schema_str else None,
            "output_schema": json.loads(output_schema_str) if output_schema_str else None,
        }

        fixture_path = FIXTURE_DIR / f"{tool_id}.json"
        with open(fixture_path, "w") as f:
            json.dump(fixture, f, indent=2, sort_keys=True)
            f.write("\n")
        print(f"  {tool_id}: OK")

    print(f"Done. Fixtures written to {FIXTURE_DIR}")


if __name__ == "__main__":
    main()
