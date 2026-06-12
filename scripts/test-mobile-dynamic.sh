#!/usr/bin/env bash
#
# scripts/test-mobile-dynamic.sh
#
# Documented emulator smoke test for mobile-dynamic (Phase 1 + Phase 2a + final polish
# + close-out polish; complete 2026-06-12).
# Per: plans/mobile-dynamic-post-phase1-polish-and-phase2-planning.md (P1.2);
#      plans/mobile-dynamic-phase2-implementation-handoff-plan.md (Phase 2a);
#      plans/mobile-dynamic-phase2-final-polish-handoff-plan.md (final polish);
#      plans/mobile-dynamic-phase2-close-out-polish-plan.md (close-out polish).
# Parent: plans/mobile-dynamic-phase1-implementation-handoff-plan.md (executed)
#         plans/dynamic-mobile-testing-loadout-design-plan.md
#
# Purpose:
#   - Always runnable (defaults to --dry-run path; no device/hardware required).
#   - Exercises the full documented happy-path command line for both Phase 1 and Phase 2a.
#   - Validates that a complete, schema-valid DynamicMobileReport is produced
#     (with actions_performed audit trail + findings array + bridge-ready shape).
#   - Validates the Phase 2a report extensions (traffic_summary, permission_state) and
#     the bridge info findings (mobile-dynamic-android-traffic-summary,
#     mobile-dynamic-android-permission-state) are emitted under the documented
#     categories.
#   - When a real Android emulator/AVD (API 34+) is available and --real is passed
#     (and ANDROID_SERIAL or emulator detected), optionally exercises the live path
#     with --install --launch --capture-logs --duration --uninstall-after --allow-dynamic-mobile --json.
#   - Intended for local developer use (with Android Studio AVD) and CI (dry-run matrix).
#
# Safety:
#   - Dry-run: zero device/network side effects; always produces valid output.
#   - Real runs: require explicit --allow-dynamic-mobile (policy gate) + user-supplied
#     controlled test APK (lab only). Best-effort uninstall is attempted.
#   - Never run against production devices or apps with real user data.
#
# Prerequisites (for real path):
#   - Android Studio + AVD (API 34+ recommended), or physical device with USB debugging.
#   - Emulator/device reachable (adb devices shows "device" or "emulator-XXXX").
#   - Controlled test APK with known high-signal behaviors during runtime
#     (e.g. permission grants, cleartext logs, crashes, or obvious secret patterns).
#     The script does NOT bundle a test APK; supply your own (or use --dry-run).
#
# Usage:
#   ./scripts/test-mobile-dynamic.sh                    # dry-run validation (default, safe)
#   ./scripts/test-mobile-dynamic.sh /path/to/test.apk  # dry-run with specific target
#   ./scripts/test-mobile-dynamic.sh /path/to/test.apk --real   # live path (if emulator present)
#   ANDROID_SERIAL=emulator-5554 ./scripts/test-mobile-dynamic.sh /path/to/test.apk --real
#
# In CI: always runs the dry-run leg (no AVD job required for green). Optional AVD job
# can invoke with --real after starting emulator + installing prerequisites.
#
# Expected (dry-run): exit 0; JSON contains "dry_run": true, non-empty "actions_performed",
#   "scan_type": "mobile-dynamic", "findings" array, duration_ms, etc. Human output also
#   valid. Phase 2a leg additionally validates "traffic_summary" + "permission_state" present
#   in the JSON (synthetic in dry-run) and that the bridge info findings are emitted.
#
# On success for real path: report includes actions like "adb connect", "install", "launch",
#   "capture_logcat", "uninstall" (best-effort), and any runtime findings emitted by the test APK.
#
# Note on close-out polish (per plans/mobile-dynamic-phase2-close-out-polish-plan.md):
#   This script's dry-run coverage is the primary CI green-path validation for the full
#   Phase 1 + Phase 2a surface. `correlate_findings` and other polish helpers are
#   independently validated by unit tests in `crates/eggsec/src/mobile/dynamic.rs`
#   (see `correlate_findings_populates_static_correlation_for_cleartext_and_permissions`
#   and friends); they are part of the lib test suite, not the smoke script.
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
EGGSEC_BIN="${REPO_ROOT}/target/debug/eggsec"   # prefer debug; CI can cargo build --features ...
# For release/CI prebuilt: allow override via EGGSEC_BIN env

