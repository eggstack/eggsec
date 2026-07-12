# Events

The event protocol provides versioned, typed events for observability into
scan lifecycle, progress, findings, and errors. All events are wrapped in
`EventEnvelope` for backward compatibility and schema tracking.

## EventEnvelope

Every event is wrapped in an envelope that carries version metadata:

```python
import eggsec

env = eggsec.EventEnvelope(
    event_type="progress",
    payload={"percentage": 50.0, "message": "Scanning..."},
)
print(env.schema_version)  # "1.0.0"
print(env.event_id)        # "evt-1719000000000"
print(env.event_type)      # "progress"
```

| Property | Type | Description |
|---|---|---|
| `schema_version` | `str` | Event schema version (currently `"1.0.0"`). |
| `event_id` | `str` | Unique event identifier. |
| `timestamp_ms` | `u64` | Unix timestamp in milliseconds. |
| `correlation_id` | `str \| None` | Optional correlation ID for grouping related events. |
| `event_type` | `str` | Type discriminator (e.g. `"progress"`, `"finding"`). |
| `payload` | `object` | Typed event payload. |

### Methods

```python
EventEnvelope.to_dict() -> dict
EventEnvelope.to_json() -> str
```

### Constructing events

Use `wrap_event()` to create envelopes from typed payloads:

```python
from eggsec import EventEnvelope, PlanningEvent, wrap_event

planning = PlanningEvent(
    operation_id="scan-ports",
    target="example.com",
    scope_summary="Allow: example.com",
)
env = wrap_event("planning", planning, correlation_id="run-42")
```

### Hash and equality

`EventEnvelope` supports `__hash__` and `__eq__` based on `event_id`,
`event_type`, and `timestamp_ms`. Two envelopes with the same `event_id`
are equal.

## Typed event payloads

Nine typed event payloads cover the full scan lifecycle. Each is a frozen
class with `to_dict()` and `to_json()` methods.

### PlanningEvent

Emitted when operation planning is initiated.

```python
PlanningEvent(operation_id: str, target: str, scope_summary: str)
```

| Field | Type | Description |
|---|---|---|
| `operation_id` | `str` | Operation being planned. |
| `target` | `str` | Target host or URL. |
| `scope_summary` | `str` | Human-readable scope summary. |

### PreflightEvent

Emitted after preflight policy evaluation completes.

```python
PreflightEvent(
    outcome: str,
    confirmations_required: list[str],
    suggested_flags: list[str],
)
```

| Field | Type | Description |
|---|---|---|
| `outcome` | `str` | `"allow"`, `"confirm"`, or `"deny"`. |
| `confirmations_required` | `list[str]` | Confirmation categories needed. |
| `suggested_flags` | `list[str]` | Suggested CLI flags. |

### StageLifecycleEvent

Emitted when a pipeline stage changes state.

```python
StageLifecycleEvent(stage: str, status: str)
```

| Field | Type | Description |
|---|---|---|
| `stage` | `str` | Stage name (e.g. `"recon"`, `"scan"`). |
| `status` | `str` | New status (e.g. `"started"`, `"completed"`, `"failed"`). |

### ProgressEvent

Emitted periodically during long-running operations.

```python
ProgressEvent(
    percentage: float,
    message: str,
    items_processed: int,
    items_total: int,
)
```

| Field | Type | Description |
|---|---|---|
| `percentage` | `float` | Completion percentage (0.0--100.0). |
| `message` | `str` | Human-readable progress message. |
| `items_processed` | `int` | Items processed so far. |
| `items_total` | `int` | Total items to process. |

### FindingEvent

Emitted when a security finding is discovered.

```python
FindingEvent(
    finding_id: str,
    severity: str,
    title: str,
    auto_added: bool,
)
```

| Field | Type | Description |
|---|---|---|
| `finding_id` | `str` | Unique finding identifier. |
| `severity` | `str` | Severity level (e.g. `"high"`, `"medium"`). |
| `title` | `str` | Short finding title. |
| `auto_added` | `bool` | Whether the finding was auto-added to a report. |

### ArtifactEvent

Emitted when an artifact is produced (file, capture, report).

