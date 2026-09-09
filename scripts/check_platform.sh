#!/usr/bin/env bash
# check_platform.sh — Phase F platform integration entry point (hermetic first).
#
# Runs the deterministic fixture layer (no root, no hardware, no network):
#   1. eggsec doctor (centralized prerequisite matrix)
#   2. packet fixture tests (parser/craft/loopback)
#   3. wireless fixture tests (parser/heuristic/frame fixtures)
#   4. mobile-dynamic fixture tests (mock ADB + Frida simulation)
#   5. mobile-dynamic dry-run smoke (no device)
#
# Live layers (netns, emulator, RF) are separate manual steps documented in
# docs/PLATFORM.md and never run here.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "=== [1/5] doctor (centralized prerequisites) ==="
cargo run -q -p eggsec-cli -- doctor 2>&1 | head -n 60

echo
echo "=== [2/5] packet fixtures (no privilege) ==="
cargo test -q -p eggsec --lib --features packet-inspection packet::fixture:: 2>&1 | tail -n 5

echo
echo "=== [3/5] wireless fixtures (no hardware) ==="
cargo test -q -p eggsec --lib --features wireless wireless::fixture:: 2>&1 | tail -n 5
cargo test -q -p eggsec --lib --features wireless,wireless-advanced wireless::fixture:: 2>&1 | tail -n 5

echo
echo "=== [4/5] mobile-dynamic fixtures (mock ADB, no emulator) ==="
cargo test -q -p eggsec-mobile-lab --features mobile-dynamic --lib 2>&1 | tail -n 5

echo
echo "=== [5/5] mobile-dynamic dry-run smoke (no device) ==="
./scripts/test-mobile-dynamic.sh 2>&1 | tail -n 15

echo
echo "PASS check_platform (fixture layer; live layers remain manual per docs/PLATFORM.md)."
