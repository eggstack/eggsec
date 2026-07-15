"""Tests for Release 4: Session lifecycle, daemon parity, storage, and streaming reporting."""
import json
import os
import tempfile
import pytest


# ============================================================================
# Import tests (Section 9)
# ============================================================================

class TestImports:
    def test_session_contract_imports(self):
        from eggsec import (
            SessionState,
            SessionIdentity,
            SessionStats,
            SessionCloseMode,
            SessionEvent,
            SessionEventStream,
            SessionCapabilities,
        )

    def test_mobile_session_imports(self):
        from eggsec import (
            MobileDeviceDescriptor,
            MobileDeviceCapabilities,
            MobileSessionConfig,
            MobileSessionState,
            MobileSessionStats,
            MobileSession,
            AsyncMobileSession,
            MobileDeviceRegistry,
        )

    def test_browser_session_imports(self):
        from eggsec import (
            BrowserCapabilities,
            BrowserSessionState,
            BrowserSessionConfig,
            BrowserSessionStats,
            BrowserNavigationEvent,
            BrowserConsoleEvent,
            BrowserNetworkEvent,
            BrowserDomSnapshot,
            BrowserFormInfo,
            BrowserFormField,
            BrowserLinkInfo,
            BrowserStorageInfo,
            BrowserCookieInfo,
            BrowserSession,
            AsyncBrowserSession,
        )

    def test_daemon_parity_imports(self):
        from eggsec import (
            DaemonProtocolVersion,
            IdempotencyKey,
            DaemonSubmissionResult,
            ReconnectOptions,
            ReplayCursor,
            ReplayResult,
            CancellationRequest,
            CancellationResult,
            TaskArtifactDescriptor,
            EventReplayInfo,
            DaemonHealthDetail,
        )

    def test_sqlite_repository_imports(self):
        from eggsec import (
            SqliteFindingRepository,
            SqliteAssessmentRepository,
            SqliteMigration,
            SqliteMigrationResult,
        )

    def test_content_addressed_store_imports(self):
        from eggsec import (
            ContentAddressedArtifactStore,
            DirectoryArtifactStore,
            ArtifactInfo,
            ArtifactData,
            IntegrityResult,
            ArtifactQuery,
        )

    def test_streaming_reporter_imports(self):
        from eggsec import (
            StreamingReportConfig,
            StreamingReporter,
            ReportSummary,
            StreamingDiffReporter,
            FindingDiffResult,
            DiffReportSummary,
            ReportManifest,
        )


# ============================================================================
# WS1: Session Contract Tests
# ============================================================================

class TestSessionState:
    def test_enum_variants(self):
        from eggsec import SessionState
        assert repr(SessionState.Created) == "SessionState.Created"
        assert str(SessionState.Created) == "Created"
        assert str(SessionState.Starting) == "Starting"
        assert str(SessionState.Running) == "Running"
        assert str(SessionState.Pausing) == "Pausing"
        assert str(SessionState.Paused) == "Paused"
        assert str(SessionState.Stopping) == "Stopping"
        assert str(SessionState.Stopped) == "Stopped"
        assert str(SessionState.Failed) == "Failed"
        assert str(SessionState.Cancelled) == "Cancelled"

    def test_all_variants_have_repr(self):
        from eggsec import SessionState
        variants = [
            SessionState.Created, SessionState.Starting, SessionState.Running,
            SessionState.Pausing, SessionState.Paused, SessionState.Stopping,
            SessionState.Stopped, SessionState.Failed, SessionState.Cancelled,
        ]
        for v in variants:
            assert repr(v).startswith("SessionState.")


class TestSessionIdentity:
    def test_construction(self):
        from eggsec import SessionIdentity
        si = SessionIdentity("sess-1", "browser", 1000)
        assert si.session_id == "sess-1"
        assert si.session_type == "browser"
        assert si.created_at_ms == 1000
        assert si.owner_id is None

    def test_construction_with_owner(self):
        from eggsec import SessionIdentity
        si = SessionIdentity("sess-1", "mobile", 2000, owner_id="user-42")
        assert si.owner_id == "user-42"

    def test_serialization_to_dict(self):
        from eggsec import SessionIdentity
        si = SessionIdentity("sess-1", "browser", 1000)
        d = si.to_dict()
        assert d["session_id"] == "sess-1"
        assert d["session_type"] == "browser"
        assert d["created_at_ms"] == 1000
        assert d["owner_id"] is None

    def test_serialization_to_json(self):
        from eggsec import SessionIdentity
        si = SessionIdentity("sess-1", "browser", 1000)
        j = si.to_json()
        parsed = json.loads(j)
        assert parsed["session_id"] == "sess-1"

    def test_repr_and_str(self):
        from eggsec import SessionIdentity
        si = SessionIdentity("sess-1", "browser", 1000)
        assert "sess-1" in repr(si)
        assert "browser" in str(si)


class TestSessionStats:
    def test_construction_defaults(self):
        from eggsec import SessionStats
        s = SessionStats()
        assert s.total_operations == 0
        assert s.completed_operations == 0
        assert s.failed_operations == 0
        assert s.cancelled_operations == 0
        assert s.elapsed_ms == 0
        assert s.last_activity_ms is None

    def test_construction_with_values(self):
        from eggsec import SessionStats
        s = SessionStats(
            total_operations=10, completed_operations=8,
            failed_operations=1, cancelled_operations=1,
            elapsed_ms=5000, last_activity_ms=4000,
        )
        assert s.total_operations == 10
        assert s.last_activity_ms == 4000

    def test_serialization(self):
        from eggsec import SessionStats
        s = SessionStats(total_operations=5, elapsed_ms=1000)
        d = s.to_dict()
        assert d["total_operations"] == 5
        j = s.to_json()
        assert "5" in j

    def test_repr_and_str(self):
        from eggsec import SessionStats
        s = SessionStats(total_operations=10, completed_operations=8, elapsed_ms=5000)
        assert "10" in repr(s)
        assert "8" in str(s)


class TestSessionCloseMode:
    def test_enum_variants(self):
        from eggsec import SessionCloseMode
        assert str(SessionCloseMode.Graceful) == "Graceful"
        assert str(SessionCloseMode.Forced) == "Forced"
        assert str(SessionCloseMode.Immediate) == "Immediate"

    def test_repr(self):
        from eggsec import SessionCloseMode
        assert repr(SessionCloseMode.Graceful) == "SessionCloseMode.Graceful"


class TestSessionEvent:
    def test_construction(self):
        from eggsec import SessionEvent
        e = SessionEvent(1, 1000, "state_changed", message="started")
        assert e.sequence == 1
        assert e.timestamp_ms == 1000
        assert e.event_type == "state_changed"
        assert e.message == "started"

    def test_construction_no_message(self):
        from eggsec import SessionEvent
        e = SessionEvent(1, 1000, "heartbeat")
        assert e.message is None

    def test_serialization(self):
        from eggsec import SessionEvent
        e = SessionEvent(1, 1000, "state_changed", message="started")
        d = e.to_dict()
        assert d["sequence"] == 1
        assert d["event_type"] == "state_changed"
        j = e.to_json()
        parsed = json.loads(j)
        assert parsed["sequence"] == 1

    def test_repr_and_str(self):
        from eggsec import SessionEvent
        e = SessionEvent(1, 1000, "state_changed", message="started")
        assert "1" in repr(e)
        assert "state_changed" in str(e)


class TestSessionEventStream:
    def test_construction(self):
        from eggsec import SessionEventStream
        s = SessionEventStream("sess-1")
        assert s.session_id == "sess-1"
        assert s.events == []
        assert s.sequence == 0

    def test_construction_with_events(self):
        from eggsec import SessionEventStream, SessionEvent
        events = [SessionEvent(1, 1000, "started"), SessionEvent(2, 2000, "stopped")]
        s = SessionEventStream("sess-1", events=events, sequence=2)
        assert len(s.events) == 2
        assert s.sequence == 2

    def test_serialization(self):
        from eggsec import SessionEventStream
        s = SessionEventStream("sess-1")
        d = s.to_dict()
        assert d["session_id"] == "sess-1"
        j = s.to_json()
        assert "sess-1" in j

    def test_repr_and_str(self):
        from eggsec import SessionEventStream
        s = SessionEventStream("sess-1")
        assert "sess-1" in repr(s)
        assert "sess-1" in str(s)


