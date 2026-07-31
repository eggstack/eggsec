#!/usr/bin/env python3
"""Focused tests for release-package-graph.py validation and ordering logic.

Uses fixture Cargo.toml files in temp directories to test invariants.
"""

import io
import importlib.util
import json
import os
import shutil
import subprocess
import tarfile
import tempfile
import tomllib
import unittest
from contextlib import redirect_stdout
from pathlib import Path
from unittest.mock import patch

# Import the helper module
_spec = importlib.util.spec_from_file_location(
    "rpg", Path(__file__).parent / "release-package-graph.py"
)
_rpg = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_rpg)


def _write_cargo_toml(path, name=None, publish=None, deps=None):
    """Write a minimal Cargo.toml for fixture use."""
    path.parent.mkdir(parents=True, exist_ok=True)
    pkg_name = name or path.parent.name
    lines = ['[package]', f'name = "{pkg_name}"', 'version = "0.1.0"']
    if publish is False:
        lines.append("publish = false")
    if deps:
        by_kind: dict[str, list] = {}
        for dep in deps:
            kind = dep.get("kind", "dependencies")
            by_kind.setdefault(kind, []).append(dep)
        for kind in ("dependencies", "build-dependencies", "dev-dependencies"):
            group = by_kind.get(kind)
            if not group:
                continue
            lines.append("")
            lines.append(f"[{kind}]")
            for dep in group:
                parts = [f'{dep["name"]} = {{ path = "{dep["path"]}"']
                if "version" in dep:
                    parts[0] += f', version = "{dep["version"]}"'
                if dep.get("optional"):
                    parts[0] += ", optional = true"
                parts[0] += " }"
                lines.append(parts[0])
    path.write_text("\n".join(lines) + "\n")


def _make_metadata(tmpdir, packages, workspace_version="0.1.0"):
    """Build a minimal cargo metadata dict from package descriptors in tmpdir."""
    ws_packages = []
    for pkg in packages:
        manifest = tmpdir / pkg["name"] / "Cargo.toml"
        ws_packages.append({
            "name": pkg["name"],
            "manifest_path": str(manifest),
            "version": pkg.get("version", workspace_version),
        })
    return {"packages": ws_packages}


class TestClassification(unittest.TestCase):
    """Tests for crate classification (publish-crates-io vs private-workspace)."""

    def setUp(self):
        self.tmpdir = Path(tempfile.mkdtemp())

    def tearDown(self):
        shutil.rmtree(self.tmpdir)

    def test_publish_false_is_private(self):
        cargo = self.tmpdir / "foo" / "Cargo.toml"
        _write_cargo_toml(cargo, name="foo", publish=False)
        assert _rpg._classify(cargo) == "private-workspace"

    def test_default_is_publish(self):
        cargo = self.tmpdir / "foo" / "Cargo.toml"
        _write_cargo_toml(cargo, name="foo")
        assert _rpg._classify(cargo) == "publish-crates-io"


