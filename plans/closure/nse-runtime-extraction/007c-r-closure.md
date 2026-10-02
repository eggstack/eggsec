# NSE Runtime Extraction Milestone 007C-R — Closure Status

Status: closed

Source implementation plan:

- `plans/implementation/nse-runtime-extraction/007-breaking-0-3-0-release.md` (M007C-R)

Source subsystem roadmap:

- `plans/subsystems/nse-runtime-extraction-roadmap.md#milestone-007--protocol-library-gating-and-controlled-automated-activation`

Applicable ADRs:

- `plans/adrs/ADR-0004-nse-automated-activation-boundary.md`
- `plans/adrs/ADR-0003-nse-host-provider-boundary.md`

Planning baseline: Eggsec `d2076c64`

Implementation repository: `eggstack/eggsec-nse`

Repository baseline reviewed: `d4a22f1dbe56f4ccfb17b2a8135aae8395f44f19` (inherited M007B closure SHA)

Implementation commits or pull requests:

- `eggstack/eggsec-nse@16cb38ee78f680bd07739dc3fc1ef776c9f19c9c` — the exact release candidate: version `0.3.0`, the 0.3.0 changelog/migration notes, README/PROVIDERS/RELEASING reconciliation, and the guard hardening.
- `eggstack/eggsec-nse@870fca7` — post-release reconciliation of the advisory and the predecessor-version disposition in `docs/RELEASING.md` (on `main`, deliberately **not** on the tag).
- Hosted CI run `37023503110` on `16cb38e`: 5/5 jobs green (MSRV 1.89, Linux, macOS, Windows, SSH runtime).

Companion security record: `plans/closure/nse-runtime-extraction/007c-s-closure.md` (M007C-S, executed 2026-10-02 against the artifact produced here).

## 1. Executive finding

**`eggsec-nse 0.3.0` is published from the exact candidate `16cb38e`, and the M007C-R exit gate is met.**

The milestone's single bounded outcome — a published, immutable `0.3.0` whose migration notes account for all 74 public functions removed from 0.2.0 — is complete and verified end to end:

- the semver gate was re-run **on the release candidate** and reports exactly the known break: 196 checks, 195 pass, **1 fail**, the single failed lint `function_missing`, **74 items**, and the item set is **byte-identical** to the archived M007C gate;
- the full §10 qualification block is 15/15 green on the candidate, and hosted CI is 5/5 green on that same SHA;
- `0.3.0` published via the manual-token recovery path (registry-side Trusted Publishing is still pending), the published archive's `.cargo_vcs_info.json` records `sha1 = 16cb38e…`, the immutable `v0.3.0` tag points at that same commit and was never moved, and the GitHub Release was created only *after* registry verification;
- docs.rs built `0.3.0`, and a registry-only scratch consumer resolves and builds `=0.3.0` for `nse`, `nse-ssh2`, and `nse,sandbox` with no path/git fallback.

The work package C guard item landed rather than being deferred as a low finding, and it closed a **live** hole rather than a hypothetical one: `nse_production_code()` truncates each file at its first `mod tests` marker, and `src/libraries/helpers.rs` — the very file that lost the two direct-connect helpers — carries 45 production lines below its test module. The absence of those helpers is now enforced by the compiler, and the truncation hole is closed by an untruncated sweep.

No stop condition triggered. No `eggstack/eggsec` production-code change occurred.

## 2. Requirement-to-evidence matrix