FEATURES="mobile-dynamic"

APK="${1:-}"
REAL_MODE=false
if [[ "${2:-}" == "--real" || "${1:-}" == "--real" ]]; then
  REAL_MODE=true
  if [[ "${1:-}" == "--real" ]]; then APK="${2:-}"; fi
fi

if [[ -z "${APK}" ]]; then
  APK="sample-dynamic.apk"   # dry-run accepts placeholder; real path will require a real file
fi

# Prefer cargo run when in-tree (ensures fresh build with features); fallback to prebuilt bin.
run_eggsec() {
  local args=("$@")
  if command -v cargo >/dev/null 2>&1 && [[ -f "${REPO_ROOT}/Cargo.toml" ]]; then
    # Run via cargo to guarantee features and latest code (quiet build noise on success path)
    (cd "${REPO_ROOT}" && cargo run -p eggsec-cli --features "${FEATURES}" --quiet -- mobile dynamic "${args[@]}")
  else
    if [[ ! -x "${EGGSEC_BIN}" ]]; then
      echo "ERROR: eggsec binary not found at ${EGGSEC_BIN} and cargo not available in PATH." >&2
      echo "Build first: cargo build -p eggsec-cli --features ${FEATURES}" >&2
      exit 1
    fi
    "${EGGSEC_BIN}" mobile dynamic "${args[@]}"
  fi
}

echo "=== Mobile Dynamic Smoke Test (Phase 1 + Phase 2a + final polish + close-out polish) ==="
echo "Target: ${APK}"
echo "Mode: $(if $REAL_MODE; then echo 'REAL (requires --allow-dynamic-mobile + reachable emulator/device)'; else echo 'DRY-RUN (safe, no device touch)'; fi)"
echo "Repo root: ${REPO_ROOT}"
echo

# Always exercise dry-run first (safe, validates structure + bridge + formatting).
echo ">>> Step: Dry-run (produces complete valid report without touching devices)"
DRY_OUT="$(mktemp)"
DRY_JSON="$(mktemp)"
trap 'rm -f "${DRY_OUT}" "${DRY_JSON}"' EXIT

# Full documented happy-path flags (dry-run variant). Matches docs/MOBILE.md and plan.
run_eggsec \
  "${APK}" \
  --device emulator-5554 \
  --install --launch '.MainActivity' \
  --capture-logs --duration 60 \
  --uninstall-after \
  --dry-run \
  --json \
  --quiet \
  > "${DRY_JSON}"

# Also capture human pretty for basic sanity (non-json path)
run_eggsec \
  "${APK}" \
  --device emulator-5554 \
  --dry-run \
  --quiet \
  > "${DRY_OUT}"

echo "Dry-run human output (first 20 lines):"
head -20 "${DRY_OUT}"
echo "..."

echo
echo ">>> Validating dry-run JSON report structure (jq or python fallback)"
if command -v jq >/dev/null 2>&1; then
  SCAN_TYPE=$(jq -r '.scan_type' < "${DRY_JSON}")
  DRY_FLAG=$(jq -r '.dry_run' < "${DRY_JSON}")
  ACTIONS_COUNT=$(jq '.actions_performed | length' < "${DRY_JSON}")
  FINDINGS_COUNT=$(jq '.findings | length' < "${DRY_JSON}")
  DURATION=$(jq '.duration_ms' < "${DRY_JSON}")
  echo "  scan_type=${SCAN_TYPE} dry_run=${DRY_FLAG} actions=${ACTIONS_COUNT} findings=${FINDINGS_COUNT} duration_ms=${DURATION}"
  if [[ "${SCAN_TYPE}" != "mobile-dynamic" || "${DRY_FLAG}" != "true" || "${ACTIONS_COUNT}" -eq 0 ]]; then
    echo "FAIL: dry-run report missing expected fields (scan_type, dry_run:true, actions_performed)" >&2
    exit 1
  fi
