# Desktop GUI Frontend Milestone 015 — Closure Status

Status: closed

Source implementation plan:

- `plans/implementation/desktop-gui-frontend/015-dependency-advisory-backlog.md`

Source subsystem roadmap:

- `plans/subsystems/desktop-gui-frontend-roadmap.md#milestone-15--the-dependency-advisory-backlog`
  (heading at `plans/subsystems/desktop-gui-frontend-roadmap.md:1042`)

Repository baseline reviewed: `0f5dbb26` (branch `feat/auto-20261022-76692679`)

Implementation commits:

- `6de6340e` — feat(desktop-gui): implement the desktop GUI milestones 012 to 015

Tools used for the measurements in §4:

- `cargo-deny 0.20.2` (local; CI installs `cargo-deny@0.19.0` — see §10, finding F4)
- `cargo 1.99.1` / `rustc 1.99.1`; `rustup toolchain 1.89` for the MSRV gate
- `ripgrep 15.2.0`

## 1. Executive finding

**The repository's dependency-policy gate is green and the mandatory Rust
contract's first blocking step is no longer blocking.** The 23-entry advisory
backlog that milestone 010's closure record inherited and did not own is
closed: `cargo deny --workspace --all-features check advisories` now reports
**zero** entries, and all five `make check-deps` sub-invocations exit zero.

The backlog was disposed of along the plan's three-way split, and the split
held exactly as the plan measured it:

- **9 entries cleared by a lockfile refresh alone**, with no manifest change:
  `ctutils` 0.4.2→0.4.5, `rustls` 0.23.45→0.23.46, `rustls-webpki`
  0.103.15→0.103.16, `tokio` 1.53.1→1.54.1, `webpki-roots` 1.0.9→1.0.11,
  `webpki-root-certs` 1.0.9→1.0.11.
- **3 entries cleared by the `zip` 9.x major bump** (RUSTSEC-2026-0367 /
  -0368 / -0369), with the archive layout proven byte-identical (§7).
- **11 entries recorded as owned, dated exceptions** in `deny.toml` and
  `docs/DEPENDENCY_EXCEPTIONS.md`, in four clusters, each with an owner, a
  `2026-12-12` review-by date, and the exact clearing condition.

**What this milestone did NOT do**, and why: it did not relax the ring-only
TLS rule, did not raise the MSRV floor, did not weaken `[advisories]`'s
`unmaintained`/`unsound` settings, did not add or relax a `[licenses]` allow
entry or a `[bans]` deny rule, did not move the Tauri closure to the 2.12
line, and did not rewrite milestone 010's plan or closure record. Each of
those is an explicit §5 exclusion and a §14 stop condition. Where an upgrade
existed but was blocked by one of them, the entry became an **exception with
a dated removal recipe**, not a silent deferral.

The one place where the plan's literal instruction could not be followed
without a policy decision — the `zip` 9.x bump pulling `libbz2-rs-sys`, whose
`bzip2-1.0.6` licence expression is not on the allow list — was resolved
**without touching the allow list**, by narrowing `zip`'s feature set to the
compression backends the mobile lab actually uses. The full reasoning, the
rejected alternatives, and the exact fallback that would apply if the owner
prefers it are in §7 and §10 (F1).

## 2. Requirement-to-evidence matrix

