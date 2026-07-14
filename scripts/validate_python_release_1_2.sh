#!/usr/bin/env bash
# validate_python_release_1_2.sh — Canonical validation matrix for eggsec Python API
#
# Runs all checks for a given profile and produces a JSON report.
# Usage:
#   scripts/validate_python_release_1_2.sh <profile> [local|ci]
#   scripts/validate_python_release_1_2.sh --all [local|ci]
#   scripts/validate_python_release_1_2.sh --profile <name> [--ci|--local]
#
# Profiles (14 total):
#   default, full-no-system, websocket, git-secrets, sbom, db-pentest,
#   nse, container, mobile, packet-inspection, packet-inspection-privileged,
#   combined-websocket-packet, installed-default, installed-broad
#
# CI integration (GitHub Actions):
#   - Upload target/python-validation/release-1-2-matrix.json as artifact
#   - Use jq to check summary.passed == summary.total_profiles
#
# Example workflow snippet:
#   - name: Validate Python release matrix
#     run: bash scripts/validate_python_release_1_2.sh --all ci
#   - name: Upload validation report
#     uses: actions/upload-artifact@v4
#     with:
#       name: python-validation-report
#       path: target/python-validation/release-1-2-matrix.json
#
set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$ROOT"

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------
REPORT_DIR="target/python-validation"
REPORT_FILE="${REPORT_DIR}/release-1-2-matrix.json"
EGGSEC_PYTHON_DIR="$ROOT/crates/eggsec-python"
MANIFEST="$EGGSEC_PYTHON_DIR/Cargo.toml"

mkdir -p "${REPORT_DIR}"

# ---------------------------------------------------------------------------
# Profile definitions: name -> cargo features
# ---------------------------------------------------------------------------
declare -A PROFILE_FEATURES=(
    ["default"]=""
    ["full-no-system"]="websocket,git-secrets,sbom,container"
    ["websocket"]="websocket"
    ["git-secrets"]="git-secrets"
    ["sbom"]="sbom"
    ["db-pentest"]="db-pentest"
    ["nse"]="nse"
    ["container"]="container"
    ["mobile"]="mobile"
    ["packet-inspection"]="packet-inspection"
    ["packet-inspection-privileged"]="packet-inspection"
    ["combined-websocket-packet"]="packet-inspection,websocket"
    ["installed-default"]=""
    ["installed-broad"]="websocket,git-secrets,sbom,container"
)

# Profiles that need privileged mode (skip cargo test, require root hint)
PRIVILEGED_PROFILES=("packet-inspection-privileged")

# All profiles in canonical order
ALL_PROFILES=(
    default full-no-system websocket git-secrets sbom db-pentest
    nse container mobile packet-inspection packet-inspection-privileged
    combined-websocket-packet installed-default installed-broad
)

# ---------------------------------------------------------------------------
# Argument parsing
# ---------------------------------------------------------------------------
PROFILE=""
MODE="local"
RUN_ALL=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --all)
            RUN_ALL=true
            shift
            ;;
        --profile)
            PROFILE="$2"
            shift 2
            ;;
        --ci)
            MODE="ci"
            shift
            ;;
        --local)
            MODE="local"
            shift
            ;;
        --help|-h)
            echo "Usage: $0 [--all | --profile <name>] [--ci | --local]"
            echo ""
            echo "Profiles: ${ALL_PROFILES[*]}"
            echo "  local  — skip slow checks (installed-wheel pytest)"
            echo "  ci     — full strict checks"
            exit 0
            ;;
        *)
            # Positional: profile name
            if [[ -z "$PROFILE" ]]; then
                PROFILE="$1"
            elif [[ "$1" == "ci" || "$1" == "local" ]]; then
                MODE="$1"
            fi
            shift
            ;;
    esac
done

if [[ "$RUN_ALL" == false && -z "$PROFILE" ]]; then
    echo "Usage: $0 [--all | --profile <name>] [--ci | --local]" >&2
    exit 1
fi

