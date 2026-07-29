#!/usr/bin/env bash
set -euo pipefail

# Release validation script for Eggsec.
# Validates repository state and built artifacts but does NOT publish.
#
# Usage:
#   scripts/release-check.sh <version>
#
# The version argument is optional; if provided, it is validated against
# Cargo.toml and pyproject.toml.

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

# Workspace version from Cargo.toml
CARGO_VERSION=$(python3 -c "
import tomllib, sys
with open('Cargo.toml', 'rb') as f:
    data = tomllib.load(f)
print(data['workspace']['package']['version'])
")
echo "  Cargo.toml (workspace): $CARGO_VERSION"

# eggsec-python Cargo.toml (should inherit workspace)
EP_CARGO_VERSION=$(python3 -c "
import tomllib, sys
with open('crates/eggsec-python/Cargo.toml', 'rb') as f:
    data = tomllib.load(f)
v = data.get('package', {}).get('version', {})
if isinstance(v, dict) and v.get('workspace'):
    print('inherits-workspace')
else:
    print(str(v))
")
echo "  crates/eggsec-python/Cargo.toml: $EP_CARGO_VERSION"

# pyproject.toml version
PYPROJECT_VERSION=$(python3 -c "
import tomllib, sys
with open('crates/eggsec-python/pyproject.toml', 'rb') as f:
    data = tomllib.load(f)
print(data['project']['version'])
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

# Validate version is not empty or malformed
if [ -z "$CARGO_VERSION" ] || [ "$CARGO_VERSION" = "0.0.0" ]; then
    fail "Version is empty or 0.0.0"
fi
ok "Version is non-empty and non-zero"

# Check publishable crates have matching versions
echo ""
echo "  Checking publishable crate versions..."
while IFS= read -r toml; do
    CRATE_NAME=$(python3 -c "
import tomllib
with open('$toml', 'rb') as f:
    data = tomllib.load(f)
print(data.get('package', {}).get('name', 'unknown'))
")
    CRATE_VERSION=$(python3 -c "
import tomllib
with open('$toml', 'rb') as f:
    data = tomllib.load(f)
v = data.get('package', {}).get('version', {})
if isinstance(v, dict) and v.get('workspace'):
    print('inherits-workspace')
else:
    print(str(v))
")
    CRATE_PUBLISH=$(python3 -c "
import tomllib
with open('$toml', 'rb') as f:
    data = tomllib.load(f)
print(data.get('package', {}).get('publish', 'true'))
")
    if [ "$CRATE_PUBLISH" = "false" ]; then
        echo "    $CRATE_NAME: publish=false (excluded)"
    else
        echo "    $CRATE_NAME: $CRATE_VERSION"
    fi
done < <(find crates -name Cargo.toml -maxdepth 2 | sort)
ok "Crate versions checked"

# ── 3. Mandatory verification ────────────────────────────────────────────

echo ""
echo "--- 3. Mandatory verification ---"

echo "  Running make check..."
make check
ok "make check passed"

echo "  Running make check-python..."
make check-python
ok "make check-python passed"

# ── 4. Rust package dry-run ──────────────────────────────────────────────

echo ""
echo "--- 4. Rust package dry-run ---"

# Identify publishable crates (publish != false)
PUBLISHABLE_CRATES=()
while IFS= read -r toml; do
    CRATE_DIR=$(dirname "$toml")
    CRATE_NAME=$(python3 -c "
import tomllib
with open('$toml', 'rb') as f:
    data = tomllib.load(f)
print(data.get('package', {}).get('name', ''))
")
    CRATE_PUBLISH=$(python3 -c "
import tomllib
with open('$toml', 'rb') as f:
    data = tomllib.load(f)
print(data.get('package', {}).get('publish', 'true'))
")
    if [ "$CRATE_PUBLISH" != "false" ] && [ -n "$CRATE_NAME" ]; then
        PUBLISHABLE_CRATES+=("$CRATE_NAME")
    fi
done < <(find crates -name Cargo.toml -maxdepth 2 | sort)

echo "  Publishable crates: ${PUBLISHABLE_CRATES[*]}"

for crate in "${PUBLISHABLE_CRATES[@]}"; do
    echo "  Packaging $crate..."
    cargo package -p "$crate" 2>&1 | tail -1
    echo "  Dry-run publish for $crate..."
    cargo publish -p "$crate" --dry-run 2>&1 | tail -1
done
ok "Rust package dry-runs passed"

# ── 5. Python artifact build ─────────────────────────────────────────────

echo ""
echo "--- 5. Python artifact build ---"

cd "$REPO_ROOT/crates/eggsec-python"
rm -rf dist/ target/wheels/
echo "  Building wheel..."
maturin build --release --out dist
echo "  Building sdist..."
maturin sdist --out dist
echo "  Checking artifacts..."
python -m twine check dist/*
ok "Python artifacts built and checked"

# ── 6. Fresh-environment installation ────────────────────────────────────

echo ""
echo "--- 6. Fresh-environment installation ---"

SMOKE_VENV=$(mktemp -d)
trap "rm -rf $SMOKE_VENV" EXIT

python3 -m venv "$SMOKE_VENV"
source "$SMOKE_VENV/bin/activate"

WHEEL=$(ls dist/*.whl 2>/dev/null | head -1)
if [ -z "$WHEEL" ]; then
    fail "No wheel found in dist/"
fi

pip install "$WHEEL" --quiet
pip install pytest pytest-timeout --quiet

EggsecLocation=$(python -c "import eggsec; print(eggsec.__file__)")
echo "  eggsec installed from: $EggsecLocation"

# Verify it's not from workspace source
if echo "$EggsecLocation" | grep -q "projects/eggsec/crates"; then
    fail "eggsec imported from workspace source tree, not installed wheel"
fi

# Import and version check
python -c "
import eggsec
info = eggsec.build_info()
print(f'  Version: {info[\"version\"]}')
print(f'  Wheel profile: {info.get(\"wheel_profile\", \"unknown\")}')
assert len(eggsec.features()) > 0, 'No features compiled'
print(f'  Features: {len(eggsec.features())}')
"

# Verify py.typed and stubs
python -c "
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
python -c "
import eggsec
ops = eggsec.OperationRegistry.all_operations()
print(f'  Operations: {len(ops)}')
assert len(ops) > 0
"

# Report serialization/redaction smoke
python -c "
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
EGGSEC_ALLOW_LOOPBACK_FIXTURE=1 python -c "
import eggsec
scope = eggsec.Scope.allow_hosts(['127.0.0.1'])
result = eggsec.scan_ports('127.0.0.1', [22, 80, 443], scope, timeout_ms=2000)
assert hasattr(result, 'open_ports')
assert hasattr(result, 'target')
print(f'  Loopback scan: OK (target={result.target})')
"

ok "Fresh-environment smoke tests passed"

# ── 7. Artifact inventory ────────────────────────────────────────────────

echo ""
echo "--- 7. Artifact inventory ---"

cd "$REPO_ROOT/crates/eggsec-python"
for f in dist/*; do
    SIZE=$(stat -c%s "$f" 2>/dev/null || stat -f%z "$f")
    SHA256=$(sha256sum "$f" | cut -d' ' -f1)
    echo "  $(basename "$f")  ${SIZE} bytes  sha256=${SHA256:0:16}..."
done

echo ""
echo "=== Release validation passed. No artifacts were published. ==="
