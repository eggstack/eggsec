//! Dynamic SSL certificate generation for HTTPS interception
//!
//! Generates on-the-fly SSL certificates for intercepting HTTPS traffic.

use crate::error::{Result, SlapperError};
use rcgen::{
    BasicConstraints, Certificate, CertificateParams, DnType, ExtendedKeyUsagePurpose,
    KeyPair, SanType,
};
use rustc_hash::FxHashMap;
use parking_lot::RwLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub struct CertGenerator {
    cache: RwLock<HashMap<String, CachedCert>>,
    validity_duration: Duration,
}

struct CachedCert {
    certificate: Certificate,
    generated_at: u64,
}

impl CertGenerator {
    pub fn new() -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
            validity_duration: Duration::from_secs(86400),
        }
    }

    pub fn with_validity(mut self, duration: Duration) -> Self {
        self.validity_duration = duration;
        self
    }

    pub fn generate_for_host(&self, host: &str) -> Result<Certificate> {
        if let Some(cached) = self.get_cached(host) {
            return Ok(cached);
        }

        let cert = self.generate_cert(host)?;
        self.cache_cert(host, &cert);
        Ok(cert)
    }

    fn generate_cert(&self, host: &str) -> Result<Certificate> {
        let mut params = CertificateParams::default();

        params.subject = vec![(DnType::CommonName, host.into())];
        params.issuer = vec![(DnType::CommonName, "Slapper Proxy CA".into())];

        params.is_ca = BasicConstraints::Constrained(0);

        params.key_usages = vec![
            rcgen::KeyUsage::DigitalSignature,
            rcgen::KeyUsage::KeyEncipherment,
        ];

        params.ext_key_usages = vec![
            ExtendedKeyUsagePurpose::ServerAuth,
            ExtendedKeyUsagePurpose::ClientAuth,
        ];

        let mut alt_names = vec![SanType::DnsName(host.to_string())];

        if host == "localhost" || host == "127.0.0.1" {
            alt_names.push(SanType::IpAddress(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)));
        }

        if let Ok(ip) = host.parse() {
            alt_names.push(SanType::IpAddress(ip));
        }

        params.subject_alt_names = alt_names;

        let key_pair = KeyPair::generate_for(&rcgen::PKCS_ECDSA_P256_SHA256)
            .map_err(|e| SlapperError::Proxy(format!("Key generation failed: {}", e)))?;

        params.key_pair = Some(key_pair);

        Certificate::from_params(params)
            .map_err(|e| SlapperError::Proxy(format!("Certificate creation failed: {}", e)))
    }

    fn get_cached(&self, host: &str) -> Option<Certificate> {
        let cache = self.cache.read();

        cache.get(host).and_then(|cached| {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();

            if now - cached.generated_at < self.validity_duration.as_secs() {
                Some(cached.certificate.clone())
            } else {
                None
            }
        })
    }

    fn cache_cert(&self, host: &str, cert: &Certificate) {
        if let mut cache = self.cache.write() {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();

            cache.insert(
                host.to_string(),
                CachedCert {
                    certificate: cert.clone(),
                    generated_at: now,
                },
            );
        }
    }

    pub fn clear_cache(&self) {
        if let mut cache = self.cache.write() {
            cache.clear();
        }
    }
}

impl Default for CertGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for CertGenerator {
    fn clone(&self) -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
            validity_duration: self.validity_duration,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