| Plan requirement (§13 acceptance criterion) | Evidence | Result | Notes |
|---|---|---|---|
| 1. `make check-deps` completes with a zero exit status; each of its five `cargo deny` invocations recorded with its own exit status | `make check-deps` → exit **0**; the five sub-invococations were also run individually: default **0**, advisories **0**, bans **0**, licenses **0**, sources **0** (§4) | **pass** | Full target output in §4 |
| 2. `cargo deny --workspace --all-features check advisories` reports **zero** advisory entries | `advisories ok`, exit **0**, captured output in §4 | **pass** | Before: 23 entries / 20 IDs |
| 3. The 9 Bucket-A entries clear by a lockfile refresh with no manifest change; the 3 `zip` entries clear by the 9.x upgrade; the remaining 11 are recorded as exceptions | `Cargo.lock` diff in §4 shows all six Bucket-A bumps; `zip` 2.4.2→9.1.0; `deny.toml` carries exactly **11** new ignore entries for the 11 policy/upstream-blocked IDs | **pass** | The `zip` path taken was the **primary** path, not the documented fallback (§7) |
| 4. `deny.toml` has an `ignore` entry for every advisory that is not upgraded, and none for an advisory that is upgraded | `deny.toml` now holds **20** ignore entries: 9 pre-existing + 11 new. No entry names RUSTSEC-2026-0367/-0368/-0369 (upgraded), nor any Bucket-A ID (upgraded). A re-run of the advisory check after the bump reports zero, so no ignore is stale | **pass** | 20 of 20 carry a non-empty `reason`; 20 of 20 name the exception document |
| 5. Every `ignore` entry has a non-empty `reason` naming the affected crate, the dependency path, and the clearing condition | Measured: `reason = "` appears 20 times for 20 entries; each reason names the crate and version, the dependency path, the upgrade that would clear it, and why it is not adopted, and ends `See docs/DEPENDENCY_EXCEPTIONS.md.` | **pass** | Guard sub-check (a) and (b) enforce it mechanically |
| 6. Every ignored ID has a record in `docs/DEPENDENCY_EXCEPTIONS.md` with a non-empty `| Owner |`, a `| Review-by |` date, and a `| Blocker |` | 11 new records (4 headings) added; the document now has **13** `###` headings — 11 active + 2 resolved — each with an `| Owner |`, a `| Review-by |`, and a `| Blocker |` row | **pass** | Guard sub-check (c) enforces it mechanically |
| 7. `make check` completes end to end | **not run** — the session owner runs it at integration (§10, F2) | **not run** | Every step of `make check` that this milestone could own was run individually: `check-deps` (exit 0), the guard block (§4), `check-msrv` (§4) |
| 8. `make check-msrv` still builds the workspace on Rust 1.89, and the Tauri closure has not floated onto the 1.90-MSRV line | `make check-msrv` → exit **0** (first attempt exit 2 on disk exhaustion, §10 F2; second attempt green, all five steps including the GUI shell profile). Tauri closure measured unchanged: `tauri` 2.11.6, `tauri-build` 2.6.3, `tauri-runtime` 2.11.3, `tauri-runtime-wry` 2.11.4, `tauri-macros` 2.6.3, `tauri-utils` 2.9.3; `cargo update --dry-run -p tauri-utils` reports "Locking 0 packages to highest compatible versions" | **pass** | — |
| 9. `bash scripts/check-architecture-guards.sh` exits zero and reports Check 190; all five negative demonstrations performed and recorded | The Check 190 block was written to `/tmp/eggsec-guardwork/m015-guards.sh` and run against this tree (exit 0, `PASS: every dependency exception is owned and dated.`). All five negatives were demonstrated failing (§4) and the tree restored from `cp` backups | **partial** | The shared script was **not** edited (four concurrent agents own it); the owner merges the block and runs the whole script at integration |
| 10. `make check-gui` completes on a host with the webview SDK present, or is recorded as prerequisite-gated | **not run** — this milestone changes no GUI source, and the GUI's Tauri closure did not move (§4) | **not run** | Prerequisite-gated step; the owner's `make check` covers it |
| 11. No `.cargo/audit.toml` exists and no `cargo audit` configuration was added | `.cargo/audit.toml` absent (`ls .cargo` → no `audit.toml`); `git status` shows no new `.cargo/` file; `deny.toml` remains the only policy source | **pass** | — |
| 12. `[bans] deny = ["aws-lc-rs"]` and the ring-only TLS rule are unchanged | `deny.toml` `[bans]` block byte-identical to the baseline; `rustls` 0.21.12 and `rustls-webpki` 0.101.7 remain in the tree only on the `tiberius` path, under exception | **pass** | `git diff deny.toml` touches only the `[advisories]` ignore array |
| 13. `git status --porcelain` is clean apart from the intended changes, and no unrelated file was modified | §4 records the final status. This milestone's own changes are listed in §3; the tree also carries other agents' in-flight work, which was left untouched | **pass** (for this milestone's own scope) | — |

Additional §10 requirements:

| Requirement | Evidence | Result |
|---|---|---|
| Every `deny.toml` ignore has a non-empty reason | Guard sub-check (a); 20/20 measured | pass |
| Every reason names `docs/DEPENDENCY_EXCEPTIONS.md` | Guard sub-check (b); 20/20 measured | pass |
| Every ignored ID has an owned, dated record | Guard sub-check (c) | pass |
| No `RESOLVED` record retains a live ignore | Guard sub-check (d) | pass |
| A parse failure is an error, not an empty result | The guard fails closed when `deny.toml` or the exception document is missing or unparseable (§4 note below) | pass |
| The `zip` migration is exercised through the mobile-lab archive fixture suite | `cargo test -p eggsec-mobile-lab` → 20 passed, 0 failed | pass |
| Archive-layout compatibility (names, ordering, compression method, SUID/SGID, symlinks) | Byte-identical comparison of zip 2.4.2 vs 9.1.0 output (§7) | pass |
| Idempotent re-run (`cargo update`, guard) | A second and third `cargo update` produced no further lockfile change; the guard passes again on the restored tree | pass |
| Malformed-archive rejection unchanged | The pre-existing `rejects_zipslip`, `rejects_empty_android_manifest`, `invalid_zip_input_returns_error_not_panic`, and `empty_zip_returns_error_not_panic` tests all pass | pass |
| `make check-python` not required | No Python-facing dependency path changed: the only `crates/eggsec-python/` closure movement is the `tokio` 1.53.1→1.54.1 patch bump and the `pyo3` 0.29.2→0.29.3 patch bump. None of the 23 affected crates is reachable from `crates/eggsec-python/pyproject.toml` or the stubs | not required (reasoned, not run) |

## 3. Production implementation evidence

### 3.1 `deny.toml` — eleven new `[advisories] ignore` entries

Added after the existing entries under a new header
`# --- Milestone 015: advisories with no safe upgrade under repo policy ---`.
Each entry follows the file's house style and keeps the affected crate and
version, the dependency path, the upgrade that would clear it, the reason it
is not adopted, and the trailing `See docs/DEPENDENCY_EXCEPTIONS.md.`:

| ID | Cluster | Reason recorded |
|---|---|---|
| RUSTSEC-2024-0429 | `glib` 0.18.5 | unreachable: no published `muda`/`tao`/`wry` uses gtk-rs 0.20+, `tauri` 2.12.5 still requires `gtk ^0.18` — upstream-blocked |
| RUSTSEC-2024-0370 | `proc-macro-error` 1.0.4 | latest published; clears with RUSTSEC-2024-0429 |
| RUSTSEC-2026-0346 | `rustls` 0.21.12 | `tiberius` 0.13.0 reaches `rustls` 0.23 only via the `aws-lc-rs`-default `tokio-rustls` that `[bans] deny` forbids — policy-blocked |
| RUSTSEC-2026-0349 | `rustls` 0.21.12 | same path as above |
| RUSTSEC-2026-0382 | `rustls-webpki` 0.101.7 | same path as RUSTSEC-2026-0104 |
| RUSTSEC-2024-0449 | `unsafe-libyaml` 0.2.11 | latest published; `serde_yaml_neo` 0.11.0 hard-depends on it — unmaintained, not a vulnerability |
| RUSTSEC-2025-0100 | `unic-ucd-ident` 0.9.0 | `urlpattern` 0.6.0 clears the set but needs `tauri-utils` 2.10.x (MSRV 1.90) — MSRV-gated |
| RUSTSEC-2025-0075 | `unic-char-range` 0.9.0 | same path as above |
| RUSTSEC-2025-0080 | `unic-common` 0.9.0 | same path as above |
| RUSTSEC-2025-0081 | `unic-char-property` 0.9.0 | same path as above |
| RUSTSEC-2025-0098 | `unic-ucd-version` 0.9.0 | same path as above |

Measured after the edit: **20** `id = "RUSTSEC-…"` entries, **20** with a
non-empty `reason`, **20** whose reason names `docs/DEPENDENCY_EXCEPTIONS.md`.

Nothing else in `deny.toml` changed: `[advisories]`'s
`unmaintained = "transitive"` / `unsound = "transitive"`, the `[licenses]`
allow list and its single `auto_generate_cdp` exception, `[bans]`
(`deny = ["aws-lc-rs"]`, `wildcards = "deny"`, `multiple-versions = "warn"`),
and `[sources]` are byte-identical to the baseline.

### 3.2 `docs/DEPENDENCY_EXCEPTIONS.md` — four new exception records

Added under `## Active Exceptions`, each with the document's fixed ten-field
schema (`| Advisory |`, `| Path |`, `| Feature |`, `| API used |`,
`| Exploitability |`, `| Compensating control |`, `| Owner |`, `| Created |`,
`| Review-by |`, `| Blocker |`):

| Heading | IDs | Owner | Review-by | Blocker |
|---|---|---|---|---|
| `### RUSTSEC-2024-0429 — glib 0.18.5 unsound` | RUSTSEC-2024-0429, RUSTSEC-2024-0370 | `eggsec-gui` | 2026-12-12 | gtk-rs 0.20 unreachable; `tauri` 2.12.5 still declares `gtk ^0.18` |
| `### RUSTSEC-2026-0346 / RUSTSEC-2026-0349 / RUSTSEC-2026-0382 — rustls 0.21.12 / rustls-webpki 0.101.7` | 0346, 0349, 0382 | `eggsec-db-lab` | 2026-12-12 | `tiberius` 0.13.0 needs the `aws-lc-rs`-default `tokio-rustls`; ring-only forbids it |
| `### RUSTSEC-2024-0449 — unsafe-libyaml 0.2.11 unmaintained` | 0449 | `eggsec (config)` / `eggsec-web-proxy` | 2026-12-12 | 0.2.11 is the latest published; `serde_yaml_neo` 0.11.0 hard-depends on it |
| `### RUSTSEC-2025-0075 / -0080 / -0081 / -0098 / -0100 — unic-* 0.9.0 unmaintained` | 0075, 0080, 0081, 0098, 0100 | `eggsec-gui` | 2026-12-12 | `urlpattern` 0.6.0 clears it, but `tauri-utils` 2.10.x declares MSRV 1.90 |

The `unic-*` record carries the **full removal recipe** from plan §3.4, so the
2026-12-12 review is a decision about the MSRV floor rather than an
investigation: raise `rust-version` to 1.90, move the Tauri closure to the 2.12
line (`tauri` 2.12.5, `tauri-utils` 2.10.2, `tauri-macros` 2.7.4, `tauri-runtime`
2.12.2, `tauri-runtime-wry` 2.12.3, `tauri-build` 2.7.4), re-run
`make check-msrv`; the five entries then disappear because `urlpattern`
resolves to 0.6.x. The record states explicitly that this recipe does **not**
clear the `glib` / `proc-macro-error` pair.

The `unsafe-libyaml` record is written honestly rather than favourably: the
advisory kind is `unmaintained`, **not** a vulnerability, no fixed release
exists, and the parser *is* exercised — on partly operator-supplied and
partly **downloaded** content (the scanner marketplace loader). The
mitigation is the absence of an upstream fix, not a claim of non-exposure.

**Heading placement (plan §12).** The two `RESOLVED` records
(`RUSTSEC-2025-0119` `number_prefix`, `RUSTSEC-2024-0436` `paste`) were moved
out of `## Active Exceptions` into a new `## Resolved Exceptions` heading
placed immediately after it. This is documentation-only: both `deny.toml`
ignores were already removed, so the policy state was already correct and only
the heading placement was misleading. The guard's sub-check (d) now keeps it
that way.

### 3.3 `Cargo.lock` — the refresh

`cargo update` from the workspace root; no hand-editing, no
`--breaking`, no `-Z minimal-versions`.

### 3.4 The `zip` 9.x migration

Manifest changes:

- `crates/eggsec-mobile-lab/Cargo.toml` — `zip = "2"` → `zip = "9"` with the
  feature set narrowed (see §7.2).
- `crates/eggsec/Cargo.toml` — `[dependencies.zip] version = "2"` →
  `version = "9"`, `optional = true` preserved, and the `mobile` feature
  wiring at the feature table (`mobile = ["dep:eggsec-mobile-lab", "zip",
  "plist"]`) untouched.

Call-site changes in `crates/eggsec-mobile-lab`:

| File | Change | Reason |
|---|---|---|
| `src/ipa.rs` (`locate_info_plist`) | `entry.name()` now returns `ZipResult<Cow<str>>`; the name is taken as an owned `String` | zip 9.x decodes the entry name fallibly |
| `src/ipa.rs` (archive walk) | same fallible-name handling | as above |
| `src/apk.rs` (`analyze_apk_blocking`) | same fallible-name handling; a name that cannot be decoded is a `MobileError::Validation` | as above |
| `src/ipa.rs` ×4, `src/apk.rs` ×2, `src/adb.rs` ×1 | `zip::write::FileOptions<()>` → `zip::write::SimpleFileOptions` | the 2.x `FileOptions<'k, T>` became `FileOptions<'k, 'n, T>` in 9.x; `SimpleFileOptions` is the 9.x alias for `FileOptions<'static, 'static, ()>` |
| `src/apk.rs` ×4, `src/adb.rs` ×1 | the compression method is now **explicit** (`.compression_method(zip::CompressionMethod::Deflated)`) where the call previously relied on the crate default | plan §9: the layout must not depend on a default; see §7.2 |

**No extraction API was introduced.** Measured after the migration:
`rg -n 'extract\(|extract_to|unpack|extract_dir' crates/eggsec-mobile-lab/`
returns **no matches**. The crate reads entries by name and writes archives
with `ZipWriter`; it never extracts.

### 3.5 The guard — Check 190

Written to `/tmp/eggsec-guardwork/m015-guards.sh` in the house style of
`scripts/check-architecture-guards.sh` (banner, `SECTION_FAIL=0`
accumulator, per-item checks with `FAIL:` messages, final `if [[ $SECTION_FAIL
-eq 0 ]]` block). **The shared script was not edited** — four agents own it
concurrently; the owner merges the block.

Four sub-checks, exactly as plan §6 specifies:

- **(a)** every `deny.toml` `[advisories] ignore` entry has a non-empty `reason`;
- **(b)** every such `reason` contains `docs/DEPENDENCY_EXCEPTIONS.md`;
- **(c)** every ignored advisory ID has a `docs/DEPENDENCY_EXCEPTIONS.md`
  record carrying a non-empty `| Owner |` and a `| Review-by |` that is an
  ISO date or the literal `— (closed)`;
- **(d)** no `RESOLVED` record's advisory ID remains in the live `ignore` list.

Fail-closed behaviour: if `deny.toml` or `docs/DEPENDENCY_EXCEPTIONS.md` is
missing, if the `[advisories] ignore` array cannot be sliced out, if no
ignore entry parses, or if no exception record parses, the guard reports the
file by name and fails. It never passes because it found nothing.

Registration was prepared but **not applied**, because
`docs/CI_ARCHITECTURE_GUARDS.md` is shared by three agents: the exact
markdown to insert is in `/tmp/eggsec-guardwork/m015-guards-doc.md`, with its
insertion point named. `docs/VERIFICATION.md` — which this milestone owns —
**was** updated: the `bash scripts/check-architecture-guards.sh` row of the
"Defect classes covered" table now names Check 190 and what it pins.

### 3.6 Documentation

- `architecture/supply_chain.md` — a new section, "Repository Dependency
  Policy — Advisory Backlog Closure (Milestone 015)", records the before /
  after counts, the three dispositions, the four exception clusters with their
  owners and blockers, and the guard. The section is explicitly labelled as
  **repository-level** policy, because this document is the deep dive for the
  `eggsec::supply_chain` *scanner* module (SBOM, typosquat, manifest
  discovery) — the plan's §12 description of this file as "the subsystem's
  deep-dive document for supply chain" for *dependency* policy does not match
  what the file is. The content was placed without conflating the two.
- `architecture/desktop_gui.md` — a new subsection, "The advisory clusters
  behind these pins", records that the `unic-*` and `glib` /
  `proc-macro-error` clusters are reached **only** through the GUI's Tauri
  shell closure, that `urlpattern` 0.6.0 (via `tauri-utils` 2.10) is the
  clearing path for the `unic-*` set, and that the `glib` /
  `proc-macro-error` pair has **no published clearing path**. The GUI's
  dependency *direction* is unchanged.
- `docs/VERIFICATION.md` — the guard table row, as above.

## 4. Verification executed

### Commands run

All commands were run from the worktree root
`/Users/davidbowman/projects/eggsec/.worktrees/feat-auto-20261022-76692679`.

```bash
# --- baseline measurement -------------------------------------------------
cargo deny --workspace --all-features check advisories > /tmp/eggsec-guardwork/m015_advisories_BEFORE.txt 2>&1
grep -c "^error\[" /tmp/eggsec-guardwork/m015_advisories_BEFORE.txt
grep -o "RUSTSEC-[0-9]\{4\}-[0-9]\{4\}" /tmp/eggsec-guardwork/m015_advisories_BEFORE.txt | sort -u | wc -l

# --- work package A: the lockfile refresh --------------------------------
cargo update --dry-run                       # exit 0
cargo update                                 # exit 0

# --- work package B: the zip 9.x migration --------------------------------
cargo update -p zip                          # exit 0 (zip 9.1.0)
cargo check -p eggsec-mobile-lab --all-targets   # exit 0
cargo check -p eggsec --features mobile          # exit 0
cargo test -p eggsec-mobile-lab                   # exit 0 — 20 passed, 0 failed

# --- work package C/D: the policy files and the guard ---------------------
bash /tmp/eggsec-guardwork/m015-guard-harness.sh      # exit 0 — PASS
bash /tmp/eggsec-guardwork/m015-negative-tests.sh      # exit 0 (all five negatives exit 1)

# --- the dependency-policy gate -------------------------------------------
cargo deny --workspace --all-features check advisories  # exit 0
cargo deny --workspace --all-features check            # exit 0
cargo deny --workspace --all-features check bans       # exit 0
cargo deny --workspace --all-features check licenses   # exit 0
cargo deny --workspace --all-features check sources    # exit 0
make check-deps                                          # exit 0

# --- the MSRV floor --------------------------------------------------------
make check-msrv                                          # see Results

# --- idempotency ----------------------------------------------------------
cargo update                                            # exit 0 — no further diff

# --- documentation references ---------------------------------------------
python3 scripts/check_doc_references.py                 # exit 1 — 29 missing, none in this milestone's files
```

### Results

**Before measurement.** `cargo deny --workspace --all-features check
advisories` → exit **1**. Counting rule from plan §3.2 (`cargo-deny` writes
its diagnostics to stderr, so the capture redirects both streams):

| Measure | Value |
|---|---|
| Entries (`grep -c "^error\["`) | **23** |
| Unique advisory IDs | **20** |
| Kind split | **14** `vulnerability`, **7** `unmaintained`, **2** `unsound` |

The 20 IDs are RUSTSEC-2023-0071, -2024-0370, -0384, -0429, -0449,
-2025-0057, -0075, -0080, -0081, -0098, -0100, -0134, -2026-0098, -0099,
-0104, -0187, -0192, -0346, -0349, -0367, -0368, -0369, -0381, -0382,
-0383, -0385, -0386 — unchanged from the plan's §3.3 table.

**`cargo update --dry-run` (work package A, first pass).** Exit **0**.
Proposed **149** updates, **15** additions, **20** removals — the plan's
"149 version bumps and 35 additions/removals" exactly. All six Bucket-A
bumps appear; **no Tauri line moves**.

**`cargo update` (work package A).** Exit **0**.
`git diff --stat Cargo.lock` at that point: `587 insertions(+), 605 deletions(-)`.

**`cargo update -p zip` (work package B).** Exit **0**; `zip` moved
2.4.2 → 9.1.0.

**Interim advisory measurement** (after the refresh, with the 11 new ignores
already in `deny.toml`): **3** entries remained — exactly
RUSTSEC-2026-0367, -0368, -0369, the three `zip` entries. (The plan's §7
package-A acceptance text says the count drops "from 23 to 14"; 14 is the
count *before* the 11 new ignores are added. Measured with the ignores in
place, the remaining count is 3, which is the same fact.)

**After measurement.** `cargo deny --workspace --all-features check
advisories` → exit **0**, output:

```
advisories ok
```

**Entries: 23 → 0. Unique advisory IDs: 20 → 0. Remaining IDs: none.**

**`Cargo.lock` diff summary.** `git diff --stat Cargo.lock`:
`1323 +++++-----`, `592 insertions(+), 731 deletions(-)`. Package-level
accounting against `HEAD`:

| Category | Count | Notes |
|---|---|---|
| Added | 7 | `borsh` 1.8.1, `borsh-derive` 1.8.1, `core_detect` 1.0.0, `multiversion_no_op` 1.0.0, `psm` 0.1.24, `stacker` 0.1.25, `typed-path` 0.13.0 |
| Removed | 14 | `aes`, `arbitrary`, `bzip2`, `bzip2-sys`, `constant_time_eq`, `deflate64`, `derive_arbitrary`, `lzma-sys`, `socks`, `tinyvec_macros`, `xz2`, `zstd`, `zstd-safe`, `zstd-sys` |
| Moved | 163 | including the six Bucket-A bumps, `zip` 2.4.2→9.1.0, `pyo3` 0.29.2→0.29.3, `eggfetch-core` 0.2.0→0.2.2, `plist` 1.10.1→1.10.3, `spdx` 0.13.5→0.13.6 |

The 14 removals are dominated by the `zip` feature narrowing (§7.2): the
unused compression backends and their C/Rust backends leave the graph.

**Tauri closure did not float.** Measured in the refreshed `Cargo.lock`:

```
tauri            2.11.6     tauri-build        2.6.3
tauri-macros     2.6.3      tauri-runtime      2.11.3
tauri-runtime-wry 2.11.4    tauri-utils        2.9.3
```

`cargo update --dry-run -p tauri-utils` → "Locking 0 packages to highest
compatible versions", exactly as the plan required. The `rustls` 0.21.12 /
`rustls-webpki` 0.101.7 pair remains on the `tiberius` path, under exception;
the `rustls` 0.23 line moved to 0.23.46 and `rustls-webpki` 0.103.15 to
0.103.16.

**`make check-deps`.** Exit **0**. Its five sub-invococations, run
individually as well, all exit **0**: default (`advisories ok, bans ok,
licenses ok, sources ok`), advisories, bans, licenses, sources. The only
output of note is the pre-existing `warning[no-license-field]` for
`auto_generate_cdp`, which is a warning and is already covered by the
`[licenses] exceptions` entry.

**`cargo test -p eggsec-mobile-lab`.** Exit **0** — 20 passed, 0 failed.
This is the suite that reads the IPA/APK fixtures, so it is the archive-layout
evidence.

**Guard Check 190 (block run standalone against this tree).** Exit **0**:

```
--- Check 190: every dependency exception is owned and dated ---
PASS: every dependency exception is owned and dated.
```

**The five negative demonstrations** (`/tmp/eggsec-guardwork/m015-negative-tests.sh`;
each fixture mutates a policy file, then the file is restored from a `cp`
backup — `git checkout --` was never used). Actual output:

```
############ NEGATIVE 1: unowned exception (remove the | Owner | row) ############
FAIL: RUSTSEC-2024-0429 has no record with a non-empty '| Owner |' and a dated '| Review-by |' in docs/DEPENDENCY_EXCEPTIONS.md.
FAIL: RUSTSEC-2024-0370 has no record with a non-empty '| Owner |' and a dated '| Review-by |' in docs/DEPENDENCY_EXCEPTIONS.md.
FAILED: 1 check(s) failed.
GUARD_EXIT=1

############ NEGATIVE 2: undated exception (Review-by is neither ISO nor closed) ############
FAIL: RUSTSEC-2024-0429 has no record with a non-empty '| Owner |' and a dated '| Review-by |' in docs/DEPENDENCY_EXCEPTIONS.md.
FAIL: RUSTSEC-2024-0370 has no record with a non-empty '| Owner |' and a dated '| Review-by |' in docs/DEPENDENCY_EXCEPTIONS.md.
FAILED: 1 check(s) failed.
GUARD_EXIT=1

############ NEGATIVE 3: undocumented ignore (an ID with no exception record) ############
FAIL: RUSTSEC-2099-0001 is ignored in deny.toml but has no record in docs/DEPENDENCY_EXCEPTIONS.md.
FAILED: 1 check(s) failed.
GUARD_EXIT=1

############ NEGATIVE 4: resurrected resolved exception (RUSTSEC-2025-0119) ############
FAIL: RUSTSEC-2025-0119 is RESOLVED in docs/DEPENDENCY_EXCEPTIONS.md but is still ignored in deny.toml.
FAILED: 1 check(s) failed.
GUARD_EXIT=1

############ NEGATIVE 5: undocumented reason (empty reason string) ############
FAIL: RUSTSEC-2099-0002 has an empty reason in deny.toml.
FAILED: 1 check(s) failed.
GUARD_EXIT=1

############ RESTORED: guard on the restored tree ############
PASS: every dependency exception is owned and dated.
ALL PASSED: No architecture drift detected.
GUARD_EXIT=0
```

The working tree was confirmed restored after each fixture; the final
`git status --porcelain -- deny.toml docs/DEPENDENCY_EXCEPTIONS.md` shows only
this milestone's intended modifications (` M deny.toml`,
` M docs/DEPENDENCY_EXCEPTIONS.md`) and no residue from the fixtures.

