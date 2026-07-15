# Daemon Protocol and Parity

This guide covers the daemon protocol types for local/remote execution parity.

## Protocol Version

```python
from eggsec import DaemonProtocolVersion

version = DaemonProtocolVersion(
    api_schema_version=1,
    operation_registry_id="eggsec-ops-v1",
    feature_profile="default",
)
print(f"Protocol: {version.protocol_version}")
print(f"Schema: {version.api_schema_version}")
```

## Idempotent Submission

```python
from eggsec import IdempotencyKey

key = IdempotencyKey.from_request(
    operation_name="port_scan",
    request_json='{"target":"example.com","ports":[80,443]}',
)
print(f"Key: {key.key}")
print(f"Hash: {key.request_hash}")
```

## Reconnect Options

```python
from eggsec import ReconnectOptions

options = ReconnectOptions(
    max_retries=5,
    retry_delay_ms=500,
    backoff_multiplier=2.0,
    max_backoff_ms=30000,
    replay_from_sequence=42,
)
```

## Replay Cursor

```python
from eggsec import ReplayCursor, ReplayResult

cursor = ReplayCursor(
    session_id="sess-1",
    last_sequence=100,
    total_events=150,
    gap_count=2,
    duplicate_count=0,
    timestamp_ms=1234567890,
)
```

## Cancellation

```python
from eggsec import CancellationRequest, CancellationResult

request = CancellationRequest(
    session_id="sess-1",
    task_id="task-1",
    reason="Timeout exceeded",
    force=False,
    requested_at_ms=1234567890,
)

result = CancellationResult(
    acknowledged=True,
    task_was_running=True,
    task_was_completed=False,
    cleanup_started=True,
    message="Task cancelled successfully",
)
```

## Task Artifact Descriptor

```python
from eggsec import TaskArtifactDescriptor

artifact = TaskArtifactDescriptor(
    artifact_id="art-1",
    task_id="task-1",
    session_id="sess-1",
    kind="screenshot",
    content_type="image/png",
    size_bytes=102400,
    content_hash="sha256/abc123...",
    created_at_ms=1234567890,
    redacted=False,
    download_url="https://daemon.local/artifacts/art-1",
)
```

## Daemon Health

```python
from eggsec import DaemonHealthDetail

health = DaemonHealthDetail(
    status="ok",
    uptime_secs=86400,
    protocol_version=2,
    active_sessions=5,
    active_clients=3,
    total_tasks_completed=1234,
    persistence_backend="sqlite",
    transport="unix",
)
```
