#!/usr/bin/env python3
"""Workspace package graph helper for release validation.

Commands:
    list     - Print package set with classifications and internal deps
    validate - Check publishability invariants
    order    - Topological publication order for crates.io packages

Requires: Python 3.11+ (tomllib in stdlib).
"""

from __future__ import annotations

import json
import subprocess
import sys
import tomllib
from collections import defaultdict
from pathlib import Path

WORKSPACE_ROOT = Path(__file__).resolve().parent.parent
CARGO_TOML = WORKSPACE_ROOT / "Cargo.toml"


def _cargo_metadata() -> dict:
    result = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--no-deps"],
        capture_output=True, text=True, cwd=WORKSPACE_ROOT, check=True,
    )
    return json.loads(result.stdout)


def _workspace_version() -> str:
    with open(CARGO_TOML, "rb") as f:
        return tomllib.load(f)["workspace"]["package"]["version"]


def _classify(manifest: Path) -> str:
    with open(manifest, "rb") as f:
        data = tomllib.load(f)
    pkg = data.get("package", {})
    if pkg.get("publish") is False:
        return "private-workspace"
    return "publish-crates-io"


def _internal_deps(manifest: Path) -> list[dict]:
    with open(manifest, "rb") as f:
        data = tomllib.load(f)
    pkg_name = data.get("package", {}).get("name", "")
    results = []
    for section in ("dependencies", "build-dependencies"):
        for dep_name, dep_val in data.get(section, {}).items():
            if isinstance(dep_val, dict) and "path" in dep_val:
                results.append({
                    "name": dep_name,
                    "kind": section,
                    "path": dep_val["path"],
                    "version": dep_val.get("version"),
                    "optional": dep_val.get("optional", False),
                })
            elif isinstance(dep_val, str):
                pass  # external
    # dev-dependencies
    for dep_name, dep_val in data.get("dev-dependencies", {}).items():
        if isinstance(dep_val, dict) and "path" in dep_val:
            results.append({
                "name": dep_name,
                "kind": "dev-dependencies",
                "path": dep_val["path"],
                "version": dep_val.get("version"),
                "optional": dep_val.get("optional", False),
            })
    return results


def cmd_list() -> None:
    meta = _cargo_metadata()
    version = _workspace_version()
    pkgs = {p["name"]: p for p in meta["packages"]}

    print(f"{'Package':<25} {'Version':<10} {'Classification':<22} {'Internal Dependencies'}")
    print("-" * 100)

    for name in sorted(pkgs):
        pkg = pkgs[name]
        manifest = Path(pkg["manifest_path"])
        classification = _classify(manifest)
        deps = _internal_deps(manifest)
        pub_deps = [d["name"] for d in deps if d["kind"] != "dev-dependencies"]
        dep_str = ", ".join(sorted(pub_deps)) if pub_deps else "(none)"
        print(f"{name:<25} {pkg['version']:<10} {classification:<22} {dep_str}")


def cmd_validate() -> int:
    meta = _cargo_metadata()
    version = _workspace_version()
    pkgs = {p["name"]: p for p in meta["packages"]}
    errors: list[str] = []

    for name, pkg in pkgs.items():
        manifest = Path(pkg["manifest_path"])
        classification = _classify(manifest)
        deps = _internal_deps(manifest)

        if classification == "private-workspace":
            # Private crate: internal dep versions don't matter for crates.io
            continue

        # publish-crates-io: validate invariants
        for dep in deps:
            if dep["kind"] == "dev-dependencies":
                continue  # dev deps are stripped from published manifest

            dep_name = dep["name"]
            dep_pkg = pkgs.get(dep_name)
            if dep_pkg is None:
                continue  # external dep

            dep_manifest = Path(dep_pkg["manifest_path"])
            dep_class = _classify(dep_manifest)

            # Rule: version must be present for normal/build deps of publishable crates
            if dep["kind"] in ("dependencies", "build-dependencies") and not dep.get("version"):
                errors.append(
                    f"  {name}: {dep['kind']} dep '{dep_name}' has path but no version\n"
                    f"    Fix: {dep_name} = {{ path = \"{dep['path']}\", version = \"{version}\" }}"
                )

            # Rule: publishable crate must not depend on private crate (in normal/build)
            if dep["kind"] in ("dependencies", "build-dependencies") and dep_class == "private-workspace":
                errors.append(
                    f"  {name}: {dep['kind']} dep '{dep_name}' is private but will remain in published package"
                )

        # Rule: internal dep version must match workspace version
        for dep in deps:
            if dep["kind"] == "dev-dependencies":
                continue
            if dep.get("version") and dep["version"] != version:
                errors.append(
                    f"  {name}: dep '{dep['name']}' has version '{dep['version']}' "
                    f"but workspace version is '{version}'"
                )

    if errors:
        print("Validation FAILED:")
        for e in errors:
            print(e)
        return 1
    print("Validation passed.")
    return 0


def cmd_order() -> int:
    meta = _cargo_metadata()
    pkgs = {p["name"]: p for p in meta["packages"]}

    # Build adjacency list for publishable crates only
    publishable = set()
    for name, pkg in pkgs.items():
        manifest = Path(pkg["manifest_path"])
        if _classify(manifest) == "publish-crates-io":
            publishable.add(name)

    graph: dict[str, set[str]] = defaultdict(set)  # dep -> dependents
    in_degree: dict[str, int] = {n: 0 for n in publishable}

    for name in publishable:
        manifest = Path(pkgs[name]["manifest_path"])
        deps = _internal_deps(manifest)
        for dep in deps:
            if dep["kind"] == "dev-dependencies":
                continue
            dep_name = dep["name"]
            if dep_name in publishable:
                graph[dep_name].add(name)
                in_degree[name] += 1

    # Kahn's algorithm with stable name sort for tie-breaking
    queue = sorted(n for n in publishable if in_degree[n] == 0)
    order: list[str] = []

    while queue:
        node = queue.pop(0)
        order.append(node)
        for dependent in sorted(graph[node]):
            in_degree[dependent] -= 1
            if in_degree[dependent] == 0:
                queue.append(dependent)
        queue.sort()  # stable tie-breaking

    if len(order) != len(publishable):
        remaining = publishable - set(order)
        print(f"ERROR: cycle detected involving: {', '.join(sorted(remaining))}", file=sys.stderr)
        return 1

    for name in order:
        print(name)
    return 0


def main() -> int:
    if len(sys.argv) < 2:
        print(__doc__.strip(), file=sys.stderr)
        return 1

    cmd = sys.argv[1]
    if cmd == "list":
        cmd_list()
        return 0
    elif cmd == "validate":
        return cmd_validate()
    elif cmd == "order":
        return cmd_order()
    else:
        print(f"Unknown command: {cmd}", file=sys.stderr)
        print("Commands: list, validate, order", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
