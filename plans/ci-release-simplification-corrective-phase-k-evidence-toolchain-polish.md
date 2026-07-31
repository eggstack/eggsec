# Corrective Phase K Plan: Final Evidence and Release-Toolchain Polish

## Status

Ready for implementation.

Corrective Phase J resolved the final implementation defect in the manual Rust
release path. Cargo now creates the `.crate` archives, the exact publishable set
is inventoried, every archive is inspected outside the source workspace, and
hosted CI evidence is recorded against the pushed implementation commit.

A final review found two documentation/evidence defects that prevent the
closure record from being fully self-consistent:

1. the Phase J local evidence table still names an inaccessible intermediate
   commit (`130c233`) rather than the committed implementation SHA
   (`b91d9f91499ff7cabfc34a8bd9eed0e64e86af43`) or the final documentation
   head;
2. the repository declares Rust 1.80 as the code MSRV, but Cargo 1.80.1 did not
   support the validated workspace packaging operation for the unpublished
   internal graph. Active release instructions do not yet state the separate
   release-tooling requirement clearly enough.

This is a documentation and evidence polish pass only. No release implementation,
CI workflow, runtime code, package manifest, public API, dependency graph, or
feature behavior should change.

## Objective

Produce a final closure record in which every local and hosted claim points to
an identifiable committed revision and the release prerequisites distinguish
code compatibility from release-tool compatibility.

The completed state must establish that:

- the complete local Phase J validation sequence was executed against an exact
  repository commit that exists in the remote history;
- the closure report uses that same SHA consistently for every Phase J local
  gate;
- no stale `130c233` reference remains in active closure documentation;
- Rust 1.80 remains the source/build MSRV unless separately changed through an
  explicit engineering decision;
- the supported manual release tooling is documented separately from the code
  MSRV;
- Cargo 1.80.1 packaging failure is retained as an observed result rather than
  hidden, upgraded to `PASS`, or used to imply that the code MSRV is invalid;
- release maintainers can determine whether their Cargo version supports the
  validated workspace packaging command before starting a release;
- no package, tag, or GitHub Release is published during validation.

## Accepted baseline that must not regress

All implementation outcomes through Corrective Phase J are accepted:

- `.github/workflows/ci.yml` remains the single mandatory workflow;
- `.github/workflows/deep-checks.yml` remains the only optional workflow;
- hosted workflows remain non-publishing and have no tag-triggered release
  side effects;
- Rust, Python, macOS portability, and Windows portability jobs remain compact;
- `make check`, `make check-python`, `make check-full`, and
  `make release-check` remain the canonical local command surface;
- Python semantic parity includes `is_vulnerable`;
- `Cargo.lock` remains minimized except for the justified `event-listener`
  security correction;
- publishable internal dependencies retain local paths plus registry versions;
- version-bump instructions include all internal dependency versions;
- Cargo owns `.crate` archive creation;
- the active packaging command remains:

  ```bash
  cargo package --workspace --no-verify --target-dir <isolated-target> \
    --exclude eggsec-cli --exclude eggsec-tui --exclude eggsec-python
  ```

- the helper enforces the exact 12-package archive set;
- every archive records package, version, path, size, and SHA-256;
- every extracted archive passes standalone
  `cargo metadata --no-deps --offline`;
- handwritten tar creation, regex manifest rewriting, and shell archive
  selection remain removed;
- registry preflight and publication remain separate manual operations;
- hosted CI run `30636819135` remains valid evidence for the implementation
  commit;
- CodeQL run `30636818358` remains valid evidence;
- branch-protection state remains `NOT VERIFIED` unless directly inspected.

Do not reopen the implementation design merely to complete this polish pass.

## Confirmed defects

### 1. Phase J local evidence names a non-remote SHA

The closure report identifies `b91d9f9` as the Phase J implementation commit,
but the Phase J evidence table attributes Cargo-native archive generation and
`make release-check` evidence to `130c233`.

`130c233` is not present in the accessible remote commit history. It may have
been a local pre-amend or pre-push commit, but the retained documentation does
not prove tree identity with `b91d9f9`.

The Phase J plan required final local evidence to be collected against the
committed implementation revision. An unavailable local SHA cannot satisfy that
recording requirement.

### 2. Final closure prose still refers to Phase I gates

The final status paragraph in
`plans/ci-verification-release-simplification-closure-report.md` still refers to
Corrective Phase I local gates even though Phase J superseded the Rust archive
validation and is the actual final implementation phase.

This is stale closure language. It should refer to the final Phase J/Phase K
validation record without rewriting the historical Phase I evidence section.

### 3. Code MSRV and release-tooling compatibility are conflated

The workspace declares:

```toml
rust-version = "1.80"
```

