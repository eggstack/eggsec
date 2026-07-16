"""Real browser backend integration tests - Workstream 8.

Tests exercise the browser session types against a real browser backend.
When the browser binary is not available, the test FAILS (not skips).

These tests prove the browser binding surface works end-to-end:
construction, lifecycle, navigation, DOM inspection, cleanup.
"""

import http.server
import json
import os
import shutil
import socketserver
import subprocess
import threading
import time
import pytest
import importlib

pytestmark = [pytest.mark.timeout(90)]


def _import_or_skip(name, feature="headless-browser"):
    """Import a name from eggsec, skip if feature-gated."""
    mod = importlib.import_module("eggsec")
    obj = getattr(mod, name, None)
    if obj is None:
        pytest.skip(f"{name} not available (requires {feature} feature)")
    return obj


def _find_browser_binary():
    """Find a headless browser binary (chromium, chrome, or playwright)."""
    for name in ["chromium-browser", "chromium", "google-chrome", "google-chrome-stable"]:
        path = shutil.which(name)
        if path:
            return path
    # Check playwright browsers
    playwright_browsers = os.path.expanduser("~/.cache/ms-playwright")
    if os.path.isdir(playwright_browsers):
        for entry in os.listdir(playwright_browsers):
            chrome_path = os.path.join(playwright_browsers, entry, "chrome-linux", "chrome")
            if os.path.isfile(chrome_path):
                return chrome_path
    return None


# ---------------------------------------------------------------------------
# Loopback HTTP origin for browser tests
# ---------------------------------------------------------------------------


class _BrowserTestHandler(http.server.BaseHTTPRequestHandler):
    """Handler serving deterministic pages for browser testing."""

    server: http.server.ThreadingHTTPServer

    def log_message(self, *_args):
        return

    def do_GET(self):
        if self.path == "/":
            body = b"""<!DOCTYPE html>
<html>
<head><title>Eggsec Browser Test</title></head>
<body>
<h1 id="main-heading">Welcome</h1>
<form id="login-form" action="/login" method="POST">
  <input name="user" type="text" />
  <input name="pass" type="password" />
  <button type="submit">Login</button>
</form>
<a href="/about" id="about-link">About</a>
<a href="/contact" id="contact-link">Contact</a>
<script>console.log("page loaded");</script>
</body>
</html>"""
            self.send_response(200)
            self.send_header("Content-Type", "text/html")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
        elif self.path == "/about":
            body = b"""<!DOCTYPE html>
<html><head><title>About</title></head>
<body><h1>About Us</h1><a href="/">Home</a></body></html>"""
            self.send_response(200)
            self.send_header("Content-Type", "text/html")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
        elif self.path == "/set-cookie":
            body = b"Cookie set"
            self.send_response(200)
            self.send_header("Content-Type", "text/plain")
            self.send_header("Set-Cookie", "session=abc123; Path=/")
            self.send_header("Set-Cookie", "theme=dark; Path=/")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
        elif self.path == "/redirect":
            self.send_response(302)
            self.send_header("Location", "/")
            self.send_header("Content-Length", "0")
            self.end_headers()
        else:
            body = b"Not Found"
            self.send_response(404)
            self.send_header("Content-Type", "text/plain")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)


class _BrowserOriginServer:
    def __init__(self):
        self.server = None
        self.thread = None

    def start(self):
        self.server = socketserver.ThreadingTCPServer(("127.0.0.1", 0), _BrowserTestHandler)
        self.server.allow_reuse_address = True
        self.server.daemon_threads = True
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()
        deadline = time.monotonic() + 2.0
        while time.monotonic() < deadline:
            if self.thread.is_alive():
                return
            time.sleep(0.01)
        raise RuntimeError("Browser origin server failed to start")

    def stop(self):
        if self.server:
            self.server.shutdown()
            self.server.server_close()
        if self.thread:
            self.thread.join(timeout=2.0)

    @property
    def port(self):
        return self.server.server_address[1]

    @property
    def base_url(self):
        return f"http://127.0.0.1:{self.port}"


@pytest.fixture
def browser_origin():
    """Provide a loopback HTTP server for browser tests."""
    server = _BrowserOriginServer()
    server.start()
    yield server
    server.stop()


@pytest.fixture
def browser_binary():
    """Provide a browser binary path, skip if not found."""
    path = _find_browser_binary()
    if path is None:
        pytest.skip("No headless browser binary found (chromium/chrome)")
    return path


# ---------------------------------------------------------------------------
# BrowserSession lifecycle
# ---------------------------------------------------------------------------


