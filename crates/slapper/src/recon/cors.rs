use crate::error::Result;
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};

use crate::utils::create_insecure_http_client;

#[derive(Debug, Clone, Serialize, Deserialize, Default, Eq, PartialEq, Hash)]
pub struct CorsAnalysis {
    pub url: String,
    pub findings: Vec<CorsFinding>,
    pub analyzed_origins: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct CorsFinding {
    pub origin: String,
    pub allows_origin: bool,
    pub access_control_allow_origin: Option<String>,
    pub access_control_allow_credentials: bool,
    pub access_control_allow_methods: Vec<String>,
    pub access_control_allow_headers: Vec<String>,
    pub access_control_expose_headers: Vec<String>,
    pub access_control_max_age: Option<i64>,
    pub is_vulnerable: bool,
    pub vulnerability_type: Option<String>,
    pub severity: String,
}

pub struct CorsAnalyzer {
    client: reqwest::Client,
}

impl CorsAnalyzer {
    pub fn new() -> Result<Self> {
        let client = create_insecure_http_client(30)?;

        Ok(Self { client })
    }

    pub async fn analyze(&self, url: &str) -> Result<CorsAnalysis> {
        let test_origins = self.generate_test_origins();

        let mut findings = FxHashSet::default();
        let mut analyzed_count = 0;

        for origin in test_origins {
            if let Some(finding) = self.test_origin(url, &origin).await {
                analyzed_count += 1;
                findings.insert(finding);
            }
        }

        let findings_vec: Vec<CorsFinding> = findings.into_iter().collect();

        Ok(CorsAnalysis {
            url: url.to_string(),
            findings: findings_vec,
            analyzed_origins: analyzed_count,
        })
    }

    fn generate_test_origins(&self) -> Vec<String> {
        vec![
            "https://evil.com".to_string(),
            "http://evil.com".to_string(),
            "https://attacker.com".to_string(),
            "null".to_string(),
            "*".to_string(),
            "https://localhost".to_string(),
            "http://localhost".to_string(),
            "https://127.0.0.1".to_string(),
            "http://127.0.0.1".to_string(),
        ]
    }

    async fn test_origin(&self, url: &str, test_origin: &str) -> Option<CorsFinding> {
        let test_url = format!("{}/", url.trim_end_matches('/'));

        let request = match self
            .client
            .get(&test_url)
            .header("Origin", test_origin)
            .header("Access-Control-Request-Method", "GET")
            .build()
        {
            Ok(req) => req,
            Err(e) => {
                tracing::debug!("Failed to build CORS test request for {}: {}", test_origin, e);
                return None;
            }
        };

        let response = match self.client.execute(request).await {
            Ok(resp) => resp,
            Err(e) => {
                tracing::debug!(
                    "CORS test request failed for {}: {}",
                    test_origin,
                    e
                );
                return None;
            }
        };

        let acao = response
            .headers()
            .get("access-control-allow-origin")
            .and_then(|v| v.to_str().ok())
            .map(String::from);

        let acac = response
            .headers()
            .get("access-control-allow-credentials")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_lowercase() == "true")
            .unwrap_or(false);

        let acam = response
            .headers()
            .get("access-control-allow-methods")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.split(',').map(|m| m.trim().to_string()).collect())
            .unwrap_or_else(|| {
                tracing::debug!("access-control-allow-methods header missing");
                Vec::new()
            });

        let acah = response
            .headers()
            .get("access-control-allow-headers")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.split(',').map(|h| h.trim().to_string()).collect())
            .unwrap_or_else(|| {
                tracing::debug!("access-control-allow-headers header missing");
                Vec::new()
            });

        let aceh = response
            .headers()
            .get("access-control-expose-headers")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.split(',').map(|h| h.trim().to_string()).collect())
            .unwrap_or_else(|| {
                tracing::debug!("access-control-expose-headers header missing");
                Vec::new()
            });

        let acma = response
            .headers()
            .get("access-control-max-age")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<i64>().ok());

        let allows_origin = acao
            .as_ref()
            .map(|h| h == test_origin || h == "*")
            .unwrap_or(false);

        let (is_vulnerable, vuln_type) = self.check_vulnerability(test_origin, &acao, acac);

        if acao.is_none() && !acac {
            return None;
        }

        Some(CorsFinding {
            origin: test_origin.to_string(),
            allows_origin,
            access_control_allow_origin: acao,
            access_control_allow_credentials: acac,
            access_control_allow_methods: acam,
            access_control_allow_headers: acah,
            access_control_expose_headers: aceh,
            access_control_max_age: acma,
            is_vulnerable,
            vulnerability_type: vuln_type,
            severity: if is_vulnerable { "high" } else { "info" }.to_string(),
        })
    }

    pub(crate) fn check_vulnerability(
        &self,
        test_origin: &str,
        acao: &Option<String>,
        acac: bool,
    ) -> (bool, Option<String>) {
        if let Some(header) = acao {
            if header == "*" && acac {
                return (true, Some("Wildcard with credentials".to_string()));
            }

            if header == "null" {
                return (true, Some("Null origin allowed".to_string()));
            }

            if test_origin == "null" && !header.is_empty() {
                return (true, Some("Null origin reflection".to_string()));
            }
        }

        (false, None)
    }
}

pub async fn analyze_cors(url: &str) -> Result<CorsAnalysis> {
    let analyzer = CorsAnalyzer::new()?;
    analyzer.analyze(url).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_vulnerability_wildcard_with_credentials() {
        let analyzer = CorsAnalyzer::new().unwrap();
        let (vulnerable, vuln_type) =
            analyzer.check_vulnerability("https://evil.com", &Some("*".to_string()), true);
        assert!(vulnerable);
        assert_eq!(vuln_type, Some("Wildcard with credentials".to_string()));
    }

    #[test]
    fn test_check_vulnerability_null_origin_allowed() {
        let analyzer = CorsAnalyzer::new().unwrap();
        let (vulnerable, vuln_type) =
            analyzer.check_vulnerability("https://evil.com", &Some("null".to_string()), false);
        assert!(vulnerable);
        assert_eq!(vuln_type, Some("Null origin allowed".to_string()));
    }

    #[test]
    fn test_check_vulnerability_null_reflection() {
        let analyzer = CorsAnalyzer::new().unwrap();
        let (vulnerable, vuln_type) =
            analyzer.check_vulnerability("null", &Some("https://evil.com".to_string()), false);
        assert!(vulnerable);
        assert_eq!(vuln_type, Some("Null origin reflection".to_string()));
    }

    #[test]
    fn test_check_vulnerability_not_vulnerable() {
        let analyzer = CorsAnalyzer::new().unwrap();
        let (vulnerable, vuln_type) = analyzer.check_vulnerability(
            "https://example.com",
            &Some("https://example.com".to_string()),
            false,
        );
        assert!(!vulnerable);
        assert!(vuln_type.is_none());
    }

    #[test]
    fn test_check_vulnerability_no_acao_header() {
        let analyzer = CorsAnalyzer::new().unwrap();
        let (vulnerable, vuln_type) =
            analyzer.check_vulnerability("https://evil.com", &None, false);
        assert!(!vulnerable);
        assert!(vuln_type.is_none());
    }

    #[test]
    fn test_generate_test_origins_count() {
        let analyzer = CorsAnalyzer::new().unwrap();
        let origins = analyzer.generate_test_origins();
        assert_eq!(origins.len(), 9);
        assert!(origins.contains(&"https://evil.com".to_string()));
        assert!(origins.contains(&"null".to_string()));
        assert!(origins.contains(&"*".to_string()));
    }
}