That declaration describes the supported compiler/toolchain floor for building
the project code. It does not automatically prove that Cargo 1.80 can execute
every maintainer-only release operation introduced later.

Phase J records that Cargo 1.80.1 could not package the complete unpublished
workspace graph using the selected workspace command. Current stable Cargo did
complete the operation successfully.

Active release instructions should therefore distinguish:

- **Code MSRV:** Rust 1.80, used for supported project compilation unless a
  separate change revises it.
- **Release-tooling requirement:** a stable Cargo version demonstrated to
  support the Phase J workspace package command. The exact tested version must
  be recorded when evidence is collected; maintainers should use that version
  or a newer compatible stable Cargo.

Do not raise the code MSRV merely to simplify release instructions.

## Scope

Primary files:

```text
plans/ci-verification-release-simplification-closure-report.md
plans/ci-release-simplification-corrective-closure-index.md
docs/RELEASING.md
docs/VERIFICATION.md
AGENTS.md
README.md
architecture/overview.md
```

Search-only review locations:

```text
.opencode/skills/
crates/eggsec-python/
docs/python/
plans/ci-release-simplification-corrective-phase-i-release-integrity.md
plans/ci-release-simplification-corrective-phase-j-cargo-native-packaging.md
```

Only update search-only files when they contain active, misleading release
prerequisite claims. Historical plan text may retain historical observations
when clearly labeled as historical or superseded.

Implementation files are out of scope unless a documentation claim cannot be
verified because the current command does not expose the required version
information. Even then, prefer recording `cargo --version` in the evidence run
rather than adding a new helper subsystem.

## Workstream 1 — Reopen the closure record narrowly

Before collecting replacement local evidence, update the corrective index and
closure report status to indicate that implementation is complete but final
evidence/toolchain polish is pending.

Recommended status language:

```text
Corrective Phase J implementation and hosted verification are complete.
Corrective Phase K is a documentation-only polish pass to bind local evidence
to a remote commit and distinguish the Rust code MSRV from the supported
release-tooling Cargo version. Publication remains manual and was not run.
```

Do not mark the Cargo-native implementation as blocked or incomplete. The
remaining issue is the provenance and clarity of the closure record.

## Workstream 2 — Select the authoritative validation commit

Use one of these two acceptable approaches:

### Preferred approach: validate current final head

1. apply the Phase K documentation changes except the final PASS evidence;
2. commit them;
3. run the complete local validation sequence against that committed SHA;
4. record that exact full SHA as the final closure commit;
5. push the final evidence update as a second documentation commit;
6. clearly distinguish:
   - implementation commit `b91d9f9...`;
   - final closure/evidence commit;
   - hosted CI run attached to the implementation commit.

This approach proves that active documentation and implementation coexist in a
clean validated tree.

### Acceptable bounded approach: revalidate the implementation commit

If maintainers prefer evidence directly against `b91d9f9...`:

1. check out a clean detached worktree at
   `b91d9f91499ff7cabfc34a8bd9eed0e64e86af43`;
2. run the required validation sequence there;
3. record that exact SHA and host/tool versions in the closure report;
4. return to `main` and apply documentation-only corrections;
5. state explicitly that current `main` differs only by planning/evidence
   documentation from the validated implementation tree.

Do not use an abbreviated or inaccessible local SHA as the authoritative
reference.

## Workstream 3 — Collect a bounded final local evidence record

Record the environment before running gates:

```bash
git rev-parse HEAD
git status --porcelain
uname -a
rustc --version --verbose
cargo --version --verbose
python3 --version
```

Required gate sequence:

```bash
python3 scripts/test_release_package_graph.py
python3 scripts/release-package-graph.py validate
python3 scripts/release-package-graph.py order
cargo metadata --locked --format-version 1 >/tmp/eggsec-final-metadata.json
make check
make check-python
make check-full
make release-check
```

Requirements:

- the working tree must be clean before validation;
- every command must complete against the same exact commit;
- a timeout is `TIMEOUT`, not `PASS`;
- an unavailable tool is `NOT RUN` or `BLOCKED`, not `PASS`;
- registry preflight remains `SKIPPED` unless explicitly invoked;
- publication remains `NOT RUN`;
- retain the exact `cargo --version --verbose` output used for the successful
  packaging proof;
- record the package-helper test count actually observed rather than copying a
  prior count;
- record that all 12 Cargo archives were generated and inspected only when the
  final command output confirms it.

Do not add another evidence bundle, JSON schema, or archival framework. A
concise Markdown evidence table is sufficient.

## Workstream 4 — Correct closure-report provenance

Update
`plans/ci-verification-release-simplification-closure-report.md` so the Phase J
and final evidence sections consistently identify:

- full Phase J implementation SHA:
  `b91d9f91499ff7cabfc34a8bd9eed0e64e86af43`;