class TestSessionCapabilities:
    def test_construction_defaults(self):
        from eggsec import SessionCapabilities
        c = SessionCapabilities()
        assert c.supports_cancellation is False
        assert c.supports_timeout is False
        assert c.supports_artifacts is False
        assert c.supports_streaming is False
        assert c.max_concurrent_operations == 1

    def test_construction_with_values(self):
        from eggsec import SessionCapabilities
        c = SessionCapabilities(
            supports_cancellation=True, supports_timeout=True,
            supports_artifacts=True, supports_streaming=True,
            max_concurrent_operations=4,
        )
        assert c.supports_cancellation is True
        assert c.max_concurrent_operations == 4

    def test_serialization(self):
        from eggsec import SessionCapabilities
        c = SessionCapabilities(supports_cancellation=True)
        d = c.to_dict()
        assert d["supports_cancellation"] is True
        j = c.to_json()
        assert "supports_cancellation" in j

    def test_repr_and_str(self):
        from eggsec import SessionCapabilities
        c = SessionCapabilities(supports_cancellation=True)
        assert "cancel=true" in repr(c)
        assert "cancel=true" in str(c)


# ============================================================================
# WS2-6: Mobile Session Tests
# ============================================================================

class TestMobileDeviceDescriptor:
    def test_construction_not_direct(self):
        """MobileDeviceDescriptor is not directly constructible from Python."""
        from eggsec import MobileDeviceRegistry
        registry = MobileDeviceRegistry()
        devices = registry.devices
        assert isinstance(devices, list)

    def test_to_dict_via_registry(self):
        """Verify the type's serialization methods exist via reflection."""
        from eggsec import MobileDeviceDescriptor
        # The class should be importable and have to_dict/to_json
        assert hasattr(MobileDeviceDescriptor, 'to_dict')
        assert hasattr(MobileDeviceDescriptor, 'to_json')


class TestMobileDeviceCapabilities:
    def test_has_serialization_methods(self):
        from eggsec import MobileDeviceCapabilities
        assert hasattr(MobileDeviceCapabilities, 'to_dict')
        assert hasattr(MobileDeviceCapabilities, 'to_json')
        assert hasattr(MobileDeviceCapabilities, '__repr__')


class TestMobileSessionConfig:
    def test_construction_minimal(self):
        from eggsec import MobileSessionConfig
        cfg = MobileSessionConfig("serial-123")
        assert cfg.device_serial == "serial-123"
        assert cfg.package_id is None
        assert cfg.install_app is False
        assert cfg.uninstall_after is False
        assert cfg.capture_logs is False
        assert cfg.capture_screenshots is False
        assert cfg.capture_network is False
        assert cfg.allow_frida is False
        assert cfg.dry_run is False

    def test_construction_full(self):
        from eggsec import MobileSessionConfig
        cfg = MobileSessionConfig(
            "serial-123",
            package_id="com.example.app",
            install_app=True,
            uninstall_after=True,
            capture_logs=True,
            capture_screenshots=True,
            capture_network=True,
            traffic_output="/tmp/traffic.pcap",
            frida_scripts=["hook.js"],
            allow_frida=True,
            timeout_secs=300,
            proxy="http://localhost:8080",
            grant_permissions=["INTERNET"],
            revoke_permissions=["CAMERA"],
            dry_run=True,
        )
        assert cfg.package_id == "com.example.app"
        assert cfg.install_app is True
        assert cfg.allow_frida is True
        assert cfg.dry_run is True
        assert "hook.js" in cfg.frida_scripts
        assert "INTERNET" in cfg.grant_permissions
        assert "CAMERA" in cfg.revoke_permissions

    def test_serialization(self):
        from eggsec import MobileSessionConfig
        cfg = MobileSessionConfig("serial-123")
        d = cfg.to_dict()
        assert d["device_serial"] == "serial-123"
        j = cfg.to_json()
        assert "serial-123" in j

    def test_repr(self):
        from eggsec import MobileSessionConfig
        cfg = MobileSessionConfig("serial-123")
        assert "serial-123" in repr(cfg)


class TestMobileSessionState:
    def test_enum_variants(self):
        from eggsec import MobileSessionState
        variants = [
            "Created", "Connecting", "Installing", "Launching",
            "Running", "Capturing", "Stopping", "Uninstalling",
            "Cleaning", "Stopped", "Failed", "Cancelled",
        ]
        for name in variants:
            assert str(getattr(MobileSessionState, name)) == name
            assert repr(getattr(MobileSessionState, name)) == f"MobileSessionState.{name}"


class TestMobileSessionStats:
    def test_has_serialization_methods(self):
        from eggsec import MobileSessionStats
        assert hasattr(MobileSessionStats, 'to_dict')
        assert hasattr(MobileSessionStats, 'to_json')
        assert hasattr(MobileSessionStats, '__repr__')


class TestMobileDeviceRegistry:
    def test_construction(self):
        from eggsec import MobileDeviceRegistry
        registry = MobileDeviceRegistry()
        assert registry.devices == []

    def test_get_device_not_found(self):
        from eggsec import MobileDeviceRegistry
        registry = MobileDeviceRegistry()
        assert registry.get_device("nonexistent") is None

    def test_serialization(self):
        from eggsec import MobileDeviceRegistry
        registry = MobileDeviceRegistry()
        d = registry.to_dict()
        assert d["devices"] == []
        j = registry.to_json()
        assert "[]" in j


class TestMobileSession:
    def test_construction(self):
        from eggsec import MobileSession, MobileSessionConfig
        cfg = MobileSessionConfig("serial-123")
        session = MobileSession("sess-1", "serial-123", cfg)
        assert session.session_id == "sess-1"
        assert session.device_serial == "serial-123"
        assert str(session.state) == "Created"

    def test_start_stop(self):
        from eggsec import MobileSession, MobileSessionConfig
        cfg = MobileSessionConfig("serial-123")
        session = MobileSession("sess-1", "serial-123", cfg)
        session.start()
        assert str(session.state) == "Connecting"
        session.stop()
        assert str(session.state) == "Stopping"

    def test_install_app_raises(self):
        from eggsec import MobileSession, MobileSessionConfig
        cfg = MobileSessionConfig("serial-123")
        session = MobileSession("sess-1", "serial-123", cfg)
        with pytest.raises(Exception):
            session.install_app("/tmp/app.apk")

    def test_launch_app_raises(self):
        from eggsec import MobileSession, MobileSessionConfig
        cfg = MobileSessionConfig("serial-123", package_id="com.example.app")
        session = MobileSession("sess-1", "serial-123", cfg)
        with pytest.raises(Exception):
            session.launch_app()

    def test_uninstall_app_raises(self):
        from eggsec import MobileSession, MobileSessionConfig
        cfg = MobileSessionConfig("serial-123", package_id="com.example.app")
        session = MobileSession("sess-1", "serial-123", cfg)
        with pytest.raises(Exception):
            session.uninstall_app()

    def test_capture_screenshot_raises(self):
        from eggsec import MobileSession, MobileSessionConfig
        cfg = MobileSessionConfig("serial-123")
        session = MobileSession("sess-1", "serial-123", cfg)
        with pytest.raises(Exception):
            session.capture_screenshot()

    def test_get_logs_raises(self):
        from eggsec import MobileSession, MobileSessionConfig
        cfg = MobileSessionConfig("serial-123")
        session = MobileSession("sess-1", "serial-123", cfg)
        with pytest.raises(Exception):
            session.get_logs()

    def test_serialization(self):
        from eggsec import MobileSession, MobileSessionConfig
        cfg = MobileSessionConfig("serial-123")
        session = MobileSession("sess-1", "serial-123", cfg)
        d = session.to_dict()
        assert d["session_id"] == "sess-1"
        j = session.to_json()
        assert "sess-1" in j

    def test_repr(self):
        from eggsec import MobileSession, MobileSessionConfig
        cfg = MobileSessionConfig("serial-123")
        session = MobileSession("sess-1", "serial-123", cfg)
        r = repr(session)
        assert "sess-1" in r
        assert "serial-123" in r

    def test_context_manager(self):
        from eggsec import MobileSession, MobileSessionConfig
        cfg = MobileSessionConfig("serial-123")
        session = MobileSession("sess-1", "serial-123", cfg)
        with session:
            assert str(session.state) == "Created"

    def test_stats(self):
        from eggsec import MobileSession, MobileSessionConfig
        cfg = MobileSessionConfig("serial-123")
        session = MobileSession("sess-1", "serial-123", cfg)
        stats = session.stats
        assert stats.screenshots_captured == 0
        assert stats.duration_ms == 0


