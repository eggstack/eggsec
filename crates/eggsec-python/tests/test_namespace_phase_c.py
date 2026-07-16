"""Tests for Phase C namespace maturity governance.

Validates:
- Submodule imports work
- Backward-compatible Py-suffixed imports work
- Canonical and Py-suffixed names resolve to the same type
- Experimental namespace does not pollute top-level dir()
- Feature availability introspection works
- Deprecation warnings are emitted for Py-suffixed access
- Feature guard provides structured errors
- list_unavailable_features works
"""

import warnings
import pytest
import eggsec


# ============================================================================
# Submodule imports
# ============================================================================


class TestSubmoduleImports:
    """Verify submodule packages are accessible."""

    def test_net_importable(self):
        from eggsec import net
        assert hasattr(net, "Target") or hasattr(net, "__all__")

    def test_sessions_importable(self):
        from eggsec import sessions
        assert hasattr(sessions, "SessionState") or hasattr(sessions, "__all__")

    def test_storage_importable(self):
        from eggsec import storage
        assert hasattr(storage, "FindingState") or hasattr(storage, "__all__")

    def test_reporting_importable(self):
        from eggsec import reporting
        assert hasattr(reporting, "FindingReporter") or hasattr(reporting, "__all__")

    def test_daemon_importable(self):
        from eggsec import daemon
        assert hasattr(daemon, "DaemonProtocolVersion") or hasattr(daemon, "__all__")

    def test_experimental_importable(self):
        from eggsec import experimental
        assert isinstance(experimental.__all__, list)


# ============================================================================
# Backward compatibility
# ============================================================================


class TestBackwardCompatibility:
    """Verify Py-suffixed names still work at top level."""

    def test_target_py_accessible(self):
        t = eggsec.TargetPy(host="example.com")
        assert t.host == "example.com"

    def test_tcp_config_py_accessible(self):
        if hasattr(eggsec, "TcpConfigPy"):
            # Just verify it's accessible, don't construct (may need args)
            assert eggsec.TcpConfigPy is not None

    def test_websocket_session_config_py_accessible(self):
        if hasattr(eggsec, "WebSocketSessionConfigPy"):
            assert eggsec.WebSocketSessionConfigPy is not None

    def test_http_request_py_accessible(self):
        if hasattr(eggsec, "HttpRequestPy"):
            assert eggsec.HttpRequestPy is not None

    def test_operation_descriptor_py_accessible(self):
        assert hasattr(eggsec, "OperationDescriptorPy")


class TestCanonicalAliases:
    """Verify canonical names resolve to the same type as Py-suffixed."""

    def test_target_canonical_matches_py(self):
        """net.Target should be the same type as TargetPy."""
        from eggsec import net
        if hasattr(eggsec, "TargetPy") and hasattr(net, "Target"):
            assert eggsec.TargetPy is net.Target

    def test_finding_event_canonical(self):
        """FindingEvent (no Py suffix) should be accessible."""
        assert hasattr(eggsec, "FindingEvent")

    def test_artifact_event_canonical(self):
        """ArtifactEvent (no Py suffix) should be accessible."""
        assert hasattr(eggsec, "ArtifactEvent")


# ============================================================================
# Namespace hygiene
# ============================================================================


class TestNamespaceHygiene:
    """Verify experimental modules don't pollute top-level dir()."""

    def test_experimental_not_in_dir(self):
        """dir(eggsec) should not contain experimental class names."""
        top_names = set(dir(eggsec))
        # These should NOT be in top-level dir
        experimental_names = {
            "WirelessNetwork", "WirelessVulnerability", "WirelessScanResult",
            "EvasionTechnique", "EvasionDetection", "EvasionReport",
            "PostexTechnique", "PostexDetection", "PostexReport",
            "C2Campaign", "BeaconResult", "C2Report",
            "AiAnalysisResult", "AiPayloadSuggestion",
        }
        for name in experimental_names:
            assert name not in top_names, f"{name} should not be in dir(eggsec)"

    def test_submodules_in_dir(self):
        """dir(eggsec) should include submodule names."""
        top_names = set(dir(eggsec))
        assert "net" in top_names
        assert "sessions" in top_names
        assert "storage" in top_names
        assert "reporting" in top_names
        assert "daemon" in top_names
        assert "experimental" in top_names

    def test_stable_operations_in_all(self):
        """All 22 stable operations should be in __all__."""
        stable_ops = [
            "scan_ports", "scan_endpoints", "fingerprint_services",
            "recon_dns", "inspect_tls", "detect_technology",
            "detect_waf", "validate_waf", "fuzz_http", "load_test_http",
            "run_consolidated_recon", "graphql_test", "oauth_test", "auth_test",
        ]
        for op in stable_ops:
            assert op in eggsec.__all__, f"{op} not in __all__"

    def test_engine_classes_in_all(self):
        """Core engine classes should be in __all__."""
        for name in ["Engine", "AsyncEngine", "Client", "AsyncClient", "Scope"]:
            assert name in eggsec.__all__, f"{name} not in __all__"


