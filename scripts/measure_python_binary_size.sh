#!/usr/bin/env bash
set -euo pipefail

OUTDIR="target/python-validation"
mkdir -p "$OUTDIR"

PYTHON=${PYTHON:-python3}
COMMIT=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")
PYTHON_VER=$($PYTHON --version 2>&1 | awk '{print $2}')
RUSTC_VER=$(rustc --version 2>&1 | awk '{print $2}')
PLATFORM=$(uname -m)-$(uname -s | tr '[:upper:]' '[:lower:]')

echo "=== Workstream 13: Binary size measurement ==="
echo "Platform:      $PLATFORM"
echo "Python:        $PYTHON_VER"
echo "Rust:          $RUSTC_VER"
echo "Commit:        $COMMIT"
echo

# --- Step 1: Default wheel ---
echo "[1/5] Building default wheel..."
rm -f "$OUTDIR"/*.whl
cd crates/eggsec-python
maturin build --release -o "../../$OUTDIR/" 2>&1 | tail -5
cd ../...

DEFAULT_WHL=$(ls "$OUTDIR"/*.whl 2>/dev/null | head -1)
if [ -z "$DEFAULT_WHL" ]; then
    echo "ERROR: Default wheel build failed"
    exit 1
fi
DEFAULT_SIZE=$(stat --format=%s "$DEFAULT_WHL")
echo "  Default wheel: $DEFAULT_WHL ($DEFAULT_SIZE bytes)"

# --- Step 2: Full wheel (attempt, may fail) ---
echo "[2/5] Attempting full-no-system wheel build..."
BROAD_SIZE="null"
BROAD_NOTE=""
rm -f "$OUTDIR"/*.whl
cd crates/eggsec-python
if maturin build --release --features full-no-system -o "../../$OUTDIR/" 2>&1 | tail -3; then
    BROAD_WHL=$(ls "$OUTDIR"/*.whl 2>/dev/null | head -1)
    if [ -n "$BROAD_WHL" ]; then
        BROAD_SIZE=$(stat --format=%s "$BROAD_WHL")
        echo "  Full wheel: $BROAD_WHL ($BROAD_SIZE bytes)"
    fi
else
    BROAD_NOTE="full-no-system features fail to compile in eggsec-python"
    echo "  Full build failed (see note in report)"
fi
cd ../..

# Rebuild default wheel if full build wiped it
if [ ! -f "$DEFAULT_WHL" ]; then
    echo "  Rebuilding default wheel (full build cleaned output)..."
    cd crates/eggsec-python
    maturin build --release -o "../../$OUTDIR/" 2>&1 | tail -3
    cd ../...
    DEFAULT_WHL=$(ls "$OUTDIR"/*.whl 2>/dev/null | head -1)
    DEFAULT_SIZE=$(stat --format=%s "$DEFAULT_WHL")
fi

# --- Step 3: Install default wheel in temp venv ---
echo "[3/5] Installing default wheel in temp venv..."
VENV_DIR=$(mktemp -d)
$PYTHON -m venv "$VENV_DIR"
"$VENV_DIR/bin/pip" install --quiet "$DEFAULT_WHL" 2>&1

EXT_SIZE=0
NATIVE_DEPS=0
SO_FILE=$(find "$VENV_DIR" -name "eggsec*.so" -type f 2>/dev/null | head -1)

if [ -n "$SO_FILE" ] && [ -f "$SO_FILE" ]; then
    EXT_SIZE=$(stat --format=%s "$SO_FILE")
    NATIVE_DEPS=$(ldd "$SO_FILE" 2>/dev/null | grep -c '=> /' || echo 0)
    echo "  Extension: $SO_FILE"
    echo "  Extension size: $EXT_SIZE bytes"
    echo "  Native deps: $NATIVE_DEPS"
else
    echo "  WARNING: Could not locate installed .so"
fi

# --- Step 4: Measure import time ---
echo "[4/5] Measuring import time..."
IMPORT_RESULT=$("$VENV_DIR/bin/python" -c "
import time
t0 = time.perf_counter()
import_error = ''
try:
    import eggsec
except ImportError as e:
    import_error = str(e)
elapsed = time.perf_counter() - t0
print(f'{elapsed:.4f}')
if import_error:
    print(f'IMPORT_ERROR:{import_error}')
" 2>&1 || echo "0.0000")
IMPORT_TIME=$(echo "$IMPORT_RESULT" | head -1)
IMPORT_ERROR=$(echo "$IMPORT_RESULT" | grep '^IMPORT_ERROR:' | sed 's/^IMPORT_ERROR://' || true)
echo "  Import time: ${IMPORT_TIME}s"
if [ -n "$IMPORT_ERROR" ]; then
    echo "  Import error: $IMPORT_ERROR"
fi

# Raw native module load time
RAW_IMPORT_TIME=$("$VENV_DIR/bin/python" -c "
import time, importlib.util, glob
t0 = time.perf_counter()
matches = glob.glob('$VENV_DIR/lib/python*/site-packages/eggsec/_core*.so')
if matches:
    spec = importlib.util.spec_from_file_location('_core', matches[0])
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
elapsed = time.perf_counter() - t0
print(f'{elapsed:.4f}')
" 2>&1 || echo "0.0000")
echo "  Native load time: ${RAW_IMPORT_TIME}s"

# --- Step 5: Cleanup and write report ---
echo "[5/5] Cleaning up and writing report..."
rm -rf "$VENV_DIR"

# Escape quotes in error message for JSON
IMPORT_ERROR_JSON=$(echo "$IMPORT_ERROR" | sed 's/"/\\"/g' || true)

cat > "$OUTDIR/binary-size-report.json" <<EOJSON
{
  "default_wheel_size_bytes": $DEFAULT_SIZE,
  "broad_wheel_size_bytes": $BROAD_SIZE,
  "broad_wheel_build_note": "${BROAD_NOTE}",
  "installed_extension_size_bytes": $EXT_SIZE,
  "native_dependency_count": $NATIVE_DEPS,
  "import_time_seconds": $IMPORT_TIME,
  "native_load_time_seconds": $RAW_IMPORT_TIME,
  "import_error_note": "${IMPORT_ERROR_JSON}",
  "platform": "$PLATFORM",
  "python_version": "$PYTHON_VER",
  "rustc_version": "$RUSTC_VER",
  "commit_hash": "$COMMIT"
}
EOJSON

echo
echo "=== Report ==="
cat "$OUTDIR/binary-size-report.json"
