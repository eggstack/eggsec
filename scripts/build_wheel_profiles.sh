#!/usr/bin/env bash
# Build eggsec Python wheels for one profile or all portable profiles.
# Usage: scripts/build_wheel_profiles.sh [core|full|full-with-system|mobile|mobile-dynamic|headless-browser|daemon-client|combined-mobile|all|all-ws30]
set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
OUT=${WHEEL_OUT:-"$ROOT/target/python-wheels"}
MANIFEST="$ROOT/crates/eggsec-python/Cargo.toml"

usage() {
    echo "Usage: $0 [core|full|full-with-system|mobile|mobile-dynamic|headless-browser|daemon-client|combined-mobile|all|all-ws30]" >&2
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
        mobile)
            echo "[wheel] building mobile profile"
            features=(--features mobile)
            ;;
        mobile-dynamic)
            echo "[wheel] building mobile-dynamic profile"
            features=(--features mobile-dynamic)
            ;;
        headless-browser)
            echo "[wheel] building headless-browser profile"
            features=(--features headless-browser)
            ;;
        daemon-client)
            echo "[wheel] building daemon-client profile"
            features=(--features daemon-client)
            ;;
        combined-mobile)
            echo "[wheel] building combined-mobile profile"
            features=(--features mobile,git-secrets,sbom)
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
elif [[ "$profile" == "all-ws30" ]]; then
    find "$OUT" -maxdepth 1 -type f -name '*.whl' -delete
    for p in core mobile mobile-dynamic headless-browser daemon-client combined-mobile; do
        build_profile "$p"
    done
    count=$(find "$OUT" -maxdepth 1 -type f -name '*.whl' | wc -l)
    if [[ "$count" -lt 6 ]]; then
        echo "FAIL: expected 6 WS30 profile wheels in $OUT (got $count)" >&2
        exit 1
    fi
    echo "PASS: built $count WS30 profile wheel(s) in $OUT"
else
    build_profile "$profile"
    wheel=$(ls -t "$OUT"/*.whl 2>/dev/null | head -1 || true)
    [[ -n "$wheel" ]] || { echo "FAIL: no wheel produced in $OUT" >&2; exit 1; }
    echo "PASS: built $(basename "$wheel")"
fi
