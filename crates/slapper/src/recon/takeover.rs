use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::LazyLock;

use crate::utils::create_insecure_http_client;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TakeoverTarget {
    pub subdomain: String,
    pub cname: Option<String>,
    pub ns: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TakeoverResult {
    pub target: TakeoverTarget,
    pub status: TakeoverStatus,
    pub service: Option<String>,
    pub evidence: String,
    pub severity: Severity,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TakeoverStatus {
    Vulnerable,
    Safe,
    Unknown,
}

pub use crate::types::Severity;

struct TakeoverFingerprint {
    service: &'static str,
    cnames: &'static [&'static str],
    nxdomain_cnames: &'static [&'static str],
    http_indicators: &'static [&'static str],
    severity: Severity,
}

static FINGERPRINTS: LazyLock<Vec<TakeoverFingerprint>> = LazyLock::new(|| {
    vec![
        TakeoverFingerprint {
            service: "AWS S3",
            cnames: &["amazonaws.com"],
            nxdomain_cnames: &["s3-website", ".s3.amazonaws.com"],
            http_indicators: &[
                "The specified bucket does not exist",
                "NoSuchBucket",
                "All access to this object has been disabled",
            ],
            severity: Severity::High,
        },
        TakeoverFingerprint {
            service: "GitHub Pages",
            cnames: &["github.io"],
            nxdomain_cnames: &["github.io"],
            http_indicators: &["There isn't a GitHub Pages site here", "404 There is no site configured here"],
            severity: Severity::Medium,
        },
        TakeoverFingerprint {
            service: "Heroku",
            cnames: &["herokuapp.com"],
            nxdomain_cnames: &["herokuapp.com"],
            http_indicators: &["No such app", "herokucdn.com/error-pages/no-such-app.html"],
            severity: Severity::High,
        },
        TakeoverFingerprint {
            service: "Azure Web Apps",
            cnames: &["azurewebsites.net", "cloudapp.net", "cloudapp.azure.com"],
            nxdomain_cnames: &["azurewebsites.net", "cloudapp.net"],
            http_indicators: &["404 Web Site not found", "does not exist or have custom domain configured"],
            severity: Severity::High,
        },
        TakeoverFingerprint {
            service: "GCP Storage",
            cnames: &["c.storage.googleapis.com", "storage.googleapis.com"],
            nxdomain_cnames: &["storage.googleapis.com"],
            http_indicators: &[
                "BucketNotFound",
                "NoSuchBucket",
                "The specified bucket does not exist",
            ],
            severity: Severity::High,
        },
        TakeoverFingerprint {
            service: "Shopify",
            cnames: &["myshopify.com", "shops.myshopify.com"],
            nxdomain_cnames: &["myshopify.com"],
            http_indicators: &["Sorry, this shop is currently unavailable", "Only one step left!"],
            severity: Severity::Medium,
        },
        TakeoverFingerprint {
            service: "Tumblr",
            cnames: &["domains.tumblr.com", "tumblr.com"],
            nxdomain_cnames: &["tumblr.com"],
            http_indicators: &["There's nothing here", "Whatever you were looking for doesn't currently exist"],
            severity: Severity::Medium,
        },
        TakeoverFingerprint {
            service: "WordPress.com",
            cnames: &["wordpress.com", "wp.com"],
            nxdomain_cnames: &["wordpress.com"],
            http_indicators: &["Do you want to register", "wordpress.com"],
            severity: Severity::Medium,
        },
        TakeoverFingerprint {
            service: "Pantheon",
            cnames: &["pantheonsite.io"],
            nxdomain_cnames: &["pantheonsite.io"],
            http_indicators: &["The gods are wise", "404 Not Found"],
            severity: Severity::Medium,
        },
        TakeoverFingerprint {
            service: "Feedpress",
            cnames: &["redirect.feedpress.me"],
            nxdomain_cnames: &["feedpress.me"],
            http_indicators: &["The feed has not been found"],
            severity: Severity::Medium,
        },
        TakeoverFingerprint {
            service: "Surge.sh",
            cnames: &["surge.sh"],
            nxdomain_cnames: &["surge.sh"],
            http_indicators: &["project not found", "surge.sh"],
            severity: Severity::Medium,
        },
        TakeoverFingerprint {
            service: "Helpjuice",
            cnames: &["helpjuice.com"],
            nxdomain_cnames: &["helpjuice.com"],
            http_indicators: &["We could not find what you're looking for"],
            severity: Severity::Medium,
        },
        TakeoverFingerprint {
            service: "Help Scout",
            cnames: &["helpscoutdocs.com"],
            nxdomain_cnames: &["helpscoutdocs.com"],
            http_indicators: &["No settings were found for this company"],
            severity: Severity::Medium,
        },
        TakeoverFingerprint {
            service: "Cargo",
            cnames: &["cargo.site", "cargocollective.com"],
            nxdomain_cnames: &["cargocollective.com"],
            http_indicators: &["If you're moving your domain away from Cargo you must make this configuration through your Cargo admin settings"],
            severity: Severity::Low,
        },
        TakeoverFingerprint {
            service: "Statamic",
            cnames: &["statamic.com"],
            nxdomain_cnames: &["statamic.com"],
            http_indicators: &["Statamic", "Site Not Found"],
            severity: Severity::Medium,
        },
        TakeoverFingerprint {
            service: "Canny",
            cnames: &["canny.io"],
            nxdomain_cnames: &["canny.io"],
            http_indicators: &["Company Not Found"],
            severity: Severity::Medium,
        },
        TakeoverFingerprint {
            service: "Statuspage",
            cnames: &["statuspage.io"],
            nxdomain_cnames: &["statuspage.io"],
            http_indicators: &["Better Status Communication", "You are being redirected"],
            severity: Severity::Medium,
        },
        TakeoverFingerprint {
            service: "Readme.io",
            cnames: &["readme.io"],
            nxdomain_cnames: &["readme.io"],
            http_indicators: &["Project Not Found"],
            severity: Severity::Medium,
        },
        TakeoverFingerprint {
            service: "Pingdom",
            cnames: &["stats.pingdom.com"],
            nxdomain_cnames: &["pingdom.com"],
            http_indicators: &["pingdom"],
            severity: Severity::Low,
        },
        TakeoverFingerprint {
            service: "Tilda",
            cnames: &["tilda.ws", "tilda.com"],
            nxdomain_cnames: &["tilda.ws"],
            http_indicators: &["Please go to the site settings and specify the domain name", "Domain name is not specified"],
            severity: Severity::Medium,
        },
        TakeoverFingerprint {
            service: "UptimeRobot",
            cnames: &["stats.uptimerobot.com"],
            nxdomain_cnames: &["uptimerobot.com"],
            http_indicators: &["This public status page does not seem to exist anymore"],
            severity: Severity::Low,
        },
        TakeoverFingerprint {
            service: "GetResponse",
            cnames: &["gr8.com"],
            nxdomain_cnames: &["gr8.com"],
            http_indicators: &["With GetResponse you can create effective landing pages"],
            severity: Severity::Medium,
        },
        TakeoverFingerprint {
            service: "Vend",
            cnames: &["vendecommerce.com"],
            nxdomain_cnames: &["vendecommerce.com"],
            http_indicators: &["Looks like you've traveled too far into cyberspace"],
            severity: Severity::Medium,
        },
        TakeoverFingerprint {
            service: "Jetbrains",
            cnames: &["myjetbrains.com"],
            nxdomain_cnames: &["myjetbrains.com"],
            http_indicators: &["is not a registered InCloud YouTrack", "is not a registered InCloud"],
            severity: Severity::Medium,
        },
        TakeoverFingerprint {
            service: "AgileCRM",
            cnames: &["agilecrm.com"],
            nxdomain_cnames: &["agilecrm.com"],
            http_indicators: &["Sorry, this account does not exist"],
            severity: Severity::Medium,
        },
        TakeoverFingerprint {
            service: "Webflow",
            cnames: &["proxy.webflow.com", "proxy-ssl.webflow.com"],
            nxdomain_cnames: &["webflow.com"],
            http_indicators: &["404 - Page not found", "The page you are looking for doesn't exist"],
            severity: Severity::Medium,
        },
        TakeoverFingerprint {
            service: "Kajabi",
            cnames: &["endpoint.mykajabi.com"],
            nxdomain_cnames: &["mykajabi.com"],
            http_indicators: &["The page you were looking for doesn't exist"],
            severity: Severity::Medium,
        },
        TakeoverFingerprint {
            service: "Thinkific",
            cnames: &["thinkific.com"],
            nxdomain_cnames: &["thinkific.com"],
            http_indicators: &["Cloudflare", "ray ID", "thinkific"],
            severity: Severity::Low,
        },
        TakeoverFingerprint {
            service: "LaunchRock",
            cnames: &["launchrock.com"],
            nxdomain_cnames: &["launchrock.com"],
            http_indicators: &["It looks like you may have taken a wrong turn somewhere"],
            severity: Severity::Medium,
        },
        TakeoverFingerprint {
            service: "Intercom",
            cnames: &["custom.intercom.help"],
            nxdomain_cnames: &["intercom.help"],
            http_indicators: &["This page does not exist", "Intercom"],
            severity: Severity::Medium,
        },
    ]
});

