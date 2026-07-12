# Callbacks and Sinks

The callback system provides push-based notification of scan events,
findings, artifacts, and progress. All sinks isolate errors so that
a failing callback never crashes the engine or propagates a Python
exception into Rust code.

## Sink types

Five sink types cover distinct event categories. Each wraps a Python
callable and invokes it with a single argument (a dict or typed object).

### AuditSink

Receives enforcement audit events as dicts.

```python
from eggsec import AuditSink, EnforcementAuditEvent

def on_audit(event: dict):
    print(f"Audit: {event['operation_id']} -> {event['outcome']}")

sink = AuditSink(on_audit)
sink.send(audit_event)  # invokes on_audit(event_dict)
```

| Method | Description |
|---|---|
| `send(event)` | Invoke the callback with an audit event dict. |
| `close()` | Stop accepting events. Further `send()` calls are no-ops. |
| `is_closed` | Property: whether the sink has been closed. |

### FindingSink

Receives findings as they are discovered.

```python
from eggsec import FindingSink

def on_finding(finding):
    print(f"Found: {finding['title']} ({finding['severity']})")

sink = FindingSink(on_finding)
```

### ArtifactSink

Receives artifact metadata when artifacts are produced.

```python
from eggsec import ArtifactSink

def on_artifact(artifact):
    print(f"Artifact: {artifact['name']} ({artifact['mime_type']})")

sink = ArtifactSink(on_artifact)
```

### ProgressSink

Receives progress updates with percentage and message.

```python
from eggsec import ProgressSink

def on_progress(percentage: float, message: str):
    bar = "#" * int(percentage / 5)
    print(f"\r[{bar:<20}] {percentage:.0f}% {message}", end="")

sink = ProgressSink(on_progress)
sink.send(50.0, "Scanning ports...")
```

### EventConsumer

Receives versioned `EventEnvelope` dicts.

```python
from eggsec import EventConsumer

def on_event(event_dict):
    print(f"Event: {event_dict['event_type']} @ {event_dict['timestamp_ms']}")

sink = EventConsumer(on_event)
```

## AsyncCallback

Wraps an `async def` handler for use from Rust callbacks:

```python
from eggsec import AsyncCallback

async def handle_event(event_dict):
    await process(event_dict)

cb = AsyncCallback(handle_event)
result = cb.invoke(event)  # returns a coroutine/object
```

| Method | Description |
|---|---|
| `invoke(event)` | Call the async handler with an `EventEnvelope` dict. Returns the coroutine result. |
| `close()` | Stop accepting invocations. |
| `is_closed` | Property: whether the callback has been closed. |

## CallbackScheduler

Queues callbacks for bounded delivery. Prevents unbounded memory growth
when events arrive faster than the consumer can process them.

```python
from eggsec import CallbackScheduler, EventEnvelope

scheduler = CallbackScheduler(capacity=1000)

# Enqueue events (returns False if queue is full)
ok = scheduler.enqueue(event)

# Drain all queued events for processing
events = scheduler.drain()

# Check queue state
print(scheduler.pending())   # number of queued events
print(scheduler.is_closed)   # False
```

| Method | Returns | Description |
|---|---|---|
| `enqueue(event)` | `bool` | Add an event. Returns `False` if queue is full or closed. |
| `drain()` | `list[EventEnvelope]` | Remove and return all queued events. |
| `pending()` | `int` | Number of events currently queued. |
| `close()` | `None` | Prevent further enqueuing. |
| `is_closed` | `bool` | Whether the scheduler is closed. |

## BackpressureChannel

A bounded, in-process channel that drops the oldest event when full.
Suitable for high-throughput scenarios where occasional data loss is
acceptable.

```python
from eggsec import BackpressureChannel, EventEnvelope

channel = BackpressureChannel(capacity=256)

channel.send(event)
received = channel.try_recv()  # EventEnvelope or None

print(channel.len())          # buffered count
print(channel.total_dropped)  # events dropped due to backpressure
print(channel.capacity)       # 256
```

| Method | Returns | Description |
|---|---|---|
| `send(event)` | `None` | Send an event. Drops oldest if full. |
| `try_recv()` | `EventEnvelope \| None` | Receive an event, or None if empty. |
| `len()` | `int` | Number of events currently buffered. |
| `is_empty()` | `bool` | Whether the channel is empty. |
| `total_dropped` | `int` | Total events dropped due to backpressure. |
| `capacity` | `int` | Maximum channel capacity. |

Supports `len()` via `__len__`.

## Error isolation

All sinks catch exceptions raised by the Python callback and log them
via `tracing::warn!`. The exception is never propagated to the caller.
This ensures:

- A buggy callback does not crash the scan engine.
- Rust code never sees a Python exception it cannot handle.
- The sink remains operational after a callback error.

## GIL behavior

Sinks acquire the GIL only during callback invocation. Between calls,
the GIL is released so other Python threads can run. This means:

- Multiple sinks can be active concurrently.
- Long-running scans do not block the Python main thread.
- Thread safety is handled by `Py<PyAny>` internally.

## Shutdown and close semantics

Every sink type has a `close()` method and an `is_closed` property:

- After `close()`, `send()` / `invoke()` / `enqueue()` become no-ops
  or return `False`.
- `close()` is idempotent -- calling it multiple times is safe.
- Sinks do not auto-close when the Python GC collects them; explicitly
  call `close()` for deterministic cleanup.

```python
# Typical shutdown pattern
sink = FindingSink(handler)
try:
    scan(sink)
finally:
    sink.close()
```

## Combining sinks with EventStream

Sinks and streams are complementary. Use `EventStream` for buffered,
filterable iteration, and sinks for push-based delivery:

```python
from eggsec import EventStream, EventConsumer, FindingSink

stream = EventStream()
consumer = EventConsumer(lambda e: stream.push(e))
finding_sink = FindingSink(lambda f: print(f"Finding: {f['title']}"))

# Wire both into a scan pipeline
# ... scan produces events ...
# Then iterate the stream
for event in stream:
    print(event["event_type"])
```
