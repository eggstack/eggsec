# Dependency Advisory Exceptions

Reviewed: 2026-10-24 (milestone 015 — dependency advisory backlog)

This document tracks active advisory exceptions that cannot yet be resolved
by simple dependency upgrades.

## Canonical policy source

**Cargo Deny (`deny.toml`) is the canonical repository dependency-policy
source.** It runs on pull requests via the `dependency-policy` job in
`.github/workflows/ci.yml` and locally via `make check-deps`
(`cargo deny --workspace --all-features check`, with `[graph] all-features =
true` in `deny.toml` so the default `cargo deny check` invocation also covers
optional-feature closures).

`cargo audit` is **diagnostic/non-canonical**. `.cargo/audit.toml` was removed
in Phase F because it suppressed a larger historical advisory set than
`deny.toml` (24 stale ignores, most for advisories already fixed) with no
owner/review metadata. Do not reintroduce it. Historical ignores are never
retained "just in case" — each accepted exception below has an advisory ID,
path, feature, API-use assessment, exploitability, compensating control,
owner, created/review-by dates, and an upstream upgrade blocker.

Source policy (`deny.toml [sources]`) fails closed: `unknown-registry =
"deny"`, `unknown-git = "deny"`, `required-git-spec = "rev"`. No git or
alternate-registry dependencies exist in `Cargo.lock` (verified 2026-09-13);
any new git dep must be pinned to a full commit `rev` and documented here with
owner/removal criteria before the gate can pass. Wildcard requirements are
denied (`wildcards = "deny"`); all workspace path dependencies carry explicit
`version = "0.1.0"` pins so the lint only fires on real `version = "*"`
declarations. `multiple-versions = "warn"` is retained — duplicate-version
noise must not obscure security failures.

## Exception Policy

Every retained advisory ignore must include:

- Advisory ID and description
- Dependency path (direct or transitive)
- Affected feature/artifact
- Whether the affected API is used
- Exploitability assessment for Eggsec
- Compensating control, if any
- Owner or owning subsystem
- Created/reviewed date
- Mandatory review-by date (no more than 90 days)
- Upgrade/removal blocker

## Active Exceptions

### RUSTSEC-2025-0057 — fxhash unmaintained

| Field | Value |
|-------|-------|
| Advisory | RUSTSEC-2025-0057 |
| Path | `fxhash` 0.2.1 -> `selectors` -> `scraper` (used in `eggsec::recon::js`, `eggsec-nse::httpspider`) |
| Feature | HTML parsing (optional, always compiled via scraper) |
| API used | No — fxhash is an internal hash map inside selectors, not exposed |
| Exploitability | Low — unmaintained, not a vulnerability |
| Compensating control | None needed; no known security impact |
| Owner | eggsec (recon) / standalone eggstack/eggsec-nse runtime |
| Created | 2025-07-01 |
| Review-by | 2026-12-12 |
| Blocker | scraper v0.22+ must drop fxhash dependency; no alternative available |

### RUSTSEC-2024-0384 — instant unmaintained

| Field | Value |
|-------|-------|
| Advisory | RUSTSEC-2024-0384 |
| Path | `instant` 0.1.13 -> `notify-types` -> `notify` v7.0.0 |
| Feature | File watching (notify used in eggsec config-watch) |
| API used | No — instant is an internal timing shim, not exposed |
| Exploitability | Low — unmaintained, not a vulnerability |
| Compensating control | None needed; no known security impact |
| Owner | eggsec-cli / eggsec-tui |
| Created | 2024-06-01 |
| Review-by | 2026-12-12 |
| Blocker | notify v8+ or upstream must drop instant; no alternative available |

### RUSTSEC-2025-0134 — rustls-pemfile unmaintained

