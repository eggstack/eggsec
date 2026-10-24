#!/usr/bin/env bash
#
# The desktop GUI's platform-qualification procedure.
#
# WHAT THIS IS. `docs/GUI_PLATFORM_QUALIFICATION.md` states which platforms the
# desktop GUI is qualified on. This script is the procedure that produces the
# evidence for those claims: it builds the shell, starts a daemon, runs the
# harness (`crates/eggsec-gui/tests/qualification.rs`), probes for the window,
# and writes a run record that the matrix's cells can cite.
#
# WHAT THIS IS NOT. It is not a window-automation driver. It does not click,
# type, or move a mouse, and it does not judge what a human saw: the window
# half of the qualification is an operator observation. The operator writes it
# down in an attestation file (`--attestation`), and the procedure merges that
# file into the run record so that the human's half is in the record from the
# first write. The script measures what a machine can measure and leaves the
# rest named.
#
# THE RULES. The promotion rules live in the matrix's own rules table
# (`docs/GUI_PLATFORM_QUALIFICATION.md` §6), and `scripts/check_gui_qualification.py`
# enforces them. This script does not carry a second copy of them; it only
# produces the record they read.
#
# HERMETICITY. This script writes only under `--out` and `${TMPDIR:-/tmp}`. It
# never elevates its privileges, never runs as root, and never removes anything
# outside those two roots.
#
# EXIT STATUS. 0 when every column's scripted outcome is `pass`, `skipped`, or
# `not-applicable`; 1 when a column's scripted outcome is `fail`, when the
# record could not be written, or when `--mode ci` could not run the harness
# at all. A skipped column is not a failed column: a named skip is an honest
# outcome, and the matrix's promotion rules refuse to promote a skipped cell.

set -euo pipefail

SCRIPT_NAME="$(basename "$0")"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

# ── Defaults ────────────────────────────────────────────────────────────────
PLATFORM=""
MODE="operator"
SOCKET=""
DAEMON_BIN=""
OUT_DIR="$REPO_ROOT/docs/evidence/gui-qualification"
RUN_DATE=""
SKIP_WINDOW=0
ATTESTATION_PATH=""
RUN_LABEL=""

# The schema of the run record this procedure produces. The validator owns the
# record's shape and writes it; the procedure names the schema it expects and
# checks the record it got back against it. The harness, the validator, and the
# evidence template carry the same string, and the static guard (check 186)
# asserts that the four copies agree.
RECORD_SCHEMA="eggsec-gui-qualification-record/v1"

# ── Output helpers ──────────────────────────────────────────────────────────
log() { printf '%s\n' "$*"; }
warn() { printf '%s\n' "$*" >&2; }
die() {
  printf '%s: %s\n' "$SCRIPT_NAME" "$*" >&2
  exit 1
}

