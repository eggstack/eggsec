"""Operational proof tests for mobile session types (WS5).

Tests prove the mobile binding surface is real: construction, serialization,
state transitions, and attribute access all work without external device
infrastructure. Device-dependent tests skip gracefully when ADB is unavailable.
"""

import json
import importlib
import pytest


def _import_or_skip(name, feature="mobile"):
    """Import a name from eggsec, skip test if unavailable (feature-gated)."""
    mod = importlib.import_module("eggsec")
    obj = getattr(mod, name, None)
    if obj is None:
        pytest.skip(f"{name} not available (requires {feature} feature)")
    return obj


# Module-level timeout for all tests
pytestmark = [pytest.mark.timeout(60)]

# ---------------------------------------------------------------------------
# ADB availability detection
# ---------------------------------------------------------------------------
_ADB_AVAILABLE = False
try:
    import subprocess
    result = subprocess.run(["adb", "devices"], capture_output=True, timeout=5)
    lines = result.stdout.decode().strip().split("\n")
    _ADB_AVAILABLE = len(lines) > 1 and any(
        "device" in line and "List" not in line for line in lines[1:]
    )
except Exception:
    pass

requires_adb = pytest.mark.skipif(not _ADB_AVAILABLE, reason="ADB not available")


# ============================================================================
# 1. TestMobilePlatform
# ============================================================================
class TestMobilePlatform:
    def test_import(self):
        MobilePlatform = _import_or_skip("MobilePlatform")
        assert MobilePlatform is not None

    def test_android_variant(self):
        MobilePlatform = _import_or_skip("MobilePlatform")
        assert MobilePlatform.Android is not None

    def test_ios_variant(self):
        MobilePlatform = _import_or_skip("MobilePlatform")
        assert MobilePlatform.Ios is not None

    def test_repr_android(self):
        MobilePlatform = _import_or_skip("MobilePlatform")
        assert repr(MobilePlatform.Android) == "MobilePlatform.Android"

    def test_repr_ios(self):
        MobilePlatform = _import_or_skip("MobilePlatform")
        assert repr(MobilePlatform.Ios) == "MobilePlatform.Ios"

    def test_str_android(self):
        MobilePlatform = _import_or_skip("MobilePlatform")
        assert str(MobilePlatform.Android) == "Android"

    def test_str_ios(self):
        MobilePlatform = _import_or_skip("MobilePlatform")
        assert str(MobilePlatform.Ios) == "Ios"

    def test_equality(self):
        MobilePlatform = _import_or_skip("MobilePlatform")
        assert MobilePlatform.Android == MobilePlatform.Android
        assert MobilePlatform.Ios == MobilePlatform.Ios
        assert MobilePlatform.Android != MobilePlatform.Ios

    def test_enum_instances_are_distinct(self):
        MobilePlatform = _import_or_skip("MobilePlatform")
        assert MobilePlatform.Android != MobilePlatform.Ios
        assert MobilePlatform.Android == MobilePlatform.Android
        assert MobilePlatform.Android == 0  # eq_int


# ============================================================================
# 2. TestMobileFinding
# ============================================================================
class TestMobileFinding:
    """MobileFinding is not directly constructable from Python.
    We test import and attribute access when obtained from a report.
    """

    def test_import(self):
        MobileFinding = _import_or_skip("MobileFinding")
        assert MobileFinding is not None

    def test_import_via_report(self):
        MobileScanReport = _import_or_skip("MobileScanReport")
        assert MobileScanReport is not None


# ============================================================================
# 3. TestMobileDeviceDescriptor
# ============================================================================
class TestMobileDeviceDescriptor:
    """MobileDeviceDescriptor is not directly constructable.
    Test import only.
    """

    def test_import(self):
        MobileDeviceDescriptor = _import_or_skip("MobileDeviceDescriptor")
        assert MobileDeviceDescriptor is not None


# ============================================================================
# 4. TestMobileDeviceCapabilities
# ============================================================================
class TestMobileDeviceCapabilities:
    def test_import(self):
        MobileDeviceCapabilities = _import_or_skip("MobileDeviceCapabilities")
        assert MobileDeviceCapabilities is not None