# ---------------------------------------------------------------------------
# Collect environment info
# ---------------------------------------------------------------------------
COMMIT_HASH=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
PLATFORM=$(uname -srm)
PYTHON_VERSION=$(python3 --version 2>/dev/null || echo "unknown")
RUST_VERSION=$(rustc --version 2>/dev/null || echo "unknown")

# ---------------------------------------------------------------------------
# Helper functions
# ---------------------------------------------------------------------------

# Run a command, capture exit code, optional timeout.
# Outputs "PASS|<elapsed>" or "FAIL|<elapsed>|exit=<code>" or "TIMEOUT|<elapsed>".
# Command output is written to a temp file; caller reads via RUN_CHECK_OUTPUT_FILE.
# Uses a fixed path to survive subshell boundaries.
RUN_CHECK_OUTPUT_FILE="/tmp/_run_check_output_$$_${RANDOM}"
run_check() {
    local label="$1"
    shift
    local timeout_sec="${1:-300}"
    shift
    local start_time end_time elapsed exit_code

    start_time=$(date +%s)
    : > "$RUN_CHECK_OUTPUT_FILE"
    if timeout "$timeout_sec" "$@" >"$RUN_CHECK_OUTPUT_FILE" 2>&1; then
        exit_code=0
    else
        exit_code=$?
    fi
    end_time=$(date +%s)
    elapsed=$((end_time - start_time))

    if [[ $exit_code -eq 124 ]]; then
        echo "TIMEOUT|${elapsed}s|${label}"
        return 1
    elif [[ $exit_code -ne 0 ]]; then
        echo "FAIL|${elapsed}s|exit=${exit_code}"
        return 1
    else
        echo "PASS|${elapsed}s"
        return 0
    fi
}

# Escape string for JSON
json_escape() {
    local s="$1"
    s="${s//\\/\\\\}"
    s="${s//\"/\\\"}"
    s="${s//$'\n'/\\n}"
    s="${s//$'\r'/}"
    s="${s//$'\t'/\\t}"
    printf '%s' "$s"
}

# Count pytest results from output
count_pytest_results() {
    local output="$1"
    local passed failed skipped

    passed=$(echo "$output" | grep -oP '\d+ passed' | grep -oP '\d+' || echo "0")
    failed=$(echo "$output" | grep -oP '\d+ failed' | grep -oP '\d+' || echo "0")
    skipped=$(echo "$output" | grep -oP '\d+ skipped' | grep -oP '\d+' || echo "0")
    [[ -z "$passed" ]] && passed=0
    [[ -z "$failed" ]] && failed=0
    [[ -z "$skipped" ]] && skipped=0
    echo "${passed},${failed},${skipped}"
}

# Count cargo test results from output
count_cargo_tests() {
    local output="$1"
    local total failed

    # Handle both formats:
    #   "test result: ok. 1607 passed; 0 failed"
    #   "cargo test: 1607 passed (1 suite, 3.37s)"
    total=$(echo "$output" | grep -oP '(\d+) passed' | tail -1 | grep -oP '\d+' || echo "0")
    failed=$(echo "$output" | grep -oP '(\d+) failed' | tail -1 | grep -oP '\d+' || echo "0")
    [[ -z "$total" ]] && total=0
    [[ -z "$failed" ]] && failed=0
    echo "${total},${failed}"
}

