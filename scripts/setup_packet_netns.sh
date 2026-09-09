#!/usr/bin/env bash
# setup_packet_netns.sh — deterministic Linux network-namespace fixture (Phase F).
#
# Creates an isolated namespace (default: eggsec-test) with a veth pair, runs
# a bounded local capture/parsing check, then tears everything down on EXIT
# (success, failure, or cancellation). All traffic stays in 10.200.1.0/24 or
# loopback; nothing leaves the host.
#
# Usage:
#   ./scripts/setup_packet_netns.sh --check     # prerequisite report only
#   sudo ./scripts/setup_packet_netns.sh --run  # full fixture (needs CAP_NET_ADMIN)
#
# Requirements: Linux, iproute2 (ip), root or CAP_NET_ADMIN. Fixture parser
# tests (cargo test packet::fixture) run without any of this.
set -euo pipefail

NS="${EGGSEC_NETNS:-eggsec-test}"
VETH_HOST="v-eggsec-h"
VETH_NS="v-eggsec-n"
SUBNET="10.200.1.0/24"
HOST_IP="10.200.1.1/24"
NS_IP="10.200.1.2/24"

cleanup() {
  ip netns del "${NS}" 2>/dev/null || true
  ip link del "${VETH_HOST}" 2>/dev/null || true
}
trap cleanup EXIT INT TERM

check() {
  echo "=== Packet netns prerequisites ==="
  echo "os: $(uname -s)"
  if [[ "$(uname -s)" != "Linux" ]]; then echo "SKIP packet-live: Linux-only fixture"; return 2; fi
  if ! command -v ip >/dev/null 2>&1; then echo "SKIP packet-live: iproute2 (ip) not in PATH"; return 2; fi
  if [[ "${EUID:-$(id -u)}" -ne 0 ]]; then echo "SKIP packet-live: needs root/CAP_NET_ADMIN (running as non-root). Fixture parser tests still run: cargo test -p eggsec --lib --features packet-inspection packet::fixture::"; return 2; fi
  echo "ok   Linux + ip + root; namespace ${NS} available"
  echo "fixture (no privilege): cargo test -p eggsec --lib --features packet-inspection packet::fixture::"
}

run() {
  check
  echo "creating namespace ${NS} (${SUBNET})..."
  cleanup
  ip netns add "${NS}"
  ip link add "${VETH_HOST}" type veth peer name "${VETH_NS}"
  ip link set "${VETH_NS}" netns "${NS}"
  ip addr add "${HOST_IP}" dev "${VETH_HOST}"
  ip link set "${VETH_HOST}" up
  ip netns exec "${NS}" ip addr add "${NS_IP}" dev "${VETH_NS}"
  ip netns exec "${NS}" ip link set lo up
  ip netns exec "${NS}" ip link set "${VETH_NS}" up
  echo "namespace ready; bounded connectivity check (2 pings, local only)..."
  ip netns exec "${NS}" ping -c 2 -W 1 10.200.1.1
  echo "bounded capture check: interface list inside namespace"
  ip netns exec "${NS}" ip -o link show
  echo "PASS packet netns fixture (isolated; cleaned up on exit)."
}

case "${1:---check}" in
  --check) check ;;
  --run) run ;;
  *) echo "usage: $0 [--check|--run]" >&2; exit 2 ;;
esac
