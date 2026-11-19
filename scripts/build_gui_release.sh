#!/usr/bin/env bash
#
# Build the desktop GUI's release binary and its build-metadata sidecar.
#
# WHY THIS EXISTS. Milestone 007 qualifies the GUI per platform and hands the
# binary to a separately-managed distribution workstream. A handoff is only
# useful if the receiver can tell whether two binaries are "the same build",
# and that question is answered by facts a prose release note drifts on: the
# commit, the tree state, the toolchain, the target triple, the resolved
# feature set, and the lockfile hash. This script records all of them beside
# the artefact, so a hash mismatch is diagnosable instead of mysterious.
#
# WHAT THIS DELIBERATELY DOES NOT DO. It does not sign, notarize, bundle, or
# publish, and it has no npm step. Those belong to the distribution workstream
# (`ADR-0007`), and the Tauri bundler stays inactive
# (`bundle.active: false`, crates/eggsec-gui/tauri.conf.json:26-28) so that
# building the GUI never emits an installer as a side effect. The script
# asserts that after the build.
#
# USAGE.
#   scripts/build_gui_release.sh [--out DIR] [--target TRIPLE] [--allow-dirty]
#
#   --out DIR        Output directory (default: dist/, which is git-ignored).
#   --target TRIPLE  Target triple to build for (default: the host triple).
#   --allow-dirty    Build from a dirty working tree. Without it the script
#                    refuses, because a release-grade binary must come from a
#                    pinned commit. The tree state is recorded either way.
#
# EXIT STATUS. 0 on success; non-zero when the tree is dirty and
# --allow-dirty was not passed, when a prerequisite is missing, or when one of
# the artefact-level assertions fails.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

OUT_DIR="dist"
ALLOW_DIRTY=0
TARGET=""

usage() {
  cat <<'EOF'
Usage: scripts/build_gui_release.sh [--out DIR] [--target TRIPLE] [--allow-dirty]

  --out DIR        Output directory (default: dist/, git-ignored).
  --target TRIPLE  Target triple to build for (default: the host triple).
  --allow-dirty    Permit a dirty working tree (the tree state is recorded).
EOF
}

while [ $# -gt 0 ]; do
  case "$1" in
    --out)
      [ $# -ge 2 ] || { echo "error: --out requires a directory" >&2; exit 2; }
      OUT_DIR="$2"
      shift 2
      ;;
    --out=*)
      OUT_DIR="${1#*=}"
      shift
      ;;
    --target)
      [ $# -ge 2 ] || { echo "error: --target requires a triple" >&2; exit 2; }
      TARGET="$2"
      shift 2
      ;;
    --target=*)
      TARGET="${1#*=}"
      shift
      ;;
    --allow-dirty)
      ALLOW_DIRTY=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

fail() { echo "FAIL: $*" >&2; exit 1; }

cd "$REPO_ROOT"

for tool in cargo rustc git python3; do
  command -v "$tool" >/dev/null 2>&1 || fail "$tool is required to build the desktop GUI."
done

# ── 1. Tree state ──────────────────────────────────────────────────────────
# A release binary must come from a pinned commit. The refusal is the point:
# `--allow-dirty` exists so an operator can produce a diagnostic build, and
# the flag they passed is recorded beside the artefact.
COMMIT_SHA="$(git rev-parse HEAD)"
TREE_STATE="clean"
if [ -n "$(git status --porcelain)" ]; then
  TREE_STATE="dirty"
fi
if [ "$TREE_STATE" = "dirty" ] && [ "$ALLOW_DIRTY" -eq 0 ]; then
  echo "FAIL: the working tree is dirty; a release build must come from a pinned commit." >&2
  echo "      Commit the change, or pass --allow-dirty to record a diagnostic build." >&2
  echo "      The tree state is recorded in the build metadata either way." >&2
  exit 1
fi
echo "Commit:  $COMMIT_SHA"
echo "Tree:    $TREE_STATE"

# ── 2. Toolchain and target ────────────────────────────────────────────────
RUSTC_VERSION="$(rustc -V)"
RUSTC_VERSION_FULL="$(rustc -vV)"
CARGO_VERSION="$(cargo -V)"
HOST_TRIPLE="$(printf '%s\n' "$RUSTC_VERSION_FULL" | awk '/^host:/ {print $2}')"
TARGET_TRIPLE="${TARGET:-$HOST_TRIPLE}"