| Requirement (plan §) | Evidence | Result | Notes |
|---|---|---|---|
| §11.1 semver reports only the 74 known removals | `007c-r-semver-report.txt`: `196 checks: 195 pass, 1 fail, 0 warn, 58 skip`; 74 `function eggsec_nse::*` items; sole failed lint `function_missing` | **pass** | item set diffed against `007c-semver-report.txt` → identical |
| §11.2 `0.3.0` publishes from the exact candidate, no `--allow-dirty`/`--no-verify` | `cargo publish` from clean `16cb38e`; neither flag used | **pass** | §4 |
| §11.3 tag + release resolve to the candidate; archive VCS identity matches | archive `.cargo_vcs_info.json` sha1 = `16cb38ee…`; `v0.3.0^{}` = `16cb38ee…`; release `target_commitish: main` published after registry verification | **pass** | tag never moved or recreated |
| §11.4 changelog documents all 74 removals + gating + corrected residual, no provider overclaim | `CHANGELOG.md` `## [0.3.0]` | **pass** | 70 modules + 4 helpers itemised; §3 |
| §11.5 scratch consumer resolves `=0.3.0` for all three feature sets | §4 | **pass** | `source = registry+…`, no path/git |
| §11.6 docs.rs state recorded | `docs.rs/eggsec-nse/0.3.0/eggsec_nse/` → HTTP 200 | **pass** | |
| §11.7 guard hardening lands with a passing negative probe, or is a recorded low finding | §4.3 — both negative probes fire | **pass** | not deferred |
| §11.8 no `eggstack/eggsec` production-code change | Eggsec tree untouched during this milestone | **pass** | |
| §11.9 publication sequencing matches the chosen authentication path | path E2; Trusted Publishing registry side still pending (`docs/RELEASING.md`) | **pass** | one path only |
| §11.10 companion `007c-s-closure.md` records the affected range + advisory/CVE + yank disposition | `007c-s-closure.md` §7 addendum | **pass** | executed 2026-10-02 |
| §11.11 both closure records written before M007D is unblocked | this record + `007c-s-closure.md` | **pass** | |
| §7 WP-A gate re-confirmation on the candidate | §4.1 | **pass** | |
| §7 WP-B version/changelog/docs | §3 | **pass** | |
| §7 WP-C guard hardening, residual pins unchanged | §4.3 | **pass** | pins byte-identical in the published archive |
| §7 WP-D exact-candidate qualification | §4.2 | **pass** | 15/15 |
| §7 WP-E publication + post-publication verification | §4.4 | **pass** | |
| §7 WP-F security-communication handoff | §6 | **pass** | |

## 3. Production implementation evidence

Candidate `16cb38e` changes 8 files (+433 / −25).

**Version and migration notes.** `Cargo.toml` `0.2.0 → 0.3.0`, with the matching `Cargo.lock` entry. `CHANGELOG.md` gains a `## [0.3.0]` section that accounts for all 74 removals: the four withdrawn `helpers::*` names with the security rationale stated plainly and a "no replacement, do not add a deprecated alias" instruction; the 70-module registration list with the mechanical `register_x_library(lua)` → `register_x_library_with_services(lua, &capability_ctx, &services)` rule and the reason the arguments are mandatory; the M007A effect-gate disclosure (159 manifest entries, `Pure` 34 / `ProviderBacked` 84 / `ManualOnlyDirectIo` 34 / `ManualOnlyAdvisory` 7, manual-only 106 → 41, 65 promotions plus `target` `Pure → ProviderBacked`); the `target.resolve` unbrokered-DNS fix; the honest 97 → 22 residual; and the inherited limitations.

The changelog also records the two facts a later reader would otherwise miscount: `register_telnet_library` is `_with_services`-only but was never published plain, so the break is 70 modules and not 71; and 18 modules still expose both forms as a pinned, separately reviewed compatibility surface.

**Guard hardening (work package C).** Two layers, deliberately different in kind:

1. *Compiler.* `src/libraries/helpers.rs` gains a `#[cfg(all(feature = "nse", doctest))] mod withdrawn_api_guards` with one `no_run` control that proves `libraries::helpers` resolves — so the `compile_fail` blocks cannot pass vacuously — and four `compile_fail` doctests for `tls_connect`, `tcp_connect_with_timeout`, `make_addr`, and `parse_socket_addr`. Compiler-enforced, whole-crate, immune to the truncation caveat.
2. *Text backstop.* `scripts/check-boundaries.sh` factors the eleven effect patterns into one shared `nse_specialized_effect_filter` and adds `nse_specialized_residual_untruncated`, which re-runs the same patterns on the **untruncated** file and requires the result to stay inside the pinned residual inventory (`comm -13` against the pins). Containment, not equality: the untruncated sweep resolves to exactly the same 22 files, so no pin was added, changed, or removed.