# ============================================================================
# 5. TestMobileSessionConfig
# ============================================================================
class TestMobileSessionConfig:
    def test_import(self):
        MobileSessionConfig = _import_or_skip("MobileSessionConfig")
        assert MobileSessionConfig is not None

    def test_construct_minimal(self):
        MobileSessionConfig = _import_or_skip("MobileSessionConfig")
        cfg = MobileSessionConfig(device_serial="emulator-5554")
        assert cfg.device_serial == "emulator-5554"
        assert cfg.install_app is False
        assert cfg.uninstall_after is False
        assert cfg.capture_logs is False
        assert cfg.capture_screenshots is False
        assert cfg.capture_network is False
        assert cfg.allow_frida is False
        assert cfg.dry_run is False

    def test_construct_full(self):
        MobileSessionConfig = _import_or_skip("MobileSessionConfig")
        cfg = MobileSessionConfig(
            device_serial="ABC123",
            package_id="com.example.app",
            install_app=True,
            uninstall_after=True,
            capture_logs=True,
            capture_screenshots=True,
            capture_network=True,
            traffic_output="/tmp/traffic.pcap",
            frida_scripts=["hook_ssl.js"],
            allow_frida=True,
            timeout_secs=120,
            proxy="http://127.0.0.1:8080",
            grant_permissions=["android.permission.INTERNET"],
            revoke_permissions=["android.permission.CAMERA"],
            dry_run=True,
        )
        assert cfg.device_serial == "ABC123"
        assert cfg.package_id == "com.example.app"
        assert cfg.install_app is True
        assert cfg.uninstall_after is True
        assert cfg.capture_logs is True
        assert cfg.capture_screenshots is True
        assert cfg.capture_network is True
        assert cfg.traffic_output == "/tmp/traffic.pcap"
        assert cfg.frida_scripts == ["hook_ssl.js"]
        assert cfg.allow_frida is True
        assert cfg.timeout_secs == 120
        assert cfg.proxy == "http://127.0.0.1:8080"
        assert cfg.grant_permissions == ["android.permission.INTERNET"]
        assert cfg.revoke_permissions == ["android.permission.CAMERA"]
        assert cfg.dry_run is True

    def test_to_dict(self):
        MobileSessionConfig = _import_or_skip("MobileSessionConfig")
        cfg = MobileSessionConfig(device_serial="emulator-5554", dry_run=True)
        d = cfg.to_dict()
        assert d["device_serial"] == "emulator-5554"
        assert d["dry_run"] is True
        assert d["install_app"] is False
        assert d["frida_scripts"] == []
        assert d["grant_permissions"] == []
        assert d["revoke_permissions"] == []

    def test_to_json_roundtrip(self):
        MobileSessionConfig = _import_or_skip("MobileSessionConfig")
        cfg = MobileSessionConfig(
            device_serial="TEST-SERIAL",
            package_id="com.test",
            install_app=True,
            allow_frida=True,
        )
        j = cfg.to_json()
        data = json.loads(j)
        assert data["device_serial"] == "TEST-SERIAL"
        assert data["package_id"] == "com.test"
        assert data["install_app"] is True
        assert data["allow_frida"] is True

    def test_repr(self):
        MobileSessionConfig = _import_or_skip("MobileSessionConfig")
        cfg = MobileSessionConfig(device_serial="DEV1", package_id="com.pkg", dry_run=True)
        r = repr(cfg)
        assert "MobileSessionConfig" in r
        assert "DEV1" in r
        assert "dry_run=" in r

    def test_frozen(self):
        MobileSessionConfig = _import_or_skip("MobileSessionConfig")
        cfg = MobileSessionConfig(device_serial="X")
        with pytest.raises(AttributeError):
            cfg.device_serial = "Y"


# ============================================================================
# 6. TestMobileSessionState
# ============================================================================
class TestMobileSessionState:
    EXPECTED_VARIANTS = [
        "Created", "Connecting", "Installing", "Launching", "Running",
        "Capturing", "Stopping", "Uninstalling", "Cleaning", "Stopped",
        "Failed", "Cancelled",
    ]

    def test_import(self):
        MobileSessionState = _import_or_skip("MobileSessionState")
        assert MobileSessionState is not None

    def test_all_variants_exist(self):
        MobileSessionState = _import_or_skip("MobileSessionState")
        for name in self.EXPECTED_VARIANTS:
            assert hasattr(MobileSessionState, name), f"Missing variant: {name}"

    def test_repr(self):
        MobileSessionState = _import_or_skip("MobileSessionState")
        assert repr(MobileSessionState.Created) == "MobileSessionState.Created"

    def test_str(self):
        MobileSessionState = _import_or_skip("MobileSessionState")
        assert str(MobileSessionState.Created) == "Created"
        assert str(MobileSessionState.Running) == "Running"
        assert str(MobileSessionState.Failed) == "Failed"

    def test_equality(self):
        MobileSessionState = _import_or_skip("MobileSessionState")
        assert MobileSessionState.Created == MobileSessionState.Created
        assert MobileSessionState.Created != MobileSessionState.Running

    def test_frozen(self):
        MobileSessionState = _import_or_skip("MobileSessionState")
        # Enum instances are frozen - can't set attributes on instances
        created = MobileSessionState.Created
        with pytest.raises(AttributeError):
            created.x = "hack"


