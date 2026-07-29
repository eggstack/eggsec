#!/usr/bin/env bash
# Unified Python CI check: one venv, one maturin develop, all retained checks.
# Used by both `make check-python` and the CI `python` job.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
VENV_DIR="${EGGSEC_PYTHON_VENV:-$ROOT_DIR/.venv-ci}"

echo "=== eggsec-python unified check ==="
echo "Venv: $VENV_DIR"
echo ""

# ── 1. Create virtualenv ─────────────────────────────────────────────────
if [ ! -d "$VENV_DIR" ]; then
    echo "--- Creating virtualenv ---"
    python3 -m venv "$VENV_DIR"
fi
# shellcheck disable=SC1091
source "$VENV_DIR/bin/activate"

# ── 2. Install dependencies ──────────────────────────────────────────────
echo "--- Installing dependencies ---"
pip install --upgrade pip --quiet
pip install maturin pytest pytest-timeout mypy pyright --quiet

# ── 3. Build extension (once) ────────────────────────────────────────────
echo "--- Building extension (maturin develop) ---"
cd "$ROOT_DIR/crates/eggsec-python"
maturin develop
cd "$ROOT_DIR"
echo "Extension built successfully."
echo ""

# ── 4. Behavioral tests ─────────────────────────────────────────────────
echo "--- Behavioral tests ---"
cd "$ROOT_DIR/crates/eggsec-python"
EGGSEC_ALLOW_LOOPBACK_FIXTURE=1 \
    pytest tests/ python/tests/ \
    --timeout=60 \
    --tb=short \
    -q
cd "$ROOT_DIR"
echo ""

# ── 5. Capability and feature metadata ───────────────────────────────────
echo "--- Capability matrix ---"
python "$SCRIPT_DIR/check-python-capability-matrix.py"
echo ""

echo "--- Architecture guards ---"
python "$SCRIPT_DIR/check-python-architecture-guards.py"
echo ""

# ── 6. Stub parity ──────────────────────────────────────────────────────
echo "--- Stub parity ---"
python "$SCRIPT_DIR/check_python_stub_parity.py"
echo ""

# ── 7. Type checks ──────────────────────────────────────────────────────
echo "--- Type checks ---"
bash "$SCRIPT_DIR/check_python_types.sh"
echo ""

echo "=== All Python checks passed ==="
