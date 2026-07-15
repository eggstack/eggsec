#!/usr/bin/env python3
"""Local versus daemon execution.

Demonstrates running the same port scan through local Engine and
daemon-backed execution paths.

Requirements:
    - eggsec with daemon-client feature for daemon path
    - Running daemon for daemon path (optional)

Usage:
    python3 docs/python/examples/local_vs_daemon_execution.py [target]
"""

import sys

import eggsec
from eggsec import Engine, Scope, PortScanRequest


def main():
    target = sys.argv[1] if len(sys.argv) > 1 else "127.0.0.1"

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
                client = daemon_connect("/tmp/eggsec-daemon.sock")
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
