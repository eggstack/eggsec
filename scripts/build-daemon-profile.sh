#!/usr/bin/env bash
# Reproducible build recipe for the daemon's GUI-serving feature profile.
#
# The profile is a BUILD-side contract: the cargo feature `service` on
# `crates/eggsec-daemon` selects the engine features a GUI-serving daemon
# contains. The list itself is owned by the engine
# (`eggsec::config::daemon_profile::DAEMON_SERVICE_PROFILE`), so this script
# never re-declares it: it reads the list from there and refuses to build when
# the daemon's manifest, the documented profile and the compiled binary would
# disagree.
#
# What the script does:
#   - refuses a dirty working tree unless --allow-dirty is passed, and records
#     the dirty flag either way;
#   - builds exactly
#     `cargo build --release --locked -p eggsec-daemon --features <profile> --target <triple>`
#     (cargo's machine-readable message format is added so the resolved engine
#     feature set can be read back; it does not change the compilation);
#   - writes a metadata sidecar next to the binary: commit SHA, dirty flag,
#     `rustc -Vv`, the resolved toolchain version, the MSRV floor, the target
#     triple, the resolved engine feature set, the `Cargo.lock` hash, the
#     binary size and its SHA-256;
#   - runs an ADVISORY prerequisite probe that reports which of the profile's
#     runtime domains are live-capable on this host.
#
# What the script never does: no signing, no bundling, no packaging, no
# installer, no updater, no npm step. It produces a binary and a record.
#
# The script carries no copy of the daemon's default socket path: the binary
# owns that default and honours `EGGSEC_DAEMON_SOCKET` itself. The printed
# hint names the operator's override when there is one, and otherwise lets the
# binary apply its own default.
#
# The probe calls the engine's OWN platform matrix
# (`crates/eggsec/src/platform/prereqs.rs` -> `PlatformReport::collect` ->
# `report_for` per domain) through `eggsec doctor`'s JSON mode, so it reports
# the engine's facts instead of re-implementing them. It is advisory: it
# reports, and it never blocks the build.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

PROFILE="service"
TARGET=""
OUT_DIR=""
ALLOW_DIRTY=0
MSRV_FLOOR="1.89"
VALID_PROFILES="service"
# The script carries no copy of the daemon's default socket path. The binary
# owns that default (`eggsec_runtime::DEFAULT_DAEMON_SOCKET_PATH`) and honours
# `EGGSEC_DAEMON_SOCKET` itself, so the hint below only has to name the
# operator's override when there is one.
SOCKET_PATH="${EGGSEC_DAEMON_SOCKET:-}"

ENGINE_MANIFEST="crates/eggsec/Cargo.toml"
DAEMON_MANIFEST="crates/eggsec-daemon/Cargo.toml"
CLI_MANIFEST="crates/eggsec-cli/Cargo.toml"
PROFILE_SOURCE="crates/eggsec/src/config/daemon_profile.rs"
PREREQ_SOURCE="crates/eggsec/src/platform/prereqs.rs"

die() {
  echo "error: $*" >&2
  exit 1
}

note() {
  echo "note: $*"
}

usage() {
  cat <<'EOF'
Usage: bash scripts/build-daemon-profile.sh [options]

Build the daemon's GUI-serving feature profile reproducibly.

Options:
  --profile <name>   Feature profile to build (only 'service' today).
                      Default: service
  --target <triple>  Target triple to build for.
                      Default: the host triple
  --out <dir>        Output directory for the binary and its metadata sidecar.
                      Default: $TMPDIR/eggsec-daemon-profile/<profile>
  --allow-dirty      Build even though the working tree has uncommitted
                      changes. The dirty flag is recorded either way.
  -h, --help         Show this help.

The build is pinned to:
  cargo build --release --locked -p eggsec-daemon --features <profile> --target <triple>

The script writes the binary plus a metadata sidecar and prints the runtime
invocation to use next. It does not sign, bundle, package, install or update
anything.
EOF
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --profile)
      [ "$#" -ge 2 ] || die "--profile needs a value"
      PROFILE="$2"
      shift 2
      ;;
    --target)
      [ "$#" -ge 2 ] || die "--target needs a value"
      TARGET="$2"
      shift 2
      ;;
    --out)
      [ "$#" -ge 2 ] || die "--out needs a value"
      OUT_DIR="$2"
      shift 2
      ;;
    --allow-dirty)
      ALLOW_DIRTY=1
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown argument '$1'" >&2
      usage >&2
      exit 2
      ;;
  esac
