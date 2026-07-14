#!/usr/bin/env bash
# Type stub validation for eggsec-python
# Checks that .pyi files are syntactically valid and that mypy can parse them
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
STUB_DIR="$ROOT_DIR/crates/eggsec-python/python/eggsec"

echo "=== Python Type Stub Validation ==="

# Step 1: Verify all .pyi files have valid Python syntax
echo ""
echo "--- Syntax validation ---"
SYNTAX_ERRORS=0
for stub in "$STUB_DIR"/*.pyi; do
    if python3 -c "import ast; ast.parse(open('$stub').read())" 2>/dev/null; then
        echo "  OK: $(basename "$stub")"
    else
        echo "  FAIL: $(basename "$stub")"
        SYNTAX_ERRORS=$((SYNTAX_ERRORS + 1))
    fi
done

if [ "$SYNTAX_ERRORS" -gt 0 ]; then
    echo "FAILED: $SYNTAX_ERRORS stub files have syntax errors"
    exit 1
fi

# Step 2: Verify __init__.py has valid syntax
echo ""
echo "--- __init__.py syntax ---"
if python3 -c "import ast; ast.parse(open('$STUB_DIR/__init__.py').read())" 2>/dev/null; then
    echo "  OK: __init__.py"
else
    echo "  FAIL: __init__.py"
    exit 1
fi

# Step 3: Run mypy on stubs (if installed)
echo ""
echo "--- mypy check ---"
if command -v mypy &>/dev/null; then
    mypy --ignore-missing-imports --no-error-summary "$STUB_DIR"/*.pyi 2>&1 || true
else
    echo "  SKIP: mypy not installed (pip install mypy)"
fi

# Step 4: Run pyright on stubs (if installed)
echo ""
echo "--- pyright check ---"
if command -v pyright &>/dev/null; then
    pyright "$STUB_DIR" 2>&1 || true
else
    echo "  SKIP: pyright not installed (npm install -g pyright)"
fi

echo ""
echo "=== Validation complete ==="
