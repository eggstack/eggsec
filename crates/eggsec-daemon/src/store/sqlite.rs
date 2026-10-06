use std::path::Path;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use rusqlite::Connection;

use super::{DaemonStore, PersistedAuditEvent};
use eggsec_runtime::{SessionId, SessionSnapshot};

const SCHEMA_DDL: &str = "
CREATE TABLE IF NOT EXISTS session_snapshots (
    session_id TEXT PRIMARY KEY,
    snapshot_json TEXT NOT NULL,
    created_at_secs INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS audit_events (
    audit_id INTEGER PRIMARY KEY AUTOINCREMENT,
    action TEXT NOT NULL,
    surface TEXT NOT NULL,
    outcome TEXT NOT NULL,
    client_id TEXT,
    session_id TEXT,
    created_at_secs INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS schema_meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
";

const SCHEMA_VERSION: &str = "2";

pub struct SqliteStore {
    /// Shared so per-call bodies can run on the blocking thread pool via
    /// `spawn_blocking` (rusqlite performs synchronous disk I/O).
    conn: Arc<Mutex<Connection>>,
}

impl SqliteStore {
    pub fn new(path: &Path) -> anyhow::Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;
        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        store.migrate()?;
        Ok(store)
    }

    pub fn new_in_memory() -> anyhow::Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;
        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&self) -> anyhow::Result<()> {
        let conn = lock_conn(&self.conn);
        conn.execute_batch(SCHEMA_DDL)?;
        let stored: Option<String> = conn
            .query_row(
                "SELECT value FROM schema_meta WHERE key = 'schema_version'",
                [],
                |row| row.get(0),
            )
            .ok();
        if let Some(existing) = stored {
            if existing != SCHEMA_VERSION {
                let existing_num: u32 = existing.parse().unwrap_or(0);
                let current_num: u32 = SCHEMA_VERSION.parse().unwrap_or(0);
                if existing_num > current_num {
                    anyhow::bail!(
                        "persisted schema version ({}) is newer than current schema ({}) — refusing to load; please upgrade eggsec",
                        existing, SCHEMA_VERSION
                    );
                }
                tracing::warn!(
                    existing_schema = %existing,
                    current_schema = %SCHEMA_VERSION,
                    "Migrating persisted schema version; data layout assumptions may need review"
                );
            }
        }
        conn.execute(
            "INSERT OR REPLACE INTO schema_meta (key, value) VALUES ('schema_version', ?1)",
            [SCHEMA_VERSION],
        )?;
        Ok(())
    }
}

/// Lock the shared SQLite connection, recovering from poisoning.
fn lock_conn(conn: &Arc<Mutex<Connection>>) -> std::sync::MutexGuard<'_, Connection> {
    conn.lock().unwrap_or_else(|poisoned| {
        tracing::warn!("sqlite connection mutex was poisoned; recovering connection");
        poisoned.into_inner()
    })
}