class TestAsyncMobileSession:
    def test_construction(self):
        from eggsec import AsyncMobileSession, MobileSessionConfig
        cfg = MobileSessionConfig("serial-123")
        session = AsyncMobileSession("sess-async-1", "serial-123", cfg)
        assert session.session_id == "sess-async-1"
        assert session.device_serial == "serial-123"
        assert str(session.state) == "Created"

    def test_serialization(self):
        from eggsec import AsyncMobileSession, MobileSessionConfig
        cfg = MobileSessionConfig("serial-123")
        session = AsyncMobileSession("sess-async-1", "serial-123", cfg)
        d = session.to_dict()
        assert d["session_id"] == "sess-async-1"
        j = session.to_json()
        assert "sess-async-1" in j

    def test_repr(self):
        from eggsec import AsyncMobileSession, MobileSessionConfig
        cfg = MobileSessionConfig("serial-123")
        session = AsyncMobileSession("sess-async-1", "serial-123", cfg)
        assert "sess-async-1" in repr(session)


# ============================================================================
# WS7-11: Browser Session Tests
# ============================================================================

class TestBrowserCapabilities:
    def test_has_serialization_methods(self):
        from eggsec import BrowserCapabilities
        assert hasattr(BrowserCapabilities, 'to_dict')
        assert hasattr(BrowserCapabilities, 'to_json')
        assert hasattr(BrowserCapabilities, '__repr__')


class TestBrowserSessionState:
    def test_enum_variants(self):
        from eggsec import BrowserSessionState
        variants = [
            "Created", "Discovering", "Launching", "Ready",
            "Navigating", "Loading", "Inspecting", "Stopping",
            "Cleaning", "Stopped", "Failed", "Cancelled",
        ]
        for name in variants:
            assert str(getattr(BrowserSessionState, name)) == name
            assert repr(getattr(BrowserSessionState, name)) == f"BrowserSessionState.{name}"


class TestBrowserSessionConfig:
    def test_construction_defaults(self):
        from eggsec import BrowserSessionConfig
        cfg = BrowserSessionConfig()
        assert cfg.target_url is None
        assert cfg.headless is True
        assert cfg.proxy is None
        assert cfg.user_agent is None
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

    def test_construction_with_values(self):
        from eggsec import BrowserSessionConfig
        cfg = BrowserSessionConfig(
            target_url="https://example.com",
            headless=False,
            proxy="http://localhost:8080",
            user_agent="CustomBot/1.0",
            viewport_width=1920,
            viewport_height=1080,
            timeout_ms=60000,
            navigation_timeout_ms=120000,
            collect_console=False,
            collect_network=False,
            collect_cookies=False,
            collect_storage=False,
            screenshot_on_complete=True,
            extra_headers=["X-Custom: value"],
            ignore_cert_errors=True,
        )
        assert cfg.target_url == "https://example.com"
        assert cfg.headless is False
        assert cfg.viewport_width == 1920
        assert "X-Custom: value" in cfg.extra_headers

    def test_serialization(self):
        from eggsec import BrowserSessionConfig
        cfg = BrowserSessionConfig()
        d = cfg.to_dict()
        assert d["headless"] is True
        assert d["viewport_width"] == 1280
        j = cfg.to_json()
        assert "1280" in j

    def test_repr(self):
        from eggsec import BrowserSessionConfig
        cfg = BrowserSessionConfig()
        r = repr(cfg)
        assert "headless=true" in r
        assert "1280" in r


class TestBrowserSessionStats:
    def test_has_serialization_methods(self):
        from eggsec import BrowserSessionStats
        assert hasattr(BrowserSessionStats, 'to_dict')
        assert hasattr(BrowserSessionStats, 'to_json')
        assert hasattr(BrowserSessionStats, '__repr__')


class TestBrowserNavigationEvent:
    def test_has_serialization_methods(self):
        from eggsec import BrowserNavigationEvent
        assert hasattr(BrowserNavigationEvent, 'to_dict')
        assert hasattr(BrowserNavigationEvent, 'to_json')
        assert hasattr(BrowserNavigationEvent, '__repr__')


class TestBrowserConsoleEvent:
    def test_has_serialization_methods(self):
        from eggsec import BrowserConsoleEvent
        assert hasattr(BrowserConsoleEvent, 'to_dict')
        assert hasattr(BrowserConsoleEvent, 'to_json')
        assert hasattr(BrowserConsoleEvent, '__repr__')


class TestBrowserNetworkEvent:
    def test_has_serialization_methods(self):
        from eggsec import BrowserNetworkEvent
        assert hasattr(BrowserNetworkEvent, 'to_dict')
        assert hasattr(BrowserNetworkEvent, 'to_json')
        assert hasattr(BrowserNetworkEvent, '__repr__')


class TestBrowserDomSnapshot:
    def test_has_serialization_methods(self):
        from eggsec import BrowserDomSnapshot
        assert hasattr(BrowserDomSnapshot, 'to_dict')
        assert hasattr(BrowserDomSnapshot, 'to_json')
        assert hasattr(BrowserDomSnapshot, '__repr__')


class TestBrowserFormInfo:
    def test_has_serialization_methods(self):
        from eggsec import BrowserFormInfo
        assert hasattr(BrowserFormInfo, 'to_dict')
        assert hasattr(BrowserFormInfo, 'to_json')


class TestBrowserFormField:
    def test_has_serialization_methods(self):
        from eggsec import BrowserFormField
        assert hasattr(BrowserFormField, 'to_dict')
        assert hasattr(BrowserFormField, 'to_json')


class TestBrowserLinkInfo:
    def test_has_serialization_methods(self):
        from eggsec import BrowserLinkInfo
        assert hasattr(BrowserLinkInfo, 'to_dict')
        assert hasattr(BrowserLinkInfo, 'to_json')


class TestBrowserStorageInfo:
    def test_has_serialization_methods(self):
        from eggsec import BrowserStorageInfo
        assert hasattr(BrowserStorageInfo, 'to_dict')
        assert hasattr(BrowserStorageInfo, 'to_json')


class TestBrowserCookieInfo:
    def test_has_serialization_methods(self):
        from eggsec import BrowserCookieInfo
        assert hasattr(BrowserCookieInfo, 'to_dict')
        assert hasattr(BrowserCookieInfo, 'to_json')


