# Desktop GUI Frontend Milestone 015 — The Dependency Advisory Backlog

Status: ready for handoff

Repository baseline: branch `feat/auto-20261022-76692679` at commit `821c8189`
("docs(plans): correct the roadmap's remaining stale approval claims"). Milestones
001–011 of this subsystem are closed, conditionally closed, or implemented.

Source roadmap:

- `plans/subsystems/desktop-gui-frontend-roadmap.md#milestone-15--the-dependency-advisory-backlog`
  (heading at `plans/subsystems/desktop-gui-frontend-roadmap.md:1042`)

Long-term requirements:

- `plans/000-long-term-specification.md#9-verification-contract`
  (`plans/000-long-term-specification.md:101-108`)
- `plans/000-long-term-specification.md#8-runtime-and-dependency-baselines`
  (`plans/000-long-term-specification.md:93-99`)
- `plans/000-long-term-specification.md#7-transport-and-cryptography-policy`
  (`plans/000-long-term-specification.md:86-91`)

Applicable ADRs:

- None. This milestone makes no architecture decision. The one ADR-shaped question
  it touches — whether the ring-only TLS rule (`plans/000-long-term-specification.md:89`)
  may be relaxed so `tiberius 0.13` can clear the `rustls 0.21` advisories — is
  **not** taken here. The rule stands; the advisories are recorded as exceptions.

Primary class: corrective (roadmap class, `plans/subsystems/desktop-gui-frontend-roadmap.md:1044`).
`plans/003-planning-process.md` §3 enumerates the production classes as
`invariant | capability | infrastructure | polish`; "corrective" is a *pass type*
(§7 of the same document), not a production class. This milestone's production
character is predominantly **infrastructure** (dependency-policy work with no
product-surface change), and it is authored under the corrective-pass rules in
`plans/implementation/README.md:171-179`.

## 0. Baseline, origin, and corrective-class record

`plans/implementation/README.md:179` requires that a corrective plan "reference the
original plan and closure record, enumerate unclosed findings, and add regression
evidence that would have caught them", and that it "MUST NOT rewrite the original
plan's completion record". This section discharges that requirement. It is an
addition to the template's §1–§16, not a replacement of any of them.

### 0.1 The original plan and its closure record

| Role | Artifact | Anchor |
|------|----------|--------|
| Original plan | `plans/implementation/desktop-gui-frontend/010-daemon-full-manual-capability-policy.md` | M010, implemented 2026-10-24 |
| Closure record | `plans/closure/desktop-gui-frontend/010-daemon-full-manual-capability-policy-closure.md` | records the advisory backlog as inherited residue |

> **Filename correction.** The milestone briefing referred to the originating plan as
> `010-full-operation-coverage-and-report-export.md`. That file does not exist in this
> subsystem. The actual files are `010-daemon-full-manual-capability-policy.md`
> (M010, the milestone that recorded the backlog) and
> `006-full-operation-coverage-and-report-export.md` (M006, a different milestone).
> The correction is made because the corrective-class rule requires citing the real
> originating plan.

### 0.2 Unclosed findings this milestone closes

M010's closure record enumerated the dependency backlog as **one** unclosed
workspace-wide finding — the advisory set was measured there and carried forward
unchanged. Later closure records in this subsystem (M006, M007, M009) do **not**
name the advisory backlog: the grep across `plans/closure/desktop-gui-frontend/`
for `advisor|advisory|cargo-deny|deny.toml|DEPENDENCY_EXCEPTIONS` matches only the
M010 closure record and the subsystem roadmap. M010 is therefore the sole
originating closure record for this backlog.

The backlog is not one finding but 23 distinct advisory entries. This milestone
closes the whole set; the per-entry closure accounting is in §3.3.

### 0.3 Why the original milestone's verification missed it

M010's closure was written against a verification contract whose dependency step
is `make check-deps` (`Makefile:152-158`). The M010 closure record treats the
advisory backlog as inherited residue rather than as a defect to be closed inside
M010's boundary, which is a scoping decision, not a verification gap. The
regression evidence this milestone adds — the guard specified in §6 "Documentation
and static guards" and the negative tests in §10 "Security and negative tests" —
is what prevents a *future* milestone from shipping an unowned exception or an
advisory backlog that nobody is tracking.

### 0.4 Corrections to the brief, the roadmap, and the cross-check

**To the roadmap's operator-value wording.** The M15 entry's operator-value
paragraph (`plans/subsystems/desktop-gui-frontend-roadmap.md:1055-1058`) read:
*"Most of the backlog is a stale lockfile rather than a genuine exception — the
patch-level upgrades alone clear a majority of the entries."* Measured on this
baseline (§3.2), patch-level lockfile refreshes alone clear **9 of 23 entries
(39.1%)** — a plurality, not a majority. The claim becomes true only if the three
`rustls`/`rustls-webpki` 0.21-line entries are counted as upgradable (12 of 23 =
52.2%), which the repository's own existing rationale at `deny.toml:34-36` says
they are **not**, because the ring-only TLS policy blocks that upgrade. The
roadmap's *direction* holds — the bulk of the backlog is not a genuine exception,
and Bucket A is real work that clears 9 entries with no manifest change — but the
arithmetic does not.

**That correction has already been applied to the roadmap**, by a concurrent
amendment dated 2026-10-24 inserted at
`plans/subsystems/desktop-gui-frontend-roadmap.md:1060-1071`. It records the same
measured figures this plan uses (23 entries / 20 IDs; 14 `vulnerability`, 7
`unmaintained`, 2 `unsound`; 9 of 23 cleared by patch-level refresh; 7 policy-
blocked; 7 with no safe upgrade) and states that "milestone 015's plan records the
corrected figure". **This plan therefore does not propose that amendment** — §12
records it as already applied, and the plan's job is to carry the corrected figure
plus one refinement the amendment does not yet contain: the amendment attributes
the `glib` 0.18.5 entry to "the exact `tauri` pin and the MSRV floor", whereas
§3.4 shows it is **additionally upstream-blocked** — no published `muda`, `tao`,
or `wry` version uses gtk-rs 0.20+, and `tauri` 2.12.5 itself still requires
`gtk ^0.18`. The plan is the place for that refinement, because the roadmap
explicitly defers the corrected detail here.

**Line-number drift note.** `plans/subsystems/desktop-gui-frontend-roadmap.md` was
under concurrent edit while this plan was written (a milestone-12 amendment
inserted ~20 lines ahead of the M15 entry, moving it from line 1012 to line 1042).
Every roadmap citation in this plan was re-verified against the file *after* that
edit settled. A reader who finds a roadmap line number off by a few lines should
locate the clause by its quoted anchor text rather than by the number.

**To the milestone briefing.** The briefing described the backlog as "16 of the
entries have a safe upgrade (patch/minor-level, no breaking change)" and "7 have
no safe upgrade". The **counts are correct** — 16 is exactly the number of entries
for which cargo-deny prints a `Solution:` line, and 7 is exactly the number for
which it prints `No safe upgrade is available!`. The **characterisation** is not:

- Only **9** of the 16 are patch-level lockfile refreshes. The other 7 require
  breaking major bumps (`rustls` 0.21→0.23, `rustls-webpki` 0.101→0.103,
  `glib` 0.18→0.20, `zip` 2→9).
- The 7 cargo-deny calls "no safe upgrade" are **not homogeneous**. Two
  (`proc-macro-error` 1.0.4, `unsafe-libyaml` 0.2.11) are genuinely unmaintained
  crates with no published alternative. The five `unic-*` crates at 0.9.0 **do**
  have a clearing path — `urlpattern` 0.6.0 dropped `unic-ucd-ident` entirely —
  and that path is gated on this repository's MSRV floor, not on upstream (§3.4).

§3.2 gives the three-bucket split this plan uses, which supersedes the two-way
split in the briefing.

**To an independent cross-check measurement.** A second, independently produced
measurement was cross-checked against this baseline before the plan was written.
Two of its counts are off by one:

| Claim | Measured value | How measured |
|-------|----------------|--------------|
| `deny.toml` has **10** `ignore` entries | **9** | count of `id = ` occurrences in `deny.toml` lines 25–37 → 9; entries at `deny.toml:27`, `:28`, `:29`, `:30`, `:32`, `:33`, `:34`, `:35`, `:36` |
| `docs/DEPENDENCY_EXCEPTIONS.md` has **8** exception headings under `## Active Exceptions` covering **10** advisory IDs | **9** headings covering **11** IDs | count of `### ` headings in `docs/DEPENDENCY_EXCEPTIONS.md` lines 50–187 → 9 (`:52`, `:67`, `:82`, `:101`, `:113`, `:128`, `:143`, `:158`, `:173`); of these, **7 are active** (covering 9 IDs) and **2 are `RESOLVED`** (`:82` RUSTSEC-2025-0119, `:101` RUSTSEC-2024-0436, one ID each) |

Two further refinements over that cross-check, both verified here:

- The `glib` 0.18.5 → 0.20.0 upgrade is **blocked upstream as well as** by the
  Tauri pin and the MSRV floor. `tauri` 2.12.5 itself still declares `gtk ^0.18`,
  and **no published version of `muda`, `tao`, or `wry` uses gtk-rs 0.20+**
  (measured across 66 `muda`, 125 `tao`, and 30 `wry` published versions: the
  distinct `gtk`/`glib` requirements are `^0.9`, `^0.10`, `^0.14`, `^0.15`, `^0.16`,
  `^0.18` only). Moving the Tauri line to 2.12 would **not** clear this advisory.
- The five `unic-*` entries are **MSRV-gated, not upgrade-less**. `urlpattern`
  0.6.0's dependency set is `icu_properties`, `regex`, `serde`, `url` — the
  `unic-*` set is gone — and `tauri-utils` 2.10.0 requires `urlpattern ^0.6`.
  `tauri-utils` 2.10.x declares `rust-version = "1.90"`, above the workspace floor
  at `Cargo.toml:39`.

## 1. Objective

Make the mandatory Rust contract **complete** by (a) upgrading every advisory
entry that has a safe upgrade under this repository's policies, and (b) recording
every advisory entry that cannot be upgraded as a dependency exception carrying
an owner, a review-by date, and a full record in the dependency-exception
document.

Concretely, one bounded outcome: `make check-deps` is green and `make check`
runs to completion, with the 23-entry advisory backlog of §3.2 reduced to zero
unrecorded entries.

## 2. Why this milestone is ready

**Closed hard dependencies: none.** The roadmap entry records
`Dependencies: none hard` (`plans/subsystems/desktop-gui-frontend-roadmap.md:1049`).
No ADR, no schema, no protocol, and no external service gates this work.

**Stable interface dependencies, verified at this baseline:**

- `deny.toml` is the canonical dependency-policy source (`deny.toml:15-37`,
  `docs/VERIFICATION.md:254`). It is the only file that may express a policy
  exception. `.cargo/audit.toml` must not exist (`docs/CI_ARCHITECTURE_GUARDS.md:152`).
- `docs/DEPENDENCY_EXCEPTIONS.md` is the canonical exception record with a fixed
  field schema (`:54-65` shows the ten-row table shape) and an explicit policy
  requiring owner, created/reviewed date, review-by date, and blocker (`:35-48`).
- `make check-deps` runs five `cargo deny --workspace --all-features check`
  invocations (`Makefile:154-158`) and fails closed when `cargo-deny` is absent
  (`Makefile:153`).
- The workspace MSRV floor is `1.89` (`Cargo.toml:39`), enforced by
  `make check-msrv` (`Makefile:135-144`).

