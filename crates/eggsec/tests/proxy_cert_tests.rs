use eggsec::proxy::intercept::cert::CertGenerator;
use std::time::Duration;

#[test]
fn test_cert_generation() {
    let generator = CertGenerator::new();
    let cert = generator.generate_for_host("example.com");
    assert!(cert.is_ok());
}

#[test]
fn test_cert_caching() {
    let generator = CertGenerator::new();

    let cert1 = generator.generate_for_host("example.com").unwrap();
    let cert2 = generator.generate_for_host("example.com").unwrap();

    assert_eq!(
        cert1.serialize_der().unwrap(),
        cert2.serialize_der().unwrap()
    );
}

#[test]
fn test_cert_caching_different_hosts() {
    let generator = CertGenerator::new();

    let cert1 = generator.generate_for_host("example.com").unwrap();
    let cert2 = generator.generate_for_host("test.com").unwrap();

    assert_ne!(
        cert1.serialize_der().unwrap(),
        cert2.serialize_der().unwrap()
    );
}

#[test]
fn test_cert_with_validity_duration() {
    let generator = CertGenerator::new().with_validity(Duration::from_secs(3600));
    let cert = generator.generate_for_host("example.com");
    assert!(cert.is_ok());
}

#[test]
fn test_cert_localhost_alt_names() {
    let generator = CertGenerator::new();

    let cert = generator.generate_for_host("localhost").unwrap();
    assert!(cert.is_ok());
}

#[test]
fn test_cert_ip_address() {
    let generator = CertGenerator::new();

    let cert = generator.generate_for_host("127.0.0.1").unwrap();
    assert!(cert.is_ok());
}

#[test]
fn test_cert_clone() {
    let generator = CertGenerator::new();
    generator.generate_for_host("example.com").unwrap();

    let cloned = generator.clone();
    let cert2 = cloned.generate_for_host("example.com").unwrap();
    assert!(cert2.is_ok());
}

#[test]
fn test_cert_clear_cache() {
    let generator = CertGenerator::new();
    generator.generate_for_host("example.com").unwrap();

    generator.clear_cache();

    let cert = generator.generate_for_host("example.com").unwrap();
    assert!(cert.is_ok());
}

#[test]
fn test_multiple_hosts_different_cache() {
    let generator = CertGenerator::new();

    generator.generate_for_host("host1.com").unwrap();
    generator.generate_for_host("host2.com").unwrap();
    generator.generate_for_host("host3.com").unwrap();

    let cert = generator.generate_for_host("host1.com").unwrap();
    assert!(cert.is_ok());
}

#[test]
fn test_cert_serialization() {
    let generator = CertGenerator::new();
    let cert = generator.generate_for_host("example.com").unwrap();

    let der = cert.serialize_der();
    assert!(der.is_ok());
    assert!(!der.unwrap().is_empty());
}

#[test]
fn test_cert_pem_serialization() {
    let generator = CertGenerator::new();
    let cert = generator.generate_for_host("example.com").unwrap();

    let pem = cert.serialize_pem();
    assert!(pem.is_ok());
    assert!(pem.unwrap().contains("CERTIFICATE"));
}
