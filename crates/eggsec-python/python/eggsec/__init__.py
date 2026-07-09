"""Eggsec: Python bindings for the Rust security assessment engine.

This is a host-language binding, not an internal plugin runtime.
The core engine is implemented in Rust; this package provides a Python-native API.
"""

from . import _core

__version__ = _core.__version__
__version_info__ = _core.__version_info__

# Re-export functions
features = _core.features
has_feature = _core.has_feature
build_info = _core.build_info

# Re-export exceptions
EggsecError = _core.EggsecError
ConfigError = _core.ConfigError
ScopeError = _core.ScopeError
EnforcementError = _core.EnforcementError
NetworkError = _core.NetworkError
ScanError = _core.ScanError
TimeoutError = _core.TimeoutError
FeatureUnavailableError = _core.FeatureUnavailableError
SerializationError = _core.SerializationError
InternalError = _core.InternalError

__all__ = [
    "__version__",
    "__version_info__",
    "features",
    "has_feature",
    "build_info",
    "EggsecError",
    "ConfigError",
    "ScopeError",
    "EnforcementError",
    "NetworkError",
    "ScanError",
    "TimeoutError",
    "FeatureUnavailableError",
    "SerializationError",
    "InternalError",
]
