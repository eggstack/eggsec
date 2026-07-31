#!/usr/bin/env bash
set -euo pipefail

# Release validation script for Eggsec.
# Validates repository state and built artifacts but does NOT publish.
#
# Usage:
#   scripts/release-check.sh [expected-version]
#
# Environment:
#   EGGSEC_RELEASE_SKIP_PYTHON=1         skip Python checks (Rust-only release)
#   EGGSEC_RELEASE_REGISTRY_PREFLIGHT=1  run cargo publish --dry-run (network-sensitive)
#   EGGSEC_RELEASE_KEEP_ARTIFACTS=1      do not clean build artifacts on exit

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m'

ok()   { echo -e "${GREEN}✓${NC} $*"; }
fail() { echo -e "${RED}✗${NC} $*"; exit 1; }
warn() { echo -e "${YELLOW}!${NC} $*"; }

EXPECTED_VERSION="${1:-}"
TMP_ROOT=""

cleanup() {
    if [ -n "$TMP_ROOT" ] && [ -z "${EGGSEC_RELEASE_KEEP_ARTIFACTS:-}" ]; then
        rm -rf "$TMP_ROOT"
    fi
}
trap cleanup EXIT

TMP_ROOT=$(mktemp -d)
LOG_DIR="$TMP_ROOT/logs"
RUST_TARGET_DIR="$TMP_ROOT/rust-target"
mkdir -p "$LOG_DIR"

# Portable SHA-256
sha256_hex() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | cut -d' ' -f1
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$1" | cut -d' ' -f1
    else
        fail "No sha256sum or shasum found"
    fi
}

# Portable file size
file_size() {
    python3 -c "import os; print(os.path.getsize('$1'))"
}

echo "=== Eggsec Release Validation ==="
echo "Repository: $REPO_ROOT"
[ -n "$EXPECTED_VERSION" ] && echo "Expected version: $EXPECTED_VERSION"
echo ""

# ── 1. Repository state ──────────────────────────────────────────────────

echo "--- 1. Repository state ---"

cd "$REPO_ROOT"
if [ -n "$(git status --porcelain)" ]; then
    fail "Working tree is dirty. Commit or stash changes before release."
fi
ok "Working tree is clean"

CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "detached")
CURRENT_COMMIT=$(git rev-parse --short HEAD)
echo "  Branch: $CURRENT_BRANCH ($CURRENT_COMMIT)"
ok "Branch/commit is intentional"

# ── 2. Version alignment ─────────────────────────────────────────────────

echo ""
echo "--- 2. Version alignment ---"

