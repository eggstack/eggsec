# NSE Runtime Extraction Milestone 007C-S — Closure Status

Status: closed (draft preparation and execution both complete — see §13 addendum; the `0.2.0` yank is deliberately deferred to M007D with the reason recorded)

Source implementation plan:

- `plans/implementation/nse-runtime-extraction/007-breaking-release-security-advisory-disposition.md` (M007C-S)

Source subsystem roadmap:

- `plans/subsystems/nse-runtime-extraction-roadmap.md#milestone-007--protocol-library-gating-and-controlled-automated-activation`

Applicable ADRs:

- `plans/adrs/ADR-0004-nse-automated-activation-boundary.md`
- `plans/adrs/ADR-0003-nse-host-provider-boundary.md`

Planning baseline: Eggsec `c5abeb26`

Implementation repository: `eggstack/eggsec-nse`

Repository baseline reviewed: `d4a22f1dbe56f4ccfb17b2a8135aae8395f44f19` (standalone `main`, 2026-10-01)

Implementation commits or pull requests: **none in either repository.** M007C-S is a security-communication/disposition slice. All completable work (source verification, draft advisory preparation, deferred yank/no-yank decisions with rationale) is recorded here. Publication and yank execution are blocked on the verified `0.3.0` artifact owned by M007C-R, which has not published.

## 1. Executive finding

**Draft preparation is complete and source-verified; publication and yank execution are blocked on the verified `0.3.0` artifact, which does not exist.**

The plan's own sequencing (§4, §5, §10) forbids publishing the advisory or yanking predecessors before the fixed artifact is usable. Registry state on 2026-10-01: `eggsec-nse` versions `[0.1.0, 0.2.0]`, neither yanked; `0.3.0` absent from crates.io, no `v0.3.0` tag in `eggstack/eggsec-nse`, no published GitHub Security Advisory (`gh api repos/eggstack/eggsec-nse/security-advisories` → `[]`). M007C-R has not published, so there is no fixed artifact to bind, disclose, or resolve against.

What this record completes:

- source proof that the candidate advisory range is `>= 0.1.0, < 0.3.0` (both immutable releases expose the bypass API; §3);
- pre-publication verification (§8 first block): version-to-version helper comparison, candidate absence check, guard negative probe, semver-report reconciliation — all pass;
- a publication-ready draft advisory with threat-model-disciplined impact language (§6);
- explicit yank/no-yank decisions for both predecessors: **deferred until verified `0.3.0`, with rationale** — not silent (§7);
- CVE decision: request at publication time; not required for M007D (§8);
- communication-surface reconciliation template distinguishing the four security facts (§9).

What remains (named, owned, gated):

1. M007C-R publishes verified `0.3.0` → then create/publish the GHSA from the §6 draft with fixed `0.3.0`.
2. Then execute the §7 yank order for `0.1.0` + `0.2.0` (preferred: yank both, subject to the re-checks named in §7).
3. Then verify registry/advisory/changelog consistency (§8 second block).

**M007D remains blocked.** Per plan §9 criterion 12, M007D unblocks only after the fixed `0.3.0` artifact exists *and* the disposition record is complete. Neither condition is met by execution yet, though this record completes the preparatory half.

## 2. Requirement-to-evidence matrix