The `nse_production_code()` caveat is now documented in-script as load-bearing rather than left as folklore, and the effect pattern list can no longer drift between the two views.

**Documentation.** `README.md` gains a `0.3` install line, an "Upgrading from 0.2" section covering both breaking changes, and an explicit statement that 0.3.0 is not complete protocol-wide scope enforcement. `docs/PROVIDERS.md` gains an `M007C` section explaining *why* the enforcement is compiler-based rather than scan-based. `docs/RELEASING.md` gains the release-history entry, the security-disclosure note, the advisory/yank reconciliation, and the two standing communication rules.

## 4. Verification executed

### 4.1 Commands run

```bash
# WP-A: the gate, re-run on the release candidate
cargo semver-checks check-release --baseline-version 0.2.0 --features nse
# → 196 checks: 195 pass, 1 fail, 0 warn, 58 skip
# → the only failure is `function_missing`, 74 items
# → item set diffed against plans/closure/.../007c-semver-report.txt: IDENTICAL
```

Two toolchain facts had to be established before that result could be read, and both are recorded because a wrong tool silently produces a *passing* answer here:

- `cargo-semver-checks 0.49.0` (the version that produced the archived M007C report) cannot parse the rustdoc JSON that the default `stable` (1.99.0) emits (`unsupported rustdoc format v61`; it supports v56/v57/v60).
- `cargo-semver-checks 0.49.0` also refuses `rustc 1.89.0` (`rustc version is not high enough: >=1.91.0 needed`). So no installed toolchain satisfies 0.49.0.
- `cargo-semver-checks 0.50.0` was installed to `/tmp/opencode/csc-050` (the existing 0.49.0 was left in place).

Running 0.50.0 directly as `0.2.0 -> 0.3.0 (major change)` prints `0 checks: 0 pass, 254 skip` / `Summary no semver update required` in 0.000s. **That is a vacuous pass and was not accepted as evidence.** Two controls were run to explain it:

- *Tool control.* 0.50.0 against the unmodified `0.2.0` tree (`d4a22f1`, version-pinned) reproduces the archived M007C result exactly: `196 checks: 195 pass, 1 fail`, 74 items, all four helpers present. So the tool is working and the archived report is reproducible.
- *Explanation.* Once a major version change is made, semver-checks permits major lints, so `function_missing` is reported as skipped rather than failed. `0.3.0` is therefore the correct version, and the plan's requirement is the break *set*, not "no break".
- *Candidate run.* To obtain the failure-level item set for the real candidate, the candidate was checked in a scratch worktree at `16cb38e` with only the version string pinned back to `0.2.0` (forcing a version-equal comparison). The release tree itself was never modified for this. Result: `196 checks: 195 pass, 1 fail, 0 warn, 58 skip`, 74 items, sole failed lint `function_missing`, and the item set is identical to the archived M007C gate.

Archival: the report is stored at `plans/closure/nse-runtime-extraction/007c-r-semver-report.txt`.

### 4.2 Results — exact-candidate qualification (WP-D)

Run from a clean tree at `16cb38ee78f680bd07739dc3fc1ef776c9f19c9c` (`git status --porcelain` empty):

| Command | Exit | Time |
|---|---|---|
| `cargo fmt --all --check` | 0 | 1s |
| `./scripts/check-boundaries.sh` | 0 | 2s |
| `cargo metadata --no-deps` | 0 | 0s |
| `cargo tree --workspace` | 0 | 0s |
| `cargo check --no-default-features` | 0 | 10s |
| `cargo check --features nse` | 0 | 145s |
| `cargo test --features nse` | 0 | 533s |
| `cargo check --features nse-ssh2` | 0 | 19s |
| `cargo check --features nse,sandbox` | 0 | 6s |
| `cargo clippy --all-targets --features nse` | 0 | 11s |
| `cargo +1.89.0 check --locked --no-default-features` | 0 | 7s |
| `cargo +1.89.0 check --locked --features nse` | 0 | 7s |
| `cargo publish --dry-run` | 0 | 8s |
| `cargo package --list` | 0 | 0s |
| `cargo package` | 0 | 1s |

