#!/usr/bin/env python3
"""Event streaming, progress sinks, and callback contracts.

Demonstrates subscribing to scan events via `EventConsumer`,
receiving progress updates via `ProgressSink`, and collecting
findings with `FindingSink`. Shows both context-manager and
manual lifecycle patterns.

Requirements:
    - eggsec (default features)

Usage:
    python3 docs/python/examples/event_streaming_progress.py
"""

import eggsec
from eggsec import (
    EventConsumer,
    ProgressSink,
    FindingSink,
    AuditSink,
    EventStream,
    Engine,
    Scope,
    PortScanRequest,
)


def demo_event_consumer():
    """Subscribe to all events emitted during a scan."""
    collected = []

    def on_event(event):
        collected.append(event)

    with EventConsumer(on_event) as consumer:
        print(f"EventConsumer: is_closed={consumer.is_closed}")

        scope = Scope.allow_hosts(["127.0.0.1"])
        engine = Engine(scope)
        req = PortScanRequest("127.0.0.1", ports="22")
        result = engine.run_port_scan(req)

    print(f"  events received: {len(collected)}")
    for ev in collected[:3]:
        ev_type = type(ev).__name__ if hasattr(ev, "__class__") else type(ev)
        print(f"    {ev_type}: {str(ev)[:80]}")
    if len(collected) > 3:
        print(f"    ... and {len(collected) - 3} more")


def demo_progress_sink():
    """Track progress percentage during a scan."""
    updates = []

    def on_progress(percentage, message):
        updates.append((percentage, message))

    with ProgressSink(on_progress) as sink:
        scope = Scope.allow_hosts(["127.0.0.1"])
        engine = Engine(scope)
        req = PortScanRequest("127.0.0.1", ports="22")
        result = engine.run_port_scan(req)

    print(f"Progress updates: {len(updates)}")
    for pct, msg in updates[-3:]:
        print(f"  {pct:.0f}%: {msg}")


def demo_finding_sink():
    """Collect findings emitted during a scan."""
    findings = []

    def on_finding(finding):
        findings.append(finding)

    with FindingSink(on_finding) as sink:
        scope = Scope.allow_hosts(["127.0.0.1"])
        engine = Engine(scope)
        req = PortScanRequest("127.0.0.1", ports="22")
        result = engine.run_port_scan(req)

    print(f"Findings collected: {len(findings)}")
    for f in findings[:3]:
        print(f"  {f}")


def demo_audit_sink():
    """Collect audit events from the enforcement gate."""
    events = []

    def on_audit(event):
        events.append(event)

    with AuditSink(on_audit) as sink:
        scope = Scope.allow_hosts(["127.0.0.1"])
        engine = Engine(scope)
        req = PortScanRequest("127.0.0.1", ports="22")
        result = engine.run_port_scan(req)

    print(f"Audit events: {len(events)}")
    for ev in events[:3]:
        print(f"  {ev}")


def demo_event_stream():
    """EventStream push/filter/snapshot API."""
    stream = EventStream.empty()
    print(f"Empty stream: len={len(stream)}, is_empty={stream.is_empty()}")

    # Push synthetic events
    for i in range(5):
        stream.push({
            "schema_version": "1.0",
            "event_id": f"evt-{i}",
            "sequence": i,
            "timestamp": "2026-01-01T00:00:00Z",
            "event_type": "progress" if i % 2 == 0 else "finding",
            "payload": {"progress": i * 20, "message": f"step {i}"},
        })

    print(f"After push: len={len(stream)}")
    latest = stream.latest()
    print(f"  latest event_type={latest.get('event_type') if latest else None}")

    progress_events = stream.filter_by_type("progress")
    print(f"  progress events: {progress_events.count()}")

    snapshot = stream.snapshot()
    print(f"  snapshot keys: {sorted(snapshot.keys())}")


def main():
    print("=== Event Consumer ===")
    demo_event_consumer()
    print()

    print("=== Progress Sink ===")
    demo_progress_sink()
    print()

    print("=== Finding Sink ===")
    demo_finding_sink()
    print()

    print("=== Audit Sink ===")
    demo_audit_sink()
    print()

    print("=== Event Stream ===")
    demo_event_stream()


if __name__ == "__main__":
    main()
