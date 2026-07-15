#!/usr/bin/env python3
"""Daemon remote cancellation.

Demonstrates CancellationToken for local cancellation and
CancellationRequest/CancellationResult for daemon remote cancellation.

Requirements:
    - eggsec with daemon-client feature

Usage:
    python3 docs/python/examples/daemon_remote_cancellation.py
"""

import time

import eggsec
from eggsec import CancellationToken, CancellationRequest, CancellationResult


def main():
    token = CancellationToken()
    print(f"Token created: {token}")

    token.cancel(reason="demo timeout")
    print(f"Token cancelled: {token.is_cancelled}")
    print(f"Reason: {token.reason()}")

    req = CancellationRequest(
        session_id="s-1",
        task_id="t-1",
        reason="operator abort",
        force=True,
    )
    print(f"Request: session={req.session_id}, task={req.task_id}, force={req.force}")

    result = CancellationResult(
        acknowledged=True,
        task_was_running=True,
        task_was_completed=False,
        cleanup_started=True,
    )
    print(f"Result: ack={result.acknowledged}, cleanup={result.cleanup_started}")


if __name__ == "__main__":
    main()