**Why this is now, and not later:** the contract cannot complete today.
`make check` runs `$(MAKE) check-deps` at `Makefile:179`, between the compile
checks (`Makefile:162-178`) and clippy (`Makefile:180`), tests (`Makefile:181-192`),
and guards (`Makefile:193`). The dependency-policy step is therefore the first
blocking step in the contract after compilation, and it blocks all of clippy, the
test suite, and the architecture guards from running at all. Every milestone that
follows in this subsystem inherits that blockage.

**Unresolved decisions, and how this plan handles them.** Two decisions are
genuinely open and are **not** taken here; both become recorded exceptions with a
named owner and a review-by date rather than silent deferrals:

| Open decision | Why it is not taken here | Handling |
|---------------|--------------------------|----------|
| Raise the workspace MSRV floor from 1.89 to 1.90 | The floor is a workspace-wide contract (`Cargo.toml:39`, `plans/000-long-term-specification.md:95`); changing it is not a dependency-upgrade decision and would invalidate `make check-msrv` assumptions across 25 crates | The five `unic-*` entries are recorded as MSRV-gated exceptions with the exact upgrade recipe and the review-by date by which the MSRV decision must be taken |
| Relax the ring-only TLS rule so `tiberius 0.13` may enable `aws-lc-rs` | Ring-only is a canonical transport invariant (`plans/000-long-term-specification.md:89`, `deny.toml` `[bans]`) with no ADR authorising its relaxation | The three `rustls` 0.21-line entries are recorded as policy-blocked exceptions, extending the existing RUSTSEC-2026-0104/-0099/-0098 record |

## 3. Current implementation evidence

### 3.1 Where the contract blocks

`Makefile:161-193` defines `check`. Its expansion, in order, is: fmt
(`Makefile:162`) → `cargo check --workspace --no-default-features`
(`Makefile:163`) → the GUI shell profile check, prerequisite-gated
(`Makefile:167`) → `cargo check -p eggsec --all-targets --no-default-features`
(`Makefile:171`) → `cargo check -p eggsec` (`Makefile:172`) → the four
`eggsec-cli` checks (`Makefile:173-177`) → `cargo test -p eggsec-cli --features
daemon-client` (`Makefile:178`) → **`$(MAKE) check-deps` (`Makefile:179`)** →
`$(MAKE) clippy` (`Makefile:180`) → the doc and unit test runs
(`Makefile:181-192`) → `bash scripts/check-architecture-guards.sh`
(`Makefile:193`).

Step 8 is the blocker. `docs/VERIFICATION.md:24` and `docs/VERIFICATION.md:69`
name `make check-deps` as the supply-chain gate; `docs/VERIFICATION.md:305`
records that `make check` passes in CI as "fmt, no-default checks, `check-deps`,
clippy, tests, guards", in that order.

`check-deps` is **five** invocations, not one (`Makefile:154-158`):

```make
check-deps:
	@command -v cargo-deny >/dev/null 2>&1 || (echo "FAIL: cargo-deny not found. ..." >&2; exit 1)
	cargo deny --workspace --all-features check
	cargo deny --workspace --all-features check advisories
	cargo deny --workspace --all-features check bans
	cargo deny --workspace --all-features check licenses
	cargo deny --workspace --all-features check sources
```

The first invocation (the default check set) already includes advisories, so the
target fails on advisories before reaching the remaining three sub-checks. The
other four sub-checks — bans, licenses, sources, and the default set's non-advisory
parts — must also stay green, and this milestone must not disturb them.

**Tool version drift (verified, and a handoff hazard).** The local tool is
`cargo-deny 0.20.2`. CI installs `cargo-deny@0.19.0`
(`.github/workflows/ci.yml:67`). The advisory *database* is fetched live from
`https://github.com/rustsec/advisory-db` (`deny.toml:16-17`), so the backlog is a
moving target in both directions: a future advisory publication can add entries
that this plan does not enumerate, and a publication or withdrawal can remove
entries that it does. §14 covers this.

### 3.2 The measured backlog

**Method.** cargo-deny writes its diagnostics to **stderr**, so the capture must
redirect both streams:

```bash
cargo deny --workspace --all-features check advisories > /tmp/m015_advisories.txt 2>&1
```

The command exits non-zero. Counting rules applied to the captured file:

- Entries: `grep -c "^error\[" /tmp/m015_advisories.txt` → **23**
- Unique advisory IDs: `grep -o "RUSTSEC-[0-9]\{4\}-[0-9]\{4\}" /tmp/m015_advisories.txt | sort -u | wc -l` → **20**
- Kind split: **14** `vulnerability`, **7** `unmaintained`, **2** `unsound`

**Result: 23 entries across 20 unique advisory IDs** — unchanged from M010's
closure record. The per-entry table is §3.3.

**The three-bucket split** (this is the backbone of the plan):

| Bucket | Entries | Meaning |
|--------|---------|---------|
| **A — lockfile refresh only** | **9** | A patch-level bump within the existing manifest requirement. No manifest change at all. |
| **B — upgrade exists but is not safe here** | **7** | An upstream fix exists; adopting it is blocked by this repository's policy, its MSRV floor, or a breaking API break. Each needs a judgement and a recorded reason. |
| **C — no upstream upgrade at all** | **7** | The affected crate itself has no newer published version. Needs an ignore with owner + review-by date. |

9 + 7 + 7 = **23**. ✓

**Bucket A — 9 entries, cleared by a lockfile refresh alone.** Verified
empirically, not inferred: `cargo update --dry-run` (no lockfile written) proposes
`ctutils` 0.4.2→0.4.5, `rustls` 0.23.45→0.23.46, `rustls-webpki` 0.103.15→0.103.16,
`tokio` 1.53.1→1.54.1, `webpki-roots` 1.0.9→1.0.11, and `webpki-root-certs`
1.0.9→1.0.11 — six patch-level bumps that between them satisfy all nine advisory
minimums in §3.3, with no manifest change. The same dry run reports
`zip v2.4.2 -> v2.6.1 (available: v9.1.0)`, i.e. the lockfile refresh alone does
**not** clear the `zip` advisories, which need ≥ 9.0.0.

**Bucket B — 7 entries, upgrade not safe here:**

- **3 entries on the `rustls` 0.21 line** (rows 4, 5, 10 of §3.3), reachable only
  via `tiberius` 0.12.3 → `tokio-rustls` 0.24.1. The repository has already decided
  this class: `deny.toml:34-36` ignores RUSTSEC-2026-0104/-0099/-0098 on the same
  crates, with the rationale that `tiberius` 0.13.0 moves to `rustls` 0.23 /
  webpki 0.103 but pulls `tokio-rustls` with default features, enabling the
  `aws-lc-rs` provider that `[bans] deny` forbids. **Verified:** `tokio-rustls`
  0.26.5's `default` feature set is `["logging", "tls12", "aws_lc_rs"]`. The new
  entries are the identical situation and join the same record.
- **1 entry on `glib` 0.18.5** (RUSTSEC-2024-0429), reachable only through the
  Tauri closure. Blocked upstream as well as by policy — see §3.4.
