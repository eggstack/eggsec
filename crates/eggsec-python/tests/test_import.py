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


def test_has_feature_core():
    assert eggsec.has_feature("core") is True


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