done

case " $VALID_PROFILES " in
  *" $PROFILE "*) ;;
  *) die "unknown profile '$PROFILE' (the only profile today is 'service')" ;;
esac

command -v cargo >/dev/null 2>&1 || die "cargo is required"
command -v python3 >/dev/null 2>&1 || die "python3 is required (it reads the engine's profile list and the manifests)"
command -v git >/dev/null 2>&1 || die "git is required (the commit SHA and the dirty flag come from it)"

HOST_TRIPLE="$(rustc -vV | sed -n 's/^host: //p' | head -n 1)"
[ -n "$HOST_TRIPLE" ] || die "could not determine the host triple from 'rustc -vV'"
if [ -z "$TARGET" ]; then
  TARGET="$HOST_TRIPLE"
fi
if [ -z "$OUT_DIR" ]; then
  OUT_DIR="${TMPDIR:-/tmp}/eggsec-daemon-profile/$PROFILE"
fi

# ── The tree state the recipe is recorded against ──────────────────────────
COMMIT="$(git rev-parse HEAD)"
if [ -n "$(git status --porcelain)" ]; then
  DIRTY="true"
else
  DIRTY="false"
fi
if [ "$DIRTY" = "true" ] && [ "$ALLOW_DIRTY" -eq 0 ]; then
  die "the working tree has uncommitted changes; commit them or pass --allow-dirty to build anyway (the dirty flag is recorded either way)"
fi
if [ "$DIRTY" = "true" ]; then
  note "building from a dirty working tree (recorded as dirty=true in the sidecar)."
fi

# ── The toolchain, and the MSRV floor ──────────────────────────────────────
TOOLCHAIN="$(rustc -Vv)"
TOOLCHAIN_VERSION="$(printf '%s\n' "$TOOLCHAIN" | sed -n 's/^rustc \([0-9][0-9.]*\).*/\1/p' | head -n 1)"
[ -n "$TOOLCHAIN_VERSION" ] || die "could not parse the rustc version from 'rustc -Vv'"
FLOOR_MAJOR="${MSRV_FLOOR%%.*}"
FLOOR_MINOR="${MSRV_FLOOR##*.}"
VER_MAJOR="${TOOLCHAIN_VERSION%%.*}"
VER_MINOR="$(printf '%s\n' "$TOOLCHAIN_VERSION" | cut -d. -f2)"
VER_MINOR="${VER_MINOR:-0}"
if [ "$VER_MAJOR" -lt "$FLOOR_MAJOR" ] ||
  { [ "$VER_MAJOR" -eq "$FLOOR_MAJOR" ] && [ "$VER_MINOR" -lt "$FLOOR_MINOR" ]; }; then
  die "the resolved toolchain (rustc $TOOLCHAIN_VERSION) is older than the workspace MSRV floor ($MSRV_FLOOR)"
fi

# ── The profile list, read from its single source of truth ─────────────────
# The engine owns the list; the daemon's manifest must forward exactly it, and
# the compiled binary must end up with exactly it. Nothing here re-declares a
# feature name.
PROFILE_INFO="$(mktemp)"
BUILD_LOG="$(mktemp)"
BUILD_ERR="$(mktemp)"
RESOLVED_ENGINE_FEATURES_FILE="$(mktemp)"
trap 'rm -f "$PROFILE_INFO" "$BUILD_LOG" "$BUILD_ERR" "$RESOLVED_ENGINE_FEATURES_FILE"' EXIT

python3 - "$PROFILE_INFO" "$PROFILE" "$PROFILE_SOURCE" "$DAEMON_MANIFEST" \
  "$ENGINE_MANIFEST" "$PREREQ_SOURCE" "$CLI_MANIFEST" <<'PYEOF'
import re
import sys
import tomllib
from pathlib import Path

(
    out_path,
    profile_name,
    profile_source,
    daemon_manifest,
    engine_manifest,
    prereq_source,
    cli_manifest,
) = sys.argv[1:8]


def read(path: str) -> str:
    return Path(path).read_text(encoding="utf-8")


