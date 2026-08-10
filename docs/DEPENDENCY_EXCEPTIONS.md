# Dependency Advisory Exceptions

Reviewed: 2026-08-07 (Phase E)

This document tracks active advisory exceptions that cannot yet be resolved
by simple dependency upgrades. All previously stale ignores have been removed.

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
| Path | `fxhash` -> `selectors` -> `scraper` v0.21.0 |
| Feature | HTML parsing (scraper used in eggsec, eggsec-nse) |
| API used | No — fxhash is an internal hash map, not exposed |
| Exploitability | Low — unmaintained, not a vulnerability |
| Compensating control | None needed; no known security impact |
| Owner | eggsec-scanner / eggsec-nse |
| Created | 2025-07-01 |
| Review-by | 2026-11-07 |
| Blocker | scraper v0.22+ must drop fxhash dependency; no alternative available |

### RUSTSEC-2024-0384 — instant unmaintained

| Field | Value |
|-------|-------|
| Advisory | RUSTSEC-2024-0384 |
| Path | `instant` -> `notify-types` -> `notify` v7.0.0 |
| Feature | File watching (notify used in eggsec) |
| API used | No — instant is an internal timing shim, not exposed |
| Exploitability | Low — unmaintained, not a vulnerability |
| Compensating control | None needed; no known security impact |
| Owner | eggsec-cli / eggsec-tui |
| Created | 2024-06-01 |
| Review-by | 2026-11-07 |
| Blocker | notify v8+ or upstream must drop instant; no alternative available |

### RUSTSEC-2025-0119 — number_prefix unmaintained

| Field | Value |
|-------|-------|
| Advisory | RUSTSEC-2025-0119 |
| Path | `number_prefix` -> `indicatif` v0.17.11 |
| Feature | Progress bars (indicatif used in eggsec) |
| API used | No — number_prefix is an internal formatting crate, not exposed |
| Exploitability | Low — unmaintained, not a vulnerability |
| Compensating control | None needed; no known security impact |
| Owner | eggsec-cli / eggsec-tui |
| Created | 2025-07-01 |
| Review-by | 2026-11-07 |
| Blocker | indicatif v0.18+ must drop number_prefix; no alternative available |

### RUSTSEC-2025-0020 — pyo3 buffer overflow

| Field | Value |
|-------|-------|
| Advisory | RUSTSEC-2025-0020 |
| Path | `pyo3` v0.22.6 (direct dependency of eggsec-python) |
| Feature | Python bindings |
| API used | Unlikely — `PyString::from_object` not called directly |
| Exploitability | Low — requires calling `PyString::from_object` with `&str` containing NUL bytes |
| Compensating control | Eggsec Python API does not pass untrusted `&str` to this function |
| Owner | eggsec-python |
| Created | 2025-07-01 |
| Review-by | 2026-11-07 |
| Blocker | Requires pyo3 >=0.24.1 (major version bump from 0.22). Deferred to Phase H. |

### RUSTSEC-2026-0177 — pyo3 missing Sync bound

| Field | Value |
|-------|-------|
| Advisory | RUSTSEC-2026-0177 |
| Path | `pyo3` v0.22.6 (direct dependency of eggsec-python) |
| Feature | Python bindings |
| API used | Possible — `PyCFunction::new_closure` may be used |
| Exploitability | Low under GIL-protected Python; higher for free-threaded Python |
| Compensating control | Eggsec targets GIL-protected CPython; free-threaded Python not supported |
| Owner | eggsec-python |
| Created | 2026-07-01 |
| Review-by | 2026-11-07 |
| Blocker | Requires pyo3 >=0.29.0 (major version bump from 0.22). Deferred to Phase H. |

### RUSTSEC-2026-0194 — quick-xml quadratic DoS

| Field | Value |
|-------|-------|
| Advisory | RUSTSEC-2026-0194 |
| Path | `quick-xml` v0.31.0 (direct dependency of eggsec, eggsec-output, eggsec-mobile-lab) |
| Feature | XML generation (junit.rs), APK manifest parsing (apk.rs) |
| API used | Partial — `BytesStart::attributes()` with default checks used in apk.rs test path |
| Exploitability | Medium — attacker-crafted XML with many attributes on one tag could cause CPU exhaustion |
| Compensating control | APK manifests are local files, not network-untrusted; JUnit is write-only |
| Owner | eggsec-output / eggsec-mobile-lab |
| Created | 2026-07-01 |
| Review-by | 2026-11-07 |
| Blocker | Requires quick-xml >=0.41.0 which raises MSRV to 1.86 (current MSRV is 1.85). Deferred. |

### RUSTSEC-2026-0195 — quick-xml NsReader OOM

| Field | Value |
|-------|-------|
| Advisory | RUSTSEC-2026-0195 |
| Path | `quick-xml` v0.31.0 (direct dependency of eggsec, eggsec-output, eggsec-mobile-lab) |
| Feature | XML generation (junit.rs), APK manifest parsing (apk.rs) |
| API used | No — `NsReader` is not used; only `Reader` is used |
| Exploitability | Low — Eggsec does not use `NsReader` |
| Compensating control | N/A — not using the affected API |
| Owner | eggsec-output / eggsec-mobile-lab |
| Created | 2026-07-01 |
| Review-by | 2026-11-07 |
| Blocker | Requires quick-xml >=0.41.0 which raises MSRV to 1.86 (current MSRV is 1.85). Deferred. |

## Resolved in Phase E

The following advisories were resolved by prior dependency upgrades and their
stale ignores were removed:

- RUSTSEC-2026-0097 (rand unsound) — upgraded rand 0.8.5 -> 0.8.6, 0.9.2 -> 0.9.3
- RUSTSEC-2026-0204 (crossbeam-epoch) — already upgraded in lockfile
- RUSTSEC-2024-0421 (idna) — no longer in dependency tree
- RUSTSEC-2026-0141 (lettre) — already upgraded in lockfile
- RUSTSEC-2026-0187 (lopdf) — removed from dependency tree
- RUSTSEC-2026-0185 (reqwest) — already upgraded in lockfile
- RUSTSEC-2026-0104 (rustls-webpki) — no longer in dependency tree
- RUSTSEC-2026-0099 (rustls-webpki) — no longer in dependency tree
- RUSTSEC-2026-0098 (rustls-webpki/sqlx) — no longer in dependency tree
- RUSTSEC-2024-0363 (paste) — no longer in dependency tree
- RUSTSEC-2024-0436 (ttf-parser) — no longer in dependency tree
- RUSTSEC-2026-0192 (anyhow) — already upgraded in lockfile

## Review Schedule

All exceptions must be reviewed by 2026-11-07. Exceptions that are still
active at review time must be re-evaluated for:

1. Whether the upstream has released a fix
2. Whether the dependency path has changed
3. Whether the exploitability assessment still holds
4. Whether the review-by date should be extended (max 90 days)

## Phase H Planned Resolutions

- **PyO3**: Upgrade to 0.29+ (major migration). Will resolve RUSTSEC-2025-0020 and RUSTSEC-2026-0177.
- **quick-xml**: Upgrade to 0.41+ (major migration, raises MSRV to 1.86). Will resolve RUSTSEC-2026-0194 and RUSTSEC-2026-0195.
- **scraper/notify/indicatif**: Monitor upstream for unmaintained dependency removal.