pub struct TakeoverDetector {
    client: reqwest::Client,
}

impl TakeoverDetector {
    pub fn new(timeout_secs: u64) -> Result<Self> {
        let client = create_insecure_http_client(timeout_secs)?;
        Ok(Self { client })
    }

    pub async fn detect(&self, targets: &[TakeoverTarget]) -> Vec<TakeoverResult> {
        let mut results = Vec::new();
        for target in targets {
            if let Ok(result) = self.check_target(target).await {
                results.push(result);
            }
        }
        results
    }

    async fn check_target(&self, target: &TakeoverTarget) -> Result<TakeoverResult> {
        let subdomain = &target.subdomain;

        let cname = target.cname.clone();
        let ns = target.ns.clone();

        if let Some(ref cname_val) = cname {
            for fingerprint in FINGERPRINTS.iter() {
                let cname_matches = fingerprint.cnames.iter().any(|c| cname_val.contains(c));
                let nxdomain_matches = fingerprint.nxdomain_cnames.iter().any(|c| cname_val.contains(c));

                if cname_matches || nxdomain_matches {
                    let http_result = self.check_http_indicators(subdomain, fingerprint).await;

                    match http_result {
                        HttpCheckResult::Vulnerable(evidence) => {
                            return Ok(TakeoverResult {
                                target: target.clone(),
                                status: TakeoverStatus::Vulnerable,
                                service: Some(fingerprint.service.to_string()),
                                evidence,
                                severity: fingerprint.severity,
                            });
                        }
                        HttpCheckResult::Safe => {
                            return Ok(TakeoverResult {
                                target: target.clone(),
                                status: TakeoverStatus::Safe,
                                service: Some(fingerprint.service.to_string()),
                                evidence: "CNAME points to valid service".to_string(),
                                severity: Severity::Info,
                            });
                        }
                        HttpCheckResult::Unknown(err) => {
                            return Ok(TakeoverResult {
                                target: target.clone(),
                                status: TakeoverStatus::Unknown,
                                service: Some(fingerprint.service.to_string()),
                                evidence: format!("HTTP check failed: {}", err),
                                severity: Severity::Info,
                            });
                        }
                    }
                }
            }
        }

        if let Some(ref ns_val) = ns {
            for fingerprint in FINGERPRINTS.iter() {
                let ns_matches = fingerprint.cnames.iter().any(|c| ns_val.contains(c));
                if ns_matches {
                    let http_result = self.check_http_indicators(subdomain, fingerprint).await;

                    match http_result {
                        HttpCheckResult::Vulnerable(evidence) => {
                            return Ok(TakeoverResult {
                                target: target.clone(),
                                status: TakeoverStatus::Vulnerable,
                                service: Some(fingerprint.service.to_string()),
                                evidence,
                                severity: fingerprint.severity,
                            });
                        }
                        HttpCheckResult::Safe => {
                            return Ok(TakeoverResult {
                                target: target.clone(),
                                status: TakeoverStatus::Safe,
                                service: Some(fingerprint.service.to_string()),
                                evidence: "NS points to valid service".to_string(),
                                severity: Severity::Info,
                            });
                        }
                        HttpCheckResult::Unknown(err) => {
                            return Ok(TakeoverResult {
                                target: target.clone(),
                                status: TakeoverStatus::Unknown,
                                service: Some(fingerprint.service.to_string()),
                                evidence: format!("HTTP check failed: {}", err),
                                severity: Severity::Info,
                            });
                        }
                    }
                }
            }
        }

        Ok(TakeoverResult {
            target: target.clone(),
            status: TakeoverStatus::Safe,
            service: None,
            evidence: "No vulnerable CNAME/NS records detected".to_string(),
            severity: Severity::Info,
        })
    }