class TestBrowserSession:
    def test_construction(self):
        from eggsec import BrowserSession, BrowserSessionConfig
        cfg = BrowserSessionConfig(target_url="https://example.com")
        session = BrowserSession(cfg)
        assert session.session_id.startswith("browser-")
        assert str(session.state) == "Created"

    def test_start_stop(self):
        from eggsec import BrowserSession, BrowserSessionConfig
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        session.start()
        assert str(session.state) == "Ready"
        session.stop()
        assert str(session.state) == "Stopped"

    def test_navigate_requires_ready(self):
        from eggsec import BrowserSession, BrowserSessionConfig
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        with pytest.raises(Exception):
            session.navigate("https://example.com")

    def test_navigate_from_ready(self):
        from eggsec import BrowserSession, BrowserSessionConfig
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        session.start()
        event = session.navigate("https://example.com")
        assert event.url == "https://example.com"

    def test_get_dom_snapshot(self):
        from eggsec import BrowserSession, BrowserSessionConfig
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        snapshot = session.get_dom_snapshot()
        assert snapshot.url == ""

    def test_config_property(self):
        from eggsec import BrowserSession, BrowserSessionConfig
        cfg = BrowserSessionConfig(viewport_width=800)
        session = BrowserSession(cfg)
        assert session.config.viewport_width == 800

    def test_stats_property(self):
        from eggsec import BrowserSession, BrowserSessionConfig
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        stats = session.stats
        assert stats.pages_navigated == 0
        assert stats.duration_ms == 0

    def test_serialization(self):
        from eggsec import BrowserSession, BrowserSessionConfig
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        d = session.to_dict()
        assert "session_id" in d
        j = session.to_json()
        assert "browser-" in j

    def test_repr(self):
        from eggsec import BrowserSession, BrowserSessionConfig
        cfg = BrowserSessionConfig()
        session = BrowserSession(cfg)
        assert "browser-" in repr(session)

    def test_context_manager(self):
        from eggsec import BrowserSession, BrowserSessionConfig
        cfg = BrowserSessionConfig()
        with BrowserSession(cfg) as session:
            assert str(session.state) == "Created"


class TestAsyncBrowserSession:
    def test_construction(self):
        from eggsec import AsyncBrowserSession, BrowserSessionConfig
        cfg = BrowserSessionConfig(target_url="https://example.com")
        session = AsyncBrowserSession(cfg)
        assert session.session_id.startswith("async-browser-")
        assert str(session.state) == "Created"

    def test_serialization(self):
        from eggsec import AsyncBrowserSession, BrowserSessionConfig
        cfg = BrowserSessionConfig()
        session = AsyncBrowserSession(cfg)
        d = session.to_dict()
        assert "session_id" in d
        j = session.to_json()
        assert "async-browser-" in j

    def test_repr(self):
        from eggsec import AsyncBrowserSession, BrowserSessionConfig
        cfg = BrowserSessionConfig()
        session = AsyncBrowserSession(cfg)
        assert "async-browser-" in repr(session)


# ============================================================================
# WS12-18: Daemon Parity Tests
# ============================================================================

class TestDaemonProtocolVersion:
    def test_construction(self):
        from eggsec import DaemonProtocolVersion
        dpv = DaemonProtocolVersion(
            api_schema_version=1,
            operation_registry_id="default",
            feature_profile="standard",
        )
        assert dpv.protocol_version == 2
        assert dpv.api_schema_version == 1
        assert dpv.operation_registry_id == "default"
        assert dpv.feature_profile == "standard"

    def test_serialization(self):
        from eggsec import DaemonProtocolVersion
        dpv = DaemonProtocolVersion(1, "default", "standard")
        d = dpv.to_dict()
        assert d["protocol_version"] == 2
        assert d["api_schema_version"] == 1
        j = dpv.to_json()
        parsed = json.loads(j)
        assert parsed["protocol_version"] == 2

    def test_repr(self):
        from eggsec import DaemonProtocolVersion
        dpv = DaemonProtocolVersion(1, "default", "standard")
        assert "protocol=2" in repr(dpv)


class TestIdempotencyKey:
    def test_from_request(self):
        from eggsec import IdempotencyKey
        key = IdempotencyKey.from_request("scan_ports", '{"target":"example.com"}')
        assert len(key.key) > 0
        assert key.operation_name == "scan_ports"
        assert len(key.request_hash) == 16
        assert key.created_at_ms > 0

    def test_from_request_deterministic_hash(self):
        from eggsec import IdempotencyKey
        k1 = IdempotencyKey.from_request("scan", '{"target":"a"}')
        k2 = IdempotencyKey.from_request("scan", '{"target":"a"}')
        assert k1.request_hash == k2.request_hash
        assert k1.key != k2.key

    def test_serialization(self):
        from eggsec import IdempotencyKey
        key = IdempotencyKey.from_request("test_op", "{}")
        d = key.to_dict()
        assert "key" in d
        assert d["operation_name"] == "test_op"
        j = key.to_json()
        assert "test_op" in j

    def test_repr(self):
        from eggsec import IdempotencyKey
        key = IdempotencyKey.from_request("test_op", "{}")
        assert "test_op" in repr(key)


class TestDaemonSubmissionResult:
    def test_has_serialization_methods(self):
        from eggsec import DaemonSubmissionResult
        assert hasattr(DaemonSubmissionResult, 'to_dict')
        assert hasattr(DaemonSubmissionResult, 'to_json')
        assert hasattr(DaemonSubmissionResult, '__repr__')


class TestReconnectOptions:
    def test_construction_defaults(self):
        from eggsec import ReconnectOptions
        ro = ReconnectOptions()
        assert ro.max_retries == 5
        assert ro.retry_delay_ms == 500
        assert ro.backoff_multiplier == 2.0
        assert ro.max_backoff_ms == 30000
        assert ro.replay_from_sequence is None

    def test_construction_custom(self):
        from eggsec import ReconnectOptions
        ro = ReconnectOptions(
            max_retries=10,
            retry_delay_ms=1000,
            backoff_multiplier=3.0,
            max_backoff_ms=60000,
            replay_from_sequence=42,
        )
        assert ro.max_retries == 10
        assert ro.replay_from_sequence == 42

    def test_serialization(self):
        from eggsec import ReconnectOptions
        ro = ReconnectOptions(max_retries=3)
        d = ro.to_dict()
        assert d["max_retries"] == 3
        j = ro.to_json()
        assert "3" in j

    def test_repr(self):
        from eggsec import ReconnectOptions
        ro = ReconnectOptions()
        assert "retries=5" in repr(ro)


class TestReplayCursor:
    def test_has_serialization_methods(self):
        from eggsec import ReplayCursor
        assert hasattr(ReplayCursor, 'to_dict')
        assert hasattr(ReplayCursor, 'to_json')
        assert hasattr(ReplayCursor, '__repr__')


class TestReplayResult:
    def test_has_serialization_methods(self):
        from eggsec import ReplayResult
        assert hasattr(ReplayResult, 'to_dict')
        assert hasattr(ReplayResult, 'to_json')
        assert hasattr(ReplayResult, '__repr__')


class TestCancellationRequest:
    def test_has_serialization_methods(self):
        from eggsec import CancellationRequest
        assert hasattr(CancellationRequest, 'to_dict')
        assert hasattr(CancellationRequest, 'to_json')
        assert hasattr(CancellationRequest, '__repr__')


class TestCancellationResult:
    def test_has_serialization_methods(self):
        from eggsec import CancellationResult
        assert hasattr(CancellationResult, 'to_dict')
        assert hasattr(CancellationResult, 'to_json')
        assert hasattr(CancellationResult, '__repr__')


class TestTaskArtifactDescriptor:
    def test_has_serialization_methods(self):
        from eggsec import TaskArtifactDescriptor
        assert hasattr(TaskArtifactDescriptor, 'to_dict')
        assert hasattr(TaskArtifactDescriptor, 'to_json')
        assert hasattr(TaskArtifactDescriptor, '__repr__')


class TestEventReplayInfo:
    def test_has_serialization_methods(self):
        from eggsec import EventReplayInfo
        assert hasattr(EventReplayInfo, 'to_dict')
        assert hasattr(EventReplayInfo, 'to_json')
        assert hasattr(EventReplayInfo, '__repr__')


