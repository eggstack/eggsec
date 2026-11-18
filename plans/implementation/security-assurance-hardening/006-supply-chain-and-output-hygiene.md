# Security Assurance Hardening Milestone 006 — Supply-Chain and Output Hygiene

Status: conditionally closed

Repository baseline: `77188755` (`feat/auto-20261117-7451c367`, 2026-11-17)

Source roadmap:

- `plans/subsystems/security-assurance-hardening-roadmap.md` §7 Milestone 6
- Findings A8, A9, A10 in §4 of the same roadmap

Long-term requirements:

- `plans/000-long-term-specification.md` (supply chain, release integrity)
- `architecture/supply_chain.md`
- `AGENTS.md` — "`make check-deps` fails closed when `cargo-deny` is absent"

Applicable ADRs: none.

Primary class: polish

## 1. Objective

Remove stale and dirty shipped artifacts, stop generated reports from executing
unpinned third-party code, and refresh the dependency-exception review record so
the remaining risk is explicit rather than inherited.

## 2. Why this milestone is ready

- No hard dependency.
- `deny.toml` is already the canonical policy source and the documented
  exception process exists (`docs/DEPENDENCY_EXCEPTIONS.md`).
- The report emitters already have a single escaping module and a test module;
  adding a script-source assertion is local.
- The container files are not exercised by `make check`, so this work cannot
  destabilize the Rust contract.

## 3. Current implementation evidence

### A9 — container and repository artifacts

- `Dockerfile:5` builds `FROM rust:1.80-bookworm`; the workspace MSRV is 1.89
  (`Cargo.toml` `rust-version`, `rust-toolchain.toml`).
- `Dockerfile:18` copies `crates/eggsec-nse/Cargo.toml`. That crate does not
  exist; NSE is the external `eggsec-nse` dependency. The `COPY` fails before
  any build starts.
- The Dockerfile copies 8 of 23 workspace manifests before the dummy-source
  build. A workspace build resolves every member manifest, so the reduced set
  cannot satisfy `cargo build` even at the correct toolchain.
- `cargo build -p eggsec-cli --release --features full` is itself valid:
  `eggsec-cli` defines `full` (`crates/eggsec-cli/Cargo.toml:76-85`, aggregating
  `eggsec/full`, `eggsec-tui?/full`, mobile, wireless-advanced, db-pentest, and
  web-proxy). That part of the command is correct and should be preserved.
- `docker-compose.yml` hardcodes `eggsec123` (Redis, Tor control, Elasticsearch)
  and `labpass` (Postgres/MySQL/MSSQL labs), sets
  `xpack.security.enabled=false`, and publishes Elasticsearch `9200`, Redis
  `6379`, and Tor `9050/9051` on the host.
- Tracked root artifacts unrelated to this project:
  `config.toml` (551 lines, an "eggpool" model-router config, no live secrets —
  all keys are commented placeholders) and `package.json` (`{}`) +
  `package-lock.json` (6-line empty lockfile named `eggsec`). The latter two are
  read by the SBOM scanner (`architecture/supply_chain.md:55,68`), so their
  presence is not inert: they make a repository SBOM report an npm surface.

### A8 — report script loading

```html
<!-- crates/eggsec-output/src/html.rs:68 -->
<script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
```

No `integrity`, no `crossorigin`, no CSP. Any party controlling that path (CDN
compromise, DNS/network interception, offline environment serving a different
answer) executes script in the report's origin when an analyst opens it. Field
escaping is otherwise thorough; this is the only third-party execution in the
output.

### A10 — dependency exceptions (baseline and post-remediation status)

The counts below are the audit snapshot for baseline `77188755`, not a live
claim about current GitHub alert state. The 2026-11-17 snapshot recorded 16
`deny.toml` advisory ignores and seven Dependabot alerts; the latter remain
identified as a dated snapshot in `docs/DEPENDENCY_EXCEPTIONS.md`.

- During Milestone 006, `lettre` was updated from 0.11.23 to 0.11.24. This
  resolved RUSTSEC-2026-0560 through RUSTSEC-2026-0565; they are not retained
  as active exceptions. The all-features dependency-policy command passes.
