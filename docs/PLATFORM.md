# Platform Integration Maturity (Phase F)

Deterministic fixtures, explicit prerequisites, and isolated live tests for
platform-sensitive domains. No new hazardous capability is added by this phase.

## Capability / prerequisite matrix

Source of truth: `crates/eggsec/src/platform/` (`PlatformReport::collect()`).
`eggsec doctor` prints the same matrix; tests use `skip_reason_for()` so a
skipped test always names the absent prerequisite.

| Domain | OS | Privilege | Devices / binaries | Feature | Fixture (always runs) | Live (isolated, may SKIP) |
|--------|----|-----------|-------------------|---------|----------------------|---------------------------|
| `mobile-dynamic` | any (emulator via TCP) | none for fixture; lab device owned by you for live | `adb` (optional, pure-Rust probe otherwise), `frida` CLI optional, emulator/AVD or lab device | `mobile-dynamic` (`→mobile`) | mock ADB server + Frida simulation: `cargo test -p eggsec-mobile-lab --features mobile-dynamic` | AVD: `scripts/setup_android_emulator.sh --start`, then `./scripts/test-mobile-dynamic.sh <apk> --real` |
| `packet-inspection` | Linux for live; fixture anywhere | `CAP_NET_RAW`/root for live only | `lo` for loopback checks; `ip` for netns | `packet-inspection` | parser/craft/hexdump/loopback: `cargo test -p eggsec --lib --features packet-inspection packet::fixture::` | netns: `sudo scripts/setup_packet_netns.sh --run` (10.200.1.0/24, cleaned on EXIT) |
| `wireless` (passive) | Linux for live; fixture anywhere | `CAP_NET_ADMIN`/root for live only | `iwlist` (wireless-tools), managed/up lab interface | `wireless` | parser/heuristic/fixtures: `cargo test -p eggsec --lib --features wireless wireless::fixture::` | lab scan: `sudo eggsec wireless wlan0 --dry-run` first, then real on owned nets |
| `wireless-advanced` (active) | Linux, lab hardware | `CAP_NET_ADMIN`/root + monitor mode + `--allow-active-wireless` | monitor interface (e.g. `wlan0mon`) | `wireless-advanced` (`→wireless`) | frame crafting dry-run: `cargo test -p eggsec --lib --features wireless,wireless-advanced wireless::fixture::` | **manual lab only, never CI**: crafted frames are tested as bytes; RF transmission is a maintainer procedure |
| `stress-testing` | Linux for live; fixture anywhere | `CAP_NET_RAW`/root for live only | none beyond scope+budgets | `stress-testing` | auth/metrics/caps: `cargo test -p eggsec --lib stress::` (no traffic) | manual, scoped, rate/duration-capped lab runs only |
| `nse` | all | none | `libssl-dev` at build | `nse` | `./scripts/test-nse.sh` (safe scripts) | local only |

A skipped live test prints e.g. `SKIP packet-inspection: CAP_NET_RAW / root
(running without capture privilege)`. Fixture tests never skip for privilege
or hardware.

## Fixture setup and teardown

- **Android**: no emulator needed for fixtures. Mock ADB speaks
  CNXN/OPEN/OKAY/WRTE/CLSE on `127.0.0.1:0`; Frida uses simulation sessions.
  Minimal test APKs are generated from source:
  `python3 scripts/make_test_apk.py --out /tmp/eggsec-test.apk`
  (deterministic, 683 bytes, no third-party fetch). Live AVD:
  `scripts/setup_android_emulator.sh --check|--start|--stop`
  (pinned API 34/x86_64 image, KVM check, software fallback note).
- **Packet**: fixture is pure craft→parse→hexdump plus loopback UDP and a
  bounded UDP traceroute to `127.0.0.1`. Live netns:
  `scripts/setup_packet_netns.sh --check|--run` (creates `eggsec-test`
  namespace + veth pair, traps EXIT/INT/TERM for cleanup).
- **Wireless**: `wireless::fixture` provides canned `iwlist` output and
  multi-BSSID networks for the rogue heuristic, plus byte-exact deauth/
  disassoc frames (never transmitted).

## Commands per profile

```bash
# Hermetic fixture layer (no root/hardware) — routine use
cargo run -p eggsec-cli -- doctor
bash scripts/check_platform.sh

# Individual fixture suites
cargo test -p eggsec --lib --features packet-inspection packet::fixture::
cargo test -p eggsec --lib --features wireless wireless::fixture::
cargo test -p eggsec --lib --features wireless,wireless-advanced wireless::fixture::
cargo test -p eggsec-mobile-lab --features mobile-dynamic --lib
./scripts/test-mobile-dynamic.sh   # dry-run, no device

# Live layers (isolated, may SKIP with named prerequisite)
sudo scripts/setup_packet_netns.sh --run
./scripts/setup_android_emulator.sh --check
```

## Expected skips on unsupported environments

- Non-Linux: packet-live, wireless-live, netns report `Unsupported`/`SKIP`.
- Non-root: live capture/scan/flood report `PrivilegeRequired` + fix
  (`sodo`/capabilities or the netns/emulator script); fixtures still pass.
- No `adb`/`iwlist`/`emulator`: live legs SKIP; pure-Rust probes and canned
  fixtures still run.
- No Frida CLI: instrumentation planning/dry-run works; live attach reports
  `Missing (optional)` and stays simulation.

## What is and is not release-gated

- **Release-gated**: fixture suites above (hermetic, deterministic), `make
  check`, `eggsec doctor` matrix presence, Python `wireless-fixture` and
  `mobile-dynamic-fixture` profiles (fail if all skip).
- **Not release-gated**: live netns/emulator/RF runs, `packet-live`,
  `active-probes`, `stress-testing`, `mobile-emulator` (manual/scheduled;
  see `.github/workflows/deep-checks.yml` `platform-integration`).
- **Never promoted by fixtures alone**: `wireless-advanced`, `stress-testing`,
  `postex`, `c2`, and other hazardous domains stay lab-only regardless of
  fixture coverage.

## Maturity checklist satisfied (Phase F)

- Centralized prerequisite detection (`platform::`) reused by doctor + tests.
- Reproducible emulator integration profile (mock ADB lifecycle + documented
  AVD runner + generated test APK).
- Deterministic packet namespace fixture + loopback lifecycle tests.
- Wireless unit/passive/manual-RF separation with fixture frames.
- Privilege containment (no full-suite-as-root; isolated helpers; doctor
  explains missing caps).
- Bounded skip budgets (fixture profiles fail on 100% skip).
- Repeated lifecycle loops (ADB 10x, Frida 20x, packet 20x, wireless 9+20x)
  with FD-leak guards where observable.
- Routine CI stays lightweight; deep/manual profiles carry integration evidence.