- final local validation SHA selected in Workstream 2;
- validation host and architecture;
- `rustc --version --verbose` result;
- `cargo --version --verbose` result;
- Python version where relevant;
- exact local gate outcomes;
- hosted CI run `30636819135` and its four successful jobs;
- CodeQL run `30636818358` and its successful Python analysis;
- branch protection as `NOT VERIFIED` unless inspected directly;
- registry preflight as `SKIPPED` unless actually run;
- publication as `NOT RUN`.

Remove all active claims that `130c233` is the authoritative validation commit.

Historical evidence sections may retain their own historical SHAs, but they
must be clearly labeled as superseded and must not be used in the final status
rule.

Replace stale final prose referring only to Phase I with language such as:

```text
Corrective Phase J established the final Cargo-native release implementation.
The Phase K evidence pass bound all blocking local gates to the recorded remote
commit and retained the verified hosted run conclusions. The CI/manual-release
simplification line is closed; registry preflight and publication remain manual
operations outside this closure.
```

## Workstream 5 — Document the release-tooling contract

Update `docs/RELEASING.md` under prerequisites or supported release environment.

Required distinction:

```text
Code MSRV: Rust 1.80.
Release tooling: use the stable Cargo version recorded in the latest successful
release-check evidence, or a newer compatible stable Cargo. Cargo 1.80.1 is not
claimed to support the workspace package operation used by release-check.
```

Record the exact successful Cargo version from Workstream 3 rather than using a
floating phrase alone.

The documentation must explain:

- the code MSRV remains Rust 1.80;
- the release script is a maintainer tool and may require newer Cargo behavior;
- a maintainer should run `cargo --version --verbose` before release validation;
- `make release-check` is the authoritative compatibility check for the chosen
  release toolchain;
- failure of the release package command on Cargo 1.80.1 does not by itself
  prove that ordinary project builds violate the Rust 1.80 MSRV;
- Linux remains the tested release host;
- macOS release-script compatibility remains unverified;
- no specific future Cargo release is guaranteed merely because it is newer;
  successful `make release-check` remains required.

Avoid introducing a hard-coded Cargo minimum in `Cargo.toml`; Cargo has no
separate manifest field for maintainer release-tool requirements, and this pass
must not alter the project MSRV.

## Workstream 6 — Reconcile active documentation

Search active documentation for misleading combinations of `MSRV`, `Rust 1.80`,
`Cargo 1.80`, `release-check`, and `cargo package`:

```bash
rg -n '130c233|MSRV|Rust 1\.80|Cargo 1\.80|cargo --version|release tooling|release-check|cargo package --workspace' \
  README.md AGENTS.md architecture docs crates/eggsec-python .opencode/skills plans
```

Rules:

- remove `130c233` from active final evidence;
- retain historical references only when necessary and clearly identified;
- do not claim Cargo 1.80 release compatibility;
- do not claim the code MSRV has increased;
- do not repeat the full release-tool contract across many files;
- designate `docs/RELEASING.md` as authoritative and use short references
  elsewhere;
- ensure the exact active Cargo packaging command remains unchanged and
  documented accurately;
- retain the distinction among local archive validation, registry dry-run, and
  actual publication.

Recommended minimal secondary wording:

```text
The code MSRV and maintainer release-tooling requirements are separate; see
`docs/RELEASING.md` for the currently validated Cargo environment.
```

## Workstream 7 — Final index closure

After all local gates pass and documentation is committed:

1. update the corrective index to mark Phase K complete;
2. retain Phases G through J as completed historical phases;
3. identify Phase K as the final evidence/toolchain polish phase;
4. state that no implementation blocker remains;
5. retain the final evidence classification:

   ```text
   CI workflow simplification: PASS
   Python semantic parity: PASS
   Lockfile minimization: PASS
   Version-bump procedure: PASS
   Cargo-native Rust archives: PASS (12/12)
   Final local validation commit: <full remote SHA>
   Hosted CI run 30636819135: PASS
   CodeQL run 30636818358: PASS
   Branch protection: NOT VERIFIED
   Registry preflight: SKIPPED
   Publication: NOT RUN
   ```

6. explicitly state that the release-tool Cargo version is documented and is
   distinct from the Rust 1.80 code MSRV.

Do not create Phase L or another roadmap unless a new implementation defect is
found during the final validation run.

## Validation searches

### Stale SHA search

```bash
rg -n '130c233' .
```

Expected result after closure:

- no active closure or release documentation references it;
- a historical note is allowed only when explicitly explaining the superseded
  local SHA and is generally unnecessary.

### Final provenance search

```bash
rg -n 'b91d9f91499ff7cabfc34a8bd9eed0e64e86af43|30636819135|30636818358' \
  plans/ci-verification-release-simplification-closure-report.md \
  plans/ci-release-simplification-corrective-closure-index.md
```

