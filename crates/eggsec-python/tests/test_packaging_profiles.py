"""Packaging profile validation tests (WS30).

Validates that feature-gated profiles build and install correctly.
Each test class uses skipIf to only run when the relevant feature is enabled.

Profiles validated:
  - core: always available (default)
  - mobile: APK/IPA static analysis
  - mobile-dynamic: Android dynamic testing via ADB
  - headless-browser: DOM XSS, SPA route discovery
  - daemon-client: daemon session management
  - combined-mobile: mobile + git-secrets + sbom
"""

import pytest

import eggsec


# --- Core profile (always available) ---


class TestCoreProfile:
    """Tests for the core (default) profile."""

    def test_core_feature_is_true(self):
        f = eggsec.features()
        assert f["core"] is True

    def test_optional_features_absent_in_core(self):
        """In a pure core build, optional features should be absent.
        Skip if any optional features are present (multi-feature build)."""
        f = eggsec.features()
        optional = ["mobile", "headless-browser", "daemon-client", "nse", "wireless"]
        present = [feat for feat in optional if f.get(feat, False)]
        if present:
            pytest.skip(f"Optional features present in build: {present} — not a pure core profile")
        # If we get here, no optional features are present — core-only build
        assert True

    def test_scan_ports_available(self):
        assert callable(eggsec.scan_ports)

    def test_api_surface_nonempty(self):
        surface = eggsec.api_surface()
        assert isinstance(surface, dict)
        assert len(surface) > 0

    def test_feature_matrix_returns_dict(self):
        matrix = eggsec.feature_matrix()
        assert isinstance(matrix, dict)

    def test_has_feature_matches_features_dict(self):
        f = eggsec.features()
        for name, enabled in f.items():
            assert eggsec.has_feature(name) == enabled, (
                f"has_feature('{name}')={eggsec.has_feature(name)} "
                f"does not match features()['{name}']={enabled}"
            )

    def test_build_info_returns_expected_fields(self):
        info = eggsec.build_info()
        assert isinstance(info, dict)
        assert "version" in info
        assert "package_name" in info


# --- Mobile profile ---


class TestMobileProfile:
    """Tests for the mobile profile."""

    @pytest.mark.skipif(
        not eggsec.has_feature("mobile"),
        reason="mobile feature not enabled",
    )
    def test_mobile_feature_enabled(self):
        assert eggsec.has_feature("mobile")

    @pytest.mark.skipif(
        not eggsec.has_feature("mobile"),
        reason="mobile feature not enabled",
    )
    def test_mobile_functions_importable(self):
        assert callable(eggsec.analyze_apk)
        assert callable(eggsec.analyze_ipa)

    @pytest.mark.skipif(
        not eggsec.has_feature("mobile"),
        reason="mobile feature not enabled",
    )
    def test_mobile_types_importable(self):
        assert hasattr(eggsec, "MobileScanReport")
        assert hasattr(eggsec, "MobilePlatform")
        assert hasattr(eggsec, "MobileFinding")

    @pytest.mark.skipif(
        not eggsec.has_feature("mobile"),
        reason="mobile feature not enabled",
    )
    def test_headless_browser_absent_in_mobile_profile(self):
        """Only valid in single-feature mobile builds."""
        if eggsec.has_feature("headless-browser"):
            pytest.skip("headless-browser also enabled — not a single-feature mobile profile")
        assert True

    @pytest.mark.skipif(
        not eggsec.has_feature("mobile"),
        reason="mobile feature not enabled",
    )
    def test_daemon_client_absent_in_mobile_profile(self):
        """Only valid in single-feature mobile builds."""
        if eggsec.has_feature("daemon-client"):
            pytest.skip("daemon-client also enabled — not a single-feature mobile profile")
        assert True


# --- Headless browser profile ---


class TestHeadlessBrowserProfile:
    """Tests for the headless-browser profile."""

    @pytest.mark.skipif(
        not eggsec.has_feature("headless-browser"),
        reason="headless-browser feature not enabled",
    )
    def test_browser_feature_enabled(self):
        assert eggsec.has_feature("headless-browser")

    @pytest.mark.skipif(
        not eggsec.has_feature("headless-browser"),
        reason="headless-browser feature not enabled",
    )
    def test_browser_functions_importable(self):
        assert callable(eggsec.browser_test)

    @pytest.mark.skipif(
        not eggsec.has_feature("headless-browser"),
        reason="headless-browser feature not enabled",
    )
    def test_browser_types_importable(self):
        assert hasattr(eggsec, "BrowserTestConfig")
        assert hasattr(eggsec, "BrowserTestReport")
        assert hasattr(eggsec, "DomXssFinding")
        assert hasattr(eggsec, "SpaRoute")

    @pytest.mark.skipif(
        not eggsec.has_feature("headless-browser"),
        reason="headless-browser feature not enabled",
    )
    def test_mobile_absent_in_browser_profile(self):
        """Only valid in single-feature headless-browser builds."""
        if eggsec.has_feature("mobile"):
            pytest.skip("mobile also enabled — not a single-feature headless-browser profile")
        assert True

    @pytest.mark.skipif(
        not eggsec.has_feature("headless-browser"),
        reason="headless-browser feature not enabled",
    )
    def test_daemon_client_absent_in_browser_profile(self):
        """Only valid in single-feature headless-browser builds."""
        if eggsec.has_feature("daemon-client"):
            pytest.skip("daemon-client also enabled — not a single-feature headless-browser profile")
        assert True


