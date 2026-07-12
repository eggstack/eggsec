"""Rust-side integration smoke tests for eggsec Python bindings.

These tests verify the compiled extension module works correctly
when installed from a built wheel or development install.
"""

import json


def test_import():
    import eggsec
    assert eggsec is not None


def test_version():
    import eggsec
    assert isinstance(eggsec.__version__, str)
    assert eggsec.__version__ == "0.1.0"


def test_version_info():
    import eggsec
    assert eggsec.__version_info__ == (0, 1, 0)


def test_features():
    import eggsec
    features = eggsec.features()
    assert isinstance(features, dict)
    assert features["core"] is True
    assert features["scanner"] is True
    assert features["async-api"] is True
    assert features["endpoint-discovery"] is True
    assert features["service-fingerprinting"] is True


def test_has_feature():
    import eggsec
    assert eggsec.has_feature("core") is True
    assert eggsec.has_feature("scanner") is True
    assert eggsec.has_feature("nonexistent") is False


def test_build_info():
    import eggsec
    info = eggsec.build_info()
    assert isinstance(info, dict)
    assert info["version"] == "0.1.0"
    assert "package_name" in info
    assert "target_triple" in info
    assert "binding_version" in info


def test_scope():
    import eggsec
    scope = eggsec.Scope.allow_hosts(["127.0.0.1", "10.0.0.0/8"])
    assert scope.is_target_allowed("127.0.0.1") is True
    assert scope.is_target_allowed("10.0.0.1") is True
    assert scope.is_target_allowed("192.0.2.1") is False

    scope2 = eggsec.Scope.deny_all()
    assert scope2.is_target_allowed("anything") is False

    scope3 = eggsec.Scope.allow_cidrs(["127.0.0.0/8"])
    assert scope3.is_target_allowed("127.0.0.1") is True
    assert scope3.is_target_allowed("192.168.1.1") is False


def test_client():
    import eggsec
    scope = eggsec.Scope.allow_hosts(["example.com"])
    client = eggsec.Client(scope)
    assert client.mode == "manual"
    assert client.scope.is_target_allowed("example.com") is True

    client2 = eggsec.Client(scope, mode="automation", concurrency=50, timeout_ms=10000)
    assert client2.mode == "automation"


def test_client_context_manager():
    import eggsec
    scope = eggsec.Scope.allow_hosts(["example.com"])
    with eggsec.Client(scope) as client:
        assert client is not None
    # Should not raise after exiting context


def test_exceptions():
    import eggsec
    # Hierarchy checks
    assert issubclass(eggsec.EggsecError, Exception)
    assert issubclass(eggsec.ConfigError, eggsec.EggsecError)
    assert issubclass(eggsec.ScopeError, eggsec.EggsecError)
    assert issubclass(eggsec.EnforcementError, eggsec.EggsecError)
    assert issubclass(eggsec.NetworkError, eggsec.EggsecError)
    assert issubclass(eggsec.ScanError, eggsec.EggsecError)
    assert issubclass(eggsec.TimeoutError, eggsec.EggsecError)
    assert issubclass(eggsec.FeatureUnavailableError, eggsec.EggsecError)
    assert issubclass(eggsec.SerializationError, eggsec.EggsecError)
    assert issubclass(eggsec.InternalError, eggsec.EggsecError)

    # Raise/catch check
    try:
        raise eggsec.NetworkError("connection refused")
    except eggsec.EggsecError as e:
        assert "connection refused" in str(e)