| Requirement (plan §) | Evidence | Result | Notes |
|---|---|---|---|
| §9.1 affected range `>= 0.1.0, < 0.3.0` | §3 tag/crate comparison | **pass** | both releases expose all 4 helpers at identical paths |
| §9.2 advisory names the two direct-connect helpers | §6 draft | **pass (draft)** | `helpers::tls_connect`, `helpers::tcp_connect_with_timeout`, plus the 2 pure withdrawn helpers for migration completeness |
| §9.3 impact language distinguishes boundary bypass from remote exploitability | §6 draft | **pass (draft)** | explicit non-claims carried over from plan §3 |
| §9.4 fixed version not published in advisory until `0.3.0` available | no advisory published; draft marks fixed as pending | **pass** | verified: advisories endpoint returns `[]` |
| §9.5 advisory published after fixed artifact verified, unless concrete no-publish reason recorded | no `0.3.0` artifact exists (§5) | **blocked with reason** | §5 records the reason; this is the plan-sanctioned alternative |
| §9.6 CVE explicitly decided, not required for M007D | §8 | **pass** | request at publication; M007D not gated on assignment |
| §9.7 `0.1.0` explicit yank/no-yank decision + rationale | §7 | **pass (deferred decision)** | yank deferred until verified `0.3.0`; rationale recorded |
| §9.8 `0.2.0` explicit yank/no-yank decision + rationale | §7 | **pass (deferred decision)** | same |
| §9.9 if yanked, registry + fresh-resolution verified | nothing yanked (§5 yank order forbids it pre-fix) | **not run (blocked)** | procedure specified in §7 for execution time |
| §9.10 no overclaim of universal provider coverage | §6 draft + §9 template | **pass (draft)** | four-way distinction required in all surfaces |
| §9.11 GHSA identifier or explicit no-advisory rationale | §5 + §6 | **pass (rationale)** | no GHSA yet; reason: fixed artifact absent |
| §9.12 M007D unblocked only after fixed artifact + disposition | §10 | **fail (still blocked)** | correct per plan; M007D stays gated |
| §8 pre-pub: compare `v0.1.0`/`v0.2.0` helper source | §3 | **pass** | byte-identical bypass API in both |
| §8 pre-pub: both direct-connect fns absent from candidate | §3 | **pass** | `d4a22f1` helpers.rs has neither; remaining `connect_timeout` hits are reqwest builder timeouts |
| §8 pre-pub: boundary guards fail if either helper restored | §4 probe | **pass** | exit 1, `M007B violation`, names `helpers.rs`; green after revert |
| §8 pre-pub: semver report still contains helper removals | §4 | **pass** | 4/4 helper entries present in archived 74-item report |
| §8 post-pub: advisory range / fixed version / refs / yank metadata | — | **not run (blocked)** | requires publication first |
| §6 surface reconciliation (GHSA/release notes/CHANGELOG/PROVIDERS/RELEASING) | §9 | **pass (template + baseline)** | standalone `CHANGELOG.md` has no `0.3.0` section yet (nothing to contradict); reconciliation procedure + message template staged for publication |

## 3. Production implementation evidence

No production, test, guard, or documentation file in `eggstack/eggsec` or `eggstack/eggsec-nse` was modified. All repository interaction was read-only auditing plus one temporary negative probe in a scratch clone (`/tmp`, reverted, `git status` clean) and crates.io/GitHub read APIs. `eggstack/eggsec` working tree holds only this planning slice.

This is expected: M007C-S owns no code change. Its deliverables are verification evidence, a draft advisory, deferred disposition decisions, and this record — all present.

## 4. Verification executed

### Commands run

```bash
# registry state (sparse-index cache, yank flags)
python3 -c "..." ~/.cargo/registry/index/.../.cache/eg/gs/eggsec-nse
# → 0.1.0 yanked=False; 0.2.0 yanked=False; no 0.3.0 entry
cargo info eggsec-nse            # max version 0.2.0
git ls-remote https://github.com/eggstack/eggsec-nse.git
# → HEAD/main d4a22f1; tags v0.1.0 / v0.2.0 only; no v0.3.0
gh api repos/eggstack/eggsec-nse/security-advisories   # → []

# source evidence (§3)
# 0.1.0 crate download + 0.2.0 registry src: identical 4-function bypass API
# (tls_connect:61, make_addr:75, tcp_connect_with_timeout:79, parse_socket_addr:96;
#  TcpStream::connect_timeout at :70 and :89 in both)
git clone --depth 1 --branch main eggstack/eggsec-nse /tmp/...  # d4a22f1
grep -n "pub fn (tls_connect|tcp_connect_with_timeout|make_addr|parse_socket_addr)" \
  src/libraries/helpers.rs      # → no match (all four absent)
git show v0.1.0:src/libraries/helpers.rs | grep ...  # 4 hits
git show v0.2.0:src/libraries/helpers.rs | grep ...  # 4 hits

# guard probe (§4): insert tcp_connect_with_timeout in production scope
./scripts/check-boundaries.sh   # → exit 1, M007B violation, names helpers.rs
# revert; ./scripts/check-boundaries.sh → exit 0 green

# semver report reconciliation
grep -c "function eggsec_nse" 007c-semver-report.txt        # 74
grep -n "helpers::" 007c-semver-report.txt                 # 4/4 present

# disposition inputs
grep -n "## [" CHANGELOG.md                                   # latest 0.2.0, no 0.3.0 section
grep -n "register_.*_library(lua)" README.md docs/PROVIDERS.md  # no stale examples
gh api "search/code?q=eggsec-nse+in:file+filename:Cargo.toml"   # 4 hits, all author forks
grep -rn "eggsec-nse" crates/eggsec/Cargo.toml               # eggsec-nse = { version = "0.2.0" }
```

