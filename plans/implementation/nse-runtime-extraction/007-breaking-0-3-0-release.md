# NSE Runtime Extraction Milestone 007C-R — Breaking 0.3.0 Release

Status: ready for handoff

Repository baseline: `eggstack/eggsec-nse@d4a22f1dbe56f4ccfb17b2a8135aae8395f44f19`

Source roadmap:

- `plans/subsystems/nse-runtime-extraction-roadmap.md#milestone-007--protocol-library-gating-and-controlled-automated-activation`

Supersedes:

- `plans/implementation/nse-runtime-extraction/007-standalone-security-patch-release.md` (M007C, blocked)

Closure evidence that forced this replan:

- `plans/closure/nse-runtime-extraction/007c-closure.md` (Status: blocked)

Applicable ADRs:

- `plans/adrs/ADR-0004-nse-automated-activation-boundary.md`
- `plans/adrs/ADR-0003-nse-host-provider-boundary.md`

Primary class: capability + security release

Release repository:

- `eggstack/eggsec-nse`

## 1. Objective

Publish the M007 runtime hardening as **`0.3.0`**, a deliberate breaking release, so Eggsec can adopt a real registry artifact before scoped automated activation begins.

One bounded outcome: a published, immutable `0.3.0` whose migration notes account for all 74 public functions removed from 0.2.0.

## 2. Why this milestone is ready

- Hard dependency `007a-closure.md`: closed.
- Hard dependency `007b-closure.md`: closed (corrective).
- M007C's §3 gate was executed and failed with a major break, which is exactly the evidence this plan needs to exist. The decision to bump rather than restore is recorded in `007c-closure.md` §4.2 and is not re-litigated here.
- Publication authentication is available (registry-scoped crates.io token present), so the §9 stop condition "publication authentication cannot be performed safely" is not expected to trigger.

Unlike M007C, **no version is presumed in advance.** The version to publish is `0.3.0` because a major break exists; if further qualification reveals additional breaking changes, `0.3.0` remains correct, and if a *smaller* break is somehow introduced by this plan, that is a defect in this plan (see §9).

## 3. Current implementation evidence

At `d4a22f1`:

- 74 public functions present in published 0.2.0 are absent: 70 `register_<mod>_library(&Lua)` and 4 `helpers::*` (`007c-closure.md` §4, with the exact module list and full `cargo semver-checks` output attached as `007c-semver-report.txt`).
- 89 `register_*_library_with_services` entry points exist, 70 of which replace a published path.
- `helpers::tcp_connect_with_timeout` and `helpers::tls_connect` were capability-bypassing public API and are gone. Their absence is enforced: restoring either into production scope makes `./scripts/check-boundaries.sh` fail with `M007B violation: specialized direct-I/O residual changed` (probe evidence in `007c-closure.md` §8).
- Hosted CI is green on this exact SHA: run `36640412317`, 5/5 jobs (MSRV 1.89, Linux, macOS, Windows, SSH runtime).
- `0.3.0` is absent from crates.io (versions present: `0.2.0`, `0.1.0`; neither yanked).
- `CHANGELOG.md` last documents 0.2.0. No `0.2.1` or `0.3.0` section exists.

## 4. Invariants that must not regress

- ADR-0004 §8: no network effect without a capability decision. Specifically, no public API may return a raw `std::net::TcpStream`/`UdpSocket` handle obtained outside the broker. `helpers::tls_connect` and `helpers::tcp_connect_with_timeout` MUST NOT be reinstated, deprecated-then-restored, or reintroduced under any name.
- `./scripts/check-boundaries.sh` must remain green, including the residual pins (15 ungated + 7 advisory) and the M005E 97-file baseline.
- The M007B manifest invariants hold: manifest `register_fn` values match real exported functions, manifest↔registration consistency is enforced both directions, and the compat allowlist stays explicit.
- No `--allow-dirty`, no `--no-verify` on any publish path.
- M007C must remain reproducible from this record: the gate output is archived, not merely summarized.

## 5. Scope

### In scope

- Version bump to `0.3.0` with explicit migration notes for all 74 removals.
- `CHANGELOG.md` entry disclosing the automated gating change, the corrected residual, and the full API break.
- `README.md` / `docs/PROVIDERS.md` accuracy pass for the new registration signature.
- Release-workflow verification.
- Exact-candidate qualification (§10).
- Publication, tag, GitHub Release, docs.rs confirmation, scratch-consumer verification.
- One static-guard hardening item from `007c-closure.md` §10 (the `nse_production_code` truncation caveat), if it can be done without changing residual pins.
- Closure record `007c-r-closure.md`.

### Explicitly out of scope

- Restoring any removed function for compatibility. The two direct-connect helpers are permanently withdrawn; the 70 registration shims are documented as migrated, not re-added.
- Further residual migration, provider design, or capability work — M007B is closed.
- Any `eggstack/eggsec` change. Adoption is M007D.
- `public_api` sync surface changes.