| Field | Value |
|-------|-------|
| Advisory | RUSTSEC-2025-0134 |
| Path | `rustls-pemfile` 1.0.4 -> `rustls-native-certs` -> `tiberius` 0.12.3 |
| Feature | `db-pentest-mssql-tiberius` marker (optional MSSQL via tiberius) |
| API used | Transitively — PEM parsing of lab MSSQL server TLS chains via rustls-native-certs; Eggsec never calls rustls-pemfile directly |
| Exploitability | Low — unmaintained wrapper around the same code now in `rustls-pki-types` (>=1.9 `PemObject`); no known vulnerability, migration is mechanical upstream |
| Compensating control | Lab-only connections to operator-authorized MSSQL targets; scope enforcement unchanged |
| Owner | eggsec-db-lab |
| Created | 2026-09-13 |
| Review-by | 2026-12-12 |
| Blocker | tiberius 0.12.3 is latest and still depends on rustls-pemfile; requires tiberius upstream to migrate to `rustls-pki-types::pem` |

### RUSTSEC-2026-0192 — ttf-parser unmaintained

| Field | Value |
|-------|-------|
| Advisory | RUSTSEC-2026-0192 |
| Path | `ttf-parser` 0.19.2 -> `owned_ttf_parser` -> `printpdf` 0.7.0 |
| Feature | `pdf` (optional; `eggsec::output::pdf` report generation) |
| API used | No — Eggsec generates PDFs with `printpdf::BuiltinFont::Helvetica` only; it never parses untrusted font files |
| Exploitability | Low — unmaintained, not a vulnerability; suggested alternative is `skrifa`, requires printpdf upstream to migrate |
| Compensating control | Generation-only usage; no untrusted font input reaches ttf-parser |
| Owner | eggsec (output) |
| Created | 2026-09-13 |
| Review-by | 2026-12-12 |
| Blocker | printpdf must migrate ttf-parser -> skrifa, or Eggsec must take the printpdf 0.7 -> 0.12 breaking major bump (see lopdf entry) |

### RUSTSEC-2026-0187 — lopdf stack overflow (nested PDF objects)

| Field | Value |
|-------|-------|
| Advisory | RUSTSEC-2026-0187 (severity 7.5 high) |
| Path | `lopdf` 0.31.0 -> `printpdf` 0.7.0 |
| Feature | `pdf` (optional; `eggsec::output::pdf` report generation) |
| API used | No — the vulnerable entry points are `lopdf::Document::load_mem`/`load*` (parsing untrusted PDFs with unbounded recursion, ~10k nested arrays abort the process). Eggsec only *generates* PDFs from its own findings; it never parses untrusted PDFs |
| Exploitability | Low for Eggsec — no untrusted PDF input reaches lopdf; a crafted PDF cannot be delivered to the vulnerable parser through any Eggsec surface |
| Compensating control | Generation-only usage; no `load*` calls in `crates/eggsec/src/output/pdf.rs` (verified: only `PdfDocument::new`, `add_builtin_font`, `use_text`) |
| Owner | eggsec (output) |
| Created | 2026-09-13 |
| Review-by | 2026-12-12 |
| Blocker | Fixed in lopdf >= 0.42.0, which requires printpdf 0.7 -> >= 0.8 (breaking major bump, current latest 0.12.8). Tracked for a dedicated printpdf upgrade pass; Dependabot will surface printpdf updates |

### RUSTSEC-2023-0071 — rsa Marvin timing sidechannel

| Field | Value |
|-------|-------|
| Advisory | RUSTSEC-2023-0071 (severity 5.9 medium) |
| Path | `rsa` 0.9.10 -> `sqlx-mysql` -> `sqlx` 0.8.0 |
| Feature | `db-pentest` (optional; MySQL probes in `eggsec-db-lab`) |
| API used | Transitively — RSA decryption inside sqlx-mysql authentication; Eggsec never calls rsa directly |
| Exploitability | Low for Eggsec — Marvin requires network timing observation of RSA decryption. Connections are operator-initiated to explicitly authorized lab targets (`EnforcementContext` scope gate); the attacker model for lab pentesting does not expose decryption timing to untrusted observers |
| Compensating control | Scope-enforced, operator-directed lab connections only; no long-lived RSA-decryption oracle exposed to untrusted networks |
| Owner | eggsec-db-lab |
| Created | 2026-09-13 |
| Review-by | 2026-12-12 |
| Blocker | No fixed upgrade available per advisory (upstream migrating to constant-time implementation); requires sqlx-mysql/rsa upstream fix |