15/15 green. `cargo test --features nse`: **701 tests passed, 0 failed** across all suites (M007B baseline was 696; the +5 are the new doctests: one control plus four `compile_fail`). The doc-test suite reports `5 passed; 0 failed; 1 ignored`.

Hosted CI run `37023503110` on the same SHA: `ssh-runtime` success, `msrv` success, `rust (windows-latest)` success, `rust (ubuntu-latest)` success, `rust (macos-latest)` success — 5/5.

### 4.3 Guard hardening: both negative probes (WP-C)

```bash
# probe 1 — direct-connect primitive placed AFTER `mod tests`
#   helpers.rs production view would emit lines 1..(183-1); the probe lived at
#   line 251, so the pre-hardening truncated scan could not see it
./scripts/check-boundaries.sh
# → exit 1
#   M007C-R violation: untruncated direct-I/O effect outside the pinned
#   residual inventory. ... src/libraries/helpers.rs
# revert → exit 0

# probe 2 — restore a withdrawn public helper
# cargo test --features nse --doc
# → FAILED. 4 passed; 1 failed
#   "Test compiled successfully, but it's marked `compile_fail`"
#   (the restored tcp_connect_with_timeout block; the other three blocks
#    correctly still fail to compile, and the control still passes)
# revert → 5 passed, 1 ignored
```

Both probes were reverted; `git status` was clean before qualification began. The residual pins shipped in the published archive are byte-identical to the repository pins (`nse-specialized-ungated.txt`, `nse-specialized-advisory.txt`).

### 4.4 Publication and post-publication verification (WP-E)

**Path chosen: E2 (manual-token recovery).** `docs/RELEASING.md` records the registry-side Trusted Publisher entry as still pending, and the workflow is tag-triggered with `rust-lang/crates-io-auth-action`; 0.2.0 used the same path. Exactly one path was used.

```bash
# 1. 0.3.0 absent
curl -s https://crates.io/api/v1/crates/eggsec-nse    # max_version 0.2.0; no 0.3.0
git ls-remote --tags origin                          # v0.1.0, v0.2.0 only

# 2. publish from the exact clean candidate (no --allow-dirty, no --no-verify,
#    token from the Cargo credential store, never on a command line)
cargo publish
# → Packaged 320 files, 3.3MiB (575.4KiB compressed)
# → Published eggsec-nse v0.3.0 at registry `crates-io`

# 3. registry artifact / source identity
curl -sL -o eggsec-nse-0.3.0.crate \
  https://static.crates.io/crates/eggsec-nse/eggsec-nse-0.3.0.crate
tar -xzf eggsec-nse-0.3.0.crate
# → .cargo_vcs_info.json: {"git":{"sha1":"16cb38ee78f680bd07739dc3fc1ef776c9f19c9c"}}
# → archive sha256 4497c2d16d6d25759d2400ce4e4632b3d51c723e871e5bc466ce6af4bc063b48
# → published source: 0 occurrences of the four withdrawn helper definitions
# → published source: 6 `compile_fail` markers (the 4 guards + prose references)
# → published residual pins byte-identical to the repository pins
# → published CHANGELOG.md has the `## [0.3.0]` section

# 4. immutable tag, created AFTER registry verification (path E2 ordering)
git tag -a v0.3.0 16cb38ee78f680bd07739dc3fc1ef776c9f19c9c
git push origin v0.3.0
# → v0.3.0^{} = 16cb38ee78f680bd07739dc3fc1ef776c9f19c9c

# 5. GitHub Release, after registry verification
gh release create v0.3.0 --title "eggsec-nse 0.3.0 — breaking security release" \
  --notes-file release-notes.md
# → published_at 2026-10-02T16:00:07Z, draft=false, 4438-char body

# 6. docs.rs
curl -o /dev/null -w "%{http_code}" https://docs.rs/eggsec-nse/0.3.0/eggsec_nse/
# → 200