### Results

- Registry/tag/advisory state: confirmed `0.3.0` absent everywhere, no advisory published, neither predecessor yanked. No registry mutation performed.
- §8 pre-publication block: 4/4 pass (comparison, absence, guard probe, semver report).
- §8 post-publication block: not run — correctly blocked, no fixed artifact.
- Eggsec-side consumption unchanged: `crates/eggsec/Cargo.toml` pins registry `0.2.0`, no override (consistent with M006B quarantine; adoption is M007D-owned).

### Not run, and why

Advisory create/publish, CVE request, `cargo yank`, yank-metadata verification, fresh-vs-locked resolution checks, and release-notes/changelog edits were all **not run**. The plan forbids them before verified `0.3.0` (§4 work package A gating, §5 yank order step 1, §10 stop conditions), and `0.3.0` does not exist. Running any of them now would violate the plan's own stop conditions.

`make check` was not run: no production code, test, guard, or manifest in this repository was touched (plans-only slice). The relevant verification for this slice is the evidence above plus the architecture-guard pass recorded in §9.

## 5. Invariant review

| Invariant | Evidence | Result |
|---|---|---|
| ADR-0004 §8: no network effect without a capability decision | the withdrawn helpers enforced it; draft (§6) states removal as the fix; guard probe proves restoration is detected | holds; communication staged |
| Fixed version never advertised before it exists | advisories endpoint `[]`; draft marks fixed `0.3.0 (pending verification)` | holds |
| No version silently left without disposition | §7 records explicit deferred decisions for both `0.1.0` and `0.2.0` | holds |
| Advisory range covers every known-vulnerable release | §3 proves `v0.1.0` + `v0.2.0`; draft range `>= 0.1.0, < 0.3.0` | holds |
| No Eggsec production change in this slice | `git status` (only `plans/`); consumption still registry `0.2.0` | holds |
| Secrets safety | crates.io token presence not inspected here; `gh` used for read APIs only; no credential appears in any record | holds |

## 6. Draft advisory (work package A output — publication-ready pending `0.3.0`)

The following is the prepared draft for the repository security advisory on `eggstack/eggsec-nse`. It MUST NOT be published until M007C-R proves crates.io `0.3.0` resolution, tag/source/archive identity, and scratch-consumer builds; at that point replace `(pending)` markers with verified values.

```text
Package: eggsec-nse (Rust / crates.io)
Ecosystem: Rust
Affected: >= 0.1.0, < 0.3.0
Fixed: 0.3.0 (pending — do not publish this advisory until verified)

Title: Public TCP helper API bypasses runtime capability/provider boundary

Summary:
  eggsec-nse versions before 0.3.0 expose two public helper functions,
  helpers::tcp_connect_with_timeout and helpers::tls_connect, that open
  raw TCP connections via std::net::TcpStream::connect_timeout directly.
  These helpers do not receive NseCapabilityContext, do not execute
  through NseHostServices or the provider broker, and therefore any
  consumer invoking them bypasses the runtime's capability decision,
  cancellation, accounting, and provider-selection boundary for that
  connection. Version 0.3.0 removes both functions (with no replacement:
  use the broker APIs broker_tcp_connect / broker_dns_lookup) along with
  the pure helpers make_addr and parse_socket_addr.

Impact (read exactly — do not overstate):
  Affected parties are embedders that relied on the crate's
  capability/provider boundary while exposing or internally invoking
  these public helpers. This advisory does NOT claim that arbitrary
  remote attackers can call these Rust functions, that every Lua/NSE
  script can reach them directly, that every 0.1.0/0.2.0 consumer is
  remotely exploitable, or that Eggsec's quarantined automated NSE path
  exposed these functions as a supported operation.

Severity: to be set at publication from the above threat model.
  Do not derive severity from the word "bypass" alone. The defect
  requires the consumer to call the public helper; it is a
  library-boundary bypass, not a remote-execution vulnerability.
  (No CVSS vector is asserted in advance; GitHub severity is assigned
  at advisory creation from this rationale.)

Vulnerable functions:
  - eggsec_nse::libraries::helpers::tls_connect
  - eggsec_nse::libraries::helpers::tcp_connect_with_timeout
Withdrawn without replacement; pure helpers make_addr / parse_socket_addr
  likewise removed (trivially inlinable; no crate-internal caller remains).

Migration:
  Move connection work to broker/provider APIs so operations are
  capability-checked and accounted. Do not recreate raw-socket helpers.
  Registration entry points also changed in 0.3.0:
  register_x_library(lua) → register_x_library_with_services(lua,
  &capability_ctx, &services); see the 0.3.0 CHANGELOG for the full
  74-item migration mapping.

Scope note (do not collapse):
  1. This advisory covers only the public Rust helper bypass fixed by
     removing the two direct-connect helpers.
  2. The broader M007 automated-library effect gating is a separate
     hardening change riding the same release.
  3. 22 specialized direct-I/O / manual-only residuals remain
     (15 ungated + 7 advisory) and are NOT provider-backed.
  4. DNS scope binding (M007D follow-up) is still deferred: 0.3.0 does
     NOT provide universal NSE protocol scope enforcement.

References (to bind at publication):
  - v0.3.0 release / migration notes (pending)
  - CHANGELOG.md [0.3.0] (pending)
  - docs/PROVIDERS.md (pending reconciliation)
```

