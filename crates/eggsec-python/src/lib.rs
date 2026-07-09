mod async_client;
mod client;
mod dto;
mod endpoint;
mod error;
mod features;
mod fingerprint;
mod finding;
mod recon;
mod runtime_async;
mod runtime_sync;
mod scanner;
mod scope;
mod version;
mod waf;

pub use error::*;
use pyo3::prelude::*;

/// The eggsec Python module.
///
/// Python bindings for the Eggsec security assessment engine.
/// This is a host-language binding over the Rust engine, not an internal plugin runtime.
#[pymodule]
pub fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("__version_info__", (0, 1, 0))?;

    // Exceptions
    m.add("EggsecError", m.py().get_type_bound::<EggsecError>())?;
    m.add("ConfigError", m.py().get_type_bound::<ConfigError>())?;
    m.add("ScopeError", m.py().get_type_bound::<ScopeError>())?;
    m.add(
        "EnforcementError",
        m.py().get_type_bound::<EnforcementError>(),
    )?;
    m.add("NetworkError", m.py().get_type_bound::<NetworkError>())?;
    m.add("ScanError", m.py().get_type_bound::<ScanError>())?;
    m.add("TimeoutError", m.py().get_type_bound::<TimeoutError>())?;
    m.add(
        "FeatureUnavailableError",
        m.py().get_type_bound::<FeatureUnavailableError>(),
    )?;
    m.add(
        "SerializationError",
        m.py().get_type_bound::<SerializationError>(),
    )?;
    m.add("InternalError", m.py().get_type_bound::<InternalError>())?;

    // Classes
    m.add_class::<scope::Scope>()?;
    m.add_class::<client::Client>()?;
    m.add_class::<async_client::AsyncClient>()?;
    m.add_class::<runtime_async::PyFuture>()?;
    m.add_class::<dto::PortScanResult>()?;
    m.add_class::<dto::OpenPort>()?;
    m.add_class::<dto::ScanStats>()?;
    m.add_class::<dto::PortRange>()?;
    m.add_class::<dto::TimingPreset>()?;
    m.add_class::<endpoint::EndpointScanConfig>()?;
    m.add_class::<endpoint::EndpointFinding>()?;
    m.add_class::<endpoint::EndpointScanStats>()?;
    m.add_class::<endpoint::EndpointScanResult>()?;
    m.add_class::<fingerprint::FingerprintEvidence>()?;
    m.add_class::<fingerprint::FingerprintConfidence>()?;
    m.add_class::<fingerprint::ServiceFingerprintResult>()?;
    m.add_class::<fingerprint::FingerprintScanResult>()?;
    // Phase D: Findings and reporting
    m.add_class::<finding::Severity>()?;
    m.add_class::<finding::Evidence>()?;
    m.add_class::<finding::Finding>()?;
    m.add_class::<finding::FindingSet>()?;
    m.add_class::<finding::Report>()?;
    // Phase D: Recon
    m.add_class::<recon::DnsRecordSet>()?;
    m.add_class::<recon::MxRecord>()?;
    m.add_class::<recon::SoaRecord>()?;
    m.add_class::<recon::TlsCertificateInfo>()?;
    m.add_class::<recon::TlsInspectionResult>()?;
    m.add_class::<recon::SslIssue>()?;
    m.add_class::<recon::TechStack>()?;
    m.add_class::<recon::TechDetectionResult>()?;
    // Phase D: WAF detection
    m.add_class::<waf::WafDetectionResultPy>()?;

    // Functions
    m.add_function(wrap_pyfunction!(features::features, m)?)?;
    m.add_function(wrap_pyfunction!(features::has_feature, m)?)?;
    m.add_function(wrap_pyfunction!(version::build_info, m)?)?;
    m.add_function(wrap_pyfunction!(scanner::scan_ports, m)?)?;
    m.add_function(wrap_pyfunction!(scanner::async_scan_ports, m)?)?;
    m.add_function(wrap_pyfunction!(scanner::scan_endpoints, m)?)?;
    m.add_function(wrap_pyfunction!(scanner::async_scan_endpoints, m)?)?;
    m.add_function(wrap_pyfunction!(scanner::fingerprint_services, m)?)?;
    m.add_function(wrap_pyfunction!(scanner::async_fingerprint_services, m)?)?;
    // Phase D: Recon functions
    m.add_function(wrap_pyfunction!(recon::recon_dns, m)?)?;
    m.add_function(wrap_pyfunction!(recon::async_recon_dns, m)?)?;
    m.add_function(wrap_pyfunction!(recon::inspect_tls, m)?)?;
    m.add_function(wrap_pyfunction!(recon::async_inspect_tls, m)?)?;
    m.add_function(wrap_pyfunction!(recon::detect_technology, m)?)?;
    m.add_function(wrap_pyfunction!(recon::async_detect_technology, m)?)?;
    // Phase D: WAF functions
    m.add_function(wrap_pyfunction!(waf::detect_waf, m)?)?;
    m.add_function(wrap_pyfunction!(waf::async_detect_waf, m)?)?;

    Ok(())
}