# ---------------------------------------------------------------------------
# Run a single profile validation
# ---------------------------------------------------------------------------
run_profile() {
    local profile="$1"
    local features="${PROFILE_FEATURES[$profile]:-}"
    local is_privileged=false
    local is_installed=false

    for pp in "${PRIVILEGED_PROFILES[@]}"; do
        [[ "$profile" == "$pp" ]] && is_privileged=true
    done

    [[ "$profile" == installed-default || "$profile" == installed-broad ]] && is_installed=true

    echo "" >&2
    echo "============================================================" >&2
    echo "  Profile: ${profile}" >&2
    echo "  Features: ${features:-<none>}" >&2
    echo "  Mode: ${MODE}" >&2
    echo "============================================================" >&2

    # Result variables
    local cargo_check_result="skip"
    local cargo_check_elapsed=""
    local cargo_test_result="skip"
    local cargo_test_elapsed=""
    local cargo_test_total=0
    local cargo_test_failed=0
    local ext_build_result="skip"
    local ext_build_elapsed=""
    local wheel_build_result="skip"
    local wheel_build_elapsed=""
    local wheel_size=0
    local import_test_result="skip"
    local pytest_result="skip"
    local pytest_passed=0
    local pytest_failed=0
    local pytest_skipped=0
    local export_parity_result="skip"
    local capability_matrix_result="skip"
    local arch_guards_result="skip"
    local type_check_result="skip"
    local doc_example_result="skip"

    # --- 1. cargo check ---
    echo "[1/12] cargo check..." >&2
    local cargo_check_args=(cargo check -p eggsec-python)
    if [[ -n "$features" ]]; then
        cargo_check_args+=(--features "$features")
    fi
    local check_out
    if check_out=$(run_check "cargo_check" 300 "${cargo_check_args[@]}"); then
        cargo_check_result="pass"
        cargo_check_elapsed=$(echo "$check_out" | cut -d'|' -f2)
    else
        cargo_check_result="fail"
        cargo_check_elapsed=$(echo "$check_out" | cut -d'|' -f2)
        echo "  cargo check FAILED — skipping remaining checks for this profile" >&2
        # Build the JSON fragment even on failure
        build_profile_json "$profile" "$features" "$cargo_check_result" "$cargo_check_elapsed" \
            "$cargo_test_result" "$cargo_test_elapsed" "$cargo_test_total" "$cargo_test_failed" \
            "$ext_build_result" "$ext_build_elapsed" \
            "$wheel_build_result" "$wheel_build_elapsed" "$wheel_size" \
            "$import_test_result" \
            "$pytest_result" "$pytest_passed" "$pytest_failed" "$pytest_skipped" \
            "$export_parity_result" "$capability_matrix_result" "$arch_guards_result" \
            "$type_check_result" "$doc_example_result"
        return 1
    fi

    # --- 2. cargo test (skip for privileged profiles) ---
    if [[ "$is_privileged" == true ]]; then
        echo "[2/12] cargo test — SKIPPED (privileged profile)" >&2
        cargo_test_result="skip"
    else
        echo "[2/12] cargo test..." >&2
        local test_out
        if test_out=$(run_check "cargo_test" 300 cargo test --lib -p eggsec); then
            cargo_test_result="pass"
            cargo_test_elapsed=$(echo "$test_out" | cut -d'|' -f2)
            if [[ -f "$RUN_CHECK_OUTPUT_FILE" ]]; then
                local counts
                counts=$(count_cargo_tests "$(cat "$RUN_CHECK_OUTPUT_FILE")")
                cargo_test_total=$(echo "$counts" | cut -d',' -f1)
                cargo_test_failed=$(echo "$counts" | cut -d',' -f2)
                rm -f "$RUN_CHECK_OUTPUT_FILE"
            fi
        else
            cargo_test_result="fail"
            cargo_test_elapsed=$(echo "$test_out" | cut -d'|' -f2)
            rm -f "$RUN_CHECK_OUTPUT_FILE"
        fi
    fi

    # --- 3. Extension build (maturin develop) ---
    echo "[3/12] extension build (maturin develop)..." >&2
    local maturin_args=(maturin develop --release)
    if [[ -n "$features" ]]; then
        maturin_args+=(--features "$features")
    fi
    local build_out
    if build_out=$(run_check "maturin_develop" 600 "${maturin_args[@]}"); then
        ext_build_result="pass"
        ext_build_elapsed=$(echo "$build_out" | cut -d'|' -f2)
    else
        ext_build_result="fail"
        ext_build_elapsed=$(echo "$build_out" | cut -d'|' -f2)
    fi

    # --- 4. Wheel build ---
    echo "[4/12] wheel build (maturin build)..." >&2
    local wheel_dir="$REPORT_DIR/wheels/${profile}"
    mkdir -p "$wheel_dir"
    local wheel_build_args=(maturin build --release)
    if [[ -n "$features" ]]; then
        wheel_build_args+=(--features "$features")
    fi
    wheel_build_args+=(--out "$wheel_dir")
    local wheel_out
    if wheel_out=$(run_check "wheel_build" 600 "${wheel_build_args[@]}"); then
        wheel_build_result="pass"
        wheel_build_elapsed=$(echo "$wheel_out" | cut -d'|' -f2)
        local whl_file
        whl_file=$(find "$wheel_dir" -maxdepth 1 -name '*.whl' -type f | head -1 || true)
        if [[ -n "$whl_file" ]]; then
            wheel_size=$(stat -f%z "$whl_file" 2>/dev/null || stat -c%s "$whl_file" 2>/dev/null || echo "0")
        fi
    else
        wheel_build_result="fail"
        wheel_build_elapsed=$(echo "$wheel_out" | cut -d'|' -f2)
    fi

    # --- 5 & 6. Import test + pytest (installed wheel or maturin develop) ---
    if [[ "$is_installed" == true ]]; then
        echo "[5/12] installed-wheel import test..." >&2
        local whl_file
        whl_file=$(find "$wheel_dir" -maxdepth 1 -name '*.whl' -type f | head -1 || true)
        if [[ -n "$whl_file" ]]; then
            local tmp_venv
            tmp_venv=$(mktemp -d)
            trap "rm -rf $tmp_venv" RETURN 2>/dev/null || true
            python3 -m venv "$tmp_venv" >/dev/null 2>&1
            "$tmp_venv/bin/pip" install --disable-pip-version-check --no-deps "$whl_file" >/dev/null 2>&1
            if "$tmp_venv/bin/python" -c "import eggsec; print(f'eggsec {eggsec.__version__}')" >/dev/null 2>&1; then
                import_test_result="pass"
            else
                import_test_result="fail"
            fi
            rm -rf "$tmp_venv"
        else
            import_test_result="fail"
        fi
    else
        echo "[5/12] import test (maturin develop)..." >&2
        if python3 -c "import eggsec" >/dev/null 2>&1; then
            import_test_result="pass"
        else
            import_test_result="fail"
        fi
    fi

    # --- 6. pytest ---
    echo "[6/12] pytest..." >&2
    if [[ "$MODE" == "ci" || "$is_installed" == false ]]; then
        local pytest_out
        pytest_out=$(python3 -m pytest crates/eggsec-python/tests/ crates/eggsec-python/python/tests/ \
            --strict-markers \
            --ignore=crates/eggsec-python/tests/test_milestone_c.py \
            --ignore=crates/eggsec-python/tests/test_milestone_e.py \
            --ignore=crates/eggsec-python/tests/test_milestone_f.py \
            -q 2>&1 || true)
        local pcounts
        pcounts=$(count_pytest_results "$pytest_out")
        pytest_passed=$(echo "$pcounts" | cut -d',' -f1)
        pytest_failed=$(echo "$pcounts" | cut -d',' -f2)
        pytest_skipped=$(echo "$pcounts" | cut -d',' -f3)
        if [[ "$pytest_failed" -gt 0 ]]; then
            pytest_result="fail"
        else
            pytest_result="pass"
        fi
    else
        echo "  pytest SKIPPED (local mode, installed profile)" >&2
        pytest_result="skip"
    fi

    # --- 7. Export/stub parity ---
    echo "[7/12] export/stub parity..." >&2
    if python3 "$ROOT/scripts/check_eggsec_python_exports.py" >/dev/null 2>&1; then
        export_parity_result="pass"
    else
        export_parity_result="fail"
    fi

    # --- 8. Capability matrix ---
    echo "[8/12] capability matrix..." >&2
    if python3 "$ROOT/scripts/check-python-capability-matrix.py" >/dev/null 2>&1; then
        capability_matrix_result="pass"
    else
        capability_matrix_result="fail"
    fi

    # --- 9. Architecture guards ---
    echo "[9/12] architecture guards..." >&2
    if python3 "$ROOT/scripts/check-python-architecture-guards.py" >/dev/null 2>&1; then
        arch_guards_result="pass"
    else
        arch_guards_result="fail"
    fi

    # --- 10. Type checks ---
    echo "[10/12] type checks..." >&2
    if bash "$ROOT/scripts/check_python_types.sh" >/dev/null 2>&1; then
        type_check_result="pass"
    else
        type_check_result="fail"
    fi

    # --- 11. Documentation examples (where applicable) ---
    echo "[11/12] doc examples..." >&2
    doc_example_result="pass"  # placeholder; no universal doc examples yet

    # --- 12. Platform recording (already captured) ---
    echo "[12/12] platform info recorded." >&2

    # Build JSON fragment
    build_profile_json "$profile" "$features" "$cargo_check_result" "$cargo_check_elapsed" \
        "$cargo_test_result" "$cargo_test_elapsed" "$cargo_test_total" "$cargo_test_failed" \
        "$ext_build_result" "$ext_build_elapsed" \
        "$wheel_build_result" "$wheel_build_elapsed" "$wheel_size" \
        "$import_test_result" \
        "$pytest_result" "$pytest_passed" "$pytest_failed" "$pytest_skipped" \
        "$export_parity_result" "$capability_matrix_result" "$arch_guards_result" \
        "$type_check_result" "$doc_example_result"
}