usage() {
  cat <<'USAGE'
Usage: bash scripts/gui_qualification.sh [options]

  --platform <linux|macos>  The platform being qualified. Defaults to the host.
  --mode <operator|ci>      `operator` attaches to a daemon the operator
                            started; `ci` starts its own daemon and fails when
                            it cannot. Defaults to `operator`.
  --socket <path>           The daemon's Unix socket.
  --daemon-bin <path>       The daemon binary to start when one is needed.
  --out <dir>               Where the record and the artefacts are written.
                            Defaults to `docs/evidence/gui-qualification`.
  --date <YYYY-MM-DD>       The record's date. Defaults to today (UTC).
  --run-label <label>       A label for a second run on the same day. The
                            record and its artefacts are named
                            `<platform>-<date>-<label>` instead of
                            `<platform>-<date>`, so a day with two runs keeps
                            two records. Lowercase letters, digits and dashes.
  --attestation <path>      An operator attestation file (json): who
                            observed the behaviours, when, on which platform,
                            and what they saw. It is merged into the run
                            record. Without it the record carries no
                            attestation at all.
  --skip-window             Do not attempt the window probe.
  -h, --help                This message.
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --platform)
      PLATFORM="${2:-}"
      shift 2
      ;;
    --mode)
      MODE="${2:-}"
      shift 2
      ;;
    --socket)
      SOCKET="${2:-}"
      shift 2
      ;;
    --daemon-bin)
      DAEMON_BIN="${2:-}"
      shift 2
      ;;
    --out)
      OUT_DIR="${2:-}"
      shift 2
      ;;
    --date)
      RUN_DATE="${2:-}"
      shift 2
      ;;
    --run-label)
      RUN_LABEL="${2:-}"
      shift 2
      ;;
    --skip-window)
      SKIP_WINDOW=1
      shift
      ;;
    --attestation)
      ATTESTATION_PATH="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      die "unknown option \`$1\` (try --help)"
      ;;
  esac
done

# ── The host ────────────────────────────────────────────────────────────────
host_os() {
  case "$(uname -s)" in
    Linux) printf 'linux\n' ;;
    Darwin) printf 'macos\n' ;;
    *) printf 'unknown\n' ;;
  esac
}

HOST_OS="$(host_os)"
HOST_ARCH="$(uname -m)"
HOST_OS_VERSION="$(uname -r)"

if [[ -z "$PLATFORM" ]]; then
  PLATFORM="$HOST_OS"
fi
case "$PLATFORM" in
  linux|macos) ;;
  *) die "\`--platform\` must be \`linux\` or \`macos\`, not \`$PLATFORM\`" ;;
esac

case "$MODE" in
  operator|ci) ;;
  *) die "\`--mode\` must be \`operator\` or \`ci\`, not \`$MODE\`" ;;
esac

if [[ -z "$RUN_DATE" ]]; then
  RUN_DATE="$(date -u +%Y-%m-%d)"
fi
if [[ ! "$RUN_DATE" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}$ ]]; then
  die "\`--date\` must be \`YYYY-MM-DD\`, not \`$RUN_DATE\`"
fi

# A run label distinguishes a second run on the same day. It becomes part of
# the record's file name, so it is restricted to the characters a file name
# may carry without quoting.
if [[ -n "$RUN_LABEL" && ! "$RUN_LABEL" =~ ^[a-z0-9][a-z0-9-]*$ ]]; then
  die "\`--run-label\` must be lowercase letters, digits and dashes, not \`$RUN_LABEL\`"
fi

for tool in git python3; do
  command -v "$tool" >/dev/null 2>&1 || die "\`$tool\` is required and was not found on PATH"
done

# ── The operator's attestation ──────────────────────────────────────────────
#
# The attestation is the human's half of a qualification: who observed the
# behaviours, when, on which platform, and what they saw. It is checked here,
# before the run does anything, so that a malformed attestation fails the
# procedure rather than the record. Whether it *agrees* with the run is a
# question the run has to answer first, so that check happens when the record
# is written.
#
# The validator owns the attestation's shape; this procedure does not carry a
# second copy of the schema.
if [[ -n "$ATTESTATION_PATH" ]]; then
  if [[ ! -f "$ATTESTATION_PATH" ]]; then
    die "\`--attestation\` names \`$ATTESTATION_PATH\`, which is not a file"
  fi
  if ! ATTESTATION_PROBLEMS="$(python3 scripts/check_gui_qualification.py \
    --attestation "$ATTESTATION_PATH" --check-attestation 2>&1)"; then
    printf '%s\n' "$ATTESTATION_PROBLEMS" >&2
    die "the attestation file \`$ATTESTATION_PATH\` is not a valid operator attestation"
  fi
  log "$SCRIPT_NAME: the attestation at $ATTESTATION_PATH is well-formed"
  if [[ "$MODE" == "ci" ]]; then
    log "$SCRIPT_NAME: note: a CI run cannot itself attest; the attestation is the human's record, and it is merged as given"
  fi
fi

mkdir -p "$OUT_DIR"
OUT_DIR="$(cd "$OUT_DIR" && pwd)"
WORK_DIR="${TMPDIR:-/tmp}/eggsec-gui-qualification-$$"
mkdir -p "$WORK_DIR"
RECORD_NAME="$PLATFORM-$RUN_DATE"
if [[ -n "$RUN_LABEL" ]]; then
  RECORD_NAME="$RECORD_NAME-$RUN_LABEL"
fi
RECORD_PATH="$OUT_DIR/$RECORD_NAME.md"

# The daemon this run started, if any. The cleanup trap stops it and removes
# its socket on every exit path, including SIGINT and SIGTERM.
DAEMON_PID=""
DAEMON_SOCKET=""
DAEMON_SHA=""
DAEMON_TASK_KINDS=""
OWNED_SOCKET=""

cleanup() {
  local status=$?
  if [[ -n "$DAEMON_PID" ]] && kill -0 "$DAEMON_PID" 2>/dev/null; then
    kill -TERM "$DAEMON_PID" 2>/dev/null || true
    for _ in $(seq 1 20); do
      kill -0 "$DAEMON_PID" 2>/dev/null || break
      sleep 0.25
    done
    if kill -0 "$DAEMON_PID" 2>/dev/null; then
      kill -KILL "$DAEMON_PID" 2>/dev/null || true
    fi
    wait "$DAEMON_PID" 2>/dev/null || true
  fi
  if [[ -n "$OWNED_SOCKET" && -S "$OWNED_SOCKET" ]]; then
    rm -f "$OWNED_SOCKET"
  fi
  exit "$status"
}
trap cleanup EXIT INT TERM

# ── Identity ────────────────────────────────────────────────────────────────
COMMIT="$(git rev-parse HEAD 2>/dev/null || printf 'unknown')"
if [[ -z "$(git status --porcelain 2>/dev/null)" ]]; then
  TREE_STATE="clean"
else
  TREE_STATE="dirty"
fi
RUSTC_VERSION="$(rustc -V 2>/dev/null || printf 'not installed')"
CARGO_VERSION="$(cargo -V 2>/dev/null || printf 'not installed')"
TARGET_TRIPLE="$(rustc -vV 2>/dev/null | sed -n 's/^host: //p' || printf 'unknown')"
# The architecture of record is the one the toolchain targeted. `uname -m`
# reports the architecture of the *process*, which on an arm64 host running
# under Rosetta is `x86_64`; the record keeps both so neither is ambiguous.
TARGET_ARCH="${TARGET_TRIPLE%%-*}"
if [[ -z "$TARGET_ARCH" || "$TARGET_ARCH" == "unknown" ]]; then
  TARGET_ARCH="$HOST_ARCH"
fi

# A run qualifies the platform it was made on. The observations a host can
# make are observations of that host, so a record that carried them under
# another platform's label would be a false statement about that platform.
HOST_MATCHES=1
HOST_MISMATCH_REASON=""
if [[ "$PLATFORM" != "$HOST_OS" ]]; then
  HOST_MATCHES=0
  HOST_MISMATCH_REASON="the host platform is \`$HOST_OS\`, not the platform being qualified (\`$PLATFORM\`); a $PLATFORM qualification needs a $PLATFORM host"
fi

log "$SCRIPT_NAME: qualifying $PLATFORM on $HOST_OS ($HOST_ARCH), mode $MODE"

# ── The prerequisite probe ──────────────────────────────────────────────────
PROBE_COMMAND="bash scripts/check_gui_prereqs.sh"
PROBE_LOG="$WORK_DIR/prereq-probe.log"
PROBE_EXIT=0
if command -v bash >/dev/null 2>&1 && [[ -x scripts/check_gui_prereqs.sh ]]; then
  set +e
  bash scripts/check_gui_prereqs.sh >"$PROBE_LOG" 2>&1
  PROBE_EXIT=$?
  set -e
else
  printf 'the prerequisite probe is missing\n' >"$PROBE_LOG"
  PROBE_EXIT=127
fi
if [[ $PROBE_EXIT -eq 0 ]]; then
  PROBE_VERDICT="BUILDABLE"
else
  PROBE_VERDICT="NOT BUILDABLE"
fi
log "$SCRIPT_NAME: prerequisite probe -> $PROBE_VERDICT (exit $PROBE_EXIT)"

# ── The build column ────────────────────────────────────────────────────────
BUILD_DIR="$WORK_DIR/build"
BUILD_STATUS="skipped"
BUILD_REASON=""
BUILD_ARTEFACT=""
BUILD_SHA=""
BUILD_LOG=""
BUILD_ARTIFACT_NAME="$RECORD_NAME.build.json"

if [[ $HOST_MATCHES -eq 0 ]]; then
  BUILD_REASON="$HOST_MISMATCH_REASON"
elif [[ "$PROBE_VERDICT" != "BUILDABLE" ]]; then
  BUILD_REASON="the prerequisite probe reported NOT BUILDABLE on this host (exit $PROBE_EXIT); the missing packages are named in the probe's output"
elif ! command -v cargo >/dev/null 2>&1; then
  BUILD_REASON="cargo is not installed on this host, so no build could be attempted"
else
  log "$SCRIPT_NAME: building the shell for $PLATFORM"
  BUILD_LOG="$WORK_DIR/build.log"
  set +e
  bash scripts/build_gui_release.sh --out "$BUILD_DIR" --allow-dirty >"$BUILD_LOG" 2>&1
  BUILD_EXIT=$?
  set -e
  if [[ $BUILD_EXIT -ne 0 ]]; then
    BUILD_STATUS="fail"
    BUILD_REASON="the release build failed (exit $BUILD_EXIT); the build log is quoted in this record"
  else
    SIDECAR="$BUILD_DIR/eggsec-gui.build.json"
    if [[ ! -f "$SIDECAR" ]]; then
      BUILD_STATUS="fail"
      BUILD_REASON="the release build produced no build sidecar at \`$SIDECAR\`"
    else
      cp "$SIDECAR" "$OUT_DIR/$BUILD_ARTIFACT_NAME"
      BUILD_ARTEFACT="$BUILD_ARTIFACT_NAME"
      BUILD_SHA="$(shasum -a 256 "$OUT_DIR/$BUILD_ARTIFACT_NAME" 2>/dev/null | cut -d' ' -f1 || true)"
      if [[ -z "$BUILD_SHA" ]]; then
        BUILD_SHA="$(sha256sum "$OUT_DIR/$BUILD_ARTIFACT_NAME" 2>/dev/null | cut -d' ' -f1 || true)"
      fi
      BUILD_STATUS="pass"
      BUILD_REASON="the release build produced \`eggsec-gui.build.json\` and its SHA-256 is recorded"
    fi
  fi
fi

# ── The daemon ──────────────────────────────────────────────────────────────
# The daemon's socket is unique per run, so two concurrent runs on one host
# do not collide (`scripts/smoke-daemon-local.sh` does the same).
OWNED_SOCKET="${TMPDIR:-/tmp}/eggsec-gui-qualification-$$.sock"
DAEMON_READY=0
DAEMON_VERSION="unknown"
DAEMON_FEATURES="unknown"

daemon_answers() {
  python3 - "$1" <<'PY'
import json, socket, sys
path = sys.argv[1]
try:
    handle = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    handle.settimeout(2)
    handle.connect(path)
    stream = handle.makefile("rwb")
    stream.write(json.dumps({"type": "Health", "request_id": "qualify-probe"}).encode() + b"\n")
    stream.flush()
    line = stream.readline()
    handle.close()
except OSError:
    sys.exit(1)
sys.exit(0 if line else 1)
PY
}

daemon_task_kinds() {
  python3 - "$1" <<'PY'
import json, socket, sys
path = sys.argv[1]
try:
    handle = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    handle.settimeout(5)
    handle.connect(path)
    stream = handle.makefile("rwb")
    stream.write(json.dumps({"type": "DeclareClient", "request_id": "qualify-probe",
                             "kind": {"type": "Gui"}, "label": "qualification"}).encode() + b"\n")
    stream.flush()
    stream.readline()
    stream.write(json.dumps({"type": "Capabilities", "request_id": "qualify-caps"}).encode() + b"\n")
    stream.flush()
    line = stream.readline()
    handle.close()
except OSError:
    sys.exit(1)
try:
    message = json.loads(line)
except ValueError:
    sys.exit(1)
kinds = message.get("capabilities", {}).get("runtime", {}).get("task_kinds", [])
print(len(kinds))
PY
}

if [[ $HOST_MATCHES -eq 0 ]]; then
  log "$SCRIPT_NAME: $HOST_MISMATCH_REASON; the daemon half and the harness do not run"
elif [[ -n "$SOCKET" ]]; then
  DAEMON_SOCKET="$SOCKET"
  if daemon_answers "$DAEMON_SOCKET"; then
    DAEMON_READY=1
  fi
fi

if [[ $DAEMON_READY -eq 0 && -n "$DAEMON_BIN" ]]; then
  if [[ ! -x "$DAEMON_BIN" ]]; then
    die "\`--daemon-bin\` names \`$DAEMON_BIN\`, which is not an executable file"
  fi
  log "$SCRIPT_NAME: starting the daemon at $DAEMON_BIN"
  # The engine admits a loopback target only when the fixture marker is set
  # (`crates/eggsec/src/utils/parsing.rs:77`), and the harness's target is
  # the loopback fixture. The marker is the daemon's, not the harness's.
  EGGSEC_ALLOW_LOOPBACK_FIXTURE=1 "$DAEMON_BIN" \
    --full-executor --socket-path "$OWNED_SOCKET" --no-persistence \
    >"$WORK_DIR/daemon.log" 2>&1 &
  DAEMON_PID=$!
  for _ in $(seq 1 60); do
    if daemon_answers "$OWNED_SOCKET"; then
      DAEMON_READY=1
      DAEMON_SOCKET="$OWNED_SOCKET"
      break
    fi
    sleep 1
  done
  if [[ $DAEMON_READY -eq 0 ]]; then
    warn "$SCRIPT_NAME: the daemon did not become ready on $OWNED_SOCKET"
  fi
fi

if [[ $DAEMON_READY -eq 1 ]]; then
  if [[ -n "$DAEMON_BIN" && -x "$DAEMON_BIN" ]]; then
    DAEMON_SHA="$(shasum -a 256 "$DAEMON_BIN" 2>/dev/null | cut -d' ' -f1 || true)"
    if [[ -z "$DAEMON_SHA" ]]; then
      DAEMON_SHA="$(sha256sum "$DAEMON_BIN" 2>/dev/null | cut -d' ' -f1 || true)"
    fi
    DAEMON_VERSION="$DAEMON_BIN (sha256 $DAEMON_SHA)"
    DAEMON_FEATURES="service"
  else
    DAEMON_VERSION="attached to an operator-started daemon"
    DAEMON_FEATURES="unknown"
  fi
fi

# A daemon that publishes no task kinds is a no-op executor: its binary was
# not built with the `service` cargo feature, and it refuses every task. The
# record says so, rather than leaving a bare "unsupported task kind".
DAEMON_TASK_KINDS="$(daemon_task_kinds "$DAEMON_SOCKET" 2>/dev/null || printf '')"
if [[ -n "$DAEMON_TASK_KINDS" && "$DAEMON_TASK_KINDS" == "0" ]]; then
  warn "$SCRIPT_NAME: the daemon publishes no task kinds; it is a no-op executor."
  warn "$SCRIPT_NAME: build it with the \`service\` cargo feature (scripts/build-daemon-profile.sh --profile service)."
fi

# ── The harness ─────────────────────────────────────────────────────────────
HARNESS_OUT="$WORK_DIR/harness"
HARNESS_RESULT="$WORK_DIR/harness-outcome.json"
mkdir -p "$HARNESS_OUT"
HARNESS_LOG="$WORK_DIR/harness.log"
HARNESS_OUTCOME="$HARNESS_RESULT"
HARNESS_COMMAND="cargo test -p eggsec-gui --no-default-features --test qualification"
HARNESS_EXIT=0
# On a host that is not the platform being qualified, the harness is given no
# daemon to start: its observations would be of this host, not of the
# platform the record names.
HARNESS_DAEMON_BIN="$DAEMON_BIN"
if [[ $HOST_MATCHES -eq 0 ]]; then
  HARNESS_DAEMON_BIN=""
fi
set +e
EGGSEC_QUALIFY_SOCKET="$DAEMON_SOCKET" \
  EGGSEC_QUALIFY_DAEMON_BIN="$HARNESS_DAEMON_BIN" \
  EGGSEC_QUALIFY_PLATFORM="$PLATFORM" \
  EGGSEC_QUALIFY_OUT="$HARNESS_OUT" \
  EGGSEC_QUALIFY_RESULT="$HARNESS_RESULT" \
  cargo test -p eggsec-gui --no-default-features --test qualification \
  >"$HARNESS_LOG" 2>&1
HARNESS_EXIT=$?
set -e

if [[ ! -f "$HARNESS_RESULT" ]]; then
  HARNESS_EXIT="${HARNESS_EXIT:-1}"
  if [[ $HARNESS_EXIT -eq 0 ]]; then
    HARNESS_EXIT=1
  fi
  warn "$SCRIPT_NAME: the harness produced no outcome file"
fi

# ── The export artefacts ────────────────────────────────────────────────────
# The harness's exported reports are copied into the evidence directory under
# names that say what they are, so a cell's artefact reference resolves.
EXPORT_ARTIFACT_NAME="$RECORD_NAME.export.json"
EXPORT_ARTEFACT=""
EXPORT_SHA=""
if [[ -f "$HARNESS_OUT/live-report.json" ]]; then
  for extension in json csv html md sarif xml; do
    if [[ -f "$HARNESS_OUT/live-report.$extension" ]]; then
      cp "$HARNESS_OUT/live-report.$extension" "$OUT_DIR/$RECORD_NAME.export.$extension"
    fi
  done
  EXPORT_ARTEFACT="$EXPORT_ARTIFACT_NAME"
  EXPORT_SHA="$(shasum -a 256 "$OUT_DIR/$EXPORT_ARTIFACT_NAME" 2>/dev/null | cut -d' ' -f1 || true)"
  if [[ -z "$EXPORT_SHA" ]]; then
    EXPORT_SHA="$(sha256sum "$OUT_DIR/$EXPORT_ARTIFACT_NAME" 2>/dev/null | cut -d' ' -f1 || true)"
  fi
fi

# ── The window probe ────────────────────────────────────────────────────────
# The window probe is an observation, not an assertion. Its absence is a named
# skip; the operator's attestation is what the promotion rules read.
#
# The probe's outcome is the `launch` column's scripted outcome. The runbook's
# criterion is exactly one window titled `Eggsec`, so:
#
#   * the probe could not run          -> `skipped`, with the absent tool named;
#   * no GUI process is running        -> `not-performed`, because there was no
#                                         window for the probe to find;
#   * exactly one window was found     -> `pass`;
#   * the GUI is running and the probe
#     found no window (or more than
#     one)                              -> `fail`, because the window never
#                                         painted (or painted ambiguously).
WINDOW_PROBE="none"
WINDOW_COUNT=""
WINDOW_SCREENSHOT=""
WINDOW_REASON=""
LAUNCH_STATUS="not-performed"
LAUNCH_REASON=""
GUI_RUNNING=0
# `pgrep -x` matches the process NAME exactly. `pgrep -f` would match this
# run's own `cargo test -p eggsec-gui ...` command line and report a GUI that
# was never launched.
if command -v pgrep >/dev/null 2>&1 && pgrep -x 'eggsec-gui' >/dev/null 2>&1; then
  GUI_RUNNING=1
fi

if [[ $HOST_MATCHES -eq 0 ]]; then
  WINDOW_REASON="$HOST_MISMATCH_REASON"
  LAUNCH_STATUS="skipped"
  LAUNCH_REASON="$HOST_MISMATCH_REASON"
elif [[ $SKIP_WINDOW -eq 1 ]]; then
  WINDOW_REASON="the window probe was skipped on request (\`--skip-window\`)"
  LAUNCH_STATUS="skipped"
  LAUNCH_REASON="the window probe was skipped on request (\`--skip-window\`)"
elif [[ "$PLATFORM" == "linux" ]]; then
  if command -v xdotool >/dev/null 2>&1; then
    WINDOW_PROBE="xdotool"
    WINDOW_COUNT="$(xdotool search --name '^Eggsec$' 2>/dev/null | wc -l | tr -d ' ')"
    WINDOW_REASON="the window probe ran on this host"
  else
    WINDOW_REASON="\`xdotool\` is not installed on this host, so the window could not be probed"
    LAUNCH_STATUS="skipped"
    LAUNCH_REASON="\`xdotool\` is not installed on this host, so the window could not be probed"
  fi
else
  if command -v osascript >/dev/null 2>&1; then
    WINDOW_PROBE="osascript"
    WINDOW_COUNT="$(osascript -e 'tell application "System Events" to tell (first process whose name is "eggsec-gui") to get name of every window' 2>/dev/null | tr ',' '\n' | grep -c . || true)"
    WINDOW_REASON="the window probe ran on this host"
  else
    WINDOW_REASON="\`osascript\` is not installed on this host, so the window could not be probed"
    LAUNCH_STATUS="skipped"
    LAUNCH_REASON="\`osascript\` is not installed on this host, so the window could not be probed"
  fi
fi

# The screenshot is taken only when there is a GUI to photograph. A
# full-screen capture of a host with no GUI running is a picture of the
# operator's desktop, not evidence of anything, and it does not belong in the
# repository.
if [[ $GUI_RUNNING -eq 1 ]]; then
  if [[ "$PLATFORM" == "linux" ]] && command -v import >/dev/null 2>&1; then
    WINDOW_SCREENSHOT="$RECORD_NAME.launch.png"
    import -window root "$OUT_DIR/$WINDOW_SCREENSHOT" >/dev/null 2>&1 || WINDOW_SCREENSHOT=""
  elif [[ "$PLATFORM" != "linux" ]] && command -v screencapture >/dev/null 2>&1; then
    WINDOW_SCREENSHOT="$RECORD_NAME.launch.png"
    screencapture -x -o "$OUT_DIR/$WINDOW_SCREENSHOT" >/dev/null 2>&1 || WINDOW_SCREENSHOT=""
  fi
fi

if [[ "$LAUNCH_STATUS" == "not-performed" ]]; then
  if [[ $GUI_RUNNING -eq 1 ]]; then
    LAUNCH_STATUS="fail"
    LAUNCH_REASON="an \`eggsec-gui\` process is running, but the probe found ${WINDOW_COUNT:-0} window(s) titled \`Eggsec\`; the runbook's criterion is exactly one"
  else
    LAUNCH_STATUS="not-performed"
    LAUNCH_REASON="no \`eggsec-gui\` process is running on this host, so the probe had no window to find; the launch step is an operator action"
  fi
elif [[ "$LAUNCH_STATUS" == "skipped" ]]; then
  :
elif [[ "${WINDOW_COUNT:-0}" == "1" ]]; then
  LAUNCH_STATUS="pass"
  LAUNCH_REASON="the probe found exactly one window titled \`Eggsec\`"
else
  LAUNCH_STATUS="fail"
  LAUNCH_REASON="the probe found ${WINDOW_COUNT:-0} windows titled \`Eggsec\`; the runbook's criterion is exactly one"
fi

LAUNCH_ARTEFACT=""
LAUNCH_SHA=""
if [[ -n "$WINDOW_SCREENSHOT" && -f "$OUT_DIR/$WINDOW_SCREENSHOT" ]]; then
  LAUNCH_ARTEFACT="$WINDOW_SCREENSHOT"
  LAUNCH_SHA="$(shasum -a 256 "$OUT_DIR/$WINDOW_SCREENSHOT" 2>/dev/null | cut -d' ' -f1 || true)"
  if [[ -z "$LAUNCH_SHA" ]]; then
    LAUNCH_SHA="$(sha256sum "$OUT_DIR/$WINDOW_SCREENSHOT" 2>/dev/null | cut -d' ' -f1 || true)"
  fi
fi

# ── The record ──────────────────────────────────────────────────────────────
#
# The record's *shape* is owned by the validator: it is the thing that reads
# the record, so it is the thing that writes it. The procedure hands the
# validator a bag of facts and asks for the record back. There is therefore
# exactly one definition of the record's schema in the repository.
RECORD_JSON="$WORK_DIR/record.json"
RUN_INPUTS="$WORK_DIR/run-inputs.json"

export PLATFORM MODE RUN_DATE RECORD_NAME RECORD_PATH OUT_DIR WORK_DIR
export RUN_LABEL
export COMMIT TREE_STATE HOST_OS HOST_OS_VERSION HOST_ARCH TARGET_TRIPLE
export RUSTC_VERSION CARGO_VERSION PROBE_COMMAND PROBE_EXIT PROBE_VERDICT
export TARGET_ARCH
export BUILD_STATUS BUILD_REASON BUILD_ARTEFACT BUILD_SHA BUILD_LOG
export DAEMON_SOCKET DAEMON_READY DAEMON_VERSION DAEMON_FEATURES DAEMON_SHA
export DAEMON_TASK_KINDS
export HARNESS_COMMAND HARNESS_EXIT HARNESS_OUTCOME HARNESS_LOG
export EXPORT_ARTEFACT EXPORT_SHA
export WINDOW_PROBE WINDOW_COUNT WINDOW_SCREENSHOT WINDOW_REASON
export LAUNCH_STATUS LAUNCH_REASON LAUNCH_ARTEFACT LAUNCH_SHA
export PROBE_LOG

python3 - "$RUN_INPUTS" <<'PY'
import json, os, sys

def read(path, limit=20000):
    try:
        with open(path, "r", errors="replace") as handle:
            return handle.read()[:limit]
    except OSError:
        return ""

facts = {
    "platform": os.environ["PLATFORM"],
    "mode": os.environ["MODE"],
    "date": os.environ["RUN_DATE"],
    "run_label": os.environ.get("RUN_LABEL", ""),
    "commit": os.environ["COMMIT"],
    "tree_state": os.environ["TREE_STATE"],
    "host": {
        "os": os.environ["HOST_OS"],
        "os_version": os.environ["HOST_OS_VERSION"],
        "arch": os.environ["TARGET_ARCH"],
        "process_arch": os.environ["HOST_ARCH"],
        "target_triple": os.environ["TARGET_TRIPLE"],
    },
    "toolchain": {
        "rustc": os.environ["RUSTC_VERSION"],
        "cargo": os.environ["CARGO_VERSION"],
    },
    "features": {"requested": ["shell"], "resolved": ["default", "shell"]},
    "prerequisite_probe": {
        "command": os.environ["PROBE_COMMAND"],
        "exit": int(os.environ["PROBE_EXIT"]),
        "verdict": os.environ["PROBE_VERDICT"],
        "output": read(os.environ["PROBE_LOG"]),
    },
    "daemon": {
        "socket": os.environ["DAEMON_SOCKET"],
        "ready": os.environ["DAEMON_READY"] == "1",
        "version": os.environ["DAEMON_VERSION"],
        "features": os.environ["DAEMON_FEATURES"],
        "task_kinds": os.environ.get("DAEMON_TASK_KINDS", ""),
        "scope": {"surface": "DesktopGuiManual", "source": "gui-default"},
    },
    "window": {
        "probe": os.environ["WINDOW_PROBE"],
        "windows": os.environ["WINDOW_COUNT"],
        "screenshot": os.environ["WINDOW_SCREENSHOT"],
        "reason": os.environ["WINDOW_REASON"],
    },
    "launch": {
        "status": os.environ["LAUNCH_STATUS"],
        "reason": os.environ["LAUNCH_REASON"],
        "artifact": os.environ["LAUNCH_ARTEFACT"],
        "sha256": os.environ["LAUNCH_SHA"],
    },
    "harness": {
        "command": os.environ["HARNESS_COMMAND"],
        "exit": int(os.environ["HARNESS_EXIT"]),
        "outcome": os.environ["HARNESS_OUTCOME"],
        "output": read(os.environ["HARNESS_LOG"]),
    },
    "build": {
        "status": os.environ["BUILD_STATUS"],
        "reason": os.environ["BUILD_REASON"],
        "artifact": os.environ["BUILD_ARTEFACT"],
        "sha256": os.environ["BUILD_SHA"],
        "log": read(os.environ.get("BUILD_LOG", "")),
    },
    "export": {
        "artifact": os.environ["EXPORT_ARTEFACT"],
        "sha256": os.environ["EXPORT_SHA"],
    },
}
with open(sys.argv[1], "w") as handle:
    json.dump(facts, handle, indent=2)
PY

RECORD_WRITER_ARGS=(
  --write-record "$RECORD_JSON"
  --run-inputs "$RUN_INPUTS"
  --record-markdown "$RECORD_PATH"
)
if [[ -n "$ATTESTATION_PATH" ]]; then
  # The attestation is passed to the writer, so the record is written once
  # with the human's half already in it.
  RECORD_WRITER_ARGS+=(--attestation "$ATTESTATION_PATH")
fi
if ! python3 scripts/check_gui_qualification.py "${RECORD_WRITER_ARGS[@]}"; then
  die "the run record was not written; see the failures above"
fi

RECORDED_SCHEMA="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["schema"])' "$RECORD_JSON")"
if [[ "$RECORDED_SCHEMA" != "$RECORD_SCHEMA" ]]; then
  die "the run record declares the schema \`$RECORDED_SCHEMA\`, but the procedure expects \`$RECORD_SCHEMA\`"
fi

log "$SCRIPT_NAME: wrote the run record at $RECORD_PATH"

# ── The verdict ─────────────────────────────────────────────────────────────
FAILURES="$(python3 - "$RECORD_JSON" <<'PY'
import json, sys
record = json.load(open(sys.argv[1]))
failed = [c["column"] for c in record["columns"] if c["scripted"] == "fail"]
print(" ".join(failed))
PY
)"
if [[ -n "$FAILURES" ]]; then
  warn "$SCRIPT_NAME: FAILED columns: $FAILURES"
  exit 1
fi
if [[ "$MODE" == "ci" && ! -f "$HARNESS_RESULT" ]]; then
  warn "$SCRIPT_NAME: the harness did not run, so the run qualifies nothing"
  exit 1
fi
log "$SCRIPT_NAME: every column's scripted outcome is pass, skipped, or not-applicable"
exit 0
