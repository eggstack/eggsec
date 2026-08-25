# Storage Module

SQLx-based PostgreSQL persistence for scan results, findings, and metadata. Feature-gated behind the `database` flag.

See also: [overview.md](overview.md), [findings.md](findings.md), [output.md](output.md), [tui.md](tui.md).

## Role & Responsibilities

- Provide CRUD operations for scans and findings against a PostgreSQL database
- Serialize `Finding` objects as JSONB columns for flexible querying
- Maintain scan lifecycle state (Running → Completed/Failed/Cancelled)
- Provide predefined SQL queries via `QueryBuilder` for common access patterns
- Offer a stub/error path when the `database` feature is disabled

## Location & Feature Gating

| Item | Location | Feature |
|------|----------|---------|
| Module declaration | `lib.rs:133-137` | `database` |
| Public module | `lib.rs:133` (`pub mod storage`) | `database` |
| Stub module | `lib.rs:135` (`mod storage`) | `not(database)` |
| Feature flag | `Cargo.toml` `database` | Depends on: `sqlx`, `sqlx/postgres`, `serde_json`, `uuid`, `chrono` |

When `database` is disabled, `init_storage()` returns `Err(Config("database feature not enabled"))` (`storage/mod.rs:61-64`), and all `Database` methods degrade to no-op stubs returning empty results or `Ok(())`.

## Files

| File | Lines | Purpose |
|------|-------|---------|
| `storage/mod.rs` | 65 | `StorageConfig` struct, `init_storage()` factory, feature-gated stub |
| `storage/models.rs` | 66 | `StoredScan`, `ScanStatus` enum (4 variants), re-export of `StoredFinding`/`FindingStatus`/`StatusChange` from `findings::lifecycle` |
| `storage/postgres.rs` | 450 | `Database` struct wrapping `PgPool`, all CRUD methods, `row_to_stored_finding()`, `parse_scan_status()` |
| `storage/queries.rs` | 64 | `QueryBuilder` with 5 static predefined SQL queries |

## Architecture

### StorageConfig (`storage/mod.rs:20-27`)

| Field | Type | Default |
|-------|------|---------|
| `host` | `String` | `"localhost"` |
| `port` | `u16` | `5432` |
| `database` | `String` | `"eggsec"` |
| `username` | `String` | `"postgres"` |
| `password` | `SensitiveString` | empty |
| `max_connections` | `u32` | `10` |

Implements `Debug` with password redacted (`"[REDACTED]"` at `mod.rs:49`). Derives `Clone`, `Serialize`, `Deserialize`.

### Database (`storage/postgres.rs:16-19`)

The `Database` struct conditionally holds a `PgPool`:

```rust
pub struct Database {
    #[cfg(feature = "database")]
    pool: PgPool,
}
```

Connection setup (`Database::new()` at `postgres.rs:22-52`):
1. Builds `PgConnectOptions` from discrete fields (host, port, username, password, database) — **not** interpolated into a URL string (to prevent credential leakage in logs, `postgres.rs:27`)
2. Creates pool via `PgPoolOptions::new().max_connections(config.max_connections).connect_with(options)`
3. Returns `Err(Config)` on connection failure

`pool_ref()` (`postgres.rs:55`) exposes the raw `PgPool` for direct SQLx queries (used by CLI handler).

### ScanStatus Enum (`storage/models.rs:18-24`)

4 variants: `Running`, `Completed`, `Failed`, `Cancelled`

Implements `Display` (capitalized strings), `PartialEq`, `Eq`. Unknown values from DB default to `Running` with a `tracing::warn!` (`postgres.rs:323`).

### Schema (Migrations)

| Migration | File | Purpose |
|-----------|------|---------|
| `001_create_scans.sql` | `migrations/001_create_scans.sql` | `scans` table: `id TEXT PK`, `target TEXT`, `scan_type TEXT`, `started_at TIMESTAMPTZ`, `completed_at TIMESTAMPTZ?`, `status TEXT DEFAULT 'Running'`, `findings_count INTEGER DEFAULT 0` |
| `002_create_findings.sql` | `migrations/002_create_findings.sql` | `findings` table: `id TEXT PK`, `scan_id TEXT FK→scans`, `finding JSONB`, `status TEXT DEFAULT 'new'`, `created_at TIMESTAMPTZ`, `updated_at TIMESTAMPTZ`, `status_history JSONB DEFAULT '[]'`. Enables `pg_trgm` extension. Creates indexes on `scan_id` and `status` |
| `003_create_users.sql` | `migrations/003_create_users.sql` | `users` table: `id TEXT PK`, `username TEXT UNIQUE`, `email TEXT UNIQUE`, `role TEXT DEFAULT 'Viewer'` |

