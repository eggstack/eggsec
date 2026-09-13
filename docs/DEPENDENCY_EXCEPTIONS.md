# Dependency Advisory Exceptions

Reviewed: 2026-09-13 (Phase F supply-chain hardening)

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
| Owner | eggsec (recon) / eggsec-nse |
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

### RUSTSEC-2025-0119 — number_prefix unmaintained

| Field | Value |
|-------|-------|
| Advisory | RUSTSEC-2025-0119 |
| Path | `number_prefix` 0.4.0 -> `indicatif` v0.17.11 |
| Feature | Progress bars (indicatif used in eggsec pipeline/scanner/loadtest) |
| API used | No — number_prefix is an internal formatting crate, not exposed |
| Exploitability | Low — unmaintained, not a vulnerability |
| Compensating control | None needed; no known security impact |
| Owner | eggsec-cli / eggsec-tui |
| Created | 2025-07-01 |
| Review-by | 2026-12-12 |
| Blocker | indicatif v0.18+ must drop number_prefix; no alternative available |

### RUSTSEC-2024-0436 — paste unmaintained

| Field | Value |
|-------|-------|
| Advisory | RUSTSEC-2024-0436 |
| Path | `paste` 1.0.15 -> `sqlx-macros-core` -> `sqlx` 0.8.0 |
| Feature | `db-pentest` (optional; `eggsec-db-lab` Postgres/MySQL probes) |
| API used | No at runtime — paste is a build-time proc-macro used by sqlx macros only; no runtime code from paste ships |
| Exploitability | Low — unmaintained, not a vulnerability; build-time only |
| Compensating control | None needed; alternative is `pastey` fork, requires sqlx upstream to migrate |
| Owner | eggsec-db-lab |
| Created | 2026-09-13 |
| Review-by | 2026-12-12 |
| Blocker | sqlx upstream must migrate paste -> pastey; no Eggsec-side fix available |

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
| Blocker | Fixed in rustls-webpki >= 0.103.12, but tiberius 0.12.3 (latest) pins rustls 0.21 / webpki 0.101.7; requires tiberius upstream to upgrade its rustls stack |

### RUSTSEC-2024-0363 — sqlx Binary Protocol misinterpretation

| Field | Value |
|-------|-------|
| Advisory | RUSTSEC-2024-0363 |
| Path | `sqlx` 0.8.0 (direct optional dependency of `eggsec` and `eggsec-db-lab`) |
| Feature | `db-pentest` (optional; Postgres/MySQL probes) |
| API used | Yes — `sqlx::postgres::PgPool` / `sqlx::mysql::MySqlPool` queries against lab databases |
| Exploitability | Low for Eggsec — truncating/overflowing casts misinterpret a malicious *server's* binary protocol responses. Servers are operator-authorized lab targets under scope enforcement; the operator already trusts the lab DB to execute test queries. No untrusted server can inject itself into this path |
| Compensating control | Scope-enforced, operator-directed lab connections; no production data flows through this path |
| Owner | eggsec-db-lab |
| Created | 2026-09-13 |
| Review-by | 2026-12-12 |
| Blocker | Fixed in sqlx >= 0.8.1, but upgrading sqlx pulls `libsqlite3-sys` 0.30.1 which conflicts (`links = "sqlite3"`) with `rusqlite` 0.31.0 (`libsqlite3-sys` 0.28.0) required by `eggsec-daemon` session persistence. Resolving requires a coordinated rusqlite/sqlx sqlite-stack upgrade; tracked separately |

## License exception

| Crate | License | Scope | Owner | Review-by | Rationale |
|-------|---------|-------|-------|-----------|-----------|
| `auto_generate_cdp` 0.4.6 | GPL-3.0-or-later (from LICENSE.txt; no Cargo license field) | Build-time only, `headless-browser` feature via `headless_chrome` codegen | eggsec (browser) | 2026-12-12 | Runs at build time to generate Chrome DevTools Protocol bindings; not linked into shipped binaries. Lab-only DOM XSS/SPA crawling feature. Blocker: headless_chrome upstream must replace or relicense the codegen tool. |
| `borrow-or-share` 0.2.4 | MIT-0 | Allowed globally | — | — | MIT No Attribution is OSI-approved permissive, equivalent in spirit to the already-allowed `0BSD`. Added to the global allow list, not an exception. |

## Yanked crate notice (warning, not exception)

`libssh2-sys` 0.3.2 is reported yanked (via `ssh2` 0.9.6, `nse-ssh2` feature).
Deny treats yanked as `warn` (does not fail the gate); `cargo audit` reports it
as an allowed warning. No ignore is recorded because yanked status calls for an
upgrade (`cargo update -p libssh2-sys`), not a suppression. Re-evaluate at each
review: if upstream never un-yanks, consider pinning or replacing the ssh2 path.

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
- RUSTSEC-2024-0363 (sqlx) — present via optional `sqlx` 0.8.0 (`db-pentest`)
- RUSTSEC-2024-0436 (paste) — present via `sqlx-macros` (`db-pentest`)
- RUSTSEC-2026-0192 (ttf-parser) — present via `printpdf` (`pdf` feature)

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