- **3 entries on `zip` 2.4.2** (RUSTSEC-2026-0367/-0368/-0369). A 2.x → 9.x major
  bump; `zip` 9.x's `write` module no longer exports the 2.x `FileOptions<'k, T>`
  shape (the 9.x signature is `FileOptions<'_, '_, T>`), so the call sites in
  `crates/eggsec-mobile-lab` need source migration. `zip` 9.1.0 declares
  `rust-version = "1.88"`, i.e. **below** the workspace floor, so the bump does
  not raise MSRV.

**Bucket C — 7 entries, no upstream upgrade:** `proc-macro-error` 1.0.4
(RUSTSEC-2024-0370, latest published; pulled by `gtk3-macros` 0.18.2 and
`glib-macros` 0.18.5); `unsafe-libyaml` 0.2.11 (RUSTSEC-2024-0449, latest
published, and `serde_yaml_neo` 0.11.0 — also the latest — hard-depends on it);
and the five `unic-*` crates at 0.9.0 (RUSTSEC-2025-0075 `unic-char-range`,
-0080 `unic-common`, -0081 `unic-char-property`, -0098 `unic-ucd-version`,
-0100 `unic-ucd-ident`), each at its latest published version but with an
*entry-level* clearing path — see §3.4.

### 3.3 Complete per-advisory table

Every row is measured from the captured cargo-deny output and cross-checked
against `cargo tree` reverse-dependency output. "Path" is the chain from the
direct workspace dependency to the affected crate; the first element is always a
**direct** dependency of a workspace member, which is what makes the row
actionable.

| # | Advisory ID | Kind | Crate @ version | Affected dependency path | Upgrade that clears it | Bucket | Action |
|---|-------------|------|-----------------|--------------------------|-------------------------|--------|--------|
| 1 | RUSTSEC-2026-0385 | vulnerability | `ctutils` 0.4.2 | `mongodb` 3.9.1 → `hmac` 0.13.0 → `digest` 0.11.3 → `ctutils` 0.4.2 | `ctutils` ≥ 0.4.4 (0.4.5 available) | **A** | lockfile refresh |
| 2 | RUSTSEC-2024-0429 | unsound | `glib` 0.18.5 | `eggsec-gui` → `tauri` 2.11.6 → `gtk` 0.18.2 → `glib` 0.18.5 (also via `wry` 0.55.1 → `gtk`, `tao` 0.35.3 → `gtk`) | `glib` ≥ 0.20.0 — **not reachable**: no published `muda`/`tao`/`wry` uses gtk-rs 0.20+ | **B** | exception (upstream-blocked) |
| 3 | RUSTSEC-2024-0370 | unmaintained | `proc-macro-error` 1.0.4 | `eggsec-gui` → `tauri` 2.11.6 → `gtk` 0.18.2 → `gtk3-macros` 0.18.2 → `proc-macro-error` 1.0.4; and → `glib-macros` 0.18.5 → `proc-macro-error` 1.0.4 | none (1.0.4 is latest); clears together with row 2 when gtk-rs reaches 0.20+ | **C** | exception (upstream-blocked) |
| 4 | RUSTSEC-2026-0346 | vulnerability | `rustls` 0.21.12 | `eggsec-db-lab` → `tiberius` 0.12.3 → `tokio-rustls` 0.24.1 → `rustls` 0.21.12 | `rustls` ≥ 0.23.46 via `tiberius` 0.13 — blocked by ring-only TLS policy | **B** | exception (policy-blocked) |
| 5 | RUSTSEC-2026-0349 | vulnerability | `rustls` 0.21.12 | `eggsec-db-lab` → `tiberius` 0.12.3 → `tokio-rustls` 0.24.1 → `rustls` 0.21.12 | `rustls` ≥ 0.23.46 via `tiberius` 0.13 — blocked by ring-only TLS policy | **B** | exception (policy-blocked) |
| 6 | RUSTSEC-2026-0346 | vulnerability | `rustls` 0.23.45 | `eggsec` / `eggsec-python` → `reqwest` → `rustls` 0.23.45 (also `eggfetch-core` 0.2.0 → `rustls` 0.23.45) | `rustls` ≥ 0.23.46 | **A** | lockfile refresh |
| 7 | RUSTSEC-2026-0347 | vulnerability | `rustls` 0.23.45 | same as row 6 | `rustls` ≥ 0.23.46 | **A** | lockfile refresh |
| 8 | RUSTSEC-2026-0348 | vulnerability | `rustls` 0.23.45 | same as row 6 | `rustls` ≥ 0.23.46 | **A** | lockfile refresh |
| 9 | RUSTSEC-2026-0349 | vulnerability | `rustls` 0.23.45 | same as row 6 | `rustls` ≥ 0.23.46 | **A** | lockfile refresh |
| 10 | RUSTSEC-2026-0382 | vulnerability | `rustls-webpki` 0.101.7 | `eggsec-db-lab` → `tiberius` 0.12.3 → `tokio-rustls` 0.24.1 → `rustls` 0.21.12 → `rustls-webpki` 0.101.7 | `rustls-webpki` ≥ 0.103.16 via `tiberius` 0.13 — blocked by ring-only TLS policy | **B** | exception (policy-blocked) |
| 11 | RUSTSEC-2026-0382 | vulnerability | `rustls-webpki` 0.103.15 | `eggsec` → `reqwest` → `rustls` 0.23.45 → `rustls-webpki` 0.103.15 (also `rustls-platform-verifier` 0.7.0) | `rustls-webpki` ≥ 0.103.16 | **A** | lockfile refresh |
| 12 | RUSTSEC-2026-0386 | unsound | `tokio` 1.53.1 | workspace-wide baseline: `eggsec`, `eggsec-cli`, `eggsec-daemon`, `eggsec-agent`, `eggsec-web-proxy`, `eggsec-python`, `eggsec-runtime` | `tokio` ≥ 1.53.3 (1.54.1 available) | **A** | lockfile refresh |
| 13 | RUSTSEC-2024-0449 | unmaintained | `unsafe-libyaml` 0.2.11 | `eggsec` → `serde_yaml_neo` 0.11.0 → `unsafe-libyaml` 0.2.11; `eggsec-web-proxy` → `serde_yaml_neo` 0.11.0 → `unsafe-libyaml` 0.2.11 | none (`serde_yaml_neo` 0.11.0 is latest and hard-depends) | **C** | exception (upstream-blocked) |
| 14 | RUSTSEC-2025-0075 | unmaintained | `unic-char-range` 0.9.0 | `eggsec-gui` → `tauri` 2.11.6 → `tauri-utils` 2.9.3 → `urlpattern` 0.3.0 → `unic-ucd-ident` 0.9.0 → `unic-char-range` 0.9.0 | `urlpattern` 0.6.0 drops the `unic-*` set — needs `tauri-utils` 2.10 (MSRV 1.90) | **C** | exception (MSRV-gated) |
| 15 | RUSTSEC-2025-0080 | unmaintained | `unic-common` 0.9.0 | same as row 14 | same as row 14 | **C** | exception (MSRV-gated) |
| 16 | RUSTSEC-2025-0081 | unmaintained | `unic-char-property` 0.9.0 | same as row 14 | same as row 14 | **C** | exception (MSRV-gated) |
| 17 | RUSTSEC-2025-0098 | unmaintained | `unic-ucd-version` 0.9.0 | same as row 14 | same as row 14 | **C** | exception (MSRV-gated) |
| 18 | RUSTSEC-2025-0100 | unmaintained | `unic-ucd-ident` 0.9.0 | same as row 14 | same as row 14 | **C** | exception (MSRV-gated) |
| 19 | RUSTSEC-2026-0383 | vulnerability | `webpki-root-certs` 1.0.9 | `eggsec` → `reqwest` → `rustls-native-certs` 0.8.4 → `webpki-root-certs` 1.0.9 (also `rustls-platform-verifier` 0.7.0, `kube-client` 4.2.0) | `webpki-root-certs` ≥ 1.0.10 (1.0.11 available) | **A** | lockfile refresh |
| 20 | RUSTSEC-2026-0381 | vulnerability | `webpki-roots` 1.0.9 | `eggsec` → `reqwest` → `rustls` 0.23.45 → `webpki-roots` 1.0.9 (also `mongodb` 3.9.1, `lettre` 0.11.23) | `webpki-roots` ≥ 1.0.10 (1.0.11 available) | **A** | lockfile refresh |
| 21 | RUSTSEC-2026-0367 | vulnerability | `zip` 2.4.2 | `eggsec-mobile-lab` → `zip` 2.4.2 (direct, `crates/eggsec-mobile-lab/Cargo.toml:46`); `eggsec` → `zip` 2.4.2 (direct, `crates/eggsec/Cargo.toml:278-280`, enabled by the `mobile` feature at `crates/eggsec/Cargo.toml:426`) | `zip` ≥ 9.0.0 (9.1.0, MSRV 1.88) | **B** | upgrade (breaking) |
| 22 | RUSTSEC-2026-0368 | vulnerability | `zip` 2.4.2 | same as row 21 | same as row 21 | **B** | upgrade (breaking) |
| 23 | RUSTSEC-2026-0369 | vulnerability | `zip` 2.4.2 | same as row 21 | same as row 21 | **B** | upgrade (breaking) |

**Closure accounting:** rows 1, 6, 7, 8, 9, 11, 12, 19, 20 (9 rows) close by
lockfile refresh; rows 21, 22, 23 (3 rows) close by the `zip` 9.x upgrade; rows 2,
3, 4, 5, 10, 13, 14, 15, 16, 17, 18 (11 rows) close as recorded exceptions.
9 + 3 + 11 = 23.

### 3.4 The two upgrade-blocked clusters, in detail

**The `unic-*` cluster (5 entries) is MSRV-gated, not upgrade-less.** This is the
single most consequential refinement in the plan, because it converts five "no
safe upgrade" entries into five exceptions with a *known, dated, mechanical*
removal path.

- `urlpattern` 0.3.0 depends on `unic-ucd-ident ^0.9.0`.
- `urlpattern` **0.6.0** depends on `icu_properties ^2`, `regex ^1.10.5`,
  `serde ^1.0.127`, `url ^2.5.6` — the `unic-*` set is **gone**.
- `tauri-utils` 2.9.3 requires `urlpattern ^0.3`; `tauri-utils` **2.10.0** requires
  `urlpattern ^0.6`.
- `tauri-utils` 2.10.0/2.10.1/2.10.2 all declare `rust-version = "1.90"`.
- The workspace floor is `rust-version = "1.89"` (`Cargo.toml:39`), and
  `make check-msrv` builds with `cargo +1.89` (`Makefile:135-144`).
- The workspace pins the whole Tauri closure with **exact** `=` requirements at
  `Cargo.toml:93-98` (`tauri` `=2.11.6`, `tauri-build` `=2.6.3`, `tauri-runtime`
  `=2.11.3`, `tauri-runtime-wry` `=2.11.4`, `tauri-macros` `=2.6.3`,
  `tauri-utils` `=2.9.3`), with the stated reason at `Cargo.toml:86-92`: the
  Tauri 2.12 / tauri-utils 2.10 / tauri-macros 2.7 lines declare MSRV 1.90, and
  `tauri` declares its runtime and
  utils dependencies with caret requirements, so without the pins a `cargo update`
  would float the closure onto the 1.90 line and break `make check-msrv`.
- **Observed behaviour confirming the mechanism:** `cargo update --dry-run -p tauri-utils`
  reports *"Locking 0 packages to highest compatible versions"* — cargo does not
  float `tauri-utils` onto 2.10.x while the workspace floor is 1.89.

**Removal recipe for the `unic-*` cluster** (for the exception record, not for
this milestone): raise the workspace `rust-version` to 1.90 in a separate,
workspace-wide decision; move the Tauri closure to the 2.12 line (`tauri` 2.12.5,
`tauri-utils` 2.10.2, `tauri-macros` 2.7.4, `tauri-runtime` 2.12.2,
`tauri-runtime-wry` 2.12.3, `tauri-build` 2.7.4); re-run `make check-msrv`; the
five `unic-*` entries then disappear because `urlpattern` resolves to 0.6.x. Note
that `tauri` 2.12.5 itself still declares `gtk ^0.18`, so this recipe clears rows
14–18 and **not** rows 2–3.

**The `glib` cluster (2 entries) is upstream-blocked as well as policy-blocked.**
Measured across all published `muda` (66), `tao` (125), and `wry` 0.5x (30)
versions, the only `gtk`/`glib` requirements that exist are `^0.9`, `^0.10`,
`^0.14`, `^0.15`, `^0.16`, and `^0.18`. `tauri` 2.11.6 **and** `tauri` 2.12.5 both
declare `gtk ^0.18` directly. `gtk-rs` 0.20 exists upstream (`glib`'s latest
published version is 0.22.10), but **no consumer in the Tauri closure has moved
to it**. Therefore the `glib` 0.18.5 advisory (RUSTSEC-2024-0429) cannot be
cleared by moving the Tauri line, by raising MSRV, or by any lockfile operation;
and `proc-macro-error` 1.0.4 (RUSTSEC-2024-0370) shares the blocker exactly — the
gtk-rs 0.20+ macro crates do not depend on it, but no published
`tauri`/`wry`/`tao`/`muda` reaches gtk-rs 0.20+ to get there.

**The `zip` cluster (3 entries) — exploitability and the real fix.** All three
advisories concern archive **extraction**: preservation of SUID/SGID bits during
extraction (RUSTSEC-2026-0367), creation of symbolic links with attacker-controlled
targets escaping the destination directory (RUSTSEC-2026-0368), and application of
Unix permissions through symlinks without confining the target (RUSTSEC-2026-0369).
**Measured: `crates/eggsec-mobile-lab` contains no `extract()`, `extract_to`,
`unpack`, or `extract_dir` call site** — the crate reads individual entries by name
and writes archives with `ZipWriter`. The affected API is therefore not exercised.
That is the compensating control, and it is a fact about this workspace, not a
general claim about the crate. The primary path remains the `zip` 9.x upgrade
(MSRV 1.88, below the floor), with the exception route as the fallback if the
migration cannot be completed safely.

### 3.5 Current state of the exception machinery

`deny.toml:15-37` — the whole `[advisories]` block, verbatim:

```toml
[advisories]
db-path = "advisory-db"
db-urls = ["https://github.com/rustsec/advisory-db"]
unmaintained = "transitive"
unsound = "transitive"

# Dependency advisory exceptions — reviewed 2026-09-13 (Phase F).
# Cargo Deny is the canonical policy source; each ignore links to the full
# record in docs/DEPENDENCY_EXCEPTIONS.md (advisory ID, path, feature, API use,
# exploitability, compensating control, owner, created/review-by, blocker).
ignore = [
    # --- Transitive: unmaintained crates with no upstream fix ---
    { id = "RUSTSEC-2025-0057", reason = "fxhash unmaintained; transitive via selectors->scraper (recon/js, nse/httpspider). Internal hash map, not exposed. See docs/DEPENDENCY_EXCEPTIONS.md." },
    # ... 8 further entries at deny.toml:28, :29, :30, :32, :33, :34, :35, :36 ...
]
```

Facts to record precisely:

- **`[advisories]` sets no `yanked` key.** cargo-deny's default applies (`warn`),
  which is what `docs/DEPENDENCY_EXCEPTIONS.md:199-200` already states. There is no
  explicit `yanked` policy to change, and the yanked notice at
  `docs/DEPENDENCY_EXCEPTIONS.md:195-205` is correctly a notice, not an exception.
- **`unmaintained = "transitive"` and `unsound = "transitive"`** (`deny.toml:18-19`)
  mean those two advisory *kinds* are enforced only for transitive dependencies.
  The 7 `unmaintained` and 2 `unsound` entries in the backlog are all transitive.
- **Every one of the 9 existing `ignore` entries carries a non-empty `reason`**,
  and every reason ends with `See docs/DEPENDENCY_EXCEPTIONS.md.` — measured: 9 of
  9 have a reason, 9 of 9 reference the document. This is the house style to copy.
- **No `ignore` entry carries an owner or a review-by date.** Those live only in
  `docs/DEPENDENCY_EXCEPTIONS.md`. This matters for the guard in §6 "Documentation
  and static guards": a guard that required an owner *inside* `deny.toml` would be
  wrong and would fail against all 9 existing entries.
- **`docs/DEPENDENCY_EXCEPTIONS.md` has 9 headings under `## Active Exceptions`**
  (`:50`–`:187`), covering 11 advisory IDs. Of the 9 headings, **2 are marked
  `RESOLVED 2026-10-06`** (`:82` RUSTSEC-2025-0119 `number_prefix`, `:101`
  RUSTSEC-2024-0436 `paste`) and are **still placed under the "Active Exceptions"
  heading**. **Assessment:** this is a structural inconsistency, not a policy
  violation — the document's own text at `:97-99` and `:108` records that the
  corresponding `deny.toml` ignores were removed rather than left as dead entries,
  so the *policy* state is correct and only the *heading placement* is misleading.
  The guard must tolerate `RESOLVED` headings in place and enforce that a RESOLVED
  ID has **no** `ignore` entry; §12 requires the two resolved headings to be moved
  under a `## Resolved Exceptions` heading as a documentation-only change.