### RUSTSEC-2026-0104 / RUSTSEC-2026-0099 / RUSTSEC-2026-0098 — rustls-webpki 0.101.7

| Field | Value |
|-------|-------|
| Advisory | RUSTSEC-2026-0104 (CRL panic), RUSTSEC-2026-0099 (wildcard name-constraint bypass), RUSTSEC-2026-0098 (URI name-constraint bypass) |
| Path | `rustls-webpki` 0.101.7 -> `rustls` 0.21.12 -> `tokio-rustls` 0.24.1 -> `tiberius` 0.12.3 |
| Feature | `db-pentest-mssql-tiberius` marker (optional MSSQL via tiberius) |
| API used | Transitively — TLS certificate verification for lab MSSQL connections; Eggsec never calls webpki directly |
| Exploitability | Low for Eggsec — all three require misissued certificates plus name-constraint/CRL configurations, reachable only after signature verification. Connections are operator-initiated to explicitly authorized lab MSSQL targets; the lab threat model does not include a misissuing CA constraining Eggsec's own lab servers |
| Compensating control | Scope-enforced lab connections; TLS verification left enabled (no `danger_accept_invalid_certs` bypass in this path) |
| Owner | eggsec-db-lab |
| Created | 2026-09-13 |
| Review-by | 2026-12-12 |
| Blocker | **Corrected 2026-10-06.** The previous note — "tiberius 0.12.3 is latest; requires tiberius upstream to upgrade its rustls stack" — is no longer accurate: upstream has since released **tiberius 0.13.0**, which does move to `tokio-rustls` 0.26 / rustls 0.23 / rustls-webpki 0.103. It was trialled and **reverted**: 0.13.0 depends on `tokio-rustls ^0.26.4` with default features, which enables rustls' `aws-lc-rs` crypto provider, and `[bans] deny = ["aws-lc-rs"]` forbids it under the repo's ring-only TLS policy. Feature unification would apply that provider to the whole build, not just MSSQL. So this is now blocked by **policy**, not by upstream. Clearing it requires either tiberius exposing a ring-only path or a maintainer decision to relax the ring-only rule |

### RUSTSEC-2024-0429 — glib 0.18.5 unsound

| Field | Value |
|-------|-------|
| Advisory | RUSTSEC-2024-0429 (unsound), RUSTSEC-2024-0370 (unmaintained `proc-macro-error` 1.0.4) |
| Path | `glib` 0.18.5 -> `gtk` 0.18.2 -> `tauri` 2.11.6 (also `wry` 0.55.1 -> `gtk`, `tao` 0.35.3 -> `gtk`); `proc-macro-error` 1.0.4 -> `gtk3-macros` 0.18.2 and `glib-macros` 0.18.5 |
| Feature | `desktop-gui` (Tauri shell closure in `eggsec-gui`; the gtk-rs stack is Linux-only) |
| API used | Transitively — GTK/glib bindings behind the Tauri shell. Eggsec never calls glib or proc-macro-error directly; `proc-macro-error` is a build-time macro dependency only |
| Exploitability | Low for Eggsec — RUSTSEC-2024-0429 is an unsoundness advisory in glib's RefCell/borrowing internals reachable only through GTK's own C boundary, and the GUI surface is a local operator-driven desktop shell that links no engine, daemon, CLI or TUI crate. RUSTSEC-2024-0370 is `unmaintained`, not a vulnerability |
| Compensating control | The affected crate is Linux-only and reached only through the Tauri shell closure; the GUI is daemon-only and makes no outbound HTTP of its own, so no untrusted input reaches the GTK boundary. RUSTSEC-2024-0370 is a build-time-only dependency and is not linked into the shipped binary |
| Owner | eggsec-gui |
| Created | 2026-10-24 |
| Review-by | 2026-12-12 |
| Blocker | gtk-rs 0.20 is unreachable: no published `muda`/`tao`/`wry` version uses gtk-rs 0.20+ (measured across 66 `muda`, 125 `tao`, and 30 `wry` published versions — only `gtk`/`glib` requirements `^0.9`, `^0.10`, `^0.14`, `^0.15`, `^0.16`, `^0.18` exist), and `tauri` 2.12.5 still declares `gtk ^0.18`. Clearing requires the upstream Tauri closure to move to gtk-rs 0.20+, which is **not** reachable by moving the Tauri line, by raising MSRV, or by any lockfile operation. `proc-macro-error` 1.0.4 is the latest published version and clears together with this entry |

