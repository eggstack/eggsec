"""Scope enforcement and cap validation tests for Phase F modules."""

import pytest
import eggsec


class TestScopeEnforcement:
    """Verify scope enforcement across network-active modules.

    Note: load_test_http, validate_waf, and fuzz_http do NOT accept a scope
    parameter at the Python binding level. Scope enforcement for these functions
    is handled at the engine/CLI layer, not through the Python API. These tests
    verify that functions which DO accept scope enforce it correctly.
    """

    def test_scan_ports_out_of_scope(self):
        """scan_ports must enforce target scope."""
        scope = eggsec.Scope.allow_hosts(["allowed.local"])
        with pytest.raises(eggsec.EnforcementError, match="not within the allowed scope"):
            eggsec.scan_ports("evil.local", [80, 443], scope, timeout_ms=1000)

    def test_scan_endpoints_out_of_scope(self):
        """scan_endpoints must enforce target scope."""
        scope = eggsec.Scope.allow_hosts(["allowed.local"])
        with pytest.raises(eggsec.EnforcementError, match="not within the allowed scope"):
            eggsec.scan_endpoints(
                "http://evil.local",
                ["/admin"],
                scope,
                timeout_ms=1000,
            )

    def test_fingerprint_services_out_of_scope(self):
        """fingerprint_services must enforce target scope."""
        scope = eggsec.Scope.allow_hosts(["allowed.local"])
        with pytest.raises(eggsec.EnforcementError, match="not within the allowed scope"):
            eggsec.fingerprint_services("evil.local", [80], scope, timeout_ms=1000)

    def test_load_test_http_no_scope_parameter(self):
        """load_test_http does not accept a scope parameter (scope enforced elsewhere)."""
        # Verify calling without scope works (the function has no scope param)
        # This confirms the API contract — scope is NOT enforced at this level
        import inspect
        sig = inspect.signature(eggsec.load_test_http)
        assert "scope" not in sig.parameters

    def test_validate_waf_no_scope_parameter(self):
        """validate_waf does not accept a scope parameter (scope enforced elsewhere)."""
        import inspect
        sig = inspect.signature(eggsec.validate_waf)
        assert "scope" not in sig.parameters

    def test_fuzz_http_no_scope_parameter(self):
        """fuzz_http does not accept a scope parameter (scope enforced elsewhere)."""
        import inspect
        sig = inspect.signature(eggsec.fuzz_http)
        assert "scope" not in sig.parameters


class TestCapValidation:
    """Verify cap validation for active/dangerous modules.

    LoadTestRunner validates total_requests > 0 and concurrency > 0,
    raising ConfigError (mapped from EggsecError::Validation).
    """

    def test_load_test_http_zero_requests(self):
        """load_test_http must reject zero requests."""
        with pytest.raises(eggsec.ScanError, match="Total requests must be greater than 0"):
            eggsec.load_test_http(
                "http://localhost/test",
                total_requests=0,
                concurrency=1,
                timeout_secs=5,
            )

    def test_load_test_http_zero_concurrency(self):
        """load_test_http must reject zero concurrency."""
        with pytest.raises(eggsec.ScanError, match="Concurrency must be greater than 0"):
            eggsec.load_test_http(
                "http://localhost/test",
                total_requests=1,
                concurrency=0,
                timeout_secs=5,
            )

    def test_load_test_http_zero_timeout(self):
        """load_test_http must reject zero timeout."""
        with pytest.raises(eggsec.ScanError, match="Timeout must be greater than 0"):
            eggsec.load_test_http(
                "http://localhost/test",
                total_requests=1,
                concurrency=1,
                timeout_secs=0,
            )


class TestFeatureAvailability:
    """Verify feature-gated modules are unavailable by default.

    These tests check that functions behind feature flags are not
    present in the module when the feature is not compiled in.
    """

    def test_stress_testing_unavailable(self):
        """stress_test should not be available without stress-testing feature."""
        if eggsec.has_feature("stress-testing"):
            pytest.skip("stress-testing feature is enabled")
        assert not hasattr(eggsec, "stress_test"), (
            "stress_test should not be in module without stress-testing feature"
        )

    def test_nse_unavailable(self):
        """nse_run should not be available without nse feature."""
        if eggsec.has_feature("nse"):
            pytest.skip("nse feature is enabled")
        assert not hasattr(eggsec, "nse_run"), (
            "nse_run should not be in module without nse feature"
        )

    def test_packet_inspection_unavailable(self):
        """parse_pcap should not be available without packet-inspection feature."""
        if eggsec.has_feature("packet-inspection"):
            pytest.skip("packet-inspection feature is enabled")
        assert not hasattr(eggsec, "parse_pcap"), (
            "parse_pcap should not be in module without packet-inspection feature"
        )

    def test_git_secrets_unavailable(self):
        """scan_git_secrets should not be available without git-secrets feature."""
        if eggsec.has_feature("git-secrets"):
            pytest.skip("git-secrets feature is enabled")
        assert not hasattr(eggsec, "scan_git_secrets"), (
            "scan_git_secrets should not be in module without git-secrets feature"
        )

    def test_sbom_unavailable(self):
        """generate_sbom should not be available without sbom feature."""
        if eggsec.has_feature("sbom"):
            pytest.skip("sbom feature is enabled")
        assert not hasattr(eggsec, "generate_sbom"), (
            "generate_sbom should not be in module without sbom feature"
        )

    def test_daemon_client_unavailable(self):
        """daemon_connect should not be available without daemon-client feature."""
        if eggsec.has_feature("daemon-client"):
            pytest.skip("daemon-client feature is enabled")
        assert not hasattr(eggsec, "daemon_connect"), (
            "daemon_connect should not be in module without daemon-client feature"
        )

    def test_loadtesting_feature_always_available(self):
        """load-testing feature should always be available."""
        assert eggsec.has_feature("load-testing") is True

    def test_waf_validation_feature_always_available(self):
        """waf-validation feature should always be available."""
        assert eggsec.has_feature("waf-validation") is True

    def test_http_fuzzing_feature_always_available(self):
        """http-fuzzing feature should always be available."""
        assert eggsec.has_feature("http-fuzzing") is True