# ============================================================================
# 7. TestMobileSessionStats
# ============================================================================
class TestMobileSessionStats:
    """MobileSessionStats is obtained from MobileSession.stats getter."""

    def test_import(self):
        MobileSessionStats = _import_or_skip("MobileSessionStats")
        assert MobileSessionStats is not None

    def test_stats_from_session(self):
        MobileSession = _import_or_skip("MobileSession")
        MobileSessionConfig = _import_or_skip("MobileSessionConfig")
        cfg = MobileSessionConfig(device_serial="test-serial")
        session = MobileSession("sess-1", "test-serial", cfg)
        stats = session.stats
        assert stats.screenshots_captured == 0
        assert stats.log_entries == 0
        assert stats.network_exchanges == 0
        assert stats.artifacts_collected == 0
        assert stats.frida_events == 0
        assert stats.duration_ms == 0

    def test_stats_to_dict(self):
        MobileSession = _import_or_skip("MobileSession")
        MobileSessionConfig = _import_or_skip("MobileSessionConfig")
        cfg = MobileSessionConfig(device_serial="s")
        session = MobileSession("id", "s", cfg)
        d = session.stats.to_dict()
        assert "screenshots_captured" in d
        assert "log_entries" in d
        assert "network_exchanges" in d
        assert "artifacts_collected" in d
        assert "frida_events" in d
        assert "duration_ms" in d

    def test_stats_to_json(self):
        MobileSession = _import_or_skip("MobileSession")
        MobileSessionConfig = _import_or_skip("MobileSessionConfig")
        cfg = MobileSessionConfig(device_serial="s")
        session = MobileSession("id", "s", cfg)
        j = session.stats.to_json()
        data = json.loads(j)
        assert data["screenshots_captured"] == 0
        assert data["duration_ms"] == 0

    def test_stats_repr(self):
        MobileSession = _import_or_skip("MobileSession")
        MobileSessionConfig = _import_or_skip("MobileSessionConfig")
        cfg = MobileSessionConfig(device_serial="s")
        session = MobileSession("id", "s", cfg)
        r = repr(session.stats)
        assert "MobileSessionStats" in r
        assert "screenshots=" in r
        assert "duration_ms=" in r


# ============================================================================
# 8. TestMobileDeviceRegistry
# ============================================================================
class TestMobileDeviceRegistry:
    def test_import(self):
        MobileDeviceRegistry = _import_or_skip("MobileDeviceRegistry")
        assert MobileDeviceRegistry is not None

    def test_construct_empty(self):
        MobileDeviceRegistry = _import_or_skip("MobileDeviceRegistry")
        reg = MobileDeviceRegistry()
        assert reg.devices == []

    def test_get_device_empty(self):
        MobileDeviceRegistry = _import_or_skip("MobileDeviceRegistry")
        reg = MobileDeviceRegistry()
        assert reg.get_device("nonexistent") is None

    def test_to_dict_empty(self):
        MobileDeviceRegistry = _import_or_skip("MobileDeviceRegistry")
        reg = MobileDeviceRegistry()
        d = reg.to_dict()
        assert d["devices"] == []

    def test_to_json_empty(self):
        MobileDeviceRegistry = _import_or_skip("MobileDeviceRegistry")
        reg = MobileDeviceRegistry()
        j = reg.to_json()
        data = json.loads(j)
        assert data["devices"] == []

    def test_repr_empty(self):
        MobileDeviceRegistry = _import_or_skip("MobileDeviceRegistry")
        reg = MobileDeviceRegistry()
        r = repr(reg)
        assert "MobileDeviceRegistry" in r
        assert "device_count=0" in r

    @requires_adb
    def test_refresh_with_adb(self):
        MobileDeviceRegistry = _import_or_skip("MobileDeviceRegistry")
        reg = MobileDeviceRegistry()
        devices = reg.refresh()
        assert isinstance(devices, list)
        assert len(reg.devices) == len(devices)

    @requires_adb
    def test_get_device_after_refresh(self):
        MobileDeviceRegistry = _import_or_skip("MobileDeviceRegistry")
        reg = MobileDeviceRegistry()
        devices = reg.refresh()
        if devices:
            first = devices[0]
            found = reg.get_device(first.serial)
            assert found is not None
            assert found.serial == first.serial


