#!/usr/bin/env python3
"""
SBOM Generation through AsyncEngine.

Demonstrates generating a Software Bill of Materials (SBOM) for a project
using the `generate_sbom` operation via AsyncEngine dispatch.

Requirements:
    - eggsec installed with `sbom` feature
    - A project directory with dependency manifests

Usage:
    python sbom_generation.py [/path/to/project]
"""

import asyncio
import sys

import eggsec
from eggsec import AsyncEngine, Scope, SbomRequest


async def main():
    # Determine project path
    project_path = sys.argv[1] if len(sys.argv) > 1 else "."

    # Verify feature availability
    features = eggsec.features()
    if not features.get("sbom", False):
        print("Error: 'sbom' feature not compiled.")
        print("Build with: maturin develop --features sbom")
        sys.exit(1)

    # Create async engine
    scope = Scope.allow_hosts(["127.0.0.1"])
    engine = AsyncEngine(scope)

    # Build the request
    request = SbomRequest(
        project_path=project_path,
        ecosystem=None,  # Auto-detect from manifest files
        format="cyclonedx",  # or "spdx"
    )

    print(f"Generating SBOM for {project_path}...")

    # Dispatch through async engine
    result = await engine.run_sbom_generation(request)

    # Check result status
    if result.status.name() == "Completed":
        report = result.payload
        print(f"\nSBOM generated: {report.format}")
        print(f"Components: {len(report.components)}")
        print(f"Dependencies: {report.direct_count} direct, {report.transitive_count} transitive")

        # Print component summary
        for component in report.components[:20]:  # Limit output
            licenses = ", ".join(component.licenses) if component.licenses else "unknown"
            direct = "direct" if component.is_direct else "transitive"
            print(f"  {component.name}@{component.version} ({component.ecosystem}) [{licenses}] ({direct})")

        if len(report.components) > 20:
            print(f"  ... and {len(report.components) - 20} more components")

        # Check for known vulnerabilities
        if hasattr(report, "vulnerabilities") and report.vulnerabilities:
            print(f"\nVulnerabilities found: {len(report.vulnerabilities)}")
            for vuln in report.vulnerabilities:
                print(f"  {vuln}: {vuln.description}")
    elif result.status.name() == "Failed":
        error = result.error
        print(f"SBOM generation failed ({error.kind}): {error.message}")
        sys.exit(1)
    else:
        print(f"Unexpected status: {result.status.name()}")
        sys.exit(1)


if __name__ == "__main__":
    asyncio.run(main())