## 7. Predecessor-version yank disposition

Cargo yanks do not delete contents; they prevent new resolution from selecting the yanked version while existing lockfiles keep working.

### `0.1.0` — decision: DEFER yank until verified `0.3.0` (explicit, not silent)

Rationale: §3 proves `0.1.0` carries the identical bypass API, so the preferred rule (plan §5) will almost certainly resolve to **yank** once `0.3.0` is verified — but yanking now, with no fixed artifact available, would strand fresh resolutions with no safe upgrade target and violate the plan's yank order (step 1: verify `0.3.0` availability first). No downstream was found requiring fresh `0.1` resolution as its only migration path (GitHub code search: only author-fork hits; weak evidence — re-check at execution time per plan §5).

### `0.2.0` — decision: DEFER yank until verified `0.3.0` (explicit, not silent)

Rationale: identical to `0.1.0`. `0.2.0` is additionally the version Eggsec itself currently consumes (`crates/eggsec/Cargo.toml`), so yanking before M007D adopts `0.3.0` would strand the principal consumer — exactly the plan §10 stop condition ("a yank would strand the principal consumer before M007D can adopt 0.3.0"). Preferred post-`0.3.0` outcome remains **yank both**, subject to the execution-time dependent re-check.

### Execution procedure (for the unblocking pass, after verified `0.3.0`)

1. Verify `0.3.0` registry availability + scratch consumption (M007C-R evidence).
2. Publish/update the advisory with fixed `0.3.0` (§6 draft).
3. Yank `0.1.0` and `0.2.0`; verify registry metadata reflects the yank.
4. Verify an existing lockfile can still resolve/build the yanked version (compatibility property).
5. Verify a fresh unconstrained resolution does not select a yanked version.
6. Re-check dependents immediately before yanking; if either release must stay unyanked, record the compatibility reason, keep the advisory range unchanged, and state the version remains affected.

## 8. Security review

- The defect is real and range-correct: both immutable public releases expose the raw-connect API at identical paths with direct `TcpStream::connect_timeout`; the M007 tree removes all four functions; the residual `connect_timeout` hits in the current `helpers.rs` are `reqwest` builder timeouts, not raw sockets.
- Threat-model discipline (§3 of plan) is preserved in the draft: library-boundary bypass framing, four explicit non-claims, no pre-asserted CVSS, no remote-exploitability claim.
- CVE decision: **request a CVE via the GitHub advisory flow at publication time; M007D is explicitly NOT gated on CVE assignment** (plan §4: "A CVE is optional; do not make M007D depend on CVE assignment"). No CVE requested now — there is no published advisory to attach it to.
- Remaining limitations carried forward: 22-file specialized residual (not provider-backed); `broker_dns_lookup` per-target membership deferred to M007D (inherited medium finding from `007b-closure.md` §12); `0.2.0` (and `0.1.0`) remain exploitable-as-described until yanked and consumers migrate.
- No active-exploitation evidence was found or claimed; therefore no emergency-disclosure override applies, and the normal follow-the-fix sequencing stands.

## 9. Documentation and operations

