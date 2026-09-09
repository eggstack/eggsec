#!/usr/bin/env bash
# setup_android_emulator.sh — reproducible Android emulator fixture (Phase F).
#
# Preferred local runner for mobile-dynamic integration: pinned system image,
# KVM check, software fallback note, and lifecycle cleanup. Lab use only.
#
# Usage:
#   ./scripts/setup_android_emulator.sh --check        # prerequisite report only
#   ./scripts/setup_android_emulator.sh --start        # start emulator (requires SDK)
#   ./scripts/setup_android_emulator.sh --stop         # stop emulator + cleanup
#
# Pinned image (documented default; override via env):
#   ANDROID_API=34  ANDROID_ABI=x86_64  ANDROID_TAG=google_apis
#   AVD_NAME=eggsec-lab  EMULATOR_PORT=5554
#
# Safety: never touches production devices. All state stays in the AVD.
# Real runs additionally require --allow-dynamic-mobile (--allow-frida for
# instrumentation) and a test APK you own (see scripts/make_test_apk.py).
set -euo pipefail

AVD_NAME="${AVD_NAME:-eggsec-lab}"
ANDROID_API="${ANDROID_API:-34}"
ANDROID_ABI="${ANDROID_ABI:-x86_64}"
ANDROID_TAG="${ANDROID_TAG:-google_apis}"
EMULATOR_PORT="${EMULATOR_PORT:-5554}"

check() {
  echo "=== Android emulator prerequisites ==="
  echo "os/arch: $(uname -s)/$(uname -m)"
  if command -v adb >/dev/null 2>&1; then echo "ok   adb: $(command -v adb)"; else echo "miss adb: install Android SDK platform-tools"; fi
  if command -v emulator >/dev/null 2>&1; then echo "ok   emulator: $(command -v emulator)"; else echo "miss emulator: install Android Studio / cmdline-tools"; fi
  if command -v avdmanager >/dev/null 2>&1; then echo "ok   avdmanager present"; else echo "miss avdmanager: needed for --start"; fi
  if [[ -e /dev/kvm ]]; then echo "ok   /dev/kvm present (hardware acceleration)"; else echo "note /dev/kvm absent: software emulation fallback (-no-accel) will be slow but usable for smoke"; fi
  if command -v frida >/dev/null 2>&1; then echo "ok   frida CLI present (optional, live instrumentation only)"; else echo "note frida CLI absent: dry-run + planning still work"; fi
  echo "pinned image: system-images;android-${ANDROID_API};${ANDROID_TAG};${ANDROID_ABI}"
  echo "avd: ${AVD_NAME}  port: ${EMULATOR_PORT}"
  echo "fixture (no emulator needed): cargo test -p eggsec-mobile-lab --features mobile-dynamic"
  echo "dry-run (no emulator needed): ./scripts/test-mobile-dynamic.sh"
}

start() {
  check
  if ! command -v avdmanager >/dev/null 2>&1 || ! command -v emulator >/dev/null 2>&1; then
    echo "SKIP mobile-dynamic live: emulator toolchain absent (see --check). Fixture tests above still run." >&2
    exit 2
  fi
  echo "Creating/starting AVD ${AVD_NAME} (API ${ANDROID_API}, ${ANDROID_ABI})..."
  avdmanager list avd | grep -q "${AVD_NAME}" || {
    echo "no" | avdmanager create avd -n "${AVD_NAME}" -k "system-images;android-${ANDROID_API};${ANDROID_TAG};${ANDROID_ABI}" --device "pixel" --force
  }
  ACCEL_ARGS=()
  [[ -e /dev/kvm ]] || ACCEL_ARGS+=(-no-accel)
  # shellcheck disable=SC2086
  nohup emulator -avd "${AVD_NAME}" -port "${EMULATOR_PORT}" -no-snapshot -no-audio -no-window "${ACCEL_ARGS[@]}" >/tmp/eggsec-emulator.log 2>&1 &
  echo "waiting for boot (adb wait-for-device + bootcompleted)..."
  adb -s "emulator-${EMULATOR_PORT}" wait-for-device
  for _ in $(seq 1 60); do
    if adb -s "emulator-${EMULATOR_PORT}" shell getprop sys.boot_completed 2>/dev/null | grep -q 1; then
      echo "emulator booted: emulator-${EMULATOR_PORT}"
      adb devices
      exit 0
    fi
    sleep 5
  done
  echo "emulator did not boot in time; see /tmp/eggsec-emulator.log" >&2
  exit 1
}

stop() {
  if command -v adb >/dev/null 2>&1; then
    adb -s "emulator-${EMULATOR_PORT}" emu kill 2>/dev/null || true
  fi
  pkill -f "emulator.*${AVD_NAME}" 2>/dev/null || true
  echo "emulator stopped (best-effort cleanup done)."
}

case "${1:---check}" in
  --check) check ;;
  --start) start ;;
  --stop) stop ;;
  *) echo "usage: $0 [--check|--start|--stop]" >&2; exit 2 ;;
esac
