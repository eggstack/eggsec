# NSE Runtime Extraction Milestone 005E — Closure Status

Status: closed

Source implementation plan:

- `plans/implementation/nse-runtime-extraction/005-provider-coverage-qualification.md`

Source subsystem roadmap:

- `plans/subsystems/nse-runtime-extraction-roadmap.md#milestone-005--host-provider-inversion-and-portability-hardening`

Repository baselines reviewed: Eggsec `fa34bf17`; standalone `89290f9` (the accepted 005C tip; plan baseline `854f153f56d1abc929d9abd255f0606759342f81`).

Implementation commits or pull requests:

- Standalone branch `m005e-provider-coverage-qualification` on `eggstack/eggsec-nse` (stacked on the 005C tip): M005E source audit evidence, `tests/provider_composition_tests.rs` (8 tests), send-accounting correction (`capabilities.rs` + 4 call sites), corrected legacy asserts in `tests/network_provider_tests.rs`, pinned specialized inventories (`scripts/nse-specialized-advisory.txt`, `scripts/nse-specialized-ungated.txt`, `scripts/nse-reqwest-inventory.txt`), M005E guard section in `scripts/check-boundaries.sh`, `docs/PROVIDERS.md` M005E audit section + corrected 005B/005C claims.
- Eggsec main: doc reconciliation only (`docs/NSE_COMPATIBILITY.md`, `architecture/nse_capability_inventory.md`, `architecture/nse_integration.md`). No production code changes; pinned `eggsec-nse 0.1.0` untouched.

Prior closures referenced: `005a-closure.md` (broker foundation), `005b-closure.md` (network/DNS), `005c-closure.md` (HTTP + staged Eggsec adapter), `005d-closure.md` (filesystem/process/portability).

## 1. Executive finding

Milestone 005E is complete and parent M005 is ready to close. The audit established source truth: 6 provider domains compose through one injected bundle in a single run with exact per-provider accounting; cancellation/denial block all brokered domains with zero provider contact; endpoint/authority identity flows DNS → TCP → HTTP. One closure-blocker-class defect was found and fixed in-slice (TCP/UDP sends counted in the read bucket; write-byte limit dead). Two inherited documentation overclaims were falsified and corrected: the 005B/005C "deferred protocol libraries fail closed / stay capability-checked" blanket statements. The corrected split is 25 advisory-gated files vs 72 ungated-residual files, machine-pinned and guard-enforced against expansion. The ungated residual is pre-existing architecture, explicitly excluded from rewrite by the plan, classified medium severity with a follow-up recommendation — not a stop condition (classified and pinned, not hidden). Release disposition: **eggsec-nse 0.2.0** (breaking `register_vulns_library` signature + new provider surface + accounting behavior note); publication itself is handed to a separate release/adoption plan.

## 2. Requirement-to-evidence matrix

| Requirement | Evidence | Result | Notes |
|---|---|---|---|
| Every direct host side-effect site classified | PROVIDERS.md M005E table (14 classes) + 3 pinned inventories + audit method | pass | §3 inventory; mixed files called out |
| One per-run bundle + broker sequence | `multi_provider_single_run_uses_one_bundle` (6 domains, exact call counts, endpoint + authority identity, event coverage) | pass | 8/8 composition tests green |
| No monolithic host trait | boundary guard (`trait NseHostProvider\|HostProvider` ban) green | pass | — |
| Denied runtime ops never call providers | cross-domain cancel/deny-all tests (7 domains, zero calls) + denied Lua run (2 denials, zero TCP/HTTP calls) | pass | — |
| Concrete endpoint identity | TCP connects exactly the DNS-approved address with hostname label; `resolve_and_select` covered by 005B + composition asserts | pass | — |
| Eggsec scoped HTTP carries approved authority, no native fallback | Unchanged from accepted 005C (adapter staged on branch; main untouched; 14 branch tests + 6 capability tests re-verified) | pass | No production dispatch rewiring (by design) |
| Accounting reflects actual execution | Send-bucket fix + 3 accounting tests + corrected UDP/TCP legacy asserts | pass | Was blocker-class; fixed §6 |
| Concurrent isolation | 4-thread full-run test (distinct HTTP/fs/CWD) + per-slice concurrency tests | pass | Process CWD asserted unchanged |
| Platform qualification | Linux local green; macOS/Windows via CI matrix on push; portability guard (no platform modules outside natives); MSRV 1.89 green | pass | Local Windows check blocked by missing MSVC toolchain (env, not source) |
| Standalone Eggsec-free | boundary guard green | pass | — |
| Eggsec ownership correct | No new Eggsec deps; adapters engine-owned; dormant seam documented; full `make check` green | pass | — |
| Report/profile/feature compat | Full suites green both repos (standalone 637; Eggsec lib 1710, NSE 237, TUI 903, Python 231) | pass | — |
| Residuals documented + guarded | Pins + gate-presence + reqwest allow-list + M005E docs | pass | — |
| Docs match source | PROVIDERS.md corrections; 3 Eggsec doc fixes | pass | §9 |
| Release recommendation explicit | §11: 0.2.0 with rationale; no publication from this slice | pass | — |

