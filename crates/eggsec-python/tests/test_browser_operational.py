"""Operational proof tests for browser session types (WS6).

Tests prove the browser binding surface is real: construction, serialization,
state transitions, and attribute access all work without external browser
infrastructure. Session methods that require a real browser engine return
stub/error responses which we test for correctness.
"""

import json
import importlib
import pytest


def _import_or_skip(name, feature="headless-browser"):
    """Import a name from eggsec, skip test if unavailable (feature-gated)."""
    mod = importlib.import_module("eggsec")
    obj = getattr(mod, name, None)
    if obj is None:
        pytest.skip(f"{name} not available (requires {feature} feature)")
    return obj


# Module-level timeout for all tests
pytestmark = [pytest.mark.timeout(60)]


# ============================================================================
# 1. TestBrowserCapabilities
# ============================================================================
class TestBrowserCapabilities:
    """BrowserCapabilities is not directly constructable.
    Test import and verify the type exists.
    """

    def test_import(self):
        BrowserCapabilities = _import_or_skip("BrowserCapabilities")
        assert BrowserCapabilities is not None


# ============================================================================
# 2. TestBrowserSessionState
# ============================================================================
class TestBrowserSessionState:
    EXPECTED_VARIANTS = [
        "Created", "Discovering", "Launching", "Ready", "Navigating",
        "Loading", "Inspecting", "Stopping", "Cleaning", "Stopped",
        "Failed", "Cancelled",
    ]

    def test_import(self):
        BrowserSessionState = _import_or_skip("BrowserSessionState")
        assert BrowserSessionState is not None

    def test_all_variants_exist(self):
        BrowserSessionState = _import_or_skip("BrowserSessionState")
        for name in self.EXPECTED_VARIANTS:
            assert hasattr(BrowserSessionState, name), f"Missing variant: {name}"

    def test_repr(self):
        BrowserSessionState = _import_or_skip("BrowserSessionState")
        assert repr(BrowserSessionState.Created) == "BrowserSessionState.Created"

    def test_str(self):
        BrowserSessionState = _import_or_skip("BrowserSessionState")
        assert str(BrowserSessionState.Created) == "Created"
        assert str(BrowserSessionState.Ready) == "Ready"
        assert str(BrowserSessionState.Failed) == "Failed"

    def test_equality(self):
        BrowserSessionState = _import_or_skip("BrowserSessionState")
        assert BrowserSessionState.Created == BrowserSessionState.Created
        assert BrowserSessionState.Created != BrowserSessionState.Ready

    def test_frozen(self):
        BrowserSessionState = _import_or_skip("BrowserSessionState")
        # Enum instances are frozen - can't set attributes on instances
        created = BrowserSessionState.Created
        with pytest.raises(AttributeError):
            created.x = "hack"

    def test_12_variants(self):
        BrowserSessionState = _import_or_skip("BrowserSessionState")
        expected = [
            "Created", "Discovering", "Launching", "Ready", "Navigating",
            "Loading", "Inspecting", "Stopping", "Cleaning", "Stopped",
            "Failed", "Cancelled",
        ]
        for name in expected:
            assert hasattr(BrowserSessionState, name), f"Missing: {name}"