- The exception field schema is fixed and uniform — see
  `docs/DEPENDENCY_EXCEPTIONS.md:54-65`: `| Advisory |`, `| Path |`, `| Feature |`,
  `| API used |`, `| Exploitability |`, `| Compensating control |`, `| Owner |`,
  `| Created |`, `| Review-by |`, `| Blocker |`. The policy at `:35-48` requires
  all ten.
- The review-by date in force across all 7 active exceptions is **2026-12-12**
  (`docs/DEPENDENCY_EXCEPTIONS.md:64`, `:79`, `:170`, `:185`), and
  `docs/DEPENDENCY_EXCEPTIONS.md:261` carries the review schedule. Today is
  2026-10-24, so 2026-12-12 is **49 days** away — inside the 90-day maximum the
  policy sets at `:47`.

### 3.6 The ownership boundary of each blocked cluster

| Cluster | Direct workspace dependency | Owner for the exception record |
|---------|------------------------------|----------------------------------|
| `rustls` 0.21 / `rustls-webpki` 0.101 | `eggsec-db-lab` (`tiberius`, `db-pentest-mssql-tiberius` marker) | **eggsec-db-lab** — matches the existing owner at `docs/DEPENDENCY_EXCEPTIONS.md:183` |
| `glib` / `proc-macro-error` | `eggsec-gui` (Tauri closure) | **eggsec-gui** — the Tauri shell closure |
| `unic-*` ×5 | `eggsec-gui` (Tauri closure) | **eggsec-gui** — same closure, same blocker family as the `glib` cluster |
| `unsafe-libyaml` | `eggsec` and `eggsec-web-proxy` (`serde_yaml_neo`) | **eggsec (config) / eggsec-web-proxy** — matches the pattern at `docs/DEPENDENCY_EXCEPTIONS.md:62` |

`unsafe-libyaml`'s API-use assessment must be recorded honestly rather than
favourably. `serde_yaml_neo` parses YAML in the engine's config loader
(`crates/eggsec/src/config/loader.rs:46`), policy resolver
(`crates/eggsec/src/policy_bridge/resolver.rs:205`), agent skill front-matter
(`crates/eggsec/src/agent/skills.rs:50`), scanner template loading and verification
(`crates/eggsec/src/scanner/templates/loader.rs:40`,
`crates/eggsec/src/scanner/templates/verify.rs:71`) and marketplace templates
(`crates/eggsec/src/scanner/templates/marketplace.rs:161`, `:282` — the last of
which parses **downloaded** template content), API-schema fuzzing
(`crates/eggsec/src/fuzzer/api_schema/mod.rs:458`,
`crates/eggsec/src/api_schema/mod.rs:57`), auth-context loading
(`crates/eggsec/src/auth_context/mod.rs:48`), OpenAPI export
(`crates/eggsec/src/tool/openapi.rs:26`), and the web-proxy's config and
intercept rules (`crates/eggsec-web-proxy/src/config.rs:181`,
`crates/eggsec-web-proxy/src/intercept/rules.rs:803`). The advisory kind is
`unmaintained`, not a vulnerability, and `unsafe-libyaml` 0.2.11 is the latest
published version with no fixed release — so the mitigation is a recorded
exception with an owner, not a claim of non-exposure.

## 4. Invariants that must not regress

- **`deny.toml` is the canonical dependency-policy source.** `.cargo/audit.toml`
  must not exist; `cargo audit` configuration must not be reintroduced
  (`docs/CI_ARCHITECTURE_GUARDS.md:152`, `AGENTS.md`).
- **Every retained dependency exception carries an owner, a review-by date, and a
  record in `docs/DEPENDENCY_EXCEPTIONS.md`** (`deny.toml:21-24`,
  `docs/DEPENDENCY_EXCEPTIONS.md:35-48`).
- **TLS is ring-only everywhere.** `rustls`/`tokio-rustls` with
  `default-features = false` plus `["ring", ...]`; `reqwest` with
  `rustls-no-provider`, never `rustls` (`plans/000-long-term-specification.md:89`).
  `[bans] deny = ["aws-lc-rs"]` (`deny.toml:70-74`) stands; this milestone does not
  relax it.
- **MSRV 1.89** (`Cargo.toml:39`), verified by `make check-msrv`
  (`Makefile:135-144`). No upgrade in this milestone may raise it.
- **No new dependency without an explicit `deny.toml` decision.** The `zip` 9.x
  bump introduces no new direct dependency but may pull new transitive ones; the
  `cargo deny check bans` and `check licenses` sub-checks (`Makefile:156-157`)
  must be re-run after it.
- **`make check-deps` fails closed when `cargo-deny` is absent**
  (`Makefile:153`). This behaviour is not changed.
- **The desktop GUI is daemon-only** and links no engine, daemon, CLI, or TUI
  crate (guard Check 155). The `unic-*` and `glib` clusters are reached through
  the GUI's Tauri shell closure only; nothing in this milestone changes the GUI's
  dependency direction.
- **`EnforcementContext::evaluate()` remains the mandatory pre-dispatch gate** and
  strict surfaces remain fail-closed. This milestone changes no dispatch,
  authorization, or scope behaviour.
- **Never `let _ =` and never `filter_map(|e| e.ok())`** — log via `tracing`
  (`plans/000-long-term-specification.md:99`).
- **No full-suite run as root** (`AGENTS.md`).

## 5. Scope

### In scope

- The 23 advisory entries in §3.3, and only those entries.
- The lockfile refresh that clears the 9 Bucket-A entries, and the `Cargo.lock`
  diff it produces.
- The `zip` 2.4.2 → 9.x upgrade and the source migration it requires in
  `crates/eggsec-mobile-lab` (and any `eggsec` call sites that use `zip`).
- New `ignore` entries in `deny.toml` for the 11 entries that cannot be upgraded
  (rows 2, 3, 4, 5, 10, 13, 14, 15, 16, 17, 18 of §3.3).
- New exception records in `docs/DEPENDENCY_EXCEPTIONS.md` for the same 11
  entries, each with the ten-field schema of `:54-65`, an owner from §3.6, and a
  review-by date.
- One new architecture guard, **Check 190**, enforcing the "no unowned exception"
  rule (§6 "Documentation and static guards").
- The documentation updates in §12.

### Explicitly out of scope

- **Adopting a different supply-chain scanner** (or replacing `cargo-deny`).
  `deny.toml` stays canonical; `cargo deny` stays the tool.
- **Relaxing the ring-only TLS rule** to let `tiberius` 0.13 land. That is a
  policy decision with no ADR, and it would require an ADR plus a
  `plans/000-long-term-specification.md` amendment — a separate milestone.
- **Raising the workspace MSRV floor to 1.90.** Out of scope for a dependency
  milestone; it is a workspace-wide contract change.
- **Moving the Tauri closure to the 2.12 line.** Blocked by the MSRV floor; the
  exception record carries the recipe instead.
- **Relaxing `unmaintained = "transitive"` or `unsound = "transitive"`** to
  `none` to make the gate green. This is explicitly rejected: it would silence
  *future* unmaintained and unsound advisories as well as the current ones, which
  is a silent weakening of the supply-chain policy rather than a fix.
- **Adding `yanked` policy keys** to `[advisories]`. The default already applies
  and the yanked notice is correctly a notice
  (`docs/DEPENDENCY_EXCEPTIONS.md:195-205`).
- **Any Python-binding, stub, or script change.** See §10 "Migration and
  compatibility tests" and §11 for why `make check-python` is not implicated.
- **Changing `EnforcementContext`, dispatch, scope, or the daemon protocol.**
- **Any GUI feature, surface, or schema change.** This milestone changes the GUI
  only through its dependency closure.

## 6. Required production changes

### Core/domain

No domain logic changes are required or permitted.

The `zip` 2.4.2 → 9.x migration is a **call-site migration inside
`crates/eggsec-mobile-lab`**, not a domain change. The affected call sites are
the IPA/APK archive readers and writers that use `zip::write::FileOptions<()>` and
`zip::write::SimpleFileOptions`. The 2.x `FileOptions` type is generic over one
lifetime and one extension parameter; the 9.x type carries an additional lifetime
parameter, and the 9.x `write` module exposes `ZipFileBuilder` / `PreparedZipFile`
as the construction path. The implementer must migrate these call sites to the
9.x API and preserve the exact archive layout the mobile-lab tests assert (entry
names, stored/deflate method, and the absence of any extraction step). A failing
archive assertion must not be "fixed" by relaxing the assertion; a genuine layout
change must be justified in the closure record.

`unsafe-libyaml` and the `unic-*` clusters require **no** code change in this
milestone: they are recorded as exceptions, and the compensating controls are the
absence of the affected APIs (for `zip`, §3.4) or the operator-supplied nature of
the parsed content (for `serde_yaml_neo`, §3.6).

### Storage and migrations

None. No persisted representation changes. The GUI owns no authoritative storage
(`plans/subsystems/desktop-gui-frontend-roadmap.md:1082-1086`).

The only persisted artifact changed is `Cargo.lock`, and it is changed by
`cargo update`, not by a schema migration. No wire format, no on-disk layout, and
no daemon protocol change is involved.

### Protocol and DTOs

None. No protocol, DTO, or wire-shape change. The daemon protocol remains
additive-only (`plans/subsystems/desktop-gui-frontend-roadmap.md:1090-1098`).

### Runtime and concurrency

No runtime or concurrency behaviour changes.

The one runtime-adjacent fact this milestone must not disturb is the workspace
Tokio baseline: `default-features = false` with per-crate `features = [...]`
(`plans/000-long-term-specification.md:96`). The `tokio` 1.53.1 → 1.54.1 bump is a
patch-level bump inside that baseline and does not alter the feature policy. If
the implementer's `cargo update` produces a Tokio version whose feature set
differs under `default-features = false`, that is a stop condition (§14), not a
silently accepted change.

All spawned tokio tasks keep their 30–300s timeout wrappers
(`plans/000-long-term-specification.md:99`). This milestone spawns no task and
adds no timeout.

### Frontend or operator surface

No frontend surface changes. The GUI's own surface, schema, and tabs are
untouched.