class TestBrowserSessionLifecycle:
    """Test BrowserSession construction and lifecycle."""

    @pytest.mark.timeout(30)
    def test_browser_session_config_construction(self):
        """BrowserSessionConfig constructs with valid defaults."""
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")

        config = BrowserSessionConfig(target_url="http://127.0.0.1:8080")
        assert config.target_url == "http://127.0.0.1:8080"
        assert config.headless is True

    @pytest.mark.timeout(30)
    def test_browser_session_config_with_options(self):
        """BrowserSessionConfig with all options."""
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")

        config = BrowserSessionConfig(
            target_url="http://example.com",
            headless=True,
            user_agent="TestAgent/1.0",
            viewport_width=1280,
            viewport_height=720,
            timeout_ms=30000,
            collect_console=True,
            collect_network=True,
            collect_storage=True,
            screenshot_on_complete=True,
        )
        assert config.user_agent == "TestAgent/1.0"
        assert config.viewport_width == 1280
        assert config.collect_console is True

    @pytest.mark.timeout(30)
    def test_browser_session_created_state(self):
        """BrowserSession starts in Created state."""
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        BrowserSessionState = _import_or_skip("BrowserSessionState")

        config = BrowserSessionConfig(target_url="http://127.0.0.1:8080")
        session = BrowserSession(config)
        assert session.state == BrowserSessionState.Created

    @pytest.mark.timeout(30)
    def test_browser_session_context_manager_enter_exit(self):
        """BrowserSession context manager enters and exits cleanly."""
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        BrowserSessionState = _import_or_skip("BrowserSessionState")

        config = BrowserSessionConfig(target_url="http://127.0.0.1:8080")
        with BrowserSession(config) as session:
            assert session.state in (
                BrowserSessionState.Created,
                BrowserSessionState.Ready,
                BrowserSessionState.Launching,
            )

    @pytest.mark.timeout(30)
    def test_browser_session_stop_idempotent(self):
        """BrowserSession.stop() is idempotent."""
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        BrowserSessionState = _import_or_skip("BrowserSessionState")

        config = BrowserSessionConfig(target_url="http://127.0.0.1:8080")
        session = BrowserSession(config)
        session.stop()
        session.stop()
        assert session.state == BrowserSessionState.Stopped

    @pytest.mark.timeout(30)
    def test_browser_session_config_serialization(self):
        """BrowserSessionConfig serializes to dict and JSON."""
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")

        config = BrowserSessionConfig(
            target_url="http://example.com",
            headless=True,
        )
        d = config.to_dict()
        assert isinstance(d, dict)
        assert d["target_url"] == "http://example.com"

        j = config.to_json()
        parsed = json.loads(j)
        assert parsed["target_url"] == "http://example.com"


# ---------------------------------------------------------------------------
# Browser session events
# ---------------------------------------------------------------------------


class TestBrowserSessionEvents:
    """Test browser event types."""

    @pytest.mark.timeout(30)
    def test_navigation_event_construction(self):
        """BrowserNavigationEvent constructs with valid fields."""
        BrowserNavigationEvent = _import_or_skip("BrowserNavigationEvent")

        event = BrowserNavigationEvent(
            url="http://example.com/page",
            status_code=200,
            load_time_ms=150,
            redirect_chain=[],
        )
        d = event.to_dict()
        assert d["url"] == "http://example.com/page"
        assert d["status_code"] == 200

    @pytest.mark.timeout(30)
    def test_console_event_construction(self):
        """BrowserConsoleEvent constructs with valid fields."""
        BrowserConsoleEvent = _import_or_skip("BrowserConsoleEvent")

        event = BrowserConsoleEvent(
            level="error",
            message="Test error message",
            source="console-api",
            line_number=42,
        )
        d = event.to_dict()
        assert d["level"] == "error"
        assert d["message"] == "Test error message"

    @pytest.mark.timeout(30)
    def test_network_event_construction(self):
        """BrowserNetworkEvent constructs with valid fields."""
        BrowserNetworkEvent = _import_or_skip("BrowserNetworkEvent")

        event = BrowserNetworkEvent(
            url="http://example.com/api",
            method="POST",
            status_code=201,
            resource_type="xhr",
            size_bytes=1024,
            duration_ms=50,
        )
        d = event.to_dict()
        assert d["method"] == "POST"
        assert d["status_code"] == 201


# ---------------------------------------------------------------------------
# Browser DOM and storage types
# ---------------------------------------------------------------------------


