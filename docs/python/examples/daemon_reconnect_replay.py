#!/usr/bin/env python3
"""Daemon reconnect and replay.

Demonstrates ReconnectOptions, ReplayCursor, and EventReplayInfo
types for daemon reconnection and event replay scenarios.

Requirements:
    - eggsec with daemon-client feature

Usage:
    python3 docs/python/examples/daemon_reconnect_replay.py
"""

import eggsec
from eggsec import (
    DaemonProtocolVersion,
    ReconnectOptions,
    ReplayCursor,
    EventReplayInfo,
)


def main():
    version = DaemonProtocolVersion(
        api_schema_version=1,
        operation_registry_id="eggsec-ops",
        feature_profile="default",
    )
    print(f"Protocol version: {version}")

    opts = ReconnectOptions(
        max_retries=5,
        retry_delay_ms=500,
        backoff_multiplier=2.0,
        replay_from_sequence=42,
    )
    print(f"Reconnect: max_retries={opts.max_retries}, backoff={opts.backoff_multiplier}x")

    cursor = ReplayCursor(
        session_id="s-1",
        last_sequence=100,
        total_events=100,
        gap_count=0,
        duplicate_count=3,
        timestamp_ms=1700000000000,
    )
    print(f"Replayed {cursor.total_events} events, {cursor.duplicate_count} duplicates skipped")
    print(f"Last sequence: {cursor.last_sequence}, gaps: {cursor.gap_count}")

    info = EventReplayInfo(
        session_id="s-1",
        from_sequence=1,
        to_sequence=100,
        event_count=100,
        ordered=True,
    )
    print(f"Replay info: {info}")


if __name__ == "__main__":
    main()
