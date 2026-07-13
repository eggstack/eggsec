#!/usr/bin/env bash
# Build eggsec Python wheels for one profile or all portable profiles.
# Usage: scripts/build_wheel_profiles.sh [core|full|full-with-system|all]
set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
OUT=${WHEEL_OUT:-"$ROOT/target/python-wheels"}
MANIFEST="$ROOT/crates/eggsec-python/Cargo.toml"

usage() {
    echo "Usage: $0 [core|full|full-with-system|all]" >&2
    exit 1
}

build_profile() {
    local profile=$1
    local features=()
    case "$profile" in
        core)
            echo "[wheel] building core profile"
            ;;
        full)
            echo "[wheel] building full-no-system profile"
            features=(--features full-no-system)
            ;;
        full-with-system)
            echo "[wheel] building full-with-system profile"
            features=(--features websocket,git-secrets,sbom,container,db-pentest,web-proxy,mobile,stress-testing,packet-inspection,nse,evasion,postex,c2,headless-browser,advanced-hunting,compliance,wireless,ai-integration,daemon-client)
            ;;
        *)
            echo "ERROR: unknown profile '$profile'" >&2
            usage
            ;;
    esac

    maturin build --release --manifest-path "$MANIFEST" "${features[@]}" --out "$OUT"
}

profile=${1:-all}
mkdir -p "$OUT"
if [[ "$profile" == "all" ]]; then
    find "$OUT" -maxdepth 1 -type f -name '*.whl' -delete
    build_profile core
    build_profile full
    count=$(find "$OUT" -maxdepth 1 -type f -name '*.whl' | wc -l)
    if [[ "$count" -lt 2 ]]; then
        echo "FAIL: expected core and full-no-system wheels in $OUT" >&2
        exit 1
    fi
    echo "PASS: built $count portable wheel(s) in $OUT"
else
    build_profile "$profile"
    wheel=$(ls -t "$OUT"/*.whl 2>/dev/null | head -1 || true)
    [[ -n "$wheel" ]] || { echo "FAIL: no wheel produced in $OUT" >&2; exit 1; }
    echo "PASS: built $(basename "$wheel")"
fi