# The resolved feature set is read from Cargo's own resolution rather than
# transcribed: a hand-written feature list is the kind of table that drifts.
CARGO_FEATURES_REQUESTED="shell"
CARGO_FEATURES_RESOLVED="$(python3 - "$CARGO_FEATURES_REQUESTED" <<'PYEOF'
import json, subprocess, sys

requested = sys.argv[1]
try:
    raw = subprocess.run(
        [
            "cargo", "metadata", "--format-version", "1",
            "--features", f"eggsec-gui/{requested}",
        ],
        capture_output=True,
        text=True,
        check=True,
    ).stdout
except subprocess.CalledProcessError as exc:
    print(f"cargo metadata failed: {exc.stderr.strip()}", file=sys.stderr)
    raise SystemExit(1)

data = json.loads(raw)
names = {pkg["id"]: pkg["name"] for pkg in (data.get("packages") or [])}
resolve = data.get("resolve") or {}
for node in resolve.get("nodes") or []:
    if names.get(node.get("id")) == "eggsec-gui":
        print(",".join(sorted(node.get("features") or [])))
        break
else:
    print("")
PYEOF
)"

# ── 3. Build ───────────────────────────────────────────────────────────────
# The invocation is pinned: `--locked` refuses a lockfile that would change,
# so the recorded lockfile hash describes the graph that was actually built.
BUILD_ARGS=(build --release -p eggsec-gui --features shell --locked)
if [ -n "$TARGET" ]; then
  BUILD_ARGS+=(--target "$TARGET")
fi
echo ""
echo "Building: cargo ${BUILD_ARGS[*]}"
cargo "${BUILD_ARGS[@]}"

if [ -n "$TARGET" ]; then
  BUILT_BINARY="target/$TARGET_TRIPLE/release/eggsec-gui"
else
  BUILT_BINARY="target/release/eggsec-gui"
fi
[ -f "$BUILT_BINARY" ] || fail "cargo reported success but $BUILT_BINARY is missing."

# ── 4. Publish into the output directory ───────────────────────────────────
mkdir -p "$OUT_DIR"
OUT_BINARY="$OUT_DIR/eggsec-gui"
cp "$BUILT_BINARY" "$OUT_BINARY"
chmod +x "$OUT_BINARY"

sha256_hex() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | cut -d' ' -f1
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | cut -d' ' -f1
  else
    fail "no sha256sum or shasum on this host; the build metadata needs a SHA-256."
  fi
}

BINARY_SHA256="$(sha256_hex "$OUT_BINARY")"
BINARY_SIZE="$(python3 -c "import os,sys; print(os.path.getsize(sys.argv[1]))" "$OUT_BINARY")"
LOCK_SHA256="$(sha256_hex "Cargo.lock")"

(cd "$OUT_DIR" && sha256sum eggsec-gui 2>/dev/null || shasum -a 256 eggsec-gui) > "$OUT_DIR/SHA256SUMS"

# ── 5. Build metadata sidecar ──────────────────────────────────────────────
# The credential check records the NAMES of credential-shaped environment
# variables that were set during the build, never their values: the fact that
# a variable existed is the audit fact, and its value is the secret.
CREDENTIAL_ENV_VARS="$(python3 - <<'PYEOF'
import os, re

pattern = re.compile(r"(token|secret|password|passwd|api_?key|private_?key|credential)", re.I)
print(",".join(sorted(name for name in os.environ if pattern.search(name))))
PYEOF
)"

METADATA_PATH="$OUT_DIR/eggsec-gui.build.json"
python3 - "$METADATA_PATH" <<PYEOF
import json, sys

metadata = {
    "schema": "eggsec-gui-build-metadata/v1",
    "binary": "eggsec-gui",
    "binary_path": sys.argv[1].rsplit("/", 1)[0] + "/eggsec-gui",
    "binary_sha256": "$BINARY_SHA256",
    "binary_size_bytes": int("$BINARY_SIZE"),
    "commit_sha": "$COMMIT_SHA",
    "tree_state": "$TREE_STATE",
    "dirty": "$TREE_STATE" == "dirty",
    "allow_dirty": "$ALLOW_DIRTY" == "1",
    "rustc_version": """$RUSTC_VERSION""".strip(),
    "rustc_version_verbose": """$RUSTC_VERSION_FULL""".strip(),
    "cargo_version": """$CARGO_VERSION""".strip(),
    "target_triple": "$TARGET_TRIPLE",
    "cargo_features_requested": "$CARGO_FEATURES_REQUESTED",
    "cargo_features_resolved": [
        feature for feature in "$CARGO_FEATURES_RESOLVED".split(",") if feature
    ],
    "cargo_lock_sha256": "$LOCK_SHA256",
    "built_at_utc": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
    "bundle_active": False,
    "signed": False,
    "notarized": False,
    "bundled": False,
    "published": False,
    "credential_env_vars_present": [
        name for name in "$CREDENTIAL_ENV_VARS".split(",") if name
    ],
}