# 7. scratch consumers, registry-only, no path/git fallback
#    eggsec-nse = { version = "=0.3.0", features = [...] }
#   ["nse"]           → source registry+https://github.com/rust-lang/crates.io-index, build OK
#   ["nse-ssh2"]      → source registry+…, build OK
#   ["nse","sandbox"] → source registry+…, build OK
#   and a run against the published artifact: "consumer030 ok"
```

No token value, prefix, or shape appears in this record. The crates.io credential was used only through the Cargo credential store.

### 4.5 Not run, and why

`make check` was not run: this milestone changes no code in `eggstack/eggsec`, and that repository's contract is defined in its own `AGENTS.md`. The equivalent Rust contract for this milestone is §4.2's 15-command block, the boundary guard, and hosted CI 5/5, all green on the exact candidate.

`cargo clippy --all-targets --features nse -- -D warnings` was not run and is not claimed: `docs/RELEASING.md` records accepted warning debt in the repository (deprecated `openssl::asn1::Asn1StringRef::as_utf8` call sites), so `-D warnings` does not pass and release qualification uses plain clippy. The historical claim to the contrary was already reconciled in the 0.2.0 release and is not re-litigated here.

## 5. Invariant review

| Invariant (plan §4) | Evidence | Result |
|---|---|---|
| ADR-0004 §8: no public API may return a raw `TcpStream`/`UdpSocket` obtained outside the broker; `tls_connect` / `tcp_connect_with_timeout` MUST NOT be reinstated, deprecated-then-restored, or reintroduced under any name | absent from the candidate, absent from the published archive, enforced by four `compile_fail` doctests, and the untruncated sweep fails closed on reintroduction (§4.3 probes 1 and 2) | holds, and enforced by the compiler rather than by convention |
| `./scripts/check-boundaries.sh` green including the residual pins (15 ungated + 7 advisory) and the M005E 97-file baseline | exit 0 on the candidate; pins byte-identical in the published archive; both negative probes still fail correctly | holds |
| M007B manifest invariants: `register_fn` matches real exported functions, manifest↔registration consistency both directions, explicit compat allowlist | boundary guard green (M007A + M007B sections unchanged in substance); M007B residual 22 and manifest counts unchanged at 159 / 84 / 41 | holds |
| No `--allow-dirty`, no `--no-verify` on any publish path | neither flag used; tree verified clean immediately before `cargo publish` | holds |
| M007C reproducible from the record: gate output archived, not summarized | `007c-semver-report.txt` (M007C) and `007c-r-semver-report.txt` (this milestone) both retained; item sets diffed identical | holds |
| Tag == published source, never moved | `v0.3.0^{}` = archive VCS sha = `16cb38ee…`; the post-release doc commit `870fca7` is on `main` only | holds |
| No `eggstack/eggsec` production-code change | Eggsec tree untouched | holds |

## 6. Failure and recovery review

- **Irreversibility of the registry.** A published version cannot be overwritten. The changelog and migration notes were completed and reviewed *before* publication precisely so that a yank is not the intended remedy for any foreseeable defect, and the 74-item completeness claim is backed by the diffed semver item set rather than by a narrative count.
- **Tag immutability under a workflow failure.** Path E2 publishes first and tags second, so a post-tag failure cannot leave a tag pointing at an unpublished commit. Had path E1 (tag-triggered) been used, a workflow failure after tag creation would have been a stop condition, not something to fix by moving the tag.
- **Tool misreporting the gate.** The most consequential failure mode available in this milestone was a semver tool that reports success for the wrong reason (a 0-check vacuous pass, or a tool too old for the rustdoc format). Both were caught by running controls and reading the check counts instead of the summary line, and both are documented in §4.1 so the next reader does not have to rediscover them.
- **A guard that passes vacuously.** The M007C audit recorded that a negative probe placed after `mod tests` passes vacuously — that exact mistake was made once and caught. §4.3 probe 1 was therefore positioned *after* the marker deliberately, with the truncation arithmetic shown, and probe 2 is a compiler check that cannot be positioned badly.
- **Restart behaviour.** If this milestone had to be re-run, the recovery point is `d4a22f1`; nothing was rewritten and no history was rebased. `0.3.0` is immutable and correct, so no corrective version plan is needed.

## 7. Migration and compatibility review

No schema, storage, protocol, or configuration migration is involved. The compatibility delta is a pure public-API break, itemised in the changelog with a per-function replacement.

- A `0.2.0` consumer must (1) add `&capability_ctx` and `&services` at 70 call sites, and (2) stop calling the two withdrawn helpers, using the broker instead. A consumer can migrate using only the changelog.
- The two new registration arguments are mandatory on purpose. Defaulting them would recreate the native-fallback path ADR-0004 forbids for automated authority claims, so this is a deliberate API decision and not an oversight.
- `0.2.0` remains published and unyanked; `0.1.0` was yanked after the advisory (see `007c-s-closure.md` §7). The `0.1.0` yank does not affect the principal consumer, which requires `^0.2.0`.
- Rollback: crates.io versions are immutable, so a defective `0.3.0` could only be yanked, and consumers pinned to `=0.3.0` would then fail resolution. The mitigation is the pre-publication completeness work in §3, not a rollback plan.
- Feature surface, MSRV (1.89), edition, and license are unchanged, so this is a pure API break with no platform or toolchain migration.

## 8. Security review

- **The release discharges a security obligation, not a semver formality.** Published 0.2.0 still exposes `helpers::tls_connect` and `helpers::tcp_connect_with_timeout`, both returning a raw `std::net::TcpStream` from an unmediated `connect_timeout`. 0.3.0 withdraws them from the public surface, and the advisory plus the `0.1.0` yank communicate that to consumers.
- **The two affected releases are `0.1.0` and `0.2.0`, not just `0.2.0`.** `src/libraries/helpers.rs` is byte-for-byte identical between the `v0.1.0` and `v0.2.0` tags and each reaches `TcpStream::connect_timeout` twice. This is why the advisory range is `>= 0.1.0, < 0.3.0`.
- **Enforcement is structural, not procedural.** The absence of the withdrawn helpers is a compiler-enforced property that fails the test run, and the truncation hole that previously let a reintroduction hide is closed by a fail-closed untruncated sweep. Neither depends on a reviewer remembering.
- **The release does not overclaim.** The changelog, the release notes, `docs/PROVIDERS.md`, and `docs/RELEASING.md` all keep four facts distinct: the withdrawn helper bypass; the broader automated-library effect gating; the 22-file residual that is *not* provider-backed; and the still-deferred DNS scope binding. Every one of them states that 0.3.0 is not complete protocol-wide scope enforcement.
- **The M007B high finding is closed in the published artifact.** `target.resolve` no longer performs an unbrokered `to_socket_addrs` under a `Pure` classification; the published source no longer contains the withdrawn direct-connect path at all.
- **Residual risks are stated, not hidden.** The 22-file residual, the ungated `DnsResolution` counter gap, and the `upnp.discover` SSDP regression are all recorded in the changelog's "Known limitations" and carried into M007D.
- **Secrets.** No credential was created, echoed, or transmitted. The crates.io token was used only via the Cargo credential store; its value does not appear in any record, log, or commit in either repository.

## 9. Documentation and operations

- `CHANGELOG.md`: new `## [0.3.0]` section plus the link reference.
- `README.md`: `0.3` install line, "Upgrading from 0.2", and the explicit not-complete-enforcement statement.
- `docs/PROVIDERS.md`: new `M007C` section (withdrawn API, why enforcement is compiler-based, what 0.3.0 still does not provide).
- `docs/RELEASING.md`: release history entry, security-disclosure note, predecessor-version disposition, two standing communication rules, and the `0.3.0` qualification-command list.
- Static guards: `scripts/check-boundaries.sh` gains the shared effect filter, the untruncated sweep, and the in-script caveat documentation. `src/libraries/helpers.rs` gains the compiler-enforced absence guard.
- Recovery instructions are unchanged and remain in `docs/RELEASING.md`; the registry-side Trusted Publisher entry is still the one open operational item and is a low residual, not a blocker.

