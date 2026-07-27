#!/usr/bin/env python3
"""Custom protocol workflow using TCP and UDP primitives.

Spins up a local TCP echo server and a local UDP server, then uses
eggsec's managed transport sessions (`TcpSession`, `UdpSocket`) to
interact with them. Demonstrates connect, read/write, context managers,
and banner probing — all against loopback.

Requirements:
    - eggsec (default features)
    - EGGSEC_ALLOW_LOOPBACK_FIXTURE=1 environment variable

Usage:
    EGGSEC_ALLOW_LOOPBACK_FIXTURE=1 python3 docs/python/examples/custom_protocol_workflow.py
"""

import socket
import socketserver
import threading
import time

from eggsec.net import (
    TcpSession, TcpConfig, UdpSocket, UdpConfig,
    AsyncTcpSession, AsyncUdpSocket,
    tcp_connect_probe, banner_probe,
)


TCP_PORT = 19878
UDP_PORT = 19879


def _await(future, timeout: float = 5.0):
    """Drive a PyFuture via its iterator protocol until a value arrives.

    PyFuture implements __await__ -> __next__ (polling). This helper
    drives that protocol manually so we do not depend on asyncio.run.
    """
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        try:
            value = next(future)
        except StopIteration as done:
            return done.value
        if value is not None:
            return value
        time.sleep(0.01)
    raise TimeoutError("Async operation did not complete")


class EchoHandler(socketserver.BaseRequestHandler):
    """Simple echo server: sends back whatever it receives."""
    def handle(self):
        try:
            data = self.request.recv(1024)
            if data:
                self.request.sendall(data)
        except Exception:
            pass


class UdpEchoHandler(socketserver.DatagramRequestHandler):
    def handle(self):
        try:
            data = self.rfile.read()
            self.wfile.write(data)
        except Exception:
            pass


def start_tcp_server():
    socketserver.TCPServer.allow_reuse_address = True
    server = socketserver.TCPServer(("127.0.0.1", TCP_PORT), EchoHandler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    time.sleep(0.2)
    return server


def start_udp_server():
    socketserver.UDPServer.allow_reuse_address = True
    server = socketserver.UDPServer(("127.0.0.1", UDP_PORT), UdpEchoHandler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    time.sleep(0.2)
    return server


def demo_tcp_connect_probe():
    """One-shot TCP connect probe — measures latency without sending data."""
    result = tcp_connect_probe("127.0.0.1", TCP_PORT, timeout_ms=3000)
    print(f"TCP connect probe: remote={result.remote_endpoint}")
    print(f"  connect_time={result.timing.tcp_connect_ms:.1f}ms")
    print(f"  total={result.timing.total_ms:.1f}ms")


def demo_banner_probe():
    """Banner probe — connects and reads the initial response."""
    result = banner_probe("127.0.0.1", TCP_PORT, timeout_ms=3000)
    if result.banner_text:
        print(f"Banner: {result.banner_text.strip()!r}")
    elif result.banner_bytes:
        print(f"Banner (raw): {result.banner_bytes!r}")
    else:
        print("Banner: (no banner — server waits for client)")
    print(f"  elapsed={result.timing.total_ms:.1f}ms")


def demo_tcp_session():
    """Managed TCP session via the async API (avoids sync write_all quirks)."""
    config = TcpConfig(host="127.0.0.1", port=TCP_PORT, connect_timeout_ms=3000)
    session = AsyncTcpSession(config)
    try:
        connect_result = _await(session.connect())
        print(f"TCP session connected: {connect_result.remote_endpoint}")

        write_result = _await(session.write_all(b"hello eggsec"))
        print(f"  sent {write_result.bytes_written} bytes in {write_result.duration_ms:.1f}ms")

        read_result = _await(session.read(max_bytes=1024))
        print(f"  received: {read_result.data!r}")
        print(f"  bytes_sent={session.bytes_sent}, bytes_recv={session.bytes_received}")
    finally:
        session.close()

    print(f"  session closed={session.is_closed}")


def demo_udp_session():
    """Managed UDP socket — send datagram, receive response."""
    config = UdpConfig(host="127.0.0.1", port=UDP_PORT, timeout_ms=3000)
    sock = AsyncUdpSocket(config)
    try:
        _await(sock.connect())
        send_result = _await(sock.send(b"ping udp"))
        print(f"UDP sent {send_result.bytes_sent} bytes in {send_result.duration_ms:.1f}ms")

        recv_result = _await(sock.recv())
        print(f"UDP recv: {recv_result.data!r} ({recv_result.bytes_received} bytes)")
    finally:
        sock.close()

    print(f"  socket closed={sock.is_closed}")


def main():
    tcp_server = start_tcp_server()
    udp_server = start_udp_server()
    try:
        demo_tcp_connect_probe()
        print()
        demo_banner_probe()
        print()
        demo_tcp_session()
        print()
        demo_udp_session()
    finally:
        tcp_server.shutdown()
        tcp_server.server_close()
        udp_server.shutdown()
        udp_server.server_close()


if __name__ == "__main__":
    main()