def profile_features() -> list[str]:
    """The engine's DAEMON_SERVICE_PROFILE list, read from its declaration."""
    source = read(profile_source)
    match = re.search(
        r"pub static DAEMON_SERVICE_PROFILE[^=]*=\s*&\[(.*?)\];", source, re.S
    )
    if match is None:
        raise SystemExit(f"could not find DAEMON_SERVICE_PROFILE in {profile_source}")
    return re.findall(r'"([^"]+)"', match.group(1))


def declared_engine_features() -> set[str]:
    with open(engine_manifest, "rb") as handle:
        return set(tomllib.load(handle).get("features", {}))


def manifest_features(feature: str) -> list[str]:
    with open(daemon_manifest, "rb") as handle:
        features = tomllib.load(handle).get("features", {})
    if feature not in features:
        raise SystemExit(f"{daemon_manifest} declares no '{feature}' feature")
    return [
        entry.split("/", 1)[1]
        for entry in features[feature]
        if isinstance(entry, str) and entry.startswith("eggsec/")
    ]


def domain_ids() -> list[str]:
    """The engine's platform domain ids, read from `all_domain_ids()`."""
    source = read(prereq_source)
    match = re.search(
        r"pub fn all_domain_ids\(\)\s*->\s*Vec<String>\s*\{(.*?)\n\}", source, re.S
    )
    if match is None:
        raise SystemExit(f"could not find all_domain_ids() in {prereq_source}")
    return re.findall(r'"([^"]+)"', match.group(1))


def probe_cli_features(profile: list[str], domains: list[str]) -> list[str]:
    """The CLI features that forward the profile's platform domains.

    The engine names each platform domain after the cargo feature that
    compiles it, so a domain belongs to the profile when its name is in the
    profile list. The CLI feature forwarding `eggsec/<domain>` is the one the
    probe needs, so the probe's matrix reports the same compiled state the
    daemon will have.
    """
    with open(cli_manifest, "rb") as handle:
        cli_features = tomllib.load(handle).get("features", {})
    forwarders: dict[str, list[str]] = {}
    for name, entries in cli_features.items():
        for entry in entries:
            if isinstance(entry, str) and entry.startswith("eggsec/"):
                forwarders.setdefault(entry.split("/", 1)[1], []).append(name)

    profile_set = set(profile)
    wanted: set[str] = set()
    for domain in domains:
        if domain not in profile_set:
            continue
        wanted.update(forwarders.get(domain, []))
    return sorted(wanted)


profile = profile_features()
declared = declared_engine_features()
manifest = manifest_features(profile_name)

rows = {
    "profile_features": " ".join(sorted(profile)),
    "manifest_features": " ".join(sorted(manifest)),
    "declared_engine_features": " ".join(sorted(declared)),
    "probe_cli_features": ",".join(probe_cli_features(profile, domain_ids())),
}

with open(out_path, "w", encoding="utf-8") as handle:
    for key, value in rows.items():
        handle.write(f"{key}\t{value}\n")
PYEOF

PROFILE_FEATURES=""
MANIFEST_FEATURES=""
DECLARED_ENGINE_FEATURES=""
PROBE_CLI_FEATURES=""
while IFS=$'\t' read -r key value; do
  case "$key" in
    profile_features) PROFILE_FEATURES="$value" ;;
    manifest_features) MANIFEST_FEATURES="$value" ;;
    declared_engine_features) DECLARED_ENGINE_FEATURES="$value" ;;
    probe_cli_features) PROBE_CLI_FEATURES="$value" ;;
  esac
done <"$PROFILE_INFO"

[ -n "$PROFILE_FEATURES" ] || die "could not read the engine's profile list from $PROFILE_SOURCE"
if [ "$PROFILE_FEATURES" != "$MANIFEST_FEATURES" ]; then
  die "the daemon's '$PROFILE' feature does not match DAEMON_SERVICE_PROFILE; manifest=[$MANIFEST_FEATURES] profile=[$PROFILE_FEATURES]"
fi