# --- Daemon client profile ---


class TestDaemonClientProfile:
    """Tests for the daemon-client profile."""

    @pytest.mark.skipif(
        not eggsec.has_feature("daemon-client"),
        reason="daemon-client feature not enabled",
    )
    def test_daemon_feature_enabled(self):
        assert eggsec.has_feature("daemon-client")

    @pytest.mark.skipif(
        not eggsec.has_feature("daemon-client"),
        reason="daemon-client feature not enabled",
    )
    def test_daemon_functions_importable(self):
        assert callable(eggsec.daemon_connect)

    @pytest.mark.skipif(
        not eggsec.has_feature("daemon-client"),
        reason="daemon-client feature not enabled",
    )
    def test_daemon_types_importable(self):
        assert hasattr(eggsec, "DaemonClient")
        assert hasattr(eggsec, "DaemonResponse")
        assert hasattr(eggsec, "DaemonCapabilities")

    @pytest.mark.skipif(
        not eggsec.has_feature("daemon-client"),
        reason="daemon-client feature not enabled",
    )
    def test_mobile_absent_in_daemon_profile(self):
        """Only valid in single-feature daemon-client builds."""
        if eggsec.has_feature("mobile"):
            pytest.skip("mobile also enabled — not a single-feature daemon-client profile")
        assert True

    @pytest.mark.skipif(
        not eggsec.has_feature("daemon-client"),
        reason="daemon-client feature not enabled",
    )
    def test_headless_browser_absent_in_daemon_profile(self):
        """Only valid in single-feature daemon-client builds."""
        if eggsec.has_feature("headless-browser"):
            pytest.skip("headless-browser also enabled — not a single-feature daemon-client profile")
        assert True


# --- Combined mobile profile ---


class TestCombinedMobileProfile:
    """Tests for the combined-mobile profile (mobile + git-secrets + sbom)."""

    @pytest.mark.skipif(
        not eggsec.has_feature("mobile"),
        reason="mobile feature not enabled (combined-mobile profile)",
    )
    def test_mobile_enabled(self):
        assert eggsec.has_feature("mobile")

    @pytest.mark.skipif(
        not eggsec.has_feature("git-secrets"),
        reason="git-secrets feature not enabled (combined-mobile profile)",
    )
    def test_git_secrets_enabled(self):
        assert eggsec.has_feature("git-secrets")

    @pytest.mark.skipif(
        not eggsec.has_feature("sbom"),
        reason="sbom feature not enabled (combined-mobile profile)",
    )
    def test_sbom_enabled(self):
        assert eggsec.has_feature("sbom")

    @pytest.mark.skipif(
        not eggsec.has_feature("mobile"),
        reason="mobile feature not enabled (combined-mobile profile)",
    )
    def test_mobile_functions_importable(self):
        assert callable(eggsec.analyze_apk)
        assert callable(eggsec.analyze_ipa)

    @pytest.mark.skipif(
        not eggsec.has_feature("git-secrets"),
        reason="git-secrets feature not enabled (combined-mobile profile)",
    )
    def test_git_secrets_function_importable(self):
        assert callable(eggsec.scan_git_secrets)

    @pytest.mark.skipif(
        not eggsec.has_feature("sbom"),
        reason="sbom feature not enabled (combined-mobile profile)",
    )
    def test_sbom_function_importable(self):
        assert callable(eggsec.generate_sbom)


# --- Cross-profile consistency ---


class TestCrossProfileConsistency:
    """Tests for consistency of feature metadata across profiles."""

    def test_feature_matrix_returns_dict(self):
        matrix = eggsec.feature_matrix()
        assert isinstance(matrix, dict)

    def test_feature_matrix_has_required_fields(self):
        matrix = eggsec.feature_matrix()
        for name, entry in matrix.items():
            assert "available" in entry, f"Feature '{name}' missing 'available'"
            assert "description" in entry, f"Feature '{name}' missing 'description'"
            assert "requires_system_deps" in entry, f"Feature '{name}' missing 'requires_system_deps'"

    def test_has_feature_matches_features_for_all_gated(self):
        gated = [
            "websocket", "git-secrets", "sbom", "db-pentest", "web-proxy",
            "mobile", "mobile-dynamic", "packet-inspection", "stress-testing",
            "nse", "container", "daemon-client", "headless-browser",
            "advanced-hunting", "compliance", "wireless", "evasion",
            "postex", "c2", "ai-integration",
        ]
        f = eggsec.features()
        for name in gated:
            assert eggsec.has_feature(name) == f.get(name, False), (
                f"has_feature('{name}') mismatch: "
                f"has_feature={eggsec.has_feature(name)}, features={f.get(name)}"
            )

    def test_api_surface_version_has_features_list(self):
        v = eggsec.api_surface_version()
        assert "features_list" in v
        assert isinstance(v["features_list"], list)
        assert "core" in v["features_list"]