- Updated by this slice: this closure record; `plans/registry.md` (M007C-S disposition); source plan status header (draft-complete → blocked-pending-`0.3.0`). No production docs touched — correct, since the surfaces to reconcile (`CHANGELOG.md`, `docs/PROVIDERS.md`, `docs/RELEASING.md`, `v0.3.0` release notes) all live in `eggstack/eggsec-nse` and the `0.3.0` sections do not exist yet.
- Baseline checked: standalone `CHANGELOG.md` latest section is `[0.2.0]` (nothing to contradict); `README.md`/`docs/PROVIDERS.md` contain no stale `register_*_library(lua)` examples.
- Static guards: `bash scripts/check-architecture-guards.sh` (this repo) — pass (see §4 commands; plans-only change introduces no new guard surface). `rg` prerequisite satisfied.
- Reconciliation template for publication time (§6 draft "Scope note" + plan §6 surface list): the publisher MUST restate the same four-way distinction (helper bypass / effect gating / 22 residuals / deferred DNS binding) across GHSA, `v0.3.0` release notes, `CHANGELOG.md`, `docs/PROVIDERS.md`, and `docs/RELEASING.md`.

## 10. Unresolved findings

| Severity | Finding | Impact | Required action |
|---|---|---|---|
| **high** | No verified `0.3.0` artifact exists, so the advisory cannot be published and predecessors cannot be yanked | Security meaning of M007 hardening is undisclosed; `0.1.0`/`0.2.0` remain resolvable with the bypass API | M007C-R: publish verified `0.3.0`; then execute §6–§7 of this record |
| medium | Draft severity/CVSS is rationale-only, no vector asserted | Cannot pre-file advisory-database severity | Assign at publication from §6 rationale |
| medium | Dependent re-check is weak (author-fork hits only) | Yank could strand an unknown downstream | Re-run code-search + dependents check immediately before yanking |
| low | CVE unattended until publication | None for M007D by design | Request at advisory publication |
| low | `nse_production_code()` post-`mod tests` truncation caveat (from `007c-closure.md` §10) | Guard could miss a helper restored after a test module | M007C-R work package C owns the hardening; this slice's probe placed the primitive pre-`mod tests` so the verification is meaningful |

## 11. Roadmap disposition

**Milestone companion slice blocked with draft complete.** Specifically:

- M007C-S draft preparation: **complete** (this record is the evidence).
- M007C-S publication + yank execution: **blocked on verified crates.io `0.3.0`** (owned by M007C-R, currently the primary handoff and still unpublished).
- **M007D remains blocked** on accepted `007c-r-closure.md` + execution of this disposition (`007c-s` follow-up pass after `0.3.0`). This record does NOT unblock M007D.
- M007E remains blocked on M007D. Automated NSE remains quarantined.
- No milestone is closed by this record beyond the M007C-S draft scope; no roadmap exit gate is claimed as met.

## 12. Registry updates

- `plans/registry.md`: M007C-S row → **blocked (draft preparation complete and source-verified; advisory publication + `0.1.0`/`0.2.0` yank execution gated on verified `0.3.0`)**; M007D/M007E remain blocked; M007C-R remains the primary current handoff.
- `plans/subsystems/nse-runtime-extraction-roadmap.md`: no structural change required (M007C-R section already names this companion and its gating; the milestone table already reflects M007C-S publish/yank gating on verified `0.3.0`).
- `plans/implementation/nse-runtime-extraction/007-breaking-release-security-advisory-disposition.md`: `Status:` header → `blocked (draft preparation complete — see 007c-s-closure.md; publication/yank pending verified 0.3.0)`.
- Follow-up: after M007C-R publishes verified `0.3.0`, open a short execution pass against this plan (publish advisory → yank → verify) and append its evidence as a dated addendum to this record or a `007c-s-exec-closure.md`, per the closure rule keeping records immutable except for factual corrections.

---

## 13. Execution addendum — 2026-10-02

The gate that produced §1's "blocked" verdict has cleared. `eggsec-nse 0.3.0` was published, verified, and bound to the fixed version required by §6, and the §7 disposition has been executed. This addendum is the dated execution record §12 asked for; §§1–12 are left as written, with only the `Status:` line above updated to point here.

Source of the fixed artifact: `plans/closure/nse-runtime-extraction/007c-r-closure.md` (M007C-R, closed) — candidate `eggstack/eggsec-nse@16cb38ee78f680bd07739dc3fc1ef776c9f19c9c`, published archive VCS identity equal to that SHA, immutable tag `v0.3.0`.

### 13.1 Fixed-artifact verification (plan §7 execution step 1)