## 6. Required production changes

### Core/domain

- `Cargo.toml`: `version = "0.3.0"`.
- `CHANGELOG.md`: new `## [0.3.0]` section. Required content:
  - **Breaking**: the 70 registration entry points, with the mechanical replacement rule (`register_x_library(lua)` → `register_x_library_with_services(lua, &capability_ctx, &services)`), the full module list, and a note that `NseCapabilityContext` + `NseHostServices` must be threaded from the caller.
  - **Breaking**: removal of `helpers::tls_connect`, `helpers::tcp_connect_with_timeout`, `helpers::make_addr`, `helpers::parse_socket_addr`, with the security rationale for the first two stated plainly: they returned raw `std::net::TcpStream` from an unmediated `connect_timeout`, so keeping them would preserve a capability bypass. Do not soften this into "deprecated".
  - **Security**: automated gating change — 65 libraries become automated-visible (manual-only → `ProviderBacked`), `target` is reclassified `Pure` → `ProviderBacked`, and the M007A effect manifest + AgentSafe/CiSafe registration/require gate are new.
  - **Security**: `target.resolve` DNS is now brokered; a `Pure`-classified library had been able to emit DNS under `CiSafe`.
  - **Honest residual**: 97 → 22 specialized direct-I/O files (15 ungated + 7 advisory), manifest manual-only 106 → 41. State explicitly that the remaining long-tail direct-I/O protocols are **not** provider-backed.
  - **Known limitations** (from `007b-closure.md` §12): `broker_dns_lookup` does not evaluate per-target membership (M007D); `DnsResolution` is not charged to `network_operations`; `upnp.discover` no longer performs SSDP multicast discovery.
- `README.md` and `docs/PROVIDERS.md`: update registration examples to the `_with_services` form. If either currently shows a `register_*_library(lua)` example, that is an existing documentation defect this milestone must fix, and it should be named in the changelog.

### Documentation and static guards

- Add a guard that a restored direct-connect primitive cannot hide after a `#[cfg(test)]` module. Cheapest correct form: assert the residual scan also runs on the *untruncated* file, or assert `mod tests` is the final item in every file the residual scan inspects. Must not change the pinned residual sets.
- If the guard work is not achievable safely in this slice, record it as a low finding in `007c-r-closure.md` rather than silently skipping it.

## 7. Ordered work packages

### Work package A — Gate re-confirmation on the release candidate

Intent: prove the release candidate introduces no *additional* breaking change beyond the 74 already known, so the migration notes are complete and the version choice is right.

Required changes: none (read-only verification).

Acceptance evidence:

```bash
cargo semver-checks check-release --baseline-version 0.2.0 --features nse
```

The only failed lint must be `function_missing`, and the reported item set must be exactly the 74 in `007c-closure.md` §4. Any additional major lint, or a different item count, is a stop condition (§9).

### Work package B — Version, changelog, and documentation

Intent: publish an honest, complete account of the break and the security posture.

Required changes: `Cargo.toml` version; `CHANGELOG.md` 0.3.0 section per §6; `README.md`/`docs/PROVIDERS.md` registration examples.

Acceptance evidence: every one of the 74 removals appears in the changelog with a replacement or an explicit "withdrawn, no replacement" statement; a reviewer can migrate a 0.2.0 consumer using only the changelog.

### Work package C — Guard hardening

Intent: close the `nse_production_code` truncation caveat from `007c-closure.md` §10 (low).

Required changes: `scripts/check-boundaries.sh` only.

Acceptance evidence: the guard still fails on the known negative probes; a new negative probe placing a direct-connect primitive *after* `mod tests` is now detected; residual pins unchanged; `./scripts/check-boundaries.sh` green.

### Work package D — Exact-candidate qualification

Intent: qualify the precise SHA that will be tagged and published.

Required changes: none.

Acceptance evidence: §10 block, all green, on the exact candidate; hosted CI green 5/5 on that SHA.

### Work package E — Publication and post-publication verification

Intent: create the immutable artifact and prove it is consumable.

Required changes: none to source.

Acceptance evidence: `0.3.0` absent before publication; publish without `--allow-dirty`/`--no-verify`; tag resolves to the candidate; published archive VCS identity equals the candidate; docs.rs state recorded; scratch consumer resolves `eggsec-nse = { version = "=0.3.0", features = ["nse"] }` from the registry with no path/git fallback; `nse`, `nse-ssh2`, `nse,sandbox` all qualify in the scratch consumer.

## 8. Failure, cancellation, restart, and contention semantics

Not applicable to the release mechanics. The one behavioral change consumers must understand is that registration now requires a caller-supplied `NseCapabilityContext` and `NseHostServices`; a consumer that has no profile context must construct a permissive one explicitly rather than relying on a default. `007c-closure.md` §4.1 records the exact signatures. Note in the changelog that `NseHostServices::native()` exists as the escape hatch for manual, operator-driven use, and that it is *not* valid for automated authority claims.