def test_all_classes_accessible():
    import eggsec
    # Core classes
    assert hasattr(eggsec, "Scope")
    assert hasattr(eggsec, "Client")
    assert hasattr(eggsec, "AsyncClient")
    assert hasattr(eggsec, "PyFuture")

    # DTO classes
    assert hasattr(eggsec, "PortScanResult")
    assert hasattr(eggsec, "OpenPort")
    assert hasattr(eggsec, "ScanStats")
    assert hasattr(eggsec, "PortRange")
    assert hasattr(eggsec, "TimingPreset")

    # Endpoint classes
    assert hasattr(eggsec, "EndpointScanConfig")
    assert hasattr(eggsec, "EndpointFinding")
    assert hasattr(eggsec, "EndpointScanStats")
    assert hasattr(eggsec, "EndpointScanResult")

    # Fingerprint classes
    assert hasattr(eggsec, "FingerprintEvidence")
    assert hasattr(eggsec, "FingerprintConfidence")
    assert hasattr(eggsec, "ServiceFingerprintResult")
    assert hasattr(eggsec, "FingerprintScanResult")

    # Finding/reporting classes
    assert hasattr(eggsec, "Severity")
    assert hasattr(eggsec, "Evidence")
    assert hasattr(eggsec, "Finding")
    assert hasattr(eggsec, "FindingSet")
    assert hasattr(eggsec, "Report")

    # Recon classes
    assert hasattr(eggsec, "DnsRecordSet")
    assert hasattr(eggsec, "MxRecord")
    assert hasattr(eggsec, "SoaRecord")
    assert hasattr(eggsec, "TlsCertificateInfo")
    assert hasattr(eggsec, "TlsInspectionResult")
    assert hasattr(eggsec, "SslIssue")
    assert hasattr(eggsec, "TechStack")
    assert hasattr(eggsec, "TechDetectionResult")

    # WAF classes
    assert hasattr(eggsec, "WafDetectionResult")

    # Functions
    assert callable(eggsec.features)
    assert callable(eggsec.has_feature)
    assert callable(eggsec.build_info)
    assert callable(eggsec.scan_ports)
    assert callable(eggsec.async_scan_ports)
    assert callable(eggsec.scan_endpoints)
    assert callable(eggsec.async_scan_endpoints)
    assert callable(eggsec.fingerprint_services)
    assert callable(eggsec.async_fingerprint_services)
    assert callable(eggsec.recon_dns)
    assert callable(eggsec.async_recon_dns)
    assert callable(eggsec.inspect_tls)
    assert callable(eggsec.async_inspect_tls)
    assert callable(eggsec.detect_technology)
    assert callable(eggsec.async_detect_technology)
    assert callable(eggsec.detect_waf)
    assert callable(eggsec.async_detect_waf)


def test_finding_and_report_roundtrip():
    import eggsec
    finding = eggsec.Finding(
        id="smoke-1",
        title="Integration smoke test",
        severity=eggsec.Severity.High,
        target="127.0.0.1",
        category="integration-test",
        description="Verifying finding serialization roundtrip",
        recommendation="No action needed",
    )
    # Finding to_dict
    d = finding.to_dict()
    assert d["id"] == "smoke-1"
    assert d["severity"] == "High"

    # Finding to_json
    j = finding.to_json()
    parsed = json.loads(j)
    assert parsed["id"] == "smoke-1"

    # Report roundtrip
    report = eggsec.Report(metadata={"test": "smoke"})
    report.add_finding(finding)
    assert len(report) == 1

    report_j = report.to_json()
    report_parsed = json.loads(report_j)
    assert len(report_parsed["findings"]) == 1
    assert report_parsed["findings"][0]["id"] == "smoke-1"
    assert report_parsed["metadata"]["test"] == "smoke"

    # FindingSet
    fs = eggsec.FindingSet()
    fs.add_finding(finding)
    assert len(fs) == 1
    by_sev = fs.by_severity(eggsec.Severity.High)
    assert len(by_sev) == 1


def test_severity_enum():
    import eggsec
    assert str(eggsec.Severity.Critical) == "Critical"
    assert str(eggsec.Severity.High) == "High"
    assert str(eggsec.Severity.Medium) == "Medium"
    assert str(eggsec.Severity.Low) == "Low"
    assert str(eggsec.Severity.Info) == "Info"

    assert eggsec.Severity.from_str("critical") == eggsec.Severity.Critical
    assert eggsec.Severity.from_str("HIGH") == eggsec.Severity.High
    assert eggsec.Severity.from_str("informational") == eggsec.Severity.Info

    try:
        eggsec.Severity.from_str("invalid")
        assert False, "Should have raised ValueError"
    except ValueError:
        pass