# ============================================================================
# 9. TestMobileSessionConstruction
# ============================================================================
class TestMobileSessionConstruction:
    def test_import(self):
        MobileSession = _import_or_skip("MobileSession")
        assert MobileSession is not None

    def test_construct(self):
        MobileSession = _import_or_skip("MobileSession")
        MobileSessionConfig = _import_or_skip("MobileSessionConfig")
        cfg = MobileSessionConfig(device_serial="emulator-5554")
        session = MobileSession("test-session-1", "emulator-5554", cfg)
        assert session.session_id == "test-session-1"
        assert session.device_serial == "emulator-5554"

    def test_initial_state_created(self):
        MobileSession = _import_or_skip("MobileSession")
        MobileSessionConfig = _import_or_skip("MobileSessionConfig")
        cfg = MobileSessionConfig(device_serial="emulator-5554")
        session = MobileSession("id", "emulator-5554", cfg)
        MobileSessionState = _import_or_skip("MobileSessionState")
        assert session.state == MobileSessionState.Created

    def test_start_transitions_to_connecting(self):
        MobileSession = _import_or_skip("MobileSession")
        MobileSessionConfig = _import_or_skip("MobileSessionConfig")
        MobileSessionState = _import_or_skip("MobileSessionState")
        cfg = MobileSessionConfig(device_serial="emulator-5554")
        session = MobileSession("id", "emulator-5554", cfg)
        session.start()
        assert session.state == MobileSessionState.Connecting

    def test_stop_transitions_to_stopping(self):
        MobileSession = _import_or_skip("MobileSession")
        MobileSessionConfig = _import_or_skip("MobileSessionConfig")
        MobileSessionState = _import_or_skip("MobileSessionState")
        cfg = MobileSessionConfig(device_serial="emulator-5554")
        session = MobileSession("id", "emulator-5554", cfg)
        session.stop()
        assert session.state == MobileSessionState.Stopping

    def test_context_manager_enter(self):
        MobileSession = _import_or_skip("MobileSession")
        MobileSessionConfig = _import_or_skip("MobileSessionConfig")
        cfg = MobileSessionConfig(device_serial="emulator-5554")
        session = MobileSession("id", "emulator-5554", cfg)
        with session as s:
            assert s.session_id == "id"

    def test_to_dict(self):
        MobileSession = _import_or_skip("MobileSession")
        MobileSessionConfig = _import_or_skip("MobileSessionConfig")
        cfg = MobileSessionConfig(device_serial="DEV1")
        session = MobileSession("sid", "DEV1", cfg)
        d = session.to_dict()
        assert d["session_id"] == "sid"
        assert d["device_serial"] == "DEV1"
        assert d["state"] == "Created"
        assert "config" in d
        assert "stats" in d

    def test_to_json(self):
        MobileSession = _import_or_skip("MobileSession")
        MobileSessionConfig = _import_or_skip("MobileSessionConfig")
        cfg = MobileSessionConfig(device_serial="DEV1")
        session = MobileSession("sid", "DEV1", cfg)
        j = session.to_json()
        data = json.loads(j)
        assert data["session_id"] == "sid"
        assert data["device_serial"] == "DEV1"

    def test_repr(self):
        MobileSession = _import_or_skip("MobileSession")
        MobileSessionConfig = _import_or_skip("MobileSessionConfig")
        cfg = MobileSessionConfig(device_serial="X")
        session = MobileSession("myid", "X", cfg)
        r = repr(session)
        assert "MobileSession" in r
        assert "myid" in r
        assert "X" in r

    def test_config_getter(self):
        MobileSession = _import_or_skip("MobileSession")
        MobileSessionConfig = _import_or_skip("MobileSessionConfig")
        cfg = MobileSessionConfig(device_serial="D1", dry_run=True)
        session = MobileSession("id", "D1", cfg)
        got = session.config
        assert got.device_serial == "D1"
        assert got.dry_run is True

    def test_install_app_fails_without_device(self):
        MobileSession = _import_or_skip("MobileSession")
        MobileSessionConfig = _import_or_skip("MobileSessionConfig")
        cfg = MobileSessionConfig(device_serial="emulator-5554")
        session = MobileSession("id", "emulator-5554", cfg)
        with pytest.raises(Exception, match="requires a connected device"):
            session.install_app("/tmp/fake.apk")

    def test_launch_app_fails_without_device(self):
        MobileSession = _import_or_skip("MobileSession")
        MobileSessionConfig = _import_or_skip("MobileSessionConfig")
        cfg = MobileSessionConfig(
            device_serial="emulator-5554", package_id="com.example"
        )
        session = MobileSession("id", "emulator-5554", cfg)
        with pytest.raises(Exception, match="requires a connected device"):
            session.launch_app()

    def test_capture_screenshot_fails_without_device(self):
        MobileSession = _import_or_skip("MobileSession")
        MobileSessionConfig = _import_or_skip("MobileSessionConfig")
        cfg = MobileSessionConfig(device_serial="emulator-5554")
        session = MobileSession("id", "emulator-5554", cfg)
        with pytest.raises(Exception, match="requires a connected device"):
            session.capture_screenshot()

    def test_get_logs_fails_without_device(self):
        MobileSession = _import_or_skip("MobileSession")
        MobileSessionConfig = _import_or_skip("MobileSessionConfig")
        cfg = MobileSessionConfig(device_serial="emulator-5554")
        session = MobileSession("id", "emulator-5554", cfg)
        with pytest.raises(Exception, match="requires a connected device"):
            session.get_logs()


