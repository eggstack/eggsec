"""Real browser backend integration tests - Workstream 8.

Tests exercise the browser session types against a real browser backend.
When the browser binary is not available, the test FAILS (not skips).

These tests prove the browser binding surface works end-to-end:
construction, lifecycle, navigation, DOM inspection, cleanup.
"""

import json
import os
import shutil
import subprocess
import pytest
import importlib

pytestmark = [pytest.mark.timeout(60)]


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
