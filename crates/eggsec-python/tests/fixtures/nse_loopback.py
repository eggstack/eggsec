"""Loopback fixture servers for NSE integration tests.

Provides TCP, HTTP, and TLS servers on high ports for NSE scripts that
need real services. NSE scripts receive the port via script_args (e.g.,
``port=18080``). A startup failure is a test failure, never a skip.
"""

from __future__ import annotations

import http.server
import json
import os
import socket
import socketserver
import ssl
import threading
import time
from pathlib import Path
from urllib.parse import urlsplit


HOST = "127.0.0.1"


class _ThreadingTcpServer(socketserver.ThreadingTCPServer):
    allow_reuse_address = True
    daemon_threads = True


class _BannerHandler(socketserver.BaseRequestHandler):
    """TCP handler that reads a request and returns a banner."""

    def handle(self) -> None:
        self.request.settimeout(2.0)
        try:
            self.request.recv(4096)
        except OSError:
            pass
        self.request.sendall(b"EGGSEC-NSE-FIXTURE/1.0\r\n")


class _HttpServer(http.server.ThreadingHTTPServer):
    allow_reuse_address = True
    daemon_threads = True


class _HttpHandler(http.server.BaseHTTPRequestHandler):
    server: _HttpServer

    def log_message(self, *_args) -> None:
        return

    def do_HEAD(self) -> None:
        self._respond(head_only=True)

    def do_GET(self) -> None:
        self._respond(head_only=False)

    def _respond(self, *, head_only: bool) -> None:
        parsed = urlsplit(self.path)
        path = parsed.path

        status = 200
        headers = {
            "Content-Type": "text/html; charset=utf-8",
            "Server": "EggsecNseFixture/1.0",
            "X-Eggsec-NSE": "true",
        }
        body = "<html><head><title>Eggsec NSE Fixture</title></head><body>OK</body></html>"
        if path == "/admin":
            body = "<html><head><title>Admin</title></head><body>Admin Page</body></html>"
        elif path == "/missing":
            status, body = 404, "<html><body>Not Found</body></html>"
        elif path == "/echo":
            headers["Content-Type"] = "application/json"
            body = json.dumps({"path": path, "method": self.command})
        elif path != "/":
            status, body = 404, "<html><body>Unknown</body></html>"

        payload = body.encode("utf-8")
        headers["Content-Length"] = str(len(payload))
        self.send_response(status)
        for key, value in headers.items():
            self.send_header(key, value)
        self.end_headers()
        if not head_only:
            self.wfile.write(payload)


class _TlsHandler(socketserver.BaseRequestHandler):
    def handle(self) -> None:
        server = self.server
        try:
            with server.context.wrap_socket(self.request, server_side=True) as stream:
                stream.settimeout(2.0)
                try:
                    stream.recv(4096)
                except OSError:
                    pass
                stream.sendall(
                    b"HTTP/1.1 200 OK\r\n"
                    b"Content-Length: 2\r\n"
                    b"Connection: close\r\n"
                    b"\r\n"
                    b"OK"
                )
        except (OSError, ssl.SSLError):
            return


class _TlsServer(socketserver.ThreadingTCPServer):
    allow_reuse_address = True
    daemon_threads = True

    def __init__(self, address, certfile: Path, keyfile: Path):
        super().__init__(address, _TlsHandler)
        self.context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        self.context.load_cert_chain(certfile=str(certfile), keyfile=str(keyfile))


class NseLoopbackFixtures:
    """Loopback TCP, HTTP, and TLS fixtures for NSE integration tests.

    Each service binds to a high port on 127.0.0.1. The NSE scripts
    receive the port via ``script_args="port=<port>"``.
    """

    def __init__(self) -> None:
        self.tcp: _ThreadingTcpServer | None = None
        self.http: _HttpServer | None = None
        self.tls: _TlsServer | None = None
        self._threads: list[threading.Thread] = []
        self._previous_fixture_env: str | None = None

    @staticmethod
    def _serve(server) -> threading.Thread:
        thread = threading.Thread(target=server.serve_forever, name="nse-fixture", daemon=True)
        thread.start()
        deadline = time.monotonic() + 2.0
        while time.monotonic() < deadline:
            if thread.is_alive():
                return thread
            time.sleep(0.01)
        raise RuntimeError("NSE fixture service failed to become ready")

    def __enter__(self) -> "NseLoopbackFixtures":
        self._previous_fixture_env = os.environ.get("EGGSEC_ALLOW_LOOPBACK_FIXTURE")
        os.environ["EGGSEC_ALLOW_LOOPBACK_FIXTURE"] = "1"
        self.tcp = _ThreadingTcpServer((HOST, 0), _BannerHandler)
        self.http = _HttpServer((HOST, 0), _HttpHandler)
        fixture_dir = Path(__file__).parent
        self.tls = _TlsServer((HOST, 0), fixture_dir / "fixture-cert.pem", fixture_dir / "fixture-key.pem")
        self._threads = [self._serve(self.tcp), self._serve(self.http), self._serve(self.tls)]
        return self

    def __exit__(self, *_exc) -> None:
        for server in (self.tcp, self.http, self.tls):
            if server is not None:
                server.shutdown()
                server.server_close()
        for thread in self._threads:
            thread.join(timeout=2.0)
        self._threads.clear()
        self.tcp = None
        self.http = None
        self.tls = None
        if self._previous_fixture_env is None:
            os.environ.pop("EGGSEC_ALLOW_LOOPBACK_FIXTURE", None)
        else:
            os.environ["EGGSEC_ALLOW_LOOPBACK_FIXTURE"] = self._previous_fixture_env

    @property
    def tcp_port(self) -> int:
        assert self.tcp is not None
        return int(self.tcp.server_address[1])

    @property
    def http_port(self) -> int:
        assert self.http is not None
        return int(self.http.server_address[1])

    @property
    def tls_port(self) -> int:
        assert self.tls is not None
        return int(self.tls.server_address[1])

    @property
    def tcp_args(self) -> str:
        """Script args string for connecting to the TCP fixture."""
        return f"port={self.tcp_port}"

    @property
    def http_args(self) -> str:
        """Script args string for connecting to the HTTP fixture."""
        return f"port={self.http_port}"

    @property
    def tls_args(self) -> str:
        """Script args string for connecting to the TLS fixture."""
        return f"port={self.tls_port}"
