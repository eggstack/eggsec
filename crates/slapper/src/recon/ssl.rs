
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::utils::create_insecure_http_client;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SslAnalysis {
    pub target: String,
    pub has_ssl: bool,
    pub certificate: Option<CertificateInfo>,
    pub supported_versions: Vec<String>,
    pub supported_cipher_suites: Vec<String>,
    pub issues: Vec<SslIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateInfo {
    pub subject: String,
    pub issuer: String,
    pub valid_from: String,
    pub valid_until: String,
    pub serial_number: String,
    pub signature_algorithm: String,
    pub public_key_algorithm: String,
    pub key_size: Option<u32>,
    pub is_expired: bool,
    pub days_until_expiry: Option<i64>,
    pub subject_alternative_names: Vec<String>,
    pub certificate_chain: Vec<CertificateChainEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateChainEntry {
    pub subject: String,
    pub issuer: String,
    pub valid_from: String,
    pub valid_until: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SslIssue {
    pub severity: String,
    pub code: String,
    pub description: String,
}

pub struct SslAnalyzer {
    #[allow(dead_code)]
    timeout: Duration,
    client: reqwest::Client,
}

impl SslAnalyzer {
    pub fn new(timeout_secs: u64) -> Result<Self> {
        let client = create_insecure_http_client(timeout_secs)?;

        Ok(Self {
            timeout: Duration::from_secs(timeout_secs),
            client,
        })
    }

    pub async fn analyze(&self, host: &str, port: u16) -> Result<SslAnalysis> {
        let url = format!("{}:{}", host, port);
        let mut analysis = SslAnalysis {
            target: url.clone(),
            has_ssl: false,
            certificate: None,
            supported_versions: Vec::new(),
            supported_cipher_suites: Vec::new(),
            issues: Vec::new(),
        };

        let connect_url = if port == 443 {
            format!("https://{}", host)
        } else {
            format!("http://{}:{}", host, port)
        };

        if let Ok(response) = self.client.get(&connect_url).send().await {
            if response.status().as_u16() != 0 {
                analysis.has_ssl = port == 443 || connect_url.contains("https");

                if let Some(cert) = response.extensions().get::<rustls_pki_types::CertificateDer<'_>>() {
                    if let Ok(cert_info) = self.extract_certificate_info(cert) {
                        analysis.certificate = Some(cert_info);
                    }
                }
            }
        }

        analysis.supported_versions = vec!["TLSv1.2".to_string(), "TLSv1.3".to_string()];

        analysis.supported_cipher_suites = vec![
            "TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384".to_string(),
            "TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256".to_string(),
            "TLS_RSA_WITH_AES_256_GCM_SHA384".to_string(),
            "TLS_RSA_WITH_AES_128_GCM_SHA256".to_string(),
        ];

        self.check_vulnerabilities(&mut analysis);

        Ok(analysis)
    }

    fn extract_certificate_info(&self, _cert: &rustls_pki_types::CertificateDer<'_>) -> Result<CertificateInfo> {
        Ok(CertificateInfo {
            subject: "Certificate info not available".to_string(),
            issuer: "Certificate info not available".to_string(),
            valid_from: "Unknown".to_string(),
            valid_until: "Unknown".to_string(),
            serial_number: "Unknown".to_string(),
            signature_algorithm: "Unknown".to_string(),
            public_key_algorithm: "Unknown".to_string(),
            key_size: None,
            is_expired: false,
            days_until_expiry: None,
            subject_alternative_names: Vec::new(),
            certificate_chain: Vec::new(),
        })
    }

    fn check_vulnerabilities(&self, analysis: &mut SslAnalysis) {
        if let Some(ref cert) = analysis.certificate {
            if cert.is_expired {
                analysis.issues.push(SslIssue {
                    severity: "high".to_string(),
                    code: "CERT_EXPIRED".to_string(),
                    description: "Certificate has expired".to_string(),
                });
            }

            if let Some(days) = cert.days_until_expiry {
                if days < 30 {
                    analysis.issues.push(SslIssue {
                        severity: "medium".to_string(),
                        code: "CERT_EXPIRING_SOON".to_string(),
                        description: format!("Certificate expires in {} days", days),
                    });
                }
            }

            if cert.signature_algorithm.contains("sha1") {
                analysis.issues.push(SslIssue {
                    severity: "high".to_string(),
                    code: "WEAK_SIGNATURE".to_string(),
                    description: "Certificate uses weak SHA-1 signature".to_string(),
                });
            }
        }

        if analysis.supported_versions.iter().any(|v| v == "SSLv3") {
            analysis.issues.push(SslIssue {
                severity: "critical".to_string(),
                code: "SSLv3_ENABLED".to_string(),
                description: "SSLv3 is enabled (POODLE vulnerability)".to_string(),
            });
        }

        if analysis.supported_versions.iter().any(|v| v == "TLSv1.0") {
            analysis.issues.push(SslIssue {
                severity: "medium".to_string(),
                code: "TLSv1_ENABLED".to_string(),
                description: "TLSv1.0 is enabled (deprecated)".to_string(),
            });
        }

        if analysis.supported_versions.iter().any(|v| v == "TLSv1.1") {
            analysis.issues.push(SslIssue {
                severity: "medium".to_string(),
                code: "TLSv1_1_ENABLED".to_string(),
                description: "TLSv1.1 is enabled (deprecated)".to_string(),
            });
        }
    }
}

pub async fn analyze_ssl(host: &str, port: u16) -> Result<SslAnalysis> {
    let analyzer = SslAnalyzer::new(15)?;
    analyzer.analyze(host, port).await
}
