#!/usr/bin/env python3
"""Verify that repository-local paths referenced in documentation exist on disk.

Scans .md files, Makefile, AGENTS.md, AGENTS.override.md, and SKILL.md files
for paths that look like repository-local references (scripts/, crates/, tests/,
docs/, etc.) and checks that each referenced path exists relative to the
workspace root.

Paths prefixed with TODO: or PLACEHOLDER: are skipped.
Files under advisory-db/, .git/, target/, node_modules/ are skipped entirely.
"""

from __future__ import annotations

import glob
import os
import re
import sys

# Workspace root is two levels up from this script (scripts/ -> repo root)
WORKSPACE_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# Directories to skip entirely during glob collection
SKIP_DIR_NAMES = {
    "node_modules", ".git", "target", "__pycache__", ".venv", "venv",
    "dist", "build", ".eggs", "advisory-db",
}

# Patterns that look like repository-local file/directory references.
# We capture the path portion only (no surrounding punctuation or markdown syntax).
PATH_PATTERNS: list[re.Pattern[str]] = [
    # Explicit relative paths: scripts/foo.py, crates/eggsec/src/lib.rs
    re.compile(r"(?<![`\w/])(\.{0,2}(?:scripts|crates|tests|docs|architecture|themes|plans)/[A-Za-z0-9_./-]+\.[A-Za-z0-9]+)"),
    # Quoted paths: "scripts/foo.py"
    re.compile(r'"((?:scripts|crates|tests|docs|architecture|themes|plans)/[A-Za-z0-9_./-]+)"'),
    # File references with explicit extensions in running text
    re.compile(r"\b((?:scripts|crates|tests|docs|architecture)/[A-Za-z0-9_./-]+\.(?:rs|py|sh|toml|yml|yaml|md|json|lua))\b"),
    # AGENTS.override.md references
    re.compile(r"\b([A-Za-z0-9_./-]+/AGENTS\.override\.md)\b"),
    # GitHub workflow references
    re.compile(r"\b((?:\.github/workflows)/[A-Za-z0-9_./-]+\.(?:yml|yaml))\b"),
]

# Explicit placeholders to skip
PLACEHOLDER_RE = re.compile(r"(?:TODO|PLACEHOLDER):")


def _is_in_skipped_dir(path: str) -> bool:
    """Check if a file path lives under any skipped directory."""
    parts = path.replace("\\", "/").split("/")
    return any(p in SKIP_DIR_NAMES for p in parts)


def find_doc_files() -> list[str]:
    """Find all markdown files, Makefile, and special config files."""
    patterns = [
        os.path.join(WORKSPACE_ROOT, "**", "*.md"),
        os.path.join(WORKSPACE_ROOT, "Makefile"),
        os.path.join(WORKSPACE_ROOT, "AGENTS.md"),
        os.path.join(WORKSPACE_ROOT, "AGENTS.override.md"),
        os.path.join(WORKSPACE_ROOT, "**", "SKILL.md"),
    ]
    files: set[str] = set()
    for pattern in patterns:
        files.update(glob.glob(pattern, recursive=True))

    # Filter out skipped directories and non-text files
    filtered: list[str] = []
    for f in sorted(files):
        rel = os.path.relpath(f, WORKSPACE_ROOT)
        if _is_in_skipped_dir(rel):
            continue
        if f.endswith((".png", ".jpg", ".gif", ".pdf", ".whl", ".tar.gz")):
            continue
        filtered.append(f)
    return filtered


def extract_paths(content: str) -> list[str]:
    """Extract repository-local path references from file content."""
    found: set[str] = set()
    for line in content.splitlines():
        if PLACEHOLDER_RE.search(line):
            continue
        for pat in PATH_PATTERNS:
            for match in pat.finditer(line):
                path = match.group(1)
                path = path.strip("./")
                if not path or len(path) < 3:
                    continue
                # Skip common false positives
                if path.startswith(("http", "https", "ftp")):
                    continue
                # Skip Python module-style imports (e.g. eggsec.cli)
                if re.match(r"^[a-z][a-z0-9_]+(\.[a-z][a-z0-9_]+)+$", path):
                    continue
                found.add(path)
    return sorted(found)


def check_path_exists(path: str) -> bool:
    """Check if a path exists relative to workspace root (as file or directory)."""
    full = os.path.join(WORKSPACE_ROOT, path)
    return os.path.exists(full)


def main() -> int:
    doc_files = find_doc_files()
    missing: list[tuple[str, str]] = []

    for doc_file in doc_files:
        try:
            with open(doc_file, encoding="utf-8", errors="replace") as f:
                content = f.read()
        except OSError as e:
            print(f"WARNING: Could not read {doc_file}: {e}", file=sys.stderr)
            continue

        paths = extract_paths(content)
        for path in paths:
            if not check_path_exists(path):
                rel_doc = os.path.relpath(doc_file, WORKSPACE_ROOT)
                missing.append((rel_doc, path))

    if missing:
        print(f"\nERROR: {len(missing)} missing file reference(s) found:\n")
        for doc, path in missing:
            print(f"  {doc} -> {path}")
        print(f"\nTotal missing: {len(missing)}")
        return 1

    print(f"OK: All file references in {len(doc_files)} documentation files resolve to existing paths.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
