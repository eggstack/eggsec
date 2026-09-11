# Platform Capability Detection

## Overview

Read-only environment detection for platform-sensitive domains, so tests and `eggsec doctor` report the same facts. Implemented in `crates/eggsec/src/platform/` (`mod.rs` + `prereqs.rs`, ~800 lines). Centralizes OS/arch/kernel, binary-presence, Linux capability, and interface checks behind a stable matrix — no privilege acquisition, no transmission, no mutation of namespaces/interfaces/devices.

Parent overview: [overview.md](overview.md). Related: [mobile.md](mobile.md), [networking.md](networking.md), [wireless.md](wireless.md).

## Role & Responsibilities

- **Single prerequisite matrix**: per-domain `DomainPrerequisites` plus fixture-vs-hardware guidance for `mobile-dynamic`, `packet-inspection`, `wireless`, and other hardware/privilege-gated domains.
- **Read-only detection**: `has_binary`, `has_cap_net_admin`, `has_cap_net_raw`, `is_root`, `list_interfaces`, `current_os/arch/kernel` — best-effort facts, never side-effecting.
- **Consistent skip reasons**: `skip_reason_for()` gives test harnesses the canonical "which prerequisite is absent" message, so skipped live tests are self-explanatory.
- **Doctor parity**: `capabilities_summary()` / `report_for()` feed `eggsec doctor` and `scripts/check_platform.sh` (both hermetic — no root/hardware required).

## Location & Feature Gating

| Component | Path | Feature Gate |
|-----------|------|-------------|
| Facade re-exports | `platform/mod.rs` | Always |
| Matrix + detection | `platform/prereqs.rs` | Always |

No feature gating — always compiled. Privileged checks report status; they never attempt to acquire privilege.

## Architecture

### PrereqStatus (4 variants)

Defined at `platform/prereqs.rs:12`:

| Variant | Meaning |
|---------|---------|
| `Available` | Present and usable |
| `Missing` | Absent on this host (binary missing, interface down, …) |
| `Unsupported` | Not supported on this OS/arch (e.g. `iwlist` outside Linux) |
| `PrivilegeRequired` | Present in principle but needs privilege the current user lacks |

`is_blocking()` returns `true` for anything except `Available`.

### Prerequisite (1 row)

`platform/prereqs.rs:32` — stable `id` (e.g. `os-linux`, `bin-adb`, `priv-root`), human `label`, `status`, factual `detail`, actionable `fix` (empty when nothing applies). Serializes with `#[serde(tag = "status", rename_all = "snake_case")]`.

### DomainPrerequisites + PlatformReport

`DomainPrerequisites` (`prereqs.rs:99`): stable domain id (`mobile-dynamic`, `packet-inspection`, `wireless`, …), prerequisite set, and fixture-vs-live guidance. `PlatformReport` aggregates across domains for `doctor` output.

### Public functions (facade)

Re-exported from `platform/mod.rs:13-17`:

- `all_domain_ids()`, `capabilities_summary()`, `report_for(domain)`
- `current_os()`, `current_arch()`, `current_kernel()`
- `has_binary(name)`, `has_cap_net_admin()`, `has_cap_net_raw()`, `is_root()`, `list_interfaces()`
- `skip_reason_for(domain)` — canonical skip message for test harnesses

## Behavior / Flow

```
test harness / `eggsec doctor`
        │ report_for(domain) / skip_reason_for(domain)
        ▼
read-only detectors (binaries, caps, OS, interfaces)
        │
        ├── Available → run live path (or still prefer fixtures in CI)
        └── Missing / Unsupported / PrivilegeRequired → SKIP with reason
                (live setup scripts SKIP on missing prerequisites;
                 never run the full suite as root)
```

Live scripts (`setup_packet_netns.sh`, `setup_android_emulator.sh`) SKIP on missing prerequisites. `EGGSEC_ALLOW_LOOPBACK_FIXTURE=1` enables loopback fixtures for Python tests.

## Integration Points

- **Doctor**: `commands/handlers/doctor.rs` renders `PlatformReport`; hermetic by design.
- **Packet**: `packet/` live capture/crafting gated on `cap_net_raw`/interfaces. See [networking.md](networking.md).
- **Wireless**: passive `iwlist` parsing vs. `wireless-advanced` injection (root + monitor mode). See [wireless.md](wireless.md).
- **Mobile dynamic**: ADB/Frida presence checks behind `mobile-dynamic`. See [mobile.md](mobile.md).
- **Stress**: raw-socket/IP-spoofing paths report `PrivilegeRequired` instead of failing obscurely. See [stress.md](stress.md).
- **CI**: `scripts/check_platform.sh` + `docs/PLATFORM.md` prerequisite matrix; platform-sensitive tests use fixtures, not hardware.

## Testing

- Hermetic unit tests: status mapping, `is_blocking()`, skip-reason formatting — no root/hardware/network required.
- Fixture-first: platform-sensitive suites run against loopback/fixtures in CI; live paths are opt-in and self-skipping.
- Doctor snapshot: `PlatformReport` serialization round-trips.

## Invariants & Gotchas

1. **Read-only always** — detection never acquires privilege, transmits, or mutates system state.
2. **Skips must name the prerequisite** — use `skip_reason_for()`, not bare `#[ignore]`.
3. **Never run the full suite as root** — privileged checks report; they don't escalate.
4. **Fixtures over hardware in CI** — live tests SKIP without prerequisites; missing setup scripts SKIP, never fail.
5. **Stable ids** — `Prerequisite.id` and domain ids are contract; renaming breaks doctor consumers and skip-message grep.

## Cross-Links

- [overview.md](overview.md) — workspace context, feature flags
- [networking.md](networking.md) — packet capture/crafting consumers
- [wireless.md](wireless.md) — WiFi passive/active gating
- [mobile.md](mobile.md) — ADB/Frida dynamic prerequisites
- [stress.md](stress.md) — raw-socket privilege reporting
- [cli_commands.md](cli_commands.md) — `doctor` command surface

---

*Last verified against source: 2026-09-11*