The only operator-visible change is the **content of the two policy files**: a
refreshed `Cargo.lock`, additional `ignore` entries in `deny.toml`, and additional
records in `docs/DEPENDENCY_EXCEPTIONS.md`. An operator reading
`docs/DEPENDENCY_EXCEPTIONS.md` must be able to determine, for every retained
advisory, who owns it, when it must be reviewed, and what would clear it.

### Security and authorization

No authorization change. `EnforcementContext::evaluate()` remains the mandatory
pre-dispatch gate; strict surfaces remain fail-closed; nothing in this milestone
relaxes a scope check or an operation policy.

Three security-relevant assessments must be recorded (not asserted away):

1. The `zip` advisory cluster is about archive **extraction**, which this
   workspace does not perform (§3.4). If the `zip` 9.x migration or any future
   change introduces an `extract()` call, the compensating control is void and
   the exception must be re-assessed.
2. The `rustls` 0.21-line cluster sits behind lab-only, operator-initiated,
   scope-enforced MSSQL connections, matching the existing record at
   `docs/DEPENDENCY_EXCEPTIONS.md:179-182`. The upgrade stays blocked by the
   ring-only rule; the cluster's blast radius does not grow.
3. The `unsafe-libyaml` cluster parses YAML that is partly operator-supplied and
   partly downloaded (§3.6). The advisory kind is `unmaintained`, not a
   vulnerability, and no fixed release exists; the exception must say so plainly
   rather than claiming the parser is unused.

### Documentation and static guards

**One new guard: Check 190 — "every dependency exception is owned and dated".**

The guard lives in `scripts/check-architecture-guards.sh`. The highest existing check
number is **176** (the file's numbered tail runs 174, 175, 176, and the
`--- Check N:` banners carry **77** distinct numbers, 100 through 176), so the
new check takes the next free number. **The number 190 is reserved for this
guard**, in the execution order of the four planned milestones: milestone 012
takes 177–179, milestone 013 takes 180, milestone 014 takes 181–189, and this
milestone takes **190**. The reservation follows the precedent of milestone
007's plan, which took 168 because the guards of milestones 006, 008, and 009
landed first. An implementer who takes this milestone out of milestone order
must re-derive the number from the script's then-current highest, and must not
reuse a number another planned milestone has reserved. The file's house style is a banner,
a `SECTION_FAIL=0` accumulator, per-item checks, and a final
`if [[ "$SECTION_FAIL" -ne 0 ]]; then ... exit 1; fi` — the implementer must follow
the pattern of the immediately preceding checks (see
`scripts/check-architecture-guards.sh:4214-4222` for the banner/section shape and
`scripts/check-architecture-guards.sh:4200-4212` for the `code_search` helper's
"a guard that silently passes is worse than none" discipline).

The guard must implement **four** sub-checks, because the ownership data is split
across two files and `deny.toml` deliberately carries no owner field:

**(a) Every `deny.toml` `[advisories] ignore` entry has a non-empty `reason`.**
Parse the `ignore = [ ... ]` array and fail if any entry's `reason` is absent or
empty. Baseline: 9 of 9 have one.

**(b) Every `deny.toml` `ignore` entry's `reason` references the exception
document.** The house style already ends every reason with
`See docs/DEPENDENCY_EXCEPTIONS.md.`; require that literal substring. Baseline:
9 of 9 satisfy it.

**(c) Every advisory ID in `deny.toml`'s `ignore` list appears in
`docs/DEPENDENCY_EXCEPTIONS.md` under a heading that carries a non-empty
`| Owner |` row and a `| Review-by |` row whose value is either an ISO date
(`YYYY-MM-DD`) or the literal `— (closed)`.** This is where the owner and the date
actually live; requiring them inside `deny.toml` would be wrong and would fail
against all 9 existing entries. Baseline: the 9 ignored IDs map to the 7 active +
2 resolved headings; the 7 active ones carry `| Owner |` and a `2026-12-12`
`| Review-by |`, the 2 resolved ones carry `— (closed)`.

**(d) No RESOLVED exception retains a live `deny.toml` ignore.** For each heading
in `docs/DEPENDENCY_EXCEPTIONS.md` whose title contains `RESOLVED`, its advisory ID
must **not** appear in `deny.toml`'s `ignore` list. Baseline: 2 such headings, 0
violations — which is the point, because the document already records at `:97-99`
and `:108` that those ignores were removed rather than left as dead entries. This
sub-check stops a resolved exception from being re-added and silently resurrected.

The guard must be **fail-closed and non-silent**: if the `ignore` array cannot be
parsed, or the exception document cannot be read, the guard fails with a message
naming the file, rather than passing because it found nothing. The `code_search`
helper's own comment at `scripts/check-architecture-guards.sh:4205-4207` states the
rule: "a guard that silently passes is worse than none."

The guard is registered in `docs/CI_ARCHITECTURE_GUARDS.md` under the supply-chain
invariants section that begins at `docs/CI_ARCHITECTURE_GUARDS.md:151` (which
already records Check 109's `deny.toml` canonicality and the
`docs/DEPENDENCY_EXCEPTIONS.md` retention record at `:156`), and in
`docs/VERIFICATION.md`'s guard table (`docs/VERIFICATION.md:79`).

**No other static guard is added.** The existing 176 checks continue to run
unchanged; `scripts/check-architecture-guards.sh` is invoked from `make check` at
`Makefile:193`.

## 7. Ordered work packages

### Work package A — Refresh the lockfile and clear the 9 Bucket-A entries

**Intent:** clear 9 of the 23 advisory entries with no manifest change at all,
establishing that the bulk of the backlog is a stale lockfile.

**Required changes:**

1. Run `cargo update` from the workspace root (workspace-wide refresh, so the
   patch-level bumps in §3.2 are all picked up in one pass). The dry run at this
   baseline proposed exactly: `ctutils` 0.4.2→0.4.5, `rustls` 0.23.45→0.23.46,
   `rustls-webpki` 0.103.15→0.103.16, `tokio` 1.53.1→1.54.1, `webpki-roots`
   1.0.9→1.0.11, `webpki-root-certs` 1.0.9→1.0.11, plus 149 other in-range
   updates.
2. Do **not** use `cargo update --breaking` or `-Z minimal-versions`. Do not
   hand-edit `Cargo.lock`.
3. Verify that the refresh did **not** float the Tauri closure.
   `cargo update --dry-run -p tauri-utils` reported *"Locking 0 packages to highest
   compatible versions"* at this baseline; the refresh must preserve that, because
   `Cargo.toml:86-92` states the exact pins exist to hold the closure on the
   MSRV-safe line. If `tauri-utils` moves to 2.10.x, the MSRV floor has been
   breached and §14 applies.
4. Re-run the advisory check and confirm that the 9 Bucket-A IDs no longer appear.

**Acceptance evidence:**

- `git diff --stat Cargo.lock` shows only version bumps, no new package
  introductions that are not already in the dependency tree of the bumped packages.
- The advisory check's entry count drops from 23 to 14, and the remaining 14 are
  exactly the Bucket-B and Bucket-C rows of §3.3 (rows 2, 3, 4, 5, 10, 13, 14,
  15, 16, 17, 18, 21, 22, 23).
- `cargo +1.89 check --workspace --no-default-features` still succeeds for the
  affected crates (the MSRV floor is unaffected by patch bumps, but the
  implementer must confirm it rather than assume it).

### Work package B — Upgrade `zip` to the 9.x line and clear 3 entries

**Intent:** clear the 3 `zip` entries with the upstream fix, rather than
recording them as exceptions, because the upstream fix exists and does not raise
MSRV (`zip` 9.1.0 declares `rust-version = "1.88"`, below the floor at
`Cargo.toml:39`).

**Required changes:**

1. Change both direct requirements from `zip = "2"` to `zip = "9"`:
   `crates/eggsec-mobile-lab/Cargo.toml:46` and `crates/eggsec/Cargo.toml:278-280`.
   Preserve the `optional = true` markers and the `mobile` feature wiring at
   `crates/eggsec/Cargo.toml:426`.
2. Migrate the `zip` call sites in `crates/eggsec-mobile-lab` to the 9.x API. The
   2.x `FileOptions<'k, T>` becomes `FileOptions<'_, '_, T>` in 9.x, and the 9.x
   `write` module exposes `ZipFileBuilder` / `PreparedZipFile` as the construction
   path. `SimpleFileOptions` remains available in 9.x.
3. Confirm that **no extraction API is introduced**. `crates/eggsec-mobile-lab`
   must still contain no `extract()`, `extract_to`, `unpack`, or `extract_dir` call
   site after the migration. If the migration cannot be completed without one,
   stop (§14).
4. Confirm the archive layout the mobile-lab tests assert is unchanged.

**Acceptance evidence:**

- `cargo deny --workspace --all-features check advisories` no longer lists
  RUSTSEC-2026-0367, -0368, or -0369.
- The `eggsec-mobile-lab` tests that read an IPA/APK fixture still pass, with the
  same entry names, compression method, and sizes.
- `cargo deny --workspace --all-features check bans` and `check licenses` remain
  green after the bump (a major bump can pull new transitive crates; the `[bans]`
  deny list and `[licenses]` allow list at `deny.toml:70-74` and `deny.toml:39-56`
  must still cover the result).

**Fallback, if the migration cannot be completed safely:** record the 3 entries
as exceptions with the "no `extract()` call site" compensating control from §3.4,
owner **eggsec-mobile-lab**, and the same review-by date as the rest. The
implementer must state in the closure record which path was taken and why. A
partial migration that leaves the tree uncompilable is not an acceptable outcome.

### Work package C — Record the 11 policy- and upstream-blocked exceptions

**Intent:** give every remaining non-upgradable entry an owner, a review-by
date, and a full record.

**Required changes.** Add the following `ignore` entries to `deny.toml`'s
`[advisories] ignore` array, in the file's existing style (one line per entry;
`{ id = "...", reason = "<what / why / dependency path / policy note>. See docs/DEPENDENCY_EXCEPTIONS.md." }`),
placed after the existing entries and grouped under a new comment header that
names the milestone:

```toml
    # --- Milestone 015: advisories with no safe upgrade under repo policy ---
    { id = "RUSTSEC-2024-0429", reason = "glib 0.18.5 unsound; transitive via gtk 0.18->tauri 2.11.6 shell closure (eggsec-gui, Linux only). Upgrade to glib>=0.20.0 is unreachable: no published muda/tao/wry version uses gtk-rs 0.20+, and tauri 2.12.5 still requires gtk ^0.18. Blocked by upstream, not by upstream policy. See docs/DEPENDENCY_EXCEPTIONS.md." },
    { id = "RUSTSEC-2024-0370", reason = "proc-macro-error 1.0.4 unmaintained; transitive via gtk3-macros 0.18.2 and glib-macros 0.18.5 (gtk-rs 0.18 stack in the eggsec-gui shell closure). 1.0.4 is the latest published version; the gtk-rs 0.20+ macro crates do not depend on it, so this clears with RUSTSEC-2024-0429. See docs/DEPENDENCY_EXCEPTIONS.md." },
    { id = "RUSTSEC-2026-0346", reason = "rustls 0.21.12 CRL/OCSP handling; transitive via rustls 0.21->tokio-rustls 0.24->tiberius 0.12.3 (db-pentest-mssql-tiberius marker, lab-only). tiberius 0.13.0 upgrades to rustls 0.23 but pulls tokio-rustls with default features, enabling the aws-lc-rs provider that [bans] deny forbids (ring-only TLS). Blocked by policy, not by upstream. See docs/DEPENDENCY_EXCEPTIONS.md." },
    { id = "RUSTSEC-2026-0349", reason = "rustls 0.21.12 same tiberius path as RUSTSEC-2026-0346; requires rustls>=0.23.46, which tiberius 0.13.0 reaches only via the aws-lc-rs-default tokio-rustls that [bans] deny forbids (ring-only TLS). Blocked by policy, not by upstream. See docs/DEPENDENCY_EXCEPTIONS.md." },
    { id = "RUSTSEC-2026-0382", reason = "rustls-webpki 0.101.7 same tiberius path as RUSTSEC-2026-0104; requires rustls-webpki>=0.103.16, which tiberius 0.13.0 reaches only via the aws-lc-rs-default tokio-rustls that [bans] deny forbids (ring-only TLS). Blocked by policy, not by upstream. See docs/DEPENDENCY_EXCEPTIONS.md." },
    { id = "RUSTSEC-2024-0449", reason = "unsafe-libyaml 0.2.11 unmaintained; transitive via serde_yaml_neo 0.11.0 (eggsec config/policy/skills/templates/api-schema and eggsec-web-proxy config/rules). 0.2.11 is the latest published version and serde_yaml_neo 0.11.0 hard-depends on it; unmaintained, not a vulnerability. See docs/DEPENDENCY_EXCEPTIONS.md." },
    { id = "RUSTSEC-2025-0100", reason = "unic-ucd-ident 0.9.0 unmaintained; transitive via urlpattern 0.3.0->tauri-utils 2.9.3 (eggsec-gui shell closure). urlpattern 0.6.0 drops the unic-* set, but tauri-utils 2.10.x requires urlpattern ^0.6 and declares MSRV 1.90, above the workspace 1.89 floor. Blocked by the MSRV floor, not by upstream. See docs/DEPENDENCY_EXCEPTIONS.md." },
    { id = "RUSTSEC-2025-0075", reason = "unic-char-range 0.9.0 unmaintained; same urlpattern 0.3.0->tauri-utils 2.9.3 path as RUSTSEC-2025-0100. Clears with urlpattern 0.6.0, which requires tauri-utils 2.10.x (MSRV 1.90, above the workspace 1.89 floor). See docs/DEPENDENCY_EXCEPTIONS.md." },
    { id = "RUSTSEC-2025-0080", reason = "unic-common 0.9.0 unmaintained; same urlpattern 0.3.0->tauri-utils 2.9.3 path as RUSTSEC-2025-0100. Clears with urlpattern 0.6.0, which requires tauri-utils 2.10.x (MSRV 1.90, above the workspace 1.89 floor). See docs/DEPENDENCY_EXCEPTIONS.md." },
    { id = "RUSTSEC-2025-0081", reason = "unic-char-property 0.9.0 unmaintained; same urlpattern 0.3.0->tauri-utils 2.9.3 path as RUSTSEC-2025-0100. Clears with urlpattern 0.6.0, which requires tauri-utils 2.10.x (MSRV 1.90, above the workspace 1.89 floor). See docs/DEPENDENCY_EXCEPTIONS.md." },
    { id = "RUSTSEC-2025-0098", reason = "unic-ucd-version 0.9.0 unmaintained; same urlpattern 0.3.0->tauri-utils 2.9.3 path as RUSTSEC-2025-0100. Clears with urlpattern 0.6.0, which requires tauri-utils 2.10.x (MSRV 1.90, above the workspace 1.89 floor). See docs/DEPENDENCY_EXCEPTIONS.md." },
```

The implementer may adjust the wording of the `reason` strings to match the file's
voice, but **must** keep, in every entry: the affected crate and version, the
dependency path, the upgrade that would clear it, the reason it is not adopted,
and the trailing `See docs/DEPENDENCY_EXCEPTIONS.md.`.

Then add the matching records to `docs/DEPENDENCY_EXCEPTIONS.md` under
`## Active Exceptions`, one `###` heading per cluster, using the ten-field schema
of `:54-65` exactly, with these owners and review-by dates:

| Heading | IDs | Owner | Review-by | Blocker (one line) |
|---------|-----|-------|-----------|--------------------|
| `### RUSTSEC-2024-0429 — glib 0.18.5 unsound` | RUSTSEC-2024-0429, RUSTSEC-2024-0370 | eggsec-gui | 2026-12-12 | gtk-rs 0.20 unreachable: no published `muda`/`tao`/`wry` uses gtk-rs 0.20+; `tauri` 2.12.5 still requires `gtk ^0.18` |
| `### RUSTSEC-2026-0346 / RUSTSEC-2026-0349 / RUSTSEC-2026-0382 — rustls 0.21.12 / rustls-webpki 0.101.7` | 0346, 0349, 0382 | eggsec-db-lab | 2026-12-12 | `tiberius` 0.13.0 requires `tokio-rustls` 0.26 with default features, enabling `aws-lc-rs`, which `[bans] deny` forbids (ring-only) |
| `### RUSTSEC-2024-0449 — unsafe-libyaml 0.2.11 unmaintained` | 0449 | eggsec (config) / eggsec-web-proxy | 2026-12-12 | `unsafe-libyaml` 0.2.11 is the latest published version; `serde_yaml_neo` 0.11.0 hard-depends on it |
| `### RUSTSEC-2025-0075 / -0080 / -0081 / -0098 / -0100 — unic-* 0.9.0 unmaintained` | 0075, 0080, 0081, 0098, 0100 | eggsec-gui | 2026-12-12 | `urlpattern` 0.6.0 clears the set, but `tauri-utils` 2.10.x declares MSRV 1.90, above the workspace 1.89 floor |

The `2026-12-12` review-by date is the date already in force across the document's
active exceptions (`:64`, `:79`, `:170`, `:185`) and is 49 days from the plan date
— inside the 90-day maximum the policy sets at `:47`. The `unic-*` record must
additionally carry the full removal recipe from §3.4 so that the review is
mechanical rather than investigative.

**Acceptance evidence:**

- `deny.toml` parses and the new entries are present with the house-style reason
  strings.
- Every new ID appears in `docs/DEPENDENCY_EXCEPTIONS.md` with a non-empty
  `| Owner |` and a `| Review-by |` of `2026-12-12`.
- The advisory check reports zero entries.

### Work package D — Add guard Check 190 and register it

**Intent:** make "no unowned exceptions" a mechanically enforced property rather
than a review convention, so the failure mode this milestone closes cannot recur.

**Required changes:** implement the guard exactly as specified in §6
"Documentation and static guards", with the four sub-checks (a)–(d), the
fail-closed/non-silent behaviour, and the file's existing banner/section style.

**Acceptance evidence:**

- `bash scripts/check-architecture-guards.sh` reports a new banner
  `--- Check 190: ...` and exits zero with the milestone's own `deny.toml` and
  `docs/DEPENDENCY_EXCEPTIONS.md` in place.
- The guard is proven to fail on a negative fixture: temporarily removing the
  `| Owner |` row from one exception record, or the `reason` from one `ignore`
  entry, makes the guard exit non-zero. The implementer must demonstrate both
  negatives and record the demonstration in the closure record. A guard that has
  never been seen to fail is not evidence.
- The guard is registered in `docs/CI_ARCHITECTURE_GUARDS.md` (the supply-chain
  section beginning at `:151`) and in `docs/VERIFICATION.md`'s guard table
  (`:79`).

### Work package E — Document, and complete the contract

**Intent:** make the policy state legible, then prove the contract completes.

**Required changes:** the documentation updates in §12, then the full
verification sweep in §11.

**Acceptance evidence:** the closure record contains the §15 evidence set, with
every command's real exit status recorded.

## 8. Failure, cancellation, restart, and contention semantics

This milestone has no runtime, task, or session semantics of its own. The
relevant failure semantics are those of the **tooling** and of the **dependency
graph**:

**Lockfile refresh fails because the network is unavailable.** `cargo update`
requires crates.io. In an offline CI runner it fails and the lockfile is not
written; `cargo deny` then re-reads the *existing* lockfile and re-reports the
same 23 entries. This is correct behaviour and must not be worked around by
committing a lockfile generated elsewhere without CI's own verification. The
implementer must ensure the committed `Cargo.lock` is regenerated in an
environment whose advisory-database state matches CI's, because `deny.toml:16-17`
fetches the advisory database live.

**`cargo-deny` is absent.** `make check-deps` fails closed (`Makefile:153`): the
target prints `FAIL: cargo-deny not found.` to stderr and exits 1. This is the
intended behaviour and must be preserved verbatim — the new guard must not depend
on `cargo-deny` being present, since the guard runs in the same contract after
`check-deps`. It reads `deny.toml` and `docs/DEPENDENCY_EXCEPTIONS.md` with plain
text tools only.

**The advisory database changes between the implementer's run and CI's run.**
`deny.toml:16-17` points at the live `rustsec/advisory-db`. A new advisory
published against a crate in this workspace would fail CI's `check-deps` even
though the local run was green. This is the intended fail-closed posture of a
supply-chain gate. It is a stop condition (§14), and the closure record must state
the advisory-database revision observed locally and in CI if one can be
determined.

**Local tool version differs from CI's.** Local `cargo-deny` is 0.20.2; CI
installs `cargo-deny@0.19.0` (`.github/workflows/ci.yml:67`). The advisory database
and the `deny.toml` semantics are the same, but the tool's *reporting* may differ —
entry ordering, extra diagnostics, and, across a version boundary, whether an
advisory is surfaced at all. The implementer must record the local version and
must not treat a local green as proof of a CI green when the versions differ. The
CI job is the authority.

**Restart and recovery.** There is no long-lived state to recover: the only
persisted artifact is `Cargo.lock`, which cargo rewrites atomically. An
interrupted `cargo update` leaves the previous lockfile intact; re-running it is
idempotent. An interrupted guard run leaves no state.

**Contention.** The only contended resources are the crates.io index and the cargo
package cache. Two concurrent `cargo update` runs in one workspace contend on the
cargo lock; cargo serialises them and the second waits. There is no partial-write
hazard for `deny.toml` or `docs/DEPENDENCY_EXCEPTIONS.md` — they are hand-edited,
and a partial edit is caught by the guard.

**Cancellation.** `cargo update` supports `--dry-run`; cargo's own cancellation on
SIGINT leaves the lockfile untouched. No tokio task is spawned by this milestone,
so `eggsec-runtime::race_with_cancel` is not involved and no timeout wrapper is
required.

**Duplicate delivery.** The milestone is idempotent in the sense that matters:
re-running Work Package A on an already-refreshed lockfile is a no-op, and
re-running Work Package C on an already-recorded exception is a no-op provided the
guard's sub-checks hold. The one non-idempotent step is Work Package D's
negative-fixture demonstration, which temporarily mutates a policy file — it must
be performed on a scratch copy or reverted immediately, and the closure record
must confirm the working tree is clean afterwards.

## 9. Compatibility and migration

**Backward compatibility.** No wire, protocol, DTO, or persisted-format
compatibility surface is touched. The daemon protocol remains additive-only
(`plans/subsystems/desktop-gui-frontend-roadmap.md:1090-1098`); the GUI re-attaches
after a restart rather than restoring its own state (`:1039-1043`).

**`zip` 2.x → 9.x migration.** The compatibility surface here is the *archive
layout* the mobile-lab produces and consumes. The implementer must verify, with
the existing mobile-lab tests, that:

- entry names and ordering are unchanged;
- the compression method per entry is unchanged (`Stored` where it was `Stored`,
  `Deflated` where it was `Deflated`);
- no entry gains SUID/SGID bits or a symlink that the 2.x writer did not emit;
- the reader still rejects a malformed archive the way it did before.

