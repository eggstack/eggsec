#!/usr/bin/env bash
# Validate an eggsec Python wheel: install in a temp venv and run smoke tests.
# Usage: scripts/validate_wheel.sh <wheel-path>
set -euo pipefail

usage() {
    echo "Usage: $0 <wheel-file>" >&2
    echo "" >&2
    echo "Arguments:" >&2
    echo "  wheel-file   Path to the .whl file to validate" >&2
    exit 1
}

if [[ $# -ne 1 ]]; then
    usage
fi

WHEEL="$1"

if [[ ! -f "$WHEEL" ]]; then
    echo "FAIL: Wheel file not found: $WHEEL" >&2
    exit 1
fi

echo "=== Eggsec Wheel Validation ==="
echo "Wheel: $(basename "$WHEEL")"
echo ""

PASS=0
FAIL=0

# Create temporary venv
VENV_DIR=$(mktemp -d)
trap 'rm -rf "$VENV_DIR"' EXIT

echo "--- Step 1: Create temporary venv ---"
python -m venv "$VENV_DIR"
VENV_PYTHON="$VENV_DIR/bin/python"
"$VENV_PYTHON" -m pip install --quiet --upgrade pip > /dev/null 2>&1

echo "--- Step 2: Install wheel ---"
"$VENV_PYTHON" -m pip install --quiet "$WHEEL" > /dev/null 2>&1

echo "--- Step 3: Import test ---"
if "$VENV_PYTHON" -c "import eggsec; print('Version:', eggsec.__version__)" 2>/dev/null; then
    echo "PASS: Import successful."
    PASS=$((PASS + 1))
else
    echo "FAIL: Import failed."
    FAIL=$((FAIL + 1))
fi

echo "--- Step 4: Feature check ---"
if "$VENV_PYTHON" -c "
import eggsec
features = eggsec.features()
assert isinstance(features, dict), 'features() did not return a dict'
assert 'core' in features, 'core feature missing'
assert features['core'] is True, 'core feature is not True'
print('Features:', features)
" 2>/dev/null; then
    echo "PASS: Feature check passed."
    PASS=$((PASS + 1))
else
    echo "FAIL: Feature check failed."
    FAIL=$((FAIL + 1))
fi

echo "--- Step 5: build_info check ---"
if "$VENV_PYTHON" -c "
import eggsec
info = eggsec.build_info()
assert isinstance(info, dict), 'build_info() did not return a dict'
assert 'version' in info, 'version missing from build_info'
assert 'package_name' in info, 'package_name missing from build_info'
print('Build info:', info)
" 2>/dev/null; then
    echo "PASS: build_info check passed."
    PASS=$((PASS + 1))
else
    echo "FAIL: build_info check failed."
    FAIL=$((FAIL + 1))
fi

echo "--- Step 6: api_surface check ---"
if "$VENV_PYTHON" -c "
import eggsec
surface = eggsec.api_surface()
assert isinstance(surface, dict), 'api_surface() did not return a dict'
assert len(surface) > 0, 'api_surface() returned empty dict'
# Check a known entry
assert 'scan_ports' in surface, 'scan_ports not in api_surface'
assert surface['scan_ports']['stability'] == 'stable', 'scan_ports stability != stable'
print('api_surface entries:', len(surface))
" 2>/dev/null; then
    echo "PASS: api_surface check passed."
    PASS=$((PASS + 1))
else
    echo "FAIL: api_surface check failed."
    FAIL=$((FAIL + 1))
fi

echo "--- Step 7: api_surface_version check ---"
if "$VENV_PYTHON" -c "
import eggsec
version_info = eggsec.api_surface_version()
assert isinstance(version_info, dict), 'api_surface_version() did not return a dict'
assert 'package_version' in version_info, 'package_version missing'
assert 'abi_version' in version_info, 'abi_version missing'
print('Version info:', version_info)
" 2>/dev/null; then
    echo "PASS: api_surface_version check passed."
    PASS=$((PASS + 1))
else
    echo "FAIL: api_surface_version check failed."
    FAIL=$((FAIL + 1))
fi

echo ""
echo "=== Results ==="
echo "Passed: $PASS"
echo "Failed: $FAIL"

if [[ $FAIL -gt 0 ]]; then
    echo "STATUS: FAIL"
    exit 1
else
    echo "STATUS: PASS"
    exit 0
fi
