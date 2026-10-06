# Platform Integration Maturity

Deterministic fixtures, explicit prerequisites, and isolated live tests for
platform-sensitive domains. No new hazardous capability is added here.

## Capability / prerequisite matrix

Source of truth: `crates/eggsec/src/platform/` — `PlatformReport::collect()`
(`prereqs.rs`). `eggsec doctor` prints the same matrix; live tests call
`skip_reason_for(domain)` so a skipped test always names the absent
prerequisite. `report_for()` knows exactly six domains: `mobile-dynamic`,
`packet-inspection`, `wireless`, `wireless-advanced`, `stress-testing`, `nse`.
Any other name falls through to an "unknown domain" report with neither fixture
nor live support — so adding a platform-sensitive domain requires a matrix here.

| Domain | OS | Privilege | Devices / binaries | Feature | Fixture (always runs) | Live (isolated, may SKIP) |
|--------|----|-----------|-------------------|---------|----------------------|---------------------------|
| `mobile-dynamic` | any (emulator via TCP) | none for fixture; lab device owned by you for live | `adb` (optional, pure-Rust probe otherwise), `frida` CLI optional, emulator/AVD or lab device | `mobile-dynamic` (`→mobile`) | mock ADB server + Frida simulation: `cargo test -p eggsec-mobile-lab --features mobile-dynamic` | AVD: `scripts/setup_android_emulator.sh --start`, then `./scripts/test-mobile-dynamic.sh <apk> --real` |
| `packet-inspection` | Linux for live; fixture anywhere | `CAP_NET_RAW`/root for live only | `lo` for loopback checks; `ip` for netns | `packet-inspection` | parser/craft/hexdump/loopback: `cargo test -p eggsec --lib --features packet-inspection packet::fixture::` | netns: `sudo scripts/setup_packet_netns.sh --run` (10.200.1.0/24, cleaned on EXIT) |
| `wireless` (passive) | Linux for live; fixture anywhere | `CAP_NET_ADMIN`/root for live only | `iwlist` (wireless-tools), managed/up lab interface | `wireless` | parser/heuristic/fixtures: `cargo test -p eggsec --lib --features wireless wireless::fixture::` | lab scan: `sudo eggsec wireless wlan0 --dry-run` first, then real on owned nets |
| `wireless-advanced` (active) | Linux, lab hardware | `CAP_NET_ADMIN`/root + monitor mode + `--allow-active-wireless` | monitor interface (e.g. `wlan0mon`) | `wireless-advanced` (`→wireless`) | frame crafting dry-run: `cargo test -p eggsec --lib --features wireless,wireless-advanced wireless::fixture::` | **manual lab only, never CI**: crafted frames are tested as bytes; RF transmission is a maintainer procedure |
| `stress-testing` | Linux for live; fixture anywhere | `CAP_NET_RAW`/root for live only | none beyond scope+budgets | `stress-testing` | auth/metrics/caps: `cargo test -p eggsec --lib stress::` (no traffic) | manual, scoped, rate/duration-capped lab runs only |
| `nse` | all | none | `libssl-dev` at build | `nse` | `./scripts/test-nse.sh` (safe scripts) | local only |

A skipped live test prints e.g. `SKIP packet-inspection: CAP_NET_RAW (running
without capture privilege) [root=false, caps=…] fix: …`. Fixture tests never skip
for privilege or hardware.

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
- Non-root: live capture/scan/flood report `PrivilegeRequired` + a `fix:` hint
  (sudo/capabilities, or the netns/emulator script); fixtures still pass.
- No `adb`/`iwlist`/`emulator`: live legs SKIP; pure-Rust probes and canned
  fixtures still run.
- No Frida CLI: instrumentation planning/dry-run works; live attach reports
  `Missing (optional)` and stays simulation.

A skip reason names the first blocking prerequisite and includes
`capabilities_summary()`, e.g.
`SKIP packet-inspection: CAP_NET_RAW (running without capture privilege) [root=false, caps=…]`.

## What is and is not release-gated

- **Release-gated**: fixture suites above (hermetic, deterministic), `make
  check`, `eggsec doctor` matrix presence, and the Python `wireless-fixture` /
  `mobile-dynamic-fixture` / `packet-parser` validation profiles (which fail if
  all their tests skip).
- **Not release-gated**: live netns/emulator/RF runs and the `packet-live`,
  `active-probes`, `stress-testing`, `mobile-emulator` Python profiles. The
  `platform-integration` job in `.github/workflows/deep-checks.yml` is
  scheduled/manual only — never PR CI — and its live legs use `|| true`, so a
  missing prerequisite can never turn the job red.
- **Never promoted by fixtures alone**: `wireless-advanced`, `stress-testing`,
  `postex`, `c2`, and other hazardous domains stay lab-only regardless of
  fixture coverage.

## Maturity checklist

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

## See Also

- `architecture/overview.md` — module index and deep-dive links
- [FEATURE_MATRIX.md](FEATURE_MATRIX.md) — feature flags and system dependencies
- [docs/VERIFICATION.md](VERIFICATION.md) — the `make check` contract
