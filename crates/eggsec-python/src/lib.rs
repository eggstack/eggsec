mod async_client;
mod client;
#[cfg(feature = "container")]
mod container;
#[cfg(feature = "daemon-client")]
mod daemon;
#[cfg(feature = "db-pentest")]
mod db_pentest;
mod dto;
mod endpoint;
mod error;
mod features;
mod finding;
mod fingerprint;
#[cfg(feature = "git-secrets")]
mod git_secrets;
mod loadtest;
#[cfg(feature = "mobile")]
mod mobile;
#[cfg(feature = "nse")]
mod nse;
#[cfg(feature = "packet-inspection")]
mod packet_inspection;
#[cfg(feature = "web-proxy")]
mod proxy;
mod recon;
mod runtime_async;
mod runtime_sync;
#[cfg(feature = "sbom")]
mod sbom;
mod scanner;
mod scope;
#[cfg(feature = "stress-testing")]
mod stress;
mod version;
mod waf;
mod waf_validation;

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
    // Phase F Track 1: WAF validation and HTTP fuzzing
    m.add_class::<waf_validation::BypassResultPy>()?;
    m.add_class::<waf_validation::WafScanResultPy>()?;
    m.add_class::<waf_validation::PayloadPy>()?;
    m.add_class::<waf_validation::FuzzResultPy>()?;
    m.add_class::<waf_validation::FuzzSessionPy>()?;
    m.add_class::<waf_validation::FuzzConfig>()?;
    // Phase F Track 4: Git secrets
    #[cfg(feature = "git-secrets")]
    {
        m.add_class::<git_secrets::Confidence>()?;
        m.add_class::<git_secrets::SecretType>()?;
        m.add_class::<git_secrets::SecretFindingPy>()?;
        m.add_class::<git_secrets::GitSecretFindingPy>()?;
        m.add_class::<git_secrets::GitSecretsSummaryPy>()?;
        m.add_class::<git_secrets::GitSecretsReportPy>()?;
    }
    // Phase F Track 5: SBOM
    #[cfg(feature = "sbom")]
    {
        m.add_class::<sbom::SbomFormatPy>()?;
        m.add_class::<sbom::SbomComponentPy>()?;
        m.add_class::<sbom::SbomVulnerabilityPy>()?;
        m.add_class::<sbom::SbomReportPy>()?;
    }
    // Phase F Track 8: Mobile lab
    #[cfg(feature = "mobile")]
    {
        m.add_class::<mobile::MobilePlatformPy>()?;
        m.add_class::<mobile::MobileFindingPy>()?;
        m.add_class::<mobile::MobileScanReportPy>()?;
    }
    // Phase F Track 6: Database pentesting
    #[cfg(feature = "db-pentest")]
    {
        m.add_class::<db_pentest::DbFindingPy>()?;
        m.add_class::<db_pentest::DbPentestReportPy>()?;
        m.add_class::<db_pentest::DbPentestConfig>()?;
    }
    // Phase F Track 9: Container security
    #[cfg(feature = "container")]
    {
        m.add_class::<container::ContainerScanTypePy>()?;
        m.add_class::<container::EscapeRiskLevelPy>()?;
        m.add_class::<container::CisCheckStatusPy>()?;
        m.add_class::<container::ImageLayerPy>()?;
        m.add_class::<container::DockerMisconfigPy>()?;
        m.add_class::<container::DockerScanResultPy>()?;
        m.add_class::<container::ClusterInfoPy>()?;
        m.add_class::<container::K8sFindingPy>()?;
        m.add_class::<container::KubernetesScanResultPy>()?;
        m.add_class::<container::EscapeRiskPy>()?;
        m.add_class::<container::EscapeDetectionResultPy>()?;
        m.add_class::<container::CisCheckPy>()?;
        m.add_class::<container::CisBenchmarkResultPy>()?;
        m.add_class::<container::ContainerFindingPy>()?;
        m.add_class::<container::ContainerReportPy>()?;
    }
    // Phase F Track 10: Packet inspection
    #[cfg(feature = "packet-inspection")]
    {
        m.add_class::<packet_inspection::CaptureConfigPy>()?;
        m.add_class::<packet_inspection::CaptureStatsPy>()?;
        m.add_class::<packet_inspection::PacketInfoPy>()?;
        m.add_class::<packet_inspection::NetworkInterfaceInfoPy>()?;
        m.add_class::<packet_inspection::PcapWriterPy>()?;
    }
    // Phase F Track 2: Load testing
    m.add_class::<loadtest::LoadTestResultPy>()?;
    m.add_class::<loadtest::LoadTestConfig>()?;
    // Phase F Track 11: Stress testing
    #[cfg(feature = "stress-testing")]
    {
        m.add_class::<stress::StressTypePy>()?;
        m.add_class::<stress::StressConfigPy>()?;
        m.add_class::<stress::StressStatsPy>()?;
        m.add_class::<stress::StressConfigSummaryPy>()?;
        m.add_class::<stress::StressResultPy>()?;
    }
    // Phase F Track 12: NSE bindings
    #[cfg(feature = "nse")]
    {
        m.add_class::<nse::NseConfigPy>()?;
        m.add_class::<nse::NseLibraryUsePy>()?;
        m.add_class::<nse::NseRuleEvaluationPy>()?;
        m.add_class::<nse::NseReportPy>()?;
    }
    // Phase F Track 7: Proxy and web proxy
    #[cfg(feature = "web-proxy")]
    {
        m.add_class::<proxy::ProxyTypePy>()?;
        m.add_class::<proxy::RotationStrategyPy>()?;
        m.add_class::<proxy::ProxyConfigPy>()?;
        m.add_class::<proxy::ProxyEntryPy>()?;
        m.add_class::<proxy::ProxyManagerPy>()?;
        m.add_class::<proxy::HealthCheckResultPy>()?;
        m.add_class::<proxy::ProxyHealthPy>()?;
    }

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
    // Phase F Track 1: WAF validation and HTTP fuzzing functions
    m.add_function(wrap_pyfunction!(waf_validation::validate_waf, m)?)?;
    m.add_function(wrap_pyfunction!(waf_validation::async_validate_waf, m)?)?;
    m.add_function(wrap_pyfunction!(waf_validation::fuzz_http, m)?)?;
    m.add_function(wrap_pyfunction!(waf_validation::async_fuzz_http, m)?)?;
    m.add_function(wrap_pyfunction!(waf_validation::generate_fuzz_payloads, m)?)?;
    // Phase F Track 4: Git secrets functions
    #[cfg(feature = "git-secrets")]
    {
        m.add_function(wrap_pyfunction!(git_secrets::scan_git_secrets, m)?)?;
        m.add_function(wrap_pyfunction!(git_secrets::async_scan_git_secrets, m)?)?;
    }
    // Phase F Track 5: SBOM functions
    #[cfg(feature = "sbom")]
    {
        m.add_function(wrap_pyfunction!(sbom::generate_sbom, m)?)?;
        m.add_function(wrap_pyfunction!(sbom::async_generate_sbom, m)?)?;
    }
    // Phase F Track 8: Mobile functions
    #[cfg(feature = "mobile")]
    {
        m.add_function(wrap_pyfunction!(mobile::analyze_apk, m)?)?;
        m.add_function(wrap_pyfunction!(mobile::async_analyze_apk, m)?)?;
        m.add_function(wrap_pyfunction!(mobile::analyze_ipa, m)?)?;
        m.add_function(wrap_pyfunction!(mobile::async_analyze_ipa, m)?)?;
    }
    // Phase F Track 6: Database pentesting functions
    #[cfg(feature = "db-pentest")]
    {
        m.add_function(wrap_pyfunction!(db_pentest::db_probe, m)?)?;
        m.add_function(wrap_pyfunction!(db_pentest::async_db_probe, m)?)?;
        m.add_function(wrap_pyfunction!(db_pentest::db_probe_with_config, m)?)?;
        m.add_function(wrap_pyfunction!(db_pentest::db_probe_postgres, m)?)?;
        m.add_function(wrap_pyfunction!(db_pentest::db_probe_mysql, m)?)?;
        m.add_function(wrap_pyfunction!(db_pentest::db_probe_mssql, m)?)?;
        m.add_function(wrap_pyfunction!(db_pentest::db_probe_mongodb, m)?)?;
        m.add_function(wrap_pyfunction!(db_pentest::db_probe_redis, m)?)?;
    }
    // Phase F Track 9: Container functions
    #[cfg(feature = "container")]
    {
        m.add_function(wrap_pyfunction!(container::scan_docker_image, m)?)?;
        m.add_function(wrap_pyfunction!(container::async_scan_docker_image, m)?)?;
        m.add_function(wrap_pyfunction!(container::scan_kubernetes, m)?)?;
        m.add_function(wrap_pyfunction!(container::async_scan_kubernetes, m)?)?;
        m.add_function(wrap_pyfunction!(container::detect_escape_risks, m)?)?;
        m.add_function(wrap_pyfunction!(container::check_cis_docker_benchmark, m)?)?;
    }
    // Phase F Track 10: Packet inspection functions
    #[cfg(feature = "packet-inspection")]
    {
        m.add_function(wrap_pyfunction!(
            packet_inspection::list_network_interfaces,
            m
        )?)?;
        m.add_function(wrap_pyfunction!(packet_inspection::parse_pcap, m)?)?;
    }
    // Phase F Track 2: Load testing functions
    m.add_function(wrap_pyfunction!(loadtest::load_test_http, m)?)?;
    m.add_function(wrap_pyfunction!(loadtest::async_load_test_http, m)?)?;
    // Phase F Track 11: Stress testing functions
    #[cfg(feature = "stress-testing")]
    {
        m.add_function(wrap_pyfunction!(stress::stress_test, m)?)?;
        m.add_function(wrap_pyfunction!(stress::async_stress_test, m)?)?;
    }
    // Phase F Track 12: NSE functions
    #[cfg(feature = "nse")]
    {
        m.add_function(wrap_pyfunction!(nse::nse_run, m)?)?;
        m.add_function(wrap_pyfunction!(nse::async_nse_run, m)?)?;
        m.add_function(wrap_pyfunction!(nse::nse_list_libraries, m)?)?;
    }
    // Phase F Track 7: Proxy and web proxy functions
    #[cfg(feature = "web-proxy")]
    {
        m.add_function(wrap_pyfunction!(proxy::create_proxy_manager, m)?)?;
        m.add_function(wrap_pyfunction!(proxy::async_add_proxy, m)?)?;
        m.add_function(wrap_pyfunction!(proxy::async_proxy_health_check, m)?)?;
    }
    // Phase F Track 13: Daemon client
    #[cfg(feature = "daemon-client")]
    {
        m.add_class::<daemon::DaemonResponsePy>()?;
        m.add_class::<daemon::DaemonClientPy>()?;
        m.add_function(wrap_pyfunction!(daemon::daemon_connect, m)?)?;
        m.add_function(wrap_pyfunction!(daemon::async_daemon_health, m)?)?;
        m.add_function(wrap_pyfunction!(daemon::async_daemon_declare_client, m)?)?;
        m.add_function(wrap_pyfunction!(daemon::async_daemon_create_session, m)?)?;
        m.add_function(wrap_pyfunction!(daemon::async_daemon_list_sessions, m)?)?;
        m.add_function(wrap_pyfunction!(daemon::async_daemon_get_snapshot, m)?)?;
        m.add_function(wrap_pyfunction!(daemon::async_daemon_close_session, m)?)?;
    }

    Ok(())
}
