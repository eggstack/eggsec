"""Managed local services for stable-core integration tests.

The fixture deliberately uses only Python's standard library. Each service is
bound to loopback and has an explicit readiness check; a startup failure is a
test failure, never a skip.
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
FIXTURE_BANNER = b"EGGSEC-FIXTURE/1.0\\r\\n"


class _ThreadingTcpServer(socketserver.ThreadingTCPServer):
    allow_reuse_address = True
    daemon_threads = True


class _TcpHandler(socketserver.BaseRequestHandler):
    def handle(self) -> None:
        server = self.server
        server.hits.append(self.client_address)
        if server.delay:
            time.sleep(server.delay)
        self.request.settimeout(1.0)
        try:
            self.request.recv(4096)
        except OSError:
            pass
        self.request.sendall(FIXTURE_BANNER)


class _HttpServer(http.server.ThreadingHTTPServer):
    allow_reuse_address = True
    daemon_threads = True

    def __init__(self, address, handler):
        super().__init__(address, handler)
        self.request_log: list[dict[str, str]] = []
        self.log_lock = threading.Lock()


class _HttpHandler(http.server.BaseHTTPRequestHandler):
    server: _HttpServer

    def log_message(self, *_args) -> None:
        # The test owns request logging; avoid nondeterministic stderr output.
        return

    def do_HEAD(self) -> None:
        self._respond(head_only=True)

    def do_GET(self) -> None:
        self._respond(head_only=False)

    def _respond(self, *, head_only: bool) -> None:
        parsed = urlsplit(self.path)
        path = parsed.path
        query = parsed.query
        with self.server.log_lock:
            self.server.request_log.append({"method": self.command, "path": path, "query": query})

        status = 200
        headers = {
            "Content-Type": "text/plain; charset=utf-8",
            "Server": "EggsecFixture/1.0",
            "X-Eggsec-Fixture": "stable-core",
        }
        body = "EGGSEC_FIXTURE_ROOT"
        if path == "/admin":
            body = "EGGSEC_FIXTURE_ADMIN"
        elif path == "/missing":
            status, body = 404, "EGGSEC_FIXTURE_MISSING"
        elif path == "/redirect-local":
            status, body = 302, "redirecting"
            headers["Location"] = "/admin"
        elif path == "/redirect-external":
            status, body = 302, "external redirect blocked"
            headers["Location"] = "http://192.0.2.1/fixture-external"
        elif path == "/slow":
            time.sleep(0.2)
            body = "EGGSEC_FIXTURE_SLOW"
        elif path == "/echo":
            headers["Content-Type"] = "application/json"
            body = json.dumps({"method": self.command, "path": path, "query": query}, sort_keys=True)
        elif path == "/waf-clean":
            body = "EGGSEC_FIXTURE_CLEAN"
        elif path == "/waf-block":
            status, body = 403, "Access Denied by EggsecFixture WAF"
            headers["Server"] = "EggsecFixtureWAF/1.0"
            headers["X-Blocked-By"] = "EggsecFixtureWAF"
        elif path.startswith("/fuzz/"):
            value = path.removeprefix("/fuzz/")
            body = json.dumps({"classification": "blocked" if "script" in value.lower() else "clean", "value": value})
        elif path == "/load":
            body = "OK"
        elif path != "/":
            status, body = 404, "EGGSEC_FIXTURE_UNKNOWN"

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
                server.hits.append(self.client_address)
                stream.settimeout(2.0)
                try:
                    stream.recv(4096)
                except OSError:
                    pass
                stream.sendall(b"HTTP/1.1 200 OK\\r\\nContent-Length: 2\\r\\nConnection: close\\r\\n\\r\\nOK")
        except (OSError, ssl.SSLError):
            # Handshake failures are recorded by the client and do not make
            # server shutdown unsafe.
            return


class _TlsServer(socketserver.ThreadingTCPServer):
    allow_reuse_address = True
    daemon_threads = True

    def __init__(self, address, certfile: Path, keyfile: Path):
        super().__init__(address, _TlsHandler)
        self.hits: list[tuple[str, int]] = []
        self.context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        self.context.load_cert_chain(certfile=str(certfile), keyfile=str(keyfile))


class StableCoreFixtures:
    """Own loopback TCP, HTTP, and TLS fixtures for one test scope."""

    def __init__(self) -> None:
        self.tcp: _ThreadingTcpServer | None = None
        self.http: _HttpServer | None = None
        self.tls: _TlsServer | None = None
        self._threads: list[threading.Thread] = []
        self.closed_port = 0
        self._previous_fixture_env: str | None = None

    @staticmethod
    def _closed_port() -> int:
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as probe:
            probe.bind((HOST, 0))
            return int(probe.getsockname()[1])

    @staticmethod
    def _serve(server) -> threading.Thread:
        thread = threading.Thread(target=server.serve_forever, name="eggsec-fixture", daemon=True)
        thread.start()
        # Binding happens in the constructor. A short readiness probe confirms
        # that the serving thread is alive before the extension is invoked.
        deadline = time.monotonic() + 2.0
        while time.monotonic() < deadline:
            if thread.is_alive():
                return thread
            time.sleep(0.01)
        raise RuntimeError("fixture service failed to become ready")

    def __enter__(self) -> "StableCoreFixtures":
        self._previous_fixture_env = os.environ.get("EGGSEC_ALLOW_LOOPBACK_FIXTURE")
        os.environ["EGGSEC_ALLOW_LOOPBACK_FIXTURE"] = "1"
        self.tcp = _ThreadingTcpServer((HOST, 0), _TcpHandler)
        self.tcp.hits = []
        self.tcp.delay = 0.0
        self.http = _HttpServer((HOST, 0), _HttpHandler)
        fixture_dir = Path(__file__).parent
        self.tls = _TlsServer((HOST, 0), fixture_dir / "fixture-cert.pem", fixture_dir / "fixture-key.pem")
        self.closed_port = self._closed_port()
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
    def http_url(self) -> str:
        return f"http://{HOST}:{self.http_port}"

    @property
    def tls_url(self) -> str:
        return f"https://{HOST}:{self.tls_port}"

    @property
    def http_requests(self) -> list[dict[str, str]]:
        assert self.http is not None
        with self.http.log_lock:
            return list(self.http.request_log)
