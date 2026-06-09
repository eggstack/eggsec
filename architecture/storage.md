# Storage Module

## Purpose

SQLx-based persistence layer using PostgreSQL for storing scan results, findings, and metadata. Feature-gated behind the `database` flag.

## Key Types

| Type | Location | Description |
|------|----------|-------------|
| `StorageConfig` | `storage/mod.rs` | Database connection configuration (host, port, credentials, pool size) |
| `Database` | `storage/postgres.rs` | PostgreSQL connection pool and operations |
| `StoredScan` | `storage/models.rs` | Database model for scan records |
| `StoredFinding` | `storage/models.rs` | Database model for finding records |

## Files

| File | Description |
|------|-------------|
| `mod.rs` | Module root: `StorageConfig`, `init_storage()` factory function |
| `postgres.rs` | PostgreSQL connection pool, CRUD operations |
| `models.rs` | Database model structs (StoredScan, StoredFinding, etc.) |
| `queries.rs` | Predefined SQL queries (insert, update, search) |

## Implementation Status

Fully implemented behind `database` feature flag. SQLx-based persistence with PostgreSQL using `PgPool` connection pool via `PgPoolOptions`. All CRUD operations use parameterized queries. `Finding` data and `status_history` are stored in JSONB columns. The unified `StoredFinding` type is re-exported from `findings::lifecycle`. Migrations live in `crates/eggsec/migrations/`. Returns a config error when the feature is not enabled.
