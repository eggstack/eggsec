#!/usr/bin/env bash
# artifact-sizes.sh — Report artifact sizes and dependency counts per profile.
#
# Usage:
#   bash scripts/artifact-sizes.sh [--release]
#
# Builds each profile, then reports:
#   - artifact path
#   - features / profile
#   - file size (bytes)
#   - stripped status
#   - direct / total crate count
#   - notable dependencies (rustls provider, sqlite, etc.)

set -euo pipefail

RELEASE=""
CARGO_ARGS=""
if [[ "${1:-}" == "--release" ]]; then
  RELEASE=" (release)"
  CARGO_ARGS="--release"
fi

report() {
  local name="$1"
  local features="$2"
  shift 2
  local cargo_extra=("$@")

  local args=("-p" "$name")
  if [[ -n "$features" ]]; then
    args+=("--no-default-features" "--features" "$features")
  fi
  args+=($CARGO_ARGS)

  # Count crates
  local total_crates
  total_crates=$(cargo tree "${args[@]}" 2>/dev/null | grep -c '^[[:space:]]*' | head -1 || echo "?")
  # More reliable: use cargo tree --depth 1 for direct deps
  local direct_crates
  direct_crates=$(cargo tree "${args[@]}" --depth 1 2>/dev/null | tail -n +2 | wc -l || echo "?")

  # Build the binary
  local build_args=("$name")
  if [[ -n "$features" ]]; then
    build_args+=("--no-default-features" "--features" "$features")
  fi
  build_args+=($CARGO_ARGS)

  cargo build "${build_args[@]}" 2>/dev/null || { echo "  [SKIP] build failed for $name"; return; }

  # Find the binary
  local target_dir="target"
  if [[ -n "$CARGO_ARGS" ]]; then
    target_dir="target/release"
  fi

  local binary_path
  binary_path=$(cargo metadata --format-version 1 2>/dev/null | python3 -c "
import json, sys
data = json.load(sys.stdin)
for pkg in data['workspace_members']:
    if pkg.split(' ')[0].endswith('/$name'):
        break
" 2>/dev/null; find "$target_dir" -name "$name" -type f -executable 2>/dev/null | head -1)

  if [[ -z "$binary_path" || ! -f "$binary_path" ]]; then
    echo "  [SKIP] binary not found for $name"
    return
  fi

  local size
  size=$(stat -c%s "$binary_path" 2>/dev/null || stat -f%z "$binary_path" 2>/dev/null)
  local stripped="unknown"
  if file "$binary_path" 2>/dev/null | grep -q "not stripped"; then
    stripped="no"
  else
    stripped="yes"
  fi

  # Check notable deps
  local notable=""
  if cargo tree "${args[@]}" 2>/dev/null | grep -q "rusqlite"; then
    notable="${notable}sqlite "
  fi
  if cargo tree "${args[@]}" 2>/dev/null | grep -q "aws-lc-rs"; then
    notable="${notable}aws-lc "
  fi
  if cargo tree "${args[@]}" 2>/dev/null | grep -q "ring v"; then
    notable="${notable}ring "
  fi
  if cargo tree "${args[@]}" 2>/dev/null | grep -q "axum v"; then
    notable="${notable}axum "
  fi

  printf "  %-30s %8s bytes  stripped=%-3s  crates=%s/%s  %s%s\n" \
    "$name" "$size" "$stripped" "$direct_crates" "$total_crates" "$features" "${notable:+ [$notable]}"
}

echo "=== Eggsec Artifact Sizes${RELEASE} ==="
echo ""

echo "--- Standard CLI/TUI (default) ---"
report "eggsec-cli" ""

echo ""
echo "--- Headless CLI ---"
report "eggsec-cli" "headless" --no-default-features --features "headless"

echo ""
echo "--- Daemon Client CLI ---"
report "eggsec-cli" "daemon-client" --no-default-features --features "daemon-client"

echo ""
echo "--- Daemon Server ---"
report "eggsec-daemon" ""

echo ""
echo "--- Full CLI ---"
report "eggsec-cli" "full"

echo ""
echo "Done."