class TestDaemonHealthDetail:
    def test_has_serialization_methods(self):
        from eggsec import DaemonHealthDetail
        assert hasattr(DaemonHealthDetail, 'to_dict')
        assert hasattr(DaemonHealthDetail, 'to_json')
        assert hasattr(DaemonHealthDetail, '__repr__')


# ============================================================================
# WS20-22: SQLite Repository Tests
# ============================================================================

class TestSqliteMigration:
    def test_construction(self):
        from eggsec import SqliteMigration
        m = SqliteMigration(1, "Initial schema", 1000)
        assert m.version == 1
        assert m.description == "Initial schema"
        assert m.applied_at_ms == 1000

    def test_serialization(self):
        from eggsec import SqliteMigration
        m = SqliteMigration(1, "Initial schema", 1000)
        d = m.to_dict()
        assert d["version"] == 1
        j = m.to_json()
        assert "Initial schema" in j

    def test_repr_and_str(self):
        from eggsec import SqliteMigration
        m = SqliteMigration(1, "Initial schema", 1000)
        assert "1" in repr(m)
        assert "Initial schema" in str(m)


class TestMigrationResult:
    def test_has_serialization_methods(self):
        from eggsec import SqliteMigrationResult
        assert hasattr(SqliteMigrationResult, 'to_dict')
        assert hasattr(SqliteMigrationResult, 'to_json')
        assert hasattr(SqliteMigrationResult, '__repr__')
        assert hasattr(SqliteMigrationResult, '__str__')


class TestSqliteFindingRepository:
    def test_construction(self):
        from eggsec import SqliteFindingRepository
        repo = SqliteFindingRepository(":memory:")
        assert repr(repo)

    def test_initialize(self):
        from eggsec import SqliteFindingRepository
        repo = SqliteFindingRepository(":memory:")
        repo.initialize()

    def test_insert_and_get(self):
        from eggsec import SqliteFindingRepository
        repo = SqliteFindingRepository(":memory:")
        repo.initialize()
        fid = repo.insert_finding('{"id":"f1","title":"Test","severity":"high"}')
        assert fid == "f1"
        result = repo.get_finding("f1")
        assert result is not None
        assert "Test" in result

    def test_insert_generated_id(self):
        from eggsec import SqliteFindingRepository
        repo = SqliteFindingRepository(":memory:")
        repo.initialize()
        fid = repo.insert_finding('{"title":"No ID","severity":"low"}')
        assert fid.startswith("find-")

    def test_query_findings(self):
        from eggsec import SqliteFindingRepository
        repo = SqliteFindingRepository(":memory:")
        repo.initialize()
        repo.insert_finding('{"id":"f1","severity":"high"}')
        repo.insert_finding('{"id":"f2","severity":"low"}')
        results = repo.query_findings(severity="high")
        assert len(results) == 1

    def test_count_findings(self):
        from eggsec import SqliteFindingRepository
        repo = SqliteFindingRepository(":memory:")
        repo.initialize()
        repo.insert_finding('{"id":"f1","severity":"high"}')
        repo.insert_finding('{"id":"f2","severity":"high"}')
        count = repo.count_findings(severity="high")
        assert count == 2

    def test_close(self):
        from eggsec import SqliteFindingRepository
        repo = SqliteFindingRepository(":memory:")
        repo.initialize()
        repo.close()

    def test_not_initialized_error(self):
        from eggsec import SqliteFindingRepository
        repo = SqliteFindingRepository(":memory:")
        with pytest.raises(Exception):
            repo.insert_finding('{"id":"f1"}')

    def test_deduplication(self):
        from eggsec import SqliteFindingRepository
        repo = SqliteFindingRepository(":memory:")
        repo.initialize()
        repo.insert_finding('{"id":"f1","dedup_key":"dk1"}')
        dup = repo.deduplicate("dk1")
        assert dup == "f1"
        assert repo.deduplicate("dk2") is None

    def test_serialization(self):
        from eggsec import SqliteFindingRepository
        repo = SqliteFindingRepository(":memory:")
        repo.initialize()
        repo.insert_finding('{"id":"f1","severity":"high"}')
        d = repo.to_dict()
        assert d["count"] >= 1
        j = repo.to_json()
        assert "f1" in j

    def test_repr(self):
        from eggsec import SqliteFindingRepository
        repo = SqliteFindingRepository(":memory:")
        r = repr(repo)
        assert "SqliteFindingRepository" in r

    def test_context_manager(self):
        from eggsec import SqliteFindingRepository
        with SqliteFindingRepository(":memory:") as repo:
            repo.initialize()
            repo.insert_finding('{"id":"f1","severity":"high"}')


class TestSqliteAssessmentRepository:
    def test_construction(self):
        from eggsec import SqliteAssessmentRepository
        repo = SqliteAssessmentRepository(":memory:")
        assert repr(repo)

    def test_create_and_get(self):
        from eggsec import SqliteAssessmentRepository
        repo = SqliteAssessmentRepository(":memory:")
        repo.initialize()
        aid = repo.create_assessment("Test", "10.0.0.1", "full-scan")
        assert aid.startswith("assess-")
        result = repo.get_assessment(aid)
        assert result is not None
        assert "Test" in result

    def test_list_assessments(self):
        from eggsec import SqliteAssessmentRepository
        repo = SqliteAssessmentRepository(":memory:")
        repo.initialize()
        repo.create_assessment("A1", "10.0.0.1", "scan")
        repo.create_assessment("A2", "10.0.0.2", "scan")
        results = repo.list_assessments()
        assert len(results) == 2

    def test_close(self):
        from eggsec import SqliteAssessmentRepository
        repo = SqliteAssessmentRepository(":memory:")
        repo.initialize()
        repo.close()

    def test_not_initialized_error(self):
        from eggsec import SqliteAssessmentRepository
        repo = SqliteAssessmentRepository(":memory:")
        with pytest.raises(Exception):
            repo.create_assessment("Test", "10.0.0.1", "scan")

    def test_serialization(self):
        from eggsec import SqliteAssessmentRepository
        repo = SqliteAssessmentRepository(":memory:")
        repo.initialize()
        repo.create_assessment("Test", "10.0.0.1", "scan")
        d = repo.to_dict()
        assert d["count"] >= 1
        j = repo.to_json()
        assert "Test" in j

    def test_repr(self):
        from eggsec import SqliteAssessmentRepository
        repo = SqliteAssessmentRepository(":memory:")
        assert "SqliteAssessmentRepository" in repr(repo)

    def test_context_manager(self):
        from eggsec import SqliteAssessmentRepository
        with SqliteAssessmentRepository(":memory:") as repo:
            repo.initialize()
            repo.create_assessment("Test", "10.0.0.1", "scan")


# ============================================================================
# WS23: Content-Addressed Store Tests
# ============================================================================

class TestArtifactInfo:
    def test_construction(self):
        from eggsec import ArtifactInfo
        info = ArtifactInfo("art-1", "abc123", "application/json", 100)
        assert info.artifact_id == "art-1"
        assert info.content_hash == "abc123"
        assert info.content_type == "application/json"
        assert info.size_bytes == 100
        assert info.redacted is False

    def test_construction_defaults(self):
        from eggsec import ArtifactInfo
        info = ArtifactInfo("art-1", "abc123", "text/plain", 50)
        assert info.metadata is None
        assert info.redacted is False

    def test_serialization(self):
        from eggsec import ArtifactInfo
        info = ArtifactInfo("art-1", "abc123", "text/plain", 50)
        d = info.to_dict()
        assert d["artifact_id"] == "art-1"
        j = info.to_json()
        assert "art-1" in j

    def test_repr(self):
        from eggsec import ArtifactInfo
        info = ArtifactInfo("art-1", "abc123", "text/plain", 50)
        assert "art-1" in repr(info)