#[async_trait]
impl DaemonStore for SqliteStore {
    async fn save_session_snapshot(&self, snapshot: &SessionSnapshot) -> anyhow::Result<()> {
        let json = serde_json::to_string(snapshot)?;
        // The session's own creation time, not the save time.
        //
        // `session_snapshots.created_at_secs` is what both `load_all_sessions`
        // and `blocking_list_sessions` order by, and what the column name says.
        // Writing the wall clock here instead meant an `INSERT OR REPLACE` on
        // every lifecycle point rewrote the row's position, so re-saving a
        // session reordered the list — the opposite of a stable "sessions, oldest
        // first" view.
        let created_at_secs = snapshot.created_at_epoch_secs as i64;
        let session_id = snapshot.session_id.to_string();
        let conn = Arc::clone(&self.conn);
        tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            let conn = lock_conn(&conn);
            conn.execute(
                "INSERT OR REPLACE INTO session_snapshots (session_id, snapshot_json, created_at_secs) VALUES (?1, ?2, ?3)",
                rusqlite::params![session_id, json, created_at_secs],
            )?;
            Ok(())
        })
        .await
        .map_err(|e| anyhow::anyhow!("sqlite task failed: {}", e))?
    }

    async fn load_session_snapshot(
        &self,
        session_id: SessionId,
    ) -> anyhow::Result<Option<SessionSnapshot>> {
        let conn = Arc::clone(&self.conn);
        tokio::task::spawn_blocking(move || -> anyhow::Result<Option<SessionSnapshot>> {
            let conn = lock_conn(&conn);
            let mut stmt =
                conn.prepare("SELECT snapshot_json FROM session_snapshots WHERE session_id = ?1")?;
            let mut rows = stmt.query_map([session_id.to_string()], |row| {
                let json: String = row.get(0)?;
                Ok(json)
            })?;
            match rows.next() {
                Some(row) => {
                    let json = row?;
                    let snapshot: SessionSnapshot = serde_json::from_str(&json)?;
                    Ok(Some(snapshot))
                }
                None => Ok(None),
            }
        })
        .await
        .map_err(|e| anyhow::anyhow!("sqlite task failed: {}", e))?
    }

    async fn load_all_sessions(&self) -> anyhow::Result<Vec<SessionSnapshot>> {
        let conn = Arc::clone(&self.conn);
        tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<SessionSnapshot>> {
            let conn = lock_conn(&conn);
            let mut stmt = conn.prepare(
                "SELECT snapshot_json FROM session_snapshots ORDER BY created_at_secs ASC",
            )?;
            let rows = stmt.query_map([], |row| {
                let json: String = row.get(0)?;
                Ok(json)
            })?;
            let mut sessions = Vec::new();
            for row in rows {
                let json = row?;
                let snapshot: SessionSnapshot = serde_json::from_str(&json)?;
                sessions.push(snapshot);
            }
            Ok(sessions)
        })
        .await
        .map_err(|e| anyhow::anyhow!("sqlite task failed: {}", e))?
    }

    async fn record_audit_event(&self, event: &PersistedAuditEvent) -> anyhow::Result<()> {
        let event = event.clone();
        let conn = Arc::clone(&self.conn);
        tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            let conn = lock_conn(&conn);
            conn.execute(
                "INSERT INTO audit_events (action, surface, outcome, client_id, session_id, created_at_secs) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    event.action,
                    event.surface,
                    event.outcome,
                    event.client_id,
                    event.session_id,
                    event.timestamp_secs as i64,
                ],
            )?;
            Ok(())
        })
        .await
        .map_err(|e| anyhow::anyhow!("sqlite task failed: {}", e))?
    }

    async fn delete_session(&self, session_id: SessionId) -> anyhow::Result<()> {
        let conn = Arc::clone(&self.conn);
        tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            let conn = lock_conn(&conn);
            conn.execute(
                "DELETE FROM session_snapshots WHERE session_id = ?1",
                [session_id.to_string()],
            )?;
            Ok(())
        })
        .await
        .map_err(|e| anyhow::anyhow!("sqlite task failed: {}", e))?
    }

    fn blocking_list_sessions(&self) -> anyhow::Result<Vec<eggsec_runtime::SessionSummary>> {
        let conn = lock_conn(&self.conn);
        let mut stmt = conn
            .prepare("SELECT snapshot_json FROM session_snapshots ORDER BY created_at_secs ASC")?;
        let rows = stmt.query_map([], |row| {
            let json: String = row.get(0)?;
            Ok(json)
        })?;
        let mut summaries = Vec::new();
        for row in rows {
            let json = row?;
            let snapshot: SessionSnapshot = serde_json::from_str(&json)?;
            summaries.push(eggsec_runtime::SessionSummary {
                session_id: snapshot.session_id,
                surface: snapshot.surface,
                scope: snapshot.scope,
                active_count: snapshot.active_tasks.len(),
                completed_count: snapshot.completed_tasks.len(),
                created_at_epoch_secs: snapshot.created_at_epoch_secs,
                owner_client_id: snapshot.owner_client_id,
            });
        }
        Ok(summaries)
    }

    fn blocking_get_snapshot(
        &self,
        session_id: &SessionId,
    ) -> anyhow::Result<Option<SessionSnapshot>> {
        let conn = lock_conn(&self.conn);
        let mut stmt =
            conn.prepare("SELECT snapshot_json FROM session_snapshots WHERE session_id = ?1")?;
        let mut rows = stmt.query_map([session_id.to_string()], |row| {
            let json: String = row.get(0)?;
            Ok(json)
        })?;
        match rows.next() {
            Some(row) => {
                let json = row?;
                let snapshot: SessionSnapshot = serde_json::from_str(&json)?;
                Ok(Some(snapshot))
            }
            None => Ok(None),
        }
    }
}