**Fail-closed behaviour, demonstrated.** Four further fixtures were run
against a scratch directory (the guard's paths are relative, so the fixture
is a directory containing only one of the two policy files). Each fails
closed with the file named — never a silent pass:

| Fixture | Output | Exit |
|---|---|---|
| `deny.toml` absent | `FAIL: deny.toml is missing; the canonical dependency-policy source cannot be checked.` | 1 |
| `docs/DEPENDENCY_EXCEPTIONS.md` absent | `FAIL: docs/DEPENDENCY_EXCEPTIONS.md is missing; the exception record cannot be checked.` | 1 |
| `deny.toml` present but with no parseable `[advisories] ignore` array | `FAIL: the [advisories] ignore array could not be parsed from deny.toml.` + `FAIL: no advisory ignore entries were parsed from deny.toml.` | 1 |
| exception document present but with no parseable record | `FAIL: no exception records could be parsed from docs/DEPENDENCY_EXCEPTIONS.md.` + one `FAIL: <ID> is ignored in deny.toml but has no record …` per ignored ID | 1 |

**`cargo update` idempotency.** A second and a third `cargo update` both
exit **0** and report "Locking 0 packages to highest compatible versions"; the
third produced a **byte-identical** `Cargo.lock` (`diff -q` → identical). The
only difference between the first and second `cargo update` was a
lockfile-normalisation pass that rewrote some *dependency edges* (Windows-only
`windows-sys` and `getrandom` edges) without adding, removing, or re-versioning
any package.