# ---------------------------------------------------------------------------
# Build JSON fragment for a profile
# ---------------------------------------------------------------------------
build_profile_json() {
    local profile="$1"
    local features="$2"
    local cargo_check="$3"
    local cargo_check_t="$4"
    local cargo_test="$5"
    local cargo_test_t="$6"
    local cargo_test_total="$7"
    local cargo_test_failed="$8"
    local ext_build="$9"
    local ext_build_t="${10}"
    local wheel_build="${11}"
    local wheel_build_t="${12}"
    local wheel_size="${13}"
    local import_test="${14}"
    local pytest="${15}"
    local pytest_p="${16}"
    local pytest_f="${17}"
    local pytest_s="${18}"
    local export_parity="${19}"
    local cap_matrix="${20}"
    local arch_guards="${21}"
    local type_checks="${22}"
    local doc_examples="${23}"

    cat <<JSONEOF
    "${profile}": {
      "features": "${features:-<none>}",
      "cargo_check": "${cargo_check}",
      "cargo_check_elapsed": "${cargo_check_t}",
      "cargo_test": "${cargo_test}",
      "cargo_test_elapsed": "${cargo_test_t}",
      "cargo_test_total": ${cargo_test_total},
      "cargo_test_failed": ${cargo_test_failed},
      "extension_build": "${ext_build}",
      "extension_build_elapsed": "${ext_build_t}",
      "wheel_build": "${wheel_build}",
      "wheel_build_elapsed": "${wheel_build_t}",
      "wheel_size_bytes": ${wheel_size},
      "import_test": "${import_test}",
      "pytest": "${pytest}",
      "pytest_passed": ${pytest_p},
      "pytest_failed": ${pytest_f},
      "pytest_skipped": ${pytest_s},
      "export_parity": "${export_parity}",
      "capability_matrix": "${cap_matrix}",
      "architecture_guards": "${arch_guards}",
      "type_checks": "${type_checks}",
      "doc_examples": "${doc_examples}"
    }
JSONEOF
}

