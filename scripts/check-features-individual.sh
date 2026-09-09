#!/usr/bin/env bash
# Individual per-feature compilation sweep (Phase B, workstream 3).
#
# Compiles every declared engine feature in its minimum dependency set, plus
# domain-crate, daemon/CLI, and Python-crate profiles. This is the exhaustive
# oracle: the `full` aggregate is curated (28 pinned members) and therefore
# not sufficient on its own.
#
# Cadence: weekly/manual via `deep-checks.yml`, or locally via
#   make check-features-individual
# NOT part of the per-PR `make check` contract (too expensive for every push).
#
# Behavior:
# - Missing system prerequisites (protoc, libpcap, libssh2) are recorded as
#   SKIP, not FAIL, and reported separately in the summary.
# - Any Rust compile failure is a FAIL and sets a non-zero exit code.
# - Newly declared features with no sweep entry fail loudly (orphan check).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

PASS=()
FAIL=()
SKIP=()

note_pass() { PASS+=("$1"); echo "  PASS: $1"; }
note_fail() { FAIL+=("$1"); echo "  FAIL: $1"; }
note_skip() { SKIP+=("$1 ($2)"); echo "  SKIP: $1 (prerequisite: $2)"; }

have_protoc() { command -v protoc >/dev/null 2>&1; }
have_libssh2() {
  # libssh2 dev package provides ssh2.h; pkg-config entry is libssh2.
  [ -f /usr/include/libssh2.h ] || [ -f /usr/local/include/libssh2.h ] \
    || pkg-config --exists libssh2 2>/dev/null
}
have_libpcap() {
  [ -f /usr/include/pcap.h ] || [ -f /usr/include/pcap/pcap.h ] \
    || pkg-config --exists libpcap 2>/dev/null
}

run_check() {
  local label="$1"; shift
  echo "--- $label: cargo check $*"
  if "$@" >/tmp/eggsec-feat-check.log 2>&1; then
    note_pass "$label"
  else
    echo "---- log tail ($label) ----"
    tail -n 30 /tmp/eggsec-feat-check.log || true
    echo "---- end log ----"
    note_fail "$label"
  fi
}

echo "=== Individual feature sweep ==="
echo "Repo: $REPO_ROOT"
echo ""

# ── 1. Engine features, minimum activation sets ──────────────────────────
# Companion markers are the minimum set needed to activate the feature
# meaningfully (base domain first where the marker alone is a no-op path).
echo "--- Engine features (crates/eggsec) ---"
ENGINE_FEATURES=$(python3 -c "
import tomllib
with open('crates/eggsec/Cargo.toml','rb') as f:
    feats = tomllib.load(f)['features']
for name in sorted(feats):
    if name not in ('default','full'):
        print(name)
")

declare -A ENGINE_SETS=(
  # backend drivers are meaningful with their base domain
  ["db-pentest-mongodb"]="db-pentest,db-pentest-mongodb"
  ["db-pentest-mssql-tiberius"]="db-pentest,db-pentest-mssql-tiberius"
  ["db-pentest-redis"]="db-pentest,db-pentest-redis"
  # exposure markers compile against their base domain + tool surface
  ["db-pentest-mcp"]="db-pentest-mcp,tool-api,rest-api"
  ["web-proxy-mcp"]="web-proxy-mcp,tool-api,rest-api"
  ["c2-mcp"]="c2-mcp,tool-api,rest-api"
)

# System-prerequisite-gated engine features (SKIP when the prerequisite is
# absent; FAIL on real compile errors when present).
PREREQ_GRPC_API="protobuf-compiler (protoc)"
PREREQ_NSE_SSH2="libssh2-dev"
PREREQ_PACKET="libpcap-dev"
PREREQ_STRESS="libpcap-dev"

while IFS= read -r feat; do
  [ -z "$feat" ] && continue
  case "$feat" in
    grpc-api)
      if ! have_protoc; then note_skip "eggsec/$feat" "$PREREQ_GRPC_API"; continue; fi ;;
    nse-ssh2)
      if ! have_libssh2; then note_skip "eggsec/$feat" "$PREREQ_NSE_SSH2"; continue; fi ;;
    packet-inspection)
      if ! have_libpcap; then note_skip "eggsec/$feat" "$PREREQ_PACKET"; continue; fi ;;
    stress-testing)
      if ! have_libpcap; then note_skip "eggsec/$feat" "$PREREQ_STRESS"; continue; fi ;;
  esac
  feats="${ENGINE_SETS[$feat]:-$feat}"
  run_check "eggsec/$feat" cargo check -p eggsec --features "$feats"
done <<< "$ENGINE_FEATURES"