If the 9.x writer's default compression differs from 2.x's, the implementer must
set it explicitly rather than accept the new default, and must say so in the
closure record.

**`Cargo.lock` refresh.** A lockfile bump is a normal supply-chain event. It does
not change any public API of the workspace. It may change the behaviour of
transitive crates that patch or minor-bump, so the implementer must run the full
test sweep (§11) rather than only the advisory check.

**Fallback compatibility.** If Work Package B's migration is abandoned in favour
of the exception route, no compatibility surface changes at all and the 3 entries
are recorded instead.

**Removal criteria for the legacy paths.** The `zip` 2.x requirement is removed
from `crates/eggsec-mobile-lab/Cargo.toml:46` and `crates/eggsec/Cargo.toml:278-280`
as part of the upgrade — no 2.x code path is left behind, and no
`zip = ">=2, <9"`-style range is introduced to keep both alive. The `rustls` 0.21 /
`tokio-rustls` 0.24 path is **not** removed; it is retained under exception and
stays live until either the ring-only rule changes or `tiberius` exposes a
ring-only path.

## 10. Required tests

This milestone's regression surface is policy-shaped, not behaviour-shaped. The
tests below are the "regression evidence that would have caught" the unclosed
findings required by `plans/implementation/README.md:179`: without them, a future
milestone can re-introduce an unowned exception or an unrecorded advisory without
any test failing.

### Focused unit tests

No new Rust unit tests are required in the workspace crates: no crate's runtime
behaviour changes, and the `zip` migration is covered by existing `eggsec-mobile-lab`
tests.

One focused test **is** required, and it is a test of the policy files rather
than of Rust code — a script-level assertion in the new guard's companion test or
as a standalone script under `scripts/`, in the style of
`scripts/check_platform.sh`. It asserts, as a *pure text* function with no network
and no `cargo-deny` invocation:

1. every `deny.toml` `[advisories] ignore` entry has a non-empty `reason`;
2. every such `reason` contains `docs/DEPENDENCY_EXCEPTIONS.md`;
3. every ignored ID has a matching `docs/DEPENDENCY_EXCEPTIONS.md` heading with a
   non-empty `| Owner |` and a `| Review-by |` matching `^[0-9]{4}-[0-9]{2}-[0-9]{2}$`
   or the literal `— (closed)`;
4. no `RESOLVED` heading's ID appears in the `ignore` list;
5. the parse of either file failing is an **error**, not an empty result.

### Integration tests

- The `check-deps` target's full five-invocation sequence must be exercised
  together, not just the `advisories` sub-check, because Work Package B's `zip`
  bump can change the `bans` and `licenses` outcomes. The closure record must
  contain the output of the whole target.
- `make check` must be exercised end-to-end, because `check-deps` is embedded at
  `Makefile:179` and its position in the contract is the thing this milestone
  fixes.
- The `zip` migration must be exercised through the `eggsec-mobile-lab` test
  suite that reads an IPA/APK fixture, since that suite is the only thing that
  proves the archive layout is unchanged.

### Restart and recovery tests

Not applicable to the runtime: this milestone introduces no long-lived state, no
task, no session, and no daemon state.

The equivalent property to test is **idempotent re-run**: running the lockfile
refresh and the guard a second time on an already-updated tree must produce no
further diff and a zero exit status. The implementer must record that
`git status --porcelain` is empty after a second `cargo update` and after a second
guard run.

### Contention and cancellation tests

Not applicable to the runtime.

The one contention-relevant property is that a **failed or interrupted
`cargo update` does not corrupt the lockfile**: cargo writes `Cargo.lock`
atomically, so an interrupted refresh leaves the previous lockfile intact and the
tree still builds. The implementer must not leave a partially-refreshed
`Cargo.lock` in the tree, and the closure record must show a clean `git status`
after the refresh.

### Security and negative tests

These are the tests that constitute the regression evidence the corrective-class
rule requires. All five are **negative** tests of the new guard, and each must be
demonstrated to fail:

1. **Unowned exception.** Remove the `| Owner |` row from one
   `docs/DEPENDENCY_EXCEPTIONS.md` record → Check 190 must exit non-zero, naming
   the offending advisory ID.
2. **Undated exception.** Set one `| Review-by |` to a value that is neither an
   ISO date nor `— (closed)` → Check 190 must exit non-zero.
3. **Undocumented ignore.** Add an `ignore` entry to `deny.toml` for an ID that
   has no record in `docs/DEPENDENCY_EXCEPTIONS.md` → Check 190 must exit
   non-zero.
4. **Resurrected resolved exception.** Add the `RUSTSEC-2025-0119` ignore entry
   back to `deny.toml` → Check 190 must exit non-zero on sub-check (d).
5. **Undocumented reason.** Add an `ignore` entry with an empty `reason` →
   Check 190 must exit non-zero on sub-check (a).

Each demonstration must be recorded in the closure record with the guard's actual
output, and the working tree must be restored to a clean state afterwards.

### Migration and compatibility tests

- **Archive-layout compatibility** for the `zip` 9.x migration, as specified in
  §9: entry names, ordering, per-entry compression method, no SUID/SGID bits, no
  new symlinks, and unchanged rejection behaviour for a malformed archive. These
  assertions belong in the existing `eggsec-mobile-lab` tests, not in new tests
  written only for this milestone.
- **MSRV compatibility**: `make check-msrv` must still build the workspace on
  Rust 1.89 after the refresh and after the `zip` bump. `zip` 9.1.0 declares
  `rust-version = "1.88"`, so the bump must not raise the floor; the implementer
  must verify rather than assume.
- **Python compatibility**: no Python-facing dependency path is implicated by the
  upgrades in this milestone, so `make check-python` is **not** required. The
  reasoning, stated so a reviewer can check it: none of the 23 affected crates
  (`ctutils`, `glib`, `proc-macro-error`, `rustls`, `rustls-webpki`, `tokio`,
  `unic-*`, `unsafe-libyaml`, `webpki-roots`, `webpki-root-certs`, `zip`) appears
  in the `eggsec-python` dependency closure, and none is reachable from
  `crates/eggsec-python/pyproject.toml` or from the Python stubs. The `tokio` bump
  does touch `eggsec-python`'s transitive closure, but a patch-level bump within
  the same feature set cannot change a Python stub's signature. If the
  implementer's `cargo update` *does* change anything under `crates/eggsec-python/`
  beyond the `tokio` patch bump, `make check-python` becomes required and the
  implementer must run it.
- **Database and storage compatibility**: not applicable; no storage change.

## 11. Required verification commands

These are the commands a future implementer runs. They are drawn from the real
`Makefile` at this baseline; no target is invented. **This plan asserts no
command's pass/fail outcome** — the closure record is where results are recorded.

```bash
# 1. The dependency-policy gate itself (the milestone's primary objective).
#    Five cargo-deny invocations; fails closed when cargo-deny is absent.
make check-deps

# 2. The advisory sub-check in isolation, with stderr captured — cargo-deny
#    writes its diagnostics to stderr, so `2>&1` is mandatory.
cargo deny --workspace --all-features check advisories > /tmp/m015_advisories.txt 2>&1

# 3. The other four policy sub-checks, individually, to attribute any failure.
cargo deny --workspace --all-features check
cargo deny --workspace --all-features check bans
cargo deny --workspace --all-features check licenses
cargo deny --workspace --all-features check sources

# 4. The lockfile refresh. `--dry-run` first; then the real refresh.
cargo update --dry-run
cargo update

# 5. The MSRV floor (requires `rustup toolchain install 1.89`).
make check-msrv

# 6. The architecture guards, including the new Check 190.
bash scripts/check-architecture-guards.sh

# 7. The full mandatory Rust contract. `check-deps` runs at Makefile:179,
#    between the compile checks and clippy/tests/guards.
make check

# 8. The GUI shell profile, because the `unic-*` and `glib` clusters live in
#    the GUI's Tauri closure.
make check-gui

# 9. The GUI shell prerequisite probe (reports; does not fail).
make check-gui-prereqs
```

`make check-python` is **not** in this list, for the reasons in §10 "Migration and
compatibility tests". It becomes required only if the implementer's `cargo update`
changes anything under `crates/eggsec-python/` beyond the `tokio` patch bump.

Optional deep checks, **not** part of this milestone's contract and not to be
claimed in the closure record unless actually run: `make check-full`,
`make check-features-individual`, `make clippy-domain`, `make test-tui-pty`
(`docs/VERIFICATION.md:229-244`).

Per `docs/VERIFICATION.md:247-250`, the dependency-policy gate runs on **every
PR**, both inside `make check` and as an independent `dependency-policy` CI job
(`docs/VERIFICATION.md:305-306`).

## 12. Documentation updates

- **`docs/DEPENDENCY_EXCEPTIONS.md`** — add the four new exception records
  specified in Work Package C, each with the ten-field schema of `:54-65`, an
  owner from §3.6, `| Created |` of the implementation date, `| Review-by |` of
  `2026-12-12`, and a `| Blocker |` line carrying the exact clearing condition.
  The `unic-*` record must carry the full removal recipe from §3.4. Also update
  the review schedule at `:261` if the implementation date moves the 90-day
  window.
- **`docs/DEPENDENCY_EXCEPTIONS.md` — heading placement.** Move the two
  `RESOLVED` headings (`:82` RUSTSEC-2025-0119, `:101` RUSTSEC-2024-0436) out of
  `## Active Exceptions` and into a new `## Resolved Exceptions` heading placed
  after it. This is a documentation-only change: the policy state is already
  correct (both ignores were removed from `deny.toml`, per `:97-99` and `:108`),
  and the guard's sub-check (d) enforces that it stays correct. §3.5 records the
  assessment that this is a structural inconsistency rather than a violation.
- **`docs/CI_ARCHITECTURE_GUARDS.md`** — register Check 190 in the supply-chain
  invariants section beginning at `:151`, alongside the existing Check 109
  (`deny.toml` canonicality, `.cargo/audit.toml` absence, `make check-deps`
  presence) and the retained-record note at `:156`.
- **`docs/VERIFICATION.md`** — add Check 190 to the guard table at `:79` and, if
  the `zip` 9.x migration changes any mobile-lab behaviour, to the defect-class
  table at `:63-79`.
- **`architecture/supply_chain.md`** — record the milestone's outcome: the
  Bucket-A lockfile refresh, the `zip` upgrade, the four new exception clusters
  with their owners and blockers, and the guard. This is the subsystem's
  deep-dive document for supply chain and must not drift from `deny.toml`.
