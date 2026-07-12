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


class TestOperationRegistry:
    """Verify OperationRegistry static methods."""

    def test_operation_count(self):
        """operation_count should return a positive integer."""
        count = eggsec.OperationRegistry.operation_count()
        assert isinstance(count, int)
        assert count > 0

    def test_operation_ids(self):
        """operation_ids should return a non-empty list of strings."""
        ids = eggsec.OperationRegistry.operation_ids()
        assert isinstance(ids, list)
        assert len(ids) > 0
        assert all(isinstance(op_id, str) for op_id in ids)
        assert "scan-ports" in ids

    def test_operation_names(self):
        """operation_names should return a non-empty list of strings."""
        names = eggsec.OperationRegistry.operation_names()
        assert isinstance(names, list)
        assert len(names) > 0
        assert all(isinstance(n, str) for n in names)

    def test_operation_count_matches_ids(self):
        """operation_count should equal len(operation_ids)."""
        count = eggsec.OperationRegistry.operation_count()
        ids = eggsec.OperationRegistry.operation_ids()
        assert count == len(ids)

    def test_operations_for_feature(self):
        """operations_for_feature should filter by feature flag."""
        # scan-ports has no required feature
        no_feature = eggsec.OperationRegistry.operations_for_feature("nonexistent-feature")
        assert isinstance(no_feature, list)
        # All returned views should have the matching feature
        for view in no_feature:
            assert view.feature_required == "nonexistent-feature"

    def test_operations_for_feature_empty(self):
        """operations_for_feature with unknown feature returns empty list."""
        result = eggsec.OperationRegistry.operations_for_feature("nonexistent-feature")
        assert result == []

    def test_operations_for_surface(self):
        """operations_for_surface should filter by surface."""
        cli_ops = eggsec.OperationRegistry.operations_for_surface("cli")
        assert isinstance(cli_ops, list)
        assert len(cli_ops) > 0
        for view in cli_ops:
            assert "cli" in view.supported_surfaces

    def test_operations_for_surface_unknown(self):
        """operations_for_surface with unknown surface returns empty list."""
        result = eggsec.OperationRegistry.operations_for_surface("nonexistent-surface")
        assert result == []

    def test_find_returns_view_with_new_fields(self):
        """find should return views with all new fields populated."""
        view = eggsec.OperationRegistry.find("scan-ports")
        assert view is not None
        assert view.operation_id == "scan-ports"
        assert view.python_async_available is True
        assert isinstance(view.supported_surfaces, list)
        assert len(view.supported_surfaces) > 0
        assert isinstance(view.default_timeout_ms, int)
        assert view.default_timeout_ms > 0

    def test_to_dict(self):
        """to_dict should return a serializable dict with all fields."""
        view = eggsec.OperationRegistry.find("scan-ports")
        assert view is not None
        d = view.to_dict()
        assert isinstance(d, dict)
        assert d["operation_id"] == "scan-ports"
        assert "operation_name" in d
        assert "default_risk" in d
        assert "default_mode" in d
        assert "target_policy" in d
        assert "request_schema" in d
        assert "result_schema" in d
        assert "feature_required" in d
        assert "python_async_available" in d
        assert "supported_surfaces" in d
        assert "default_timeout_ms" in d

    def test_descriptor_for_target(self):
        """descriptor_for_target should return a valid OperationDescriptor."""
        view = eggsec.OperationRegistry.find("scan-ports")
        assert view is not None
        desc = view.descriptor_for_target("example.com")
        assert desc.operation_id == "scan-ports"


class TestDomainRegistry:
    """Verify DomainRegistry static methods."""

    def test_all_domains(self):
        """all_domains should return a list of DomainDescriptorPy."""
        domains = eggsec.DomainRegistry.all_domains()
        assert isinstance(domains, list)
        assert len(domains) > 0
        for d in domains:
            assert hasattr(d, "id")
            assert hasattr(d, "display_name")
            assert hasattr(d, "description")
            assert hasattr(d, "category")
            assert hasattr(d, "required_feature")
            assert hasattr(d, "operations")
            assert isinstance(d.operations, list)

    def test_available_domains(self):
        """available_domains should return a subset of all_domains."""
        all_d = eggsec.DomainRegistry.all_domains()
        avail = eggsec.DomainRegistry.available_domains()
        assert isinstance(avail, list)
        # Available is a subset of all
        avail_ids = {d.id for d in avail}
        all_ids = {d.id for d in all_d}
        assert avail_ids.issubset(all_ids)

    def test_find_existing(self):
        """find should return a domain descriptor for known IDs."""
        d = eggsec.DomainRegistry.find("db-pentest")
        # May be None if feature-gated and not compiled
        if d is not None:
            assert d.id == "db-pentest"
            assert d.display_name == "Database Pentesting"
            assert d.category == "defense-lab"

    def test_find_nonexistent(self):
        """find should return None for unknown domain IDs."""
        d = eggsec.DomainRegistry.find("nonexistent-domain")
        assert d is None

    def test_domain_to_dict(self):
        """to_dict should return a serializable dict."""
        domains = eggsec.DomainRegistry.all_domains()
        assert len(domains) > 0
        d = domains[0]
        result = d.to_dict()
        assert isinstance(result, dict)
        assert "id" in result
        assert "display_name" in result
        assert "description" in result
        assert "category" in result
        assert "required_feature" in result
        assert "operations" in result
        assert "is_available" in result

    def test_domain_is_available(self):
        """is_available should return a bool."""
        domains = eggsec.DomainRegistry.all_domains()
        for d in domains:
            assert isinstance(d.is_available, bool)