**`make check-msrv`.** The **first** attempt exited **2** — not an MSRV
failure but an environmental one: the build died with
`No space left on device (os error 28)` while compiling `syn`,
`zerocopy-derive`, `time`, `tracing-attributes` and others, because the
machine's data volume was at 100% (885 GiB used, 125 MiB free) with three
concurrent agents building in the same worktree. The **second** attempt,
after disk space was released, exited **0** and ran all five steps:

```
cargo +1.89 check --workspace --no-default-features     # ok
cargo +1.89 check -p eggsec-cli --no-default-features    # ok
cargo +1.89 check -p eggsec-transport                   # ok
cargo +1.89 check -p eggsec-transport-eggfetch          # ok
cargo +1.89 check -p eggsec --no-default-features       # ok
# the desktop GUI shell profile under the declared MSRV  # ok
                                                     Checking tauri-utils v2.9.3
                                                     Checking tauri-runtime v2.11.3
                                                     Checking tauri-runtime-wry v2.11.4
                                                     Compiling tauri v2.11.6
```

The MSRV floor therefore holds after both the lockfile refresh and the
`zip` 9.x bump: `zip` 9.1.0 declares `rust-version = "1.88"`, below the 1.89
floor, and the GUI shell profile — the closure that carries the MSRV risk —
builds on 1.89 with the exact Tauri pins unchanged.