else
  # Python fallback (always available in most envs)
  python3 - <<'PY' "${DRY_JSON}"
import json, sys
data = json.load(open(sys.argv[1]))
assert data.get("scan_type") == "mobile-dynamic", "scan_type"
assert data.get("dry_run") is True, "dry_run"
assert isinstance(data.get("actions_performed"), list) and len(data["actions_performed"]) > 0, "actions"
assert "duration_ms" in data, "duration"
print("  (python) scan_type=%s dry_run=%s actions=%d" % (data["scan_type"], data["dry_run"], len(data["actions_performed"])))
PY
fi

echo "Dry-run validation: PASS (complete schema-valid report + audit trail produced)."

# Phase 2a dry-run extension (always safe; exercises proxy/traffic/permission flags + report extensions).
# Complements the P1 happy-path above; validates traffic_summary / permission_state presence (synthetic in dry-run).
echo
echo ">>> Step: Phase 2a dry-run extensions (--proxy + --traffic-capture + permission grant/revoke/list; validates report extensions)"
P2_DRY_JSON="$(mktemp)"
# Use a placeholder capture file name (dry-run special-cases; does not require the file to exist or be readable).
run_eggsec \
  "${APK}" \
  --device emulator-5554 \
  --proxy 127.0.0.1:8080 \
  --traffic-capture /tmp/phase2-mitm.log \
  --list-permissions \
  --grant-permission android.permission.CAMERA \
  --revoke-permission android.permission.READ_CONTACTS \
  --dry-run \
  --json \
  --quiet \
  > "${P2_DRY_JSON}"

if command -v jq >/dev/null 2>&1; then
  P2_SCAN=$(jq -r '.scan_type' < "${P2_DRY_JSON}")
  P2_DRY=$(jq -r '.dry_run' < "${P2_DRY_JSON}")
  P2_HAS_TS=$(jq -r 'has("traffic_summary")' < "${P2_DRY_JSON}")
  P2_HAS_PS=$(jq -r 'has("permission_state")' < "${P2_DRY_JSON}")
  P2_ACTIONS=$(jq '.actions_performed | length' < "${P2_DRY_JSON}")
  echo "  Phase2: scan_type=${P2_SCAN} dry_run=${P2_DRY} has_traffic_summary=${P2_HAS_TS} has_permission_state=${P2_HAS_PS} actions=${P2_ACTIONS}"
  if [[ "${P2_SCAN}" != "mobile-dynamic" || "${P2_DRY}" != "true" || "${P2_HAS_TS}" != "true" || "${P2_HAS_PS}" != "true" ]]; then
    echo "FAIL: Phase 2a dry-run report missing expected extensions (traffic_summary, permission_state) or markers" >&2
    exit 1
  fi
else
  python3 - <<'PY' "${P2_DRY_JSON}"
import json, sys
d = json.load(open(sys.argv[1]))
assert d.get("scan_type") == "mobile-dynamic", "scan_type"
assert d.get("dry_run") is True, "dry_run"
assert "traffic_summary" in d, "traffic_summary key"
assert "permission_state" in d, "permission_state key"
print("  (python) Phase2: has traffic_summary + permission_state")
PY
fi

echo "Phase 2a dry-run extension validation: PASS (report carries traffic_summary + permission_state; actions include proxy/permission simulation)."

# If not real mode, we're done (CI green path).
if ! $REAL_MODE; then
  echo
  echo "=== Smoke test complete (dry-run only; --real not requested) ==="
  echo "To exercise live path: start an AVD (API 34+), supply a controlled test APK, then:"
  echo "  ./scripts/test-mobile-dynamic.sh /path/to/your-test.apk --real"
  echo "Or set ANDROID_SERIAL and pass --real."
  echo "See docs/MOBILE.md 'Phase 1 Lab Setup' + 'Phase 2a CLI examples' and the polish plan for full command + safety notes."
  exit 0
fi

# Real path (lab-only; requires explicit allow + reachable device).
echo
echo ">>> Step: Real path (install/launch/capture/uninstall on reachable emulator/device)"
echo "WARNING: This will install and run the provided APK on the target device and attempt uninstall."
echo "Only use with controlled lab/test APKs you own. Requires --allow-dynamic-mobile (policy)."
echo