Migrations are applied via `handle_storage_init` (`commands/handlers/storage.rs:186-239`) using `include_str!()` to embed the SQL files. No versioned migration runner — init runs all three with `CREATE TABLE IF NOT EXISTS`.

## Behavior / Flow

### Connection / Pool Setup

1. Caller provides `StorageConfig` (defaults cover local dev: `localhost:5432/eggsec`)
2. `init_storage()` → `Database::new()` → `PgConnectOptions` + `PgPoolOptions`
3. Pool acquired with configurable `max_connections` (default 10)
4. On failure: `EggsecError::Config("Failed to connect to database: ...")`

### CRUD Flows

| Method | SQL Pattern | Feature Gate | Lines |
|--------|-------------|:---:|-------|
| `insert_scan()` | `INSERT ... ON CONFLICT DO UPDATE` (upsert) | `database` | `postgres.rs:59-85` |
| `get_scan()` | `SELECT * FROM scans WHERE id = $1` | `database` | `postgres.rs:87-111` |
| `list_scans()` | `SELECT * FROM scans ORDER BY started_at DESC LIMIT $1` | `database` | `postgres.rs:113-140` |
| `insert_finding()` | `INSERT ... ON CONFLICT DO UPDATE` (upsert) — finding as JSONB, status_history as JSONB | `database` | `postgres.rs:142-176` |
| `get_finding()` | `SELECT * FROM findings WHERE id = $1` | `database` | `postgres.rs:178-194` |
| `update_finding_status()` | `UPDATE findings SET status = $1, updated_at = NOW() WHERE id = $2` | `database` | `postgres.rs:196-212` |
| `list_findings()` | `SELECT * FROM findings WHERE scan_id = $1 ORDER BY created_at DESC OFFSET $2 LIMIT $3` | `database` | `postgres.rs:214-241` |
| `list_all_findings()` | `SELECT * FROM findings ORDER BY created_at DESC OFFSET $1 LIMIT $2` | `database` | `postgres.rs:243-269` |
| `get_findings_by_severity()` | `SELECT * FROM findings WHERE finding->>'severity' = $1 ORDER BY created_at DESC` | `database` | `postgres.rs:271-293` |
| `update_scan_findings_count()` | `UPDATE scans SET findings_count = (SELECT COUNT(*)::int FROM findings WHERE scan_id = $1) WHERE id = $1` | `database` | `postgres.rs:295-312` |

All non-feature stubs return `Ok(())`, `Ok(None)`, or `Ok(vec![])`.

### Finding Persistence Format

Findings are stored with:
- `finding`: JSONB column containing the full `Finding` struct (17 fields) serialized via `serde_json::to_value()` (`postgres.rs:148`)
- `status`: Text column using `FindingStatus::Display` (`"new"`, `"confirmed"`, `"accepted_risk"`, `"false_positive"`, `"remediated"`, `"reopened"`)
- `status_history`: JSONB column containing `Vec<StatusChange>` serialized via `serde_json::to_value()` (`postgres.rs:150`)
- Deserialization on read: `row_to_stored_finding()` (`postgres.rs:330-384`) deserializes JSON fields and parses status strings; unknown statuses default to `FindingStatus::New` with a `tracing::warn!`

### Predefined Queries (`queries.rs`)

| Query | Method | Purpose | Line |
|-------|--------|---------|------|
| `find_open_findings_by_severity()` | SQL string | Find `new` findings by severity (pg_trgm) | `queries.rs:4` |
| `find_recent_scans()` | SQL string | Recent scans with limit | `queries.rs:8` |
| `find_findings_by_cve()` | SQL string | Findings by CVE ID (JSONB path) | `queries.rs:12` |
| `count_findings_by_status()` | SQL string | Aggregate count by status | `queries.rs:16` |
| `find_duplicate_findings()` | SQL string | Fuzzy duplicate detection via `pg_trgm` similarity | `queries.rs:20` |

