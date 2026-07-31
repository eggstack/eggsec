#!/usr/bin/env python3
"""Cargo package graph and Cargo-native archive validation.

Commands:
    list
    validate
    order
    version-locations
    package-workspace <target-dir>
    inspect-archive <archive> [package-name] [version]
    inspect-inventory <archive-inventory.jsonl>

Cargo owns package assembly.  This helper only derives the package set, records
Cargo's generated archives, and performs read-only archive inspection.  The
default release path is registry-independent but still uses Cargo's package
normalization; registry-sensitive dry-runs remain a separate operation.
"""

from __future__ import annotations

import hashlib
import json
import subprocess
import sys
import tarfile
import tempfile
import tomllib
from collections import defaultdict
from pathlib import Path

WORKSPACE_ROOT = Path(__file__).resolve().parent.parent
CARGO_TOML = WORKSPACE_ROOT / "Cargo.toml"
PROHIBITED_ARCHIVE_PARTS = (".git/", "target/", ".venv/", ".venv-ci/", "dist/", "exports/")


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
    tables = [(kind, data.get(kind, {})) for kind in ("dependencies", "build-dependencies", "dev-dependencies")]
    for target_name, target in data.get("target", {}).items():
        if isinstance(target, dict):
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
            dep_manifest = (manifest.parent / dep_val["path"]).resolve() / "Cargo.toml"
            package_name = dep_val.get("package", manifest_key)
            if dep_manifest.exists():
                package_name = _load_manifest(dep_manifest).get("package", {}).get("name", package_name)
            results.append({
                "name": manifest_key, "package": package_name, "kind": kind,
                "path": dep_val["path"], "version": dep_val.get("version"),
                "optional": dep_val.get("optional", False), "manifest": str(manifest),
            })
    return results


def _publishable_packages(meta: dict) -> dict[str, dict]:
    return {
        p["name"]: p for p in meta["packages"]
        if _classify(Path(p["manifest_path"])) == "publish-crates-io"
    }


def cmd_list() -> None:
    pkgs = {p["name"]: p for p in _cargo_metadata()["packages"]}
    print(f"{'Package':<25} {'Version':<10} {'Classification':<22} {'Internal Dependencies'}")
    print("-" * 100)
    for name in sorted(pkgs):
        pkg = pkgs[name]
        deps = _internal_deps(Path(pkg["manifest_path"]))
        dep_str = ", ".join(sorted(d["package"] for d in deps if not d["kind"].endswith("dev-dependencies"))) or "(none)"
        print(f"{name:<25} {pkg['version']:<10} {_classify(Path(pkg['manifest_path'])):<22} {dep_str}")


def cmd_validate() -> int:
    meta = _cargo_metadata()
    version = _workspace_version()
    pkgs = {p["name"]: p for p in meta["packages"]}
    errors: list[str] = []
    for pkg in pkgs.values():
        manifest = Path(pkg["manifest_path"])
        if _classify(manifest) != "publish-crates-io":
            continue
        for dep in _internal_deps(manifest):
            if dep["kind"].endswith("dev-dependencies"):
                continue
            dep_pkg = pkgs.get(dep["package"])
            if dep_pkg is None:
                continue
            prefix = f"{manifest}: dependency '{dep['name']}' (package '{dep['package']}')"
            if not dep.get("version"):
                errors.append(f"  {prefix} has path but no version; expected release version {version}")
            elif dep["version"] != version:
                errors.append(f"  {prefix} found version '{dep['version']}', expected release version '{version}'")
            if _classify(Path(dep_pkg["manifest_path"])) == "private-workspace":
                errors.append(f"  {prefix} is private but remains in published runtime/build dependencies")
    if errors:
        print("Validation FAILED:\n" + "\n".join(errors))
        return 1
    print("Validation passed.")
    return 0


def cmd_version_locations() -> int:
    version = _workspace_version()
    for pkg in sorted(_cargo_metadata()["packages"], key=lambda p: p["name"]):
        for dep in _internal_deps(Path(pkg["manifest_path"])):
            if dep.get("version"):
                print(f"{pkg['manifest_path']}: {dep['name']} -> {dep['package']} (version {dep['version']}; expected {version})")
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
        print(f"ERROR: cycle detected involving: {', '.join(sorted(set(pkgs) - set(order)))}", file=sys.stderr)
        return 1
    print("\n".join(order))
    return 0


def _archive_manifest_member(names: list[str]) -> str | None:
    candidates = [n for n in names if n.count("/") == 1 and n.endswith("/Cargo.toml")]
    return candidates[0] if len(candidates) == 1 else None


def _archive_record(path: Path, package: str, version: str) -> dict:
    return {"package": package, "version": version, "archive": str(path.resolve()),
            "size": path.stat().st_size, "sha256": hashlib.sha256(path.read_bytes()).hexdigest()}