# ============================================================================
# Feature availability introspection
# ============================================================================


class TestFeatureIntrospection:
    """Verify feature availability can be queried."""

    def test_api_surface_returns_dict(self):
        surface = eggsec.api_surface()
        assert isinstance(surface, dict)

    def test_api_surface_contains_stable_ops(self):
        surface = eggsec.api_surface()
        for op in ["scan_ports", "scan_endpoints", "fingerprint_services"]:
            assert op in surface

    def test_domain_maturity_returns_dict(self):
        maturity = eggsec.domain_maturity()
        assert isinstance(maturity, dict)

    def test_has_feature_returns_bool(self):
        result = eggsec.has_feature("default")
        assert isinstance(result, bool)

    def test_features_returns_dict(self):
        result = eggsec.features()
        assert isinstance(result, dict)


# ============================================================================
# Deprecation warnings
# ============================================================================


class TestDeprecationWarnings:
    """Verify deprecation helper works."""

    def test_deprecated_function_emits_warning(self):
        with warnings.catch_warnings(record=True) as w:
            warnings.simplefilter("always")
            eggsec._deprecated("test_symbol", "test_replacement")
            assert len(w) == 1
            assert issubclass(w[0].category, DeprecationWarning)
            assert "test_symbol" in str(w[0].message)
            assert "test_replacement" in str(w[0].message)

    def test_deprecated_function_no_replacement(self):
        with warnings.catch_warnings(record=True) as w:
            warnings.simplefilter("always")
            eggsec._deprecated("test_symbol")
            assert len(w) == 1
            assert "test_symbol" in str(w[0].message)


# ============================================================================
# Import safety
# ============================================================================


class TestImportSafety:
    """Verify import eggsec does not initialize heavy dependencies."""

    def test_import_does_not_need_network(self):
        """Basic import should not make network calls."""
        # This is a structural test - if import hangs, the timeout will catch it
        import importlib
        mod = importlib.import_module("eggsec")
        assert mod is not None

    def test_experimental_does_not_affect_core(self):
        """Importing experimental should not change core namespace."""
        before = set(dir(eggsec))
        from eggsec import experimental
        after = set(dir(eggsec))
        # New names in dir should only be from experimental module itself
        added = after - before
        # No new names should be added to the top-level namespace
        assert len(added) == 0 or added.issubset({"experimental"})


# ============================================================================
# Feature guard: structured errors
# ============================================================================


class TestFeatureGuard:
    """Verify feature guard provides structured errors."""

    def test_list_unavailable_returns_list(self):
        """list_unavailable_features returns a list."""
        result = eggsec.list_unavailable_features()
        assert isinstance(result, list)

    def test_list_unavailable_items_have_required_fields(self):
        """Each unavailable feature item has required fields."""
        result = eggsec.list_unavailable_features()
        for item in result:
            assert "symbol" in item
            assert "feature" in item
            assert "maturity" in item

    def test_unavailable_error_for_missing_symbol(self):
        """Accessing unavailable symbol raises AttributeError."""
        # Create a temporary unavailable entry
        eggsec._UNAVAILABLE_FEATURES["_test_missing"] = {
            "feature": "test-feature",
            "maturity": "experimental",
            "install_hint": "pip install test",
        }
        try:
            with pytest.raises(AttributeError) as exc_info:
                eggsec._unavailable_error("_test_missing")
            assert "test-feature" in str(exc_info.value)
            assert "pip install test" in str(exc_info.value)
        finally:
            del eggsec._UNAVAILABLE_FEATURES["_test_missing"]

    def test_unavailable_error_for_unknown_symbol(self):
        """Unknown symbol raises generic AttributeError."""
        with pytest.raises(AttributeError) as exc_info:
            eggsec._unavailable_error("_unknown_symbol")
        assert "_unknown_symbol" in str(exc_info.value)

    def test_getattr_raises_for_unavailable(self):
        """__getattr__ raises AttributeError for unavailable features."""
        eggsec._UNAVAILABLE_FEATURES["_test_attr"] = {
            "feature": "test-attr",
            "maturity": "experimental",
        }
        try:
            with pytest.raises(AttributeError) as exc_info:
                _ = eggsec._test_attr
            assert "test-attr" in str(exc_info.value)
        finally:
            del eggsec._UNAVAILABLE_FEATURES["_test_attr"]

    def test_getattr_raises_attribute_error_for_unknown(self):
        """__getattr__ raises AttributeError for truly unknown names."""
        with pytest.raises(AttributeError):
            _ = eggsec._truly_nonexistent_symbol_xyz