- The primary TLS stack is fixed (`rustls 0.23.46`,
  `rustls-webpki 0.103.16`); the flagged older copies are the `tiberius` →
  `rustls 0.21.12` / `webpki 0.101.7` path, plus `lopdf 0.31` via
  `printpdf 0.7`.
- `db-pentest` remains a member of the engine `full` feature, but the MSSQL
  `tiberius` adapter is separately gated by
  `db-pentest-mssql-tiberius`. The full CLI feature graph excludes
  `rustls 0.21.12`; the explicit MSSQL feature includes the `tiberius` 0.12.3
  path. Those exceptions therefore apply to the optional MSSQL path, not the
  Docker `full` build. `pdf` is also optional, so `lopdf` requires its explicit
  feature.
- `RUSTSEC-2023-0071` (`rsa` Marvin timing side-channel) has no fixed version
  and is reachable through `sqlx-mysql` in `db-pentest`.
- The retained `RUSTSEC-2026-0543` (`chrono`) exception's current review-by
  date is 2027-02-15 (not the earlier 2026-12-12 date); see
  `docs/DEPENDENCY_EXCEPTIONS.md` for owner, review date, and migration blocker.

## 4. Invariants that must not regress

- `deny.toml` remains canonical; no `.cargo/audit.toml` and no undocumented
  ignore is added.
- `make check-deps` keeps failing closed when `cargo-deny` is absent and keeps
  running `--workspace --all-features`.
- Every new or retained exception has an owner and a review-by date in
  `docs/DEPENDENCY_EXCEPTIONS.md`.
- Ring-only TLS and the `aws-lc-rs` ban are not weakened.
- The report keeps working offline: summary tables must render with no network.
- HTML/XML/CSV escaping is unchanged, and CSV formula neutralization is not
  touched.
- `make check` and `make check-deps` pass.

## 5. Scope

### In scope

- `Dockerfile` correctness (MSRV, crate set, feature selection).
- Docker build-context exclusion of local Compose credentials and Git metadata.
- `docker-compose.yml` credential parameterization and lab-only exposure.
- Disposition of root `config.toml`, `package.json`, `package-lock.json`.
- Report chart loading (vendor or SRI) plus a no-remote-script assertion.
- Dependency-exception review record refresh and the `db-pentest` in `full`
  decision.

### Explicitly out of scope