class TestInternalDeps(unittest.TestCase):
    """Tests for internal dependency extraction."""

    def setUp(self):
        self.tmpdir = Path(tempfile.mkdtemp())

    def tearDown(self):
        shutil.rmtree(self.tmpdir)

    def test_path_only_normal_dep(self):
        cargo = self.tmpdir / "foo" / "Cargo.toml"
        _write_cargo_toml(cargo, name="foo", deps=[{"name": "bar", "path": "../bar"}])
        deps = _rpg._internal_deps(cargo)
        self.assertEqual(len(deps), 1)
        self.assertEqual(deps[0]["kind"], "dependencies")
        self.assertIsNone(deps[0]["version"])

    def test_path_and_version_dep(self):
        cargo = self.tmpdir / "foo" / "Cargo.toml"
        _write_cargo_toml(cargo, name="foo", deps=[{"name": "bar", "path": "../bar", "version": "0.1.0"}])
        deps = _rpg._internal_deps(cargo)
        self.assertEqual(len(deps), 1)
        self.assertEqual(deps[0]["version"], "0.1.0")

    def test_optional_dep(self):
        cargo = self.tmpdir / "foo" / "Cargo.toml"
        _write_cargo_toml(cargo, name="foo", deps=[{"name": "bar", "path": "../bar", "version": "0.1.0", "optional": True}])
        deps = _rpg._internal_deps(cargo)
        self.assertEqual(len(deps), 1)
        self.assertTrue(deps[0]["optional"])

    def test_dev_dep_included(self):
        cargo = self.tmpdir / "foo" / "Cargo.toml"
        path = cargo
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(
            '[package]\nname = "foo"\nversion = "0.1.0"\n\n'
            '[dev-dependencies]\nbar = { path = "../bar" }\n'
        )
        deps = _rpg._internal_deps(cargo)
        self.assertEqual(len(deps), 1)
        self.assertEqual(deps[0]["kind"], "dev-dependencies")

    def test_build_dep_included(self):
        cargo = self.tmpdir / "foo" / "Cargo.toml"
        path = cargo
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(
            '[package]\nname = "foo"\nversion = "0.1.0"\n\n'
            '[build-dependencies]\nbar = { path = "../bar", version = "0.1.0" }\n'
        )
        deps = _rpg._internal_deps(cargo)
        self.assertEqual(len(deps), 1)
        self.assertEqual(deps[0]["kind"], "build-dependencies")

    def test_external_dep_ignored(self):
        cargo = self.tmpdir / "foo" / "Cargo.toml"
        path = cargo
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(
            '[package]\nname = "foo"\nversion = "0.1.0"\n\n'
            '[dependencies]\nserde = "1.0"\nbar = { path = "../bar", version = "0.1.0" }\n'
        )
        deps = _rpg._internal_deps(cargo)
        self.assertEqual(len(deps), 1)
        self.assertEqual(deps[0]["name"], "bar")

    def test_target_specific_and_alias_resolve_to_package(self):
        cargo = self.tmpdir / "foo" / "Cargo.toml"
        (self.tmpdir / "real-bar").mkdir()
        (self.tmpdir / "real-bar" / "Cargo.toml").write_text(
            '[package]\nname = "real-bar"\nversion = "0.1.0"\n'
        )
        cargo.parent.mkdir(parents=True, exist_ok=True)
        cargo.write_text(
            '[package]\nname = "foo"\nversion = "0.1.0"\n\n'
            '[target."cfg(unix)".dependencies]\n'
            'bar_alias = { package = "real-bar", path = "../real-bar", version = "0.1.0", optional = true }\n'
        )
        deps = _rpg._internal_deps(cargo)
        self.assertEqual(len(deps), 1)
        self.assertEqual(deps[0]["name"], "bar_alias")
        self.assertEqual(deps[0]["package"], "real-bar")
        self.assertTrue(deps[0]["optional"])


