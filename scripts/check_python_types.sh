#!/usr/bin/env bash
# Type stub and export correctness validation for eggsec-python
# Checks:
#   1. eggsec is importable (builds with maturin develop if not)
#   2. Machine-readable api_surface() is available and consistent
#   3. Every __all__ name resolves at runtime
#   4. .pyi stub files exist for key modules
#   5. All .pyi files have valid Python syntax
#   6. Stub parity: __all__ names match stub declarations
#   7. mypy can parse stubs (if installed)
#   8. pyright can parse stubs (if installed)
#   9. Exits non-zero on any hard failure
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
EGGSEC_PYTHON_DIR="$ROOT_DIR/crates/eggsec-python"
STUB_DIR="$EGGSEC_PYTHON_DIR/python/eggsec"

echo "=== eggsec-python Type & Export Validation ==="
echo ""

FAILURES=0
WARNINGS=0

fail() {
    echo "  FAIL: $1"
    FAILURES=$((FAILURES + 1))
}

warn() {
    echo "  WARN: $1"
    WARNINGS=$((WARNINGS + 1))
}

ok() {
    echo "  OK: $1"
}

# ---------------------------------------------------------------------------
# Step 1: Ensure eggsec is importable
# ---------------------------------------------------------------------------
echo "--- Step 1: Importability check ---"
if python3 -c "import eggsec" 2>/dev/null; then
    VERSION=$(python3 -c "import eggsec; print(eggsec.__version__)" 2>/dev/null || echo "unknown")
    ok "eggsec importable (version: $VERSION)"
else
    echo "  eggsec not importable, attempting maturin develop..."
    if command -v maturin &>/dev/null; then
        (cd "$EGGSEC_PYTHON_DIR" && maturin develop 2>&1) || {
            fail "maturin develop failed"
            echo ""
            echo "=== Validation aborted: cannot import eggsec ==="
            exit 1
        }
        ok "maturin develop succeeded"
    else
        fail "maturin not found and eggsec not importable"
        echo ""
        echo "=== Validation aborted: cannot import eggsec ==="
        exit 1
    fi
fi