```python
ArtifactEvent(
    artifact_name: str,
    kind: str,
    mime_type: str,
    size_bytes: int,
)
```

| Field | Type | Description |
|---|---|---|
| `artifact_name` | `str` | Artifact file name. |
| `kind` | `str` | Artifact type (e.g. `"pcap"`, `"screenshot"`). |
| `mime_type` | `str` | MIME type. |
| `size_bytes` | `int` | Size in bytes. |

### CancellationEvent

Emitted when an operation is cancelled.

```python
CancellationEvent(reason: str, cancelled_by: str)
```

| Field | Type | Description |
|---|---|---|
| `reason` | `str` | Cancellation reason. |
| `cancelled_by` | `str` | Who or what initiated cancellation. |

### FailureEvent

Emitted when an operation fails.

```python
FailureEvent(error_type: str, error_message: str, is_retryable: bool)
```

| Field | Type | Description |
|---|---|---|
| `error_type` | `str` | Error category (e.g. `"network"`, `"timeout"`). |
| `error_message` | `str` | Human-readable error message. |
| `is_retryable` | `bool` | Whether the operation can be retried. |

### CompletionEvent

Emitted when an operation completes (success or failure).

```python
CompletionEvent(status: str, stats: dict | None, duration_ms: int)
```

| Field | Type | Description |
|---|---|---|
| `status` | `str` | Final status (e.g. `"success"`, `"partial"`, `"failed"`). |
| `stats` | `dict \| None` | Optional aggregate statistics. |
| `duration_ms` | `int` | Total operation duration in milliseconds. |

## EventStream

A push-based event stream with filtering and iteration:

```python
from eggsec import EventStream, EventEnvelope

stream = EventStream()

# Push events
env = EventEnvelope("progress", {"percentage": 10.0})
stream.push(env)

# Filter by type
progress_events = stream.filter_by_type("progress")

# Filter by correlation ID
run_events = stream.filter_by_correlation("run-42")

# Iterate
for event_dict in stream:
    print(event_dict["event_type"])

# Snapshot metadata
print(stream.snapshot())  # {"total_events": 1, "filter_type": None, ...}
```

### Methods

| Method | Returns | Description |
|---|---|---|
| `push(event)` | `None` | Append an `EventEnvelope` to the stream. |
| `len()` | `int` | Number of events (unfiltered). |
| `is_empty()` | `bool` | Whether the stream has no events. |
| `get(i)` | `dict` | Get event at index `i` as a dict. |
| `filter_by_type(type)` | `EventStream` | New stream filtered by event type. |
| `filter_by_correlation(id)` | `EventStream` | New stream filtered by correlation ID. |
| `to_list()` | `list[dict]` | All events as a list of dicts. |
| `latest()` | `dict \| None` | Most recent event, or None. |
| `count()` | `int` | Number of events in the current view. |
| `snapshot()` | `dict` | Metadata about the stream and active filters. |

### Async iteration

`EventStream` supports the async iterator protocol (`async for`).

### Context manager

```python
with EventStream() as stream:
    stream.push(event)
```

### Legacy conversion

```python
from eggsec import event_stream_from_legacy

# Convert a list of ExecutionEvent objects to an EventStream
stream = event_stream_from_legacy(legacy_events)
```

## EVENT_SCHEMA_VERSION

Module-level constant for the current event schema version:

```python
import eggsec
print(eggsec.EVENT_SCHEMA_VERSION)  # "1.0.0"
```

## Backward compatibility

- New event types may be added in future versions.
- Existing event types will not have fields removed or renamed.
- Consumers should ignore unknown `event_type` values gracefully.
- The `schema_version` field allows consumers to detect schema changes.
- `from_legacy()` converts older `ExecutionEvent` objects to versioned
  `EventEnvelope` instances.

## Schema version evolution policy

| Change type | Version bump | Example |
|---|---|---|
| New event type added | Minor | Adding `ComplianceEvent` |
| New field on existing event | Minor | Adding `host` to `ProgressEvent` |
| Field removed or renamed | Major | Removing `auto_added` from `FindingEvent` |
| `schema_version` format change | Major | Changing from `"1.0.0"` to `"2.0.0"` |