Note: `QueryBuilder` methods return `&'static str` — these are raw SQL strings, not executed by `Database`. They are available for callers to use with `sqlx::query()`.

## Data Model

### StoredScan (`models.rs:8-16`)

| Field | Type | Notes |
|-------|------|-------|
| `id` | `String` | UUID v4, generated by `StoredScan::new()` |
| `target` | `String` | Target URL/IP |
| `scan_type` | `String` | Scan type identifier |
| `started_at` | `DateTime<Utc>` | Set on creation |
| `completed_at` | `Option<DateTime<Utc>>` | Set by `complete()` |
| `status` | `ScanStatus` | Running/Completed/Failed/Cancelled |
| `findings_count` | `usize` | DB column is `INTEGER`; cast via `as i64` / `as usize` |

### ScanStatus Enum (`models.rs:18-24`)

4 variants: `Running`, `Completed`, `Failed`, `Cancelled`

### StoredFinding (re-exported from `findings::lifecycle`)

Re-exported at `models.rs:5`. The canonical type lives in `findings/lifecycle.rs:49-57`:

| Field | Type | Notes |
|-------|------|-------|
| `finding` | `Finding` | Full canonical `Finding` struct (17 fields) |
| `scan_id` | `String` | FK to `scans.id` |
| `status` | `FindingStatus` | 6-variant lifecycle status |
| `created_at` | `DateTime<Utc>` | |
| `updated_at` | `DateTime<Utc>` | |
| `status_history` | `Vec<StatusChange>` | Transition audit trail |

### FindingStatus Enum (`findings/lifecycle.rs:6-13`)

6 variants: `New`, `Confirmed`, `AcceptedRisk`, `FalsePositive`, `Remediated`, `Reopened`

Transitions validated by `valid_transitions()` (`lifecycle.rs:17-27`). Each `StoredFinding::change_status()` records a `StatusChange` with timestamp and optional note.

## Public API

| Function/Method | Signature | Feature Gate |
|-----------------|-----------|:---:|
| `init_storage()` | `async fn init_storage(config: &StorageConfig) -> Result<Database>` | always (stub when off) |
| `Database::new()` | `async fn new(config: &StorageConfig) -> Result<Self>` | always |
| `Database::pool_ref()` | `fn pool_ref(&self) -> &PgPool` | `database` |
| `Database::insert_scan()` | `async fn insert_scan(&self, scan: &StoredScan) -> Result<()>` | always |
| `Database::get_scan()` | `async fn get_scan(&self, id: &str) -> Result<Option<StoredScan>>` | always |
| `Database::list_scans()` | `async fn list_scans(&self, limit: usize) -> Result<Vec<StoredScan>>` | always |
| `Database::insert_finding()` | `async fn insert_finding(&self, stored: &StoredFinding) -> Result<()>` | always |
| `Database::get_finding()` | `async fn get_finding(&self, id: &str) -> Result<Option<StoredFinding>>` | always |
| `Database::update_finding_status()` | `async fn update_finding_status(&self, id: &str, status: FindingStatus) -> Result<()>` | always |
| `Database::list_findings()` | `async fn list_findings(&self, scan_id: &str, offset: usize, limit: usize) -> Result<Vec<StoredFinding>>` | always |
| `Database::list_all_findings()` | `async fn list_all_findings(&self, offset: usize, limit: usize) -> Result<Vec<StoredFinding>>` | always |
| `Database::get_findings_by_severity()` | `async fn get_findings_by_severity(&self, severity: Severity) -> Result<Vec<StoredFinding>>` | always |
| `Database::update_scan_findings_count()` | `async fn update_scan_findings_count(&self, scan_id: &str) -> Result<()>` | always |

All methods have feature-gated dual implementations — full SQLx body behind `#[cfg(feature = "database")]`, no-op stubs behind `#[cfg(not(feature = "database"))]`.

## Integration Points

### CLI

- `handle_storage()` (`commands/handlers/storage.rs:5-12`) dispatches `StorageCommand::{Query,Export,Stats,Init}`
- `handle_storage_query()` (`:14-73`) supports `--sql` for raw queries and named query types (`recent_scans`, `all_findings`)
- `handle_storage_init()` (`:186-239`) runs the 3 migrations via `include_str!()` with optional `--force` to drop tables first
- `handle_storage_stats()` (`:125-184`) shows scan counts by status
- `handle_storage_export()` (`:75-123`) exports findings to JSON files

