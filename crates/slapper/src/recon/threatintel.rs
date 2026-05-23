#![allow(dead_code)]

use crate::error::{Result, SlapperError};
use crate::types::SensitiveString;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use urlencoding;

use crate::utils::create_http_client;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThreatIntel {
    pub target: String,
    pub ip_reputation: Option<IpReputation>,
    pub domain_reputation: Option<DomainReputation>,
    pub passive_dns: Vec<PassiveDnsRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpReputation {
    pub ip: String,
    pub score: i32,
    pub category: String,
    pub threats: Vec<String>,
    pub asn: Option<String>,
    pub isp: Option<String>,
    pub country: Option<String>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainReputation {
    pub domain: String,
    pub score: i32,
    pub category: String,
    pub threats: Vec<String>,
    pub registrar: Option<String>,
    pub created_date: Option<String>,
    pub nameservers: Vec<String>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassiveDnsRecord {
    pub record_type: String,
    pub value: String,
    pub first_seen: Option<String>,
    pub last_seen: Option<String>,
    pub source: String,
}

pub struct ThreatIntelClient {
    client: Client,
    virustotal_key: Option<SensitiveString>,
    alienvault_key: Option<SensitiveString>,
    shodan_key: Option<SensitiveString>,
    threatstream_key: Option<SensitiveString>,
}

impl ThreatIntelClient {
    pub fn new(
        virustotal_key: Option<SensitiveString>,
        alienvault_key: Option<SensitiveString>,
        shodan_key: Option<SensitiveString>,
        threatstream_key: Option<SensitiveString>,
    ) -> Result<Self> {
        let client = create_http_client(30)?;

        Ok(Self {
            client,
            virustotal_key,
            alienvault_key,
            shodan_key,
            threatstream_key,
        })
    }

    pub async fn check_ip(&self, ip: &str) -> Result<ThreatIntel> {
        let mut intel = ThreatIntel {
            target: ip.to_string(),
            ..Default::default()
        };

        if let Some(ref key) = self.virustotal_key {
            if let Ok(reputation) = self.check_virustotal_ip(ip, key.expose_secret()).await {
                intel.ip_reputation = Some(reputation);
            }
        }

        if let Some(ref key) = self.shodan_key {
            if let Ok(reputation) = self.check_shodan_ip(ip, key.expose_secret()).await {
                if intel.ip_reputation.is_none() {
                    intel.ip_reputation = Some(reputation);
                }
            }
        }

        if let Some(ref key) = self.alienvault_key {
            if let Ok(pdns) = self.check_alienvault_pdns(ip, key.expose_secret()).await {
                intel.passive_dns = pdns;
            }
        }

        Ok(intel)
    }

    pub async fn check_domain(&self, domain: &str) -> Result<ThreatIntel> {
        let mut intel = ThreatIntel {
            target: domain.to_string(),
            ..Default::default()
        };

        if let Some(ref key) = self.virustotal_key {
            if let Ok(reputation) = self
                .check_virustotal_domain(domain, key.expose_secret())
                .await
            {
                intel.domain_reputation = Some(reputation);
            }
        }

        if let Some(ref key) = self.alienvault_key {
            if let Ok(pdns) = self
                .check_alienvault_domain_pdns(domain, key.expose_secret())
                .await
            {
                intel.passive_dns = pdns;
            }
        }

        Ok(intel)
    }

    async fn check_virustotal_ip(&self, ip: &str, api_key: &str) -> Result<IpReputation> {
        let url = format!("https://www.virustotal.com/api/v3/ip_addresses/{}", ip);

        let response = self
            .client
            .get(&url)
            .header("x-apikey", api_key)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(SlapperError::Recon("VT API request failed".to_string()));
        }

        let vt_resp: serde_json::Value = response.json().await?;

        let data = vt_resp
            .get("data")
            .and_then(|d| d.get("attributes"))
            .ok_or_else(|| SlapperError::Recon("Invalid VT response".to_string()))?;

        let last_analysis = data
            .get("last_analysis_stats")
            .ok_or_else(|| SlapperError::Recon("No analysis stats".to_string()))?;

        let malicious = last_analysis
            .get("malicious")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let suspicious = last_analysis
            .get("suspicious")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let score = ((malicious * 10 + suspicious * 5) as i32).min(100);

        let mut threats = Vec::new();
        if let Some(categories) = data.get("categories").and_then(|c| c.as_object()) {
            threats.reserve(categories.len());
            for (cat, _) in categories {
                threats.push(cat.clone());
            }
        }

        Ok(IpReputation {
            ip: ip.to_string(),
            score,
            category: if score > 50 {
                "malicious"
            } else if score > 20 {
                "suspicious"
            } else {
                "clean"
            }
            .to_string(),
            threats,
            asn: data
                .get("asn")
                .and_then(|v| v.as_i64())
                .map(|n| format!("AS{}", n)),
            isp: data.get("isp").and_then(|v| v.as_str()).map(String::from),
            country: data
                .get("country")
                .and_then(|v| v.as_str())
                .map(String::from),
            source: "VirusTotal".to_string(),
        })
    }

    async fn check_virustotal_domain(
        &self,
        domain: &str,
        api_key: &str,
    ) -> Result<DomainReputation> {
        let url = format!("https://www.virustotal.com/api/v3/domains/{}", domain);

        let response = self
            .client
            .get(&url)
            .header("x-apikey", api_key)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(SlapperError::Recon("VT API request failed".to_string()));
        }

        let vt_resp: serde_json::Value = response.json().await?;

        let data = vt_resp
            .get("data")
            .and_then(|d| d.get("attributes"))
            .ok_or_else(|| SlapperError::Recon("Invalid VT response".to_string()))?;

        let last_analysis = data
            .get("last_analysis_stats")
            .ok_or_else(|| SlapperError::Recon("No analysis stats".to_string()))?;

        let malicious = last_analysis
            .get("malicious")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let suspicious = last_analysis
            .get("suspicious")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let score = ((malicious * 10 + suspicious * 5) as i32).min(100);

        let mut threats = Vec::new();
        if let Some(categories) = data.get("categories").and_then(|c| c.as_object()) {
            threats.reserve(categories.len());
            for (cat, _) in categories {
                threats.push(cat.clone());
            }
        }

        Ok(DomainReputation {
            domain: domain.to_string(),
            score,
            category: if score > 50 {
                "malicious"
            } else if score > 20 {
                "suspicious"
            } else {
                "clean"
            }
            .to_string(),
            threats,
            registrar: data
                .get("registrar")
                .and_then(|v| v.as_str())
                .map(String::from),
            created_date: data
                .get("creation_date")
                .and_then(|v| v.as_str())
                .map(String::from),
            nameservers: data
                .get("name_servers")
                .and_then(|ns| ns.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_else(|| {
                    tracing::debug!("nameservers field missing or invalid");
                    Vec::new()
                }),
            source: "VirusTotal".to_string(),
        })
    }

    async fn check_shodan_ip(&self, ip: &str, api_key: &str) -> Result<IpReputation> {
        let url = format!(
            "https://api.shodan.io/shodan/host/{}?key={}",
            ip,
            urlencoding::encode(api_key)
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(SlapperError::Recon("Shodan API request failed".to_string()));
        }

        let shodan_resp: serde_json::Value = response.json().await?;

        let score = if shodan_resp.get("vulns").is_some() {
            80
        } else if shodan_resp.get("tags").is_some() {
            40
        } else {
            0
        };

        let mut threats = Vec::new();
        if let Some(vulns) = shodan_resp.get("vulns").and_then(|v| v.as_array()) {
            threats.reserve(vulns.len().min(5));
            for vuln in vulns.iter().take(5) {
                if let Some(cve) = vuln.as_str() {
                    threats.push(cve.to_string());
                }
            }
        }

        Ok(IpReputation {
            ip: ip.to_string(),
            score,
            category: if score > 50 { "vulnerable" } else { "clean" }.to_string(),
            threats,
            asn: shodan_resp
                .get("asn")
                .and_then(|v| v.as_str())
                .map(String::from),
            isp: shodan_resp
                .get("isp")
                .and_then(|v| v.as_str())
                .map(String::from),
            country: shodan_resp
                .get("country_name")
                .and_then(|v| v.as_str())
                .map(String::from),
            source: "Shodan".to_string(),
        })
    }

    async fn check_alienvault_pdns(
        &self,
        ip: &str,
        api_key: &str,
    ) -> Result<Vec<PassiveDnsRecord>> {
        let url = format!(
            "https://otx.alienvault.com/api/v1/indicators/IPv4/{}/passive_dns",
            ip
        );

        let response = self
            .client
            .get(&url)
            .header("X-OTX-API-KEY", api_key)
            .send()
            .await?;

        if !response.status().is_success() {
            return Ok(Vec::new());
        }

        let otx_resp: serde_json::Value = response.json().await?;

        let mut records = Vec::new();
        if let Some(pdns) = otx_resp.get("passive_dns").and_then(|p| p.as_array()) {
            for record in pdns.iter().take(20) {
                if let Some(hostname) = record.get("hostname").and_then(|h| h.as_str()) {
                    records.push(PassiveDnsRecord {
                        record_type: "A".to_string(),
                        value: hostname.to_string(),
                        first_seen: record
                            .get("first_seen")
                            .and_then(|v| v.as_str())
                            .map(String::from),
                        last_seen: record
                            .get("last_seen")
                            .and_then(|v| v.as_str())
                            .map(String::from),
                        source: "AlienVault OTX".to_string(),
                    });
                }
            }
        }

        Ok(records)
    }

    async fn check_alienvault_domain_pdns(
        &self,
        domain: &str,
        api_key: &str,
    ) -> Result<Vec<PassiveDnsRecord>> {
        let url = format!(
            "https://otx.alienvault.com/api/v1/indicators/domain/{}/passive_dns",
            domain
        );

        let response = self
            .client
            .get(&url)
            .header("X-OTX-API-KEY", api_key)
            .send()
            .await?;

        if !response.status().is_success() {
            return Ok(Vec::new());
        }

        let otx_resp: serde_json::Value = response.json().await?;

        let mut records = Vec::new();
        if let Some(pdns) = otx_resp.get("passive_dns").and_then(|p| p.as_array()) {
            records.reserve(pdns.len().min(20));
            for record in pdns.iter().take(20) {
                if let Some(ip) = record.get("address").and_then(|a| a.as_str()) {
                    records.push(PassiveDnsRecord {
                        record_type: "A".to_string(),
                        value: ip.to_string(),
                        first_seen: record
                            .get("first_seen")
                            .and_then(|v| v.as_str())
                            .map(String::from),
                        last_seen: record
                            .get("last_seen")
                            .and_then(|v| v.as_str())
                            .map(String::from),
                        source: "AlienVault OTX".to_string(),
                    });
                }
            }
        }

        Ok(records)
    }
}

pub async fn check_threat_intel(
    target: &str,
    is_ip: bool,
    virustotal_key: Option<&SensitiveString>,
    alienvault_key: Option<&SensitiveString>,
    shodan_key: Option<&SensitiveString>,
) -> Result<ThreatIntel> {
    let client = ThreatIntelClient::new(
        virustotal_key.cloned(),
        alienvault_key.cloned(),
        shodan_key.cloned(),
        None,
    )?;

    if is_ip {
        client.check_ip(target).await
    } else {
        client.check_domain(target).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_threat_intel_default() {
        let intel = ThreatIntel::default();
        assert!(intel.target.is_empty());
        assert!(intel.ip_reputation.is_none());
        assert!(intel.domain_reputation.is_none());
        assert!(intel.passive_dns.is_empty());
    }

    #[test]
    fn test_ip_reputation_serialization() {
        let rep = IpReputation {
            ip: "8.8.8.8".to_string(),
            score: 0,
            category: "clean".to_string(),
            threats: vec![],
            asn: Some("AS15169".to_string()),
            isp: Some("Google LLC".to_string()),
            country: Some("US".to_string()),
            source: "VirusTotal".to_string(),
        };
        let json = serde_json::to_string(&rep).unwrap();
        assert!(json.contains("8.8.8.8"));
        assert!(json.contains("Google"));
        let decoded: IpReputation = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.ip, "8.8.8.8");
        assert_eq!(decoded.score, 0);
    }

    #[test]
    fn test_domain_reputation_serialization() {
        let rep = DomainReputation {
            domain: "evil.com".to_string(),
            score: 85,
            category: "malicious".to_string(),
            threats: vec!["malware".to_string(), "phishing".to_string()],
            registrar: Some("GoDaddy".to_string()),
            created_date: Some("2020-01-01".to_string()),
            nameservers: vec!["ns1.evil.com".to_string()],
            source: "VirusTotal".to_string(),
        };
        let json = serde_json::to_string(&rep).unwrap();
        let decoded: DomainReputation = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.domain, "evil.com");
        assert_eq!(decoded.score, 85);
        assert_eq!(decoded.threats.len(), 2);
    }

    #[test]
    fn test_passive_dns_record_serialization() {
        let record = PassiveDnsRecord {
            record_type: "A".to_string(),
            value: "1.2.3.4".to_string(),
            first_seen: Some("2023-01-01".to_string()),
            last_seen: Some("2024-01-01".to_string()),
            source: "AlienVault OTX".to_string(),
        };
        let json = serde_json::to_string(&record).unwrap();
        let decoded: PassiveDnsRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.value, "1.2.3.4");
        assert_eq!(decoded.source, "AlienVault OTX");
    }

    #[test]
    fn test_threat_intel_client_new() {
        let client = ThreatIntelClient::new(None, None, None, None);
        assert!(client.is_ok());
        let client = ThreatIntelClient::new(
            Some(SensitiveString::new("vt-key".to_string())),
            None,
            None,
            None,
        );
        assert!(client.is_ok());
    }

    #[test]
    fn test_ip_reputation_clone() {
        let rep = IpReputation {
            ip: "1.2.3.4".to_string(),
            score: 50,
            category: "suspicious".to_string(),
            threats: vec!["spam".to_string()],
            asn: None,
            isp: None,
            country: None,
            source: "Shodan".to_string(),
        };
        let cloned = rep.clone();
        assert_eq!(cloned.ip, "1.2.3.4");
        assert_eq!(cloned.score, 50);
    }

    #[test]
    fn test_domain_reputation_clone() {
        let rep = DomainReputation {
            domain: "test.com".to_string(),
            score: 10,
            category: "clean".to_string(),
            threats: vec![],
            registrar: None,
            created_date: None,
            nameservers: vec![],
            source: "VirusTotal".to_string(),
        };
        let cloned = rep.clone();
        assert_eq!(cloned.domain, "test.com");
    }

    #[test]
    fn test_passive_dns_record_clone() {
        let record = PassiveDnsRecord {
            record_type: "A".to_string(),
            value: "5.6.7.8".to_string(),
            first_seen: None,
            last_seen: None,
            source: "AlienVault OTX".to_string(),
        };
        let cloned = record.clone();
        assert_eq!(cloned.value, "5.6.7.8");
    }

    #[test]
    fn test_threat_intel_serialization() {
        let intel = ThreatIntel {
            target: "8.8.8.8".to_string(),
            ip_reputation: Some(IpReputation {
                ip: "8.8.8.8".to_string(),
                score: 0,
                category: "clean".to_string(),
                threats: vec![],
                asn: Some("AS15169".to_string()),
                isp: None,
                country: None,
                source: "VirusTotal".to_string(),
            }),
            domain_reputation: None,
            passive_dns: vec![PassiveDnsRecord {
                record_type: "A".to_string(),
                value: "8.8.8.8".to_string(),
                first_seen: None,
                last_seen: None,
                source: "AlienVault OTX".to_string(),
            }],
        };
        let json = serde_json::to_string(&intel).unwrap();
        let decoded: ThreatIntel = serde_json::from_str(&json).unwrap();
        assert!(decoded.ip_reputation.is_some());
        assert_eq!(decoded.passive_dns.len(), 1);
    }

    #[test]
    fn test_ip_reputation_score_categories() {
        let clean = IpReputation {
            ip: "1.1.1.1".to_string(),
            score: 0,
            category: "clean".to_string(),
            threats: vec![],
            asn: None,
            isp: None,
            country: None,
            source: "VT".to_string(),
        };
        let suspicious = IpReputation {
            ip: "1.1.1.2".to_string(),
            score: 30,
            category: "suspicious".to_string(),
            threats: vec!["spam".to_string()],
            asn: None,
            isp: None,
            country: None,
            source: "VT".to_string(),
        };
        let malicious = IpReputation {
            ip: "1.1.1.3".to_string(),
            score: 80,
            category: "malicious".to_string(),
            threats: vec!["malware".to_string(), "phishing".to_string()],
            asn: None,
            isp: None,
            country: None,
            source: "VT".to_string(),
        };
        assert_eq!(clean.category, "clean");
        assert_eq!(suspicious.category, "suspicious");
        assert_eq!(malicious.category, "malicious");
    }
}
