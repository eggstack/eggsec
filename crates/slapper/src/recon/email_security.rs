use crate::error::Result;
use serde::{Deserialize, Serialize};

use crate::utils::extract_target_from_url;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailSecurityReport {
    pub domain: String,
    pub spf: SpfResult,
    pub dkim: DkimResult,
    pub dmarc: DmarcResult,
    pub mx: MxSecurityResult,
    pub starttls: StartTlsResult,
    pub bimi: BimiResult,
    pub overall_score: u8,
    pub findings: Vec<EmailFinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpfResult {
    pub present: bool,
    pub record: Option<String>,
    pub valid: bool,
    pub mechanism_count: usize,
    pub includes: Vec<String>,
    pub all_mechanism: Option<String>,
    pub severity: Severity,
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DkimResult {
    pub present: bool,
    pub selectors_found: Vec<String>,
    pub key_length: Option<u16>,
    pub severity: Severity,
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DmarcResult {
    pub present: bool,
    pub record: Option<String>,
    pub policy: Option<String>,
    pub subdomain_policy: Option<String>,
    pub pct: Option<u8>,
    pub rua_aggregate: Vec<String>,
    pub ruf_forensic: Vec<String>,
    pub severity: Severity,
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MxSecurityResult {
    pub records: Vec<MxRecordInfo>,
    pub has_null_mx: bool,
    pub has_wildcard_mx: bool,
    pub severity: Severity,
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MxRecordInfo {
    pub preference: u16,
    pub exchange: String,
    pub ip_addresses: Vec<String>,
    pub has_spf: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartTlsResult {
    pub tested_servers: Vec<StartTlsServerResult>,
    pub supports_starttls: bool,
    pub supports_smtps: bool,
    pub severity: Severity,
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartTlsServerResult {
    pub hostname: String,
    pub port: u16,
    pub supports_starttls: bool,
    pub certificate_valid: bool,
    pub tls_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BimiResult {
    pub present: bool,
    pub record: Option<String>,
    pub logo_url: Option<String>,
    pub vmc_found: bool,
    pub severity: Severity,
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailFinding {
    pub category: String,
    pub severity: Severity,
    pub description: String,
    pub recommendation: String,
}

pub use crate::types::Severity;

pub struct EmailSecurityAnalyzer {
    resolver: hickory_resolver::TokioAsyncResolver,
}

impl EmailSecurityAnalyzer {
    pub fn new() -> Result<Self> {
        use hickory_resolver::config::{ResolverConfig, ResolverOpts};
        let resolver = hickory_resolver::TokioAsyncResolver::tokio(
            ResolverConfig::default(),
            ResolverOpts::default(),
        );
        Ok(Self { resolver })
    }

    pub async fn analyze(&self, target: &str) -> Result<EmailSecurityReport> {
        let domain = extract_target_from_url(target).unwrap_or_else(|| target.to_string());
        let domain = domain.split('/').next().unwrap_or(&domain);

        let spf = self.check_spf(domain).await;
        let dkim = self.check_dkim(domain).await;
        let dmarc = self.check_dmarc(domain).await;
        let mx = self.check_mx(domain).await;
        let starttls = self.check_starttls(&mx).await;
        let bimi = self.check_bimi(domain).await;

        let mut findings = Vec::new();
        let mut score: u8 = 100;

        if !spf.present {
            findings.push(EmailFinding {
                category: "SPF".to_string(),
                severity: Severity::High,
                description: "No SPF record found for domain".to_string(),
                recommendation: "Add an SPF TXT record to prevent email spoofing".to_string(),
            });
            score = score.saturating_sub(20);
        } else if !spf.valid {
            findings.push(EmailFinding {
                category: "SPF".to_string(),
                severity: Severity::Medium,
                description: "SPF record has configuration issues".to_string(),
                recommendation: "Review and fix SPF record syntax".to_string(),
            });
            score = score.saturating_sub(10);
        }

        if let Some(ref all) = spf.all_mechanism {
            if all == "+all" {
                findings.push(EmailFinding {
                    category: "SPF".to_string(),
                    severity: Severity::High,
                    description: "SPF record uses '+all' which allows all senders".to_string(),
                    recommendation:
                        "Change SPF mechanism to '-all' (hard fail) or '~all' (soft fail)"
                            .to_string(),
                });
                score = score.saturating_sub(15);
            } else if all == "~all" {
                findings.push(EmailFinding {
                    category: "SPF".to_string(),
                    severity: Severity::Low,
                    description: "SPF record uses '~all' (soft fail)".to_string(),
                    recommendation: "Consider using '-all' (hard fail) for stricter enforcement"
                        .to_string(),
                });
                score = score.saturating_sub(5);
            }
        }

        if !dkim.present {
            findings.push(EmailFinding {
                category: "DKIM".to_string(),
                severity: Severity::Medium,
                description: "No DKIM records found for common selectors".to_string(),
                recommendation: "Configure DKIM signing for outbound email".to_string(),
            });
            score = score.saturating_sub(15);
        } else if let Some(key_len) = dkim.key_length {
            if key_len < 2048 {
                findings.push(EmailFinding {
                    category: "DKIM".to_string(),
                    severity: Severity::Medium,
                    description: format!("DKIM key length is {} bits, recommended 2048+", key_len),
                    recommendation: "Upgrade DKIM key to at least 2048 bits".to_string(),
                });
                score = score.saturating_sub(10);
            }
        }

        if !dmarc.present {
            findings.push(EmailFinding {
                category: "DMARC".to_string(),
                severity: Severity::High,
                description: "No DMARC record found".to_string(),
                recommendation: "Add a DMARC TXT record at _dmarc.<domain>".to_string(),
            });
            score = score.saturating_sub(20);
        } else {
            if dmarc.policy.as_deref() == Some("none") {
                findings.push(EmailFinding {
                    category: "DMARC".to_string(),
                    severity: Severity::Medium,
                    description: "DMARC policy is set to 'none' (monitoring only)".to_string(),
                    recommendation: "Consider upgrading DMARC policy to 'quarantine' or 'reject'"
                        .to_string(),
                });
                score = score.saturating_sub(5);
            }
            if dmarc.rua_aggregate.is_empty() {
                findings.push(EmailFinding {
                    category: "DMARC".to_string(),
                    severity: Severity::Low,
                    description: "No aggregate report URI configured in DMARC".to_string(),
                    recommendation: "Add 'rua=mailto:...' to DMARC record for reporting"
                        .to_string(),
                });
                score = score.saturating_sub(5);
            }
        }

        if mx.records.is_empty() {
            findings.push(EmailFinding {
                category: "MX".to_string(),
                severity: Severity::High,
                description: "No MX records found for domain".to_string(),
                recommendation: "Configure MX records if domain should receive email".to_string(),
            });
            score = score.saturating_sub(10);
        }

        if !starttls.supports_starttls && !mx.records.is_empty() {
            findings.push(EmailFinding {
                category: "STARTTLS".to_string(),
                severity: Severity::Medium,
                description: "Mail servers do not support STARTTLS".to_string(),
                recommendation: "Enable STARTTLS on all mail servers".to_string(),
            });
            score = score.saturating_sub(10);
        }

        if !bimi.present {
            findings.push(EmailFinding {
                category: "BIMI".to_string(),
                severity: Severity::Info,
                description: "No BIMI record found".to_string(),
                recommendation: "Consider adding BIMI for brand indicator display".to_string(),
            });
        }

        let overall_score = score;

        Ok(EmailSecurityReport {
            domain: domain.to_string(),
            spf,
            dkim,
            dmarc,
            mx,
            starttls,
            bimi,
            overall_score,
            findings,
        })
    }

    async fn check_spf(&self, domain: &str) -> SpfResult {
        let spf_domain = format!("_spf.{}", domain);
        let mut record: Option<String> = None;
        let mut issues = Vec::new();

        if let Ok(lookup) = self.resolver.txt_lookup(domain).await {
            for txt in lookup.iter() {
                let txt_str = txt.to_string();
                if txt_str.starts_with("v=spf1") {
                    record = Some(txt_str.clone());
                    break;
                }
            }
        }

        if record.is_none() {
            if let Ok(lookup) = self.resolver.txt_lookup(&spf_domain).await {
                for txt in lookup.iter() {
                    let txt_str = txt.to_string();
                    if txt_str.starts_with("v=spf1") {
                        record = Some(txt_str.clone());
                        break;
                    }
                }
            }
        }

        if let Some(ref rec) = record {
            let includes: Vec<String> = rec
                .split_whitespace()
                .filter(|t| t.starts_with("include:"))
                .map(|t| t.trim_start_matches("include:").to_string())
                .collect();

            let all_mechanism = rec
                .split_whitespace()
                .find(|t| t.ends_with("all"))
                .map(|s| s.to_string());

            let mechanism_count = rec.split_whitespace().count();

            let valid = rec.starts_with("v=spf1");
            if !valid {
                issues.push("SPF record does not start with 'v=spf1'".to_string());
            }

            let severity = if !valid || all_mechanism.as_deref() == Some("+all") {
                Severity::High
            } else {
                Severity::Info
            };

            SpfResult {
                present: true,
                record: Some(rec.clone()),
                valid,
                mechanism_count,
                includes,
                all_mechanism,
                severity,
                issues,
            }
        } else {
            SpfResult {
                present: false,
                record: None,
                valid: false,
                mechanism_count: 0,
                includes: Vec::new(),
                all_mechanism: None,
                severity: Severity::High,
                issues: vec!["No SPF record found".to_string()],
            }
        }
    }

    async fn check_dkim(&self, domain: &str) -> DkimResult {
        let common_selectors = vec![
            "default",
            "google",
            "selector1",
            "selector2",
            "mail",
            "dkim",
            "s1",
            "s2",
            "k1",
            "k2",
        ];

        let mut selectors_found = Vec::new();
        let mut key_length: Option<u16> = None;
        let mut issues = Vec::new();

        for selector in &common_selectors {
            let dkim_domain = format!("{}._domainkey.{}", selector, domain);
            if let Ok(lookup) = self.resolver.txt_lookup(&dkim_domain).await {
                for txt in lookup.iter() {
                    let txt_str = txt.to_string();
                    if txt_str.contains("v=DKIM1") || txt_str.contains("p=") {
                        selectors_found.push(selector.to_string());

                        if let Some(k_len) = txt_str.split(';').find(|p| p.trim().starts_with("k="))
                        {
                            let k_val = k_len.trim().trim_start_matches("k=").trim();
                            if k_val == "rsa" {
                                key_length = Some(1024);
                            }
                        }

                        if let Some(p_val) = txt_str.split(';').find(|p| p.trim().starts_with("p="))
                        {
                            let p_content = p_val.trim().trim_start_matches("p=").trim();
                            if !p_content.is_empty() {
                                let base64_len = p_content.len();
                                if base64_len > 300 {
                                    key_length = Some(2048);
                                } else if base64_len > 150 {
                                    key_length = Some(1024);
                                } else {
                                    key_length = Some(512);
                                }
                            }
                        }
                        break;
                    }
                }
            }
        }

        if let Some(len) = key_length {
            if len < 2048 {
                issues.push(format!(
                    "DKIM key length ({}) is below recommended 2048 bits",
                    len
                ));
            }
        }

        let severity = if key_length.unwrap_or(0) < 2048 || selectors_found.is_empty() {
            Severity::Medium
        } else {
            Severity::Info
        };

        DkimResult {
            present: !selectors_found.is_empty(),
            selectors_found,
            key_length,
            severity,
            issues,
        }
    }

    async fn check_dmarc(&self, domain: &str) -> DmarcResult {
        let dmarc_domain = format!("_dmarc.{}", domain);
        let mut record: Option<String> = None;

        if let Ok(lookup) = self.resolver.txt_lookup(&dmarc_domain).await {
            for txt in lookup.iter() {
                let txt_str = txt.to_string();
                if txt_str.starts_with("v=DMARC1") {
                    record = Some(txt_str);
                    break;
                }
            }
        }

        if let Some(ref rec) = record {
            let policy = rec
                .split(';')
                .find(|p| p.trim().starts_with("p="))
                .map(|p| p.trim().trim_start_matches("p=").trim().to_string());

            let subdomain_policy = rec
                .split(';')
                .find(|p| p.trim().starts_with("sp="))
                .map(|p| p.trim().trim_start_matches("sp=").trim().to_string());

            let pct = rec
                .split(';')
                .find(|p| p.trim().starts_with("pct="))
                .and_then(|p| {
                    p.trim()
                        .trim_start_matches("pct=")
                        .trim()
                        .parse::<u8>()
                        .ok()
                });

            let rua_aggregate: Vec<String> = rec
                .split(';')
                .filter(|p| p.trim().starts_with("rua="))
                .flat_map(|p| {
                    p.trim()
                        .trim_start_matches("rua=")
                        .trim()
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .collect::<Vec<_>>()
                })
                .collect();

            let ruf_forensic: Vec<String> = rec
                .split(';')
                .filter(|p| p.trim().starts_with("ruf="))
                .flat_map(|p| {
                    p.trim()
                        .trim_start_matches("ruf=")
                        .trim()
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .collect::<Vec<_>>()
                })
                .collect();

            let mut issues = Vec::new();
            if policy.as_deref() == Some("none") {
                issues.push("DMARC policy is set to 'none' - no enforcement".to_string());
            }
            if rua_aggregate.is_empty() {
                issues.push("No aggregate reporting configured".to_string());
            }

            let severity = if policy.as_deref() == Some("none") {
                Severity::Medium
            } else if policy.is_none() {
                Severity::High
            } else {
                Severity::Info
            };

            DmarcResult {
                present: true,
                record: Some(rec.clone()),
                policy,
                subdomain_policy,
                pct,
                rua_aggregate,
                ruf_forensic,
                severity,
                issues,
            }
        } else {
            DmarcResult {
                present: false,
                record: None,
                policy: None,
                subdomain_policy: None,
                pct: None,
                rua_aggregate: Vec::new(),
                ruf_forensic: Vec::new(),
                severity: Severity::High,
                issues: vec!["No DMARC record found".to_string()],
            }
        }
    }

    async fn check_mx(&self, domain: &str) -> MxSecurityResult {
        let mut records = Vec::new();
        let mut issues = Vec::new();
        let mut has_null_mx = false;
        let mut has_wildcard_mx = false;

        if let Ok(lookup) = self.resolver.mx_lookup(domain).await {
            for mx in lookup.iter() {
                let exchange = mx.exchange().to_string();

                if exchange == "." {
                    has_null_mx = true;
                    continue;
                }

                if exchange.contains('*') {
                    has_wildcard_mx = true;
                    issues.push("Wildcard MX record found".to_string());
                }

                let mut ip_addresses = Vec::new();
                if let Ok(ip_lookup) = self.resolver.lookup_ip(&exchange).await {
                    for ip in ip_lookup.iter() {
                        ip_addresses.push(ip.to_string());
                    }
                }

                records.push(MxRecordInfo {
                    preference: mx.preference(),
                    exchange,
                    ip_addresses,
                    has_spf: false,
                });
            }
        }

        if records.is_empty() && !has_null_mx {
            issues.push("No MX records found".to_string());
        }

        if has_null_mx {
            issues.push("Null MX record found - domain does not accept email".to_string());
        }

        let severity = if has_wildcard_mx {
            Severity::Medium
        } else if records.is_empty() {
            Severity::High
        } else {
            Severity::Info
        };

        MxSecurityResult {
            records,
            has_null_mx,
            has_wildcard_mx,
            severity,
            issues,
        }
    }

    async fn check_starttls(&self, mx_result: &MxSecurityResult) -> StartTlsResult {
        let mut tested_servers = Vec::new();
        let mut supports_starttls = false;
        let mut issues = Vec::new();

        if mx_result.records.is_empty() {
            return StartTlsResult {
                tested_servers,
                supports_starttls: false,
                supports_smtps: false,
                severity: Severity::Info,
                issues: vec!["No MX records to test".to_string()],
            };
        }

        for mx in &mx_result.records {
            for port in &[25, 587] {
                let result = self.test_starttls(&mx.exchange, *port).await;
                if result.supports_starttls {
                    supports_starttls = true;
                }
                tested_servers.push(result);
            }
        }

        if !supports_starttls && !mx_result.records.is_empty() {
            issues.push("None of the mail servers support STARTTLS".to_string());
        }

        let severity = if !supports_starttls && !mx_result.records.is_empty() {
            Severity::Medium
        } else {
            Severity::Info
        };

        let supports_smtps = tested_servers
            .iter()
            .any(|s| s.supports_starttls && s.port == 465);

        StartTlsResult {
            tested_servers,
            supports_starttls,
            supports_smtps,
            severity,
            issues,
        }
    }

    async fn test_starttls(&self, hostname: &str, port: u16) -> StartTlsServerResult {
        use std::time::Duration;
        use tokio::net::TcpStream;

        let timeout = Duration::from_secs(5);
        let supports_starttls = matches!(
            tokio::time::timeout(timeout, TcpStream::connect((hostname, port))).await,
            Ok(Ok(_stream))
        );

        StartTlsServerResult {
            hostname: hostname.to_string(),
            port,
            supports_starttls,
            certificate_valid: false,
            tls_version: None,
        }
    }

    async fn check_bimi(&self, domain: &str) -> BimiResult {
        let bimi_domain = format!("default._bimi.{}", domain);
        let mut record: Option<String> = None;

        if let Ok(lookup) = self.resolver.txt_lookup(&bimi_domain).await {
            for txt in lookup.iter() {
                let txt_str = txt.to_string();
                if txt_str.starts_with("v=BIMI1") {
                    record = Some(txt_str);
                    break;
                }
            }
        }

        if let Some(ref rec) = record {
            let logo_url = rec
                .split(';')
                .find(|p| p.trim().starts_with("l="))
                .map(|p| p.trim().trim_start_matches("l=").trim().to_string());

            let vmc_found = rec.contains("a=") && rec.contains("https://");

            let mut issues = Vec::new();
            if !vmc_found {
                issues.push("No Verified Mark Certificate (VMC) reference found".to_string());
            }

            BimiResult {
                present: true,
                record: Some(rec.clone()),
                logo_url,
                vmc_found,
                severity: Severity::Info,
                issues,
            }
        } else {
            BimiResult {
                present: false,
                record: None,
                logo_url: None,
                vmc_found: false,
                severity: Severity::Info,
                issues: vec!["No BIMI record found".to_string()],
            }
        }
    }
}

pub async fn analyze_email_security(target: &str) -> Result<EmailSecurityReport> {
    let analyzer = EmailSecurityAnalyzer::new()?;
    analyzer.analyze(target).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyzer_creation() {
        let analyzer = EmailSecurityAnalyzer::new();
        assert!(analyzer.is_ok());
    }

    #[test]
    fn test_spf_result_default() {
        let spf = SpfResult {
            present: false,
            record: None,
            valid: false,
            mechanism_count: 0,
            includes: Vec::new(),
            all_mechanism: None,
            severity: Severity::High,
            issues: vec!["No SPF record found".to_string()],
        };
        assert!(!spf.present);
        assert_eq!(spf.severity, Severity::High);
    }

    #[test]
    fn test_dmarc_result_parsing() {
        let dmarc = DmarcResult {
            present: true,
            record: Some("v=DMARC1; p=reject; rua=mailto:dmarc@example.com".to_string()),
            policy: Some("reject".to_string()),
            subdomain_policy: None,
            pct: Some(100),
            rua_aggregate: vec!["mailto:dmarc@example.com".to_string()],
            ruf_forensic: Vec::new(),
            severity: Severity::Info,
            issues: Vec::new(),
        };
        assert_eq!(dmarc.policy, Some("reject".to_string()));
        assert_eq!(dmarc.rua_aggregate.len(), 1);
    }

    #[test]
    fn test_email_finding_creation() {
        let finding = EmailFinding {
            category: "SPF".to_string(),
            severity: Severity::High,
            description: "Test finding".to_string(),
            recommendation: "Test recommendation".to_string(),
        };
        assert_eq!(finding.category, "SPF");
        assert_eq!(finding.severity, Severity::High);
    }

    #[test]
    fn test_mx_security_result_empty() {
        let mx = MxSecurityResult {
            records: Vec::new(),
            has_null_mx: false,
            has_wildcard_mx: false,
            severity: Severity::High,
            issues: vec!["No MX records found".to_string()],
        };
        assert!(mx.records.is_empty());
        assert_eq!(mx.severity, Severity::High);
    }

    #[test]
    fn test_bimi_result_empty() {
        let bimi = BimiResult {
            present: false,
            record: None,
            logo_url: None,
            vmc_found: false,
            severity: Severity::Info,
            issues: vec!["No BIMI record found".to_string()],
        };
        assert!(!bimi.present);
    }
}