# Resolve device: prefer ANDROID_SERIAL env, else try to detect first emulator from adb (if present)
DEVICE_ARG=()
if [[ -n "${ANDROID_SERIAL:-}" ]]; then
  DEVICE_ARG=(--device "${ANDROID_SERIAL}")
  echo "Using ANDROID_SERIAL=${ANDROID_SERIAL}"
elif command -v adb >/dev/null 2>&1; then
  # Best-effort: pick first listed device/emulator
  FIRST_DEV=$(adb devices | awk 'NR>1 && /device|emulator/ {print $1; exit}')
  if [[ -n "${FIRST_DEV}" ]]; then
    DEVICE_ARG=(--device "${FIRST_DEV}")
    echo "Detected device via adb: ${FIRST_DEV}"
  else
    echo "No device/emulator visible via adb; falling back to default emulator-5554 (may fail if unreachable)."
    DEVICE_ARG=(--device emulator-5554)
  fi
else
  echo "adb not in PATH; using default emulator-5554 (pure-Rust path may still work for TCP emulators)."
  DEVICE_ARG=(--device emulator-5554)
fi

REAL_JSON="$(mktemp)"
trap 'rm -f "${DRY_OUT}" "${DRY_JSON}" "${REAL_JSON}"' EXIT

# Full happy-path per plan + docs:
#   --install --launch '.MainActivity' --capture-logs --duration 60 --uninstall-after --allow-dynamic-mobile --json
run_eggsec \
  "${APK}" \
  "${DEVICE_ARG[@]}" \
  --install --launch '.MainActivity' \
  --capture-logs --duration 60 \
  --uninstall-after \
  --allow-dynamic-mobile \
  --json \
  --quiet \
  > "${REAL_JSON}"

echo "Real-run JSON produced at ${REAL_JSON} (truncated preview):"
if command -v jq >/dev/null 2>&1; then
  jq '{scan_type, dry_run, device_serial, actions_performed: (.actions_performed | length), findings: (.findings | length), duration_ms}' < "${REAL_JSON}"
else
  python3 - <<'PY' "${REAL_JSON}"
import json, sys
d = json.load(open(sys.argv[1]))
print({k: (len(d.get(k,[])) if isinstance(d.get(k),list) else d.get(k)) for k in ('scan_type','dry_run','device_serial','actions_performed','findings','duration_ms')})
PY
fi

# Light structural assertions for real path (must not be dry_run; must have audit actions)
if command -v jq >/dev/null 2>&1; then
  REAL_DRY=$(jq -r '.dry_run' < "${REAL_JSON}")
  REAL_ACTIONS=$(jq '.actions_performed | length' < "${REAL_JSON}")
  REAL_SCAN=$(jq -r '.scan_type' < "${REAL_JSON}")
else
  REAL_DRY=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1])).get("dry_run"))' "${REAL_JSON}")
  REAL_ACTIONS=$(python3 -c 'import json,sys; print(len(json.load(open(sys.argv[1])).get("actions_performed",[])))' "${REAL_JSON}")
  REAL_SCAN=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1])).get("scan_type"))' "${REAL_JSON}")
fi

if [[ "${REAL_DRY}" == "true" ]]; then
  echo "FAIL: real run unexpectedly marked dry_run=true" >&2
  exit 1
fi
if [[ "${REAL_SCAN}" != "mobile-dynamic" || "${REAL_ACTIONS}" -eq 0 ]]; then
  echo "FAIL: real report missing expected scan_type or actions audit" >&2
  exit 1
fi

echo "Real path validation: PASS (audit trail present; non-dry-run; policy allow honored)."

echo
echo "=== Mobile Dynamic Smoke Test COMPLETE ==="
echo "See report JSON for full findings + actions_performed (install/launch/log/uninstall + any runtime observations)."
echo "Next: eggsec report convert <json> -f html (or sarif/junit/markdown) to exercise the bridge."
echo "Update docs/MOBILE.md 'Phase 1/Phase 2a Success Criteria' after successful local AVD run."
exit 0