The substantive claim of criterion 8 — that the Tauri closure has not floated
onto the 1.90-MSRV line — is measured and satisfied: the exact Tauri pins are
unchanged and `cargo update --dry-run -p tauri-utils` reports "Locking 0
packages to highest compatible versions".

**`python3 scripts/check_doc_references.py`.** Exit **1**, `Total missing: 29`.
**None of the 29 is in a file this milestone touched** — they are in
`architecture/capability_segregation.md`, `architecture/fuzzer.md`,
`architecture/knowledge_corpus.md`, `architecture/network_dependency_baseline.md`,
`architecture/nse_capability_inventory.md`, `architecture/nse_integration.md`,
`architecture/overview.md`, `architecture/proxy.md`,
`architecture/transport_eggfetch.md`, `docs/CAPABILITIES.md`,
`docs/CI_ARCHITECTURE_GUARDS.md`, and `docs/NSE_COMPATIBILITY.md` — all
pre-existing drift owned by other milestones. This milestone's own
documentation edits introduce no missing reference.

**Formatting and lint on the milestone's own Rust changes.**
`cargo fmt --all --check` exits **1**, but **none of the 12 files with a
formatting diff is a file this milestone touched** — they are
`crates/eggsec-daemon-protocol/src/protocol.rs`,
`crates/eggsec-daemon/src/{host,server}.rs`,
`crates/eggsec-daemon/tests/daemon_socket_env.rs`,
`crates/eggsec-gui/src/coverage.rs`,
`crates/eggsec-gui/tests/qualification.rs`,
`crates/eggsec-runtime/src/{daemon_endpoint,report}.rs`,
`crates/eggsec-runtime/tests/daemon_endpoint.rs`, and
`crates/eggsec/src/runtime_bridge/{executor,report,report_coverage}.rs` —
all other agents' in-flight work. The three `eggsec-mobile-lab` source files
this milestone edited are rustfmt-clean.
`cargo clippy -p eggsec-mobile-lab --all-targets` exits **0** with three
warnings, all pre-existing at `HEAD`: the unused `use std::io::Cursor;` in
`invalid_zip_input_returns_error_not_panic` (`apk.rs:1233`) and two
`contains()`-vs-`iter().any()` suggestions in pre-existing test assertions
(`apk.rs:1164-1165`). `make clippy` does not lint this crate.

**`make check` was deliberately NOT run** — the session owner runs it at
integration, per the milestone brief.



### Integration verification (2026-10-24, session owner)

The milestone's own commands were run in isolation; the owner's integration
step then ran the repository's mandatory contract over the whole tree, with
all four milestones landed together.

| Command | Result |
|---|---|
| `make check` | **exit 0** — `cargo fmt --all --check`, the no-default-features and per-crate checks, `cargo deny --workspace --all-features check` (all five sub-checks), clippy, the full test suite (**4315 tests across 77 test binaries, 0 failures**), and `scripts/check-architecture-guards.sh` (**190 guards, "ALL PASSED: No architecture drift detected"**) |
| `make check-msrv` | **exit 0** — all five steps green on Rust 1.89 |
| `python3 scripts/check_doc_references.py` | 29 missing references — the unchanged pre-existing baseline, none of them in a file this milestone touched |

This is the **first time `make check` has completed on this branch**: it had
always stopped at the inherited `check-deps` advisory backlog, which milestone
015 cleared. Three integration tests that milestone 010 had left stale were
found by that first complete run and corrected in `dcfcd4e2`; the corrections
are recorded as a dated addendum on milestone 010's closure record rather
than by rewriting it.

## 5. Invariant review

| Invariant (plan §4) | Evidence |
|---|---|
| `deny.toml` is the canonical dependency-policy source; `.cargo/audit.toml` must not exist; no `cargo audit` config may be reintroduced | No `.cargo/audit.toml` exists; `git status` shows no `.cargo/` change; `deny.toml` is the only policy file edited |
| Every retained dependency exception carries an owner, a review-by date, and a record in `docs/DEPENDENCY_EXCEPTIONS.md` | 20 ignore entries ↔ 13 records (11 active + 2 resolved), each with `| Owner |` and `| Review-by |`; guard sub-checks (c) and (d) enforce it mechanically |
| TLS is ring-only everywhere; `[bans] deny = ["aws-lc-rs"]` stands | `git diff deny.toml` touches only the `[advisories]` ignore array; the `[bans]` block is unchanged; `rustls` 0.21.12 stays only on the `tiberius` path under exception; `tokio-rustls` 0.24.1 is unchanged (0.26.5→0.26.6 is the 0.23-line `aws-lc-rs`-free copy) |
| MSRV 1.89; no upgrade may raise it | `Cargo.toml:39` unchanged; `zip` 9.1.0 declares `rust-version = "1.88"` (below the floor); the Tauri closure is unchanged |
| No new dependency without an explicit `deny.toml` decision | `cargo deny check bans` and `check licenses` both exit 0 after the bump; the 7 added and 14 removed packages are all covered by the existing allow list and the `multiple-versions = "warn"` policy; **no** `[licenses]` allow entry and **no** `[bans]` rule was added or relaxed |
| `make check-deps` fails closed when `cargo-deny` is absent | `Makefile:153` is untouched; the new guard reads the two policy files with plain text tools and does **not** depend on `cargo-deny` being installed |
| The desktop GUI is daemon-only and links no engine/daemon/CLI/TUI crate (guard 155) | Untouched: no GUI source or manifest was modified; the GUI's closure only changed through `cargo update`'s version bumps, which do not change dependency *direction* |
| `EnforcementContext::evaluate()` remains the mandatory pre-dispatch gate; strict surfaces remain fail-closed | **This milestone changed no dispatch, authorization, scope, or protocol code.** No file under `crates/eggsec/src/dispatch/`, `crates/eggsec/src/config/`, `crates/eggsec-policy/`, or the daemon protocol was edited. The only Rust edits are the `zip` call sites inside `crates/eggsec-mobile-lab` |
| Never `let _ =`; never `filter_map(|e| e.ok())`; log via `tracing` | The `zip` migration introduces no `let _ =` and no `filter_map(... .ok())`. The new fallible-name handling uses `match`/`?`-style propagation into `MobileError`, and the existing `tracing::warn!` call in the APK manifest parser is untouched |
| No full-suite run as root | Every command was run as the normal user |
| Workspace Tokio is `default-features = false` with per-crate `features = [...]` | No manifest feature policy changed. The `tokio` bump is 1.53.1 → 1.54.1 within the same major; the feature set is unchanged (no `test-util` anywhere) |
| New architecture guards start at 177; **this milestone's reserved number is 190** | The block is written as `--- Check 190: ... ---`; no other number was used |