class TestIntegrityResult:
    def test_construction(self):
        from eggsec import IntegrityResult
        ir = IntegrityResult(True, "abc123", "abc123", 100)
        assert ir.valid is True
        assert ir.expected_hash == "abc123"
        assert ir.actual_hash == "abc123"
        assert ir.size_bytes == 100

    def test_construction_mismatch(self):
        from eggsec import IntegrityResult
        ir = IntegrityResult(False, "abc123", "def456", 100)
        assert ir.valid is False

    def test_serialization(self):
        from eggsec import IntegrityResult
        ir = IntegrityResult(True, "abc123", "abc123", 100)
        d = ir.to_dict()
        assert d["valid"] is True
        j = ir.to_json()
        assert "abc123" in j

    def test_repr(self):
        from eggsec import IntegrityResult
        ir = IntegrityResult(True, "abc123", "abc123", 100)
        assert "valid=true" in repr(ir)


class TestArtifactQuery:
    def test_construction_defaults(self):
        from eggsec import ArtifactQuery
        q = ArtifactQuery()
        assert q.content_type is None
        assert q.min_size is None
        assert q.max_size is None
        assert q.limit == 100
        assert q.offset == 0

    def test_construction_with_values(self):
        from eggsec import ArtifactQuery
        q = ArtifactQuery(
            content_type="application/json",
            min_size=10,
            max_size=1000,
            limit=50,
            offset=10,
        )
        assert q.content_type == "application/json"
        assert q.limit == 50

    def test_serialization(self):
        from eggsec import ArtifactQuery
        q = ArtifactQuery(content_type="text/plain")
        d = q.to_dict()
        assert d["content_type"] == "text/plain"
        j = q.to_json()
        assert "text/plain" in j

    def test_repr(self):
        from eggsec import ArtifactQuery
        q = ArtifactQuery()
        assert "ArtifactQuery" in repr(q)


class TestContentAddressedArtifactStore:
    def test_construction(self):
        from eggsec import ContentAddressedArtifactStore
        store = ContentAddressedArtifactStore("/tmp/test-ca-store")
        assert repr(store)

    def test_initialize(self, tmp_path):
        from eggsec import ContentAddressedArtifactStore
        store = ContentAddressedArtifactStore(str(tmp_path / "ca"))
        store.initialize()
        assert os.path.isdir(str(tmp_path / "ca"))

    def test_put_and_get(self, tmp_path):
        from eggsec import ContentAddressedArtifactStore
        store = ContentAddressedArtifactStore(str(tmp_path / "ca"))
        store.initialize()
        info = store.put(b"hello world", "text/plain")
        assert info.size_bytes == 11
        assert info.content_hash
        result = store.get(info.content_hash)
        assert result is not None
        assert result.info.size_bytes == 11

    def test_has(self, tmp_path):
        from eggsec import ContentAddressedArtifactStore
        store = ContentAddressedArtifactStore(str(tmp_path / "ca"))
        store.initialize()
        info = store.put(b"data", "text/plain")
        assert store.has(info.content_hash) is True
        assert store.has("nonexistent") is False

    def test_verify(self, tmp_path):
        from eggsec import ContentAddressedArtifactStore
        store = ContentAddressedArtifactStore(str(tmp_path / "ca"))
        store.initialize()
        info = store.put(b"data", "text/plain")
        result = store.verify(info.content_hash)
        assert result.valid is True

    def test_list_artifacts(self, tmp_path):
        from eggsec import ContentAddressedArtifactStore
        store = ContentAddressedArtifactStore(str(tmp_path / "ca"))
        store.initialize()
        store.put(b"data1", "text/plain")
        store.put(b"data2", "text/plain")
        items = store.list_artifacts(10, 0)
        assert len(items) == 2

    def test_deduplication(self, tmp_path):
        from eggsec import ContentAddressedArtifactStore
        store = ContentAddressedArtifactStore(str(tmp_path / "ca"))
        store.initialize()
        info1 = store.put(b"same", "text/plain")
        info2 = store.put(b"same", "text/plain")
        assert info1.content_hash == info2.content_hash
        items = store.list_artifacts(10, 0)
        assert len(items) == 1

    def test_serialization(self, tmp_path):
        from eggsec import ContentAddressedArtifactStore
        store = ContentAddressedArtifactStore(str(tmp_path / "ca"))
        store.initialize()
        store.put(b"data", "text/plain")
        d = store.to_dict()
        assert d["base_dir"] == str(tmp_path / "ca")
        j = store.to_json()
        assert "data" in j

    def test_repr(self, tmp_path):
        from eggsec import ContentAddressedArtifactStore
        store = ContentAddressedArtifactStore(str(tmp_path / "ca"))
        r = repr(store)
        assert "ContentAddressedArtifactStore" in r

    def test_context_manager(self, tmp_path):
        from eggsec import ContentAddressedArtifactStore
        with ContentAddressedArtifactStore(str(tmp_path / "ca")) as store:
            store.initialize()
            store.put(b"data", "text/plain")


class TestDirectoryArtifactStore:
    def test_construction(self):
        from eggsec import DirectoryArtifactStore
        store = DirectoryArtifactStore("/tmp/test-dir-store")
        assert repr(store)

    def test_construction_non_flat(self):
        from eggsec import DirectoryArtifactStore
        store = DirectoryArtifactStore("/tmp/test-dir-store", flat=False)
        assert repr(store)

    def test_initialize(self, tmp_path):
        from eggsec import DirectoryArtifactStore
        store = DirectoryArtifactStore(str(tmp_path / "dir"))
        store.initialize()
        assert os.path.isdir(str(tmp_path / "dir"))

    def test_put_and_get(self, tmp_path):
        from eggsec import DirectoryArtifactStore
        store = DirectoryArtifactStore(str(tmp_path / "dir"))
        store.initialize()
        info = store.put("report.json", b'{"key":"value"}', "application/json")
        assert info.artifact_id == "report.json"
        result = store.get("report.json")
        assert result is not None
        assert result.info.size_bytes == 15

    def test_has(self, tmp_path):
        from eggsec import DirectoryArtifactStore
        store = DirectoryArtifactStore(str(tmp_path / "dir"))
        store.initialize()
        store.put("file.txt", b"content", "text/plain")
        assert store.has("file.txt") is True
        assert store.has("nonexistent.txt") is False

    def test_list_artifacts(self, tmp_path):
        from eggsec import DirectoryArtifactStore
        store = DirectoryArtifactStore(str(tmp_path / "dir"))
        store.initialize()
        store.put("a.txt", b"a", "text/plain")
        store.put("b.txt", b"b", "text/plain")
        items = store.list_artifacts(10, 0)
        assert len(items) == 2

    def test_serialization(self, tmp_path):
        from eggsec import DirectoryArtifactStore
        store = DirectoryArtifactStore(str(tmp_path / "dir"))
        store.initialize()
        store.put("file.txt", b"data", "text/plain")
        d = store.to_dict()
        assert d["base_dir"] == str(tmp_path / "dir")
        j = store.to_json()
        assert "file.txt" in j

    def test_repr(self, tmp_path):
        from eggsec import DirectoryArtifactStore
        store = DirectoryArtifactStore(str(tmp_path / "dir"))
        r = repr(store)
        assert "DirectoryArtifactStore" in r

    def test_context_manager(self, tmp_path):
        from eggsec import DirectoryArtifactStore
        with DirectoryArtifactStore(str(tmp_path / "dir")) as store:
            store.initialize()
            store.put("file.txt", b"data", "text/plain")


# ============================================================================
# WS26-27: Streaming Reporter Tests
# ============================================================================

