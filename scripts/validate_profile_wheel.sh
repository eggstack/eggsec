#!/usr/bin/env bash
# Validate an eggsec wheel for a specific WS30 packaging profile.
# Usage: scripts/validate_profile_wheel.sh <wheel-path> <profile>
#
# Profiles: core, mobile, mobile-dynamic, headless-browser, daemon-client,
#           combined-mobile
#
# Checks: import, feature assertions, .so size, native dependency count.
set -euo pipefail

if [[ $# -ne 2 ]]; then
    echo "Usage: $0 <wheel-file> <profile>" >&2
    exit 1
fi

WHEEL=$1
PROFILE=$2

if [[ ! -f "$WHEEL" ]]; then
    echo "FAIL: wheel not found: $WHEEL" >&2
    exit 1
fi

VENV_DIR=$(mktemp -d)
trap 'rm -rf "$VENV_DIR"' EXIT

echo "=== WS30 Profile Validation: $PROFILE ==="
echo "Wheel: $(basename "$WHEEL")"

python3 -m venv "$VENV_DIR" >/dev/null 2>&1
"$VENV_DIR/bin/pip" install --disable-pip-version-check --no-deps "$WHEEL" >/dev/null 2>&1

# 1. Basic import
"$VENV_DIR/bin/python" -c "import eggsec; print('OK: eggsec imported')"

# 2. Feature assertions per profile
case "$PROFILE" in
    core)
        "$VENV_DIR/bin/python" -c "
import eggsec
f = eggsec.features()
for feat in ['mobile', 'headless-browser', 'daemon-client', 'nse', 'wireless']:
    assert not f.get(feat, False), f'{feat} should be disabled in core'
print('OK: core features validated')
"
        ;;
    mobile)
        "$VENV_DIR/bin/python" -c "
import eggsec
f = eggsec.features()
assert f.get('mobile', False), 'mobile should be enabled'
for feat in ['headless-browser', 'daemon-client']:
    assert not f.get(feat, False), f'{feat} should be disabled in mobile profile'
print('OK: mobile features validated')
"
        ;;
    mobile-dynamic)
        "$VENV_DIR/bin/python" -c "
import eggsec
f = eggsec.features()
assert f.get('mobile-dynamic', False), 'mobile-dynamic should be enabled'
print('OK: mobile-dynamic features validated')
"
        ;;
    headless-browser)
        "$VENV_DIR/bin/python" -c "
import eggsec
f = eggsec.features()
assert f.get('headless-browser', False), 'headless-browser should be enabled'
for feat in ['mobile', 'daemon-client']:
    assert not f.get(feat, False), f'{feat} should be disabled in headless-browser profile'
print('OK: headless-browser features validated')
"
        ;;
    daemon-client)
        "$VENV_DIR/bin/python" -c "
import eggsec
f = eggsec.features()
assert f.get('daemon-client', False), 'daemon-client should be enabled'
for feat in ['mobile', 'headless-browser']:
    assert not f.get(feat, False), f'{feat} should be disabled in daemon-client profile'
print('OK: daemon-client features validated')
"
        ;;
    combined-mobile)
        "$VENV_DIR/bin/python" -c "
import eggsec
f = eggsec.features()
for feat in ['mobile', 'git-secrets', 'sbom']:
    assert f.get(feat, False), f'{feat} should be enabled in combined-mobile'
print('OK: combined-mobile features validated')
"
        ;;
    *)
        echo "FAIL: unknown profile '$PROFILE'" >&2
        exit 1
        ;;
esac

# 3. Profile-specific import checks
"$VENV_DIR/bin/python" -c "
import eggsec
# Core is always importable
assert hasattr(eggsec, 'scan_ports')
assert hasattr(eggsec, 'features')
assert hasattr(eggsec, 'api_surface')
print('OK: core API surface present')
"

case "$PROFILE" in
    mobile)
        "$VENV_DIR/bin/python" -c "
import eggsec
assert hasattr(eggsec, 'analyze_apk'), 'analyze_apk missing'
assert hasattr(eggsec, 'analyze_ipa'), 'analyze_ipa missing'
print('OK: mobile entry points importable')
"
        ;;
    mobile-dynamic)
        "$VENV_DIR/bin/python" -c "
import eggsec
assert hasattr(eggsec, 'analyze_apk'), 'analyze_apk missing'
assert hasattr(eggsec, 'list_mobile_devices'), 'list_mobile_devices missing'
print('OK: mobile-dynamic entry points importable')
"
        ;;
    headless-browser)
        "$VENV_DIR/bin/python" -c "
import eggsec
assert hasattr(eggsec, 'browser_test'), 'browser_test missing'
print('OK: headless-browser entry points importable')
"
        ;;
    daemon-client)
        "$VENV_DIR/bin/python" -c "
import eggsec
assert hasattr(eggsec, 'daemon_connect'), 'daemon_connect missing'
print('OK: daemon-client entry points importable')
"
        ;;
esac

# 4. Measure .so size
SO_PATH=$("$VENV_DIR/bin/python" -c "import eggsec._core; print(eggsec._core.__file__)" 2>/dev/null || true)
SO_SIZE=0
if [[ -n "$SO_PATH" && -f "$SO_PATH" ]]; then
    SO_SIZE=$(stat -c%s "$SO_PATH" 2>/dev/null || stat -f%z "$SO_PATH" 2>/dev/null || echo "0")
    echo "SO size: $SO_SIZE bytes"
else
    echo "WARNING: could not locate .so file"
fi

# 5. Native dependency count
NATIVE_DEPS=0
if [[ -n "$SO_PATH" && -f "$SO_PATH" ]]; then
    NATIVE_DEPS=$(ldd "$SO_PATH" 2>/dev/null | grep -c '=>' || echo "0")
    echo "Native deps: $NATIVE_DEPS"
fi

# 6. Wheel size
WHEEL_SIZE=$(stat -c%s "$WHEEL" 2>/dev/null || stat -f%z "$WHEEL" 2>/dev/null || echo "0")
echo "Wheel size: $WHEEL_SIZE bytes"

echo
echo "PROFILE=$PROFILE SO_SIZE=$SO_SIZE NATIVE_DEPS=$NATIVE_DEPS WHEEL_SIZE=$WHEEL_SIZE"
echo "PASS: $PROFILE profile validated"