class TestValidate(unittest.TestCase):
    """Tests for the validate command logic."""

    def setUp(self):
        self.tmpdir = Path(tempfile.mkdtemp())

    def tearDown(self):
        shutil.rmtree(self.tmpdir)

    def _run_validate(self, manifests, workspace_version="0.1.0"):
        """Helper: create fixture Cargo.toml files and run validate.

        manifests: dict of name -> {publish, deps}
        Returns (exit_code, stdout_text).
        """
        packages = []
        for name, spec in manifests.items():
            cargo = self.tmpdir / name / "Cargo.toml"
            _write_cargo_toml(cargo, name=name, publish=spec.get("publish"), deps=spec.get("deps"))
            packages.append({"name": name})

        metadata = _make_metadata(self.tmpdir, packages, workspace_version)

        original_cargo_metadata = _rpg._cargo_metadata
        original_workspace_version = _rpg._workspace_version
        _rpg._cargo_metadata = lambda: metadata
        _rpg._workspace_version = lambda: workspace_version

        buf = io.StringIO()
        with redirect_stdout(buf):
            exit_code = _rpg.cmd_validate()

        _rpg._cargo_metadata = original_cargo_metadata
        _rpg._workspace_version = original_workspace_version

        return exit_code, buf.getvalue()

    def test_path_only_normal_dep_rejected(self):
        code, output = self._run_validate({
            "crate-a": {"deps": [{"name": "crate-b", "path": "../crate-b"}]},
            "crate-b": {"deps": []},
        })
        self.assertEqual(code, 1)
        self.assertIn("has path but no version", output)

    def test_path_and_version_dep_accepted(self):
        code, output = self._run_validate({
            "crate-a": {"deps": [{"name": "crate-b", "path": "../crate-b", "version": "0.1.0"}]},
            "crate-b": {"deps": []},
        })
        self.assertEqual(code, 0)
        self.assertIn("passed", output.lower())

    def test_private_dep_rejected_for_publishable(self):
        code, output = self._run_validate({
            "crate-a": {"deps": [{"name": "crate-priv", "path": "../crate-priv", "version": "0.1.0"}]},
            "crate-priv": {"publish": False, "deps": []},
        })
        self.assertEqual(code, 1)
        self.assertIn("private", output.lower())

    def test_dev_dep_on_private_allowed(self):
        code, output = self._run_validate({
            "crate-a": {"deps": [{"name": "crate-priv", "path": "../crate-priv", "kind": "dev-dependencies"}]},
            "crate-priv": {"publish": False, "deps": []},
        })
        # dev deps are skipped in validation, so this should pass
        self.assertEqual(code, 0)

    def test_version_mismatch_rejected(self):
        code, output = self._run_validate(
            {
                "crate-a": {"deps": [{"name": "crate-b", "path": "../crate-b", "version": "0.2.0"}]},
                "crate-b": {"deps": []},
            },
            workspace_version="0.1.0",
        )
        self.assertEqual(code, 1)
        self.assertIn("0.2.0", output)
        self.assertIn("0.1.0", output)


class TestOrdering(unittest.TestCase):
    """Tests for topological ordering logic."""

    def setUp(self):
        self.tmpdir = Path(tempfile.mkdtemp())

    def tearDown(self):
        shutil.rmtree(self.tmpdir)

    def _run_order(self, manifests, workspace_version="0.1.0"):
        """Helper: create fixture Cargo.toml files and run order.

        manifests: dict of name -> {publish, deps}
        Returns (exit_code, ordered_names).
        """
        packages = []
        for name, spec in manifests.items():
            cargo = self.tmpdir / name / "Cargo.toml"
            _write_cargo_toml(cargo, name=name, publish=spec.get("publish"), deps=spec.get("deps"))
            packages.append({"name": name})

        metadata = _make_metadata(self.tmpdir, packages, workspace_version)

        original_cargo_metadata = _rpg._cargo_metadata
        _rpg._cargo_metadata = lambda: metadata

        buf = io.StringIO()
        with redirect_stdout(buf):
            exit_code = _rpg.cmd_order()

        _rpg._cargo_metadata = original_cargo_metadata

        lines = [l.strip() for l in buf.getvalue().strip().splitlines() if l.strip()]
        return exit_code, lines

    def test_linear_order(self):
        code, order = self._run_order({
            "a": {"deps": [{"name": "b", "path": "../b", "version": "0.1.0"}]},
            "b": {"deps": [{"name": "c", "path": "../c", "version": "0.1.0"}]},
            "c": {"deps": []},
        })
        self.assertEqual(code, 0)
        self.assertEqual(order, ["c", "b", "a"])

    def test_diamond_order(self):
        code, order = self._run_order({
            "top": {"deps": [
                {"name": "left", "path": "../left", "version": "0.1.0"},
                {"name": "right", "path": "../right", "version": "0.1.0"},
            ]},
            "left": {"deps": [{"name": "bottom", "path": "../bottom", "version": "0.1.0"}]},
            "right": {"deps": [{"name": "bottom", "path": "../bottom", "version": "0.1.0"}]},
            "bottom": {"deps": []},
        })
        self.assertEqual(code, 0)
        idx = {name: i for i, name in enumerate(order)}
        self.assertLess(idx["bottom"], idx["left"])
        self.assertLess(idx["bottom"], idx["right"])
        self.assertLess(idx["left"], idx["top"])
        self.assertLess(idx["right"], idx["top"])

    def test_cycle_detected(self):
        code, order = self._run_order({
            "a": {"deps": [{"name": "b", "path": "../b", "version": "0.1.0"}]},
            "b": {"deps": [{"name": "a", "path": "../a", "version": "0.1.0"}]},
        })
        self.assertEqual(code, 1)

    def test_private_excluded_from_ordering(self):
        code, order = self._run_order({
            "pub-a": {"deps": [{"name": "priv-b", "path": "../priv-b", "version": "0.1.0"}]},
            "priv-b": {"publish": False, "deps": []},
        })
        self.assertEqual(code, 0)
        self.assertNotIn("priv-b", order)
        self.assertIn("pub-a", order)

    def test_optional_dep_included_in_ordering(self):
        code, order = self._run_order({
            "main": {"deps": [{"name": "opt", "path": "../opt", "version": "0.1.0", "optional": True}]},
            "opt": {"deps": []},
        })
        self.assertEqual(code, 0)
        self.assertEqual(order, ["opt", "main"])

    def test_stable_tiebreaking(self):
        code, order = self._run_order({
            "z-pkg": {"deps": []},
            "a-pkg": {"deps": []},
            "m-pkg": {"deps": []},
        })
        self.assertEqual(code, 0)
        self.assertEqual(order, ["a-pkg", "m-pkg", "z-pkg"])