with open(sys.argv[1], "w", encoding="utf-8") as handle:
    json.dump(metadata, handle, indent=2, sort_keys=True)
    handle.write("\n")
PYEOF

echo ""
echo "Artefact:  $OUT_BINARY"
echo "Metadata:  $METADATA_PATH"
echo "SHA-256:   $BINARY_SHA256"
echo "Size:      $BINARY_SIZE bytes"

# ── 6. Artefact-level assertions ───────────────────────────────────────────
# These are the milestone's own boundary, checked on the artefact rather than
# on the source tree: the source-level statements are already guarded, but a
# guard cannot see what the build actually emitted.

# A. No installer artefact. The bundler is off; a build that produced a .deb,
#    .rpm, .AppImage, .dmg, .app, .msi, or .exe would mean the boundary moved.
INSTALLERS="$(find "$OUT_DIR" -maxdepth 2 \
  \( -name '*.deb' -o -name '*.rpm' -o -name '*.AppImage' -o -name '*.dmg' \
     -o -name '*.app' -o -name '*.msi' -o -name '*.exe' \) -print)"
if [ -n "$INSTALLERS" ]; then
  echo "FAIL: the build emitted installer artefacts; the bundler must stay inactive." >&2
  printf '%s\n' "$INSTALLERS" >&2
  exit 1
fi

# B. No credential-shaped material in the shipped metadata.
if grep -Eqi '(BEGIN [A-Z ]*PRIVATE KEY|AWS_SECRET_ACCESS_KEY|AWS_ACCESS_KEY_ID|xox[baprs]-|ghp_[A-Za-z0-9]{20,}|AKIA[0-9A-Z]{16})' "$METADATA_PATH"; then
  echo "FAIL: the build metadata contains credential-shaped material." >&2
  exit 1
fi

# C. No credential-shaped material in the shipped binary. The markers are
#    deliberately high-signal: a generic "password" search over a release
#    binary matches ordinary strings and would make the check useless.
if grep -aEq '(BEGIN [A-Z ]*PRIVATE KEY|AWS_SECRET_ACCESS_KEY|AWS_ACCESS_KEY_ID|xox[baprs]-|ghp_[A-Za-z0-9]{20,}|AKIA[0-9A-Z]{16})' "$OUT_BINARY"; then
  echo "FAIL: the built binary contains credential-shaped material." >&2
  exit 1
fi

# D. The shipped binary's embedded CSP still admits no remote origin beyond
#    the Tauri IPC origin. The CSP is compiled into the binary from
#    crates/eggsec-gui/tauri.conf.json, so the source-level guard (check 158)
#    has an artefact-level counterpart here. The check is exact-match rather
#    than a loose scan: a substring hunt over a release binary matches
#    unrelated strings and turns a real gate into noise.
CSP_VALUE="$(python3 - <<'PYEOF'
import json, pathlib

data = json.loads(pathlib.Path("crates/eggsec-gui/tauri.conf.json").read_text())
print(((data.get("app") or {}).get("security") or {}).get("csp", ""))
PYEOF
)"
if [ -z "$CSP_VALUE" ]; then
  echo "FAIL: crates/eggsec-gui/tauri.conf.json defines no CSP; the artefact check cannot run." >&2
  exit 1
fi
REMOTE_ORIGINS="$(printf '%s' "$CSP_VALUE" | grep -Eo 'https?://[A-Za-z0-9._-]+' | grep -v '^http://ipc\.localhost$' || true)"
if [ -n "$REMOTE_ORIGINS" ]; then
  echo "FAIL: the CSP compiled into the binary admits a remote origin:" >&2
  printf '%s\n' "$REMOTE_ORIGINS" >&2
  exit 1
fi
if ! grep -aFq "$CSP_VALUE" "$OUT_BINARY"; then
  echo "FAIL: the built binary does not carry the checked-in CSP verbatim." >&2
  exit 1
fi

echo ""
echo "Checks:  no installer artefact, no credential-shaped material, CSP admits no remote origin."
echo "Nothing was signed, notarized, bundled, or published."