| Check | Evidence | Result |
|---|---|---|
| `0.3.0` absent before publication | crates.io `max_version` 0.2.0; `git ls-remote --tags` v0.1.0/v0.2.0 only | pass |
| Registry availability + consumption | three registry-only scratch consumers resolve `=0.3.0` for `nse`, `nse-ssh2`, `nse,sandbox` (`source = registry+…`, no path/git fallback) and build; one runs end to end | pass |
| archive VCS identity == candidate | published `.cargo_vcs_info.json` `sha1 = 16cb38ee…`; archive sha256 `4497c2d1…` | pass |
| fixed artifact really lacks the defect | 0 occurrences of the four withdrawn helper definitions in the published source; 6 `compile_fail` markers present; residual pins byte-identical | pass |

### 13.2 Pre-publication verification (plan §8 first block) — re-run against the real candidate

| Check | Result |
|---|---|
| `v0.1.0` vs `v0.2.0` helper source compared | **byte-for-byte identical** (`diff` empty), and both reach `TcpStream::connect_timeout` twice |
| both direct-connect functions absent from the 0.3.0 candidate | absent in the working tree and in the published archive |
| boundary guards fail if either helper is restored | two independent layers, both probed: the untruncated sweep fails closed (`exit 1`, `M007C-R violation`) for a primitive placed *after* `mod tests`, and the `compile_fail` doctests fail `cargo test --features nse --doc` for a restored `tcp_connect_with_timeout` |
| semver report still contains the helper removals | 4/4 present in the 74-item set; full item set diffed identical to the archived M007C gate |

The third row is materially stronger than the §1 record's probe. §1 proved the *text* guard fires for a primitive placed in production scope; M007C-R found that `nse_production_code()` truncates at the first `mod tests` and that `helpers.rs` itself carries 45 production lines below its test module, so a text-only probe would have been a weak proof. The shipped enforcement is now compiler-based, and the probe was deliberately positioned after the marker.

### 13.3 Advisory publication (plan §7 execution step 2, §4 work package C)

```text
GHSA-w2g3-v83j-frp2
repository:      eggstack/eggsec-nse
ecosystem:       rust (crates.io)
package:         eggsec-nse
affected range:  >= 0.1.0, < 0.3.0
patched:         0.3.0
severity:        medium
state:           published
published_at:    2026-10-02T16:00:48Z
url:             https://github.com/eggstack/eggsec-nse/security/advisories/GHSA-w2g3-v83j-frp2
```

Post-publication verification (plan §8 second block): the published range is exactly `>= 0.1.0, < 0.3.0` and therefore covers both released vulnerable versions; the fixed version is exactly `0.3.0`; the advisory references the `v0.3.0` release, the `v0.3.0` CHANGELOG, and `docs/PROVIDERS.md`. This satisfies plan §9 criteria 1, 2, 3, 4, 5, 10, and 11.

Severity was assigned from §3's threat model rather than from the word "bypass", as §6 required: the defect requires the consumer to call the public helper, so it is scored as local attack vector, low complexity, no privileges, no user interaction, unchanged scope, low confidentiality and low integrity impact, no availability impact — CVSS 3.1 base ≈ 5.1, i.e. medium. The advisory body carries the four explicit non-claims verbatim from plan §3 and the four-way scope note from §6.

**CVE decision: request via the GitHub advisory flow at publication time; M007D is explicitly not gated on assignment** (plan §4). The advisory was published through that flow, which is the request. Assignment is asynchronous: at closure `cve_id` was still `null` and the advisory had not yet been ingested into the global GitHub Advisory Database (`github.com/advisories/GHSA-w2g3-v83j-frp2` → 404). Neither fact gates M007D, and neither is claimed as done.

### 13.4 Dependents re-check (plan §5, immediately before the yank)

| Source | Result |
|---|---|
| GitHub code search, `eggsec-nse` in `filename:Cargo.toml` | 4 hits, all in `dbowm91/slapper` — the author's own monorepo, which consumes the crate as a **workspace path member** (`"crates/eggsec-nse"`), so a crates.io yank cannot affect it |
| crates.io dependents endpoint | not available for this crate (404) — recorded as absent evidence, not as "no dependents" |
| known consumers | one registry consumer: `eggstack/eggsec`, `crates/eggsec/Cargo.toml` requires `^0.2.0` |

This confirms §1's "weak evidence" note: there is still no external downstream, and the only registry consumer is the principal consumer. Evidence is no weaker than it was at draft time; it is now positively identified rather than merely unobserved.

### 13.5 Yank disposition — the outcome differs from the draft's preferred outcome

The draft deferred both versions pending `0.3.0`, and recorded "yank both" as the preferred post-`0.3.0` outcome. On execution, **that preference does not hold for `0.2.0`**, and the divergence is recorded rather than papered over.

