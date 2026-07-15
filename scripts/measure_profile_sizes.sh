#!/usr/bin/env bash
# Measure binary sizes for each WS30 packaging profile.
# Usage: scripts/measure_profile_sizes.sh
#
# Builds each profile and reports wheel size, .so size, and native dep count.
# Output is CSV to stdout and optionally a JSON report.
set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
OUT=${WHEEL_OUT:-"$ROOT/target/python-wheels"}
MANIFEST="$ROOT/crates/eggsec-python/Cargo.toml"
REPORT=${REPORT_PATH:-"$OUT/profile-size-report.json"}

PROFILES=(
    "core:"
    "mobile:--features mobile"
    "mobile-dynamic:--features mobile-dynamic"
    "headless-browser:--features headless-browser"
    "daemon-client:--features daemon-client"
    "combined-mobile:--features mobile,git-secrets,sbom"
)

mkdir -p "$OUT"

echo "Profile,Wheel Size (bytes),SO Size (bytes),Native Deps"
echo "-------,-----------------,----------------,-----------"

JSON_ENTRIES=()

for entry in "${PROFILES[@]}"; do
    profile="${entry%%:*}"
    features="${entry#*:}"

    # Build
    if [[ -n "$features" ]]; then
        maturin build --release --manifest-path "$MANIFEST" $features --out "$OUT" 2>/dev/null
    else
        maturin build --release --manifest-path "$MANIFEST" --out "$OUT" 2>/dev/null
    fi

    # Find latest wheel
    wheel=$(ls -t "$OUT"/*.whl 2>/dev/null | head -1 || true)
    if [[ -z "$wheel" ]]; then
        echo "$profile,N/A,N/A,N/A"
        JSON_ENTRIES+=("{\"profile\":\"$profile\",\"wheel_size\":0,\"so_size\":0,\"native_deps\":0,\"error\":\"no wheel produced\"}")
        continue
    fi

    wheel_size=$(stat -c%s "$wheel" 2>/dev/null || stat -f%z "$wheel" 2>/dev/null || echo "0")

    # Extract and measure .so
    tmpdir=$(mktemp -d)
    unzip -q "$wheel" -d "$tmpdir" 2>/dev/null || true
    so=$(find "$tmpdir" -name '_core*.so' -o -name '_core*.pyd' | head -1 || true)
    so_size=0
    native_deps=0
    if [[ -n "$so" && -f "$so" ]]; then
        so_size=$(stat -c%s "$so" 2>/dev/null || stat -f%z "$so" 2>/dev/null || echo "0")
        native_deps=$(ldd "$so" 2>/dev/null | grep -c '=>' || echo "0")
    fi
    rm -rf "$tmpdir"

    echo "$profile,$wheel_size,$so_size,$native_deps"
    JSON_ENTRIES+=("{\"profile\":\"$profile\",\"wheel_size\":$wheel_size,\"so_size\":$so_size,\"native_deps\":$native_deps}")
done

# Write JSON report
if [[ -n "$REPORT" ]]; then
    {
        echo "{"
        echo "  \"profiles\": ["
        for i in "${!JSON_ENTRIES[@]}"; do
            if [[ $i -lt $((${#JSON_ENTRIES[@]} - 1)) ]]; then
                echo "    ${JSON_ENTRIES[$i]},"
            else
                echo "    ${JSON_ENTRIES[$i]}"
            fi
        done
        echo "  ]"
        echo "}"
    } > "$REPORT"
    echo
    echo "Report written to: $REPORT"
fi
