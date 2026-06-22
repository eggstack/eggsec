use eggsec::proxy::intercept::{CertGenerator, CertMaterial};
use std::time::Duration;

fn assert_valid_material(cert: &CertMaterial) {
    assert!(!cert.cert_der.is_empty());
    assert!(!cert.key_der.is_empty());
}

#[test]
fn test_cert_generation() {
    let generator = CertGenerator::new();
    let cert = generator.generate_for_host("example.com").unwrap();
    assert_valid_material(&cert);
}

#[test]
fn test_cert_caching() {
    let generator = CertGenerator::new();

    let cert1 = generator.generate_for_host("example.com").unwrap();
    let cert2 = generator.generate_for_host("example.com").unwrap();

    assert_eq!(cert1.cert_der, cert2.cert_der);
    assert_eq!(cert1.key_der, cert2.key_der);
}

#[test]
fn test_cert_caching_different_hosts() {
    let generator = CertGenerator::new();

    let cert1 = generator.generate_for_host("example.com").unwrap();
    let cert2 = generator.generate_for_host("test.com").unwrap();

    assert_ne!(cert1.cert_der, cert2.cert_der);
}

#[test]
fn test_cert_with_validity_duration() {
    let generator = CertGenerator::new().with_validity(Duration::from_secs(3600));
    let cert = generator.generate_for_host("example.com").unwrap();
    assert_valid_material(&cert);
}

#[test]
fn test_cert_localhost_alt_names() {
    let generator = CertGenerator::new();

    let cert = generator.generate_for_host("localhost").unwrap();
    assert_valid_material(&cert);
}

#[test]
fn test_cert_ip_address() {
    let generator = CertGenerator::new();

    let cert = generator.generate_for_host("127.0.0.1").unwrap();
    assert_valid_material(&cert);
}

#[test]
fn test_cert_clone() {
    let generator = CertGenerator::new();
    let cert1 = generator.generate_for_host("example.com").unwrap();

    let cloned = generator.clone();
    let cert2 = cloned.generate_for_host("example.com").unwrap();
    assert_eq!(cert1.cert_der, cert2.cert_der);
    assert_eq!(cert1.key_der, cert2.key_der);
}

#[test]
fn test_cert_clear_cache() {
    let generator = CertGenerator::new();
    let before = generator.generate_for_host("example.com").unwrap();

    generator.clear_cache();

    let after = generator.generate_for_host("example.com").unwrap();
    assert_ne!(before.cert_der, after.cert_der);
    assert_ne!(before.key_der, after.key_der);
}

#[test]
fn test_multiple_hosts_different_cache() {
    let generator = CertGenerator::new();

    generator.generate_for_host("host1.com").unwrap();
    generator.generate_for_host("host2.com").unwrap();
    generator.generate_for_host("host3.com").unwrap();

    let cert = generator.generate_for_host("host1.com").unwrap();
    assert_valid_material(&cert);
}

#[test]
fn test_cert_serialization() {
    let generator = CertGenerator::new();
    let cert = generator.generate_for_host("example.com").unwrap();

    assert_valid_material(&cert);
}