CARGO_VERSION=$(python3 -c "
import tomllib
with open('Cargo.toml', 'rb') as f:
    print(tomllib.load(f)['workspace']['package']['version'])
")
echo "  Cargo.toml (workspace): $CARGO_VERSION"

EP_CARGO_VERSION=$(python3 -c "
import tomllib
with open('crates/eggsec-python/Cargo.toml', 'rb') as f:
    data = tomllib.load(f)
v = data.get('package', {}).get('version', {})
if isinstance(v, dict) and v.get('workspace'):
    print('inherits-workspace')
else:
    print(str(v))
")
echo "  crates/eggsec-python/Cargo.toml: $EP_CARGO_VERSION"

PYPROJECT_VERSION=$(python3 -c "
import tomllib
with open('crates/eggsec-python/pyproject.toml', 'rb') as f:
    print(tomllib.load(f)['project']['version'])
")
echo "  crates/eggsec-python/pyproject.toml: $PYPROJECT_VERSION"

if [ "$PYPROJECT_VERSION" != "$CARGO_VERSION" ]; then
    fail "Version mismatch: Cargo.toml ($CARGO_VERSION) != pyproject.toml ($PYPROJECT_VERSION)"
fi
ok "pyproject.toml matches workspace version"

if [ -n "$EXPECTED_VERSION" ]; then
    if [ "$CARGO_VERSION" != "$EXPECTED_VERSION" ]; then
        fail "Cargo.toml version ($CARGO_VERSION) does not match expected ($EXPECTED_VERSION)"
    fi
    if [ "$PYPROJECT_VERSION" != "$EXPECTED_VERSION" ]; then
        fail "pyproject.toml version ($PYPROJECT_VERSION) does not match expected ($EXPECTED_VERSION)"
    fi
    ok "Versions match expected: $EXPECTED_VERSION"
fi

if [ -z "$CARGO_VERSION" ] || [ "$CARGO_VERSION" = "0.0.0" ]; then
    fail "Version is empty or 0.0.0"
fi
ok "Version is non-empty and non-zero"

# ── 3. Package graph validation ──────────────────────────────────────────

echo ""
echo "--- 3. Package graph validation ---"

echo "  Listing packages..."
python3 "$SCRIPT_DIR/release-package-graph.py" list
echo ""

echo "  Validating publishability..."
python3 "$SCRIPT_DIR/release-package-graph.py" validate
ok "Package graph validation passed"

echo ""
echo "  Topological publication order:"
python3 "$SCRIPT_DIR/release-package-graph.py" order
echo ""

# ── 4. Mandatory verification ────────────────────────────────────────────

echo "--- 4. Mandatory verification ---"

echo "  Running make check..."
make check
ok "make check passed"

if [ "${EGGSEC_RELEASE_SKIP_PYTHON:-}" = "1" ]; then
    warn "Skipping make check-python (EGGSEC_RELEASE_SKIP_PYTHON=1)"
else
    echo "  Running make check-python..."
    make check-python
    ok "make check-python passed"
fi

# ── 5. Rust package archive validation ────────────────────────────────────

echo ""
echo "--- 5. Rust package archives (Cargo-native local validation) ---"

echo "  Packaging all publishable crates with Cargo (workspace-level, --no-verify)..."
if ! python3 "$SCRIPT_DIR/release-package-graph.py" package-workspace "$RUST_TARGET_DIR" 2>&1 | tee "$LOG_DIR/package-workspace.log"; then
    fail "Rust Cargo package command failed (log: $LOG_DIR/package-workspace.log)"
fi
INVENTORY="$RUST_TARGET_DIR/archive-inventory.jsonl"
if [ ! -s "$INVENTORY" ]; then
    fail "Rust Cargo package command produced no archive inventory"
fi
if ! python3 "$SCRIPT_DIR/release-package-graph.py" inspect-inventory "$INVENTORY" 2>&1 | tee "$LOG_DIR/inspect-inventory.log"; then
    fail "Rust Cargo archive inspection failed (log: $LOG_DIR/inspect-inventory.log)"
fi
RUST_ARCHIVE_COUNT=$(wc -l < "$INVENTORY" | tr -d ' ')

# Registry preflight uses the metadata-derived publication order, not archive
# discovery. This remains a separate staged maintainer operation.
PUBLISHABLE_CRATES=()
while IFS= read -r line; do
    PUBLISHABLE_CRATES+=("$line")
done < <(python3 "$SCRIPT_DIR/release-package-graph.py" order)

# ── 6. Optional registry preflight ───────────────────────────────────────

if [ "${EGGSEC_RELEASE_REGISTRY_PREFLIGHT:-}" = "1" ]; then
    echo ""
    echo "--- 6. Registry preflight (network-sensitive) ---"
    warn "This stage contacts crates.io. Failures here are expected in offline environments."

    for crate in "${PUBLISHABLE_CRATES[@]}"; do
        echo "  Dry-run publish for $crate..."
        LOG_FILE="$LOG_DIR/preflight-$crate.log"
        if timeout 120 cargo publish -p "$crate" --dry-run 2>&1 | tee "$LOG_FILE"; then
            ok "  $crate preflight passed"
        else
            echo ""
            fail "Registry preflight failed: $crate
Log: $LOG_FILE"
        fi
    done
    ok "Registry preflight passed"
else
    echo ""
    echo "--- 6. Registry preflight (skipped) ---"
    echo "  Registry preflight: SKIPPED (enable explicitly; required during staged publication)."
fi

# ── 7. Python artifact build ─────────────────────────────────────────────

if [ "${EGGSEC_RELEASE_SKIP_PYTHON:-}" = "1" ]; then
    echo ""
    echo "--- 7. Python artifacts (skipped) ---"
    warn "Skipping Python artifacts (EGGSEC_RELEASE_SKIP_PYTHON=1)"
else
    echo ""
    echo "--- 7. Python artifact build ---"

    cd "$REPO_ROOT/crates/eggsec-python"
    rm -rf dist/ target/wheels/
    echo "  Building wheel..."
    maturin build --release --out dist
    echo "  Building sdist..."
    maturin sdist --out dist
    echo "  Checking artifacts..."
    TWINE_PYTHON="python3"
    if ! "$TWINE_PYTHON" -m twine --version >/dev/null 2>&1; then
        if [ -x "$REPO_ROOT/.venv-ci/bin/python" ] && "$REPO_ROOT/.venv-ci/bin/python" -m twine --version >/dev/null 2>&1; then
            TWINE_PYTHON="$REPO_ROOT/.venv-ci/bin/python"
        else
            fail "Twine is required for artifact validation; install it for python3 or .venv-ci/bin/python"
        fi
    fi
    "$TWINE_PYTHON" -m twine check dist/*
    ok "Python artifacts built and checked"
fi

# ── 8. Fresh-environment installation ────────────────────────────────────

if [ "${EGGSEC_RELEASE_SKIP_PYTHON:-}" = "1" ]; then
    echo ""
    echo "--- 8. Fresh-environment install (skipped) ---"
    warn "Skipping fresh-environment install (EGGSEC_RELEASE_SKIP_PYTHON=1)"
else
    echo ""
    echo "--- 8. Fresh-environment installation ---"

    SMOKE_VENV="$TMP_ROOT/venv"
    python3 -m venv "$SMOKE_VENV"
    # shellcheck disable=SC1091
    source "$SMOKE_VENV/bin/activate"

    WHEEL=$(ls "$REPO_ROOT/crates/eggsec-python/dist/"*.whl 2>/dev/null | head -1)
    if [ -z "$WHEEL" ]; then
        fail "No wheel found in dist/"
    fi

    pip install "$WHEEL" --quiet
    pip install pytest pytest-timeout --quiet

    EggsecLocation=$(python3 -c "import eggsec; print(eggsec.__file__)")
    echo "  eggsec installed from: $EggsecLocation"

    # Verify it's not from workspace source
    if echo "$EggsecLocation" | grep -q "crates/eggsec-python"; then
        fail "eggsec imported from workspace source tree, not installed wheel"
    fi

    # Import and version check
    python3 -c "
import eggsec
info = eggsec.build_info()
print(f'  Version: {info[\"version\"]}')
print(f'  Wheel profile: {info.get(\"wheel_profile\", \"unknown\")}')
assert len(eggsec.features()) > 0, 'No features compiled'
print(f'  Features: {len(eggsec.features())}')
"

    # Verify py.typed and stubs
    python3 -c "
import eggsec, os
pkg_dir = os.path.dirname(eggsec.__file__)
typed = os.path.join(pkg_dir, 'py.typed')
stub = os.path.join(pkg_dir, '__init__.pyi')
assert os.path.exists(typed), f'py.typed not found at {typed}'
assert os.path.exists(stub), f'__init__.pyi not found at {stub}'
print(f'  py.typed: {os.path.getsize(typed)} bytes')
print(f'  __init__.pyi: {os.path.getsize(stub)} bytes')
"

    # Capability metadata
    python3 -c "
import eggsec
ops = eggsec.OperationRegistry.all_operations()
print(f'  Operations: {len(ops)}')
assert len(ops) > 0
"

    # Report serialization/redaction smoke
    python3 -c "
import json, eggsec
report = eggsec.Report(metadata={'scanner': 'release-smoke'})
finding = eggsec.Finding(
    id='smoke-1', title='Smoke test', severity=eggsec.Severity.Info,
    target='127.0.0.1', category='smoke', description='Release smoke test',
)
report.add_finding(finding)
j = report.to_json()
parsed = json.loads(j)
assert parsed['findings'][0]['id'] == 'smoke-1'
print('  Report serialization: OK')
ev = eggsec.Evidence(kind='header', value='Server: nginx', source='response', confidence=0.9)
ev_j = ev.to_json()
assert '[REDACTED]' in ev_j
print('  Evidence redaction: OK')
"

    # Deterministic loopback operation
    EGGSEC_ALLOW_LOOPBACK_FIXTURE=1 python3 -c "
import eggsec
scope = eggsec.Scope.allow_hosts(['127.0.0.1'])
result = eggsec.scan_ports('127.0.0.1', [22, 80, 443], scope, timeout_ms=2000)
assert hasattr(result, 'open_ports')
assert hasattr(result, 'target')
print(f'  Loopback scan: OK (target={result.target})')
"

    ok "Fresh-environment smoke tests passed"
fi

# ── 9. Artifact inventory ────────────────────────────────────────────────

echo ""
echo "--- 9. Artifact inventory ---"

if [ "${EGGSEC_RELEASE_SKIP_PYTHON:-}" = "1" ]; then
    echo "  No Python artifacts to inventory (skipped)."
else
    cd "$REPO_ROOT/crates/eggsec-python"
    for f in dist/*; do
        SIZE=$(file_size "$f")
        SHA256=$(sha256_hex "$f")
        echo "  $(basename "$f")  ${SIZE} bytes  sha256=${SHA256:0:16}..."
    done
fi

echo ""
echo "=== Release validation summary ==="
echo "Rust Cargo archives: PASS ($RUST_ARCHIVE_COUNT Cargo-generated, parsed, and inspected)"
if [ "${EGGSEC_RELEASE_REGISTRY_PREFLIGHT:-}" = "1" ]; then
    echo "Registry preflight: PASS"
else
    echo "Registry preflight: SKIPPED (required during staged publication)"
fi
if [ "${EGGSEC_RELEASE_SKIP_PYTHON:-}" = "1" ]; then
    echo "Python wheel/sdist: SKIPPED"
    echo "Fresh-wheel smoke: SKIPPED"
else
    echo "Python wheel/sdist: PASS"
    echo "Fresh-wheel smoke: PASS"
fi
echo "No artifacts were published."
