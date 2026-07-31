#!/usr/bin/env python3
"""Workspace package graph and deterministic archive checks.

Commands:
    list                         package set and internal dependencies
    validate                     publishability and version invariants
    order                        topological publication order
    version-locations            version-qualified internal dependency inventory
    inspect-archive <archive>    inspect one Cargo package archive

The helper intentionally uses only the Python standard library.  Archive
creation uses Cargo's registry-independent ``cargo package --list`` selection
and a deterministic local package copy because some Cargo versions still
resolve unpublished dependencies during ``--no-verify``. Registry-backed
checks are kept separate for the staged manual publication procedure.
"""

from __future__ import annotations

import io
import json
import re
import subprocess
import sys
import tarfile
import tomllib
from collections import defaultdict
from pathlib import Path

WORKSPACE_ROOT = Path(__file__).resolve().parent.parent
CARGO_TOML = WORKSPACE_ROOT / "Cargo.toml"
PROHIBITED_ARCHIVE_PARTS = (".git/", "target/", ".venv/", ".venv-ci/", "dist/", "*.pcap", "*.pcapng", "exports/")
PROHIBITED_PRIVATE_PACKAGES = {"eggsec-cli", "eggsec-tui", "eggsec-python"}


def _cargo_metadata() -> dict:
    result = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--no-deps"],
        capture_output=True, text=True, cwd=WORKSPACE_ROOT, check=True,
    )
    return json.loads(result.stdout)


def _workspace_version() -> str:
    with open(CARGO_TOML, "rb") as f:
        return tomllib.load(f)["workspace"]["package"]["version"]


def _load_manifest(manifest: Path) -> dict:
    with open(manifest, "rb") as f:
        return tomllib.load(f)


def _classify(manifest: Path) -> str:
    return "private-workspace" if _load_manifest(manifest).get("package", {}).get("publish") is False else "publish-crates-io"


def _dependency_tables(data: dict) -> list[tuple[str, dict]]:
    """Yield dependency tables, including every target-specific table."""
    tables = [(kind, data.get(kind, {})) for kind in ("dependencies", "build-dependencies", "dev-dependencies")]
    for target_name, target in data.get("target", {}).items():
        if not isinstance(target, dict):
            continue
        for kind in ("dependencies", "build-dependencies", "dev-dependencies"):
            tables.append((f"target.{target_name}.{kind}", target.get(kind, {})))
    return tables


def _internal_deps(manifest: Path) -> list[dict]:
    data = _load_manifest(manifest)
    results = []
    for kind, table in _dependency_tables(data):
        for manifest_key, dep_val in table.items():
            if not isinstance(dep_val, dict) or "path" not in dep_val:
                continue
            dep_path = (manifest.parent / dep_val["path"]).resolve()
            dep_manifest = dep_path / "Cargo.toml"
            package_name = dep_val.get("package")
            if dep_manifest.exists():
                package_name = _load_manifest(dep_manifest).get("package", {}).get("name", package_name)
            results.append({
                "name": manifest_key,
                "package": package_name or manifest_key,
                "kind": kind,
                "path": dep_val["path"],
                "version": dep_val.get("version"),
                "optional": dep_val.get("optional", False),
                "manifest": str(manifest),
            })
    return results


def _publishable_packages(meta: dict) -> dict[str, dict]:
    return {
        p["name"]: p for p in meta["packages"]
        if _classify(Path(p["manifest_path"])) == "publish-crates-io"
    }


def cmd_list() -> None:
    meta = _cargo_metadata()
    pkgs = {p["name"]: p for p in meta["packages"]}
    print(f"{'Package':<25} {'Version':<10} {'Classification':<22} {'Internal Dependencies'}")
    print("-" * 100)
    for name in sorted(pkgs):
        pkg = pkgs[name]
        deps = _internal_deps(Path(pkg["manifest_path"]))
        pub_deps = [d["package"] for d in deps if not d["kind"].endswith("dev-dependencies")]
        dep_str = ", ".join(sorted(pub_deps)) if pub_deps else "(none)"
        print(f"{name:<25} {pkg['version']:<10} {_classify(Path(pkg['manifest_path'])):<22} {dep_str}")


def cmd_validate() -> int:
    meta = _cargo_metadata()
    version = _workspace_version()
    pkgs = {p["name"]: p for p in meta["packages"]}
    errors: list[str] = []
    for name, pkg in pkgs.items():
        manifest = Path(pkg["manifest_path"])
        if _classify(manifest) != "publish-crates-io":
            continue
        for dep in _internal_deps(manifest):
            if dep["kind"].endswith("dev-dependencies"):
                continue
            dep_pkg = pkgs.get(dep["package"])
            if dep_pkg is None:
                continue
            dep_class = _classify(Path(dep_pkg["manifest_path"]))
            prefix = f"{manifest}: dependency '{dep['name']}' (package '{dep['package']}')"
            if not dep.get("version"):
                errors.append(f"  {prefix} has path but no version; expected release version {version}")
            elif dep["version"] != version:
                errors.append(
                    f"  {manifest}: dependency key '{dep['name']}' (package '{dep['package']}') "
                    f"found version '{dep['version']}', expected release version '{version}'"
                )
            if dep_class == "private-workspace":
                errors.append(f"  {prefix} is private but remains in published runtime/build dependencies")
    if errors:
        print("Validation FAILED:")
        print("\n".join(errors))
        return 1
    print("Validation passed.")
    return 0