class TestStreamingReportConfig:
    def test_construction(self):
        from eggsec import StreamingReportConfig
        cfg = StreamingReportConfig("json")
        assert cfg.format == "json"
        assert cfg.output_path is None
        assert cfg.buffer_size == 100
        assert cfg.include_artifacts is False
        assert cfg.include_evidence is False
        assert cfg.redact_secrets is True
        assert cfg.timestamp_format == "rfc3339"

    def test_construction_with_values(self):
        from eggsec import StreamingReportConfig
        cfg = StreamingReportConfig(
            "sarif",
            output_path="/tmp/report.sarif",
            buffer_size=50,
            include_artifacts=True,
            include_evidence=True,
            redact_secrets=False,
            timestamp_format="unix",
        )
        assert cfg.format == "sarif"
        assert cfg.output_path == "/tmp/report.sarif"
        assert cfg.buffer_size == 50
        assert cfg.redact_secrets is False

    def test_serialization(self):
        from eggsec import StreamingReportConfig
        cfg = StreamingReportConfig("json")
        d = cfg.to_dict()
        assert d["format"] == "json"
        j = cfg.to_json()
        assert "json" in j

    def test_repr(self):
        from eggsec import StreamingReportConfig
        cfg = StreamingReportConfig("json")
        assert "json" in repr(cfg)


class TestStreamingReporter:
    def test_construction(self):
        from eggsec import StreamingReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json")
        reporter = StreamingReporter(cfg)
        assert repr(reporter)

    def test_start_and_write(self):
        from eggsec import StreamingReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json")
        reporter = StreamingReporter(cfg)
        reporter.start()
        reporter.write_finding('{"id":"f1","severity":"high","title":"Test"}')
        assert reporter.get_buffered_count() == 1

    def test_flush(self):
        from eggsec import StreamingReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json")
        reporter = StreamingReporter(cfg)
        reporter.start()
        reporter.write_finding('{"id":"f1","severity":"high","title":"Test"}')
        reporter.flush()
        assert reporter.get_buffered_count() == 0

    def test_finish(self):
        from eggsec import StreamingReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json")
        reporter = StreamingReporter(cfg)
        reporter.start()
        reporter.write_finding('{"id":"f1","severity":"high","title":"Test"}')
        reporter.write_finding('{"id":"f2","severity":"low","title":"Test2"}')
        summary = reporter.finish()
        assert summary.format == "json"
        assert summary.total_findings == 0

    def test_start_already_started_error(self):
        from eggsec import StreamingReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json")
        reporter = StreamingReporter(cfg)
        reporter.start()
        with pytest.raises(Exception):
            reporter.start()

    def test_write_before_start_error(self):
        from eggsec import StreamingReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json")
        reporter = StreamingReporter(cfg)
        with pytest.raises(Exception):
            reporter.write_finding('{"id":"f1"}')

    def test_serialization(self):
        from eggsec import StreamingReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json")
        reporter = StreamingReporter(cfg)
        d = reporter.to_dict()
        assert "started" in d
        j = reporter.to_json()
        assert "json" in j

    def test_repr(self):
        from eggsec import StreamingReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json")
        reporter = StreamingReporter(cfg)
        assert "json" in repr(reporter)

    def test_with_output_file(self, tmp_path):
        from eggsec import StreamingReporter, StreamingReportConfig
        out = str(tmp_path / "report.jsonl")
        cfg = StreamingReportConfig("json", output_path=out)
        reporter = StreamingReporter(cfg)
        reporter.start()
        reporter.write_finding('{"id":"f1","severity":"high"}')
        reporter.finish()
        assert os.path.exists(out)


class TestReportSummary:
    def test_has_serialization_methods(self):
        from eggsec import ReportSummary
        assert hasattr(ReportSummary, 'to_dict')
        assert hasattr(ReportSummary, 'to_json')
        assert hasattr(ReportSummary, '__repr__')


class TestStreamingDiffReporter:
    def test_construction(self):
        from eggsec import StreamingDiffReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json")
        reporter = StreamingDiffReporter(cfg)
        assert repr(reporter)

    def test_construction_with_baseline(self):
        from eggsec import StreamingDiffReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json")
        baseline = '{"findings":[{"id":"f1","severity":"high","title":"Old"}]}'
        reporter = StreamingDiffReporter(cfg, baseline_json=baseline)
        assert repr(reporter)

    def test_start_and_write(self):
        from eggsec import StreamingDiffReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json")
        reporter = StreamingDiffReporter(cfg)
        reporter.start()
        result = reporter.write_finding('{"id":"f1","severity":"high","title":"New"}')
        assert result is not None
        assert result.diff_status == "new"

    def test_diff_against_baseline(self):
        from eggsec import StreamingDiffReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json")
        baseline = '{"findings":[{"id":"f1","severity":"high","title":"Old"}]}'
        reporter = StreamingDiffReporter(cfg, baseline_json=baseline)
        reporter.start()
        result = reporter.write_finding('{"id":"f1","severity":"low","title":"Old"}')
        assert result.diff_status == "changed"
        assert any("severity" in c for c in result.changes)

    def test_finish(self):
        from eggsec import StreamingDiffReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json")
        reporter = StreamingDiffReporter(cfg)
        reporter.start()
        reporter.write_finding('{"id":"f1","severity":"high","title":"New"}')
        summary = reporter.finish()
        assert summary.total_findings == 0
        assert summary.new_findings == 0

    def test_serialization(self):
        from eggsec import StreamingDiffReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json")
        reporter = StreamingDiffReporter(cfg)
        d = reporter.to_dict()
        assert "started" in d
        j = reporter.to_json()
        assert "json" in j

    def test_repr(self):
        from eggsec import StreamingDiffReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json")
        reporter = StreamingDiffReporter(cfg)
        assert "json" in repr(reporter)


class TestFindingDiffResult:
    def test_has_serialization_methods(self):
        from eggsec import FindingDiffResult
        assert hasattr(FindingDiffResult, 'to_dict')
        assert hasattr(FindingDiffResult, 'to_json')
        assert hasattr(FindingDiffResult, '__repr__')


class TestDiffReportSummary:
    def test_has_serialization_methods(self):
        from eggsec import DiffReportSummary
        assert hasattr(DiffReportSummary, 'to_dict')
        assert hasattr(DiffReportSummary, 'to_json')
        assert hasattr(DiffReportSummary, '__repr__')


class TestReportManifest:
    def test_construction(self):
        from eggsec import ReportManifest
        m = ReportManifest(
            "rep-1", "json", 1000, 5, "abc123",
        )
        assert m.report_id == "rep-1"
        assert m.format == "json"
        assert m.created_at_ms == 1000
        assert m.finding_count == 5
        assert m.content_hash == "abc123"
        assert m.schema_version == "1.0.0"
        assert m.tool_version
        assert m.artifact_ids == []

    def test_construction_with_artifacts(self):
        from eggsec import ReportManifest
        m = ReportManifest(
            "rep-1", "json", 1000, 5, "abc123",
            schema_version="2.0.0",
            tool_version="1.0",
            artifact_ids=["art-1", "art-2"],
        )
        assert m.schema_version == "2.0.0"
        assert len(m.artifact_ids) == 2

    def test_serialization(self):
        from eggsec import ReportManifest
        m = ReportManifest("rep-1", "json", 1000, 5, "abc123")
        d = m.to_dict()
        assert d["report_id"] == "rep-1"
        assert d["finding_count"] == 5
        j = m.to_json()
        assert "rep-1" in j

    def test_repr(self):
        from eggsec import ReportManifest
        m = ReportManifest("rep-1", "json", 1000, 5, "abc123")
        assert "rep-1" in repr(m)


# ============================================================================
# Serialization Round-Trip Tests (Section 8)
# ============================================================================