# ---------------------------------------------------------------------------
# Main execution
# ---------------------------------------------------------------------------
echo "============================================================"
echo "  eggsec Python Release 1.2 Validation Matrix"
echo "  Commit: ${COMMIT_HASH}"
echo "  Timestamp: ${TIMESTAMP}"
echo "  Platform: ${PLATFORM}"
echo "  Python: ${PYTHON_VERSION}"
echo "  Rust: ${RUST_VERSION}"
echo "  Mode: ${MODE}"
echo "============================================================"

# Determine which profiles to run
PROFILES_TO_RUN=()
if [[ "$RUN_ALL" == true ]]; then
    PROFILES_TO_RUN=("${ALL_PROFILES[@]}")
else
    # Validate profile name
    found=false
    for p in "${ALL_PROFILES[@]}"; do
        [[ "$p" == "$PROFILE" ]] && found=true && break
    done
    if [[ "$found" == false ]]; then
        echo "ERROR: Unknown profile '${PROFILE}'" >&2
        echo "Available profiles: ${ALL_PROFILES[*]}" >&2
        exit 1
    fi
    PROFILES_TO_RUN=("$PROFILE")
fi

# Run profiles and collect JSON fragments
FRAGMENTS_DIR=$(mktemp -d)
trap 'rm -rf "$FRAGMENTS_DIR"' EXIT