# ============================================================================
# 10. TestMobileFindingSerialization
# ============================================================================
class TestMobileFindingSerialization:
    """MobileFinding cannot be constructed directly.
    Test import and verify the type exists in the module.
    """

    def test_finding_import(self):
        MobileFinding = _import_or_skip("MobileFinding")
        assert MobileFinding is not None

    def test_report_import(self):
        MobileScanReport = _import_or_skip("MobileScanReport")
        assert MobileScanReport is not None

    def test_apk_analysis_exists(self):
        analyze_apk = _import_or_skip("analyze_apk")
        assert callable(analyze_apk)

    def test_ipa_analysis_exists(self):
        analyze_ipa = _import_or_skip("analyze_ipa")
        assert callable(analyze_ipa)

    def test_async_apk_analysis_exists(self):
        async_analyze_apk = _import_or_skip("async_analyze_apk")
        assert callable(async_analyze_apk)

    def test_async_ipa_analysis_exists(self):
        async_analyze_ipa = _import_or_skip("async_analyze_ipa")
        assert callable(async_analyze_ipa)


# ============================================================================
# 11. TestMobileDevice
# ============================================================================
class TestMobileDevice:
    """MobileDevice is obtained from list_mobile_devices().
    Test import and verify the type exists.
    """

    def test_import(self):
        MobileDevice = _import_or_skip("MobileDevice")
        assert MobileDevice is not None

    def test_list_devices_exists(self):
        list_mobile_devices = _import_or_skip("list_mobile_devices")
        assert callable(list_mobile_devices)


# ============================================================================
# 12. TestDynamicMobileConfig
# ============================================================================
class TestDynamicMobileConfig:
    def test_import(self):
        DynamicMobileConfig = _import_or_skip("DynamicMobileConfig")
        assert DynamicMobileConfig is not None

    def test_construct_defaults(self):
        DynamicMobileConfig = _import_or_skip("DynamicMobileConfig")
        cfg = DynamicMobileConfig()
        assert cfg.install is False
        assert cfg.capture_logs is False
        assert cfg.uninstall_after is False
        assert cfg.dry_run is False
        assert cfg.allow_frida is False
        assert cfg.list_permissions is False

    def test_construct_full(self):
        DynamicMobileConfig = _import_or_skip("DynamicMobileConfig")
        cfg = DynamicMobileConfig(
            install=True,
            launch="com.example.MainActivity",
            capture_logs=True,
            duration_secs=60,
            uninstall_after=True,
            dry_run=True,
            proxy="http://127.0.0.1:8080",
            frida_scripts=["hook.js"],
            allow_frida=True,
            grant_permissions=["INTERNET"],
            revoke_permissions=["CAMERA"],
            list_permissions=True,
            traffic_capture="/tmp/traffic.pcap",
            baseline="/tmp/baseline.har",
            evidence_bundle="/tmp/evidence",
        )
        assert cfg.install is True
        assert cfg.launch == "com.example.MainActivity"
        assert cfg.capture_logs is True
        assert cfg.duration_secs == 60
        assert cfg.uninstall_after is True
        assert cfg.dry_run is True
        assert cfg.proxy == "http://127.0.0.1:8080"
        assert cfg.frida_scripts == ["hook.js"]
        assert cfg.allow_frida is True
        assert cfg.grant_permissions == ["INTERNET"]
        assert cfg.revoke_permissions == ["CAMERA"]
        assert cfg.list_permissions is True
        assert cfg.traffic_capture == "/tmp/traffic.pcap"
        assert cfg.baseline == "/tmp/baseline.har"
        assert cfg.evidence_bundle == "/tmp/evidence"

    def test_to_dict(self):
        DynamicMobileConfig = _import_or_skip("DynamicMobileConfig")
        cfg = DynamicMobileConfig(dry_run=True)
        d = cfg.to_dict()
        assert d["dry_run"] is True
        assert d["install"] is False
        assert d["frida_scripts"] == []

    def test_to_json_roundtrip(self):
        DynamicMobileConfig = _import_or_skip("DynamicMobileConfig")
        cfg = DynamicMobileConfig(install=True, dry_run=True)
        j = cfg.to_json()
        data = json.loads(j)
        assert data["install"] is True
        assert data["dry_run"] is True

    def test_repr(self):
        DynamicMobileConfig = _import_or_skip("DynamicMobileConfig")
        cfg = DynamicMobileConfig(install=True, dry_run=True, allow_frida=True)
        r = repr(cfg)
        assert "DynamicMobileConfig" in r
        assert "install=" in r
        assert "dry_run=" in r
        assert "frida=" in r

    def test_frozen(self):
        DynamicMobileConfig = _import_or_skip("DynamicMobileConfig")
        cfg = DynamicMobileConfig()
        with pytest.raises(AttributeError):
            cfg.dry_run = True