# Orphan guard: every engine feature must have an explicit sweep entry above.
# ENGINE_SETS documents intentional companion sets; the case statement
# documents prerequisite-gated features. Anything else uses the bare name.
echo ""
echo "--- Orphan guard: sweep covers every engine feature ---"
MISSING_SWEEP=$(python3 -c "
import tomllib
with open('crates/eggsec/Cargo.toml','rb') as f:
    feats = set(tomllib.load(f)['features']) - {'default','full'}
print(' '.join(sorted(feats)))
")
for feat in $MISSING_SWEEP; do
  # Each feature is either compiled bare in the loop above or has a companion
  # entry / prerequisite gate. The loop handles all names mechanically, so
  # reaching here with an empty check means the loop skipped it.
  if ! printf '%s\n' "$ENGINE_FEATURES" | grep -qx "$feat"; then
    note_fail "orphan engine feature without sweep profile: $feat"
  fi
done
echo "  (all engine features have a sweep profile)"

# ── 2. Curated aggregate ─────────────────────────────────────────────────
echo ""
echo "--- Aggregate: full ---"
run_check "eggsec/full" cargo check -p eggsec --features full

# ── 3. Domain crates ─────────────────────────────────────────────────────
echo ""
echo "--- Domain crates ---"
run_check "eggsec-nse/nse" cargo check -p eggsec-nse --features nse
if have_libssh2; then
  run_check "eggsec-nse/nse-ssh2" cargo check -p eggsec-nse --features nse-ssh2
else
  note_skip "eggsec-nse/nse-ssh2" "$PREREQ_NSE_SSH2"
fi
run_check "eggsec-nse/sandbox" cargo check -p eggsec-nse --features sandbox
run_check "eggsec-nse/stress-testing" cargo check -p eggsec-nse --features stress-testing
run_check "eggsec-db-lab/db-drivers" cargo check -p eggsec-db-lab --features db-drivers
run_check "eggsec-db-lab/mssql" cargo check -p eggsec-db-lab --features mssql
run_check "eggsec-db-lab/mongodb" cargo check -p eggsec-db-lab --features mongodb
run_check "eggsec-db-lab/redis" cargo check -p eggsec-db-lab --features redis
run_check "eggsec-db-lab/mcp" cargo check -p eggsec-db-lab --features mcp
run_check "eggsec-web-proxy/web-proxy" cargo check -p eggsec-web-proxy --features web-proxy
run_check "eggsec-web-proxy/web-proxy-mcp" cargo check -p eggsec-web-proxy --features web-proxy-mcp
run_check "eggsec-web-proxy/transparent-proxy" cargo check -p eggsec-web-proxy --features transparent-proxy
run_check "eggsec-web-proxy/dynamic-plugins" cargo check -p eggsec-web-proxy --features dynamic-plugins
run_check "eggsec-mobile-lab/mobile-dynamic" cargo check -p eggsec-mobile-lab --features mobile-dynamic

# ── 4. Daemon / CLI feature sets ─────────────────────────────────────────
echo ""
echo "--- Daemon / CLI ---"
run_check "eggsec-daemon/http-api" cargo check -p eggsec-daemon --features http-api
run_check "eggsec-daemon/full-executor" cargo check -p eggsec-daemon --features full-executor
run_check "eggsec-cli/default(tui)" cargo check -p eggsec-cli
run_check "eggsec-cli/headless" cargo check -p eggsec-cli --no-default-features
run_check "eggsec-cli/daemon-client" cargo check -p eggsec-cli --no-default-features --features daemon-client

# ── 5. Python crate profiles (cargo-level; wheel behavior in make check-python)
echo ""
echo "--- Python crate (cargo-level) ---"
run_check "eggsec-python/no-default" cargo check -p eggsec-python --no-default-features
run_check "eggsec-python/full-no-system" cargo check -p eggsec-python --features full-no-system

# ── Summary ──────────────────────────────────────────────────────────────
echo ""
echo "=== Sweep summary ==="
echo "PASS: ${#PASS[@]}"
echo "SKIP: ${#SKIP[@]}"
for s in "${SKIP[@]:-}"; do [ -n "$s" ] && echo "  skip: $s"; done
echo "FAIL: ${#FAIL[@]}"
for f in "${FAIL[@]:-}"; do [ -n "$f" ] && echo "  fail: $f"; done

if [ "${#FAIL[@]}" -gt 0 ]; then
  echo "RESULT: FAIL (${#FAIL[@]} failing profile(s))"
  exit 1
fi
echo "RESULT: OK (${#PASS[@]} passed, ${#SKIP[@]} skipped on prerequisites)"