# ── The advisory prerequisite probe ────────────────────────────────────────
# It reports which of the profile's runtime domains are live-capable on THIS
# host, using the engine's own platform matrix. It never blocks the build: if
# it cannot run, the build continues and the note says so.
probe_prerequisites() {
  if [ -z "$PROBE_CLI_FEATURES" ]; then
    note "no profile domain maps to a CLI feature; skipping the prerequisite probe."
    return 0
  fi

  local probe_json probe_err
  probe_json="$(mktemp)"
  probe_err="$(mktemp)"

  local -a probe_args=(--release --locked)
  if [ "$TARGET" = "$HOST_TRIPLE" ]; then
    # Same target directory as the build: the probe's compilation is reused.
    probe_args+=(--target "$TARGET")
  fi

  echo "--- prerequisite probe (advisory): the engine's own platform matrix ---"
  if ! EGGSEC_DOCTOR_JSON=1 cargo run -q "${probe_args[@]}" -p eggsec-cli \
    --no-default-features --features "$PROBE_CLI_FEATURES" -- doctor \
    >"$probe_json" 2>"$probe_err"; then
    note "the prerequisite probe could not run; it is advisory, so the build continues."
    tail -n 10 "$probe_err" >&2 || true
    rm -f "$probe_json" "$probe_err"
    return 0
  fi

  python3 - "$probe_json" "$PROFILE_FEATURES" <<'PYEOF'
import json
import sys

report_path, profile_features = sys.argv[1], sys.argv[2]
profile = set(profile_features.split())

with open(report_path, encoding="utf-8") as handle:
    report = json.load(handle)

print(
    f"host: {report.get('os')}/{report.get('arch')} root={report.get('is_root')} "
    f"cap_net_raw={report.get('cap_net_raw')} cap_net_admin={report.get('cap_net_admin')}"
)
for domain in report.get("domains", []):
    features = domain.get("features", [])
    if not set(features) & profile:
        # A domain outside the profile (e.g. `mobile-dynamic`) is not this
        # daemon's business.
        continue
    live = "yes" if domain.get("live_supported") else "no"
    print(f"  {domain.get('domain')}: live-capable={live} (features: {', '.join(features)})")
    for prerequisite in domain.get("prerequisites", []):
        # `PrereqStatus` is a tagged enum, so the status arrives as an object.
        status = prerequisite.get("status")
        if isinstance(status, dict):
            status = status.get("status")
        if status == "available":
            continue
        print(
            f"      {status}: {prerequisite.get('label')} "
            f"— {prerequisite.get('detail')}"
        )
        if prerequisite.get("fix"):
            print(f"        fix: {prerequisite.get('fix')}")
PYEOF
  rm -f "$probe_json" "$probe_err"
}

probe_prerequisites

# ── The build ──────────────────────────────────────────────────────────────
CARGO_COMMAND="cargo build --release --locked -p eggsec-daemon --features $PROFILE --target $TARGET"
echo "--- build: $CARGO_COMMAND ---"
if ! cargo build --release --locked -p eggsec-daemon --features "$PROFILE" \
  --target "$TARGET" --message-format=json >"$BUILD_LOG" 2>"$BUILD_ERR"; then
  echo "error: the profile build failed" >&2
  tail -n 40 "$BUILD_ERR" >&2 || true
  exit 1
fi

# The engine feature set the binary was actually compiled with, read back from
# cargo's own artifact record.
python3 - "$RESOLVED_ENGINE_FEATURES_FILE" "$BUILD_LOG" "$DECLARED_ENGINE_FEATURES" <<'PYEOF'
import json
import sys

out_path, build_log, declared = sys.argv[1], sys.argv[2], set(sys.argv[3].split())
resolved: set[str] = set()
seen = False

with open(build_log, encoding="utf-8", errors="replace") as handle:
    for line in handle:
        line = line.strip()
        if not line.startswith("{"):
            continue
        try:
            message = json.loads(line)
        except json.JSONDecodeError:
            continue
        if message.get("reason") != "compiler-artifact":
            continue
        package = message.get("package_id", "").split("#", 1)[0].rstrip("/")
        if not package.endswith("/crates/eggsec"):
            continue
        seen = True
        for feature in message.get("features", []) or []:
            resolved.add(feature)

# The engine's "default" feature is empty, and the optional-dependency
# features cargo reports next to them are not engine features; what matters is
# the set of the engine's own declared features the binary was built with.
declared_resolved = sorted(resolved & declared - {"default"})

with open(out_path, "w", encoding="utf-8") as handle:
    handle.write(" ".join(declared_resolved) if seen else "")
PYEOF
RESOLVED_ENGINE_FEATURES="$(cat "$RESOLVED_ENGINE_FEATURES_FILE")"