**`0.1.0` — YANKED.** All three plan §5 conditions hold: the advisory concludes the raw-connect API violates a security guarantee a consumer could reasonably rely on; `0.3.0` is available and builds for all three feature combinations; and no identified downstream requires a fresh `0.1` resolution as its only migration path. Executed with `cargo yank --version 0.1.0 eggsec-nse`; crates.io now reports `0.1.0 yanked = true`.

Yank behavior verified rather than assumed (plan §7 steps 3–5):

| Property | Result |
|---|---|
| registry metadata reflects the yank | `0.1.0 yanked = True`, `0.2.0` and `0.3.0` unaffected |
| an existing lockfile still resolves **and builds** the yanked version | lockfile pinned to `0.1.0` → `cargo build` OK |
| a fresh unconstrained/`^0.1` resolution does not select the yanked version | `failed to select a version for the requirement ^0.1 … version 0.1.0 is yanked` |
| consumers are not stranded with no upgrade target | unconstrained `>=0.1.0` now resolves to `0.3.0` |

**`0.2.0` — NOT YANKED, deliberately, with the reason recorded.** Plan §5's third condition fails, and plan §10's stop condition fires:

> a yank would strand the principal consumer before M007D can adopt 0.3.0

`eggstack/eggsec` requires `eggsec-nse = { version = "0.2.0" }`, i.e. `^0.2.0`, and `0.2.0` is the only `0.2.x` that has ever been published. A fresh resolution of `^0.2.0` therefore has exactly one candidate, and the `0.1.0` experiment above establishes the mechanism directly: a yanked sole in-range candidate is a hard resolution failure, not a soft warning. Yanking `0.2.0` today would break fresh resolution for the principal consumer, whose adoption of `0.3.0` is M007D's work and is explicitly out of scope here.

Consistent with plan §5's instruction for a deliberately unyanked version:

- the compatibility reason is recorded above and in the `007c-r-closure.md` §10 findings;
- **the advisory affected range is unchanged** at `>= 0.1.0, < 0.3.0`;
- **`0.2.0` remains affected.** Not being yanked is not being safe. Anyone resolving `^0.2.0` today still gets the capability-bypassing helpers;
- the revisit trigger is explicit: execute the `0.2.0` yank once M007D has adopted `0.3.0`, and re-run the §7 steps 3–5 at that time.

### 13.6 Communication-surface reconciliation (plan §6, §9 criterion 10)

All five surfaces carry the same message and all five keep the four facts distinct:

| Surface | State |
|---|---|
| GitHub Security Advisory | `GHSA-w2g3-v83j-frp2` published 2026-10-02, four non-claims and the four-way scope note inline |
| `v0.3.0` GitHub Release notes | created after registry verification; explicit "What 0.3.0 does not do" section enumerating all four facts |
| `CHANGELOG.md` `[0.3.0]` | "Honest residual" and "Known limitations" sections; states 0.3.0 is not complete protocol-wide scope enforcement |
| `docs/PROVIDERS.md` | `M007C` section; closes with the four-fact non-collapse statement |
| `docs/RELEASING.md` | security-disclosure note, advisory identity, and the recorded per-version disposition |

The `docs/RELEASING.md` update landed as a **post-release commit on `main` (`870fca7`)**, not on the tag, so `v0.3.0` keeps pointing at the published source. This was necessary for consistency rather than optional: while the advisory was unpublished, `RELEASING.md` correctly said the disposition was owned by the consuming repository, and that sentence became false the moment the advisory was published.

### 13.7 Requirement-to-evidence delta

| Requirement | §1–§12 result | Now | Change |
|---|---|---|---|
| §9.4 fixed version not published before `0.3.0` available | pass | pass | still true: `0.3.0` verified before the advisory was created |
| §9.5 advisory published after the fixed artifact is verified, unless a reason is recorded | blocked with reason | **pass** | `GHSA-w2g3-v83j-frp2` published 2026-10-02T16:00:48Z, after registry and scratch-consumer verification |
| §9.6 CVE explicitly decided, not required for M007D | pass | pass | requested via the advisory flow; assignment async and not claimed |
| §9.7 `0.1.0` explicit decision + rationale | deferred | **executed: yanked** | |
| §9.8 `0.2.0` explicit decision + rationale | deferred | **executed: not yanked, reason recorded** | |
| §9.9 if yanked, registry + fresh-resolution verified | not run | **pass** | §13.5, all four properties |
| §9.11 GHSA identifier or explicit no-advisory rationale | rationale | **pass** | identifier recorded |
| §9.12 M007D unblocked only after fixed artifact + disposition | fail (blocked) | **pass** | both conditions met |
| §8 post-pub: advisory range / fixed version / refs | not run | **pass** | §13.3 |
| §6 surface reconciliation | template + baseline | **pass** | §13.6 |
| §1 high finding (no verified `0.3.0`) | high | **closed** | |

