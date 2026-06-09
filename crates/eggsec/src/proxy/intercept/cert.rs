//! Dynamic SSL certificate generation for HTTPS interception
//!
//! Generates on-the-fly SSL certificates for intercepting HTTPS traffic.

use crate::error::{EggsecError, Result};
use parking_lot::RwLock;
use rcgen::{
    BasicConstraints, CertificateParams, ExtendedKeyUsagePurpose, IsCa, KeyPair, KeyUsagePurpose,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub struct CertGenerator {
    cache: Arc<RwLock<HashMap<String, CachedCert>>>,
    validity_duration: Duration,
}

struct CachedCert {
    der_bytes: Vec<u8>,
    key_der_bytes: Vec<u8>,
    generated_at: u64,
}

fn unix_timestamp_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

impl CertGenerator {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            validity_duration: Duration::from_secs(86400),
        }
    }

    pub fn with_validity(mut self, duration: Duration) -> Self {
        self.validity_duration = duration;
        self
    }

    pub fn generate_for_host(&self, host: &str) -> Result<CertMaterial> {
        if let Some(cached) = self.get_cached(host) {
            return Ok(cached);
        }

        let material = self.generate_cert(host)?;
        self.cache_cert(host, &material);
        Ok(material)
    }

    fn generate_cert(&self, host: &str) -> Result<CertMaterial> {
        let mut alt_names = vec![host.to_string()];

        if host == "localhost" || host == "127.0.0.1" {
            alt_names.push("127.0.0.1".to_string());
        }

        let mut params = CertificateParams::new(alt_names)
            .map_err(|e| EggsecError::Proxy(format!("Certificate params failed: {}", e)))?;

        params.is_ca = IsCa::Ca(BasicConstraints::Constrained(0));

        params.key_usages = vec![
            KeyUsagePurpose::DigitalSignature,
            KeyUsagePurpose::KeyEncipherment,
        ];

        params.extended_key_usages = vec![
            ExtendedKeyUsagePurpose::ServerAuth,
            ExtendedKeyUsagePurpose::ClientAuth,
        ];

        let key_pair = KeyPair::generate()
            .map_err(|e| EggsecError::Proxy(format!("Key generation failed: {}", e)))?;

        let cert = params
            .self_signed(&key_pair)
            .map_err(|e| EggsecError::Proxy(format!("Certificate creation failed: {}", e)))?;

        let der_bytes = cert.der().to_vec();
        let key_der_bytes = key_pair.serialize_der();

        Ok(CertMaterial {
            cert_der: der_bytes,
            key_der: key_der_bytes,
        })
    }

    fn get_cached(&self, host: &str) -> Option<CertMaterial> {
        let cache = self.cache.read();

        cache.get(host).and_then(|cached| {
            let now = unix_timestamp_secs();

            let age = now.saturating_sub(cached.generated_at);
            if age < self.validity_duration.as_secs() {
                Some(CertMaterial {
                    cert_der: cached.der_bytes.clone(),
                    key_der: cached.key_der_bytes.clone(),
                })
            } else {
                None
            }
        })
    }

    fn cache_cert(&self, host: &str, material: &CertMaterial) {
        if let Some(mut cache) = self.cache.try_write() {
            let now = unix_timestamp_secs();

            cache.insert(
                host.to_string(),
                CachedCert {
                    der_bytes: material.cert_der.clone(),
                    key_der_bytes: material.key_der.clone(),
                    generated_at: now,
                },
            );
        }
    }

    pub fn clear_cache(&self) {
        if let Some(mut cache) = self.cache.try_write() {
            cache.clear();
        }
    }
}

#[derive(Debug, Clone)]
pub struct CertMaterial {
    pub cert_der: Vec<u8>,
    pub key_der: Vec<u8>,
}

impl Default for CertGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for CertGenerator {
    fn clone(&self) -> Self {
        Self {
            cache: Arc::clone(&self.cache),
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

        assert_eq!(cert1.cert_der, cert2.cert_der);
        assert_eq!(cert1.key_der, cert2.key_der);
    }

    #[test]
    fn test_cert_material_has_data() {
        let generator = CertGenerator::new();
        let material = generator.generate_for_host("example.com").unwrap();
        assert!(!material.cert_der.is_empty());
        assert!(!material.key_der.is_empty());
    }
}