## 10. Unresolved findings

| Severity | Finding | Impact | Required action |
|---|---|---|---|
| medium | crates.io Trusted Publisher entry still absent, so `0.3.0` used the manual-token recovery path | Every release keeps a human token step; tag-before-publish ordering is unavailable while the workflow is tag-triggered | Add the Trusted Publisher entry in crate settings; then prefer path E1 for the next release |
| medium | `broker_dns_lookup` gates `DnsResolution` on `DenyAll` only and does not evaluate per-target membership for the resolved name (inherited from `007b-closure.md` §12) | A promoted `ProviderBacked` library can resolve a name outside the approved target set under `AgentSafe`; egress/timing surface, no connection possible | M007D: bind runtime DNS policy to approved scope before re-exposure |
| medium | `0.2.0` remains published and unyanked, so the capability-bypassing helpers are still reachable by anyone who resolves `^0.2.0` | The advisory range stays live for `0.2.0` consumers | M007D adopts `0.3.0`, then the `0.2.0` yank is executed; see `007c-s-closure.md` §7 |
| low | `DnsResolution` is not charged to `network_operations`, so a permitted brokered lookup is bounded only by wall-clock/instruction budgets (pre-existing M005B behavior) | DoS-budget gap for DNS-heavy scripts under `AgentSafe`; not a scope or authority bypass | Add a DNS resolution counter when the runtime DNS policy is bound to approved scope in M007D |
| low | `upnp.discover` performs a brokered TCP connect to the SSDP multicast group instead of real UDP multicast discovery | Functional regression: SSDP discovery does not work (safe — the capability gate refuses it out of scope) | Restore real SSDP semantics with a future unconnected/multicast provider; explicitly out of scope here |
| low | Repository carries accepted clippy warning debt, so `-D warnings` does not pass | Release qualification relies on plain clippy plus the hosted matrix | Clear the debt under a separate plan; do not re-add `-D warnings` to release docs before then |
| low | Advisory CVE assignment and GitHub Advisory Database ingestion are asynchronous and were still pending at closure | The GHSA is published on the repository but not yet in the global database | Confirm `cve_id` and database ingestion; neither gates M007D |