### 13.8 What this addendum does not change

- §5's known limitations are inherited unchanged: the 22-file specialized residual is still not provider-backed; `broker_dns_lookup` still does not evaluate per-target membership for the resolved name (M007D's first task); `DnsResolution` is still not charged to `network_operations`; `upnp.discover` still does not perform real SSDP multicast discovery.
- `0.2.0` is still resolvable and still affected. This addendum does not claim the exposure is closed for existing `0.2.0` consumers; it claims the range is *correctly disclosed* and that a safe upgrade target exists.
- CVE assignment and GitHub Advisory Database ingestion remain outstanding operational items. Neither gates M007D.

### 13.9 Roadmap disposition (supersedes §11)

**M007C-S closed**, with one obligation deliberately carried into M007D rather than completed here.

- Draft preparation: complete (§§3–9).
- Advisory published with fixed `0.3.0`: complete (§13.3).
- `0.1.0` yank executed and verified: complete (§13.5).
- `0.2.0` yank: **deferred to M007D** by design, reason recorded, version stated as still affected (§13.5).
- **M007D is unblocked.** Both of plan §9 criterion 12's conditions are now met: the fixed `0.3.0` artifact exists and is verified, and the disposition record is complete. M007D additionally inherits (a) the `broker_dns_lookup` per-target membership gap and (b) the `0.2.0` yank.
- **M007E remains blocked** on accepted M007D closure. Automated NSE remains quarantined.

### 13.10 Eggsec-side verification, and a pre-existing failure this slice surfaced

§4 of this record stated that `make check` was not run because the slice was plans-only. That is no longer the whole truth, and the correction is recorded here rather than left implicit.

`make check` was run for the eggsec-side planning changes (commit `af85386c`). It **failed on a pre-existing clippy error that this slice did not introduce**:

```text
error: use of deprecated method `std::sync::atomic::Atomic::<usize>::fetch_update`:
       renamed to `try_update` for consistency
  --> crates/eggsec/src/distributed/worker.rs:74:14
make[1]: *** [Makefile:37: clippy] Error 101
```

It is pre-existing, and that was established rather than assumed: the slice's commit touches zero `.rs` files, and `git diff HEAD~1 HEAD -- crates/eggsec/src/distributed/worker.rs` is empty. A clippy/stdlib update in the toolchain made the deprecation observable; it is not a regression from this work. Leaving the repository's mandatory contract red was not an acceptable outcome for a pass that touched the planning control surface, so the error was fixed in a follow-up commit.

The obvious fix was also **wrong**, and the second attempt proves the point worth recording:

1. `fetch_update` → `try_update` cleared the deprecation lint but immediately failed with `current MSRV is 1.89.0 but this item is stable since 1.95.0` (`clippy::incompatible_msrv`). `try_update` does not exist at this workspace's MSRV, so the rename would have silently traded a lint for a broken MSRV contract.
2. The correct fix is a scoped `#[allow(deprecated)]` on `WorkerBudget::try_reserve`, carrying a comment naming the MSRV constraint so the deprecation is not "helpfully" renamed later.

`crates/eggsec/src/distributed/worker.rs` is the guarded capacity path — architecture guard "Worker max_concurrency capacity holds" covers it, and the reservation logic is unchanged by either attempt.

After the fix, `make check` is green end to end: `cargo check` (workspace no-default-features, engine, CLI, CLI no-default-features), `make check-deps` (`cargo deny`: advisories, bans, licenses, sources all ok), `make clippy` (all four `-D warnings` targets), the doc tests, the engine/CLI/feature test suites, the `eggsec-output` / `eggsec-report-model` / `eggsec-policy` / `eggsec-transport-eggfetch` / `eggsec-tui` test suites, and `bash scripts/check-architecture-guards.sh` (`ALL PASSED: No architecture drift detected`).

`crates/eggsec/Cargo.toml` still requires `eggsec-nse = { version = "0.2.0" }` and the lockfile still pins `0.2.0`. That is correct and deliberate: adopting `0.3.0` is M007D's work, and the `^0.2.0` requirement is precisely why the `0.2.0` yank was deferred rather than executed.