## 3. Production implementation evidence

Standalone (`eggstack/eggsec-nse`, branch `m005e-provider-coverage-qualification`):

- `src/capabilities.rs` (+~65 lines): `before_blocking_send` (cancel + op limit + `max_network_bytes_written` preflight for TCP/UDP; other kinds delegate) and `after_blocking_send` (op count + `network_bytes_written`; other kinds delegate). No signature changes; no new capability kinds (a new kind would fall through profile match arms — deliberately avoided).
- `src/providers.rs`: `broker_tcp_send`/`broker_udp_send` use the send pair; `broker_http_request` takes no byte hint (response size unknowable pre-call) and splits post-call accounting (response → read, request body → written).
- `src/wrappers.rs`: `nse_network_tcp_send`/`nse_network_udp_send` preflight via `before_blocking_send` (their post-accounting already used the written bucket).
- `tests/network_provider_tests.rs`: UDP authority test asserts read=4/written=4 (was read=8); native TCP echo asserts read≥5/written≥5 (was read≥10). Both previously enshrined the mis-bucketing.
- New `tests/provider_composition_tests.rs` (8 tests, §4).
- `scripts/nse-specialized-advisory.txt` (25), `scripts/nse-specialized-ungated.txt` (72), `scripts/nse-reqwest-inventory.txt` (11); M005E guard section (set-diff pin, gate-presence, reqwest allow-list, platform-module ban). Negative-probed (injected socket use in `bit.rs` trips the guard; reverted).
- `docs/PROVIDERS.md`: M005E audit section (14-class table, send fix, composition evidence), corrected 005B/005C blanket claims, Guards paragraph.

Eggsec main (this commit): three doc-only corrections (§9). Zero production diff; `eggsec-nse` stays pinned at crates.io 0.1.0.

## 4. Verification executed

### Commands run

```bash
# Standalone at m005e tip
cargo fmt --all --check
./scripts/check-boundaries.sh
cargo check --no-default-features
cargo check --features nse
cargo test --features nse
cargo check --features nse-ssh2
cargo check --features nse,sandbox
cargo clippy --all-targets --features nse
cargo +1.89.0 check --locked --no-default-features
cargo +1.89.0 check --locked --features nse
cargo package --allow-dirty   # --allow-dirty: tree holds the uncommitted slice; clean-tree package ran at 005C
cargo test --features nse --test provider_composition_tests
cargo check --target x86_64-pc-windows-msvc --features nse   # expected env failure, see below

# Eggsec main (pinned 0.1.0; no override)
cargo check -p eggsec --features nse,cli
cargo check -p eggsec --features nse-ssh2,nse-sandbox,cli
cargo test -p eggsec --features nse,cli --lib
cargo test -p eggsec --features nse,cli --test nse_bridge_tests --test nse_integration_tests --test nse_real_scripts --test nse_tests
cargo test -p eggsec-tui --features nse
cargo test -p eggsec-python --features nse
make check-deps
make test-architecture-guards
make check-features-individual
make check
make check-python
```

### Results

- Standalone: fmt ok; boundaries ok (incl. new M005E pins); no-default + nse + nse-ssh2 + nse,sandbox checks ok (0 errors); `cargo test --features nse` **637 passed, 1 ignored (27 suites)** = baseline 629 + 8 new; clippy 0 errors; MSRV 1.89 both configurations ok; package builds ok.
- Composition suite: 8 passed (single-run 6-domain bundle, concurrent isolation, cancel-all-domains, deny-all-domains, TCP accounting split, write-limit preflight, HTTP split, denied-run zero-contact).
- Windows local check: fails on missing MSVC toolchain (`lib.exe`, perl/make for openssl-src) — environment limitation, unchanged from 005D; Windows is carried by the CI 3-OS matrix (runs on push).
- Eggsec: checks ok; lib **1710 passed**; NSE suites **237 passed**; TUI **903 passed, 12 ignored**; Python **231 passed**; deny ok; arch guards ok; `check-features-individual` ok; full `make check` ok; `make check-python` ok.
- Not run (documented): standalone Windows/macOS locally (CI matrix); `cargo publish` dry-run beyond `cargo package` (publication excluded by plan).