# ============================================================================
# 3. TestBrowserSessionConfig
# ============================================================================
class TestBrowserSessionConfig:
    def test_import(self):
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        assert BrowserSessionConfig is not None

    def test_construct_defaults(self):
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig()
        assert cfg.headless is True
        assert cfg.viewport_width == 1280
        assert cfg.viewport_height == 720
        assert cfg.timeout_ms == 30000
        assert cfg.navigation_timeout_ms == 60000
        assert cfg.collect_console is True
        assert cfg.collect_network is True
        assert cfg.collect_cookies is True
        assert cfg.collect_storage is True
        assert cfg.screenshot_on_complete is False
        assert cfg.ignore_cert_errors is False
        assert cfg.target_url is None
        assert cfg.proxy is None
        assert cfg.user_agent is None

    def test_construct_full(self):
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig(
            target_url="https://example.com",
            headless=False,
            proxy="http://127.0.0.1:8080",
            user_agent="Mozilla/5.0 TestBot",
            viewport_width=1920,
            viewport_height=1080,
            timeout_ms=60000,
            navigation_timeout_ms=120000,
            collect_console=False,
            collect_network=False,
            collect_cookies=False,
            collect_storage=False,
            screenshot_on_complete=True,
            extra_headers=["X-Custom: test"],
            ignore_cert_errors=True,
        )
        assert cfg.target_url == "https://example.com"
        assert cfg.headless is False
        assert cfg.proxy == "http://127.0.0.1:8080"
        assert cfg.user_agent == "Mozilla/5.0 TestBot"
        assert cfg.viewport_width == 1920
        assert cfg.viewport_height == 1080
        assert cfg.timeout_ms == 60000
        assert cfg.navigation_timeout_ms == 120000
        assert cfg.collect_console is False
        assert cfg.collect_network is False
        assert cfg.collect_cookies is False
        assert cfg.collect_storage is False
        assert cfg.screenshot_on_complete is True
        assert cfg.extra_headers == ["X-Custom: test"]
        assert cfg.ignore_cert_errors is True

    def test_to_dict(self):
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig(headless=False, viewport_width=800)
        d = cfg.to_dict()
        assert d["headless"] is False
        assert d["viewport_width"] == 800
        assert d["viewport_height"] == 720
        assert d["extra_headers"] == []

    def test_to_json_roundtrip(self):
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig(
            target_url="https://test.example",
            headless=False,
            timeout_ms=45000,
        )
        j = cfg.to_json()
        data = json.loads(j)
        assert data["target_url"] == "https://test.example"
        assert data["headless"] is False
        assert data["timeout_ms"] == 45000

    def test_repr(self):
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig(headless=False, viewport_width=1920, viewport_height=1080, timeout_ms=5000)
        r = repr(cfg)
        assert "BrowserSessionConfig" in r
        assert "headless=" in r
        assert "1920x1080" in r
        assert "5000ms" in r

    def test_frozen(self):
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig()
        with pytest.raises(AttributeError):
            cfg.headless = False

    def test_extra_headers_property(self):
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig(extra_headers=["A: 1", "B: 2"])
        assert cfg.extra_headers == ["A: 1", "B: 2"]