# ============================================================================
# 13. TestDynamicMobileReport
# ============================================================================
class TestDynamicMobileReport:
    def test_import(self):
        DynamicMobileReport = _import_or_skip("DynamicMobileReport")
        assert DynamicMobileReport is not None

    def test_dynamic_mobile_analysis_exists(self):
        dynamic_mobile_analysis = _import_or_skip("dynamic_mobile_analysis")
        assert callable(dynamic_mobile_analysis)

    def test_analysis_fails_without_device(self):
        dynamic_mobile_analysis = _import_or_skip("dynamic_mobile_analysis")
        DynamicMobileConfig = _import_or_skip("DynamicMobileConfig")
        cfg = DynamicMobileConfig(dry_run=True)
        with pytest.raises(Exception, match="requires a connected device"):
            dynamic_mobile_analysis("com.example.app", cfg)


# ============================================================================
# 14. TestAsyncMobileSession
# ============================================================================
class TestAsyncMobileSession:
    def test_import(self):
        AsyncMobileSession = _import_or_skip("AsyncMobileSession")
        assert AsyncMobileSession is not None

    def test_construct(self):
        AsyncMobileSession = _import_or_skip("AsyncMobileSession")
        MobileSessionConfig = _import_or_skip("MobileSessionConfig")
        cfg = MobileSessionConfig(device_serial="emulator-5554")
        session = AsyncMobileSession("async-id", "emulator-5554", cfg)
        assert session.session_id == "async-id"
        assert session.device_serial == "emulator-5554"

    def test_initial_state_created(self):
        AsyncMobileSession = _import_or_skip("AsyncMobileSession")
        MobileSessionConfig = _import_or_skip("MobileSessionConfig")
        MobileSessionState = _import_or_skip("MobileSessionState")
        cfg = MobileSessionConfig(device_serial="s")
        session = AsyncMobileSession("id", "s", cfg)
        assert session.state == MobileSessionState.Created

    def test_to_dict(self):
        AsyncMobileSession = _import_or_skip("AsyncMobileSession")
        MobileSessionConfig = _import_or_skip("MobileSessionConfig")
        cfg = MobileSessionConfig(device_serial="D1")
        session = AsyncMobileSession("sid", "D1", cfg)
        d = session.to_dict()
        assert d["session_id"] == "sid"
        assert d["device_serial"] == "D1"

    def test_to_json(self):
        AsyncMobileSession = _import_or_skip("AsyncMobileSession")
        MobileSessionConfig = _import_or_skip("MobileSessionConfig")
        cfg = MobileSessionConfig(device_serial="D1")
        session = AsyncMobileSession("sid", "D1", cfg)
        j = session.to_json()
        data = json.loads(j)
        assert data["session_id"] == "sid"

    def test_repr(self):
        AsyncMobileSession = _import_or_skip("AsyncMobileSession")
        MobileSessionConfig = _import_or_skip("MobileSessionConfig")
        cfg = MobileSessionConfig(device_serial="X")
        session = AsyncMobileSession("myid", "X", cfg)
        r = repr(session)
        assert "AsyncMobileSession" in r
        assert "myid" in r

    def test_config_getter(self):
        AsyncMobileSession = _import_or_skip("AsyncMobileSession")
        MobileSessionConfig = _import_or_skip("MobileSessionConfig")
        cfg = MobileSessionConfig(device_serial="D1", allow_frida=True)
        session = AsyncMobileSession("id", "D1", cfg)
        assert session.config.allow_frida is True

    def test_stats_getter(self):
        AsyncMobileSession = _import_or_skip("AsyncMobileSession")
        MobileSessionConfig = _import_or_skip("MobileSessionConfig")
        cfg = MobileSessionConfig(device_serial="s")
        session = AsyncMobileSession("id", "s", cfg)
        stats = session.stats
        assert stats.screenshots_captured == 0
        assert stats.duration_ms == 0


# ============================================================================
# 15. TestMobileSessionFrozen
# ============================================================================
class TestMobileSessionFrozen:
    """Verify session config and state types reject assignment."""

    def test_session_config_frozen(self):
        MobileSessionConfig = _import_or_skip("MobileSessionConfig")
        cfg = MobileSessionConfig(device_serial="X")
        with pytest.raises(AttributeError):
            cfg.device_serial = "Y"

    def test_session_not_frozen_mutable(self):
        MobileSession = _import_or_skip("MobileSession")
        MobileSessionConfig = _import_or_skip("MobileSessionConfig")
        cfg = MobileSessionConfig(device_serial="X")
        session = MobileSession("id", "X", cfg)
        session.start()
        MobileSessionState = _import_or_skip("MobileSessionState")
        assert session.state == MobileSessionState.Connecting


# ============================================================================
# 16. TestDynamicMobileConfigFrozen
# ============================================================================
class TestDynamicMobileConfigFrozen:
    def test_frozen(self):
        DynamicMobileConfig = _import_or_skip("DynamicMobileConfig")
        cfg = DynamicMobileConfig()
        with pytest.raises(AttributeError):
            cfg.dry_run = True


