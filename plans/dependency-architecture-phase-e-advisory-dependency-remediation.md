# Phase E Plan: Advisory Cleanup and Dependency Security Remediation

## Status

Executed 2026-08-07. All 17 stale ignores removed. 7 live exceptions documented
in `docs/DEPENDENCY_EXCEPTIONS.md` with review-by dates. rand upgraded (patch).
PyO3 and quick-xml major upgrades deferred to Phase H (MSRV/major-version
blockers).

## Objective

Restore `cargo deny` and dependency review as meaningful security signals by
removing stale global suppressions, upgrading direct security-relevant
dependencies, and establishing a narrow time-bounded exception process for
advisories that cannot yet be removed.

This phase must reduce risk without turning dependency maintenance into an
unbounded all-at-once ecosystem upgrade.

## Current concerns

The current advisory configuration contains a long ignore list covering direct
and transitive dependencies, including Python binding, XML, HTTP/TLS, PDF, and
utility paths. Several reasons describe already-upgraded versions or temporary
“awaiting database update” states without expiry dates.

The repository also has direct dependencies on older major/minor lines whose
security and MSRV constraints influence later architecture work. The most urgent
direct paths identified in review include:

```text
PyO3
Quick-XML
reqwest/rustls-webpki paths
printpdf/lopdf path
scraper/selectors path
notify path
indicatif path
```

The actual current advisory database and dependency graph must be checked at
implementation time. This plan does not assume every historical ignore still
matches.

## Scope

Primary files:

```text
deny.toml
Cargo.toml
Cargo.lock
crates/*/Cargo.toml
crates/eggsec-python/src/
crates/eggsec-python/python/
crates/eggsec-python/tests/
crates/eggsec-output/src/
crates/eggsec/src/
crates/eggsec-nse/src/
crates/eggsec-web-proxy/src/
docs/VERIFICATION.md
docs/BUILD.md
docs/RELEASING.md
AGENTS.md
```

Only touch product code when required by a dependency API migration or to remove
an actually unused vulnerable path.

## Non-goals

This phase does not:

- upgrade every dependency to latest in one commit;
- remove an advisory ignore without confirming the dependency path;
- claim an advisory is irrelevant merely because the feature is optional;
- replace mature dependencies solely to avoid an advisory entry;
- raise MSRV implicitly without documenting and testing it;
- publish a release;
- add multiple dependency scanners to compensate for weak exception hygiene;
- keep a vulnerable direct dependency solely to preserve an outdated nominal
  MSRV.

## Required exception policy

Every retained advisory ignore must include, in adjacent comments or a companion
Markdown table:

```text
advisory ID
direct or transitive dependency path
affected feature/artifact
whether the affected API is used
exploitability assessment for Eggsec
compensating control, if any
owner or owning subsystem
created/reviewed date
mandatory review-by date
upgrade/removal blocker
```

The review-by date should normally be no more than 60–90 days away. Exceptions
must not use indefinite language such as “waiting for upstream” without an issue,
version condition, or review deadline.

Do not create an elaborate exception database. A concise, consistently formatted
`deny.toml` comment block or one small `docs/DEPENDENCY_EXCEPTIONS.md` file is
sufficient.

## Workstream 1 — Reproduce and classify current advisories

Run:

```bash
cargo deny check advisories
cargo tree -d
cargo tree -i <affected-crate>
cargo metadata --locked --format-version 1
```

For each current ignore:

1. confirm whether the advisory still triggers without the ignore;
2. identify all paths and feature profiles that include it;
3. determine whether it is direct, optional direct, dev-only, build-only, or
   transitive;
4. confirm whether Eggsec calls the affected API;
5. classify as remove-stale-ignore, patch/upgrade, feature-isolate, replace,
   temporary-retain, or no-longer-present.

Record results in the implementation PR or the bounded dependency exception
document. Do not generate an evidence bundle.

## Workstream 2 — Remove stale and false-positive suppressions

Immediately remove ignores when:

- the affected version is no longer in `Cargo.lock`;
- the advisory database now recognizes the patched version;
- the dependency path was removed;
- the ignore references a version or reason that no longer applies;
- the advisory was withdrawn or corrected upstream.

Run `cargo deny check advisories` after each logical batch so one stale ignore does
not obscure another.

## Workstream 3 — Upgrade PyO3 in a bounded migration

Upgrade the Python binding from the current 0.22 line to a maintained release
that resolves the applicable direct advisories and supports the chosen MSRV.

Required migration steps:

- review PyO3 migration guides for every crossed release;
- update `pyo3`, maturin compatibility, and build metadata coherently;
- compile the extension before broad source edits to capture actual errors;
- migrate GIL/bound API, conversion, module, type, and `PyResult` changes in small
  batches;
- preserve stable Python package names, operation APIs, exceptions, stubs, and
  sync/async behavior;
- run installed-package tests, not only Rust unit tests;
- update the eventual MSRV decision explicitly rather than allowing transitive
  Cargo resolution to set it accidentally.