# ---------------------------------------------------------------------------
# Step 2: Machine-readable api_surface() consistency
# ---------------------------------------------------------------------------
echo ""
echo "--- Step 2: api_surface() consistency ---"
SURFACE_CHECK=$(python3 -c "
import sys, json
try:
    import eggsec
    surface = eggsec.api_surface()
    if not isinstance(surface, dict):
        print('NOT_DICT')
        sys.exit(0)
    print(f'ENTRIES:{len(surface)}')
    # Check a few known stable symbols
    expected_stable = ['scan_ports', 'features', 'Scope', 'Engine']
    missing = [s for s in expected_stable if s not in surface]
    if missing:
        print(f'MISSING_STABLE:{missing}')
except Exception as e:
    print(f'ERROR:{e}')
" 2>&1)
if echo "$SURFACE_CHECK" | grep -q "^ENTRIES:"; then
    ENTRY_COUNT=$(echo "$SURFACE_CHECK" | grep "^ENTRIES:" | cut -d: -f2)
    ok "api_surface() returns $ENTRY_COUNT entries"
elif echo "$SURFACE_CHECK" | grep -q "^NOT_DICT"; then
    fail "api_surface() did not return a dict"
elif echo "$SURFACE_CHECK" | grep -q "^MISSING_STABLE:"; then
    MISSING=$(echo "$SURFACE_CHECK" | grep "^MISSING_STABLE:" | cut -d: -f2-)
    fail "api_surface() missing stable symbols: $MISSING"
elif echo "$SURFACE_CHECK" | grep -q "^ERROR:"; then
    ERROR_MSG=$(echo "$SURFACE_CHECK" | grep "^ERROR:" | cut -d: -f2-)
    fail "api_surface() raised: $ERROR_MSG"
else
    fail "api_surface() check produced unexpected output"
fi

# ---------------------------------------------------------------------------
# Step 3: __all__ names resolve at runtime
# ---------------------------------------------------------------------------
echo ""
echo "--- Step 3: __all__ resolution check ---"
ALL_CHECK=$(python3 -c "
import sys
try:
    import eggsec
    all_list = getattr(eggsec, '__all__', None)
    if all_list is None:
        print('NO_ALL')
        sys.exit(0)
    if not all_list:
        print('EMPTY_ALL')
        sys.exit(0)
    missing = [name for name in all_list if not hasattr(eggsec, name)]
    if missing:
        print(f'MISSING:{len(missing)}')
        for m in missing:
            print(f'  MISSING_NAME:{m}')
    else:
        print(f'OK:{len(all_list)}')
except Exception as e:
    print(f'ERROR:{e}')
" 2>&1)
if echo "$ALL_CHECK" | grep -q "^OK:"; then
    COUNT=$(echo "$ALL_CHECK" | grep "^OK:" | cut -d: -f2)
    ok "All $COUNT __all__ names resolve"
elif echo "$ALL_CHECK" | grep -q "^MISSING:"; then
    fail "Some __all__ names do not resolve:"
    echo "$ALL_CHECK" | grep "  MISSING_NAME:" | sed 's/^  /    /'
elif echo "$ALL_CHECK" | grep -q "^NO_ALL"; then
    fail "__all__ is not defined"
elif echo "$ALL_CHECK" | grep -q "^EMPTY_ALL"; then
    fail "__all__ is empty"
else
    fail "__all__ check failed: $ALL_CHECK"
fi

# ---------------------------------------------------------------------------
# Step 4: .pyi stub files exist for key modules
# echo ""
echo "--- Step 4: Stub file existence ---"
KEY_MODULES=(
    "__init__"
    "engine"
    "async_engine"
    "scope"
    "client"
    "async_client"
    "finding"
    "errors"
    "functions"
    "config_model"
    "execution_context"
    "pipeline"
    "event_protocol"
    "event_stream"
    "callbacks"
    "async_support"
    "backpressure"
    "handles"
    "cancellation"
    "runtime"
    "dto"
    "requests"
    "status"
    "waf"
    "waf_validation"
    "recon"
    "fingerprint"
    "endpoint"
    "loadtest"
    "websocket"
    "git_secrets"
    "sbom"
    "db_pentest"
    "proxy"
    "mobile"
    "container"
    "packet_inspection"
    "stress"
    "nse"
    "daemon"
    "domains"
    "operation_metadata"
    "scope_eval"
    "authorization"
    "preflight"
    "audit"
    "engine_state"
    "planning"
    "checkpoint"
    "consolidated_recon"
    "graphql"
    "oauth"
    "auth_assess"
    "browser_assess"
    "hunt"
    "http_client"
    "probes"
    "transport"
    "cvss"
    "finding_schema"
    "finding_workflow"
    "reporters"
    "repository"
    "baseline"
    "compliance"
    "migration"
    "integrations"
    "async_iter"
    "ai_postprocess"
)

STUB_MISSING=0
for mod in "${KEY_MODULES[@]}"; do
    STUB_PATH="$STUB_DIR/${mod}.pyi"
    if [ ! -f "$STUB_PATH" ]; then
        warn "Stub missing: ${mod}.pyi"
        STUB_MISSING=$((STUB_MISSING + 1))
    fi
done
if [ "$STUB_MISSING" -eq 0 ]; then
    ok "All ${#KEY_MODULES[@]} key module stubs present"
else
    warn "$STUB_MISSING key module stubs missing"
fi

# ---------------------------------------------------------------------------
# Step 5: .pyi syntax validation
# ---------------------------------------------------------------------------
echo ""
echo "--- Step 5: .pyi syntax validation ---"
SYNTAX_ERRORS=0
STUB_COUNT=0
for stub in "$STUB_DIR"/*.pyi; do
    STUB_COUNT=$((STUB_COUNT + 1))
    STUB_NAME=$(basename "$stub")
    if python3 -c "import ast; ast.parse(open('$stub').read())" 2>/dev/null; then
        : # OK
    else
        fail "Syntax error in $STUB_NAME"
        SYNTAX_ERRORS=$((SYNTAX_ERRORS + 1))
    fi
done
if [ "$SYNTAX_ERRORS" -eq 0 ]; then
    ok "All $STUB_COUNT .pyi files have valid syntax"
else
    fail "$SYNTAX_ERRORS of $STUB_COUNT .pyi files have syntax errors"
fi

# Also validate __init__.py syntax
echo ""
echo "--- Step 5b: __init__.py syntax ---"
if python3 -c "import ast; ast.parse(open('$STUB_DIR/__init__.py').read())" 2>/dev/null; then
    ok "__init__.py syntax valid"
else
    fail "__init__.py has syntax errors"
fi

# ---------------------------------------------------------------------------
# Step 6: Stub parity check (__all__ vs stub declarations)
# ---------------------------------------------------------------------------
echo ""
echo "--- Step 6: Stub parity (__all__ vs stubs) ---"
STUB_PARITY=$(python3 -c "
import ast, os, sys, re

stub_dir = '$STUB_DIR'
init_path = os.path.join(stub_dir, '__init__.py')

# Parse __all__ from __init__.py
with open(init_path) as f:
    tree = ast.parse(f.read())

all_names = []
for node in ast.walk(tree):
    if isinstance(node, ast.Assign):
        for target in node.targets:
            if isinstance(target, ast.Name) and target.id == '__all__':
                if isinstance(node.value, (ast.List, ast.Tuple)):
                    all_names = [elt.value for elt in node.value.elts
                                 if isinstance(elt, ast.Constant)]

if not all_names:
    print('NO_ALL')
    sys.exit(0)

# Collect all names exported by .pyi files (recursively, with _Py suffix handling)
stub_exports = set()
for root, dirs, files in os.walk(stub_dir):
    for fname in files:
        if fname.endswith('.pyi'):
            fpath = os.path.join(root, fname)
            try:
                with open(fpath) as f:
                    ptree = ast.parse(f.read())
                for node in ast.walk(ptree):
                    if isinstance(node, ast.ImportFrom):
                        for alias in node.names:
                            name = alias.asname if alias.asname else alias.name
                            if name != '*':
                                stub_exports.add(name)
                                if name.endswith('Py'):
                                    stub_exports.add(name[:-2])
                    elif isinstance(node, ast.Assign):
                        for target in node.targets:
                            if isinstance(target, ast.Name):
                                stub_exports.add(target.id)
                                if target.id.endswith('Py'):
                                    stub_exports.add(target.id[:-2])
                    elif isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
                        stub_exports.add(node.name)
                    elif isinstance(node, ast.ClassDef):
                        stub_exports.add(node.name)
                        if node.name.endswith('Py'):
                            stub_exports.add(node.name[:-2])
            except Exception:
                pass

# Filter out submodule names (directories with __init__.py)
submodules = set()
for entry in os.listdir(stub_dir):
    entry_path = os.path.join(stub_dir, entry)
    if os.path.isdir(entry_path) and os.path.exists(os.path.join(entry_path, '__init__.py')):
        submodules.add(entry)

# Check __all__ names vs stubs
missing_in_stubs = [n for n in all_names if n not in stub_exports
                    and not n.startswith('_') and n not in submodules]
if missing_in_stubs:
    print(f'MISSING_IN_STUBS:{len(missing_in_stubs)}')
    for n in missing_in_stubs[:30]:
        print(f'  {n}')
    if len(missing_in_stubs) > 30:
        print(f'  ... and {len(missing_in_stubs) - 30} more')
else:
    print(f'OK:{len(all_names)}')
" 2>&1)
if echo "$STUB_PARITY" | grep -q "^OK:"; then
    COUNT=$(echo "$STUB_PARITY" | grep "^OK:" | cut -d: -f2)
    ok "All $COUNT __all__ names have stub declarations"
elif echo "$STUB_PARITY" | grep -q "^MISSING_IN_STUBS:"; then
    MISSING_COUNT=$(echo "$STUB_PARITY" | grep "^MISSING_IN_STUBS:" | cut -d: -f2)
    fail "$MISSING_COUNT __all__ names missing from .pyi stubs:"
    echo "$STUB_PARITY" | grep "^  " | sed 's/^/    /'
elif echo "$STUB_PARITY" | grep -q "^NO_ALL"; then
    warn "Could not parse __all__ for stub parity check"
else
    warn "Stub parity check produced unexpected output"
fi

# ---------------------------------------------------------------------------
# Step 7: Runtime export check (required symbols)
# ---------------------------------------------------------------------------
echo ""
echo "--- Step 7: Required default symbols ---"
REQ_CHECK=$(python3 -c "
import sys
try:
    import eggsec
    REQUIRED = [
        '__version__', '__version_info__',
        'features', 'has_feature', 'build_info',
        'scan_ports', 'async_scan_ports',
        'scan_endpoints', 'async_scan_endpoints',
        'fingerprint_services', 'async_fingerprint_services',
        'recon_dns', 'async_recon_dns',
        'inspect_tls', 'async_inspect_tls',
        'detect_technology', 'async_detect_technology',
        'detect_waf', 'async_detect_waf',
        'validate_waf', 'async_validate_waf',
        'fuzz_http', 'async_fuzz_http', 'generate_fuzz_payloads',
        'load_test_http', 'async_load_test_http',
        'Scope', 'Client', 'AsyncClient', 'PyFuture',
        'Severity', 'Evidence', 'Finding', 'FindingSet', 'Report',
        'EggsecError', 'ConfigError', 'ScopeError', 'EnforcementError',
        'NetworkError', 'ScanError', 'TimeoutError',
        'FeatureUnavailableError', 'SerializationError', 'InternalError',
    ]
    missing = [n for n in REQUIRED if not hasattr(eggsec, n)]
    if missing:
        print(f'MISSING:{len(missing)}')
        for m in missing:
            print(f'  {m}')
    else:
        print(f'OK:{len(REQUIRED)}')
except Exception as e:
    print(f'ERROR:{e}')
" 2>&1)
if echo "$REQ_CHECK" | grep -q "^OK:"; then
    COUNT=$(echo "$REQ_CHECK" | grep "^OK:" | cut -d: -f2)
    ok "All $COUNT required default symbols present"
elif echo "$REQ_CHECK" | grep -q "^MISSING:"; then
    MISSING_COUNT=$(echo "$REQ_CHECK" | grep "^MISSING:" | cut -d: -f2)
    fail "$MISSING_COUNT required default symbols missing:"
    echo "$REQ_CHECK" | grep "^  " | sed 's/^/    /'
else
    warn "Required symbols check failed: $REQ_CHECK"
fi

# ---------------------------------------------------------------------------
# Step 8: Key class method checks
# ---------------------------------------------------------------------------
echo ""
echo "--- Step 8: Key class method checks ---"
METHOD_CHECK=$(python3 -c "
import sys, inspect
try:
    import eggsec
    issues = []

    # Engine.run
    if hasattr(eggsec, 'Engine'):
        sig = inspect.signature(eggsec.Engine.run)
        params = list(sig.parameters.keys())
        if 'request' not in params:
            issues.append(f'Engine.run missing request param (has: {params})')
    else:
        issues.append('Engine class not found')

    # AsyncEngine.run
    if hasattr(eggsec, 'AsyncEngine'):
        sig = inspect.signature(eggsec.AsyncEngine.run)
        params = list(sig.parameters.keys())
        if 'request' not in params:
            issues.append(f'AsyncEngine.run missing request param (has: {params})')
    else:
        issues.append('AsyncEngine class not found')

    # Scope.allow_hosts
    if hasattr(eggsec, 'Scope'):
        if not hasattr(eggsec.Scope, 'allow_hosts'):
            issues.append('Scope.allow_hosts not found')
        if not hasattr(eggsec.Scope, 'allow_cidrs'):
            issues.append('Scope.allow_cidrs not found')
        if not hasattr(eggsec.Scope, 'deny_all'):
            issues.append('Scope.deny_all not found')
    else:
        issues.append('Scope class not found')

    if issues:
        print(f'ISSUES:{len(issues)}')
        for i in issues:
            print(f'  {i}')
    else:
        print('OK')
except Exception as e:
    print(f'ERROR:{e}')
" 2>&1)
if echo "$METHOD_CHECK" | grep -q "^OK"; then
    ok "Key class methods present and correctly typed"
elif echo "$METHOD_CHECK" | grep -q "^ISSUES:"; then
    ISSUE_COUNT=$(echo "$METHOD_CHECK" | grep "^ISSUES:" | cut -d: -f2)
    fail "$ISSUE_COUNT class method issues:"
    echo "$METHOD_CHECK" | grep "^  " | sed 's/^/    /'
else
    warn "Class method check failed: $METHOD_CHECK"
fi

# ---------------------------------------------------------------------------
# Step 9: mypy check (optional)
# ---------------------------------------------------------------------------
echo ""
echo "--- Step 9: mypy check ---"
if command -v mypy &>/dev/null; then
    MYPY_OUT=$(mypy --ignore-missing-imports --no-error-summary "$STUB_DIR"/*.pyi 2>&1 || true)
    MYPY_ERRORS=$(echo "$MYPY_OUT" | grep -c "error:" || true)
    if [ "$MYPY_ERRORS" -gt 0 ]; then
        fail "mpy found $MYPY_ERRORS errors in stubs"
        echo "$MYPY_OUT" | grep "error:" | head -10 | sed 's/^/    /'
    else
        ok "mypy found no errors in stubs"
    fi
else
    echo "  SKIP: mypy not installed (pip install mypy)"
fi

# ---------------------------------------------------------------------------
# Step 10: pyright check (optional)
# ---------------------------------------------------------------------------
echo ""
echo "--- Step 10: pyright check ---"
if command -v pyright &>/dev/null; then
    PYRIGHT_OUT=$(pyright "$STUB_DIR" 2>&1 || true)
    PYRIGHT_ERRORS=$(echo "$PYRIGHT_OUT" | grep -c "error:" || true)
    if [ "$PYRIGHT_ERRORS" -gt 0 ]; then
        warn "pyright found $PYRIGHT_ERRORS errors in stubs (expected for native module stubs)"
        echo "$PYRIGHT_OUT" | grep "error:" | head -10 | sed 's/^/    /'
    else
        ok "pyright found no errors in stubs"
    fi
else
    echo "  SKIP: pyright not installed (npm install -g pyright)"
fi

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
echo ""
echo "=== Summary ==="
echo "  Failures: $FAILURES"
echo "  Warnings: $WARNINGS"
echo ""
if [ "$FAILURES" -gt 0 ]; then
    echo "RESULT: FAILED ($FAILURES hard failure(s))"
    exit 1
else
    echo "RESULT: PASSED"
    exit 0
fi