### RUSTSEC-2026-0346 / RUSTSEC-2026-0349 / RUSTSEC-2026-0382 — rustls 0.21.12 / rustls-webpki 0.101.7

| Field | Value |
|-------|-------|
| Advisory | RUSTSEC-2026-0346 (rustls CRL/OCSP handling), RUSTSEC-2026-0349 (rustls 0.21.12), RUSTSEC-2026-0382 (rustls-webpki 0.101.7) |
| Path | `rustls` 0.21.12 -> `tokio-rustls` 0.24.1 -> `tiberius` 0.12.3; `rustls-webpki` 0.101.7 -> `rustls` 0.21.12 -> `tokio-rustls` 0.24.1 -> `tiberius` 0.12.3 |
| Feature | `db-pentest-mssql-tiberius` marker (optional MSSQL via tiberius, in `eggsec-db-lab`) |
| API used | Transitively — TLS certificate verification for lab MSSQL connections; Eggsec never calls rustls or rustls-webpki directly on this path |
| Exploitability | Low for Eggsec — the affected rustls 0.21-line code is reachable only behind operator-initiated, scope-enforced MSSQL connections to explicitly authorized lab targets; the lab threat model does not include a misissuing CA constraining Eggsec's own lab servers |
| Compensating control | Scope-enforced lab connections; TLS verification left enabled (no `danger_accept_invalid_certs` bypass in this path). The ring-only provider rule (`[bans] deny = ["aws-lc-rs"]`) is unchanged |
| Owner | eggsec-db-lab |
| Created | 2026-10-24 |
| Review-by | 2026-12-12 |
| Blocker | `tiberius` 0.13.0 moves to `tokio-rustls` 0.26 / rustls 0.23 / rustls-webpki 0.103, but it depends on `tokio-rustls` with default features, which enables the `aws-lc-rs` crypto provider that `[bans] deny = ["aws-lc-rs"]` forbids under the ring-only TLS policy. Blocked by **policy**, not by upstream: clearing requires either `tiberius` exposing a ring-only path or a maintainer decision to relax the ring-only rule (which would need an ADR). The `rustls` 0.23 line used by `reqwest` is unaffected — that instance cleared by the 0.23.46 lockfile refresh |

### RUSTSEC-2024-0449 — unsafe-libyaml 0.2.11 unmaintained

| Field | Value |
|-------|-------|
| Advisory | RUSTSEC-2024-0449 (unmaintained) |
| Path | `unsafe-libyaml` 0.2.11 -> `serde_yaml_neo` 0.11.0 (in `eggsec` config/policy/skills/templates/api-schema and `eggsec-web-proxy` config/rules) |
| Feature | Always compiled (`serde_yaml_neo` is a non-optional dependency of the engine and the web proxy) |
| API used | Transitively — YAML parsing of engine configuration (`crates/eggsec/src/config/loader.rs`), policy resolution (`crates/eggsec/src/policy_bridge/resolver.rs`), agent skill front-matter (`crates/eggsec/src/agent/skills.rs`), scanner and marketplace template loading (`crates/eggsec/src/scanner/templates/`), API-schema fuzzing (`crates/eggsec/src/fuzzer/api_schema/`, `crates/eggsec/src/api_schema/`), auth-context loading, OpenAPI export, and the web proxy's config and intercept rules |
| Exploitability | Moderate for Eggsec — the parsed content is **partly operator-supplied and partly downloaded**: the scanner marketplace loader (`crates/eggsec/src/scanner/templates/marketplace.rs`) parses downloaded template content. The advisory kind is `unmaintained`, not a vulnerability, and `unsafe-libyaml` 0.2.11 has no fixed release; the parser is not unused and is not claimed to be unexposed |
| Compensating control | Downloaded template content is fetched over the workspace transport with its own scope/authority checks, and operator-supplied configuration is trusted input by definition; no YAML parse result is used to construct shell commands or paths without validation. The mitigating factor is the absence of a fix upstream, not a claim of non-exposure |
| Owner | eggsec (config) / eggsec-web-proxy |
| Created | 2026-10-24 |
| Review-by | 2026-12-12 |
| Blocker | `unsafe-libyaml` 0.2.11 is the latest published version and `serde_yaml_neo` 0.11.0 (also the latest) hard-depends on it; clearing requires `serde_yaml_neo` upstream to migrate to a maintained YAML backend, or a workspace-wide replacement of the YAML parser |