### Dispatch

- `TaskKind::Storage(StorageParams)` → `dispatch/security.rs` `run_storage_task()` (`:224-318`)
- `TaskResult::Storage`, `TaskResult::StorageListScans`, `TaskResult::StorageListFindings` (`dispatch/types.rs:120-129`)
- Modes: `connect`, `list_scans`, `list_findings`, `search_cve`

### TUI

- Storage tab: feature-gated `database` (`tui.md` tab #24, `stable_id: "storage"`, operation: `"storage"`)
- Builds `RunRequest` via `TaskBuilder`, produces `TaskKind::Storage`

### Findings Store Relationship

- `StoredFinding` wraps the canonical `Finding` struct from `findings/mod.rs`
- `finding` field stored as JSONB, enabling both structured and full-text queries
- `StatusChange` records provide a complete audit trail of lifecycle transitions
- The findings store (`findings/store.rs` JSONL) provides file-based persistence independent of the database backend

## Testing

- `models.rs:56-66`: `test_scan_creation` — verifies initial state
- `postgres.rs:386-449`: Feature-gated database tests:
  - `test_scan_status_parse_roundtrip` — all 4 `ScanStatus` variants roundtrip through `Display`/`parse_scan_status`
  - `test_scan_status_parse_unknown` — unknown strings default to `Running`
  - `test_scan_status_parse_case_insensitive` — mixed-case parsing
- `postgres.rs:390-398`: `test_storage_config_defaults` — verifies default values
- `queries.rs:28-63`: 5 tests verifying query string content

## Invariants & Gotchas

1. **No pool acquisition timeout**: `PgPoolOptions` does not set `.acquire_timeout()` — uses sqlx default (30s). For high-concurrency deployments, this may need configuration.
2. **Upsert on conflict**: Both `insert_scan` and `insert_finding` use `ON CONFLICT DO UPDATE`, meaning re-inserting with the same ID silently overwrites. This is by design for idempotent writes.
3. **`findings_count` cast**: `StoredScan.findings_count` is `usize` in Rust but `INTEGER` in PostgreSQL. The cast uses `as i64` / `as usize` (`postgres.rs:79, 103, 131`). Large counts (> i64::MAX) would wrap, but this is practically impossible for finding counts.
4. **JSONB deserialization failure**: If `finding` JSONB becomes corrupted, `row_to_stored_finding()` returns an error (`postgres.rs:338-344`). Status history deserialization failure logs a warning and defaults to empty vec (`postgres.rs:364-371`) — non-fatal but lossy.
5. **QueryBuilder is unused**: `queries.rs` defines 5 queries but `Database` does not call them. They are available for external callers but are dead code within the module itself.
6. **Users table**: Migration 003 creates a `users` table but no Rust code references it — it exists for future use.
7. **No connection pool lifecycle**: No `close()` or shutdown method on `Database`. The pool drops when `Database` is dropped.
8. **Stale `crate::error::EggsecError::Config` usage**: All database errors are wrapped as `Config` variants (`postgres.rs:40, 82, 94, 120, 151, 173, 208, 230, 257, 281, 308`). This conflates connection failures with query failures, making error handling less precise for callers.

## Bug Sweep

| Finding | File:Line | Severity | Description |
|---------|-----------|----------|-------------|
| Raw SQL in CLI handler | `commands/handlers/storage.rs:24` | Medium | `sqlx::query(sql)` where `sql` is user-provided (`--sql` arg) — **SQL injection vector**. The handler passes user-supplied SQL directly to `sqlx::query()` with no parameterization or sanitization. |
| `unwrap_or_default` on history | `postgres.rs:364` | Low | `serde_json::from_value(history_json).unwrap_or_else(...)` — defaults to empty vec on deserialization failure, losing history. Logged but silent to caller. |
| No pool timeout config | `postgres.rs:35-37` | Low | `PgPoolOptions::new()` uses default `acquire_timeout` (30s). Under load, connections may queue without explicit timeout. |
| Missing connection validation | `postgres.rs:22-52` | Low | `Database::new()` connects but does not run `SELECT 1` or similar to validate the connection is live. Pool creation may succeed with a lazy connection that fails on first use. |

*Last verified against source: 2026-08-25*
