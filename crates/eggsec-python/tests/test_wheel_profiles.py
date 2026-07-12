"""Tests for wheel profile behavior and package introspection.

Verifies that the default (core) wheel profile exposes the expected API,
that feature-gated types are conditionally available, and that metadata
constants and introspection functions work correctly.
"""

import pytest
import eggsec


class TestCoreProfileBasics:
    """Tests that the default import works and core features are present."""

    def test_import_succeeds(self):
        assert eggsec is not None

    def test_version_is_string(self):
        assert isinstance(eggsec.__version__, str)
        assert eggsec.__version__ == "0.1.0"

    def test_version_info_tuple(self):
        assert isinstance(eggsec.__version_info__, tuple)
        assert eggsec.__version_info__ == (0, 1, 0)

    def test_features_returns_dict(self):
        result = eggsec.features()
        assert isinstance(result, dict)
        assert result["core"] is True
        assert result["scanner"] is True
        assert result["async-api"] is True
        assert result["endpoint-discovery"] is True
        assert result["service-fingerprinting"] is True

    def test_has_feature_core(self):
        assert eggsec.has_feature("core") is True

    def test_has_feature_scanner(self):
        assert eggsec.has_feature("scanner") is True

    def test_has_feature_unknown_returns_false(self):
        assert eggsec.has_feature("nonexistent") is False


class TestFeatureMatrix:
    """Tests for feature_matrix() introspection."""

    def test_returns_dict(self):
        matrix = eggsec.feature_matrix()
        assert isinstance(matrix, dict)

    def test_core_entry_structure(self):
        matrix = eggsec.feature_matrix()
        core = matrix["core"]
        assert "available" in core
        assert "description" in core
        assert "requires_system_deps" in core
        assert core["available"] is True
        assert isinstance(core["description"], str)
        assert core["requires_system_deps"] is False

    def test_feature_gated_entry_structure(self):
        matrix = eggsec.feature_matrix()
        websocket = matrix["websocket"]
        assert "available" in websocket
        assert "description" in websocket
        assert "requires_system_deps" in websocket
        assert isinstance(websocket["description"], str)

    def test_all_always_available_features_present(self):
        matrix = eggsec.feature_matrix()
        always_available = [
            "core",
            "scanner",
            "async-api",
            "endpoint-discovery",
            "service-fingerprinting",
            "waf-detection",
            "waf-validation",
            "http-fuzzing",
            "load-testing",
            "findings-reporting",
        ]
        for name in always_available:
            assert name in matrix, f"Feature '{name}' missing from feature_matrix"
            assert matrix[name]["available"] is True, f"Feature '{name}' not marked available"
            assert matrix[name]["requires_system_deps"] is False, (
                f"Feature '{name}' incorrectly requires system deps"
            )


class TestFeatureGatedAvailability:
    """Tests that feature-gated types raise ImportError when feature is missing."""

    @pytest.mark.parametrize(
        "feature_name,attr_name",
        [
            ("websocket", "websocket_probe"),
            ("git-secrets", "scan_git_secrets"),
            ("sbom", "generate_sbom"),
            ("db-pentest", "db_probe"),
            ("mobile", "analyze_apk"),
            ("packet-inspection", "parse_pcap"),
            ("stress-testing", "stress_test"),
            ("nse", "nse_run"),
            ("daemon-client", "daemon_connect"),
            ("evasion", "evasion_scan"),
            ("postex", "postex_scan"),
            ("c2", "c2_scan"),
            ("headless-browser", "browser_test"),
            ("advanced-hunting", "hunt_test"),
        ],
    )
    def test_feature_gated_attr_missing_when_disabled(self, feature_name, attr_name):
        if eggsec.has_feature(feature_name):
            pytest.skip(f"{feature_name} feature is enabled in this build")
        assert not hasattr(eggsec, attr_name), (
            f"{attr_name} should not be in module without {feature_name} feature"
        )

    @pytest.mark.parametrize(
        "feature_name,attr_name",
        [
            ("websocket", "WebSocketReport"),
            ("git-secrets", "GitSecretsReport"),
            ("sbom", "SbomReport"),
            ("db-pentest", "DbPentestReport"),
            ("mobile", "MobileScanReport"),
            ("packet-inspection", "CaptureConfig"),
            ("stress-testing", "StressType"),
            ("nse", "NseConfig"),
            ("daemon-client", "DaemonClient"),
        ],
    )
    def test_feature_gated_type_missing_when_disabled(self, feature_name, attr_name):
        if eggsec.has_feature(feature_name):
            pytest.skip(f"{feature_name} feature is enabled in this build")
        assert not hasattr(eggsec, attr_name), (
            f"{attr_name} type should not be in module without {feature_name} feature"
        )