### RUSTSEC-2025-0075 / -0080 / -0081 / -0098 / -0100 — unic-* 0.9.0 unmaintained

| Field | Value |
|-------|-------|
| Advisory | RUSTSEC-2025-0075 (`unic-char-range`), RUSTSEC-2025-0080 (`unic-common`), RUSTSEC-2025-0081 (`unic-char-property`), RUSTSEC-2025-0098 (`unic-ucd-version`), RUSTSEC-2025-0100 (`unic-ucd-ident`) |
| Path | `unic-*` 0.9.0 -> `unic-ucd-ident` 0.9.0 -> `urlpattern` 0.3.0 -> `tauri-utils` 2.9.3 -> `tauri` 2.11.6 (in the `eggsec-gui` shell closure) |
| Feature | `desktop-gui` (Tauri shell closure in `eggsec-gui`) |
| API used | Transitively — Unicode property lookup inside `urlpattern` pattern matching behind the Tauri shell. Eggsec never calls the `unic-*` crates directly |
| Exploitability | Low for Eggsec — all five advisories are `unmaintained`, not vulnerabilities; the crates are reached only through the GUI's URL-pattern validation inside the Tauri shell closure, and the GUI is a local operator-driven daemon-only client |
| Compensating control | The `unic-*` set is reached only through the Tauri shell closure; no untrusted network input from the GUI reaches URL-pattern evaluation, and the crates are pure data tables with no unsafe code or I/O |
| Owner | eggsec-gui |
| Created | 2026-10-24 |
| Review-by | 2026-12-12 |
| Blocker | **MSRV-gated, not upgrade-less.** `urlpattern` 0.6.0 drops the whole `unic-*` set (its dependencies become `icu_properties`, `regex`, `serde`, `url`), but `tauri-utils` 2.10.0+ is the first line requiring `urlpattern ^0.6`, and `tauri-utils` 2.10.0/2.10.1/2.10.2 all declare `rust-version = "1.90"`, above this workspace's `rust-version = "1.89"` floor (`Cargo.toml:39`). **Removal recipe:** (1) raise the workspace `rust-version` to 1.90 in a separate, workspace-wide decision; (2) move the Tauri closure to the 2.12 line (`tauri` 2.12.5, `tauri-utils` 2.10.2, `tauri-macros` 2.7.4, `tauri-runtime` 2.12.2, `tauri-runtime-wry` 2.12.3, `tauri-build` 2.7.4); (3) re-run `make check-msrv`; the five `unic-*` entries then disappear because `urlpattern` resolves to 0.6.x. Note that `tauri` 2.12.5 still declares `gtk ^0.18`, so this recipe clears this entry and **not** the glib / proc-macro-error entry above |

## Resolved Exceptions

These records are retained for history only. Their `deny.toml` ignore entries were
removed when the underlying dependency left the graph, so no live ignore backs
them — architecture guard Check 190 enforces that a RESOLVED advisory ID never
reappears in `deny.toml`'s `ignore` list.

### RUSTSEC-2025-0119 — number_prefix unmaintained — **RESOLVED 2026-10-06**

