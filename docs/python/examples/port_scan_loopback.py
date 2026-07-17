#!/usr/bin/env python3
"""Port scanning against a local loopback TCP server.

Spins up a TCP server on 127.0.0.1 in a background thread, scans it
with both sync and async engines, and prints the results. No public
internet access required.

Requirements:
    - eggsec (default features)
    - EGGSEC_ALLOW_LOOPBACK_FIXTURE=1 environment variable

Usage:
    EGGSEC_ALLOW_LOOPBACK_FIXTURE=1 python3 docs/python/examples/port_scan_loopback.py
"""

import socketserver
import threading
import time

import eggsec
from eggsec import Engine, Scope, AsyncEngine, PortScanRequest


LISTEN_PORT = 19876


class QuietHandler(socketserver.BaseRequestHandler):
    def handle(self):
        self.request.sendall(b"OK\n")


def start_server():
    server = socketserver.TCPServer(("127.0.0.1", LISTEN_PORT), QuietHandler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    time.sleep(0.3)
    return server


def run_sync():
    scope = Scope.allow_hosts(["127.0.0.1"])
    engine = Engine(scope)
    req = PortScanRequest("127.0.0.1", ports=str(LISTEN_PORT))
    result = engine.run_port_scan(req)
    print(f"Sync  : status={result.status.name()}")
    if result.payload:
        for port_info in result.payload.get("open_ports", []):
            print(f"  port {port_info.get('port')}: open")


async def run_async():
    scope = Scope.allow_hosts(["127.0.0.1"])
    engine = AsyncEngine(scope)
    req = PortScanRequest("127.0.0.1", ports=str(LISTEN_PORT))
    result = await engine.run_port_scan(req)
    print(f"Async : status={result.status.name()}")
    if result.payload:
        for port_info in result.payload.get("open_ports", []):
            print(f"  port {port_info.get('port')}: open")


def main():
    server = start_server()
    try:
        run_sync()
        import asyncio
        asyncio.run(run_async())
    finally:
        server.shutdown()


if __name__ == "__main__":
    main()
