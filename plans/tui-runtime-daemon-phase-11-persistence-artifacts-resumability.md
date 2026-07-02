# Phase 11 Plan: Persistence, Artifacts, and Resumability

## Goal

Add durable daemon session state, artifact indexing, and restart-safe session recovery without changing the local-only daemon security boundary. After this phase, the daemon should be able to restart and recover useful session metadata, completed task outcomes, artifact references, and audit records.

This phase should not attempt full distributed persistence. It is local daemon durability only.

## Current Baseline

The daemon/runtime architecture now has:

- `eggsec-runtime` for lifecycle, sessions, events, snapshots, task outcomes, and generation tracking.
- `eggsec-daemon` for local transport, daemon host, protocol, client registry, session access, and authorization.
- `TaskOutcome::Result(TaskResultEnvelope)` for protocol-neutral completion summaries.
- `ArtifactRef` as a protocol-facing artifact pointer.
- Multi-client observer/controller/owner/approver semantics.
- Explicit daemon authorization and unsupported `ApprovePolicy` placeholder behavior.

The missing piece is durable state. Today a daemon restart likely loses session/task history and may leave artifacts unindexed.

## Scope

Persist only what is safe and necessary for local resumability:

- daemon session metadata;
- session access metadata;
- runtime surface/scope;
- completed task records and outcomes;
- artifact references and metadata;
- audit events for security-relevant daemon actions;
- daemon version/schema version;
- optional recent event replay cursor.

Do not persist secrets, raw traffic payloads, credentials, or high-risk task inputs unless explicitly classified and stored under a redaction policy.

## Storage Design

Preferred storage is SQLite behind a small repository layer.

Suggested crate/module layout:

```text
crates/eggsec-daemon/src/store/
  mod.rs
  sqlite.rs
  schema.rs
  models.rs
```

If the repo already has a persistence crate or database abstraction, reuse it rather than adding a parallel system.

Suggested configuration:

```text
~/.local/share/eggsec/daemon/eggsec-daemon.sqlite
~/.local/share/eggsec/daemon/artifacts/
```

Follow platform conventions if the repo already has config/data-dir helpers.

## Data Model

Minimum tables:

```text
sessions
  session_id TEXT PRIMARY KEY
  surface TEXT NOT NULL
  scope_json TEXT
  owner_client_kind TEXT
  created_at_secs INTEGER NOT NULL
  updated_at_secs INTEGER NOT NULL
  generation INTEGER NOT NULL
  labels_json TEXT NOT NULL

session_access
  session_id TEXT PRIMARY KEY
  owner_client_id TEXT
  owner_client_kind TEXT
  default_observer_allowed BOOLEAN NOT NULL
  default_controller_allowed BOOLEAN NOT NULL
  allowed_clients_json TEXT NOT NULL

tasks
  task_id TEXT PRIMARY KEY
  session_id TEXT NOT NULL
  kind TEXT NOT NULL
  status TEXT NOT NULL
  request_json TEXT
  outcome_json TEXT
  started_at_secs INTEGER
  completed_at_secs INTEGER
  generation INTEGER NOT NULL

artifacts
  artifact_id TEXT PRIMARY KEY
  session_id TEXT NOT NULL
  task_id TEXT
  kind TEXT NOT NULL
  path TEXT
  mime_type TEXT
  summary TEXT
  created_at_secs INTEGER NOT NULL
  metadata_json TEXT NOT NULL

audit_events
  audit_id TEXT PRIMARY KEY
  session_id TEXT
  task_id TEXT
  client_id TEXT
  client_kind TEXT
  command TEXT NOT NULL
  decision TEXT NOT NULL
  reason TEXT
  created_at_secs INTEGER NOT NULL
  metadata_json TEXT NOT NULL

schema_meta
  key TEXT PRIMARY KEY
  value TEXT NOT NULL
```

Use JSON columns initially to avoid overfitting. Keep DTO serialization stable.

## Workstream 1: Store Interface

Add a trait that the daemon host can use without tying runtime to SQLite.

Suggested trait:

