#!/usr/bin/env bash
# Build eggsec Python wheels for different profiles.
# Usage: scripts/build_wheel_profiles.sh <profile>
# Profiles: core, full, full-with-system
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE_DIR="${SCRIPT_DIR}/../crates/eggsec-python"

usage() {
    echo "Usage: $0 <profile>" >&2
    echo "" >&2
    echo "Profiles:" >&2
    echo "  core              No optional features (default)" >&2
    echo "  full              All non-system-dep features (websocket, git-secrets, sbom, container)" >&2
    echo "  full-with-system  All features including system-dep ones" >&2
    exit 1
}

if [[ $# -ne 1 ]]; then
    usage
fi

PROFILE="$1"

case "$PROFILE" in
    core)
        FEATURES=""
        echo "Building core profile (no optional features)..."
        ;;
    full)
        FEATURES="--features full-no-system"
        echo "Building full profile (full-no-system: websocket, git-secrets, sbom, container)..."
        ;;
    full-with-system)
        FEATURES="--features websocket,git-secrets,sbom,container,db-pentest,web-proxy,mobile,stress-testing,packet-inspection,nse,evasion,postex,c2,headless-browser,advanced-hunting,compliance,wireless,ai-integration,daemon-client"
        echo "Building full-with-system profile (all features)..."
        ;;
    *)
        echo "ERROR: Unknown profile '$PROFILE'" >&2
        usage
        ;;
esac

if [[ ! -d "$CRATE_DIR" ]]; then
    echo "ERROR: Crate directory not found: $CRATE_DIR" >&2
    exit 1
fi

# Build the wheel
echo ""
echo "--- Building wheel ---"
BUILD_CMD=(maturin build --release)
if [[ -n "$FEATURES" ]]; then
    BUILD_CMD+=($FEATURES)
fi

echo "Running: ${BUILD_CMD[*]}"
"${BUILD_CMD[@]}"

# Find the resulting wheel
WHEEL_DIR="${CRATE_DIR}/target/wheels"
WHEEL=$(ls -t "$WHEEL_DIR"/*.whl 2>/dev/null | head -1)

if [[ -z "$WHEEL" ]]; then
    echo "ERROR: No wheel found in $WHEEL_DIR" >&2
    exit 1
fi

echo ""
echo "--- Result ---"
echo "Wheel: $(basename "$WHEEL")"
echo "Path:  $WHEEL"

# Validate the wheel imports successfully
echo ""
echo "--- Validation ---"
python -c "import eggsec; print('Version:', eggsec.__version__); print('Features:', eggsec.features())"

echo ""
echo "Profile '$PROFILE' build complete."