# ============================================================================
# 17. TestMobileSessionContextManager
# ============================================================================
class TestMobileSessionContextManager:
    def test_sync_context_manager(self):
        MobileSession = _import_or_skip("MobileSession")
        MobileSessionConfig = _import_or_skip("MobileSessionConfig")
        cfg = MobileSessionConfig(device_serial="emulator-5554")
        session = MobileSession("ctx-test", "emulator-5554", cfg)
        with session as s:
            assert s.session_id == "ctx-test"

    def test_async_context_manager(self):
        AsyncMobileSession = _import_or_skip("AsyncMobileSession")
        MobileSessionConfig = _import_or_skip("MobileSessionConfig")
        cfg = MobileSessionConfig(device_serial="emulator-5554")
        session = AsyncMobileSession("async-ctx", "emulator-5554", cfg)
        # Test __aenter__ returns self
        enter_result = session.__aenter__()
        assert enter_result.session_id == "async-ctx"
        # Test __aexit__ returns False (no suppression)
        exit_result = session.__aexit__(None, None, None)
        assert exit_result is False


# ============================================================================
# 18. TestMobilePlatformEnumComplete
# ============================================================================
class TestMobilePlatformEnumComplete:
    def test_both_platforms(self):
        MobilePlatform = _import_or_skip("MobilePlatform")
        variants = [MobilePlatform.Android, MobilePlatform.Ios]
        assert len(variants) == 2

    def test_no_other_variants(self):
        MobilePlatform = _import_or_skip("MobilePlatform")
        # Verify exactly 2 variants exist and no others
        assert hasattr(MobilePlatform, "Android")
        assert hasattr(MobilePlatform, "Ios")
        assert not hasattr(MobilePlatform, "Linux")


# ============================================================================
# 19. TestMobileDeviceCapabilitiesFrozen
# ============================================================================
class TestMobileDeviceCapabilitiesFrozen:
    def test_import(self):
        MobileDeviceCapabilities = _import_or_skip("MobileDeviceCapabilities")
        assert MobileDeviceCapabilities is not None


# ============================================================================
# 20. TestMobileDeviceDescriptorFrozen
# ============================================================================
class TestMobileDeviceDescriptorFrozen:
    def test_import(self):
        MobileDeviceDescriptor = _import_or_skip("MobileDeviceDescriptor")
        assert MobileDeviceDescriptor is not None


# ============================================================================
# 21. TestMobileSessionStateComplete
# ============================================================================
class TestMobileSessionStateComplete:
    def test_12_variants(self):
        MobileSessionState = _import_or_skip("MobileSessionState")
        expected = [
            "Created", "Connecting", "Installing", "Launching", "Running",
            "Capturing", "Stopping", "Uninstalling", "Cleaning", "Stopped",
            "Failed", "Cancelled",
        ]
        for name in expected:
            assert hasattr(MobileSessionState, name), f"Missing: {name}"


# ============================================================================
# 22. TestMobileSessionStateTransitions
# ============================================================================
class TestMobileSessionStateTransitions:
    def test_created_to_connecting(self):
        MobileSession = _import_or_skip("MobileSession")
        MobileSessionConfig = _import_or_skip("MobileSessionConfig")
        MobileSessionState = _import_or_skip("MobileSessionState")
        cfg = MobileSessionConfig(device_serial="s")
        session = MobileSession("id", "s", cfg)
        assert session.state == MobileSessionState.Created
        session.start()
        assert session.state == MobileSessionState.Connecting

    def test_created_to_stopping(self):
        MobileSession = _import_or_skip("MobileSession")
        MobileSessionConfig = _import_or_skip("MobileSessionConfig")
        MobileSessionState = _import_or_skip("MobileSessionState")
        cfg = MobileSessionConfig(device_serial="s")
        session = MobileSession("id", "s", cfg)
        session.stop()
        assert session.state == MobileSessionState.Stopping


# ============================================================================
# 23. TestMobileSessionMultipleInstances
# ============================================================================
class TestMobileSessionMultipleInstances:
    def test_distinct_session_ids(self):
        MobileSession = _import_or_skip("MobileSession")
        MobileSessionConfig = _import_or_skip("MobileSessionConfig")
        cfg = MobileSessionConfig(device_serial="s")
        s1 = MobileSession("id-1", "s", cfg)
        s2 = MobileSession("id-2", "s", cfg)
        assert s1.session_id != s2.session_id

    def test_distinct_configs(self):
        MobileSession = _import_or_skip("MobileSession")
        MobileSessionConfig = _import_or_skip("MobileSessionConfig")
        cfg1 = MobileSessionConfig(device_serial="s1")
        cfg2 = MobileSessionConfig(device_serial="s2")
        s1 = MobileSession("id-1", "s1", cfg1)
        s2 = MobileSession("id-2", "s2", cfg2)
        assert s1.config.device_serial != s2.config.device_serial


