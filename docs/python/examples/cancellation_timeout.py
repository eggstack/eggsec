#!/usr/bin/env python3
"""Cancellation tokens and timeout handling.

Demonstrates creating and using CancellationToken for cooperative
cancellation and Engine timeout configuration. All logic is
in-memory with a loopback scan.

Requirements:
    - eggsec (default features)

Usage:
    python3 docs/python/examples/cancellation_timeout.py
"""

import time
import threading

import eggsec
from eggsec import CancellationToken, Engine, Scope, PortScanRequest


def main():
    # -- Cancellation token basics --
    token = CancellationToken()
    print(f"Initial state: cancelled={token.is_cancelled()}")

    token.cancel("user requested abort")
    print(f"After cancel: cancelled={token.is_cancelled()}, reason={token.reason()}")

    # Serialization round-trip
    d = token.to_dict()
    print(f"Serialized: {d}")

    # -- Engine timeout handling --
    scope = Scope.allow_hosts(["127.0.0.1"])
    engine = Engine(scope)

    # Scan with a short timeout using a typed request
    req = PortScanRequest("127.0.0.1", ports="19877")
    result = engine.run_port_scan(req)
    print(f"\nScan result: status={result.status.name()}")

    # -- Pipeline cancellation with a background thread --
    pipeline_token = CancellationToken()

    def cancel_after(delay):
        time.sleep(delay)
        pipeline_token.cancel("watchdog triggered")

    watcher = threading.Thread(target=cancel_after, args=(0.5,), daemon=True)
    watcher.start()

    print(f"\nToken before watcher: cancelled={pipeline_token.is_cancelled()}")
    time.sleep(0.6)
    print(f"Token after watcher:  cancelled={pipeline_token.is_cancelled()}, "
          f"reason={pipeline_token.reason()}")


if __name__ == "__main__":
    main()
