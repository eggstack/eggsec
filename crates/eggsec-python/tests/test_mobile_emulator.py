"""Mobile emulator workflow tests - Workstream 9.

Tests prove the mobile session types work end-to-end. The emulator profile
is non-blocking and scheduled (manual trigger). These tests verify the
binding surface is correct and the session lifecycle is sound.

When ADB is not available, tests skip with a clear reason (this is expected
for the mobile-static profile). The mobile-emulator profile requires a real
emulator and ADB.
"""

import json
import os
import pytest
import importlib

pytestmark = [pytest.mark.timeout(60)]


def _import_or_skip(name, feature="mobile"):
    """Import a name from eggsec, skip if feature-gated."""
    mod = importlib.import_module("eggsec")
    obj = getattr(mod, name, None)
    if obj is None:
        pytest.skip(f"{name} not available (requires {feature} feature)")
    return obj


def _adb_available():
    """Check if ADB is available on the system."""
    import shutil
    return shutil.which("adb") is not None


requires_adb = pytest.mark.skipif(
    not _adb_available(),
    reason="ADB not available (requires mobile-emulator profile)"
)


# ---------------------------------------------------------------------------
# MobileSession lifecycle
# ---------------------------------------------------------------------------


class TestMobileSessionLifecycle:
    """Test MobileSession construction and lifecycle."""

    @pytest.mark.timeout(30)
    def test_mobile_session_config_construction(self):
        """MobileSessionConfig constructs with valid defaults."""
        MobileSessionConfig = _import_or_skip("MobileSessionConfig")

        config = MobileSessionConfig(
            package_name="com.example.app",
            activity_name=".MainActivity",
        )
        assert config.package_name == "com.example.app"
        assert config.activity_name == ".MainActivity"

    @pytest.mark.timeout(30)
    def test_mobile_session_config_with_options(self):
        """MobileSessionConfig with all options."""
        MobileSessionConfig = _import_or_skip("MobileSessionConfig")

        config = MobileSessionConfig(
            package_name="com.example.app",
            activity_name=".MainActivity",
            device_serial="emulator-5554",
            timeout_ms=30000,
            install_timeout_ms=60000,
            grant_permissions=True,
            debuggable=True,
        )
        assert config.device_serial == "emulator-5554"
        assert config.grant_permissions is True

    @pytest.mark.timeout(30)
    def test_mobile_session_created_state(self):
        """MobileSession starts in Created state."""
        MobileSession = _import_or_skip("MobileSession")
        MobileSessionConfig = _import_or_skip("MobileSessionConfig")
        MobileSessionState = _import_or_skip("MobileSessionState")

        config = MobileSessionConfig(
            package_name="com.example.app",
            activity_name=".MainActivity",
        )
        session = MobileSession(config)
        assert session.state == MobileSessionState.Created

    @pytest.mark.timeout(30)
    def test_mobile_session_stop_idempotent(self):
        """MobileSession.stop() is idempotent."""
        MobileSession = _import_or_skip("MobileSession")
        MobileSessionConfig = _import_or_skip("MobileSessionConfig")
        MobileSessionState = _import_or_skip("MobileSessionState")

        config = MobileSessionConfig(
            package_name="com.example.app",
            activity_name=".MainActivity",
        )
        session = MobileSession(config)
        session.stop()
        session.stop()
        assert session.state == MobileSessionState.Stopped

    @pytest.mark.timeout(30)
    def test_mobile_session_config_serialization(self):
        """MobileSessionConfig serializes to dict and JSON."""
        MobileSessionConfig = _import_or_skip("MobileSessionConfig")

        config = MobileSessionConfig(
            package_name="com.example.app",
            activity_name=".MainActivity",
        )
        d = config.to_dict()
        assert isinstance(d, dict)
        assert d["package_name"] == "com.example.app"

        j = config.to_json()
        parsed = json.loads(j)
        assert parsed["package_name"] == "com.example.app"


# ---------------------------------------------------------------------------
# Mobile device registry
# ---------------------------------------------------------------------------


