"""Scope enforcement and cap validation tests for Phase F modules."""

import pytest
import eggsec


class TestScopeEnforcement:
    """Verify scope enforcement across all network-active modules."""

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

    def test_load_test_http_out_of_scope(self):
        """load_test_http must enforce target scope."""
        scope = eggsec.Scope.allow_hosts(["allowed.local"])
        with pytest.raises(eggsec.EnforcementError, match="not within the allowed scope"):
            eggsec.load_test_http(
                "http://evil.local",
                total_requests=1,
                concurrency=1,
                timeout_secs=1,
                scope=scope,
            )

    def test_validate_waf_out_of_scope(self):
        """validate_waf must enforce target scope."""
        scope = eggsec.Scope.allow_hosts(["allowed.local"])
        with pytest.raises(eggsec.EnforcementError, match="not within the allowed scope"):
            eggsec.validate_waf("http://evil.local", scope=scope)

    def test_fuzz_http_out_of_scope(self):
        """fuzz_http must enforce target scope."""
        scope = eggsec.Scope.allow_hosts(["allowed.local"])
        with pytest.raises(eggsec.EnforcementError, match="not within the allowed scope"):
            eggsec.fuzz_http("http://evil.local", scope=scope)

    def test_client_load_test_http_out_of_scope(self):
        """Client.load_test_http must enforce target scope."""
        scope = eggsec.Scope.allow_hosts(["allowed.local"])
        client = eggsec.Client(scope)
        with pytest.raises(eggsec.EnforcementError, match="not within the allowed scope"):
            client.load_test_http(
                "http://evil.local",
                total_requests=1,
                concurrency=1,
                timeout_secs=1,
            )

    def test_client_validate_waf_out_of_scope(self):
        """Client.validate_waf must enforce target scope."""
        scope = eggsec.Scope.allow_hosts(["allowed.local"])
        client = eggsec.Client(scope)
        with pytest.raises(eggsec.EnforcementError, match="not within the allowed scope"):
            client.validate_waf("http://evil.local")

    def test_client_fuzz_http_out_of_scope(self):
        """Client.fuzz_http must enforce target scope."""
        scope = eggsec.Scope.allow_hosts(["allowed.local"])
        client = eggsec.Client(scope)
        with pytest.raises(eggsec.EnforcementError, match="not within the allowed scope"):
            client.fuzz_http("http://evil.local")


class TestCapValidation:
    """Verify cap validation for active/dangerous modules."""

    def test_load_test_http_zero_requests(self):
        """load_test_http must reject zero requests."""
        scope = eggsec.Scope.allow_hosts(["localhost"])
        with pytest.raises(ValueError, match="total_requests must be > 0"):
            eggsec.load_test_http(
                "http://localhost/test",
                total_requests=0,
                concurrency=1,
                timeout_secs=5,
                scope=scope,
            )

    def test_load_test_http_zero_concurrency(self):
        """load_test_http must reject zero concurrency."""
        scope = eggsec.Scope.allow_hosts(["localhost"])
        with pytest.raises(ValueError, match="concurrency must be > 0"):
            eggsec.load_test_http(
                "http://localhost/test",
                total_requests=1,
                concurrency=0,
                timeout_secs=5,
                scope=scope,
            )

    def test_load_test_http_zero_timeout(self):
        """load_test_http must reject zero timeout."""
        scope = eggsec.Scope.allow_hosts(["localhost"])
        with pytest.raises(ValueError, match="timeout_secs must be > 0"):
            eggsec.load_test_http(
                "http://localhost/test",
                total_requests=1,
                concurrency=1,
                timeout_secs=0,
                scope=scope,
            )

    def test_fuzz_http_zero_concurrency(self):
        """fuzz_http must reject zero concurrency."""
        scope = eggsec.Scope.allow_hosts(["localhost"])
        with pytest.raises(ValueError, match="concurrency must be > 0"):
            eggsec.fuzz_http("http://localhost/test", scope=scope, concurrency=0)

    def test_fuzz_http_zero_timeout(self):
        """fuzz_http must reject zero timeout."""
        scope = eggsec.Scope.allow_hosts(["localhost"])
        with pytest.raises(ValueError, match="timeout must be > 0"):
            eggsec.fuzz_http("http://localhost/test", scope=scope, timeout=0)


class TestFeatureAvailability:
    """Verify feature-gated modules are unavailable by default."""

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
