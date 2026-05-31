# Storage Architecture Review
**Document:** architecture/storage.md
**Reviewed:** 2026-05-31
**Accuracy:** Medium
**Lines Reviewed:** 27

## Verified Claims
- Feature-gated behind `database` flag: Verified at `crates/slapper/src/storage/mod.rs:55,60`
- `StorageConfig` struct: Verified at `crates/slapper/src/storage/mod.rs:20` with fields `host`, `port`, `database`, `username`, `password`, `max_connections`
- `Database` struct: Verified at `crates/slapper/src/storage/postgres.rs:7` with field `config`
- `ScanModel` / `StoredScan` struct: Documented as `ScanModel`. Actual type name is `StoredScan` (`crates/slapper/src/storage/models.rs:6`)
- `FindingModel` / `StoredFinding` struct: Documented as `FindingModel`. Actual type name is `StoredFinding` (`crates/slapper/src/storage/models.rs:25`)
- `init_storage()` factory function: Verified at `crates/slapper/src/storage/mod.rs:56`
- PostgreSQL connection pool and CRUD operations: Partially verified. `Database::new()` exists (`postgres.rs:12`) but all CRUD methods are stub implementations returning empty results (e.g., `insert_scan` returns `Ok(())`, `get_scan` returns `Ok(None)`, `list_scans` returns `Ok(vec![])`)
- `queries.rs`: Verified at `crates/slapper/src/storage/queries.rs` with `QueryBuilder` struct and methods
- Returns config error when feature not enabled: Verified at `crates/slapper/src/storage/mod.rs:62-64`
- All files present: `mod.rs`, `postgres.rs`, `models.rs`, `queries.rs` - verified

## Discrepancies
- **Model type names**: Documented as `ScanModel` and `FindingModel`. Actual type names are `StoredScan` (`models.rs:6`) and `StoredFinding` (`models.rs:25`)
- **Database pool claim**: Documented as "PostgreSQL connection pool and operations". Actual: `Database` struct only holds a `StorageConfig` - no actual connection pool (`sqlx::PgPool` or similar) is present. All CRUD methods are stubs.
- **Queries not integrated**: `queries.rs` defines `QueryBuilder` with static methods that return raw SQL strings, but these are never used by the `Database` struct. The queries module is disconnected from the actual database operations.
- **Additional models undocumented**: `StoredUser` (`models.rs:48`), `ScanStatus` enum (`models.rs:17`), `FindingStatus` enum (`models.rs:39`), `UserRole` enum (`models.rs:56`) are not documented

## Bugs Found
- **Stub database implementation**: All `Database` methods are stubs that return empty results. This means the storage module is non-functional. `insert_scan()`, `get_scan()`, `list_scans()`, `insert_finding()`, `get_findings_for_scan()`, `update_finding_status()` all return hardcoded empty values (`crates/slapper/src/storage/postgres.rs:19-54`)

## Improvement Opportunities
- Implement actual PostgreSQL connection pool using `sqlx` as described in the module purpose
- Integrate `QueryBuilder` with `Database` methods
- Rename model types to match documentation or update documentation to match `StoredScan`/`StoredFinding`

## Stale Items
- The "SQLx-based persistence layer using PostgreSQL" claim is aspirational - the actual implementation contains only stubs with no SQLx usage
- `ScanModel`/`FindingModel` type names are stale and should be updated to `StoredScan`/`StoredFinding`