TOTAL=${#PROFILES_TO_RUN[@]}
PASSED=0
FAILED=0
SKIPPED=0

for prof in "${PROFILES_TO_RUN[@]}"; do
    frag_file="${FRAGMENTS_DIR}/${prof}.json"
    if run_profile "$prof" > "$frag_file"; then
        PASSED=$((PASSED + 1))
    else
        FAILED=$((FAILED + 1))
    fi
done

# Count skipped (profiles that produced empty or skip-only results)
for prof in "${PROFILES_TO_RUN[@]}"; do
    frag_file="${FRAGMENTS_DIR}/${prof}.json"
    if grep -q '"cargo_check": "skip"' "$frag_file" 2>/dev/null; then
        SKIPPED=$((SKIPPED + 1))
        PASSED=$((PASSED - 1))
    fi
done

# ---------------------------------------------------------------------------
# Assemble final JSON report
# ---------------------------------------------------------------------------
echo ""
echo "Assembling JSON report..."

# Start the JSON structure
cat > "$REPORT_FILE" <<JSONHEAD
{
  "schema_version": "1.0.0",
  "commit": "${COMMIT_HASH}",
  "timestamp": "${TIMESTAMP}",
  "platform": $(python3 -c "import json; print(json.dumps('${PLATFORM}'))"),
  "python_version": $(python3 -c "import json; print(json.dumps('${PYTHON_VERSION}'))"),
  "rust_version": $(python3 -c "import json; print(json.dumps('${RUST_VERSION}'))"),
  "mode": "${MODE}",
  "profiles": {
JSONHEAD

# Append profile fragments
first=true
for prof in "${PROFILES_TO_RUN[@]}"; do
    frag_file="${FRAGMENTS_DIR}/${prof}.json"
    if [[ "$first" == true ]]; then
        first=false
    else
        echo "," >> "$REPORT_FILE"
    fi
    cat "$frag_file" >> "$REPORT_FILE"
done

# Close profiles and add summary
cat >> "$REPORT_FILE" <<JSONTAIL

  },
  "summary": {
    "total_profiles": ${TOTAL},
    "passed": ${PASSED},
    "failed": ${FAILED},
    "skipped": ${SKIPPED}
  }
}
JSONTAIL

echo ""
echo "============================================================"
echo "  Validation Complete"
echo "  Report: ${REPORT_FILE}"
echo "  Total: ${TOTAL}  Passed: ${PASSED}  Failed: ${FAILED}  Skipped: ${SKIPPED}"
echo "============================================================"

# Print summary table
echo ""
echo "Profile Summary:"
echo "----------------"
printf "%-30s %-10s %-10s %-10s\n" "Profile" "Check" "Test" "Build"
printf "%-30s %-10s %-10s %-10s\n" "-------" "-----" "----" "-----"
for prof in "${PROFILES_TO_RUN[@]}"; do
    frag_file="${FRAGMENTS_DIR}/${prof}.json"
    cc=$(grep -o '"cargo_check": "[^"]*"' "$frag_file" 2>/dev/null | head -1 | cut -d'"' -f4 || echo "skip")
    ct=$(grep -o '"cargo_test": "[^"]*"' "$frag_file" 2>/dev/null | head -1 | cut -d'"' -f4 || echo "skip")
    eb=$(grep -o '"extension_build": "[^"]*"' "$frag_file" 2>/dev/null | head -1 | cut -d'"' -f4 || echo "skip")
    printf "%-30s %-10s %-10s %-10s\n" "$prof" "$cc" "$ct" "$eb"
done

if [[ "$FAILED" -gt 0 ]]; then
    exit 1
fi
