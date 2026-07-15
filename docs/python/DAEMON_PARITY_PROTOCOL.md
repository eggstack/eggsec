# Daemon Parity Protocol

## Overview

The daemon parity protocol defines the contract between `eggsec-python`'s
daemon client and the `eggsec-daemon` host for remote session execution.
Release 4 Workstream 12 targets **protocol v2**, closing the gaps between
local `Engine`/`AsyncEngine` execution and daemon-mediated execution.

Until the parity milestone closes, daemon-client APIs remain
**provisional** — the stable-core guarantee covers local execution only.

## Protocol Version

The current protocol version is **v2**. Version negotiation happens during
the `declare_client()` handshake:

```
Client → Daemon:  { "type": "DeclareClient", "version": 2, ... }
Daemon → Client:  { "type": "ServerCapabilities", "min_version": 2, "max_version": 2, ... }
```

If the client's version is outside the daemon's supported range, the
connection is rejected with `ProtocolVersionMismatch`. v1 clients are no
longer supported after Release 4.

## Idempotency and Submission

All `RunRequest` submissions include a client-generated `idempotency_key`
(String, UUID v4 recommended). The daemon uses this key to deduplicate
submissions:

1. If a request with the same `idempotency_key` is already in-flight, the
   daemon returns the existing `task_id` without creating a new task.
2. If the request completed and its result is still in the artifact store,
   the daemon returns the cached `TaskOutcome` without re-executing.
3. If the idempotency key is unknown, a new task is created.

Idempotency keys are stored for the session's `artifact_ttl_secs` (default
24 hours). After expiration, the key is evicted and a duplicate submission
would create a new task.

## Reconnect and Replay

When a daemon client disconnects (network interruption, process restart),
the reconnection protocol ensures no work is lost:

### Reconnect Handshake

```
Client → Daemon:  { "type": "Reconnect", "session_id": "...", "last_event_seq": N }
Daemon → Client:  { "type": "ReconnectAck", "missed_events": [...], "active_tasks": [...] }
```

### Replay Semantics

- **Events since `last_event_seq`**: The daemon replays all `RuntimeEvent`
  entries with `sequence > last_event_seq` in order. This covers status
  changes, findings, and artifact creation events.
- **Active tasks**: Any task that was in-flight at disconnect time is
  included in `active_tasks`. The client can choose to re-subscribe or
  cancel.
- **Completed tasks**: Results for tasks that completed during the disconnect
  window are available via `get_task_result()` with the original `task_id`.
  The result is served from the artifact store without re-execution.

### Sequence Numbers

Every `RuntimeEvent` carries a monotonically increasing `sequence` field
(u64). The daemon persists sequence numbers with the session, so replay
survives daemon restarts within the session TTL.

## Cancellation Semantics

Task cancellation is at-least-once:

```
Client → Daemon:  { "type": "CancelTask", "task_id": "..." }
Daemon → Client:  { "type": "TaskCancelled", "task_id": "...", "state": "cancelling"|"cancelled" }
```

- The daemon transitions the task to `Cancelling` immediately and sends
  `TaskCancelled` with `state: "cancelling"`.
- Once the engine confirms the cancellation (or the timeout expires), a
  final `TaskCancelled` with `state: "cancelled"` is emitted.
- If the client disconnects during cancellation, the daemon completes the
  cancellation independently. On reconnect, the task state reflects the
  final outcome.
- Multiple `CancelTask` messages for the same task are idempotent — the
  daemon returns the current cancellation state without error.

Cancellation does not guarantee immediate resource release. The engine
follows cooperative cancellation via `tokio::select!` cancellation points.
Tasks that do not respect cancellation are terminated on the next timeout
boundary.

## Artifact Parity

Daemon execution must produce identical artifacts to local execution for
the same `RunRequest`. Artifact parity is verified by the release test
suite:

| Artifact | Local Source | Daemon Source | Parity Check |
|----------|-------------|---------------|--------------|
| Findings | `TaskResult.findings` | Content-addressed store | Identical JSON |
| Reports | Output format files | Content-addressed store | Byte-identical |
| Events | In-memory broadcast | Persisted sequence log | Ordered subset |
| Session state | `SessionSnapshot` | SQLite + hydration | Field-by-field |

### Content-Addressed Storage

Artifacts are stored by SHA-256 hash of their content. This means:

- Duplicate artifacts across tasks are stored once.
- Artifact retrieval is O(1) by content hash.
- Artifact integrity is verified on every read.

The `ArtifactStore` trait abstracts storage backends:
- `DirectoryArtifactStore`: Filesystem-backed (default)
- `SqliteArtifactStore`: Database-backed (optional)

### Report Diffing

`ReportDiff` compares two reports (local vs daemon, or two daemon runs)
for structural equivalence:

```python
from eggsec import compare_reports

diff = compare_reports(local_report, daemon_report)
assert diff.is_identical  # True if findings, metadata, and stats match
assert diff.finding_diffs == []  # Empty if no per-finding differences
```

## Event Replay and Ordering

Events are the primary mechanism for the daemon client to observe session
progress. The ordering contract:

1. **Total ordering per session**: Events within a single session are
   ordered by `sequence` number. No two events share a sequence.
2. **Causal ordering**: If event A causally precedes event B (e.g., task
   creation precedes task completion), then `A.sequence < B.sequence`.
3. **Replay completeness**: Replaying all events from `sequence=0` reconstructs
   the full session state, including all task outcomes, findings, and
   artifacts.
4. **No gaps in replay**: The daemon never skips sequence numbers within a
   session. If a gap is detected, the client should request a full
   reconnection.

### Event Buffer

The daemon maintains a circular event buffer per session (default capacity:
10,000 events). Events older than the buffer window are evicted but remain
accessible via the artifact store (findings and reports persist beyond the
event buffer).

The client tracks its last received `sequence` and uses it for reconnect
replay, ensuring exactly-once delivery semantics for the application layer.

### Event Types and Priority

| Event Kind | Priority | Description |
|------------|----------|-------------|
| `task.created` | High | New task submitted |
| `task.progress` | Low | Intermediate progress update |
| `task.completed` | High | Task finished successfully |
| `task.failed` | High | Task failed with error |
| `task.cancelled` | High | Task cancellation confirmed |
| `finding.created` | High | New finding discovered |
| `finding.updated` | Medium | Finding severity/status changed |
| `artifact.created` | Medium | New artifact stored |
| `session.snapshot` | High | Full session state snapshot |

High-priority events are never evicted from the in-flight buffer. Low and
medium priority events are evicted when the buffer reaches capacity.
