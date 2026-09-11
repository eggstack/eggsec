# Daemon Execution Parity (Phase E WS4–WS6)

> Status: Phase E is complete (protocol v2 shipped, `DAEMON_PROTOCOL_VERSION = 2`).
> This document is retained as the historical parity record. For current
> transport configuration, schema, and CLI commands, see [DAEMON.md](DAEMON.md).

Local `Engine`/runtime execution and daemon-backed execution share the same
canonical request/result contracts (`RunRequest`/`TaskKind` in,
`TaskOutcome`/`TaskResultEnvelope` out). This document records where the two
paths behave identically, where the daemon intentionally differs because it
is durable and multi-client, and how programmable clients retrieve results
after reconnect.

Wire protocol version: `DAEMON_PROTOCOL_VERSION = 2` (v2 adds durable
`GetTaskResult` / `TaskResult`; see below).

## Parity matrix (WS4)

| Area | Local execution | Daemon execution | Verdict |
|------|----------------|------------------|---------|
| Request normalization | `RunRequest { task_kind, surface, labels }` validated at dispatch | Same `RunRequest` type; `SubmitTask` carries it unchanged; unknown task kinds rejected with `InvalidRequest` | Same |
| Policy/scope enforcement | `EnforcementContext::evaluate()` pre-dispatch gate | Same gate inside the runtime executor; manual surfaces additionally support `ApprovePolicy` | Same, plus approval flow |
| Task submission identifiers | Caller-held task IDs | Daemon mints `TaskId` per `SubmitTask`, returned in `TaskSubmitted` | Same shape, daemon-minted IDs |
| Progress/event ordering | In-process channel order | `broadcast::channel` per runtime; `SessionSnapshot.generation` increments on every state change for change detection | Same ordering; generation counter is the ordering signal |
| Lag/replay semantics | N/A (no lag) | Events are transient broadcasts — a lagged receiver misses them. Durable state is always re-readable via snapshot/result queries (see below) | Intentionally different: snapshots, not replay |
| Result retrieval after completion | Direct return value | `GetTaskResult { session_id, task_id }` → `TaskResult { status, outcome }`; live state first, persisted snapshot fallback | Daemon superset (durable) |
| Cancellation before/after start | `CancellationToken` | `CancelTask` / `CancelActive`; single-active-task policy auto-cancels the previous task on new submit | Same, plus auto-cancel policy |
| Client disconnect/reconnect | N/A | Disconnect drops the socket only; session and task state persist server-side. Reconnect = new socket + `DeclareClient`, then re-`Subscribe` and/or query snapshots/results | Documented flow (no resume handshake) |
| Daemon restart with persistence | N/A | `recover_persisted_state()` hydrates snapshots; active/queued tasks are marked `Cancelled` (`"interrupted by daemon restart"`); completed outcomes survive | Intentionally different: tasks are dropped, results kept |
| Timeouts | Caller-side | `Subscribe` reads and HTTP handlers are bounded; persistence fan-out uses `PERSISTENCE_TASK_TIMEOUT` | Same posture, daemon-side bounds |
| Structured errors | `OperationError` DTOs | `ServerMessage::Error { code: ErrorCode, message }`; `TaskFailed` carries `RuntimeErrorInfo` | Same structure, transport envelope differs |
| Artifact references/retrieval | In-process `Artifact` handles | `ArtifactRef { id, kind, path, mime_type, summary }` inside outcomes and snapshots; metadata survives the same lifecycle as the task result | Same metadata; bytes stay daemon-side |
| Capability discovery/version negotiation | Feature flags at build time | `Capabilities` → `DaemonCapabilities { runtime, transports }`; `Health` carries `protocol_version`; clients must check it before sending commands | Daemon superset |
| Ownership/RBAC | Single caller | `ClientRole` (Owner/Controller/Observer/Approver) × `CommandPermission`; `GetTaskResult` requires `Observer`; ownership persists across restarts | Daemon superset |

## Result retrieval and reconnect/replay (WS5)

Three rules cover every reconnect scenario:

1. **Completed results never depend on event delivery.** `GetTaskResult`
   reads the live snapshot first and the persisted snapshot second, so a
   client that connects after completion — or after a daemon restart — gets
   the same `TaskResult { status, outcome }` as a client that watched the
   `TaskCompleted` broadcast. CLI: `eggsec task result <session> <task>`;
   HTTP: `GET /sessions/{id}/tasks/{task_id}`; Python:
   `async_daemon_get_task_result(client, session_id, task_id)`.
2. **Reconnect is re-declare + re-subscribe.** There is no session resume
   handshake: open a new socket, `DeclareClient`, then `GetSnapshot` /
   `GetTaskResult` for durable state and `Subscribe` for new events. The
   `generation` counter on each snapshot tells the client whether anything
   changed while it was away.
3. **Duplicates are detectable.** A repeated `TaskResult` with an identical
   `(session_id, task_id, status)` triple carries no new information;
   clients key on that triple. Cancellation state persists in the snapshot
   (`Cancelled` status + `last_error`), so a cancelled task reads back as
   cancelled after reconnect. Artifact metadata rides inside the outcome
   and snapshots, sharing the result lifecycle.

What is *not* provided: an event log or replay buffer. `RuntimeEvent`
broadcasts are transient by design; missed events are recovered as state
(`Snapshot`/`TaskResult`), never as replayed events. Lagged receivers
distinguish "missed events" (generation jumped) from "terminal closure"
(`SessionClosed` / snapshot `closed=true`) via the snapshot, not the
stream.

## Python client parity (WS6)

The Python daemon client maps onto the same canonical contracts as local
execution — there is no second family of result DTOs:

```text
local Engine  -> canonical result (typed DTOs, e.g. PortScanReport)
DaemonClient  -> DaemonResponsePy transport envelope + canonical payloads
                 (TaskResult outcome JSON uses the same TaskOutcome schema)
```

`async_daemon_get_task_result()` returns the canonical `TaskOutcome` JSON
for a completed task, so local and daemon paths stay type-compatible with
transport metadata (`request_id`, `ok`, `error_code`) kept separate in
`DaemonResponsePy`. Sync/async parity follows the existing
`runtime_sync::block_on` / `PyFuture` split used by every other daemon
client function. End-to-end coverage uses a spawned local daemon fixture
over an ephemeral Unix socket (see `test_daemon_integration.py`); no
public network is involved.
