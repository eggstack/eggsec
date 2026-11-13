#!/usr/bin/env python3
"""Local versus daemon execution.

Demonstrates running the same port scan through local Engine and
daemon-backed execution paths.

Requirements:
    - eggsec with daemon-client feature for daemon path
    - Running daemon for daemon path (optional)

Usage:
    python3 docs/python/examples/local_vs_daemon_execution.py [target]

The daemon's socket path comes from ``EGGSEC_DAEMON_SOCKET`` when it is set,
and from the daemon's built-in default otherwise. The default is stated once,
in ``crates/eggsec-runtime/src/daemon_endpoint.rs``; the Python bindings
carry no copy of it, so this example reads the same environment variable the
daemon binary reads. Pass a path as the second argument to override both.
"""

import os
import sys

import eggsec
from eggsec import Engine, Scope, PortScanRequest


def main():
    target = sys.argv[1] if len(sys.argv) > 1 else "127.0.0.1"
    # The daemon's socket path: the operator's override, else the daemon's
    # own default. The default lives in the daemon's canonical crate, so the
    # example asks the daemon rather than carrying a copy of it.
    socket_path = (
        sys.argv[2]
        if len(sys.argv) > 2
        else os.environ.get("EGGSEC_DAEMON_SOCKET", "").strip()
    )

    scope = Scope.allow_hosts([target])
    request = PortScanRequest(target=target, ports=[22, 80, 443])

    engine = Engine(scope)
    result = engine.run_port_scan(request)
    print(f"Local: status={result.status}, findings={len(result.payload.findings) if result.payload else 0}")

    features = eggsec.features()
    if features.get("daemon-client", False):
        try:
            from eggsec import daemon_connect
            import asyncio

            async def daemon_run():
                if not socket_path:
                    print(
                        "No socket path: set EGGSEC_DAEMON_SOCKET or pass one as the "
                        "second argument (the daemon's own default is not duplicated here)."
                    )
                    return
                client = daemon_connect(socket_path)
                health = await eggsec.async_daemon_health(client)
                print(f"Daemon health: {health}")
                client.close()

            asyncio.run(daemon_run())
        except Exception as e:
            print(f"Daemon not available: {e}")
    else:
        print("Daemon client feature not enabled")


if __name__ == "__main__":
    main()
