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
scan_ports = _core.scan_ports
async_scan_ports = _core.async_scan_ports
scan_endpoints = _core.scan_endpoints
async_scan_endpoints = _core.async_scan_endpoints
fingerprint_services = _core.fingerprint_services
async_fingerprint_services = _core.async_fingerprint_services

# Re-export classes
Scope = _core.Scope
Client = _core.Client
AsyncClient = _core.AsyncClient
PyFuture = _core.PyFuture
PortScanResult = _core.PortScanResult
OpenPort = _core.OpenPort
ScanStats = _core.ScanStats
PortRange = _core.PortRange
TimingPreset = _core.TimingPreset
EndpointScanConfig = _core.EndpointScanConfig
EndpointFinding = _core.EndpointFinding
EndpointScanStats = _core.EndpointScanStats
EndpointScanResult = _core.EndpointScanResult
FingerprintEvidence = _core.FingerprintEvidence
FingerprintConfidence = _core.FingerprintConfidence
ServiceFingerprintResult = _core.ServiceFingerprintResult
FingerprintScanResult = _core.FingerprintScanResult

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
    # Functions
    "features",
    "has_feature",
    "build_info",
    "scan_ports",
    "async_scan_ports",
    "scan_endpoints",
    "async_scan_endpoints",
    "fingerprint_services",
    "async_fingerprint_services",
    # Classes
    "Scope",
    "Client",
    "AsyncClient",
    "PyFuture",
    "PortScanResult",
    "OpenPort",
    "ScanStats",
    "PortRange",
    "TimingPreset",
    "EndpointScanConfig",
    "EndpointFinding",
    "EndpointScanStats",
    "EndpointScanResult",
    "FingerprintEvidence",
    "FingerprintConfidence",
    "ServiceFingerprintResult",
    "FingerprintScanResult",
    # Exceptions
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