if [ -z "$RESOLVED_ENGINE_FEATURES" ]; then
  note "cargo did not report the engine's resolved feature set; recording the profile list."
  RESOLVED_ENGINE_FEATURES="$PROFILE_FEATURES"
elif [ "$RESOLVED_ENGINE_FEATURES" != "$PROFILE_FEATURES" ]; then
  die "the compiled engine feature set does not match DAEMON_SERVICE_PROFILE; compiled=[$RESOLVED_ENGINE_FEATURES] profile=[$PROFILE_FEATURES]"
fi

# ── The output: the binary plus its metadata sidecar ──────────────────────
BINARY_SRC="target/$TARGET/release/eggsec-daemon"
[ -f "$BINARY_SRC" ] || die "the build reported success but $BINARY_SRC is missing"

mkdir -p "$OUT_DIR"
case "$OUT_DIR" in
  "$REPO_ROOT"/*)
    note "the output directory is inside the source tree ($OUT_DIR)."
    ;;
esac

BINARY_PATH="$OUT_DIR/eggsec-daemon"
METADATA_PATH="$OUT_DIR/eggsec-daemon.metadata.json"
cp "$BINARY_SRC" "$BINARY_PATH"
chmod +x "$BINARY_PATH"

sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  else
    die "neither sha256sum nor shasum is available to hash '$1'"
  fi
}

BINARY_SIZE="$(wc -c <"$BINARY_PATH" | tr -d ' ')"
BINARY_SHA256="$(sha256_of "$BINARY_PATH")"
LOCK_SHA256="$(sha256_of "Cargo.lock")"

python3 - "$METADATA_PATH" "$PROFILE" "$COMMIT" "$DIRTY" "$TOOLCHAIN" \
  "$TOOLCHAIN_VERSION" "$MSRV_FLOOR" "$TARGET" "$RESOLVED_ENGINE_FEATURES" \
  "$LOCK_SHA256" "$BINARY_SIZE" "$BINARY_SHA256" "$CARGO_COMMAND" <<'PYEOF'
import json
import sys

(
    path,
    profile,
    commit,
    dirty,
    toolchain,
    toolchain_version,
    msrv,
    target,
    resolved_features,
    lock_sha256,
    binary_size,
    binary_sha256,
    cargo_command,
) = sys.argv[1:14]

metadata = {
    "schema": "eggsec-daemon-profile-metadata/1",
    "profile": profile,
    "package": "eggsec-daemon",
    "cargo_command": cargo_command,
    "commit": commit,
    "dirty": dirty == "true",
    "toolchain": toolchain,
    "toolchain_version": toolchain_version,
    "msrv": msrv,
    "target": target,
    "resolved_engine_features": sorted(resolved_features.split()),
    "resolved_engine_features_source": "eggsec::config::daemon_profile::DAEMON_SERVICE_PROFILE",
    "cargo_lock_sha256": lock_sha256,
    "binary": "eggsec-daemon",
    "binary_size_bytes": int(binary_size),
    "binary_sha256": binary_sha256,
}

with open(path, "w", encoding="utf-8") as handle:
    json.dump(metadata, handle, indent=2, sort_keys=True)
    handle.write("\n")
PYEOF

echo ""
echo "--- built ---"
echo "profile:            $PROFILE"
echo "commit:             $COMMIT (dirty=$DIRTY)"
echo "target:             $TARGET"
echo "toolchain:          rustc $TOOLCHAIN_VERSION (MSRV floor $MSRV_FLOOR)"
echo "engine features:    $RESOLVED_ENGINE_FEATURES"
echo "binary:             $BINARY_PATH ($BINARY_SIZE bytes)"
echo "binary sha256:      $BINARY_SHA256"
echo "Cargo.lock sha256:  $LOCK_SHA256"
echo "metadata sidecar:   $METADATA_PATH"
echo ""
echo "Next step (the runtime flag is unchanged — it is a different mechanism"
echo "from the cargo feature, and it still selects the engine executor):"
if [[ -n "$SOCKET_PATH" ]]; then
  echo "  $BINARY_PATH --full-executor --socket-path $SOCKET_PATH"
else
  echo "  $BINARY_PATH --full-executor"
  echo "  (the daemon applies its own default socket path; set EGGSEC_DAEMON_SOCKET to choose another)"
fi
