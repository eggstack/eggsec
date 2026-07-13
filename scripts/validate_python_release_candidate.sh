#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$ROOT"

run() {
  echo "[release] $*"
  "$@"
}

run cargo fmt --all --check
run cargo clippy --lib -p eggsec
run cargo check -p eggsec-python
run cargo check -p eggsec-python --features full-no-system
run cargo test --lib -p eggsec
run pytest crates/eggsec-python/tests/ crates/eggsec-python/python/tests/ \
  --strict-markers --ignore=crates/eggsec-python/tests/test_milestone_c.py \
  --ignore=crates/eggsec-python/tests/test_milestone_e.py \
  --ignore=crates/eggsec-python/tests/test_milestone_f.py
run bash scripts/check-architecture-guards.sh
run bash scripts/build_wheel_profiles.sh

while IFS= read -r wheel; do
  run bash scripts/validate_wheel.sh "$wheel"
done < <(find target/python-wheels -maxdepth 1 -type f -name '*.whl' | sort)

echo "PASS: Python release-candidate validation completed"
