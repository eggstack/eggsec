
#![allow(dead_code)]

use crate::error::Result;
use hickory_resolver::config::{ResolverConfig, ResolverOpts};
use hickory_resolver::TokioAsyncResolver;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

use crate::utils::create_http_client;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SubdomainResult {
    pub domain: String,
    pub subdomains: Vec<SubdomainInfo>,
    pub sources: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubdomainInfo {
    pub subdomain: String,
    pub ip_addresses: Vec<String>,
    pub has_mx: bool,
    pub has_cname: bool,
    pub has_txt: bool,
}

fn create_resolver_opts() -> ResolverOpts {
    let mut opts = ResolverOpts::default();
    opts.timeout = Duration::from_secs(10);
    opts.attempts = 2;
    opts
}

pub struct SubdomainEnumerator {
    client: reqwest::Client,
    resolver: TokioAsyncResolver,
    concurrency: usize,
}

impl SubdomainEnumerator {
    pub fn new(concurrency: usize) -> Result<Self> {
        let client = create_http_client(10)?;

        let resolver = TokioAsyncResolver::tokio(ResolverConfig::default(), create_resolver_opts());

        Ok(Self {
            client,
            resolver,
            concurrency,
        })
    }

    pub async fn enumerate(&self, domain: &str) -> Result<SubdomainResult> {
        let mut subdomains = HashSet::new();
        let mut sources = Vec::new();

        if let Ok(crtsh_subdomains) = self.query_crtsh(domain).await {
            for sub in crtsh_subdomains {
                subdomains.insert(sub);
            }
            sources.push("crt.sh".to_string());
        }

        if let Ok(alexa_subdomains) = self.query_alexa(domain).await {
            for sub in alexa_subdomains {
                subdomains.insert(sub);
            }
            sources.push("alexa".to_string());
        }

        if let Ok(threatminer_subdomains) = self.query_threatminer(domain).await {
            for sub in threatminer_subdomains {
                subdomains.insert(sub);
            }
            sources.push("threatminer".to_string());
        }

        let subdomain_infos = self.verify_subdomains(domain, &subdomains).await;

        Ok(SubdomainResult {
            domain: domain.to_string(),
            subdomains: subdomain_infos,
            sources,
        })
    }

    async fn query_crtsh(&self, domain: &str) -> Result<HashSet<String>> {
        let url = format!("https://crt.sh/?q={}&output=json", domain);
        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Ok(HashSet::new());
        }

        let crt_entries: Vec<CrtShEntry> = response.json().await.unwrap_or_default();
        let mut subdomains = HashSet::new();

        for entry in crt_entries {
            if let Some(name_value) = entry.name_value {
                for name in name_value.split('\n') {
                    let name = name.trim();
                    if name.ends_with(&format!(".{}", domain)) || name == domain {
                        let subdomain = if let Some(stripped) = name.strip_prefix("www.") {
                            stripped.to_string()
                        } else {
                            name.to_string()
                        };
                        if subdomain.ends_with(&format!(".{}", domain)) {
                            subdomains.insert(subdomain);
                        }
                    }
                }
            }
        }

        Ok(subdomains)
    }

    async fn query_alexa(&self, _domain: &str) -> Result<HashSet<String>> {
        Ok(HashSet::new())
    }

    async fn query_threatminer(&self, domain: &str) -> Result<HashSet<String>> {
        let url = format!(
            "https://api.threatminer.org/v2/domain.php?q={}&rt=6",
            domain
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Ok(HashSet::new());
        }

        let threatminer_resp: ThreatMinerResponse = response.json().await.unwrap_or_default();
        Ok(threatminer_resp.results.into_iter().collect())
    }

    async fn verify_subdomains(
        &self,
        domain: &str,
        subdomains: &HashSet<String>,
    ) -> Vec<SubdomainInfo> {
        let semaphore = Arc::new(Semaphore::new(self.concurrency));
        let mut handles = Vec::new();

        for subdomain in subdomains {
            let subdomain = subdomain.clone();
            let domain = domain.to_string();
            let semaphore = Arc::clone(&semaphore);
            let resolver =
                TokioAsyncResolver::tokio(ResolverConfig::default(), create_resolver_opts());

            let handle = tokio::spawn(async move {
                let _permit = semaphore.acquire().await.ok();
                let mut info = SubdomainInfo {
                    subdomain: subdomain.clone(),
                    ip_addresses: Vec::new(),
                    has_mx: false,
                    has_cname: false,
                    has_txt: false,
                };

                let fqdn = if subdomain == domain {
                    subdomain.clone()
                } else {
                    format!("{}.{}", subdomain, domain)
                };

                if let Ok(lookup) = resolver.lookup_ip(&fqdn).await {
                    for ip in lookup.iter() {
                        info.ip_addresses.push(ip.to_string());
                    }
                }

                if let Ok(mx_lookup) = resolver.mx_lookup(&fqdn).await {
                    info.has_mx = mx_lookup.iter().count() > 0;
                }

                if let Ok(txt_lookup) = resolver.txt_lookup(&fqdn).await {
                    info.has_txt = txt_lookup.iter().count() > 0;
                }

                info
            });

            handles.push(handle);
        }

        let mut results = Vec::new();
        for handle in handles {
            if let Ok(info) = handle.await {
                if !info.ip_addresses.is_empty() || info.has_mx || info.has_txt {
                    results.push(info);
                }
            }
        }

        results
    }

    pub async fn bruteforce(&self, domain: &str, wordlist: &[String]) -> Result<SubdomainResult> {
        let semaphore = Arc::new(Semaphore::new(self.concurrency));
        let mut handles = Vec::new();

        for word in wordlist {
            let subdomain = format!("{}.{}", word, domain);
            let semaphore = Arc::clone(&semaphore);
            let resolver =
                TokioAsyncResolver::tokio(ResolverConfig::default(), create_resolver_opts());

            let handle = tokio::spawn(async move {
                let _permit = semaphore.acquire().await.ok();

                if let Ok(lookup) = resolver.lookup_ip(&subdomain).await {
                    let ips: Vec<String> = lookup.iter().map(|ip| ip.to_string()).collect();
                    if !ips.is_empty() {
                        return Some(SubdomainInfo {
                            subdomain,
                            ip_addresses: ips,
                            has_mx: false,
                            has_cname: false,
                            has_txt: false,
                        });
                    }
                }
                None
            });

            handles.push(handle);
        }

        let mut subdomains = Vec::new();
        for handle in handles {
            if let Ok(Some(info)) = handle.await {
                subdomains.push(info);
            }
        }

        Ok(SubdomainResult {
            domain: domain.to_string(),
            subdomains,
            sources: vec!["dns-bruteforce".to_string()],
        })
    }
}