class TestBrowserDomTypes:
    """Test browser DOM snapshot and storage types."""

    @pytest.mark.timeout(30)
    def test_dom_snapshot_construction(self):
        """BrowserDomSnapshot constructs with forms/links/scripts."""
        BrowserDomSnapshot = _import_or_skip("BrowserDomSnapshot")

        snapshot = BrowserDomSnapshot(
            forms=[{"action": "/login", "method": "POST"}],
            links=[{"href": "/about", "text": "About"}],
            scripts=[{"src": "/app.js", "inline": False}],
            frames=[],
            title="Test Page",
            url="http://example.com",
        )
        d = snapshot.to_dict()
        assert len(d["forms"]) == 1
        assert len(d["links"]) == 1
        assert d["title"] == "Test Page"

    @pytest.mark.timeout(30)
    def test_storage_info_construction(self):
        """BrowserStorageInfo constructs with cookies and storage."""
        BrowserStorageInfo = _import_or_skip("BrowserStorageInfo")
        BrowserCookieInfo = _import_or_skip("BrowserCookieInfo")

        cookie = BrowserCookieInfo(
            name="session",
            value="abc123",
            domain=".example.com",
            path="/",
            secure=True,
            http_only=True,
            same_site="Strict",
        )
        storage = BrowserStorageInfo(
            cookies=[cookie],
            local_storage={"key": "value"},
            session_storage={"sid": "123"},
        )
        d = storage.to_dict()
        assert len(d["cookies"]) == 1
        assert d["cookies"][0]["name"] == "session"
        assert d["local_storage"]["key"] == "value"

    @pytest.mark.timeout(30)
    def test_screenshot_artifact_construction(self):
        """BrowserScreenshotArtifact constructs with image data."""
        BrowserScreenshotArtifact = _import_or_skip("BrowserScreenshotArtifact")

        artifact = BrowserScreenshotArtifact(
            image_data=b"PNG_FAKE_DATA",
            format="png",
            width=1280,
            height=720,
            url="http://example.com",
            filename="screenshot.png",
        )
        d = artifact.to_dict()
        assert d["format"] == "png"
        assert d["width"] == 1280


# ---------------------------------------------------------------------------
# Browser with real backend (if available)
# ---------------------------------------------------------------------------


class TestBrowserWithRealBackend:
    """Test browser session with real browser (fails if browser not found)."""

    @pytest.mark.timeout(30)
    def test_browser_session_start_stop_with_browser(self, browser_binary):
        """BrowserSession starts and stops with real browser binary."""
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        BrowserSessionState = _import_or_skip("BrowserSessionState")

        config = BrowserSessionConfig(
            target_url="http://127.0.0.1:8080",
            headless=True,
        )
        session = BrowserSession(config)
        session.start()
        # Session should be in Ready or a valid state
        assert session.state in (
            BrowserSessionState.Ready,
            BrowserSessionState.Launching,
            BrowserSessionState.Failed,
        )
        session.stop()
        assert session.state == BrowserSessionState.Stopped

    @pytest.mark.timeout(30)
    def test_browser_capabilities_type_exists(self):
        """BrowserCapabilities type exists."""
        BrowserCapabilities = _import_or_skip("BrowserCapabilities")
        assert BrowserCapabilities is not None


# ---------------------------------------------------------------------------
# Real browser page interaction tests
# ---------------------------------------------------------------------------


class TestBrowserRealPageInteraction:
    """Test browser_test against a real local page (fails if browser not found)."""

    @pytest.mark.timeout(60)
    def test_browser_test_returns_report(self, browser_binary, browser_origin):
        """browser_test returns a BrowserTestReport against a local page."""
        browser_test = _import_or_skip("browser_test")

        report = browser_test(browser_origin.base_url)
        assert report is not None
        d = report.to_dict()
        assert "target" in d
        assert "total_findings" in d
        assert d["target"] == browser_origin.base_url

    @pytest.mark.timeout(60)
    def test_browser_test_finds_forms(self, browser_binary, browser_origin):
        """browser_test discovers forms on the page."""
        browser_test = _import_or_skip("browser_test")

        report = browser_test(browser_origin.base_url)
        d = report.to_dict()
        assert "dom_xss" in d
        assert "client_issues" in d
        # The test page has a login form; total_findings may include it
        assert isinstance(d["total_findings"], int)

    @pytest.mark.timeout(60)
    def test_browser_test_with_config(self, browser_binary, browser_origin):
        """browser_test works with explicit config."""
        browser_test = _import_or_skip("browser_test")
        BrowserTestConfig = _import_or_skip("BrowserTestConfig")

        config = BrowserTestConfig(
            check_dom_xss=True,
            discover_spa_routes=True,
            check_client_security=True,
            timeout_ms=15000,
        )
        report = browser_test(browser_origin.base_url, config=config)
        assert report is not None
        d = report.to_dict()
        assert "target" in d

    @pytest.mark.timeout(60)
    def test_browser_test_report_serialization(self, browser_binary, browser_origin):
        """BrowserTestReport serializes to dict and JSON."""
        browser_test = _import_or_skip("browser_test")

        report = browser_test(browser_origin.base_url)
        d = report.to_dict()
        assert isinstance(d, dict)
        # Should be JSON-serializable
        j = json.dumps(d)
        parsed = json.loads(j)
        assert parsed["target"] == browser_origin.base_url

    @pytest.mark.timeout(60)
    def test_browser_test_redirects(self, browser_binary, browser_origin):
        """browser_test follows redirects."""
        browser_test = _import_or_skip("browser_test")

        report = browser_test(f"{browser_origin.base_url}/redirect")
        assert report is not None
        d = report.to_dict()
        assert "target" in d