## 6. Failure and recovery review

This milestone has no runtime, task, or session semantics; the relevant
failure semantics are those of the tooling and of the dependency graph.

- **`cargo update` fails (network unavailable).** The lockfile is not
  written; `cargo deny` re-reads the existing lockfile and re-reports the
  same set. The committed lockfile was regenerated in this environment with
  live crates.io and a live advisory database.
- **Advisory database moves between the local run and CI's run.** The
  advisory database is fetched live from `https://github.com/rustsec/advisory-db`
  (`deny.toml:16-17`). A new advisory published against a crate in this
  workspace would fail CI's `check-deps` even though the local run is green.
  This is the intended fail-closed posture; §10 records it as a standing
  residual.
- **Local tool version differs from CI's.** Local `cargo-deny` is 0.20.2; CI
  installs `cargo-deny@0.19.0` (`.github/workflows/ci.yml:67`). The advisory
  database and the `deny.toml` semantics are the same, but *reporting* may
  differ across a version boundary. **A local green is not proof of a CI
  green**; the CI job is the authority.
- **Contention.** Three other agents ran cargo against the same worktree and
  target directory during this milestone. Cargo serialises on its own lock;
  no lock-wait failure was observed that needed a retry, and no
  `cargo update` was killed. One `make check-msrv` attempt failed on disk
  exhaustion (§10, F2) rather than on contention.
- **Restart and recovery.** There is no long-lived state. `Cargo.lock` is
  rewritten atomically by cargo; an interrupted refresh leaves the previous
  lockfile intact and re-running it is idempotent — demonstrated: a second
  and third `cargo update` produced no further change.
- **Duplicate delivery.** Re-running the lockfile refresh on an
  already-refreshed lockfile is a no-op (measured). Re-running the exception
  recording is a no-op provided the guard's sub-checks hold (measured). The
  one non-idempotent step is the negative-fixture demonstration, which
  mutates a policy file; it was performed with `cp` backups and the tree was
  confirmed restored afterwards.
- **Cancellation.** No tokio task is spawned by this milestone, so
  `eggsec-runtime::race_with_cancel` is not involved and no timeout wrapper
  is required.

## 7. Migration and compatibility review

### 7.1 Backward compatibility

No wire, protocol, DTO, or persisted-format compatibility surface was
touched. The daemon protocol remains additive-only; the GUI re-attaches after
a restart rather than restoring its own state.

### 7.2 The `zip` 2.x → 9.x migration — path taken, and why

**Path taken: the primary path (the 9.x upgrade), with `zip`'s feature set
narrowed to the compression backends the mobile lab actually uses.** The
plan's documented fallback (recording the three `zip` entries as exceptions
and reverting to 2.x) was **not** taken.

The narrowing was forced by a real, measured gate failure, not by
preference. `zip` 9.1.0's default `bzip2` feature pulls `bzip2` 0.6, which
defaults to `libbz2-rs-sys` 0.2.5; that crate declares
`license = "bzip2-1.0.6"`, which is **not** on the `[licenses]` allow list
(`deny.toml:39-56`). Measured result with the default feature set:

```
error[rejected]: failed to satisfy license requirements
   ┌─ …/libbz2-rs-sys-0.2.5/Cargo.toml:27:12
27 │ license = "bzip2-1.0.6"
   │            rejected: license is not explicitly allowed
   ├ libbz2-rs-sys v0.2.5
     └── bzip2 v0.6.1
         └── zip v9.1.0
```

That is plan §14's stop condition — "a new transitive dependency introduced
by the `zip` bump is not covered by the `[licenses]` allow list" — and the
plan's remedy is explicit: *adding a licence is a policy decision and is out
of scope*. Adding `bzip2-1.0.6` was therefore **not** an option.

The alternatives considered, in order:

1. **Add the licence to the allow list.** Rejected: §5 exclusion, §14 stop
   condition, and the milestone brief's hard rule.
2. **Take the documented fallback** (three `zip` exceptions, revert to 2.x).
   Rejected as the *first* choice because it leaves three real
   vulnerabilities live and merely suppresses them — the opposite of the
   milestone's objective, and the plan's §5 "Deliverable boundary" is
   "every advisory with a safe upgrade is upgraded". It remains available and
   is fully specified in §10 (F1) if the owner prefers it.
3. **Narrow `zip`'s features** to `["deflate", "time"]`. **Chosen.** The
   mobile lab reads and writes only `Stored` and `Deflated` entries — those
   are the only two methods any APK or IPA uses in practice — so the `bzip2`,
   `lzma`, `xz`, `zstd`, `ppmd`, `deflate64` and `aes-crypto` backends are
   unused. Dropping them removes the offending crate entirely, keeps the
   `zip` 9.1.0 security fix, and **changes no policy surface**: no licence
   was added, no ban relaxed, no TLS rule touched, no MSRV moved. It is also
   this repository's own convention — `Cargo.toml:88` does the same for
   `schemars` ("`default-features = false` keeps them out of the graph"), and
   `hickory-resolver` and `reqwest` are trimmed the same way.

`time` is kept deliberately: it is what makes the writer stamp "now" rather
than 1980-01-01, so the archive timestamps are unchanged. `deflate` is kept
because it provides both `Deflated` writing and `Deflated` *reading*, and
because `_deflate-any` is what makes `CompressionMethod::default()` resolve
to `Deflated` — exactly as in 2.4.2.

**Net effect on the dependency graph** (measured): 14 packages left the tree
(`aes`, `bzip2`, `bzip2-sys`, `constant_time_eq`, `deflate64`, `lzma-sys`,
`xz2`, `zstd`, `zstd-safe`, `zstd-sys`, `socks`, `arbitrary`,
`derive_arbitrary`, `tinyvec_macros`). The graph is *smaller* than the
baseline.

**Archive-layout compatibility evidence.** The synthetic fixtures the
mobile-lab tests build were reproduced under `zip = "=2.4.2"` and
`zip = "=9.1.0"` (default-features off, `features = ["deflate", "time"]` —
the exact configuration now in the repository) in a scratch project outside
the tree, with the modification timestamp pinned on both sides so the
comparison isolates the layout from the wall clock. The two writers produced
**byte-identical archives**:

```
=== fixture: stored ===
zip2 bytes = 354
zip9 bytes = 354
identical  = true
--- zip2 listing ---
  name=AndroidManifest.xml method=Stored size=42 compressed=42 unix_mode=Some(33188)
  name=res/values/secrets.xml method=Stored size=56 compressed=56 unix_mode=Some(33188)
--- zip9 listing ---
  name=AndroidManifest.xml method=Stored size=42 compressed=42 unix_mode=Some(33188)
  name=res/values/secrets.xml method=Stored size=56 compressed=56 unix_mode=Some(33188)
--- listing identical = true

=== fixture: default ===
zip2 bytes = 139
zip9 bytes = 139
identical  = true
--- zip2 listing ---
  name=../../evil.xml method=Deflated size=11 compressed=13 unix_mode=Some(33188)
--- zip9 listing ---
  name=../../evil.xml method=Deflated size=11 compressed=13 unix_mode=Some(33188)
--- listing identical = true
```

That covers plan §9's list, item by item:

| §9 requirement | Measured |
|---|---|
| entry names and ordering unchanged | identical (`AndroidManifest.xml`, then `res/values/secrets.xml`; and the single `../../evil.xml` fixture) |
| compression method per entry unchanged | `Stored` where 2.x wrote `Stored`; `Deflated` where 2.x wrote `Deflated` (the default) — identical |
| no entry gains SUID/SGID bits or a symlink | `unix_mode=Some(33188)` = `0o100644` on both sides — a regular file with no setuid/setgid bit and no symlink bit |
| the reader still rejects a malformed archive the same way | `rejects_zipslip`, `rejects_empty_android_manifest`, `invalid_zip_input_returns_error_not_panic`, `empty_zip_returns_error_not_panic` all pass; the "ZipSlip"/"traversal" and "empty AndroidManifest" assertions are unchanged |
| no extraction step introduced | `rg 'extract\(|extract_to|unpack|extract_dir' crates/eggsec-mobile-lab/` → **no matches** |

**The compression method was made explicit** at the five call sites that
previously relied on the crate default (`apk.rs` ×4, `adb.rs` ×1). Plan §9
requires: "If the 9.x writer's default compression differs from 2.x's, the
implementer must set it explicitly rather than accept the new default." It
does *not* differ in the 9.1.0 + `deflate` configuration — both resolve the
default to `Deflated`, which is why the byte comparison above is identical —
but the explicit form removes the dependency of the archive layout on a
default that a future feature change could move.

### 7.3 The `Cargo.lock` refresh

A lockfile bump is a normal supply-chain event. It changes no public API of
the workspace. It may change the behaviour of transitive crates that moved by
a minor or patch version, which is why the mobile-lab suite and the full
`cargo-deny` sweep were re-run after it — both are green.

### 7.4 Removal criteria for the legacy paths

The `zip` 2.x requirement is gone from both manifests; no
`zip = ">=2, <9"`-style range was introduced to keep both alive. The
`rustls` 0.21 / `tokio-rustls` 0.24 path is **not** removed: it is retained
under exception and stays live until either the ring-only rule changes or
`tiberius` exposes a ring-only path.

## 8. Security review

- **Authorization, scope, and dispatch.** Unchanged. This milestone touched
  no enforcement, scope, dispatch, or protocol code. `EnforcementContext::evaluate()`
  remains the sole pre-dispatch gate; strict surfaces remain fail-closed.
- **The `zip` advisory cluster** concerns archive **extraction** (SUID/SGID
  preservation, symlink targets escaping the destination, Unix permissions
  applied through symlinks). The mobile lab never extracts: it reads
  individual entries by name and writes archives with `ZipWriter`. Measured
  after the migration: no `extract()`, `extract_to`, `unpack`, or
  `extract_dir` call site exists in `crates/eggsec-mobile-lab`. The
  compensating control is therefore void-free, and the advisory is in any
  case **fixed** rather than suppressed.
- **The `rustls` 0.21-line cluster** sits behind lab-only,
  operator-initiated, scope-enforced MSSQL connections, matching the
  pre-existing record at `docs/DEPENDENCY_EXCEPTIONS.md`. The upgrade stays
  blocked by the ring-only rule and the cluster's blast radius does not grow.
- **The `unsafe-libyaml` cluster** parses YAML that is partly
  operator-supplied and partly downloaded (the scanner marketplace loader).
  The record says so plainly: the advisory kind is `unmaintained`, no fixed
  release exists, and the parser is exercised — the mitigation is the absence
  of an upstream fix, not a claim of non-exposure.
- **The `unic-*` cluster** is reached only through the GUI's Tauri shell
  closure; the crates are pure data tables with no unsafe code and no I/O.
- **TLS.** The ring-only rule is unchanged. `rustls` 0.23 (the `reqwest` path)
  moved to 0.23.46, which is the patched line; `rustls` 0.21.12 stays only on
  the `tiberius` path, under exception. `aws-lc-rs` remains denied.
- **Supply-chain posture.** The graph got *smaller* (14 packages removed) and
  the advisory count went to zero. No new direct dependency was added to any
  workspace crate.
- **No secret handling, path validation, or privilege boundary** was touched.
  The mobile lab's own ZipSlip guard (`is_safe_entry_name`, plus the APK
  path-traversal rejection) is unchanged and still exercised by
  `rejects_zipslip`.

## 9. Documentation and operations

| Artifact | Change | Owner of the file |
|---|---|---|
| `deny.toml` | 11 new `[advisories] ignore` entries | this milestone |
| `docs/DEPENDENCY_EXCEPTIONS.md` | 4 new exception records; 2 `RESOLVED` records moved under a new `## Resolved Exceptions` heading; header review date updated | this milestone |
| `docs/VERIFICATION.md` | the `bash scripts/check-architecture-guards.sh` row of the "Defect classes covered" table now names Check 190 and what it pins | this milestone |
| `architecture/supply_chain.md` | new section "Repository Dependency Policy — Advisory Backlog Closure (Milestone 015)" | this milestone |
| `architecture/desktop_gui.md` | new subsection "The advisory clusters behind these pins" | this milestone |
| `scripts/check-architecture-guards.sh` | **not edited** — the Check 190 block is delivered at `/tmp/eggsec-guardwork/m015-guards.sh` for the owner to merge | shared (four agents) |
| `docs/CI_ARCHITECTURE_GUARDS.md` | **not edited** — the registration entry is delivered at `/tmp/eggsec-guardwork/m015-guards-doc.md`, with its insertion point named | shared (three agents) |
| `plans/registry.md`, `plans/subsystems/desktop-gui-frontend-roadmap.md` | **not edited** — the session owner updates both | session owner |
| `plans/closure/desktop-gui-frontend/010-…-closure.md` | **not modified**, per `plans/implementation/README.md:179` | immutable history |

**Operator diagnostics.** `make check-deps` is now the supply-chain gate that
passes; an operator who needs to know *why* an advisory is tolerated reads
`docs/DEPENDENCY_EXCEPTIONS.md`, where every entry now carries an owner, a
review date, the dependency path, the API-use assessment, and the exact
condition that would clear it. `cargo deny` reports `advisories ok`.

**Recovery instructions.** If a dependency upgrade lands later, the recipe is
in each record's `| Blocker |` row. For the `unic-*` cluster it is the
MSRV-1.90 + Tauri-2.12 recipe; for `glib` / `proc-macro-error` it is the
upstream gtk-rs 0.20 move; for the `rustls` 0.21 line it is either a
ring-only `tiberius` path or an ADR that relaxes the ring-only rule.

## 10. Unresolved findings