def cmd_package_workspace() -> int:
    if len(sys.argv) != 3:
        print("Usage: package-workspace <target-dir>", file=sys.stderr)
        return 1
    target = Path(sys.argv[2]).resolve()
    target.mkdir(parents=True, exist_ok=True)
    packages = _publishable_packages(_cargo_metadata())
    excludes = [p["name"] for p in _cargo_metadata()["packages"] if p["name"] not in packages]
    command = ["cargo", "package", "--workspace", "--no-verify", "--target-dir", str(target)]
    for name in excludes:
        command.extend(["--exclude", name])
    # Inherit Cargo's streams so its complete diagnostics remain visible and its
    # status is returned unchanged to release-check.sh.
    result = subprocess.run(command, cwd=WORKSPACE_ROOT, check=False)
    if result.returncode:
        return result.returncode
    package_dir = target / "package"
    expected = {f"{name}-{pkg['version']}.crate": (name, pkg["version"]) for name, pkg in packages.items()}
    actual = sorted(p for p in package_dir.glob("*.crate") if p.is_file())
    actual_names = [p.name for p in actual]
    if len(actual_names) != len(set(actual_names)) or set(actual_names) != set(expected):
        print(f"Archive set mismatch: expected {sorted(expected)}, found {actual_names}", file=sys.stderr)
        return 1
    inventory = target / "archive-inventory.jsonl"
    with inventory.open("w", encoding="utf-8") as output:
        for path in sorted(actual, key=lambda p: p.name):
            name, version = expected[path.name]
            output.write(json.dumps(_archive_record(path, name, version), sort_keys=True) + "\n")
    print(f"Cargo-native archive inventory: {inventory}")
    print(f"Cargo-native archives: {len(actual)}/{len(expected)}")
    return 0


def _standalone_metadata(root: Path, manifest: Path) -> str | None:
    result = subprocess.run(
        ["cargo", "metadata", "--manifest-path", str(manifest), "--format-version", "1", "--no-deps", "--offline"],
        cwd=root, capture_output=True, text=True, check=False,
    )
    if result.returncode:
        return f"standalone cargo metadata failed: {result.stderr.strip() or result.stdout.strip()}"
    return None


def inspect_archive(archive: Path, expected_name: str | None = None, expected_version: str | None = None,
                    standalone: bool = False) -> list[str]:
    errors: list[str] = []
    if not archive.is_file():
        return [f"archive does not exist: {archive}"]
    try:
        with tarfile.open(archive, "r:gz") as tar:
            names = tar.getnames()
            manifest_name = _archive_manifest_member(names)
            if not manifest_name:
                return ["archive must contain exactly one top-level Cargo.toml"]
            if any(".." in Path(name).parts or name.startswith("/") for name in names):
                errors.append("archive contains an unsafe path")
            manifest = tomllib.loads(tar.extractfile(manifest_name).read().decode())
            package = manifest.get("package", {})
            name, version = package.get("name"), package.get("version")
            if expected_name and name != expected_name:
                errors.append(f"packaged name '{name}' does not match expected '{expected_name}'")
            if expected_version and version != expected_version:
                errors.append(f"packaged version '{version}' does not match expected '{expected_version}'")
            for member in names:
                relative = member.split("/", 1)[1] if "/" in member else member
                if any(relative.startswith(part) for part in PROHIBITED_ARCHIVE_PARTS):
                    errors.append(f"prohibited archive entry: {member}")
                if relative.endswith((".pcap", ".pcapng")):
                    errors.append(f"prohibited archive entry: {member}")
            source_manifest = None
            if expected_name:
                source_manifest = next((Path(p["manifest_path"]) for p in _cargo_metadata()["packages"] if p["name"] == expected_name), None)
            if source_manifest:
                source_data = _load_manifest(source_manifest)
                source_package = source_data.get("package", {})
                root_package = _load_manifest(CARGO_TOML).get("workspace", {}).get("package", {})
                for field in ("repository", "license", "license-file", "readme", "edition", "rust-version"):
                    expected = source_package.get(field)
                    if isinstance(expected, dict) and expected.get("workspace"):
                        expected = root_package.get(field)
                    if field in ("readme", "license-file") and isinstance(expected, str):
                        # Cargo rewrites workspace-relative package file paths
                        # to the basename stored in the published manifest.
                        expected = Path(expected).name
                    if expected is not None and package.get(field) != expected:
                        errors.append(f"packaged metadata '{field}' is '{package.get(field)}', expected '{expected}'")
                for field in ("readme", "license-file"):
                    configured = package.get(field)
                    if configured and not any(Path(n).name == Path(configured).name for n in names):
                        errors.append(f"packaged {field} is missing: {configured}")
                package_names = {p["name"] for p in _cargo_metadata()["packages"]}
                private_names = {p["name"] for p in _cargo_metadata()["packages"] if _classify(Path(p["manifest_path"])) == "private-workspace"}
                for kind, table in _dependency_tables(manifest):
                    for dep_name, dep_val in table.items():
                        if not isinstance(dep_val, dict):
                            continue
                        if "path" in dep_val:
                            errors.append(f"packaged dependency '{dep_name}' retains local path in {kind}")
                        dep_package = dep_val.get("package", dep_name)
                        if dep_package in private_names:
                            errors.append(f"packaged dependency '{dep_name}' targets private package '{dep_package}'")
                        if dep_package in package_names and dep_package != expected_name and dep_val.get("version") != expected_version:
                            errors.append(f"packaged internal dependency '{dep_name}' has version '{dep_val.get('version')}', expected '{expected_version}'")
                features = manifest.get("features", {})
                valid_feature_names = set(features) | {key for _, table in _dependency_tables(manifest) for key, value in table.items() if isinstance(value, dict) and value.get("optional")}
                for feature_name, refs in features.items():
                    for ref in refs if isinstance(refs, list) else []:
                        base = ref.removeprefix("dep:").split("/", 1)[0].removesuffix("?")
                        if base and base not in valid_feature_names:
                            errors.append(f"feature '{feature_name}' references unknown feature or optional dependency '{base}'")
                for forbidden in ("workspace", "patch", "replace"):
                    if forbidden in manifest:
                        errors.append(f"packaged manifest retains [{forbidden}]")
                def has_workspace(value: object) -> bool:
                    if isinstance(value, dict):
                        if value.get("workspace") is True:
                            return True
                        return any(has_workspace(v) for v in value.values())
                    if isinstance(value, list):
                        return any(has_workspace(v) for v in value)
                    return False
                if has_workspace(manifest):
                    errors.append("packaged manifest retains workspace inheritance")
            if standalone and not errors:
                with tempfile.TemporaryDirectory(prefix="eggsec-crate-") as directory:
                    destination = Path(directory)
                    tar.extractall(destination)
                    extracted = destination / manifest_name.split("/", 1)[0]
                    metadata_error = _standalone_metadata(destination, extracted / "Cargo.toml")
                    if metadata_error:
                        errors.append(metadata_error)
    except (tarfile.TarError, tomllib.TOMLDecodeError, UnicodeDecodeError, OSError) as exc:
        errors.append(f"cannot inspect archive: {exc}")
    return errors