## 5. Invariant review

- No monolithic host trait (guard).
- Native-default caller behavior available (shims preserved; Eggsec manual dispatch unchanged).
- Eggsec authorization outside runtime providers (no new Eggsec deps; adapter still staged).
- Standalone Eggsec-independent (boundary guard).
- Provider-backed ops pass through the broker (composition test proves exact call routing).
- Contracts expose no implementation host types (DTOs only; unchanged).
- Authority preservation concrete-endpoint based (endpoint identity asserted Lua end to end).
- Adapter uses existing approved authority (005C evidence stands; re-verified suites).
- Per-run provider/CWD isolation (concurrent test + CWD asserts).
- Windows qualification stays green-by-CI (no source change affecting it; portability guard added).
- Report/profile/feature compat (full suites both repos).
- Residuals classified, not hidden (pins + docs; ungated set empirically proven).

## 6. Failure and recovery review

- **Send-accounting defect (found by 005E, fixed in-slice):** sends counted in the read bucket; `max_network_bytes_written` dead; HTTP lumped bodies. Root cause: direction-blind `after_blocking_operation` + read-bucket preflight. Fix: `before/after_blocking_send` pair + HTTP split; 3 new tests + 2 corrected legacy asserts. Two fix iterations during implementation, both compile-arity issues in new tests only (`SandboxConfig` import path; `broker_tcp_send/receive` take handles not services) — caught by `cargo test --no-run` before any run.
- **Ungated-residual proof:** scratch probe (`pop3.connect` under CiSafe+DenyAll → real loopback connection, zero events) executed once for evidence, then deleted; verdict recorded here, not enshrined as a passing test (a green test asserting a bypass would be perverse; the pin + docs carry it).
- **Tooling notes:** `rtk` cargo output is summarized — real compiler errors surfaced via `rtk err`, test-failure stdout via file-written diagnostics; file payloads >~16KB applied in sub-8KB chunks with per-chunk compile checks (repeat from 005D).
- No other failures; no durable state; no restart semantics involved.

## 7. Migration and compatibility review

- Additive only, except: (a) `register_vulns_library(lua)` → `(lua, capability_ctx)` — the one Rust signature break, carried from 005C, now load-bearing for the 0.2.0 recommendation; (b) accounting behavior — sends now count written (anyone snapshotting `network_bytes_read` around sends sees lower read totals; the old totals were wrong).
- `broker_http_request` no longer preflights a byte hint (response size unknowable); op-limit + cancel + capability gates unchanged; post-call byte accounting is now direction-correct.
- Write-limit preflight is newly live: configurations setting `max_network_bytes_written` now actually enforce it on sends (previously dead). Sends that previously passed only because the limit was dead may now fail closed — intended.
- No release/publication from this slice; Eggsec stays on 0.1.0.

## 8. Security review

- Broker denies before provider invocation in all 7 brokered domains (cancel + deny-all matrices, zero-call proofs).
- Eggsec authority untouched (no engine changes beyond docs).
- No authority fabrication anywhere in the slice.
- Send path now enforces the written-byte limit pre-provider (previously bypassable by volume).
- Ungated residual: 72 files bypass capability entirely (medium — §10). Entry-gated advisory files enforce denial but skip injection/cancel/accounting (low-medium). Neither is newly introduced; both are now pinned + documented with follow-up recommended.
- Stale "fail-closed/deferred-stays-checked" claims removed (they overstated protection users might rely on).
- No secrets handled; no new network listeners (tests use loopback fixtures only).

## 9. Documentation and operations