| Field | Value |
|-------|-------|
| Advisory | RUSTSEC-2025-0119 |
| Path | *(was)* `number_prefix` 0.4.0 -> `indicatif` v0.17.11 |
| Feature | Progress bars (indicatif used in eggsec pipeline/scanner/loadtest) |
| API used | No — number_prefix is an internal formatting crate, not exposed |
| Exploitability | Low — unmaintained, not a vulnerability |
| Compensating control | None needed; no known security impact |
| Owner | eggsec-cli / eggsec-tui |
| Created | 2025-07-01 |
| Resolution | The documented blocker was "`indicatif` v0.18+ must drop `number_prefix`". `indicatif` has since advanced to **0.18.6**, which dropped the dependency: `number_prefix` no longer appears anywhere in `Cargo.lock`, so the advisory is no longer reachable. |
| Review-by | — (closed) |

> **Closed 2026-10-06:** the `deny.toml` ignore for `RUSTSEC-2025-0119` has now been
> removed, following the `paste` precedent below rather than being left as a dead
> entry. Verified: `cargo deny --workspace --all-features check advisories` → `advisories ok`.

### RUSTSEC-2024-0436 — paste unmaintained — **RESOLVED 2026-10-06**

| Field | Value |
|-------|-------|
| Advisory | RUSTSEC-2024-0436 |
| Path | *(was)* `paste` 1.0.15 -> `sqlx-macros-core` -> `sqlx` 0.8.0 |
| Feature | `db-pentest` (optional; `eggsec-db-lab` Postgres/MySQL probes) |
| Resolution | Upgrading `sqlx` 0.8.0 → 0.8.6 for RUSTSEC-2024-0363 also dropped `paste` from the dependency graph entirely, so this advisory is no longer reachable. The `deny.toml` ignore was removed rather than left as a dead entry |
| Owner | eggsec-db-lab |
| Created | 2026-09-13 |
| Review-by | — (closed) |

## License exception

| Crate | License | Scope | Owner | Review-by | Rationale |
|-------|---------|-------|-------|-----------|-----------|
| `auto_generate_cdp` 0.4.6 | GPL-3.0-or-later (from LICENSE.txt; no Cargo license field) | Build-time only, `headless-browser` feature via `headless_chrome` codegen | eggsec (browser) | 2026-12-12 | Runs at build time to generate Chrome DevTools Protocol bindings; not linked into shipped binaries. Lab-only DOM XSS/SPA crawling feature. Blocker: headless_chrome upstream must replace or relicense the codegen tool. |
| `borrow-or-share` 0.2.4 | MIT-0 | Allowed globally | — | — | MIT No Attribution is OSI-approved permissive, equivalent in spirit to the already-allowed `0BSD`. Added to the global allow list, not an exception. |

## Yanked crate notice (warning, not exception)

`libssh2-sys` is currently at **0.3.3** (via `ssh2` 0.9.6, `nse-ssh2` feature).
An earlier revision pinned 0.3.2, which was reported yanked; the lockfile has
since advanced past it. Deny treats yanked as `warn` (does not fail the gate) by
default, and `[advisories]` in `deny.toml` sets no stricter `yanked` value. No
ignore is recorded because yanked status calls for an upgrade
(`cargo update -p libssh2-sys`), not a suppression. Re-evaluate at each review:
if the current version is ever yanked, consider pinning or replacing the ssh2
path.

## Resolved in Corrective Closure Pass (2026-08-11)

The following advisories were resolved by dependency upgrades in this pass:

- **RUSTSEC-2025-0020** (pyo3 buffer overflow) — upgraded pyo3 0.22.6 -> 0.29.2
- **RUSTSEC-2026-0177** (pyo3 missing Sync bound) — upgraded pyo3 0.22.6 -> 0.29.2
- **RUSTSEC-2026-0194** (quick-xml quadratic DoS) — upgraded quick-xml 0.31.0 -> 0.41.0
- **RUSTSEC-2026-0195** (quick-xml NsReader OOM) — upgraded quick-xml 0.31.0 -> 0.41.0