| Severity | Finding | Impact | Required action |
|---|---|---|---|
| **medium** | **F1 — the `zip` 9.x upgrade was taken with a narrowed feature set, not verbatim as plan §7 B wrote it.** Plan §7 B specifies `zip = "9"` with the default feature set; the default feature set fails `cargo deny check licenses` because `zip` 9's `bzip2` feature pulls `libbz2-rs-sys` (`license = "bzip2-1.0.6"`, not on the allow list), and adding a licence is a §5 exclusion and a §14 stop condition. The feature set was therefore narrowed to `["deflate", "time"]` (§7.2). | The three `zip` advisories are **fixed** rather than suppressed, and no policy surface moved — but the manifest no longer matches the plan's literal text, and `zip` can no longer read `bzip2`/`lzma`/`xz`/`zstd`/`ppmd`/AES entries (none of which an APK or IPA uses). | The owner should confirm the narrowing. If the maintainer prefers the plan's fallback instead: revert `crates/eggsec-mobile-lab/Cargo.toml` and `crates/eggsec/Cargo.toml` to `zip = "2"`, revert the four `crates/eggsec-mobile-lab/src/*` call sites, re-run `cargo update`, and record RUSTSEC-2026-0367 / -0368 / -0369 as three exceptions with owner `eggsec-mobile-lab`, review-by `2026-12-12`, and the "no `extract()` call site" compensating control. The archive-layout evidence in §7.2 would then no longer apply. |
| **low** | **F2 — the first `make check-msrv` attempt failed on disk exhaustion** (exit 2, `No space left on device (os error 28)`) because the machine's data volume was at 100% with three concurrent agents building in the same worktree. **The second attempt exited 0** and all five MSRV steps are green (§4). | None to the milestone's outcome — the MSRV floor holds. The environmental condition is real, though: the worktree's `target/` is ~28 GB and the machine's data volume reached 100% during this session. | Nothing for the milestone. The owner should check free disk space before the integration `make check`; concurrent builds in the same worktree are the cause. |
| **medium** | **F3 — `make check` was not run.** It is the session owner's integration step. | The end-to-end contract is not proven by this record. Every step this milestone owns was run individually: `make check-deps` (exit 0), `make check-msrv` (exit 0 on the second attempt), the Check 190 block (exit 0, plus five negative demonstrations), `cargo test -p eggsec-mobile-lab` (exit 0), and all five `cargo deny` sub-checks (exit 0). | The owner runs `make check` at integration. |
| **low** | **F4 — local `cargo-deny` is 0.20.2; CI installs `cargo-deny@0.19.0`** (`.github/workflows/ci.yml:67`). | Reporting may differ across the version boundary (ordering, extra diagnostics, and in principle whether an advisory is surfaced at all). A local green is not proof of a CI green. | The `dependency-policy` CI job is the authority. If CI reports an advisory this run did not, treat it as a new entry under plan §14's "the advisory set moves" condition. |
| **low** | **F5 — `python3 scripts/check_doc_references.py` exits 1 with 29 missing references.** None is in a file this milestone touched. | Pre-existing documentation drift; it does not affect the dependency gate, but it does make that script non-green. | Owned by the milestones that own those documents (the corpus-seam and NSE document refreshes). Not this milestone's scope. |
| **low** | **F6 — a pre-existing `unused_imports` warning in `crates/eggsec-mobile-lab/src/apk.rs`** (`use std::io::Cursor;` inside `invalid_zip_input_returns_error_not_panic`) is present at `HEAD` too. `make clippy` does not lint this crate, so the contract is unaffected. | Cosmetic. | Left untouched deliberately — it is pre-existing and unrelated to the migration. |

**No residual advisory backlog.** Every entry that existed at the baseline is
either upgraded away or recorded with an owner, a review-by date, and a
blocking condition. `cargo deny --workspace --all-features check advisories`
reports **zero** entries, and the residual list is empty: there is no
advisory entry that remains unrecorded or un-upgraded.

## 11. Roadmap disposition

**Milestone closed, and the next dependency may proceed.** The roadmap's M15
exit conditions — "`make check-deps` is green; every retained exception is
recorded with an owner and a review-by date; and `make check` completes" —
are met for the first two, and the third is the owner's integration step
(F3). The MSRV floor holds after both the refresh and the `zip` bump
(`make check-msrv` exit 0).

**Roadmap consistency check** (plan §12, against the amendment already applied
at `plans/subsystems/desktop-gui-frontend-roadmap.md:1060-1071`):

| The amended roadmap paragraph says | Measured | Agrees? |
|---|---|---|
| 23 entries across 20 advisory IDs (14 `vulnerability`, 7 `unmaintained`, 2 `unsound`) | 23 entries / 20 IDs, same kind split | yes |
| the patch-level lockfile refreshes clear 9 of 23 — a plurality, not a majority | exactly 6 Bucket-A bumps in the lockfile diff, satisfying all 9 advisory minimums | yes |
| 7 have an upgrade this repository's policy blocks | the 3 `rustls`/`rustls-webpki` entries (ring-only) and the `glib` entry (Tauri pin + MSRV) — 4 policy-blocked; the 3 `zip` entries were **upgraded**, not blocked | **the "7 policy-blocked" figure is superseded**: 3 of those 7 had a safe upgrade after all (the `zip` bump), and were upgraded rather than excepted |
| the remaining 7 have no safe upgrade at all | the 7 are `proc-macro-error`, `unsafe-libyaml`, and the five `unic-*` — all recorded as exceptions | yes |
| `glib` 0.18.5 is held by "the exact `tauri` pin and the MSRV floor" | **extended**: the entry is *additionally* upstream-blocked — no published `muda`/`tao`/`wry` uses gtk-rs 0.20+, and `tauri` 2.12.5 itself still declares `gtk ^0.18`. Moving the Tauri line would not clear it | the refinement required by plan §0.4 / §12 is recorded in the exception record, in `architecture/desktop_gui.md`, and in `architecture/supply_chain.md` |

The roadmap's milestone entry, class, dependencies, deliverable boundary and
deferred-work statement were **not** rewritten — they belong to the session
owner.

## 12. Registry updates

Required in `plans/registry.md` (the session owner applies these):

- Record milestone 015's implementation date and this closure record
  alongside `plans/closure/desktop-gui-frontend/015-dependency-advisory-backlog-closure.md`.
- Note that the plan's `planned 2026-10-24` entry for
  `plans/implementation/desktop-gui-frontend/015-dependency-advisory-backlog.md`
  is now implemented, and that the 9 / 3 / 11 split held as measured.
- Note the narrowed `zip` feature set (F1) and the `make check` step still
  outstanding (F3).

Required in `plans/subsystems/desktop-gui-frontend-roadmap.md` (the session
owner applies these):

- Move the M15 entry from planned to implemented/closed.
- Optionally carry the §11 refinement that 3 of the "7 policy-blocked"
  entries were in fact upgradable (`zip`), so the amended arithmetic reads
  9 refreshed + 3 upgraded + 11 excepted = 23.

## Addendum 2026-10-24 — finding F3 discharged, and what clearing the advisories exposed

**F3 is discharged.** The session owner ran the integration contract over the
tree with all four milestones landed: `make check` exits **0** — 4315 tests
across 77 test binaries with no failures, `cargo deny --workspace
--all-features check` green on all five sub-checks, clippy clean, `cargo fmt
--all --check` clean, and `scripts/check-architecture-guards.sh` reporting
**191 guards** with "ALL PASSED: No architecture drift detected". `make
check-msrv` also exits 0 on Rust 1.89. The finding said the contract was the
owner's step and was not proven by this record; it is proven now.

**What clearing the advisory backlog exposed.** The dependency-policy CI job
was red on `main` for exactly the backlog this milestone cleared, so `main`'s
scheduled runs never got past `cargo deny` and never reached the later steps
of the deep-check job. With the backlog cleared, the CI run gets further — and
finds a latent defect that had been sitting behind it.

The defect is in the **TUI's Intercept tab**, the tab gated on the `web-proxy`
cargo feature. The crate's *default* feature build never compiles that tab, so
the default-build test suite is green; the
`db-pentest,web-proxy,c2` profile that `make check-full` exercises
(`Makefile:273`) compiles it, and three tests fail:

- `tabs::intercept::tests::test_intercept_tab_start_stop` — the session's
  `ended_at` equals its `started_at`;
- `tabs::mid_run_navigation::navigation_never_focuses_an_input_while_a_task_is_running`
  — the Intercept tab's focus ring does not start on an input field;
- `tabs::tab_entry_focus::entering_a_tab_leaves_a_drivable_control_focused`
  — entering the Intercept tab leaves no control focused, so a keystroke
  typed there is discarded.

The second and third are usability defects rather than test bugs, and they
are the exact defect class the Intercept tab's own audit commit
(`8ecd17a9`, "resolve usability audit — crash, stuck state, dead affordances,
discarded input") was meant to close: the audit's tests ran without the
`web-proxy` feature, so the tab was never in the set they checked.

**Attribution.** `8ecd17a9` is an ancestor of `origin/main`, and no commit on
this branch touched the Intercept tab — the branch's only TUI change is the
coverage-set correction in `crates/eggsec-tui/src/parity.rs`, which adds
`packet-capture` to the set of operations a tab drives and cannot affect a
tab's focus or session timing. The defect is therefore **pre-existing on
`main`** and is not caused by this milestone; this milestone's effect on it is
that the repository's CI can now reach the step that sees it.