- Adding `cargo-chef` or any new build-caching dependency.
- Base-image digest pinning across the repository.
- A SBOM policy change or a new scanner.
- Migrating `chrono`, `lopdf`, `rsa`, or `tiberius` (these are recorded
  decisions with dates, not this milestone's work).
- Changing the `full` feature set itself (only its documentation/decision).

## 6. Required production changes

### Core/domain

None in the engine.

### Storage and migrations

None.

### Protocol and DTOs

None.

### Runtime and concurrency

None.

### Frontend or operator surface

**Report output.** Replace the CDN `<script>` with one of:

- **Preferred:** vendor a pinned Chart.js build into `eggsec-output` (e.g.
  `assets/chart.min.js` included with `include_str!`) and inline it in the
  report. This keeps reports offline and removes third-party trust entirely.
  Note the file size in the closure record and confirm the crate's `exclude`
  list does not strip the asset from publication.
- **Fallback:** pin an exact version, add `integrity="sha384-..."` and
  `crossorigin="anonymous"`, and add a CSP `<meta>` that allows only the
  computed hash (`script-src 'sha384-...'`).
- In both cases the report must render its summary without the script: guard the
  chart construction on the library being present.

**Container.** Rebuild the Dockerfile:

- `FROM rust:1.89-bookworm` (or the value in `rust-toolchain.toml`; read it
  first and keep one source of truth).
- Remove the `crates/eggsec-nse` copy.
- Remove the dummy-source caching stage and copy the workspace (`COPY . .`), OR
  copy all 23 manifests. Prefer removing the stage: correctness over cache, and
  the current stage is already wrong.
- Add a root `.dockerignore` that excludes root/nested `.env` and `.env.*` files
  and `.git`, so local Compose credentials and repository history are not sent
  to the builder. `.dockerignore` does not prevent Compose from reading `.env`
  for interpolation.
- Keep `--features full`. It is valid (`crates/eggsec-cli/Cargo.toml:76-85`);
  do not invent a replacement. Verify the resulting binary includes the intended
  surfaces and record the feature list in the closure evidence.
- Pin the toolchain to the workspace MSRV (`Cargo.toml` `rust-version = "1.89"`),
  not to whatever `rust-toolchain.toml` says for day-to-day development: that
  file pins `stable` and explicitly defers the MSRV to `Cargo.toml`.
- Keep the non-root runtime user and `ENTRYPOINT`.

**Compose.** Replace every literal credential with an environment substitution
that fails closed (`${REDIS_PASSWORD:?set REDIS_PASSWORD}`), keep the lab
profiles, and bind lab service ports to `127.0.0.1` rather than all interfaces.
For Elasticsearch, either keep the lab-only security-off setting with a comment
that it is lab-only and loopback-bound, or enable the built-in security. Do not
silently change the lab's functional behavior without recording it.

**Root artifacts.**

- `config.toml`: delete. It belongs to another project, has no references in
  this repository's code or CI, and risks being mistaken for an Eggsec config.
- `package.json` / `package-lock.json`: disposition explicitly.
  - If they were accidental (most likely), delete them.
  - If they exist as SBOM fixtures, move them under a documented fixture path
    (e.g. `crates/eggsec/tests/fixtures/supply-chain/npm/`) so the repository
    root does not masquerade as an npm project, and update
    `architecture/supply_chain.md` if it names the root path.
  Record the decision.

### Security and authorization

No policy change. The dependency decisions below must not weaken `deny.toml`
enforcement.

### Documentation and static guards

- `DOCKER.md` / `DOCKER_COMPOSE.md` for the new variables and loopback binds.
- `docs/DEPENDENCY_EXCEPTIONS.md`: add an alert→exception mapping table with
  the 7 open alerts and their exception IDs, and refresh review-by dates.
- `architecture/supply_chain.md` if the npm fixture path changes.
- Guard: assert the generated HTML contains no `https://` script source. This
  can be a unit test rather than a shell guard.

## 7. Ordered work packages

### Work package A — Report third-party trust

Intent: open a report without executing remote code.

Required changes: vendor or SRI as in §6; guard the chart call; test the output.

Acceptance evidence: a unit test that `convert_to_html(...)` output contains no
`<script src="https://`; the report still renders summary values.

### Work package B — Dockerfile repair

Intent: `docker build .` succeeds at the real MSRV.

Required changes: as in §6.

Acceptance evidence: a successful local image build and
`docker run --rm eggsec:test --version`; the feature list quoted in the closure
record.

### Work package C — Compose hardening

Intent: no hardcoded credentials and no unintended host exposure.

Required changes: substitutions that fail closed; loopback port binds; lab-only
comments.

Acceptance evidence: `docker compose config` with variables set and unset
(unset must fail with a clear message); no literal password remains in the file.

### Work package D — Root artifact disposition

Intent: the repository root reflects this project only.

Required changes: delete `config.toml`; delete or relocate the npm files.

Acceptance evidence: `git status` clean of the removed paths; a repo-wide
reference check (grep) shows no build/CI dependency; SBOM tests unchanged or
updated.

### Work package E — Dependency-exception review

Intent: the remaining risk is explicit and dated.

Required changes:

- Add the alert mapping table.
- Re-confirm owners and review-by dates for all 16 ignores; any exception
  without a date gets one.
- Keep `db-pentest` in `full`; do not imply that this enables Tiberius. Record
  that `db-pentest-mssql-tiberius` is an independent opt-in marker and verify
  both the `full` graph (no rustls 0.21.12) and the optional MSSQL graph (the
  tiberius path is present). Removing `db-pentest` from `full` is out of scope.
- Confirm the `chrono` review-by date and that the doc explains why a
  version-only fix does not exist.

Acceptance evidence: `make check-deps` passes; the doc renders the mapping
correctly; no undocumented ignore is present.

## 8. Failure, cancellation, restart, and contention semantics

- Image build failure is a build-time error; no runtime semantics.
- Compose with an unset required variable fails at `config` time, before any
  container starts.
- Report without the chart library must still render summaries; a missing
  vendored asset is a compile error (`include_str!`), not a runtime failure.
- No concurrency, cancellation, or restart behavior changes.

## 9. Compatibility and migration

- The Dockerfile's corrected feature list may produce a differently sized binary
  than the (broken) previous definition; document what is included.
- Compose users must set the now-required variables; update `DOCKER_COMPOSE.md`
  and the example `.env` if one exists.
- Removing root `package.json` changes what a repository-level SBOM scan
  reports (no npm surface). That is the intended correction; state it.
- The report gains an inline asset (size increase) or an SRI/CSP requirement.
  Downstream consumers that post-process the HTML must be told.

## 10. Required tests

### Focused unit tests

- `convert_to_html` output contains no remote script source.
- `convert_to_html` still escapes a hostile title/target (existing tests remain).
- Chart guard: output without the library still contains the summary numbers.

### Integration tests

- `make check` (report tests live in `eggsec-output`).
- `docker build` and `docker compose config` are manual evidence, recorded.

### Restart and recovery tests

Not applicable.

### Contention and cancellation tests

Not applicable.

### Security and negative tests

- No literal password remains in `docker-compose.yml` (grep assertion, manual).
- Unset compose variable fails closed.
- `make check-deps` still fails when an undocumented ignore is added (verify by
  temporary local edit, then revert).

### Migration and compatibility tests

- `cargo metadata` / `cargo check --workspace --no-default-features` after the
  Dockerfile feature decision (the Dockerfile is not compiled by CI, so verify
  the chosen feature set with a local `cargo build`).

## 11. Required verification commands

```bash
make check
make check-deps
docker build -t eggsec:audit-006 .
docker run --rm eggsec:audit-006 --version
docker compose config
```

If the environment cannot run Docker, record that the container evidence is
unverified rather than claiming it passed.

Do not claim commands that were not actually run in the closure record.

## 12. Documentation updates

- `DOCKER.md`, `DOCKER_COMPOSE.md`.
- `docs/DEPENDENCY_EXCEPTIONS.md` (mapping table + dates).
- `architecture/supply_chain.md` (only if the npm fixture path changes).
- `CHANGELOG.md` if the project records container/report changes there.

## 13. Acceptance criteria

1. `docker build` succeeds from a clean checkout at the workspace MSRV and the
   image runs `eggsec --version`.
2. The Dockerfile references no nonexistent crate and builds at the declared
   MSRV; `--features full` remains and resolves.
3. `docker-compose.yml` contains no hardcoded credential and fails closed when a
   required variable is unset.
4. The Docker build context excludes local `.env` / `.env.*` files and `.git`.
5. Lab service ports are loopback-bound or explicitly documented as
   intentionally exposed.
6. Report HTML contains no remote script source and still renders summaries.
7. Root `config.toml` is removed; npm files are removed or relocated with a
   recorded rationale.
8. `docs/DEPENDENCY_EXCEPTIONS.md` maps all 7 open alerts to exception IDs with
   owner and review-by dates.
9. `make check` and `make check-deps` pass.

## 14. Stop conditions

Stop and report rather than improvise when:

- the Dockerfile needs a build-caching tool that is not already a dependency;
- the workspace cannot be made to build in the image without network access the
  environment does not provide (report the blocker; do not vendor sources);
- enabling Elasticsearch security would change the lab's documented behavior in
  a way that breaks the smoke scripts;
- a dependency decision would require removing `db-pentest` from `full`
  (behavior change; needs explicit approval);
- deleting `package.json` would break a test that depends on the repository root
  as an SBOM fixture (inspect before deleting).

## 15. Closure evidence required

- Commit(s)/PR.
- `make check` and `make check-deps` output.
- Docker build output and the exact feature list used, or an explicit statement
  that the container build could not be run.
- `docker compose config` output for the set/unset cases.
- The alert→exception mapping table as added.
- The root-artifact disposition decision.
- The report test output showing no remote script source.

## 16. Handoff notes

- Read `rust-toolchain.toml` and the workspace `rust-version` before choosing the
  image tag; they must agree or the plan stops.
- The Dockerfile's dummy-source stage is the reason the crate list drifted; do
  not "fix" it by adding one more manifest and leaving the rest.
- `crates/eggsec-output` has an `exclude` list in its manifest; if you vendor the
  chart asset, verify it is not excluded from publication.
- Do not touch the CSV formula-injection logic while editing report output.
- Preserve unrelated user changes in the worktree.
