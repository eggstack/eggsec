use std::path::Path;
use std::sync::Mutex;

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
    conn: Mutex<Connection>,
}

impl SqliteStore {
    pub fn new(path: &Path) -> anyhow::Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;
        let store = Self {
            conn: Mutex::new(conn),
        };
        store.migrate()?;
        Ok(store)
    }

    pub fn new_in_memory() -> anyhow::Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;
        let store = Self {
            conn: Mutex::new(conn),
        };
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&self) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(SCHEMA_DDL)?;
        conn.execute(
            "INSERT OR REPLACE INTO schema_meta (key, value) VALUES ('schema_version', ?1)",
            [SCHEMA_VERSION],
        )?;
        Ok(())
    }
}

#[async_trait]
impl DaemonStore for SqliteStore {
    async fn save_session_snapshot(&self, snapshot: &SessionSnapshot) -> anyhow::Result<()> {
        let json = serde_json::to_string(snapshot)?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let session_id = snapshot.session_id.to_string();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO session_snapshots (session_id, snapshot_json, created_at_secs) VALUES (?1, ?2, ?3)",
            rusqlite::params![session_id, json, now],
        )?;
        Ok(())
    }

    async fn load_session_snapshot(
        &self,
        session_id: SessionId,
    ) -> anyhow::Result<Option<SessionSnapshot>> {
        let conn = self.conn.lock().unwrap();
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

    async fn load_all_sessions(&self) -> anyhow::Result<Vec<SessionSnapshot>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT snapshot_json FROM session_snapshots ORDER BY created_at_secs ASC")?;
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
    }

    async fn record_audit_event(&self, event: &PersistedAuditEvent) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
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
    }

    async fn delete_session(&self, session_id: SessionId) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM session_snapshots WHERE session_id = ?1",
            [session_id.to_string()],
        )?;
        Ok(())
    }

    fn blocking_list_sessions(&self) -> anyhow::Result<Vec<eggsec_runtime::SessionSummary>> {
        let conn = self.conn.lock().unwrap();
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
                created_at_secs: snapshot.created_at_secs,
            });
        }
        Ok(summaries)
    }

    fn blocking_get_snapshot(
        &self,
        session_id: &SessionId,
    ) -> anyhow::Result<Option<SessionSnapshot>> {
        let conn = self.conn.lock().unwrap();
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