    async fn check_http_indicators(&self, subdomain: &str, fingerprint: &TakeoverFingerprint) -> HttpCheckResult {
        let url = format!("https://{}", subdomain);

        match self.client.get(&url).send().await {
            Ok(response) => {
                if let Ok(body) = response.text().await {
                    for indicator in fingerprint.http_indicators {
                        if body.contains(indicator) {
                            return HttpCheckResult::Vulnerable(format!(
                                "Found indicator '{}' in response from {}",
                                indicator, subdomain
                            ));
                        }
                    }
                    HttpCheckResult::Safe
                } else {
                    HttpCheckResult::Unknown("Failed to read response body".to_string())
                }
            }
            Err(e) => {
                let err_str = e.to_string();
                if err_str.contains("dns error") || err_str.contains("resolve") || err_str.contains("NXDOMAIN") {
                    for cname_pattern in fingerprint.nxdomain_cnames {
                        if let Some(cname) = fingerprint.cnames.first() {
                            if err_str.contains(*cname) || cname_pattern.contains(*cname) {
                                return HttpCheckResult::Vulnerable(format!(
                                    "DNS resolution failed for {} pointing to {}",
                                    subdomain, cname
                                ));
                            }
                        }
                    }
                }
                HttpCheckResult::Unknown(err_str)
            }
        }
    }