Do not combine this migration with unrelated Python API expansion.

## Workstream 4 — Upgrade direct Quick-XML users

Upgrade direct Quick-XML dependencies to a non-advisory line and adapt report,
configuration, mobile, or protocol code as required.

Before changing code:

- identify direct `Reader`, `NsReader`, deserialization, and writer use;
- identify transitive older Quick-XML versions and their owners;
- distinguish direct upgrade work from transitive blockers.

Required validation:

- parsing remains bounded for untrusted XML;
- namespace behavior is tested where relevant;
- report output remains stable or documented;
- malformed and adversarial input tests cover the affected parser mode;
- no unbounded buffering or event loop is introduced.

If a transitive package retains an affected Quick-XML line, document the exact
path and upgrade blocker separately. Do not keep a blanket ignore that also hides
direct usage.

## Workstream 5 — Resolve HTTP/TLS advisory paths

Inspect reqwest, Rustls, rustls-webpki, native-tls, and OpenSSL dependency paths
across engine, Python, NSE, proxy, and daemon dev dependencies.

Actions may include:

- updating lockfile-compatible patch versions;
- aligning direct reqwest declarations;
- removing unused native-tls paths;
- choosing one Rustls provider in Phase G;
- narrowing optional feature activation;
- upgrading transitive owners.

Do not prematurely make the broad TLS-provider topology change in this phase if
it would entangle advisory correction with artifact restructuring. A minimal
patch may land here, followed by Phase G consolidation.

## Workstream 6 — Review PDF, scraper, notify, and UI utility paths

For each remaining advisory path:

- determine whether the feature is compiled in standard artifacts;
- determine whether the affected API handles untrusted input;
- check for a maintained compatible upgrade;
- isolate the dependency behind an existing or new narrow adapter feature if it
  is genuinely optional;
- remove unused functionality only when no user-visible capability depends on
  it;
- document a bounded exception otherwise.

PDF report generation, HTML parsing, file watching, SMTP notification, and
progress UI should later move to narrower adapter boundaries in Phase F. Avoid
large code movement here unless it is the cleanest way to eliminate the advisory
path.

## Workstream 7 — Add dependency exception hygiene checks

Add a small script or test only if needed to enforce review dates and required
fields. Preferred implementation:

- structured comments or a small TOML/Markdown table;
- one script that fails only for expired or malformed exceptions;
- invoked by `make check-full`/scheduled diagnostics, not necessarily every
  mandatory PR.

Do not introduce a new service, bot, issue synchronizer, or generated report.

## Workstream 8 — Update dependency policy documentation

Document:

- direct versus transitive advisory ownership;
- required exception fields;
- review cadence;
- when an optional feature advisory blocks release;
- when MSRV may be raised to accept a security fix;
- that `cargo deny` is an optional broad/release-preparation diagnostic unless a
  critical direct advisory requires immediate merge blocking;
- that release publication remains manual.

Keep `docs/VERIFICATION.md` authoritative for command placement.

## Validation commands

Use staged validation:

```bash
cargo deny check advisories
cargo tree -d
cargo check --workspace --no-default-features
cargo test -p eggsec --lib
cargo test -p eggsec-output
cargo test -p eggsec-python
make check-python
make check-full
```

Run feature-specific checks for dependencies actually changed, for example:

```bash
cargo check -p eggsec --features pdf
cargo check -p eggsec --features nse
cargo check -p eggsec --features web-proxy
cargo check -p eggsec --features mobile
```

Do not require every optional backend in one migration commit.

## Commit and sequencing guidance

Prefer separate commits or PRs for:

1. stale ignore removal;
2. PyO3 migration;
3. direct Quick-XML migration;
4. patch-level HTTP/TLS corrections;
5. remaining bounded exceptions and documentation.

Keep `Cargo.lock` reviewable. Reject unrelated mass churn caused by broad
`cargo update` commands.

## Rollback considerations

If a major dependency migration breaks public behavior, revert that migration
batch while retaining stale-ignore cleanup and the exception policy. Add a
narrow temporary exception with an explicit review date; do not restore a broad
unexplained ignore list.

## Acceptance criteria

1. Every current ignore has been revalidated against the current advisory DB.
2. Ignores for absent or already patched versions are removed.
3. Every retained ignore identifies dependency path, affected feature/artifact,
   API usage, owner, and review-by date.
4. PyO3 is upgraded to a maintained non-affected line or has a documented
   short-lived blocker with explicit review date.
5. Direct Quick-XML dependencies are upgraded beyond the applicable affected
   line.
6. Direct reqwest/Rustls patch advisories are resolved where compatible updates
   exist.
7. Transitive unresolved advisories are isolated and documented by exact path.
8. Optional features do not justify silent indefinite suppression.
9. Dependency upgrades preserve stable Python and Rust behavior.
10. MSRV changes are explicit and deferred to or coordinated with Phase H.
11. `cargo deny check advisories` reports only narrowly accepted, current
    exceptions.
12. No duplicate dependency scanner or evidence framework is added.
13. No package or release is published.