Expected:

- implementation SHA appears consistently;
- hosted run IDs appear with accurate conclusions;
- final local validation SHA appears in the final evidence section.

### Release-tooling search

```bash
rg -n 'Code MSRV|Release tooling|Cargo 1\.80\.1|cargo --version --verbose' \
  docs/RELEASING.md docs/VERIFICATION.md AGENTS.md README.md architecture
```

Expected:

- `docs/RELEASING.md` contains the authoritative distinction;
- other active files contain at most concise references;
- no file claims Cargo 1.80.1 passed the workspace package operation.

### No-scope-expansion diff

```bash
git diff --name-only <phase-k-base>..HEAD
```

Expected files should be limited to planning, release documentation, and
possibly concise documentation references. The following should not change:

```text
.github/workflows/
Cargo.toml
Cargo.lock
crates/*/Cargo.toml
crates/*/src/
src/
scripts/release-check.sh
scripts/release-package-graph.py
scripts/test_release_package_graph.py
Makefile
```

If an implementation file changes, document why and reassess whether this is
still a polish-only phase.

## Acceptance criteria

Phase K is complete only when all of the following are true:

1. The corrective index and closure report are reopened for Phase K before new
   final evidence is claimed.
2. The Phase J implementation remains unchanged.
3. The final local validation is executed against one exact clean commit.
4. That commit exists in the remote repository history.
5. The full validation SHA is recorded, not only an abbreviation.
6. `130c233` is removed from active final evidence.
7. `git status --porcelain` was empty before the evidence run.
8. `rustc --version --verbose` is recorded.
9. `cargo --version --verbose` is recorded.
10. `python3 --version` is recorded where Python gates are claimed.
11. Package-helper tests pass against the selected commit.
12. The real workspace package graph validates and orders successfully.
13. Locked Cargo metadata succeeds.
14. `make check` passes.
15. `make check-python` passes.
16. `make check-full` passes.
17. `make release-check` passes.
18. The final release check confirms 12/12 Cargo-generated archives.
19. Every final command result is classified honestly.
20. Registry preflight is `SKIPPED` unless actually run.
21. Publication is `NOT RUN`.
22. Hosted CI run `30636819135` remains accurately recorded as successful for
    Rust, Python, macOS, and Windows jobs.
23. CodeQL run `30636818358` remains accurately recorded as successful.
24. Branch protection remains `NOT VERIFIED` unless directly inspected.
25. The closure report no longer describes Phase I as the final gate owner.
26. `docs/RELEASING.md` distinguishes the Rust 1.80 code MSRV from the
    maintainer release-tooling Cargo requirement.
27. The exact successful Cargo version is documented.
28. Cargo 1.80.1 package-command failure remains documented without being
    misrepresented as a code-MSRV failure.
29. Linux remains the tested release host.
30. macOS release-script support remains unclaimed.
31. No hard-coded release Cargo minimum is added to the project manifest.
32. No workflow, implementation script, runtime code, manifest, lockfile, or
    dependency changes are introduced.
33. No package, tag, or GitHub Release is published.
34. The corrective index marks Phase K complete only after the final evidence
    table is updated.
35. No further corrective phase is opened absent a newly demonstrated defect.

## Explicit non-goals

- Changing the Rust code MSRV.
- Adding a Cargo version gate to ordinary builds or CI.
- Adding a new release script or package helper.
- Re-running or redesigning hosted CI workflows.
- Adding a release workflow.
- Publishing crates, Python packages, tags, or GitHub Releases.
- Running registry preflight solely to make the evidence table larger.
- Introducing a local registry.
- Adding Docker, provenance, signing, SBOM, attestation, or release bots.
- Modifying runtime behavior or public APIs.
- Updating dependencies.
- Reorganizing crates.
- Creating a new evidence framework.
- Expanding the testing matrix.

## Rollback strategy

If any final local gate fails:

1. do not mark Phase K complete;
2. record the exact command and result as `FAIL`, `BLOCKED`, or `TIMEOUT`;
3. determine whether the failure is environmental, documentary, or a newly
   exposed implementation defect;
4. fix only a documentary/environmental issue within Phase K;
5. if a genuine implementation defect is found, reopen the relevant prior
   phase or create a narrowly scoped implementation plan rather than hiding the
   result in documentation;
6. do not alter the Rust MSRV or release architecture as an expedient workaround.

## Handoff notes

This pass should be small. The expected implementation is one documentation
commit, one clean local validation run, and one final evidence/index commit.

The most important requirement is provenance: every final PASS must name a
remote commit and the exact toolchain used. The second requirement is semantic
clarity: Rust 1.80 remains the code MSRV, while the maintainer release command
uses the separately documented Cargo version that actually passed the Phase J
workspace packaging proof.