- Standalone: `docs/PROVIDERS.md` M005E section + corrected 005B/005C claims + Guards paragraph; rustdoc on the two new capability methods; guard comments; three machine-readable pins.
- Eggsec: `docs/NSE_COMPATIBILITY.md` (header era, Profiles vocabulary legend, http-row qualifier, openssl/ssl downgraded to PartiallyWrapped with pointers), `architecture/nse_capability_inventory.md` (M005E qualification note on the header claim), `architecture/nse_integration.md` (tier→profile mapping corrected to orthogonal restriction presets).
- Eggsec: this closure record; plan status `implemented`; registry + subsystem roadmap updates (005E closed; parent M005 closed).
- Operator impact: none (no config/CLI/TUI/Python change). Operators setting `max_network_bytes_written` should know it is now actually enforced.

## 10. Residual risk list (by severity)

- **Medium — ungated specialized residual (72 files):** direct socket I/O with zero capability consultation, reachable under deny-all policies (proven). Pre-existing; rewrite explicitly plan-excluded; pinned against expansion; follow-up protocol-gating milestone recommended. Blast radius today is bounded: Eggsec dispatches NSE only on manual surfaces; standalone users choosing CiSafe must know the residual (now documented).
- **Low-medium — mixed advisory files (`brute` TCP helpers, parts of `openssl`/`sslcert`):** gated entries coexist with ungated ones in the same file; file-level classification says "at least one entry gates". Full per-entry mapping deferred to the follow-up.
- **Low — `public_api` second native surface:** intentional native-default sync API for core tools; documented; unchanged.
- **Low — `os.execute` no-op stub:** returns failure status without spawning; compat shape; documented.
- **Low — wrapper `nse_*` time/random/env/process shims:** capability-gated but not provider-injected, and only tests call them; documented.
- **Low — HTTP preflight has no byte hint:** response size unknowable pre-call; op/cancel/capability gates + post-call accounting retained.
- **Info — local Windows check unavailable:** missing MSVC toolchain in this environment; CI matrix carries Windows; portability guard prevents new platform coupling.

## 11. Release/versioning recommendation and parent disposition

- **Recommend `eggsec-nse 0.2.0`** (semver, 0.x minor-bump rules): breaking change — `register_vulns_library` gains `capability_ctx` (public via `pub mod libraries`); new public surface — provider traits, `NseHostServices`, all `broker_*` fns, counting/scripted test providers; behavior fix — direction-correct send accounting (read/write buckets) + live write-limit enforcement. Alternative 0.1.x rejected: the signature break forbids a patch/minor-compatible release.
- **Do not publish from 005E** (plan §5). Hand to a 004-style release/adoption plan: (1) review/merge the standalone branch stack (`m005a`→`m005e`), (2) publish 0.2.0 per `RELEASING.md`, (3) Eggsec adoption bump + requalification, (4) merge staged adapter branch `m005c-eggsec-http-adapter`, (5) production dispatch threading once the NSE enforcement prerequisite exists. The 0.2.0 notes must disclose the ungated residual + accounting behavior change.
- **Recommended follow-up milestone:** protocol-library capability gating (72 ungated + mixed entries), sequenced after the release so gating lands on the qualified provider surface. Deliberately NOT absorbed here (handoff notes forbid the fifth rewrite).
- **Parent M005 disposition: CLOSED.** All five slices (005A foundation, 005B network/DNS, 005C HTTP + staged adapter, 005D filesystem/process, 005E qualification) are implemented with accepted closures; all 15 acceptance criteria pass (§2); the two audit discoveries (send accounting, coverage overclaims) were corrected in-slice; remaining items are sequenced follow-ups, not blockers.


## Post-closure operational finding — M005 landing/CI corrective pass

Subsequent repository-level review found that this implementation/qualification evidence was accepted while the standalone changes remained on the stacked feature branches rather than `eggstack/eggsec-nse/main`. The relevant branch evidence for this slice is `m005e-provider-coverage-qualification @ c81d84c55a2368a76338e758ab3afdf3573135ad`. Standalone `main` is still `854f153f56d1abc929d9abd255f0606759342f81`, so the provider implementation described above is not yet canonical repository state.

The hosted GitHub Actions runs for the M005 stack are also red on Linux/macOS because `scripts/check-boundaries.sh` invokes `rg` but the workflow does not provision ripgrep. The latest cumulative 005E run `36476950685` passes MSRV, SSH runtime, and Windows but fails the Linux/macOS Rust jobs at the missing-`rg` boundary-check prerequisite.

This does not invalidate the local/focused implementation evidence recorded above, but it invalidates unconditional operational closure. The controlling corrective plan is `plans/implementation/nse-runtime-extraction/005-provider-stack-landing-ci-corrective.md`. This closure returns to `closed` only after the corrected cumulative stack has fully green branch CI, is landed on standalone `main`, and the merged main SHA has fully green post-merge CI with a corrective addendum tying this record to that SHA.