## 11. Roadmap disposition

**Milestone closed; M007D may proceed.** The M007C-R exit gate — "Eggsec has a published immutable `0.3.0` registry artifact whose migration notes account for all 74 removals, `007c-r-closure.md` is accepted, companion `007c-s-closure.md` records the security advisory/yank disposition for both `0.1.0` and `0.2.0`" — is met in full. The companion disposition is executed in `007c-s-closure.md` §7, with the `0.2.0` yank deliberately deferred to M007D and the reason recorded.

- M007C-R: **closed** (this record).
- M007C-S: **closed** with the `0.2.0` yank carried into M007D as an inherited obligation (`007c-s-closure.md`).
- M007D: no longer blocked on the release artifact or on the security disposition. Its own inherited medium finding (`broker_dns_lookup` per-target membership) becomes its first task, and adopting `0.3.0` is what unlocks the deferred `0.2.0` yank.
- M007E: remains blocked on accepted M007D closure. Automated NSE stays quarantined.
- M007 as a whole is not closed by this record.

**GO for M007D**, with two obligations carried forward: close the `broker_dns_lookup` scope-binding gap before re-exposure, and execute the `0.2.0` yank once Eggsec requires `^0.3.0`.

## 12. Registry updates

- `plans/registry.md`: M007C-R row → **closed**; M007C-S row → **closed** (advisory published, `0.1.0` yanked, `0.2.0` deliberately not); M007D row → **ready for handoff** with the two inherited obligations; M007E row stays blocked; the `nse-runtime-extraction` subsystem row notes milestones 001-007C closed and M007D as the current handoff.
- `plans/subsystems/nse-runtime-extraction-roadmap.md`: M007C-R status → **closed** with its closure record linked; M007C-S recorded as the completed security companion; M007D status → **ready for handoff**; M007E unchanged; the §12 milestone table updated to match.
- `plans/implementation/nse-runtime-extraction/007-breaking-0-3-0-release.md`: `Status:` header → `closed (see plans/closure/nse-runtime-extraction/007c-r-closure.md)`.
- `plans/implementation/nse-runtime-extraction/007-breaking-release-security-advisory-disposition.md`: `Status:` header → closed, with the deferred `0.2.0` yank named.
- New evidence file: `plans/closure/nse-runtime-extraction/007c-r-semver-report.txt`.