class TestSerializationRoundTrips:
    def test_session_identity_roundtrip(self):
        import json as json_mod
        from eggsec import SessionIdentity
        si = SessionIdentity("sess-1", "browser", 1000, owner_id="user-1")
        j = si.to_json()
        parsed = json_mod.loads(j)
        assert parsed["session_id"] == "sess-1"
        assert parsed["owner_id"] == "user-1"

    def test_session_stats_roundtrip(self):
        import json as json_mod
        from eggsec import SessionStats
        s = SessionStats(total_operations=10, elapsed_ms=5000)
        j = s.to_json()
        parsed = json_mod.loads(j)
        assert parsed["total_operations"] == 10

    def test_session_event_roundtrip(self):
        import json as json_mod
        from eggsec import SessionEvent
        e = SessionEvent(1, 1000, "started", message="ok")
        j = e.to_json()
        parsed = json_mod.loads(j)
        assert parsed["sequence"] == 1
        assert parsed["message"] == "ok"

    def test_session_event_stream_roundtrip(self):
        import json as json_mod
        from eggsec import SessionEventStream
        s = SessionEventStream("sess-1")
        j = s.to_json()
        parsed = json_mod.loads(j)
        assert parsed["session_id"] == "sess-1"

    def test_session_capabilities_roundtrip(self):
        import json as json_mod
        from eggsec import SessionCapabilities
        c = SessionCapabilities(supports_cancellation=True, max_concurrent_operations=4)
        j = c.to_json()
        parsed = json_mod.loads(j)
        assert parsed["supports_cancellation"] is True
        assert parsed["max_concurrent_operations"] == 4

    def test_mobile_session_config_roundtrip(self):
        import json as json_mod
        from eggsec import MobileSessionConfig
        cfg = MobileSessionConfig("serial-1", package_id="com.app", dry_run=True)
        j = cfg.to_json()
        parsed = json_mod.loads(j)
        assert parsed["device_serial"] == "serial-1"
        assert parsed["dry_run"] is True

    def test_browser_session_config_roundtrip(self):
        import json as json_mod
        from eggsec import BrowserSessionConfig
        cfg = BrowserSessionConfig(target_url="https://example.com", headless=False)
        j = cfg.to_json()
        parsed = json_mod.loads(j)
        assert parsed["target_url"] == "https://example.com"
        assert parsed["headless"] is False

    def test_daemon_protocol_version_roundtrip(self):
        import json as json_mod
        from eggsec import DaemonProtocolVersion
        dpv = DaemonProtocolVersion(1, "default", "standard")
        j = dpv.to_json()
        parsed = json_mod.loads(j)
        assert parsed["protocol_version"] == 2
        assert parsed["feature_profile"] == "standard"

    def test_reconnect_options_roundtrip(self):
        import json as json_mod
        from eggsec import ReconnectOptions
        ro = ReconnectOptions(max_retries=10, replay_from_sequence=42)
        j = ro.to_json()
        parsed = json_mod.loads(j)
        assert parsed["max_retries"] == 10
        assert parsed["replay_from_sequence"] == 42

    def test_sqlite_migration_roundtrip(self):
        import json as json_mod
        from eggsec import SqliteMigration
        m = SqliteMigration(1, "Create tables", 1000)
        j = m.to_json()
        parsed = json_mod.loads(j)
        assert parsed["version"] == 1
        assert parsed["description"] == "Create tables"

    def test_artifact_info_roundtrip(self):
        import json as json_mod
        from eggsec import ArtifactInfo
        info = ArtifactInfo("art-1", "abc123", "text/plain", 100)
        j = info.to_json()
        parsed = json_mod.loads(j)
        assert parsed["artifact_id"] == "art-1"
        assert parsed["content_hash"] == "abc123"

    def test_integrity_result_roundtrip(self):
        import json as json_mod
        from eggsec import IntegrityResult
        ir = IntegrityResult(True, "abc", "abc", 100)
        j = ir.to_json()
        parsed = json_mod.loads(j)
        assert parsed["valid"] is True

    def test_artifact_query_roundtrip(self):
        import json as json_mod
        from eggsec import ArtifactQuery
        q = ArtifactQuery(content_type="text/plain", limit=50)
        j = q.to_json()
        parsed = json_mod.loads(j)
        assert parsed["content_type"] == "text/plain"
        assert parsed["limit"] == 50

    def test_streaming_report_config_roundtrip(self):
        import json as json_mod
        from eggsec import StreamingReportConfig
        cfg = StreamingReportConfig("sarif", buffer_size=50)
        j = cfg.to_json()
        parsed = json_mod.loads(j)
        assert parsed["format"] == "sarif"
        assert parsed["buffer_size"] == 50

    def test_report_manifest_roundtrip(self):
        import json as json_mod
        from eggsec import ReportManifest
        m = ReportManifest("rep-1", "json", 1000, 5, "abc123")
        j = m.to_json()
        parsed = json_mod.loads(j)
        assert parsed["report_id"] == "rep-1"
        assert parsed["finding_count"] == 5

    def test_valid_json_output(self):
        """All to_json() outputs must be valid JSON."""
        from eggsec import (
            SessionIdentity, SessionStats, SessionEvent, SessionEventStream,
            SessionCapabilities, MobileSessionConfig, BrowserSessionConfig,
            DaemonProtocolVersion, ReconnectOptions, SqliteMigration,
            ArtifactInfo, IntegrityResult, ArtifactQuery,
            StreamingReportConfig, ReportManifest,
        )
        types_and_instances = [
            SessionIdentity("s", "b", 0),
            SessionStats(),
            SessionEvent(1, 0, "test"),
            SessionEventStream("s"),
            SessionCapabilities(),
            MobileSessionConfig("serial"),
            BrowserSessionConfig(),
            DaemonProtocolVersion(1, "r", "p"),
            ReconnectOptions(),
            SqliteMigration(1, "desc", 0),
            ArtifactInfo("a", "h", "t", 0),
            IntegrityResult(True, "h", "h", 0),
            ArtifactQuery(),
            StreamingReportConfig("json"),
            ReportManifest("r", "json", 0, 0, "h"),
        ]
        for obj in types_and_instances:
            j = obj.to_json()
            parsed = json.loads(j)
            assert parsed is not None


# ============================================================================
# Repr/Str tests for all constructible types
# ============================================================================

class TestReprStr:
    def test_all_constructible_types_have_repr(self):
        from eggsec import (
            SessionState, SessionIdentity, SessionStats, SessionCloseMode,
            SessionEvent, SessionEventStream, SessionCapabilities,
            MobileSessionConfig, MobileSessionState, MobileSession,
            AsyncMobileSession, MobileDeviceRegistry,
            BrowserSessionState, BrowserSessionConfig, BrowserSession,
            AsyncBrowserSession,
            DaemonProtocolVersion, ReconnectOptions,
            SqliteFindingRepository, SqliteAssessmentRepository,
            SqliteMigration,
            ContentAddressedArtifactStore, DirectoryArtifactStore,
            ArtifactInfo, IntegrityResult, ArtifactQuery,
            StreamingReportConfig, StreamingReporter, StreamingDiffReporter,
            ReportManifest,
        )
        instances = [
            SessionState.Created,
            SessionIdentity("s", "b", 0),
            SessionStats(),
            SessionCloseMode.Graceful,
            SessionEvent(1, 0, "test"),
            SessionEventStream("s"),
            SessionCapabilities(),
            MobileSessionConfig("serial"),
            MobileSessionState.Created,
            MobileSession("s", "d", MobileSessionConfig("d")),
            AsyncMobileSession("s", "d", MobileSessionConfig("d")),
            MobileDeviceRegistry(),
            BrowserSessionState.Created,
            BrowserSessionConfig(),
            BrowserSession(BrowserSessionConfig()),
            AsyncBrowserSession(BrowserSessionConfig()),
            DaemonProtocolVersion(1, "r", "p"),
            ReconnectOptions(),
            SqliteFindingRepository(":memory:"),
            SqliteAssessmentRepository(":memory:"),
            SqliteMigration(1, "desc", 0),
            ContentAddressedArtifactStore("/tmp/ca"),
            DirectoryArtifactStore("/tmp/dir"),
            ArtifactInfo("a", "h", "t", 0),
            IntegrityResult(True, "h", "h", 0),
            ArtifactQuery(),
            StreamingReportConfig("json"),
            StreamingReporter(StreamingReportConfig("json")),
            StreamingDiffReporter(StreamingReportConfig("json")),
            ReportManifest("r", "json", 0, 0, "h"),
        ]
        for obj in instances:
            r = repr(obj)
            assert isinstance(r, str) and len(r) > 0