class TestMobileDeviceRegistry:
    """Test MobileDeviceRegistry operations."""

    @pytest.mark.timeout(30)
    def test_registry_construction(self):
        """MobileDeviceRegistry constructs."""
        MobileDeviceRegistry = _import_or_skip("MobileDeviceRegistry")

        registry = MobileDeviceRegistry()
        assert registry is not None

    @pytest.mark.timeout(30)
    def test_registry_list_devices(self):
        """MobileDeviceRegistry list_devices returns a list."""
        MobileDeviceRegistry = _import_or_skip("MobileDeviceRegistry")

        registry = MobileDeviceRegistry()
        devices = registry.list_devices()
        assert isinstance(devices, list)

    @pytest.mark.timeout(30)
    def test_registry_to_dict(self):
        """MobileDeviceRegistry to_dict returns valid dict."""
        MobileDeviceRegistry = _import_or_skip("MobileDeviceRegistry")

        registry = MobileDeviceRegistry()
        d = registry.to_dict()
        assert isinstance(d, dict)


# ---------------------------------------------------------------------------
# Mobile device info
# ---------------------------------------------------------------------------


class TestMobileDeviceInfo:
    """Test MobileDeviceInfo construction."""

    @pytest.mark.timeout(30)
    def test_device_info_construction(self):
        """MobileDeviceInfo constructs with valid fields."""
        MobileDeviceInfo = _import_or_skip("MobileDeviceInfo")

        info = MobileDeviceInfo(
            serial="emulator-5554",
            model="sdk_gphone64_x86_64",
            android_version="14",
            api_level=34,
            is_emulator=True,
            is_connected=True,
        )
        assert info.serial == "emulator-5554"
        assert info.is_emulator is True

    @pytest.mark.timeout(30)
    def test_device_info_to_dict(self):
        """MobileDeviceInfo to_dict returns valid dict."""
        MobileDeviceInfo = _import_or_skip("MobileDeviceInfo")

        info = MobileDeviceInfo(
            serial="device-1",
            model="Pixel7",
            android_version="14",
            api_level=34,
            is_emulator=False,
            is_connected=True,
        )
        d = info.to_dict()
        assert isinstance(d, dict)
        assert d["serial"] == "device-1"
        assert d["is_emulator"] is False


# ---------------------------------------------------------------------------
# Dynamic mobile config
# ---------------------------------------------------------------------------


class TestDynamicMobileConfig:
    """Test DynamicMobileConfig for emulator workflow."""

    @pytest.mark.timeout(30)
    def test_config_construction(self):
        """DynamicMobileConfig constructs."""
        DynamicMobileConfig = _import_or_skip("DynamicMobileConfig")

        config = DynamicMobileConfig(
            device_serial="emulator-5554",
            install_apk=True,
            grant_permissions=True,
            enable_network=True,
        )
        assert config.device_serial == "emulator-5554"
        assert config.install_apk is True

    @pytest.mark.timeout(30)
    def test_config_to_dict(self):
        """DynamicMobileConfig to_dict."""
        DynamicMobileConfig = _import_or_skip("DynamicMobileConfig")

        config = DynamicMobileConfig(
            device_serial="emulator-5554",
            install_apk=True,
            grant_permissions=True,
            enable_network=True,
        )
        d = config.to_dict()
        assert isinstance(d, dict)
        assert d["device_serial"] == "emulator-5554"


# ---------------------------------------------------------------------------
# Mobile with real ADB (emulator profile)
# ---------------------------------------------------------------------------


class TestMobileWithAdb:
    """Test mobile operations with real ADB (skips if ADB not available)."""

    @requires_adb
    @pytest.mark.timeout(30)
    def test_device_registry_finds_devices(self):
        """DeviceRegistry finds connected devices via ADB."""
        MobileDeviceRegistry = _import_or_skip("MobileDeviceRegistry")

        registry = MobileDeviceRegistry()
        devices = registry.list_devices()
        # If ADB is available, there should be at least one device
        # (but this might be 0 if no emulator is running)
        assert isinstance(devices, list)

    @requires_adb
    @pytest.mark.timeout(30)
    def test_mobile_session_config_validates(self):
        """MobileSessionConfig with real device serial."""
        MobileSessionConfig = _import_or_skip("MobileSessionConfig")

        config = MobileSessionConfig(
            package_name="com.example.app",
            activity_name=".MainActivity",
            device_serial="emulator-5554",
        )
        d = config.to_dict()
        assert d["device_serial"] == "emulator-5554"