```rust
pub trait DaemonStore: Send + Sync {
    async fn save_session(&self, snapshot: &SessionSnapshot, access: &SessionAccess) -> Result<()>;
    async fn load_sessions(&self) -> Result<Vec<PersistedSession>>;
    async fn save_task_record(&self, session_id: SessionId, record: &TaskRecord) -> Result<()>;
    async fn save_artifact(&self, artifact: &ArtifactRecord) -> Result<()>;
    async fn save_audit_event(&self, event: &AuditEvent) -> Result<()>;
}
```

Use a no-op store for tests or memory-only daemon mode.

## Workstream 2: Audit Event Model

Define a durable audit event for security-relevant actions.

Audit at least:

- client declaration;
- session creation/close;
- session attach/subscribe;
- task submit/cancel;
- permission denial;
- policy approval attempts;
- unsupported policy approval;
- daemon start/shutdown if practical.

Fields should include client ID, client kind, session ID, surface, command, decision, reason, timestamp, and request ID.

## Workstream 3: Artifact Indexing

Add a local artifact index that records metadata without assuming every artifact is safe to expose.

Rules:

- `ArtifactRef` should not expose arbitrary paths to remote clients without classification.
- Store artifact path, kind, MIME type, summary, task ID, session ID, and metadata.
- Add a future-facing `access_class` field if useful: `PublicSummary`, `LocalPathOnly`, `Sensitive`, `Redacted`.
- Do not add artifact download transport in this phase unless local-only and guarded.

## Workstream 4: Daemon Startup Recovery

On daemon start:

1. Open store.
2. Run migrations.
3. Load persisted sessions.
4. Reconstruct session summaries and completed task history.
5. Do not automatically resume active tasks unless explicitly supported.
6. Mark previously active tasks as `Interrupted` or `Abandoned` with a restart reason.
7. Emit audit event for recovery.

Active task recovery should be conservative. Do not restart network/security operations automatically.

## Workstream 5: Snapshot Persistence

Persist snapshots at safe lifecycle points:

- session creation;
- task submitted;
- task started;
- task completed/failed/cancelled;
- session closed;
- access change;
- artifact registration.

Avoid high-frequency progress persistence unless throttled.

## Workstream 6: CLI/TUI Resumability

TUI daemon attach should show recovered sessions after daemon restart.

CLI should support:

```text
eggsec session list --socket <path>
eggsec session snapshot <session-id> --socket <path>
```

and see recovered completed task history.

## Files Likely to Change

- `crates/eggsec-daemon/Cargo.toml`
- `crates/eggsec-daemon/src/config.rs`
- `crates/eggsec-daemon/src/host.rs`
- `crates/eggsec-daemon/src/protocol.rs`
- `crates/eggsec-daemon/src/store/*`
- `crates/eggsec-runtime/src/session.rs`
- `crates/eggsec-runtime/src/event.rs`
- `crates/eggsec-cli/src/daemon_cli.rs`
- `crates/eggsec-tui/src/runtime_client/daemon.rs`
- `architecture/overview.md`
- `architecture/tui.md`

## Tests

- Store migration creates expected schema.
- Session persists and reloads with surface/scope/generation.
- Completed task outcome persists and reloads.
- Artifact refs persist and reload.
- Permission denial writes audit event.
- Unsupported `ApprovePolicy` writes audit event.
- Daemon restart marks active tasks interrupted, not resumed.
- TUI/CLI can list recovered sessions through daemon host/client tests.

## Non-Goals

Do not implement cloud sync.

Do not auto-resume active security tasks after restart.

Do not expose artifacts over public transport.

Do not store secrets or raw sensitive payloads without redaction.

Do not add remote authentication.

## Validation

Run:

```bash
cargo fmt --all --check
cargo check -p eggsec-daemon
cargo test -p eggsec-daemon
cargo check -p eggsec-runtime
cargo test -p eggsec-runtime
cargo check -p eggsec-cli
cargo check -p eggsec-tui
./scripts/check-architecture-guards.sh
```

## Acceptance Criteria

- Daemon state persists across restart.
- Completed task outcomes and artifact refs survive restart.
- Active tasks are marked interrupted, not auto-resumed.
- Audit events are durable for daemon security decisions.
- TUI/CLI can inspect recovered sessions.
- Runtime remains persistence-free; persistence belongs to daemon/store layer.