- **`architecture/desktop_gui.md`** — record that the `unic-*` and
  `glib`/`proc-macro-error` clusters are reached only through the GUI's Tauri
  shell closure, that `urlpattern` 0.6.0 (via `tauri-utils` 2.10) is the clearing
  path for the `unic-*` set, and that the `glib`/`proc-macro-error` pair has no
  published clearing path. The GUI's dependency *direction* is unchanged.- **`plans/subsystems/desktop-gui-frontend-roadmap.md`** — the operator-value
  correction is **already applied** at `:1060-1071` by a concurrent amendment
  dated 2026-10-24 (see §0.4), so there is nothing to amend. The implementer's
  only roadmap obligation is a **consistency check**: confirm that the amended
  paragraph's "3 cleared by the `zip` upgrade" claim matches what Work Package B
  actually achieves, and that its attribution of the `glib` entry ("the exact
  `tauri` pin and the MSRV floor") is extended to note the upstream blocker
  established in §3.4. The roadmap's milestone entry, class, dependencies,
  deliverable boundary, exit conditions, and deferred-work statement are **not**
  to be rewritten by the implementer.
- **`plans/registry.md`** — register this plan (the handoff rule at
  `plans/implementation/README.md:167`).
- **Not changed:** `plans/closure/desktop-gui-frontend/010-daemon-full-manual-capability-policy-closure.md`.
  `plans/implementation/README.md:179` forbids rewriting the original plan's
  completion record. This milestone's own closure record is a **new** file.

## 13. Acceptance criteria

1. `make check-deps` completes with a zero exit status, and each of its five
   `cargo deny` invocations is recorded with its own exit status.
2. `cargo deny --workspace --all-features check advisories` reports **zero**
   advisory entries, with the captured output recorded.
3. Of the original 23 entries: the 9 Bucket-A entries are cleared by a lockfile
   refresh with no manifest change; the 3 `zip` entries are cleared by the 9.x
   upgrade (or, on the documented fallback, recorded as exceptions); the remaining
   11 are recorded as exceptions.
4. `deny.toml` contains an `ignore` entry for every advisory that is not upgraded,
   and **no** `ignore` entry for an advisory that is upgraded — i.e. there is
   neither an unrecorded advisory nor a stale ignore.
5. Every `ignore` entry in `deny.toml` has a non-empty `reason` naming the
   affected crate, the dependency path, and the clearing condition.
6. Every advisory ID in `deny.toml`'s `ignore` list has a record in
   `docs/DEPENDENCY_EXCEPTIONS.md` carrying a non-empty `| Owner |`, a
   `| Review-by |` date, and a `| Blocker |`.
7. `make check` completes end to end.
8. `make check-msrv` still builds the workspace on Rust 1.89, and the Tauri
   closure has not floated onto the 1.90-MSRV line.
9. `bash scripts/check-architecture-guards.sh` exits zero and reports Check 190;
   all five negative demonstrations of §10 "Security and negative tests" were
   performed and recorded.
10. `make check-gui` completes on a host with the webview SDK present, or is
    recorded as prerequisite-gated on a host without it.
11. No `.cargo/audit.toml` exists and no `cargo audit` configuration was added.
12. `[bans] deny = ["aws-lc-rs"]` and the ring-only TLS rule are unchanged.
13. `git status --porcelain` is clean apart from the intended changes, and no
    unrelated file was modified.

## 14. Stop conditions

The agent must stop and report rather than improvise when:

- **the advisory set moves.** A new advisory is published against a crate in
  this workspace between the implementer's run and CI's run, or an advisory in
  the §3.3 table is withdrawn. The advisory database is fetched live
  (`deny.toml:16-17`), so the backlog is not a fixed input. Report the new entry
  and its path; do not extend the milestone's scope to absorb it without a
  decision.
- **the Tauri closure floats.** `cargo update` moves `tauri-utils` to 2.10.x or
  `tauri` to 2.12.x. That breaches the MSRV floor (`Cargo.toml:39`,
  `Cargo.toml:86-92`). Re-pin and report; do not raise the floor.
- **`cargo update` proposes a breaking change** that is not one of the six
  Bucket-A bumps. Report the proposal and its effect before accepting it.
- **the `zip` 9.x migration cannot be completed safely** — in particular if it
  cannot be completed without introducing an archive-extraction call site, or if
  the archive layout assertions cannot be preserved. Take the documented fallback
  (three exceptions with the "no `extract()`" compensating control) and record
  which path was taken.
- **a new transitive dependency** introduced by the `zip` bump is not covered by
  the `[licenses]` allow list (`deny.toml:39-56`) or is matched by a `[bans]`
  deny rule (`deny.toml:70-74`). Adding a licence or relaxing a ban is a policy
  decision and is out of scope.
- **the ring-only TLS rule or the `[bans] deny = ["aws-lc-rs"]` entry would have
  to change** to adopt `tiberius` 0.13. That is a policy decision requiring an
  ADR; record the exception instead.
- **the MSRV floor would have to rise to 1.90** to clear the `unic-*` cluster.
  Record the exception with its removal recipe instead.
- **external evidence is required but unavailable.** If crates.io is unreachable
  for the refresh, or the advisory database cannot be fetched, the milestone
  cannot be verified; report the blockage rather than asserting a result.
- **`cargo-deny` is absent.** `make check-deps` fails closed by design
  (`Makefile:153`); this is a blockage to report, not a reason to install a tool
  and re-run unrecorded.
- **the local `cargo-deny` version differs from CI's** (0.20.2 locally versus
  0.19.0 at `.github/workflows/ci.yml:67`) in a way that changes which advisories
  are reported. Report the divergence; the CI job is authoritative.
- **repository evidence contradicts a canonical invariant** — for example if
  `.cargo/audit.toml` exists, or if `[advisories]`'s `unmaintained`/`unsound`
  settings differ from those quoted in this plan.
- **scope would expand into another subsystem roadmap.** The `zip` migration
  touches `crates/eggsec-mobile-lab`, which is a different subsystem's crate. If
  the migration requires changes beyond the `zip` call sites — a change to the
  mobile-lab's parsing contract, its report DTOs, or its feature flags — that is
  a different milestone's scope. Report it.

## 15. Closure evidence required

The closure record for this milestone must contain:

1. The **baseline**: branch and commit SHA, and the `cargo-deny` version used locally.
2. The **before measurement**: the captured advisory output and the counting rule,
   showing 23 entries / 20 unique IDs, and the per-entry table as §3.3.
3. The **after measurement**: the captured advisory output and the counting rule,
   showing **zero** entries, with each of the five `make check-deps`
   sub-invocations recorded with its own exit status.
4. The **`Cargo.lock` diff summary**: which crates moved and which did not, with
   an explicit statement that the Tauri closure did not float.
5. The **`zip` migration record**: the manifest changes, the call-site changes,
   the archive-layout compatibility evidence, and — if the fallback was taken —
   an explicit statement of which path was taken and why.
6. The **exception records**: the four new `docs/DEPENDENCY_EXCEPTIONS.md` entries
   with their owners and review-by dates, and the new `deny.toml` `ignore` entries
   with their reason strings.
7. The **guard evidence**: the Check 190 banner and its zero exit status, plus the
   **actual output of all five negative demonstrations** in §10 "Security and
   negative tests", with the working tree confirmed clean afterwards.
8. The **contract evidence**: the recorded exit status of `make check-deps`,
   `make check-msrv`, `bash scripts/check-architecture-guards.sh`, `make check`,
   `make check-gui`, and `make check-gui-prereqs`. No command may be reported as
   passing unless it was run.
9. The **scope evidence**: an explicit statement that `make check-python` was not
   run, with the reasoning from §10 "Migration and compatibility tests", or its
   recorded result if the `zip` bump changed anything under `crates/eggsec-python/`.
10. The **roadmap consistency check**: confirmation that the already-applied
    amendment at `plans/subsystems/desktop-gui-frontend-roadmap.md:1060-1071` still
    matches the measured outcome (§0.4, §12), including the `zip` upgrade's
    actual disposition and the `glib` upstream blocker.
11. A statement that `plans/closure/desktop-gui-frontend/010-daemon-full-manual-capability-policy-closure.md`
    was **not** modified, per `plans/implementation/README.md:179`.
12. The **residual backlog**: any advisory entry that remains unrecorded or
    un-upgraded after the milestone, with its reason. A green `check-deps` with an
    undocumented residual is a failed milestone.

## 16. Handoff notes

**Known hazards.**

- **The advisory database is live.** `deny.toml:16-17` fetches `rustsec/advisory-db`
  at check time. The 23-entry set measured here is a snapshot of 2026-10-24, not a
  constant. Treat §3.3 as the milestone's *input*, and expect the closure record's
  output set to differ if a new advisory lands in between.
- **cargo-deny version drift.** Local 0.20.2, CI 0.19.0
  (`.github/workflows/ci.yml:67`). A local green is not proof of a CI green.
- **The `cargo update` blast radius is larger than the milestone's.** The dry run
  at this baseline proposed 149 version bumps and 35 additions/removals across
  the whole workspace, not just the six Bucket-A crates. The implementer must
  review the full `Cargo.lock` diff, not only the six bumps that clear advisories.
  This is why the full test sweep in §11 is mandatory.
- **Do not run `cargo update --breaking` or `-Z minimal-versions`.**
- **Do not relax `unmaintained`/`unsound` to make the gate green.** That is a
  silent policy weakening, explicitly out of scope (§5).
- **Do not introduce `.cargo/audit.toml` or any `cargo audit` configuration.**

**Environment requirements.**

- `cargo-deny` must be installed for `make check-deps` (it fails closed without
  it, `Makefile:153`).
- `rustup toolchain install 1.89` for `make check-msrv` (`Makefile:134`).
- `rg` (ripgrep) for the guards (`scripts/check-architecture-guards.sh:4211`).
- The GUI shell profile needs the platform webview SDK; `make check-gui-prereqs`
  reports but does not fail (`Makefile:206-207`), and `make check`'s GUI step is
  prerequisite-gated (`Makefile:167`).
- Network access to crates.io for the refresh and to the advisory database for
  the check.

**Serial-test requirements.** None introduced. The guard is a text check and is
fast; the existing test suites retain their own serial requirements.

**Never run the full suite as root** (`AGENTS.md`).

**Preserved user changes.** This plan introduces **no** working-tree change of its
own beyond this file. At the time of writing the worktree also carried a modified
`plans/subsystems/desktop-gui-frontend-roadmap.md` and untracked plan files
`012-daemon-socket-path-and-bind-contract.md`,
`013-report-bridge-full-coverage.md`, and
`014-gui-qualification-driver-and-runbook.md`, all produced by concurrent workers.
**None of them is this plan's to touch, and none may be reverted.** The implementer
must confirm `git status --porcelain` shows only the intended changes to
`deny.toml`, `Cargo.lock`, `docs/DEPENDENCY_EXCEPTIONS.md`, the manifests, the
`zip` call sites, and the guard script.

**Concurrent-edit hazard.** `plans/subsystems/desktop-gui-frontend-roadmap.md` was
being edited while this plan was written, and a milestone-12 amendment inserted
~20 lines ahead of the M15 entry (moving it from line 1012 to 1042). Every roadmap
citation here was re-verified after that edit settled, but the file may move
again: locate a roadmap clause by its quoted anchor text, not by its line number.
The files this plan cites most heavily — `deny.toml`, `Makefile`, `Cargo.toml`,
`docs/DEPENDENCY_EXCEPTIONS.md`, `docs/VERIFICATION.md`,
`docs/CI_ARCHITECTURE_GUARDS.md`, and `scripts/check-architecture-guards.sh` —
were **not** under concurrent edit at the time of writing.

**Scope discipline for the implementer.** The implementer may adjust file-level
mechanics (`plans/implementation/README.md:169`), but may not: weaken a canonical
invariant, raise the MSRV floor, relax the ring-only TLS rule, relax
`unmaintained`/`unsound`, add a `[licenses]` allow entry or a `[bans]` deny
relaxation, or rewrite the M010 closure record. If any of those appears
necessary, that is a stop condition (§14), not an implementation detail.

**The one open decision this milestone hands forward.** The five `unic-*` entries
have a mechanical clearing path that is gated on a workspace-wide MSRV decision
(`Cargo.toml:39`) the roadmap does not own. This milestone records that path in
the exception record so the review on 2026-12-12 is a decision about the MSRV
floor rather than an investigation. Raising the floor is **not** this milestone's
work, and no part of this plan should be read as proposing it.