class TestEndToEnd(unittest.TestCase):
    """End-to-end tests against the real workspace."""

    def test_validate_real_workspace(self):
        """The real workspace must pass validation."""
        import subprocess
        result = subprocess.run(
            ["python3", str(Path(__file__).parent / "release-package-graph.py"), "validate"],
            capture_output=True, text=True, cwd=Path(__file__).parent.parent,
        )
        self.assertEqual(result.returncode, 0, f"Validation failed:\n{result.stdout}\n{result.stderr}")

    def test_order_real_workspace(self):
        """The real workspace must produce a valid acyclic order."""
        import subprocess
        result = subprocess.run(
            ["python3", str(Path(__file__).parent / "release-package-graph.py"), "order"],
            capture_output=True, text=True, cwd=Path(__file__).parent.parent,
        )
        self.assertEqual(result.returncode, 0, f"Ordering failed:\n{result.stdout}\n{result.stderr}")
        lines = [l.strip() for l in result.stdout.strip().splitlines() if l.strip()]
        self.assertIn("eggsec-core", lines)
        self.assertIn("eggsec", lines)
        self.assertLess(lines.index("eggsec-core"), lines.index("eggsec"))


class TestArchiveInspection(unittest.TestCase):
    def setUp(self):
        self.tmpdir = Path(tempfile.mkdtemp())
        self.source = self.tmpdir / "fixture" / "Cargo.toml"
        self.source.parent.mkdir(parents=True)
        self.source.write_text(
            '[package]\nname = "fixture"\nversion = "0.1.0"\n'
            'repository = "https://example.invalid/eggsec"\nlicense = "MIT"\n'
            'rust-version = "1.80"\n'
        )
        self.other = self.tmpdir / "other" / "Cargo.toml"
        self.other.parent.mkdir()
        self.other.write_text('[package]\nname = "eggsec-other"\nversion = "0.1.0"\n')
        self.old_metadata = _rpg._cargo_metadata
        self.old_version = _rpg._workspace_version
        _rpg._cargo_metadata = lambda: {"packages": [
            {"name": "fixture", "manifest_path": str(self.source)},
            {"name": "eggsec-other", "manifest_path": str(self.other)},
        ]}
        _rpg._workspace_version = lambda: "0.1.0"

    def tearDown(self):
        _rpg._cargo_metadata = self.old_metadata
        _rpg._workspace_version = self.old_version
        shutil.rmtree(self.tmpdir)

    def _archive(self, manifest_text, members=("README.md", "LICENSE-MIT")):
        archive = self.tmpdir / "fixture-0.1.0.crate"
        with tarfile.open(archive, "w:gz") as tar:
            root = "fixture-0.1.0/"
            for name, content in (("Cargo.toml", manifest_text), *[(m, "content") for m in members]):
                info = tarfile.TarInfo(root + name)
                data = content.encode()
                info.size = len(data)
                tar.addfile(info, io.BytesIO(data))
        return archive

    def test_normalized_registry_dependency_is_accepted(self):
        archive = self._archive(
            '[package]\nname = "fixture"\nversion = "0.1.0"\n'
            'repository = "https://example.invalid/eggsec"\nlicense = "MIT"\nrust-version = "1.80"\n\n'
            '[dependencies]\nother = { version = "0.1.0" }\n'
        )
        self.assertEqual(_rpg.inspect_archive(archive, "fixture", "0.1.0"), [])

    def test_retained_path_and_version_mismatch_are_rejected(self):
        archive = self._archive(
            '[package]\nname = "fixture"\nversion = "0.1.0"\n'
            'repository = "https://example.invalid/eggsec"\nlicense = "MIT"\nrust-version = "1.80"\n\n'
            '[dependencies]\nother = { package = "eggsec-other", path = "../other", version = "0.2.0" }\n'
        )
        errors = _rpg.inspect_archive(archive, "fixture", "0.1.0")
        self.assertTrue(any("retains local path" in error for error in errors))
        self.assertTrue(any("version '0.2.0'" in error for error in errors))

    def test_prohibited_entries_are_rejected(self):
        archive = self._archive(
            '[package]\nname = "fixture"\nversion = "0.1.0"\n'
            'repository = "https://example.invalid/eggsec"\nlicense = "MIT"\nrust-version = "1.80"\n',
            members=("README.md", "target/bad.o"),
        )
        errors = _rpg.inspect_archive(archive, "fixture", "0.1.0")
        self.assertTrue(any("prohibited archive entry" in error for error in errors))

    def test_missing_readme_is_reported(self):
        archive = self._archive(
            '[package]\nname = "fixture"\nversion = "0.1.0"\nreadme = "README.md"\n'
            'repository = "https://example.invalid/eggsec"\nlicense = "MIT"\nrust-version = "1.80"\n',
            members=("LICENSE-MIT",),
        )
        errors = _rpg.inspect_archive(archive, "fixture", "0.1.0")
        self.assertTrue(any("README" in error for error in errors))