# ============================================================================
# 4. TestBrowserSessionStats
# ============================================================================
class TestBrowserSessionStats:
    def test_import(self):
        BrowserSessionStats = _import_or_skip("BrowserSessionStats")
        assert BrowserSessionStats is not None

    def test_stats_from_session(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        stats = session.stats
        assert stats.pages_navigated == 0
        assert stats.dom_snapshots == 0
        assert stats.console_events == 0
        assert stats.network_requests == 0
        assert stats.cookies_collected == 0
        assert stats.screenshots_taken == 0
        assert stats.artifacts_collected == 0
        assert stats.duration_ms == 0

    def test_stats_to_dict(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        d = session.stats.to_dict()
        assert "pages_navigated" in d
        assert "dom_snapshots" in d
        assert "console_events" in d
        assert "network_requests" in d
        assert "cookies_collected" in d
        assert "screenshots_taken" in d
        assert "artifacts_collected" in d
        assert "duration_ms" in d

    def test_stats_to_json(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        j = session.stats.to_json()
        data = json.loads(j)
        assert data["pages_navigated"] == 0
        assert data["dom_snapshots"] == 0

    def test_stats_repr(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        r = repr(session.stats)
        assert "BrowserSessionStats" in r
        assert "pages=" in r
        assert "duration=" in r


# ============================================================================
# 5. TestBrowserNavigationEvent
# ============================================================================
class TestBrowserNavigationEvent:
    def test_import(self):
        BrowserNavigationEvent = _import_or_skip("BrowserNavigationEvent")
        assert BrowserNavigationEvent is not None

    def test_event_from_navigate(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        session.start()
        event = session.navigate("https://example.com")
        assert event.url == "https://example.com"
        assert event.final_url == "https://example.com"
        assert event.status_code == 0
        assert event.load_time_ms == 0
        assert event.timestamp_ms > 0
        assert isinstance(event.redirect_chain, list)

    def test_event_to_dict(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        session.start()
        event = session.navigate("https://test.example")
        d = event.to_dict()
        assert d["url"] == "https://test.example"
        assert d["final_url"] == "https://test.example"
        assert d["status_code"] == 0
        assert d["load_time_ms"] == 0
        assert "redirect_chain" in d
        assert "timestamp_ms" in d

    def test_event_to_json(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        session.start()
        event = session.navigate("https://json.test")
        j = event.to_json()
        data = json.loads(j)
        assert data["url"] == "https://json.test"
        assert data["status_code"] == 0

    def test_event_repr(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        session.start()
        event = session.navigate("https://repr.test")
        r = repr(event)
        assert "BrowserNavigationEvent" in r
        assert "https://repr.test" in r
        assert "status=" in r
        assert "load_time=" in r


# ============================================================================
# 6. TestBrowserConsoleEvent
# ============================================================================
class TestBrowserConsoleEvent:
    def test_import(self):
        BrowserConsoleEvent = _import_or_skip("BrowserConsoleEvent")
        assert BrowserConsoleEvent is not None

    def test_console_events_initially_empty(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        events = session.get_console_events()
        assert events == []


# ============================================================================
# 7. TestBrowserNetworkEvent
# ============================================================================
class TestBrowserNetworkEvent:
    def test_import(self):
        BrowserNetworkEvent = _import_or_skip("BrowserNetworkEvent")
        assert BrowserNetworkEvent is not None

    def test_network_events_initially_empty(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        events = session.get_network_events()
        assert events == []


# ============================================================================
# 8. TestBrowserDomSnapshot
# ============================================================================
class TestBrowserDomSnapshot:
    def test_import(self):
        BrowserDomSnapshot = _import_or_skip("BrowserDomSnapshot")
        assert BrowserDomSnapshot is not None

    def test_snapshot_from_session(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        snap = session.get_dom_snapshot()
        assert snap.url == ""
        assert snap.title is None
        assert snap.forms == []
        assert snap.links == []
        assert snap.scripts == []
        assert snap.frames == []
        assert snap.timestamp_ms > 0

    def test_snapshot_to_dict(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        snap = session.get_dom_snapshot()
        d = snap.to_dict()
        assert "url" in d
        assert "title" in d
        assert "forms" in d
        assert "links" in d
        assert "scripts" in d
        assert "frames" in d
        assert "timestamp_ms" in d

    def test_snapshot_to_json(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        snap = session.get_dom_snapshot()
        j = snap.to_json()
        data = json.loads(j)
        assert "url" in data
        assert "forms" in data
        assert "links" in data
        assert "scripts" in data

    def test_snapshot_repr(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        snap = session.get_dom_snapshot()
        r = repr(snap)
        assert "BrowserDomSnapshot" in r
        assert "forms=" in r
        assert "links=" in r
        assert "scripts=" in r

    def test_snapshot_increments_stats(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        assert session.stats.dom_snapshots == 0
        session.get_dom_snapshot()
        assert session.stats.dom_snapshots == 1
        session.get_dom_snapshot()
        assert session.stats.dom_snapshots == 2


# ============================================================================
# 9. TestBrowserFormInfo
# ============================================================================
class TestBrowserFormInfo:
    def test_import(self):
        BrowserFormInfo = _import_or_skip("BrowserFormInfo")
        assert BrowserFormInfo is not None


# ============================================================================
# 10. TestBrowserFormField
# ============================================================================
class TestBrowserFormField:
    def test_import(self):
        BrowserFormField = _import_or_skip("BrowserFormField")
        assert BrowserFormField is not None


# ============================================================================
# 11. TestBrowserLinkInfo
# ============================================================================
class TestBrowserLinkInfo:
    def test_import(self):
        BrowserLinkInfo = _import_or_skip("BrowserLinkInfo")
        assert BrowserLinkInfo is not None


# ============================================================================
# 12. TestBrowserStorageInfo
# ============================================================================
class TestBrowserStorageInfo:
    def test_import(self):
        BrowserStorageInfo = _import_or_skip("BrowserStorageInfo")
        assert BrowserStorageInfo is not None

    def test_storage_from_session(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        storage = session.get_cookies()
        assert storage.local_storage == []
        assert storage.session_storage == []
        assert storage.cookies == []

    def test_storage_to_dict(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        storage = session.get_cookies()
        d = storage.to_dict()
        assert "local_storage" in d
        assert "session_storage" in d
        assert "cookies" in d

    def test_storage_to_json(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        storage = session.get_cookies()
        j = storage.to_json()
        data = json.loads(j)
        assert data["local_storage"] == []
        assert data["session_storage"] == []
        assert data["cookies"] == []


# ============================================================================
# 13. TestBrowserCookieInfo
# ============================================================================
class TestBrowserCookieInfo:
    def test_import(self):
        BrowserCookieInfo = _import_or_skip("BrowserCookieInfo")
        assert BrowserCookieInfo is not None


# ============================================================================
# 14. TestBrowserSessionConstruction
# ============================================================================
class TestBrowserSessionConstruction:
    def test_import(self):
        BrowserSession = _import_or_skip("BrowserSession")
        assert BrowserSession is not None

    def test_construct_default(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        assert session.session_id.startswith("browser-")
        assert session.session_id != ""

    def test_initial_state_created(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        BrowserSessionState = _import_or_skip("BrowserSessionState")
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        assert session.state == BrowserSessionState.Created

    def test_start_transitions_to_ready(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        BrowserSessionState = _import_or_skip("BrowserSessionState")
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        session.start()
        assert session.state == BrowserSessionState.Ready

    def test_stop_transitions_to_stopped(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        BrowserSessionState = _import_or_skip("BrowserSessionState")
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        session.start()
        session.stop()
        assert session.state == BrowserSessionState.Stopped

    def test_stop_from_created(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        BrowserSessionState = _import_or_skip("BrowserSessionState")
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        session.stop()
        assert session.state == BrowserSessionState.Stopped

    def test_config_getter(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig(headless=False, viewport_width=800)
        session = BrowserSession(cfg)
        got = session.config
        assert got.headless is False
        assert got.viewport_width == 800

    def test_to_dict(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig(headless=False)
        session = BrowserSession(cfg)
        d = session.to_dict()
        assert "session_id" in d
        assert "state" in d
        assert "config" in d
        assert "stats" in d
        assert d["state"] == "Created"

    def test_to_json(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        j = session.to_json()
        data = json.loads(j)
        assert "session_id" in data
        assert "state" in data
        assert "pages_navigated" in data

    def test_repr(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig(headless=True)
        session = BrowserSession(cfg)
        r = repr(session)
        assert "BrowserSession" in r
        assert session.session_id in r
        assert "Created" in r

    def test_str(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        s = str(session)
        assert "BrowserSession" in s
        assert "Created" in s

    def test_context_manager(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        BrowserSessionState = _import_or_skip("BrowserSessionState")
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        session.start()
        with session as s:
            assert s.state == BrowserSessionState.Ready
        assert session.state == BrowserSessionState.Stopped

    def test_navigate_after_start(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        BrowserSessionState = _import_or_skip("BrowserSessionState")
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        session.start()
        event = session.navigate("https://example.com")
        assert event.url == "https://example.com"
        assert session.state == BrowserSessionState.Ready
        assert session.stats.pages_navigated == 1

    def test_navigate_fails_before_start(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        with pytest.raises(Exception, match="Cannot navigate"):
            session.navigate("https://example.com")

    def test_screenshot_after_start(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        session.start()
        artifact = session.take_screenshot()
        assert artifact.artifact_id.startswith("screenshot-")
        assert session.stats.screenshots_taken == 1

    def test_execute_script_fails_without_engine(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        session.start()
        with pytest.raises(Exception, match="requires an active browser engine"):
            session.execute_script("return document.title")

    def test_wait_for_selector_fails_without_engine(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        session.start()
        with pytest.raises(Exception, match="requires an active browser engine"):
            session.wait_for_selector("#app")

    def test_multiple_navigations(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        session.start()
        for i in range(5):
            event = session.navigate(f"https://example.com/page/{i}")
            assert event.url == f"https://example.com/page/{i}"
        assert session.stats.pages_navigated == 5


# ============================================================================
# 15. TestBrowserEventSerialization
# ============================================================================
class TestBrowserEventSerialization:
    def test_navigation_event_json_roundtrip(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        session.start()
        event = session.navigate("https://json.test")
        j = event.to_json()
        data = json.loads(j)
        assert data["url"] == "https://json.test"
        assert isinstance(data["timestamp_ms"], int)
        assert isinstance(data["redirect_chain"], list)

    def test_dom_snapshot_json_roundtrip(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        snap = session.get_dom_snapshot()
        j = snap.to_json()
        data = json.loads(j)
        assert isinstance(data["forms"], list)
        assert isinstance(data["links"], list)
        assert isinstance(data["scripts"], list)

    def test_storage_json_roundtrip(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        storage = session.get_cookies()
        j = storage.to_json()
        data = json.loads(j)
        assert isinstance(data["cookies"], list)
        assert isinstance(data["local_storage"], list)


# ============================================================================
# 16. TestAsyncBrowserSession
# ============================================================================
class TestAsyncBrowserSession:
    def test_import(self):
        AsyncBrowserSession = _import_or_skip("AsyncBrowserSession")
        assert AsyncBrowserSession is not None

    def test_construct(self):
        AsyncBrowserSession = _import_or_skip("AsyncBrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig()
        session = AsyncBrowserSession(cfg)
        assert session.session_id.startswith("async-browser-")

    def test_initial_state_created(self):
        AsyncBrowserSession = _import_or_skip("AsyncBrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        BrowserSessionState = _import_or_skip("BrowserSessionState")
        cfg = BrowserSessionConfig()
        session = AsyncBrowserSession(cfg)
        assert session.state == BrowserSessionState.Created

    def test_to_dict(self):
        AsyncBrowserSession = _import_or_skip("AsyncBrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig(headless=False)
        session = AsyncBrowserSession(cfg)
        d = session.to_dict()
        assert "session_id" in d
        assert "state" in d
        assert "config" in d
        assert "stats" in d

    def test_to_json(self):
        AsyncBrowserSession = _import_or_skip("AsyncBrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig()
        session = AsyncBrowserSession(cfg)
        j = session.to_json()
        data = json.loads(j)
        assert "session_id" in data

    def test_repr(self):
        AsyncBrowserSession = _import_or_skip("AsyncBrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig(headless=True)
        session = AsyncBrowserSession(cfg)
        r = repr(session)
        assert "AsyncBrowserSession" in r
        assert "headless=" in r

    def test_str(self):
        AsyncBrowserSession = _import_or_skip("AsyncBrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig()
        session = AsyncBrowserSession(cfg)
        s = str(session)
        assert "AsyncBrowserSession" in s

    def test_config_getter(self):
        AsyncBrowserSession = _import_or_skip("AsyncBrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig(viewport_width=800)
        session = AsyncBrowserSession(cfg)
        assert session.config.viewport_width == 800

    def test_stats_getter(self):
        AsyncBrowserSession = _import_or_skip("AsyncBrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig()
        session = AsyncBrowserSession(cfg)
        stats = session.stats
        assert stats.pages_navigated == 0
        assert stats.dom_snapshots == 0

    def test_async_context_manager(self):
        AsyncBrowserSession = _import_or_skip("AsyncBrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig()
        session = AsyncBrowserSession(cfg)
        # Test __aenter__ returns self
        enter_result = session.__aenter__()
        assert enter_result.session_id == session.session_id
        # Test __aexit__ returns False (no suppression)
        exit_result = session.__aexit__(None, None, None)
        assert exit_result is False


# ============================================================================
# 17. TestBrowserDomEvent
# ============================================================================
class TestBrowserDomEvent:
    def test_import(self):
        BrowserDomEvent = _import_or_skip("BrowserDomEvent")
        assert BrowserDomEvent is not None

    def test_construct(self):
        BrowserDomEvent = _import_or_skip("BrowserDomEvent")
        evt = BrowserDomEvent(
            event_id="evt-1",
            session_id="sess-1",
            event_type="mutation",
            sequence=1,
            timestamp_ms=1000,
            url="https://example.com",
        )
        assert evt.event_id == "evt-1"
        assert evt.session_id == "sess-1"
        assert evt.event_type == "mutation"
        assert evt.sequence == 1
        assert evt.timestamp_ms == 1000
        assert evt.url == "https://example.com"

    def test_construct_with_optionals(self):
        BrowserDomEvent = _import_or_skip("BrowserDomEvent")
        evt = BrowserDomEvent(
            event_id="e1",
            session_id="s1",
            event_type="attr_change",
            sequence=2,
            timestamp_ms=2000,
            url="https://example.com/page",
            selector="#app",
            attribute_name="class",
            old_value="hidden",
            new_value="visible",
            element_tag="div",
        )
        assert evt.selector == "#app"
        assert evt.attribute_name == "class"
        assert evt.old_value == "hidden"
        assert evt.new_value == "visible"
        assert evt.element_tag == "div"

    def test_to_dict(self):
        BrowserDomEvent = _import_or_skip("BrowserDomEvent")
        evt = BrowserDomEvent(
            event_id="e1", session_id="s1", event_type="mutation",
            sequence=1, timestamp_ms=1000, url="https://example.com",
        )
        d = evt.to_dict()
        assert d["event_id"] == "e1"
        assert d["event_type"] == "mutation"
        assert d["sequence"] == 1
        assert d["selector"] is None
        assert d["element_tag"] is None

    def test_to_json(self):
        BrowserDomEvent = _import_or_skip("BrowserDomEvent")
        evt = BrowserDomEvent(
            event_id="e1", session_id="s1", event_type="mutation",
            sequence=1, timestamp_ms=1000, url="https://example.com",
        )
        j = evt.to_json()
        data = json.loads(j)
        assert data["event_id"] == "e1"
        assert data["event_type"] == "mutation"

    def test_repr(self):
        BrowserDomEvent = _import_or_skip("BrowserDomEvent")
        evt = BrowserDomEvent(
            event_id="e1", session_id="s1", event_type="mutation",
            sequence=1, timestamp_ms=1000, url="https://example.com",
        )
        r = repr(evt)
        assert "BrowserDomEvent" in r
        assert "e1" in r
        assert "mutation" in r
        assert "seq=1" in r


# ============================================================================
# 18. TestBrowserDownloadEvent
# ============================================================================
class TestBrowserDownloadEvent:
    def test_import(self):
        BrowserDownloadEvent = _import_or_skip("BrowserDownloadEvent")
        assert BrowserDownloadEvent is not None

    def test_construct(self):
        BrowserDownloadEvent = _import_or_skip("BrowserDownloadEvent")
        evt = BrowserDownloadEvent(
            event_id="dl-1",
            session_id="sess-1",
            sequence=1,
            timestamp_ms=5000,
            url="https://example.com/file.zip",
        )
        assert evt.event_id == "dl-1"
        assert evt.url == "https://example.com/file.zip"
        assert evt.status == "completed"

    def test_construct_with_optionals(self):
        BrowserDownloadEvent = _import_or_skip("BrowserDownloadEvent")
        evt = BrowserDownloadEvent(
            event_id="dl-2",
            session_id="sess-1",
            sequence=2,
            timestamp_ms=6000,
            url="https://example.com/data.bin",
            suggested_filename="data.bin",
            content_type="application/octet-stream",
            size_bytes=1024,
            download_path="/tmp/data.bin",
            status="completed",
        )
        assert evt.suggested_filename == "data.bin"
        assert evt.content_type == "application/octet-stream"
        assert evt.size_bytes == 1024
        assert evt.download_path == "/tmp/data.bin"

    def test_to_dict(self):
        BrowserDownloadEvent = _import_or_skip("BrowserDownloadEvent")
        evt = BrowserDownloadEvent(
            event_id="dl-1", session_id="s1", sequence=1,
            timestamp_ms=5000, url="https://example.com/f.zip",
        )
        d = evt.to_dict()
        assert d["event_id"] == "dl-1"
        assert d["status"] == "completed"
        assert d["suggested_filename"] is None
        assert d["content_type"] is None

    def test_to_json(self):
        BrowserDownloadEvent = _import_or_skip("BrowserDownloadEvent")
        evt = BrowserDownloadEvent(
            event_id="dl-1", session_id="s1", sequence=1,
            timestamp_ms=5000, url="https://example.com/f.zip",
        )
        j = evt.to_json()
        data = json.loads(j)
        assert data["event_id"] == "dl-1"

    def test_repr(self):
        BrowserDownloadEvent = _import_or_skip("BrowserDownloadEvent")
        evt = BrowserDownloadEvent(
            event_id="dl-1", session_id="s1", sequence=1,
            timestamp_ms=5000, url="https://example.com/f.zip",
        )
        r = repr(evt)
        assert "BrowserDownloadEvent" in r
        assert "dl-1" in r
        assert "https://example.com/f.zip" in r


# ============================================================================
# 19. TestBrowserSecurityObservation
# ============================================================================
class TestBrowserSecurityObservation:
    def test_import(self):
        BrowserSecurityObservation = _import_or_skip("BrowserSecurityObservation")
        assert BrowserSecurityObservation is not None

    def test_construct(self):
        BrowserSecurityObservation = _import_or_skip("BrowserSecurityObservation")
        obs = BrowserSecurityObservation(
            observation_id="obs-1",
            session_id="sess-1",
            sequence=1,
            timestamp_ms=3000,
            observation_type="missing_header",
            url="https://example.com",
            severity="medium",
            description="Missing X-Content-Type-Options header",
        )
        assert obs.observation_id == "obs-1"
        assert obs.observation_type == "missing_header"
        assert obs.severity == "medium"
        assert obs.header_name is None
        assert obs.tls_version is None

    def test_construct_with_optionals(self):
        BrowserSecurityObservation = _import_or_skip("BrowserSecurityObservation")
        obs = BrowserSecurityObservation(
            observation_id="obs-2",
            session_id="sess-1",
            sequence=2,
            timestamp_ms=4000,
            observation_type="weak_tls",
            url="https://example.com",
            severity="high",
            description="TLS 1.0 detected",
            header_name="Server",
            header_value="nginx/1.14",
            tls_version="TLSv1.0",
            certificate_info="CN=example.com, valid until 2025-12-31",
        )
        assert obs.header_name == "Server"
        assert obs.header_value == "nginx/1.14"
        assert obs.tls_version == "TLSv1.0"
        assert obs.certificate_info is not None

    def test_to_dict(self):
        BrowserSecurityObservation = _import_or_skip("BrowserSecurityObservation")
        obs = BrowserSecurityObservation(
            observation_id="obs-1", session_id="s1", sequence=1,
            timestamp_ms=3000, observation_type="missing_header",
            url="https://example.com", severity="low",
            description="test",
        )
        d = obs.to_dict()
        assert d["observation_id"] == "obs-1"
        assert d["severity"] == "low"
        assert d["header_name"] is None
        assert d["tls_version"] is None

    def test_to_json(self):
        BrowserSecurityObservation = _import_or_skip("BrowserSecurityObservation")
        obs = BrowserSecurityObservation(
            observation_id="obs-1", session_id="s1", sequence=1,
            timestamp_ms=3000, observation_type="header_check",
            url="https://example.com", severity="info",
            description="test",
        )
        j = obs.to_json()
        data = json.loads(j)
        assert data["observation_id"] == "obs-1"
        assert data["observation_type"] == "header_check"

    def test_repr(self):
        BrowserSecurityObservation = _import_or_skip("BrowserSecurityObservation")
        obs = BrowserSecurityObservation(
            observation_id="obs-1", session_id="s1", sequence=1,
            timestamp_ms=3000, observation_type="missing_header",
            url="https://example.com", severity="medium",
            description="test",
        )
        r = repr(obs)
        assert "BrowserSecurityObservation" in r
        assert "obs-1" in r
        assert "missing_header" in r
        assert "medium" in r


# ============================================================================
# 20. TestBrowserSessionFrozen
# ============================================================================
class TestBrowserSessionFrozen:
    def test_config_frozen(self):
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig()
        with pytest.raises(AttributeError):
            cfg.headless = False

    def test_state_frozen(self):
        BrowserSessionState = _import_or_skip("BrowserSessionState")
        # Enum instances are frozen - can't set attributes on instances
        created = BrowserSessionState.Created
        with pytest.raises(AttributeError):
            created.x = "hack"


# ============================================================================
# 21. TestBrowserSessionMultipleInstances
# ============================================================================
class TestBrowserSessionMultipleInstances:
    def test_distinct_session_ids(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig()
        s1 = BrowserSession(cfg)
        import time
        time.sleep(0.001)  # Ensure different timestamp
        s2 = BrowserSession(cfg)
        assert s1.session_id != s2.session_id
        assert s1.session_id.startswith("browser-")
        assert s2.session_id.startswith("browser-")

    def test_independent_configs(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg1 = BrowserSessionConfig(headless=True)
        cfg2 = BrowserSessionConfig(headless=False)
        s1 = BrowserSession(cfg1)
        s2 = BrowserSession(cfg2)
        assert s1.config.headless is True
        assert s2.config.headless is False


# ============================================================================
# 22. TestBrowserSessionStatsAccumulation
# ============================================================================
class TestBrowserSessionStatsAccumulation:
    def test_dom_snapshot_stats_increase(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        for _ in range(3):
            session.get_dom_snapshot()
        assert session.stats.dom_snapshots == 3

    def test_screenshot_stats_increase(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        session.start()
        for _ in range(4):
            session.take_screenshot()
        assert session.stats.screenshots_taken == 4

    def test_cookie_stats_increase(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        for _ in range(2):
            session.get_cookies()
        assert session.stats.cookies_collected == 2

    def test_pages_navigated_increase(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        session.start()
        for i in range(5):
            session.navigate(f"https://example.com/{i}")
        assert session.stats.pages_navigated == 5


# ============================================================================
# 23. TestBrowserConfigOptionals
# ============================================================================
class TestBrowserConfigOptionals:
    def test_optional_fields_default_none(self):
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig()
        assert cfg.target_url is None
        assert cfg.proxy is None
        assert cfg.user_agent is None

    def test_empty_extra_headers(self):
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig()
        assert cfg.extra_headers == []


# ============================================================================
# 24. TestBrowserSessionStateTransitions
# ============================================================================
class TestBrowserSessionStateTransitions:
    def test_created_to_ready_to_stopped(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        BrowserSessionState = _import_or_skip("BrowserSessionState")
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        assert session.state == BrowserSessionState.Created
        session.start()
        assert session.state == BrowserSessionState.Ready
        session.stop()
        assert session.state == BrowserSessionState.Stopped

    def test_navigate_transitions(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        BrowserSessionState = _import_or_skip("BrowserSessionState")
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        session.start()
        assert session.state == BrowserSessionState.Ready
        session.navigate("https://example.com")
        assert session.state == BrowserSessionState.Ready


# ============================================================================
# 25. TestBrowserDomSnapshotAttributes
# ============================================================================
class TestBrowserDomSnapshotAttributes:
    def test_forms_property(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        snap = session.get_dom_snapshot()
        assert isinstance(snap.forms, list)

    def test_links_property(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        snap = session.get_dom_snapshot()
        assert isinstance(snap.links, list)

    def test_scripts_property(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        snap = session.get_dom_snapshot()
        assert isinstance(snap.scripts, list)

    def test_frames_property(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        snap = session.get_dom_snapshot()
        assert isinstance(snap.frames, list)


# ============================================================================
# 26. TestBrowserNavigationEventFields
# ============================================================================
class TestBrowserNavigationEventFields:
    def test_redirect_chain_property(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        session.start()
        event = session.navigate("https://example.com")
        assert isinstance(event.redirect_chain, list)

    def test_timestamp_is_positive(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        session.start()
        event = session.navigate("https://example.com")
        assert event.timestamp_ms > 0


# ============================================================================
# 27. TestBrowserStorageInfoFields
# ============================================================================
class TestBrowserStorageInfoFields:
    def test_local_storage_property(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        storage = session.get_cookies()
        assert isinstance(storage.local_storage, list)

    def test_session_storage_property(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        storage = session.get_cookies()
        assert isinstance(storage.session_storage, list)

    def test_cookies_property(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        storage = session.get_cookies()
        assert isinstance(storage.cookies, list)


# ============================================================================
# 28. TestBrowserTestConfig
# ============================================================================
class TestBrowserTestConfig:
    def test_import(self):
        BrowserTestConfig = _import_or_skip("BrowserTestConfig")
        assert BrowserTestConfig is not None

    def test_construct_defaults(self):
        BrowserTestConfig = _import_or_skip("BrowserTestConfig")
        cfg = BrowserTestConfig()
        assert cfg.check_dom_xss is True
        assert cfg.discover_spa_routes is True
        assert cfg.check_client_security is True
        assert cfg.timeout_ms == 30000
        assert cfg.xss_payload == "<img src=x onerror=alert(1)>"

    def test_construct_custom(self):
        BrowserTestConfig = _import_or_skip("BrowserTestConfig")
        cfg = BrowserTestConfig(
            check_dom_xss=False,
            discover_spa_routes=False,
            check_client_security=False,
            timeout_ms=60000,
            xss_payload="<script>alert(1)</script>",
        )
        assert cfg.check_dom_xss is False
        assert cfg.discover_spa_routes is False
        assert cfg.check_client_security is False
        assert cfg.timeout_ms == 60000
        assert cfg.xss_payload == "<script>alert(1)</script>"

    def test_repr(self):
        BrowserTestConfig = _import_or_skip("BrowserTestConfig")
        cfg = BrowserTestConfig()
        r = repr(cfg)
        assert "BrowserTestConfig" in r
        assert "dom_xss=" in r
        assert "spa_routes=" in r

    def test_frozen(self):
        BrowserTestConfig = _import_or_skip("BrowserTestConfig")
        cfg = BrowserTestConfig()
        with pytest.raises(AttributeError):
            cfg.check_dom_xss = False


# ============================================================================
# 29. TestBrowserTestReport
# ============================================================================
class TestBrowserTestReport:
    def test_import(self):
        BrowserTestReport = _import_or_skip("BrowserTestReport")
        assert BrowserTestReport is not None

    def test_browser_test_exists(self):
        browser_test = _import_or_skip("browser_test")
        assert callable(browser_test)

    def test_async_browser_test_exists(self):
        async_browser_test = _import_or_skip("async_browser_test")
        assert callable(async_browser_test)


# ============================================================================
# 30. TestBrowserSessionEndToEnd
# ============================================================================
class TestBrowserSessionEndToEnd:
    def test_full_lifecycle(self):
        BrowserSession = _import_or_skip("BrowserSession")
        BrowserSessionConfig = _import_or_skip("BrowserSessionConfig")
        BrowserSessionState = _import_or_skip("BrowserSessionState")

        cfg = BrowserSessionConfig(headless=True, viewport_width=1024)
        session = BrowserSession(cfg)

        assert session.state == BrowserSessionState.Created
        session.start()
        assert session.state == BrowserSessionState.Ready

        event = session.navigate("https://example.com")
        assert event.url == "https://example.com"
        assert session.stats.pages_navigated == 1

        snap = session.get_dom_snapshot()
        assert snap.timestamp_ms > 0
        assert session.stats.dom_snapshots == 1

        storage = session.get_cookies()
        assert storage.cookies == []

        artifact = session.take_screenshot()
        assert artifact.artifact_id.startswith("screenshot-")
        assert session.stats.screenshots_taken == 1

        d = session.to_dict()
        assert d["state"] == "Ready"

        session.stop()
        assert session.state == BrowserSessionState.Stopped