def cmd_version_locations() -> int:
    version = _workspace_version()
    meta = _cargo_metadata()
    for pkg in sorted(meta["packages"], key=lambda p: p["name"]):
        manifest = Path(pkg["manifest_path"])
        for dep in _internal_deps(manifest):
            if dep.get("version"):
                print(f"{manifest}: {dep['name']} -> {dep['package']} (version {dep['version']}; expected {version})")
    return 0


def cmd_order() -> int:
    pkgs = _publishable_packages(_cargo_metadata())
    graph: dict[str, set[str]] = defaultdict(set)
    in_degree = {name: 0 for name in pkgs}
    for name, pkg in pkgs.items():
        for dep in _internal_deps(Path(pkg["manifest_path"])):
            if dep["kind"].endswith("dev-dependencies"):
                continue
            if dep["package"] in pkgs:
                graph[dep["package"]].add(name)
                in_degree[name] += 1
    queue = sorted(name for name, degree in in_degree.items() if degree == 0)
    order: list[str] = []
    while queue:
        node = queue.pop(0)
        order.append(node)
        for dependent in sorted(graph[node]):
            in_degree[dependent] -= 1
            if in_degree[dependent] == 0:
                queue.append(dependent)
        queue.sort()
    if len(order) != len(pkgs):
        remaining = set(pkgs) - set(order)
        print(f"ERROR: cycle detected involving: {', '.join(sorted(remaining))}", file=sys.stderr)
        return 1
    print("\n".join(order))
    return 0


def _archive_manifest_member(names: list[str]) -> str | None:
    candidates = [n for n in names if n.count("/") == 1 and n.endswith("/Cargo.toml")]
    return candidates[0] if len(candidates) == 1 else None


def inspect_archive(archive: Path, expected_name: str | None = None, expected_version: str | None = None) -> list[str]:
    errors: list[str] = []
    if not archive.is_file():
        return [f"archive does not exist: {archive}"]
    try:
        with tarfile.open(archive, "r:gz") as tar:
            names = tar.getnames()
            manifest_name = _archive_manifest_member(names)
            if not manifest_name:
                return ["archive must contain exactly one top-level Cargo.toml"]
            manifest = tomllib.loads(tar.extractfile(manifest_name).read().decode())
            package = manifest.get("package", {})
            name = package.get("name")
            version = package.get("version")
            if expected_name and name != expected_name:
                errors.append(f"packaged name '{name}' does not match expected '{expected_name}'")
            if expected_version and version != expected_version:
                errors.append(f"packaged version '{version}' does not match expected '{expected_version}'")
            for member in names:
                relative = member.split("/", 1)[1] if "/" in member else member
                if any(relative.startswith(part.rstrip("*")) for part in PROHIBITED_ARCHIVE_PARTS if not part.endswith("*.pcap")):
                    errors.append(f"prohibited archive entry: {member}")
                if relative.endswith((".pcap", ".pcapng")) or relative.startswith("exports/"):
                    errors.append(f"prohibited archive entry: {member}")
            if expected_name:
                source_manifest = next((Path(p["manifest_path"]) for p in _cargo_metadata()["packages"] if p["name"] == expected_name), None)
                if source_manifest:
                    source = _load_manifest(source_manifest).get("package", {})
                    for field in ("repository", "license", "rust-version"):
                        expected = source.get(field)
                        if isinstance(expected, dict) and expected.get("workspace"):
                            root_pkg = _load_manifest(CARGO_TOML).get("workspace", {}).get("package", {})
                            expected = root_pkg.get(field)
                        if expected is not None and package.get(field) != expected:
                            errors.append(f"packaged metadata '{field}' is '{package.get(field)}', expected '{expected}'")
                    readme = package.get("readme")
                    if readme:
                        readme_name = Path(readme).name
                        if not any(Path(n).name == readme_name for n in names):
                            errors.append(f"packaged README is missing: {readme_name}")
                    license_file = package.get("license-file")
                    if license_file:
                        license_name = Path(license_file).name
                        if not any(Path(n).name == license_name for n in names):
                            errors.append(f"packaged license file is missing: {license_name}")
                    for dep in _dependency_tables(manifest):
                        kind, table = dep
                        if kind.endswith("dev-dependencies"):
                            continue
                        for dep_name, dep_val in table.items():
                            if isinstance(dep_val, dict) and "path" in dep_val:
                                errors.append(f"packaged dependency '{dep_name}' retains local path in {kind}")
                            package_name = dep_val.get("package", dep_name) if isinstance(dep_val, dict) else dep_name
                            if package_name in PROHIBITED_PRIVATE_PACKAGES:
                                errors.append(f"packaged dependency '{dep_name}' targets private package '{package_name}'")
                            if package_name.startswith("eggsec-") and isinstance(dep_val, dict) and dep_val.get("version") != expected_version:
                                errors.append(f"packaged internal dependency '{dep_name}' has version '{dep_val.get('version')}', expected '{expected_version}'")
    except (tarfile.TarError, tomllib.TOMLDecodeError, UnicodeDecodeError, OSError) as exc:
        errors.append(f"cannot inspect archive: {exc}")
    return errors