## 9. Compatibility and migration

This is a breaking release. `0.2.0` consumers must:

1. Replace every `register_x_library(lua)` call with `register_x_library_with_services(lua, &capability_ctx, &services)`. Mechanical, but the two new arguments are mandatory and cannot be defaulted, because defaulting them would recreate the native-fallback path ADR-0004 forbids for automated claims.
2. Stop calling `helpers::tcp_connect_with_timeout` / `helpers::tls_connect`. There is no replacement: use the broker (`broker_tcp_connect`, `broker_dns_lookup`) so the operation is capability-checked and accounted.
3. Stop calling `helpers::make_addr` / `helpers::parse_socket_addr`; both are trivial to inline, and no crate-internal caller remains.
4. Accept that the two-step upgrade is not available from 0.2.0 — there is no 0.2.1.

Rollback: crates.io versions are immutable. A bad 0.3.0 can only be yanked, and consumers pinned to `=0.3.0` will then fail resolution. Before publishing, confirm the changelog and migration notes are complete enough that a yank is not the intended remedy for any foreseeable defect.

## 10. Required verification

Run on the exact candidate SHA:

```bash
cargo fmt --all --check
./scripts/check-boundaries.sh
cargo metadata --no-deps
cargo tree --workspace
cargo check --no-default-features
cargo check --features nse
cargo test --features nse
cargo check --features nse-ssh2
cargo check --features nse,sandbox
cargo clippy --all-targets --features nse
cargo +1.89.0 check --locked --no-default-features
cargo +1.89.0 check --locked --features nse
cargo publish --dry-run
cargo package --list
cargo package
cargo semver-checks check-release --baseline-version 0.2.0 --features nse   # expect exactly the known 74
```

Hosted Linux/macOS/Windows/MSRV/SSH CI must be green on the same candidate.

## 11. Acceptance criteria

1. `cargo semver-checks` reports only the 74 known removals.
2. `0.3.0` publishes from the exact candidate, without `--allow-dirty` or `--no-verify`.
3. Tag and GitHub Release resolve to the candidate; published archive VCS identity matches.
4. Changelog documents all 74 removals, the automated gating change, and the corrected residual without overclaiming provider coverage.
5. Scratch consumer resolves `=0.3.0` from the registry for `nse`, `nse-ssh2`, and `nse,sandbox`, with no path/git fallback.
6. docs.rs state recorded.
7. Guard hardening either lands with a passing negative probe, or is recorded as a low finding.
8. No `eggstack/eggsec` change.
9. Closure record `007c-r-closure.md` written, and M007D's hard dependency updated to point at it.

## 12. Stop conditions

Stop and report rather than improvise when:

- `cargo semver-checks` reports any major lint beyond the known `function_missing`, or a different item count than 74;
- any proposal arises to restore `helpers::tcp_connect_with_timeout` or `helpers::tls_connect` in any form, including a deprecated alias;
- candidate CI is red;
- residual documentation overclaims provider coverage;
- source/tag/archive identity diverges;
- publication authentication cannot be performed safely;
- the guard hardening would require changing residual pins (that is M007B-owned scope, not release scope);
- release work leaks into `eggstack/eggsec` or into provider design.

## 13. Closure evidence required

`plans/closure/nse-runtime-extraction/007c-r-closure.md` must record:

- the M007A/B/C closures and the §3 gate failure that forced the replan;
- `cargo semver-checks` output archived, with the item count confirmed as 74;
- exact candidate SHA and hosted run;
- full §10 command list with pass/fail per command;
- package and `publish --dry-run` result;
- publication method (Trusted Publishing if configured, else the manual-token path, stated without exposing credentials);
- registry artifact and source/tag identity;
- docs.rs state;
- scratch consumer results for all three feature combinations;
- the exact version published;
- changelog completeness confirmation against the 74-item list;
- unresolved findings by severity;
- GO/NO-GO for M007D.

## 14. Handoff notes

- The two removed direct-connect helpers are the security payload of this release. Do not let a reviewer "helpfully" restore them for compatibility; the guard will fail, and if the guard is bypassed the release is wrong.
- `007c-closure.md` §8 records a real caveat in the guard: `nse_production_code()` truncates each file at its first `mod tests`. If you add a negative probe for work package C, place it in production scope, or it will pass vacuously — this exact mistake was made and caught during the M007C audit.
- M007B's closure and tests are the current baseline (696 tests, `d4a22f1`); expect ~2 min for the semver check and ~2 min for the full qualification block.
- The 70-module list in `007c-closure.md` §4.1 is the authoritative migration mapping; do not regenerate it from the current tree, which has drifted (94 plain functions now exist, including compat wrappers added for `target`/`radius`/`dnsbl`).
