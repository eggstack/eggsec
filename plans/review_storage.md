# Storage Module Architecture Review

**Document:** architecture/storage.md
**Reviewed:** 2026-06-02
**Accuracy:** High
**Lines Reviewed:** 27

## Verified Claims
- [StorageConfig]: Verified at `crates/slapper/src/storage/mod.rs:20`
- [Database]: Verified at `crates/slapper/src/storage/postgres.rs:8`
- [StoredScan]: Verified at `crates/slapper/src/storage/models.rs:6`
- [StoredFinding]: Verified at `crates/slapper/src/storage/models.rs:25`
- [Files: mod.rs, postgres.rs, models.rs, queries.rs]: Verified
- [init_storage() factory function]: Verified at `crates/slapper/src/storage/mod.rs:56`
- [Feature-gated behind database]: Verified at `crates/slapper/src/storage/mod.rs:55` and `mod.rs:60-64`

## Discrepancies
- None significant.

## Bugs Found
- [WARNING: Stub implementation]: The `Database` struct at `postgres.rs:6-7` explicitly states "WARNING: Stub implementation - not connected to a real database". All methods return empty results or `None` without actual database operations. This is not mentioned in the architecture document (priority: high - documentation is misleading)

## Improvement Opportunities
- [No actual SQLx integration]: Despite being in a "SQLx-based persistence layer" (per doc), there is no SQLx dependency used. All database methods are stubs returning empty results. Consider implementing or removing the SQLx claim from the architecture (priority: high)
- [Sensitive passwords in config not encrypted]: `StorageConfig` at `mod.rs:25` stores `password: SensitiveString`, but there's no encryption at rest. Database credentials are logged in plain text if Debug trait is used (mod.rs:49 shows "[REDACTED]" but the actual password field serialization needs verification) (priority: medium)

## Stale Items
- [storage/queries.rs is barely used]: The `QueryBuilder` struct in `queries.rs` exists but is never used by the Database struct. The postgres.rs stub doesn't use any queries from queries.rs (priority: low)

## Code Interrogation Findings
- [Missing connection pooling configuration]: `StorageConfig` has `max_connections: u32` field but `Database::new()` ignores it since it's a stub. A real implementation would need to use this for SQLx pool configuration.
- [No transaction support]: The Database stub doesn't have transaction methods (begin, commit, rollback), which would be needed for real-world usage.