class TestReleaseContract(unittest.TestCase):
    def test_script_does_not_classify_registry_failure_as_pass(self):
        script = (Path(__file__).parent / "release-check.sh").read_text()
        helper = (Path(__file__).parent / "release-package-graph.py").read_text()
        self.assertIn("package-workspace", script)
        self.assertIn("inspect-inventory", script)
        self.assertIn('"cargo", "package", "--workspace", "--no-verify"', helper)
        self.assertNotIn("cmd_" + "create_archive", helper)
        self.assertNotIn("tarfile.open(output, \"w:gz\")", helper)
        self.assertNotIn("map" + "file", script)
        self.assertIn("Registry preflight: SKIPPED", script)
        self.assertNotIn("PACKAGE_FIRST_RELEASE", script)
        self.assertNotIn('grep -q "no matching package named"', script)

    def test_synthetic_workspace_package_command_normalizes_inheritance(self):
        root = self.tmpdir = Path(tempfile.mkdtemp())
        try:
            for name in ("dep", "app", "private"):
                (root / name / "src").mkdir(parents=True)
            (root / "Cargo.toml").write_text(
                '[workspace]\nmembers = ["app", "dep", "private"]\nresolver = "2"\n'
                '[workspace.package]\nversion = "0.1.0"\nedition = "2021"\nlicense = "MIT"\n'
                'repository = "https://example.invalid/fixture"\n\n'
                '[workspace.dependencies]\n'
                'alias = { package = "eggsec-fixture-dep-9f4c", path = "dep", version = "0.1.0" }\n'
            )
            (root / "dep" / "Cargo.toml").write_text(
                '[package]\nname = "eggsec-fixture-dep-9f4c"\nversion.workspace = true\n'
                'edition.workspace = true\nlicense.workspace = true\nrepository.workspace = true\n'
            )
            (root / "app" / "Cargo.toml").write_text(
                '[package]\nname = "eggsec-fixture-app-9f4c"\nversion.workspace = true\n'
                'edition.workspace = true\nlicense.workspace = true\nrepository.workspace = true\n\n'
                '[dependencies]\nalias = { package = "eggsec-fixture-dep-9f4c", workspace = true, optional = true }\n\n'
                '[target."cfg(unix)".dependencies]\nalias = { workspace = true }\n\n'
                '[features]\noptional-dep = ["dep:alias"]\n'
            )
            (root / "app" / "src" / "lib.rs").write_text("")
            (root / "dep" / "src" / "lib.rs").write_text("")
            (root / "private" / "Cargo.toml").write_text(
                '[package]\nname = "eggsec-fixture-private-9f4c"\nversion.workspace = true\npublish = false\n'
            )
            (root / "private" / "src" / "lib.rs").write_text("")
            target = root / "proof-target"
            metadata = {"packages": [
                {"name": "eggsec-fixture-app-9f4c", "version": "0.1.0", "manifest_path": str(root / "app" / "Cargo.toml")},
                {"name": "eggsec-fixture-dep-9f4c", "version": "0.1.0", "manifest_path": str(root / "dep" / "Cargo.toml")},
                {"name": "eggsec-fixture-private-9f4c", "version": "0.1.0", "manifest_path": str(root / "private" / "Cargo.toml")},
            ]}
            old_root, old_cargo, old_metadata = _rpg.WORKSPACE_ROOT, _rpg.CARGO_TOML, _rpg._cargo_metadata
            try:
                _rpg.WORKSPACE_ROOT = root
                _rpg.CARGO_TOML = root / "Cargo.toml"
                _rpg._cargo_metadata = lambda: metadata
                old_argv = _rpg.sys.argv
                _rpg.sys.argv = ["release-package-graph.py", "package-workspace", str(target)]
                self.assertEqual(_rpg.cmd_package_workspace(), 0)
                _rpg.sys.argv = old_argv
            finally:
                _rpg.WORKSPACE_ROOT, _rpg.CARGO_TOML, _rpg._cargo_metadata = old_root, old_cargo, old_metadata
            inventory = target / "archive-inventory.jsonl"
            records = [json.loads(line) for line in inventory.read_text().splitlines()]
            self.assertEqual({record["package"] for record in records}, {"eggsec-fixture-app-9f4c", "eggsec-fixture-dep-9f4c"})
            self.assertNotIn("eggsec-fixture-private-9f4c", {record["package"] for record in records})
            app_archive = target / "package" / "eggsec-fixture-app-9f4c-0.1.0.crate"
            errors = _rpg.inspect_archive(app_archive, "eggsec-fixture-app-9f4c", "0.1.0", standalone=True)
            self.assertEqual(errors, [], errors)
            with tarfile.open(app_archive, "r:gz") as tar:
                manifest = tomllib.loads(tar.extractfile("eggsec-fixture-app-9f4c-0.1.0/Cargo.toml").read().decode())
            self.assertEqual(manifest["package"]["license"], "MIT")
            self.assertEqual(manifest["dependencies"]["alias"]["package"], "eggsec-fixture-dep-9f4c")
            self.assertTrue(manifest["dependencies"]["alias"]["optional"])
            self.assertIn("target", manifest)
        finally:
            shutil.rmtree(root)

    def test_package_command_returns_cargo_failure_status(self):
        old_metadata = _rpg._cargo_metadata
        old_argv = _rpg.sys.argv
        try:
            root = Path(tempfile.mkdtemp())
            (root / "Cargo.toml").write_text('[package]\nname = "fixture"\nversion = "0.1.0"\n')
            _rpg._cargo_metadata = lambda: {"packages": [{"name": "fixture", "version": "0.1.0", "manifest_path": str(root / "Cargo.toml")}]}
            _rpg.sys.argv = ["release-package-graph.py", "package-workspace", str(root / "target")]
            with patch.object(_rpg.subprocess, "run", return_value=type("Result", (), {"returncode": 37})()):
                self.assertEqual(_rpg.cmd_package_workspace(), 37)
        finally:
            _rpg._cargo_metadata = old_metadata
            _rpg.sys.argv = old_argv
            shutil.rmtree(root)


if __name__ == "__main__":
    unittest.main()
