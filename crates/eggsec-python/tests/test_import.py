"""Tests for eggsec Python bindings - Phase A foundation."""

import eggsec


def test_version_is_string():
    assert isinstance(eggsec.__version__, str)
    assert eggsec.__version__ == "0.1.0"


def test_version_info():
    assert isinstance(eggsec.__version_info__, tuple)
    assert eggsec.__version_info__ == (0, 1, 0)


def test_features_returns_dict():
    result = eggsec.features()
    assert isinstance(result, dict)
    assert "core" in result
    assert result["core"] is True
    assert result["scanner"] is True
    assert result["async-api"] is True
    assert result["endpoint-discovery"] is True
    assert result["service-fingerprinting"] is True


def test_has_feature_core():
    assert eggsec.has_feature("core") is True


def test_has_feature_scanner():
    assert eggsec.has_feature("scanner") is True


def test_has_feature_async_api():
    assert eggsec.has_feature("async-api") is True


def test_has_feature_endpoint_discovery():
    assert eggsec.has_feature("endpoint-discovery") is True


def test_has_feature_fingerprinting():
    assert eggsec.has_feature("service-fingerprinting") is True


def test_has_feature_unknown():
    assert eggsec.has_feature("nonexistent") is False


def test_build_info():
    info = eggsec.build_info()
    assert isinstance(info, dict)
    assert "version" in info
    assert "package_name" in info


def test_exception_hierarchy():
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


def test_exception_can_be_raised():
    try:
        raise eggsec.ConfigError("test config error")
    except eggsec.EggsecError as e:
        assert "test config error" in str(e)


def test_async_client_import():
    assert hasattr(eggsec, "AsyncClient")
    assert hasattr(eggsec, "PyFuture")


def test_endpoint_classes_import():
    assert hasattr(eggsec, "EndpointScanConfig")
    assert hasattr(eggsec, "EndpointFinding")
    assert hasattr(eggsec, "EndpointScanStats")
    assert hasattr(eggsec, "EndpointScanResult")


def test_fingerprint_classes_import():
    assert hasattr(eggsec, "FingerprintEvidence")
    assert hasattr(eggsec, "FingerprintConfidence")
    assert hasattr(eggsec, "ServiceFingerprintResult")
    assert hasattr(eggsec, "FingerprintScanResult")


def test_async_functions_import():
    assert callable(eggsec.async_scan_ports)
    assert callable(eggsec.async_scan_endpoints)
    assert callable(eggsec.async_fingerprint_services)


def test_endpoint_functions_import():
    assert callable(eggsec.scan_endpoints)
    assert callable(eggsec.fingerprint_services)


def test_distributed_imports():
    assert hasattr(eggsec, "DistributedTaskType")
    assert hasattr(eggsec, "WorkerStatus")
    assert hasattr(eggsec, "WorkerRegistration")
    assert hasattr(eggsec, "Heartbeat")
    assert callable(eggsec.distributed_task_types)
    assert callable(eggsec.distributed_generate_psk)


def test_notification_imports():
    assert hasattr(eggsec, "WebhookEvent")
    assert hasattr(eggsec, "FindingSummary")
    assert hasattr(eggsec, "NotifyScanStats")
    assert hasattr(eggsec, "WebhookConfig")
    assert callable(eggsec.notify_scan_started)
    assert callable(eggsec.notify_scan_complete)
    assert callable(eggsec.notify_findings)
    assert callable(eggsec.notify_error)