pub struct NoopStore;

#[async_trait]
impl DaemonStore for NoopStore {
    async fn save_session_snapshot(&self, _snapshot: &SessionSnapshot) -> anyhow::Result<()> {
        Ok(())
    }

    async fn load_session_snapshot(
        &self,
        _session_id: SessionId,
    ) -> anyhow::Result<Option<SessionSnapshot>> {
        Ok(None)
    }

    async fn load_all_sessions(&self) -> anyhow::Result<Vec<SessionSnapshot>> {
        Ok(Vec::new())
    }

    async fn record_audit_event(&self, _event: &PersistedAuditEvent) -> anyhow::Result<()> {
        Ok(())
    }

    async fn delete_session(&self, _session_id: SessionId) -> anyhow::Result<()> {
        Ok(())
    }

    fn blocking_list_sessions(&self) -> anyhow::Result<Vec<eggsec_runtime::SessionSummary>> {
        Ok(Vec::new())
    }

    fn blocking_get_snapshot(
        &self,
        _session_id: &SessionId,
    ) -> anyhow::Result<Option<SessionSnapshot>> {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::DaemonStore;

    fn snapshot(id: SessionId, created_at: u64) -> SessionSnapshot {
        SessionSnapshot {
            session_id: id,
            surface: eggsec_runtime::RuntimeSurface::TuiManual,
            scope: None,
            created_at_epoch_secs: created_at,
            generation: 0,
            active_tasks: vec![],
            completed_tasks: vec![],
            capabilities: eggsec_runtime::RuntimeCapabilities::default(),
            closed: false,
            closed_at: None,
            owner_client_id: None,
        }
    }

    /// Drive one async save from a plain `#[test]`, for the cases that exercise
    /// the synchronous `blocking_*` API and so cannot be `#[tokio::test]`.
    fn save_blocking(store: &SqliteStore, id: SessionId, created_at: u64) {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime")
            .block_on(store.save_session_snapshot(&snapshot(id, created_at)));
    }

    /// A round-trip through SQLite must return the snapshot it stored.
    ///
    /// `SqliteStore` had no coverage at all before the rusqlite 0.31 -> 0.32
    /// bump, which was needed to clear the `sqlx` advisory: both crates link
    /// `sqlite3`, so they must agree on `libsqlite3-sys`. These tests are what
    /// make that upgrade verified rather than merely compiled.
    #[tokio::test]
    async fn sqlite_save_then_load_round_trips() {
        let store = SqliteStore::new_in_memory().unwrap();
        let id = SessionId::new();

        store
            .save_session_snapshot(&snapshot(id, 42))
            .await
            .unwrap();

        let loaded = store
            .load_session_snapshot(id)
            .await
            .unwrap()
            .expect("stored snapshot must be readable");
        assert_eq!(loaded.session_id, id);
        assert_eq!(loaded.created_at_epoch_secs, 42);
    }

    #[tokio::test]
    async fn sqlite_load_missing_session_is_none() {
        let store = SqliteStore::new_in_memory().unwrap();
        assert!(store
            .load_session_snapshot(SessionId::new())
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn sqlite_load_all_is_ordered_by_creation() {
        let store = SqliteStore::new_in_memory().unwrap();
        let (older, newer) = (SessionId::new(), SessionId::new());
        store
            .save_session_snapshot(&snapshot(newer, 200))
            .await
            .unwrap();
        store
            .save_session_snapshot(&snapshot(older, 100))
            .await
            .unwrap();

        let all = store.load_all_sessions().await.unwrap();
        assert_eq!(all.len(), 2, "both snapshots must be listed");
        assert_eq!(all[0].created_at_epoch_secs, 100, "ordered by created_at");
        assert_eq!(all[1].created_at_epoch_secs, 200);
    }

    /// Saving the same session twice must replace, not duplicate or error --
    /// the snapshot path is taken repeatedly across a session's lifecycle.
    #[tokio::test]
    async fn sqlite_repeated_save_replaces() {
        let store = SqliteStore::new_in_memory().unwrap();
        let id = SessionId::new();
        store.save_session_snapshot(&snapshot(id, 1)).await.unwrap();
        let mut updated = snapshot(id, 2);
        updated.generation = 7;
        store.save_session_snapshot(&updated).await.unwrap();

        let all = store.load_all_sessions().await.unwrap();
        assert_eq!(all.len(), 1, "re-save must not create a second row");
        assert_eq!(all[0].generation, 7);
    }

    #[tokio::test]
    async fn sqlite_delete_removes_the_row() {
        let store = SqliteStore::new_in_memory().unwrap();
        let id = SessionId::new();
        store.save_session_snapshot(&snapshot(id, 1)).await.unwrap();
        store.delete_session(id).await.unwrap();

        assert!(store.load_session_snapshot(id).await.unwrap().is_none());
        assert!(store.load_all_sessions().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn sqlite_records_audit_events_with_and_without_optional_fields() {
        let store = SqliteStore::new_in_memory().unwrap();
        let with = PersistedAuditEvent {
            action: "dispatch".into(),
            surface: "rest_strict".into(),
            outcome: "allow".into(),
            client_id: Some("client-1".into()),
            session_id: Some("session-1".into()),
            timestamp_secs: 1_700_000_000,
        };
        let without = PersistedAuditEvent {
            action: "denied".into(),
            surface: "agent_strict".into(),
            outcome: "deny".into(),
            client_id: None,
            session_id: None,
            timestamp_secs: 0,
        };
        store.record_audit_event(&with).await.unwrap();
        store.record_audit_event(&without).await.unwrap();

        let count: i64 = lock_conn(&store.conn)
            .query_row("SELECT COUNT(*) FROM audit_events", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 2, "both audit rows must persist");
    }

    #[test]
    fn sqlite_blocking_api_matches_async_list() {
        let store = SqliteStore::new_in_memory().unwrap();
        let (first, second) = (SessionId::new(), SessionId::new());
        save_blocking(&store, first, 10);
        save_blocking(&store, second, 20);

        let summaries = store.blocking_list_sessions().unwrap();
        assert_eq!(summaries.len(), 2);
        assert_eq!(summaries[0].created_at_epoch_secs, 10);
        assert_eq!(summaries[1].created_at_epoch_secs, 20);
        assert_eq!(
            store
                .blocking_get_snapshot(&first)
                .unwrap()
                .map(|s| s.session_id),
            Some(first),
            "blocking_get_snapshot must resolve a stored id"
        );
    }

    /// The file-backed store must survive dropping the handle: this is the
    /// daemon-restart recovery path, and the reason WAL mode is set.
    #[test]
    fn sqlite_persists_across_reopen() {
        let dir = std::env::temp_dir().join(format!("eggsec-sqlite-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("sessions.sqlite3");
        let id = SessionId::new();

        {
            let store = SqliteStore::new(&path).unwrap();
            save_blocking(&store, id, 99);
        } // handle dropped, simulating a daemon restart

        let reopened = SqliteStore::new(&path).unwrap();
        let loaded = reopened
            .blocking_get_snapshot(&id)
            .unwrap()
            .expect("snapshot must survive reopen");
        assert_eq!(loaded.session_id, id);
        assert_eq!(loaded.created_at_epoch_secs, 99);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A database written by a newer eggsec must be refused, not silently
    /// downgraded -- the schema-version guard in `migrate`.
    #[test]
    fn sqlite_refuses_newer_schema_version() {
        let store = SqliteStore::new_in_memory().unwrap();
        lock_conn(&store.conn)
            .execute(
                "INSERT OR REPLACE INTO schema_meta (key, value) VALUES ('schema_version', ?1)",
                ["9999"],
            )
            .unwrap();

        let err = store.migrate().expect_err("a newer schema must be refused");
        assert!(
            err.to_string().contains("newer than current schema"),
            "the refusal must explain itself, got: {err}"
        );
    }

    #[test]
    fn sqlite_store_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<SqliteStore>();
    }
}