def cmd_inspect_archive() -> int:
    if len(sys.argv) < 3:
        print("Usage: inspect-archive <path-to-crate> [package-name] [version]", file=sys.stderr)
        return 1
    archive = Path(sys.argv[2])
    expected_name = sys.argv[3] if len(sys.argv) > 3 else None
    expected_version = sys.argv[4] if len(sys.argv) > 4 else _workspace_version()
    errors = inspect_archive(archive, expected_name, expected_version, standalone=True)
    if errors:
        print("Archive inspection FAILED:\n" + "\n".join(f"  {error}" for error in errors))
        return 1
    print(f"Archive inspection passed: {archive}")
    return 0


def cmd_inspect_inventory() -> int:
    if len(sys.argv) != 3:
        print("Usage: inspect-inventory <archive-inventory.jsonl>", file=sys.stderr)
        return 1
    inventory = Path(sys.argv[2])
    try:
        records = [json.loads(line) for line in inventory.read_text().splitlines() if line.strip()]
    except (OSError, json.JSONDecodeError) as exc:
        print(f"Invalid archive inventory: {exc}", file=sys.stderr)
        return 1
    for record in records:
        archive = Path(record["archive"])
        if not archive.is_file() or archive.stat().st_size != record["size"] or hashlib.sha256(archive.read_bytes()).hexdigest() != record["sha256"]:
            print(f"Archive inventory mismatch: {archive}", file=sys.stderr)
            return 1
        errors = inspect_archive(archive, record["package"], record["version"], standalone=True)
        if errors:
            print(f"Archive inspection FAILED: {archive}\n" + "\n".join(f"  {error}" for error in errors), file=sys.stderr)
            return 1
        print(json.dumps(record, sort_keys=True))
    print(f"Rust Cargo archives: PASS ({len(records)} Cargo-generated, parsed, and inspected)")
    return 0


def main() -> int:
    if len(sys.argv) < 2:
        print(__doc__.strip(), file=sys.stderr)
        return 1
    commands = {"list": lambda: (cmd_list() or 0), "validate": cmd_validate, "order": cmd_order,
                "version-locations": cmd_version_locations, "package-workspace": cmd_package_workspace,
                "inspect-archive": cmd_inspect_archive, "inspect-inventory": cmd_inspect_inventory}
    command = commands.get(sys.argv[1])
    if command is None:
        print(f"Unknown command: {sys.argv[1]}", file=sys.stderr)
        return 1
    return command()


if __name__ == "__main__":
    sys.exit(main())