#[derive(Debug, Deserialize)]
struct CrtShEntry {
    #[serde(rename = "name_value")]
    name_value: Option<String>,
}

#[derive(Debug, Deserialize)]
#[derive(Default)]
struct ThreatMinerResponse {
    #[serde(default)]
    results: Vec<String>,
}


pub async fn enumerate_subdomains(domain: &str, concurrency: usize) -> Result<SubdomainResult> {
    let enumerator = SubdomainEnumerator::new(concurrency)?;
    enumerator.enumerate(domain).await
}

pub async fn bruteforce_subdomains(
    domain: &str,
    wordlist: &[String],
    concurrency: usize,
) -> Result<SubdomainResult> {
    let enumerator = SubdomainEnumerator::new(concurrency)?;
    enumerator.bruteforce(domain, wordlist).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subdomain_result_serialization() {
        let result = SubdomainResult {
            domain: "example.com".to_string(),
            subdomains: vec![
                SubdomainInfo {
                    subdomain: "www".to_string(),
                    ip_addresses: vec!["93.184.216.34".to_string()],
                    has_mx: false,
                    has_cname: false,
                    has_txt: true,
                },
            ],
            sources: vec!["crt.sh".to_string()],
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("example.com"));
        assert!(json.contains("crt.sh"));
        let decoded: SubdomainResult = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.domain, "example.com");
        assert_eq!(decoded.subdomains.len(), 1);
    }

    #[test]
    fn test_subdomain_info_serialization() {
        let info = SubdomainInfo {
            subdomain: "api".to_string(),
            ip_addresses: vec!["1.2.3.4".to_string(), "5.6.7.8".to_string()],
            has_mx: true,
            has_cname: true,
            has_txt: false,
        };
        let json = serde_json::to_string(&info).unwrap();
        let decoded: SubdomainInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.subdomain, "api");
        assert_eq!(decoded.ip_addresses.len(), 2);
        assert!(decoded.has_mx);
    }

    #[test]
    fn test_subdomain_result_default() {
        let result = SubdomainResult::default();
        assert!(result.domain.is_empty());
        assert!(result.subdomains.is_empty());
        assert!(result.sources.is_empty());
    }

    #[test]
    fn test_subdomain_enumerator_new() {
        let result = SubdomainEnumerator::new(10);
        assert!(result.is_ok());
        let result = SubdomainEnumerator::new(0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_subdomain_info_clone() {
        let info = SubdomainInfo {
            subdomain: "www".to_string(),
            ip_addresses: vec!["1.2.3.4".to_string()],
            has_mx: false,
            has_cname: false,
            has_txt: false,
        };
        let cloned = info.clone();
        assert_eq!(cloned.subdomain, "www");
        assert_eq!(cloned.ip_addresses.len(), 1);
    }

    #[test]
    fn test_crt_sh_entry_deserialization() {
        let entry_json = r#"{"name_value":"www.example.com\napi.example.com"}"#;
        let entry: CrtShEntry = serde_json::from_str(entry_json).unwrap();
        assert!(entry.name_value.is_some());
        let val = entry.name_value.unwrap();
        assert!(val.contains("www.example.com"));
    }

    #[test]
    fn test_crtsh_entry_empty() {
        let entry_json = r#"{"name_value":null}"#;
        let entry: CrtShEntry = serde_json::from_str(entry_json).unwrap();
        assert!(entry.name_value.is_none());
    }

    #[test]
    fn test_threatminer_response_default() {
        let resp: ThreatMinerResponse = ThreatMinerResponse::default();
        assert!(resp.results.is_empty());
    }

    #[test]
    fn test_threatminer_response_deserialization() {
        let json = r#"{"results":["sub1.example.com","sub2.example.com"]}"#;
        let resp: ThreatMinerResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.results.len(), 2);
        assert_eq!(resp.results[0], "sub1.example.com");
    }

    #[test]
    fn test_subdomain_result_sources() {
        let result = SubdomainResult {
            domain: "example.com".to_string(),
            subdomains: vec![],
            sources: vec!["crt.sh".to_string(), "alexa".to_string(), "threatminer".to_string()],
        };
        assert_eq!(result.sources.len(), 3);
    }
}