def _normalized_manifest_text(source_manifest: Path) -> str:
    """Create Cargo's publish-facing shape when --no-verify cannot archive."""
    root_package = _load_manifest(CARGO_TOML).get("workspace", {}).get("package", {})
    workspace_fields = {key: value for key, value in root_package.items() if not isinstance(value, dict)}
    lines = []
    section = ""
    for line in source_manifest.read_text().splitlines():
        stripped = line.strip()
        if stripped.startswith("["):
            section = stripped
        match = re.match(r"^(\s*)(version|edition|license|repository|rust-version|authors|homepage|documentation)\.workspace\s*=\s*true\s*$", line)
        if match:
            key = match.group(2)
            value = workspace_fields.get(key)
            if value is not None:
                lines.append(f"{match.group(1)}{key} = {json.dumps(value)}")
                continue
        # Cargo removes local paths from publish-facing dependency tables.
        if "path" in line and "{" in line and "}" in line:
            line = re.sub(r"path\s*=\s*\"[^\"]*\"\s*,?\s*", "", line)
            line = re.sub(r",\s*}", " }", line)
        lines.append(line)
    return "\n".join(lines) + "\n"


def cmd_create_archive() -> int:
    if len(sys.argv) < 4:
        print("Usage: create-archive <package-name> <output.crate>", file=sys.stderr)
        return 1
    package_name, output = sys.argv[2], Path(sys.argv[3])
    packages = {p["name"]: p for p in _cargo_metadata()["packages"]}
    package = packages.get(package_name)
    if not package:
        print(f"unknown workspace package: {package_name}", file=sys.stderr)
        return 1
    source_manifest = Path(package["manifest_path"])
    crate_root = source_manifest.parent
    package_metadata = _load_manifest(source_manifest).get("package", {})
    version = package["version"]
    listing = subprocess.run(
        ["cargo", "package", "-p", package_name, "--allow-dirty", "--no-verify", "--list"],
        cwd=WORKSPACE_ROOT, capture_output=True, text=True, check=False,
    )
    if listing.returncode != 0:
        print(listing.stdout, end="")
        print(listing.stderr, end="", file=sys.stderr)
        return listing.returncode
    files = [line.strip() for line in listing.stdout.splitlines() if line.strip() and not line.startswith("warning:")]
    output.parent.mkdir(parents=True, exist_ok=True)
    with tarfile.open(output, "w:gz") as archive:
        root = f"{package_name}-{version}"
        for relative in files:
            source = crate_root / relative
            if not source.is_file() and relative in {Path(package_metadata.get("readme", "")).name, Path(package_metadata.get("license-file", "")).name}:
                configured = package_metadata.get("readme") if relative == Path(package_metadata.get("readme", "")).name else package_metadata.get("license-file")
                if configured:
                    source = (crate_root / configured).resolve()
            if not source.is_file():
                continue
            info = tarfile.TarInfo(f"{root}/{relative}")
            content = _normalized_manifest_text(source_manifest) if relative == "Cargo.toml" else source.read_bytes()
            info.size = len(content)
            archive.addfile(info, io.BytesIO(content if isinstance(content, bytes) else content.encode()))
    print(f"Created deterministic local archive: {output}")
    return 0


def cmd_inspect_archive() -> int:
    if len(sys.argv) < 3:
        print("Usage: inspect-archive <path-to-crate> [package-name] [version]", file=sys.stderr)
        return 1
    archive = Path(sys.argv[2])
    expected_name = sys.argv[3] if len(sys.argv) > 3 else None
    expected_version = sys.argv[4] if len(sys.argv) > 4 else _workspace_version()
    errors = inspect_archive(archive, expected_name, expected_version)
    if errors:
        print("Archive inspection FAILED:")
        print("\n".join(f"  {error}" for error in errors))
        return 1
    print(f"Archive inspection passed: {archive}")
    return 0


def main() -> int:
    if len(sys.argv) < 2:
        print(__doc__.strip(), file=sys.stderr)
        return 1
    commands = {
        "list": lambda: (cmd_list() or 0),
        "validate": cmd_validate,
        "order": cmd_order,
        "version-locations": cmd_version_locations,
        "inspect-archive": cmd_inspect_archive,
        "create-archive": cmd_create_archive,
    }
    command = commands.get(sys.argv[1])
    if command is None:
        print(f"Unknown command: {sys.argv[1]}", file=sys.stderr)
        return 1
    return command()


if __name__ == "__main__":
    sys.exit(main())