class TestApiSurfaceIntrospection:
    """Tests for api_surface() and api_surface_version()."""

    def test_api_surface_returns_dict(self):
        result = eggsec.api_surface()
        assert isinstance(result, dict)

    def test_api_surface_has_known_entries(self):
        result = eggsec.api_surface()
        assert "scan_ports" in result
        assert "features" in result
        assert "build_info" in result

    def test_api_surface_entry_structure(self):
        result = eggsec.api_surface()
        entry = result["scan_ports"]
        assert "stability" in entry
        assert "deprecated" in entry
        assert entry["stability"] == "stable"
        assert entry["deprecated"] is False

    def test_api_surface_stable_entries_not_deprecated(self):
        result = eggsec.api_surface()
        for name, info in result.items():
            if info["stability"] == "stable":
                assert info["deprecated"] is False, (
                    f"'{name}' is marked stable but deprecated"
                )

    def test_api_surface_version_returns_dict(self):
        result = eggsec.api_surface_version()
        assert isinstance(result, dict)

    def test_api_surface_version_has_required_keys(self):
        result = eggsec.api_surface_version()
        assert "package_version" in result
        assert "schema_version" in result
        assert "protocol_version" in result
        assert "abi_version" in result
        assert "features_list" in result

    def test_api_surface_version_values_are_strings(self):
        result = eggsec.api_surface_version()
        assert isinstance(result["package_version"], str)
        assert isinstance(result["schema_version"], str)
        assert isinstance(result["protocol_version"], str)
        assert isinstance(result["abi_version"], str)

    def test_api_surface_version_features_list(self):
        result = eggsec.api_surface_version()
        features = result["features_list"]
        assert isinstance(features, list)
        assert "core" in features
        assert "scanner" in features


class TestVersionConstants:
    """Tests for machine-readable version constants."""

    def test_schema_version_is_string(self):
        assert isinstance(eggsec.__schema_version__, str)

    def test_protocol_version_is_string(self):
        assert isinstance(eggsec.__protocol_version__, str)

    def test_abi_version_is_string(self):
        assert isinstance(eggsec.__abi_version__, str)

    def test_finding_schema_version_exists(self):
        assert hasattr(eggsec, "FINDING_SCHEMA_VERSION")
        assert isinstance(eggsec.FINDING_SCHEMA_VERSION, str)

    def test_event_schema_version_exists(self):
        assert hasattr(eggsec, "EVENT_SCHEMA_VERSION")
        assert isinstance(eggsec.EVENT_SCHEMA_VERSION, str)


class TestExperimentalSubmodule:
    """Tests for the experimental namespace."""

    def test_importable(self):
        from eggsec import experimental
        assert experimental is not None

    def test_has_all_list(self):
        from eggsec import experimental
        assert hasattr(experimental, "__all__")
        assert isinstance(experimental.__all__, list)


class TestDeprecatedWarning:
    """Tests for DeprecatedWarning and deprecated_warning().

    DeprecatedWarning is registered in _core but not re-exported at the
    top-level eggsec package. It is accessible via eggsec._core.
    """

    def test_deprecated_warning_class_exists_in_core(self):
        from eggsec._core import DeprecatedWarning
        assert DeprecatedWarning is not None

    def test_deprecated_warning_instantiation(self):
        from eggsec._core import DeprecatedWarning
        w = DeprecatedWarning("test deprecation")
        assert str(w) == "test deprecation"

    def test_deprecated_warning_default_message(self):
        from eggsec._core import DeprecatedWarning
        w = DeprecatedWarning()
        assert "deprecated" in str(w).lower()

    def test_deprecated_warning_repr(self):
        from eggsec._core import DeprecatedWarning
        w = DeprecatedWarning("msg")
        assert "DeprecatedWarning" in repr(w)

    def test_deprecated_warning_is_plain_class(self):
        from eggsec._core import DeprecatedWarning
        w = DeprecatedWarning("test")
        assert isinstance(w, DeprecatedWarning)
        assert hasattr(w, "message")


class TestAlwaysAvailableFunctions:
    """Tests for functions that should always be available in any wheel."""

    def test_scan_ports_callable(self):
        assert callable(eggsec.scan_ports)

    def test_async_scan_ports_callable(self):
        assert callable(eggsec.async_scan_ports)

    def test_scan_endpoints_callable(self):
        assert callable(eggsec.scan_endpoints)

    def test_fingerprint_services_callable(self):
        assert callable(eggsec.fingerprint_services)

    def test_recon_dns_callable(self):
        assert callable(eggsec.recon_dns)

    def test_inspect_tls_callable(self):
        assert callable(eggsec.inspect_tls)

    def test_detect_technology_callable(self):
        assert callable(eggsec.detect_technology)

    def test_detect_waf_callable(self):
        assert callable(eggsec.detect_waf)

    def test_validate_waf_callable(self):
        assert callable(eggsec.validate_waf)

    def test_fuzz_http_callable(self):
        assert callable(eggsec.fuzz_http)

    def test_load_test_http_callable(self):
        assert callable(eggsec.load_test_http)

    def test_consolidated_recon_callable(self):
        assert callable(eggsec.run_consolidated_recon)


class TestAlwaysAvailableClasses:
    """Tests for classes that should always be available in any wheel."""

    @pytest.mark.parametrize(
        "class_name",
        [
            "Scope",
            "Client",
            "AsyncClient",
            "Engine",
            "AsyncEngine",
            "PortScanResult",
            "OpenPort",
            "ScanStats",
            "Severity",
            "Finding",
            "Report",
            "FindingSet",
            "EndpointScanConfig",
            "EndpointFinding",
            "FingerprintEvidence",
            "FingerprintConfidence",
            "ServiceFingerprintResult",
            "DnsRecordSet",
            "TlsCertificateInfo",
            "TechStack",
            "WafDetectionResult",
            "ExecutionStatus",
            "ExecutionStats",
            "OperationResult",
            "EggsecConfig",
            "LoadedScope",
            "OperationRegistry",
            "EnforcementContext",
            "EventEnvelope",
            "EventStream",
        ],
    )
    def test_class_accessible(self, class_name):
        assert hasattr(eggsec, class_name), f"{class_name} not accessible"
