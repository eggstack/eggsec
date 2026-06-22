#[cfg(feature = "cloud")]
use eggsec::recon::cloud::CloudDiscovery;
use eggsec::recon::threatintel::ThreatIntel;

#[test]
fn test_threat_intel_empty() {
    let intel = ThreatIntel::default();
    assert!(intel.target.is_empty());
    assert!(intel.ip_reputation.is_none());
    assert!(intel.domain_reputation.is_none());
    assert!(intel.passive_dns.is_empty());
}

#[test]
#[cfg(feature = "cloud")]
fn test_cloud_discovery_empty() {
    let discovery = CloudDiscovery::default();
    assert!(discovery.domain.is_empty());
    assert!(discovery.s3_buckets.is_empty());
    assert!(discovery.azure_blobs.is_empty());
    assert!(discovery.gcp_storage.is_empty());
    assert!(discovery.firebase.is_empty());
}

#[test]
fn test_whois_result_parsing() {
    let result = eggsec::recon::whois::WhoisResult {
        domain: "example.com".to_string(),
        registrar: Some("Example Registrar".to_string()),
        created_date: Some("2020-01-01".to_string()),
        expires_date: Some("2025-01-01".to_string()),
        updated_date: Some("2023-01-01".to_string()),
        nameservers: vec!["ns1.example.com".to_string()],
        status: vec!["ok".to_string()],
        registrant: None,
        raw_data: None,
    };

    assert_eq!(result.domain, "example.com");
    assert!(result.registrar.is_some());
    assert_eq!(result.nameservers.len(), 1);
}