    pub async fn detect_single(&self, subdomain: &str, cname: Option<String>, ns: Option<String>) -> Result<TakeoverResult> {
        let target = TakeoverTarget {
            subdomain: subdomain.to_string(),
            cname,
            ns,
        };
        self.check_target(&target).await
    }
}

enum HttpCheckResult {
    Vulnerable(String),
    Safe,
    Unknown(String),
}

pub async fn detect_takeovers(subdomains: &[String], dns_records: Option<&crate::recon::dns_records::DnsRecords>, timeout_secs: u64) -> Result<Vec<TakeoverResult>> {
    let detector = TakeoverDetector::new(timeout_secs)?;

    let mut cname_map: HashMap<String, String> = HashMap::new();
    let mut ns_map: HashMap<String, String> = HashMap::new();

    if let Some(records) = dns_records {
        for cname in &records.cname {
            let parts: Vec<&str> = cname.splitn(2, ' ').collect();
            if parts.len() == 2 {
                cname_map.insert(parts[0].to_string(), parts[1].to_string());
            }
        }
        for ns in &records.ns {
            ns_map.insert(ns.to_string(), ns.to_string());
        }
    }

    let mut targets = Vec::new();
    for subdomain in subdomains {
        let cname = cname_map.get(subdomain).cloned();
        let ns = ns_map.get(subdomain).cloned();
        targets.push(TakeoverTarget {
            subdomain: subdomain.clone(),
            cname,
            ns,
        });
    }

    Ok(detector.detect(&targets).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fingerprints_loaded() {
        assert!(!FINGERPRINTS.is_empty());
        assert!(FINGERPRINTS.len() >= 20);
    }

    #[test]
    fn test_aws_s3_fingerprint_exists() {
        let s3 = FINGERPRINTS.iter().find(|f| f.service == "AWS S3");
        assert!(s3.is_some());
        let s3 = s3.unwrap();
        assert!(s3.cnames.iter().any(|c| c.contains("amazonaws")));
        assert!(s3.http_indicators.iter().any(|i| i.contains("NoSuchBucket")));
    }

    #[test]
    fn test_github_pages_fingerprint_exists() {
        let gh = FINGERPRINTS.iter().find(|f| f.service == "GitHub Pages");
        assert!(gh.is_some());
        let gh = gh.unwrap();
        assert!(gh.cnames.iter().any(|c| c.contains("github.io")));
    }

    #[test]
    fn test_takeover_detector_creation() {
        let detector = TakeoverDetector::new(10);
        assert!(detector.is_ok());
    }

    #[test]
    fn test_takeover_target_creation() {
        let target = TakeoverTarget {
            subdomain: "test.example.com".to_string(),
            cname: Some("nonexistent.s3.amazonaws.com".to_string()),
            ns: None,
        };
        assert_eq!(target.subdomain, "test.example.com");
        assert!(target.cname.is_some());
    }

    #[test]
    fn test_takeover_status_serialization() {
        let status = TakeoverStatus::Vulnerable;
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("Vulnerable"));
    }

    #[test]
    fn test_takeover_result_creation() {
        let target = TakeoverTarget {
            subdomain: "old.example.com".to_string(),
            cname: Some("old.github.io".to_string()),
            ns: None,
        };
        let result = TakeoverResult {
            target,
            status: TakeoverStatus::Vulnerable,
            service: Some("GitHub Pages".to_string()),
            evidence: "Test evidence".to_string(),
            severity: Severity::Medium,
        };
        assert_eq!(result.status, TakeoverStatus::Vulnerable);
        assert_eq!(result.service.unwrap(), "GitHub Pages");
    }
}