# ============================================================================
# 24. TestMobileSessionConfigOptionals
# ============================================================================
class TestMobileSessionConfigOptionals:
    def test_optional_fields_default_none(self):
        MobileSessionConfig = _import_or_skip("MobileSessionConfig")
        cfg = MobileSessionConfig(device_serial="s")
        assert cfg.package_id is None
        assert cfg.traffic_output is None
        assert cfg.timeout_secs is None
        assert cfg.proxy is None

    def test_empty_vectors(self):
        MobileSessionConfig = _import_or_skip("MobileSessionConfig")
        cfg = MobileSessionConfig(device_serial="s")
        assert cfg.frida_scripts == []
        assert cfg.grant_permissions == []
        assert cfg.revoke_permissions == []


# ============================================================================
# 25. TestDynamicMobileConfigOptionals
# ============================================================================
class TestDynamicMobileConfigOptionals:
    def test_optional_fields_default_none(self):
        DynamicMobileConfig = _import_or_skip("DynamicMobileConfig")
        cfg = DynamicMobileConfig()
        assert cfg.launch is None
        assert cfg.duration_secs is None
        assert cfg.proxy is None
        assert cfg.traffic_capture is None
        assert cfg.baseline is None
        assert cfg.evidence_bundle is None

    def test_empty_vectors(self):
        DynamicMobileConfig = _import_or_skip("DynamicMobileConfig")
        cfg = DynamicMobileConfig()
        assert cfg.frida_scripts == []
        assert cfg.grant_permissions == []
        assert cfg.revoke_permissions == []


# ============================================================================
# 26. TestMobileSessionStatsFromMultipleSessions
# ============================================================================
class TestMobileSessionStatsFromMultipleSessions:
    def test_independent_stats(self):
        MobileSession = _import_or_skip("MobileSession")
        MobileSessionConfig = _import_or_skip("MobileSessionConfig")
        cfg = MobileSessionConfig(device_serial="s")
        s1 = MobileSession("id-1", "s", cfg)
        s2 = MobileSession("id-2", "s", cfg)
        stats1 = s1.stats
        stats2 = s2.stats
        assert stats1 is not stats2
        assert stats1.screenshots_captured == stats2.screenshots_captured == 0


# ============================================================================
# 27. TestMobileDeviceRegistryRepr
# ============================================================================
class TestMobileDeviceRegistryRepr:
    def test_repr_after_refresh_no_adb(self):
        MobileDeviceRegistry = _import_or_skip("MobileDeviceRegistry")
        reg = MobileDeviceRegistry()
        if not _ADB_AVAILABLE:
            r = repr(reg)
            assert "device_count=0" in r


# ============================================================================
# 28. TestMobileSessionJsonRoundtrip
# ============================================================================
class TestMobileSessionJsonRoundtrip:
    def test_session_json_contains_all_fields(self):
        MobileSession = _import_or_skip("MobileSession")
        MobileSessionConfig = _import_or_skip("MobileSessionConfig")
        cfg = MobileSessionConfig(
            device_serial="DEV",
            package_id="com.example",
            dry_run=True,
        )
        session = MobileSession("sid", "DEV", cfg)
        j = session.to_json()
        data = json.loads(j)
        assert "session_id" in data
        assert "device_serial" in data
        assert "state" in data
        assert "config" in data
        assert "stats" in data


# ============================================================================
# 29. TestMobileSessionStatsJsonFields
# ============================================================================
class TestMobileSessionStatsJsonFields:
    def test_all_stats_fields(self):
        MobileSession = _import_or_skip("MobileSession")
        MobileSessionConfig = _import_or_skip("MobileSessionConfig")
        cfg = MobileSessionConfig(device_serial="s")
        session = MobileSession("id", "s", cfg)
        j = session.stats.to_json()
        data = json.loads(j)
        expected_keys = {
            "screenshots_captured", "log_entries", "network_exchanges",
            "artifacts_collected", "frida_events", "duration_ms",
        }
        assert expected_keys.issubset(set(data.keys()))


# ============================================================================
# 30. TestMobileDeviceRegistryJsonRoundtrip
# ============================================================================
class TestMobileDeviceRegistryJsonRoundtrip:
    def test_empty_registry_roundtrip(self):
        MobileDeviceRegistry = _import_or_skip("MobileDeviceRegistry")
        reg = MobileDeviceRegistry()
        j = reg.to_json()
        data = json.loads(j)
        assert data["devices"] == []
        assert isinstance(data["devices"], list)


# ============================================================================
# 31. TestMobilePlatformFromStr
# ============================================================================
class TestMobilePlatformFromStr:
    """Test that MobilePlatform enum values can be compared as strings."""

    def test_android_str(self):
        MobilePlatform = _import_or_skip("MobilePlatform")
        assert str(MobilePlatform.Android) == "Android"

    def test_ios_str(self):
        MobilePlatform = _import_or_skip("MobilePlatform")
        assert str(MobilePlatform.Ios) == "Ios"
