# Phase F Plan: Platform Integration Maturity

## Status

Status: Ready for implementation.

## Objective

Improve confidence in platform-sensitive domains by building reproducible integration environments, deterministic fixtures, explicit prerequisite detection, and lifecycle tests. This phase targets mobile dynamic analysis, packet inspection, selected wireless behavior, and other system-dependent capabilities whose maturity is constrained primarily by environment availability rather than missing public types.

The emphasis is testability and operational correctness. This phase does not authorize expansion of hazardous capability.

## Preconditions

Canonical request/result/event contracts from Phase C should be stable before introducing long-lived platform fixtures. Phase E may establish shared artifact/session conventions that these integration tests should reuse.

## Primary areas

```text
crates/eggsec-mobile-lab/
crates/eggsec/src/mobile/
crates/eggsec/src/packet/
crates/eggsec/src/wireless/
crates/eggsec/src/stress/
crates/eggsec-nse/
crates/eggsec-python/src/*mobile*
crates/eggsec-python/src/*packet*
crates/eggsec-python/src/*wireless*
.github/workflows/deep-checks.yml
scripts/
docker-compose.yml
docs/MOBILE.md
docs/WIRELESS.md
docs/BUILD.md
docs/VERIFICATION.md
docs/python/domain-maturity.md
```

## Non-goals

This phase does not add deauthentication, credential capture, persistence, C2, exploit delivery, or other new attack primitives. It does not require privileged jobs in routine PR CI. It does not make hardware-dependent capabilities part of the default build. It does not depend on public internet targets.

## Workstream 1 — Build a platform capability/prerequisite matrix

For each platform-sensitive domain, document and detect:

- supported operating systems;
- required privileges/capabilities (`root`, `CAP_NET_RAW`, `CAP_NET_ADMIN`, etc.);
- required devices/interfaces;
- external executables such as ADB/Frida/browser tooling where genuinely required;
- kernel/network namespace requirements;
- architecture restrictions;
- feature flags;
- safe fixture mode versus real-hardware mode;
- whether the test can run in a container/VM/host only.

Expose prerequisite detection as a reusable diagnostic rather than scattering environment checks through tests.

A skipped test must state which prerequisite is absent.

## Workstream 2 — Android mobile-dynamic fixture

Create a reproducible Android integration environment suitable for ADB and the subset of Frida/runtime behaviors Eggsec currently claims.

Preferred options, in order:

1. documented local Android emulator/AVD runner with a pinned system image;
2. CI/self-hosted or manually triggered Linux runner capable of KVM acceleration;
3. software-emulated fallback only if runtime is acceptable.

Provide a small intentionally vulnerable/test APK owned by the repository or generated from source, covering only safe deterministic behaviors needed for validation: manifest/package discovery, launch, logcat, file push/pull if supported, process enumeration, instrumentation attach where currently implemented, lifecycle cleanup.

Do not fetch arbitrary third-party APKs during tests.

## Workstream 3 — ADB protocol lifecycle tests

Exercise the Rust-native ADB path against the emulator:

- transport handshake;
- device selection;
- shell command execution;
- sync push/pull;
- process/package discovery;
- timeout/cancellation;
- service close/cleanup;
- malformed/closed response handling;
- reconnect after emulator/device restart where supported.

Assert that failed close/cleanup operations are observable but do not corrupt later sessions.

## Workstream 4 — Frida/runtime instrumentation evidence

For the subset currently exposed by `mobile-dynamic`:

- detect backend availability;
- attach to the controlled test app;
- execute benign inspection/instrumentation actions already implemented by Eggsec;
- capture events/results;
- detach/cleanup;
- verify cancellation and app termination behavior.

If Frida is an optional external dependency, keep it optional and report unsupported clearly when absent.

## Workstream 5 — Packet-inspection network namespace fixture

Create a Linux-only deterministic network fixture using namespaces/veth/loopback or equivalent local isolation.

Test currently supported behaviors such as:

- packet capture with bounded packet count/duration;
- filter application;
- header parsing/hexdump;
- traceroute/ICMP behavior where deterministic inside the namespace;
- packet-send lifecycle for safe local packets;
- cancellation and timeout;
- interface disappearance/error handling;
- resource cleanup and no lingering capture tasks.

All generated traffic must remain inside the local fixture namespace/subnet.

## Workstream 6 — Wireless safe-fixture strategy

Separate wireless tests into:

- pure parser/state/unit tests requiring no hardware;
- passive scan/configuration tests that can use fixture data or monitor-mode capable test hardware;
- advanced active behaviors that remain manual/hardware-gated.

For active wireless features already present, build protocol/state tests around crafted fixture frames where possible rather than transmitting them. Real RF transmission should remain an explicitly documented lab/manual test and should not be required for ordinary release readiness unless the project later establishes dedicated RF-isolated hardware.

## Workstream 7 — Root/privilege containment

Tests requiring privilege must run in a disposable isolated environment.

Requirements:

- never require developers to run the entire test suite as root;
- isolate privileged helper/test processes;
- minimize Linux capabilities rather than granting blanket privilege where possible;
- enforce local target/scope fixtures;
- cleanup namespaces/interfaces/processes on success, failure, and cancellation;
- provide a `doctor`/preflight output that explains missing capabilities.

## Workstream 8 — Scheduled/manual CI topology

Add optional deep-check jobs only where a reliable hosted environment exists.

Recommended cadence:

- emulator/mobile integration: scheduled/manual, optionally self-hosted if GitHub-hosted KVM constraints are unsuitable;
- network namespace/packet tests: scheduled/manual Linux job with required capabilities if safe/available;
- passive fixture-based wireless tests: routine or deep checks depending runtime;
- real hardware/RF tests: documented maintainer procedure, not hosted CI.

Do not turn unavailable hardware into permanent green skips with no owner. Track the intended execution environment in documentation.

## Workstream 9 — Bounded skip budgets and maturity evidence

For Python/platform suites, retain or extend the existing validation-profile concept so a suite cannot silently become 100% skipped.

Record separately:

- tests that are expected to run in the fixture environment;
- tests intentionally hardware-only;
- tests unsupported on the current OS;
- tests blocked by a known external prerequisite.

A fixture profile should fail if required integration tests unexpectedly skip.

## Workstream 10 — Operational robustness checks

Run repeated lifecycle loops to detect:

- file descriptor leaks;
- orphaned child processes;
- stale ADB services;
- leaked browser/emulator processes;
- network namespace/interface leaks;
- hanging capture tasks;
- Tokio task leaks where observable;
- non-idempotent cleanup.

Use bounded repetition sufficient to catch lifecycle regressions without making routine development slow.

## Workstream 11 — Documentation and maturity review

Update build/verification/domain docs with:

- exact prerequisites;
- fixture setup and teardown;
- commands for each platform profile;
- expected skips on unsupported environments;
- what is and is not release-gated;
- which maturity checklist items are now satisfied.

Do not promote `wireless-advanced` or other hazardous domains solely because fixture tests exist.

## Acceptance criteria

- mobile-dynamic has a reproducible emulator integration profile that executes nontrivial ADB/runtime lifecycle tests;
- packet inspection has a deterministic local privileged/network-namespace fixture on supported Linux environments;
- wireless testing clearly separates fixture/passive/manual-RF validation;
- prerequisite detection is centralized and diagnostic;
- privileged tests are isolated and do not require the full suite to run as root;
- required fixture-profile tests cannot silently all skip;
- repeated lifecycle tests show no known persistent resource leaks;
- platform-sensitive documentation accurately states support/prerequisites;
- routine CI remains lightweight while deep/manual profiles provide real integration evidence;
- no new hazardous capability is added.

## Completion record

Record baseline/final SHA, tested OS/kernel/tool versions, fixture images/artifacts, commands, required privileges, test/skip counts, lifecycle-loop results, and any external/hardware blockers.