#!/usr/bin/env bash
# Validate an eggsec Python wheel in an isolated environment.
# Usage: scripts/validate_wheel.sh <wheel-path>
set -euo pipefail

if [[ $# -ne 1 || ! -f "$1" ]]; then
    echo "Usage: $0 <wheel-file>" >&2
    exit 1
fi

WHEEL=$1
VENV_DIR=$(mktemp -d)
trap 'rm -rf "$VENV_DIR"' EXIT
VENV_PYTHON="$VENV_DIR/bin/python"

echo "=== Eggsec Wheel Validation: $(basename "$WHEEL") ==="
python3 -m venv "$VENV_DIR"
"$VENV_PYTHON" -m pip install --disable-pip-version-check --no-deps "$WHEEL" >/dev/null

# The smoke target is an explicitly scoped loopback fixture; do not carry this
# opt-in into normal package execution.
EGGSEC_ALLOW_LOOPBACK_FIXTURE=1 "$VENV_PYTHON" - <<'PY'
import importlib.resources
import eggsec

assert eggsec.__version__ == "0.1.0"
features = eggsec.features()
assert isinstance(features, dict) and features.get("core") is True
info = eggsec.build_info()
assert isinstance(info, dict) and "version" in info and "package_name" in info
surface = eggsec.api_surface()
assert isinstance(surface, dict) and surface["scan_ports"]["stability"] == "stable"
version_info = eggsec.api_surface_version()
assert isinstance(version_info, dict) and "abi_version" in version_info
assert importlib.resources.files("eggsec").joinpath("py.typed").is_file()
assert set(eggsec.__all__) >= {"Engine", "AsyncEngine", "Scope", "scan_ports"}

scope = eggsec.Scope.allow_hosts(["127.0.0.1"])
result = eggsec.scan_ports("127.0.0.1", [9], scope, timeout_ms=250)
assert result.target == "127.0.0.1"
assert isinstance(result.to_json(), str)
print(f"installed-wheel smoke passed: eggsec {eggsec.__version__}")
PY

echo "STATUS: PASS"