## Phase E resolution claims superseded by Phase F all-features gate

Phase E recorded several advisories as "resolved / no longer in tree" based on
the default-feature dependency closure (`cargo deny check` without
`--all-features`). The Phase F gate (`[graph] all-features = true`,
`cargo deny --workspace --all-features check`) covers optional features
(`pdf`, `db-pentest`, `db-pentest-mssql-tiberius`, `nse-ssh2`, `sbom`,
`headless-browser`) and re-surfaced them. They are now documented as active
exceptions above with lab-only/generation-only assessments, not as resolved:

- RUSTSEC-2026-0187 (lopdf) — present via `printpdf` (`pdf` feature)
- RUSTSEC-2026-0104 / -0099 / -0098 (rustls-webpki) — present via `tiberius` (`db-pentest-mssql-tiberius` marker)
- RUSTSEC-2026-0192 (ttf-parser) — present via `printpdf` (`pdf` feature)

Two entries from the list above were re-surfaced by Phase F and have since been
**resolved (2026-10-06)** by upgrading `sqlx` 0.8.0 → 0.8.6:

- RUSTSEC-2024-0363 (sqlx) — fixed in sqlx >= 0.8.1
- RUSTSEC-2024-0436 (paste) — left the graph with the `sqlx` upgrade

The documented blocker for the sqlx upgrade was a `links = "sqlite3"` conflict:
sqlx >= 0.8.1 needs `libsqlite3-sys` ^0.30.1 while `eggsec-daemon`'s `rusqlite`
0.31 needed ^0.28. Both crates claim the same native link, so cargo refused to
resolve both. Resolved by moving `rusqlite` 0.31 → 0.32, which uses
`libsqlite3-sys` ^0.30.0 — the same line sqlx needs. `rusqlite` 0.32 is an
adjacent-minor bump over the `Connection` / `execute` / `prepare` / `query_map`
/ `params!` surface `eggsec-daemon` uses; `SqliteStore` had no test coverage, so
one was added to `crates/eggsec-daemon/src/store/sqlite.rs` to verify the
upgrade rather than assume it.

Genuinely resolved and not in any closure (verified 2026-09-13 via
`cargo deny --workspace --all-features check` passing with only the active
ignores above):

- RUSTSEC-2026-0097 (rand unsound) — upgraded rand 0.8.5 -> 0.8.6, 0.9.2 -> 0.9.3
- RUSTSEC-2026-0204 (crossbeam-epoch) — upgraded in lockfile
- RUSTSEC-2024-0421 (idna) — no longer in any closure
- RUSTSEC-2026-0141 (lettre) — upgraded in lockfile
- RUSTSEC-2026-0185 (reqwest) — upgraded in lockfile
- RUSTSEC-2025-0020 / RUSTSEC-2026-0177 (pyo3) — upgraded to 0.29.2
- RUSTSEC-2026-0194 / RUSTSEC-2026-0195 (quick-xml) — upgraded to 0.41.0
- RUSTSEC-2026-0192 (anyhow) — upgraded in lockfile (note: distinct from the ttf-parser RUSTSEC-2026-0192 collision in the old Phase E notes; the anyhow advisory ID was mis-recorded there and is not re-used)

The stale `.cargo/audit.toml` ignore IDs for these already-fixed advisories
were removed with the file in Phase F and are not restored.

## Review Schedule

All exceptions must be reviewed by 2026-12-12. Exceptions that are still
active at review time must be re-evaluated for:

1. Whether the upstream has released a fix
2. Whether the dependency path has changed
3. Whether the exploitability assessment still holds
4. Whether the review-by date should be extended (max 90 days)
# Retired Git source (Milestone 004)

`https://github.com/eggstack/eggsec-nse` was the sole allowed Git source while
Milestone 003 qualified the standalone runtime. Milestone 004 switched Eggsec
to the published `eggsec-nse` crates.io release and removed the allow-list entry
from `deny.toml`; `unknown-git = "deny"` remains fail-closed with no allowed
Git source. The consumed version has since advanced to **0.2.0**. This section
is retained as history only.