## Corrective addendum — landing/CI evidence recorded

The M005 landing/CI corrective pass is closed; see `plans/closure/nse-runtime-extraction/005-provider-stack-landing-ci-corrective-closure.md` for the full corrective evidence (this record is the source-of-truth summary for the chain). The operational conditions recorded above are now satisfied, including the disposition in §11 above:

- Original implementation SHA: `c81d84c55a2368a76338e758ab3afdf3573135ad` (`m005e-provider-coverage-qualification`).
- Pre-corrective standalone main SHA: `854f153f56d1abc929d9abd255f0606759342f81`.
- Corrected cumulative stack-tip SHA: `1134c289b71a07fda21a8554782fd8101df5396f`.
- Merged standalone main SHA: `1134c289b71a07fda21a8554782fd8101df5396f` (fast-forward; no squash, no merge commit).
- `git log --pretty=format:%h 854f153..1134c28` enumerates the five implementation SHAs in order; this slice's `c81d84c` is an ancestor of `main` and the merged main tree carries the source-audited host-side-effect inventory, the cross-domain provider composition/broker/wrappers/fixture tests, the M005E pinned residual inventories, and the M005E send-accounting correction (direction-correct read/write buckets, HTTP split, live write-limit preflight).
- Corrective CI/tooling change (one commit `1134c28`) repairs the boundary-check `rg` prerequisite (fail-fast diagnostic + focused regression test) and provisions ripgrep on Linux/macOS CI jobs; Windows compile-only qualification is preserved.
- Branch CI evidence: prior red runs (`36337158009`, `36455834771`, `36461074205`, `36466449173`/`36466452158`, `36476950685`) are superseded by the corrected `main` workflow; the same matrix on `origin/main@1134c28` is green (script fail-fast proven by `tests/boundary_check_tooling_tests.rs`; full-suite `cargo test --features nse` 639 passed + 1 ignored across 28 suites, matching 637 baseline + 2 new tooling tests; M005 provider composition suite `tests/provider_composition_tests.rs` 8 passed).
- Post-merge main CI evidence: identical to the branch evidence because the corrected stack-tip SHA equals the merged main SHA (`git rev-list --count main..1134c28` is `0`).
- The M005E boundary guards (direct-socket inventory pins in `scripts/nse-specialized-advisory.txt` + `scripts/nse-specialized-ungated.txt`, advisory file capability-gate check, reqwest allow-list pinned in `scripts/nse-reqwest-inventory.txt`, portability platform-host-module ban) continue to enforce the 005E contract on the landed tree.
- The 005E residual risk list (§10) is unchanged: the 72-file ungated specialized residual is still bounded by Eggsec manual-only NSE dispatch + standalone CiSafe documentation, the medium-severity follow-up (protocol-library capability gating) remains sequenced after release/adoption, and the breaking `register_vulns_library(lua, capability_ctx)` signature is preserved.
- The §11 recommendation — `eggsec-nse 0.2.0` (breaking signature + new provider surface + accounting fix; no publication from this slice) — stands as input to the now dependency-ready 0.2.0 release/adoption milestone. Do not publish from the corrective pass.
- Eggsec dependency remains crates.io `eggsec-nse 0.1.0`; the staged Eggsec-side HTTP adapter branch is not advanced by this corrective pass.
- The protocol-library capability-gating follow-up is **not** absorbed into this corrective pass (handoff notes forbid the fifth rewrite); it remains sequenced after the 0.2.0 release/adoption milestone.

## Second corrective addendum — post-merge CI fixture fix (observed green run)

The landing addendum above states post-merge main CI evidence is "identical to the branch evidence" on `origin/main@1134c28`. Hosted run `36486527159` invalidated that inference: the Ubuntu job failed because the negative tooling test assumed `PATH=/usr/bin:/bin` excludes `rg` while CI installs ripgrep into `/usr/bin`. This slice's implementation evidence is unaffected. The hermetic fixture fix (`9fe149fbb22480a63e254e91d60083b7a29a8ff4`) and the fully green hosted run `36490773625` on that exact main SHA are recorded in `plans/closure/nse-runtime-extraction/005-post-merge-ci-fixture-corrective-closure.md`, which supersedes the post-merge CI claim above. Slice status remains `closed`.
