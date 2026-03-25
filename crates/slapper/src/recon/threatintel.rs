
#![allow(dead_code)]

use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};

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
    virustotal_key: Option<String>,
    alienvault_key: Option<String>,
    shodan_key: Option<String>,
    threatstream_key: Option<String>,
}

impl ThreatIntelClient {
    pub fn new(
        virustotal_key: Option<String>,
        alienvault_key: Option<String>,
        shodan_key: Option<String>,
        threatstream_key: Option<String>,
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
            if let Ok(reputation) = self.check_virustotal_ip(ip, key).await {
                intel.ip_reputation = Some(reputation);
            }
        }

        if let Some(ref key) = self.shodan_key {
            if let Ok(reputation) = self.check_shodan_ip(ip, key).await {
                if intel.ip_reputation.is_none() {
                    intel.ip_reputation = Some(reputation);
                }
            }
        }

        if let Some(ref key) = self.alienvault_key {
            if let Ok(pdns) = self.check_alienvault_pdns(ip, key).await {
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
            if let Ok(reputation) = self.check_virustotal_domain(domain, key).await {
                intel.domain_reputation = Some(reputation);
            }
        }

        if let Some(ref key) = self.alienvault_key {
            if let Ok(pdns) = self.check_alienvault_domain_pdns(domain, key).await {
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
            return Err(anyhow::anyhow!("VT API request failed"));
        }

        let vt_resp: serde_json::Value = response.json().await?;

        let data = vt_resp
            .get("data")
            .and_then(|d| d.get("attributes"))
            .ok_or_else(|| anyhow::anyhow!("Invalid VT response"))?;

        let last_analysis = data
            .get("last_analysis_stats")
            .ok_or_else(|| anyhow::anyhow!("No analysis stats"))?;

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
            return Err(anyhow::anyhow!("VT API request failed"));
        }

        let vt_resp: serde_json::Value = response.json().await?;

        let data = vt_resp
            .get("data")
            .and_then(|d| d.get("attributes"))
            .ok_or_else(|| anyhow::anyhow!("Invalid VT response"))?;

        let last_analysis = data
            .get("last_analysis_stats")
            .ok_or_else(|| anyhow::anyhow!("No analysis stats"))?;

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
                .unwrap_or_default(),
            source: "VirusTotal".to_string(),
        })
    }

    async fn check_shodan_ip(&self, ip: &str, api_key: &str) -> Result<IpReputation> {
        let url = format!("https://api.shodan.io/shodan/host/{}?key={}", ip, api_key);

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Shodan API request failed"));
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
    virustotal_key: Option<String>,
    alienvault_key: Option<String>,
    shodan_key: Option<String>,
) -> Result<ThreatIntel> {
    let client = ThreatIntelClient::new(virustotal_key, alienvault_key, shodan_key, None)?;

    if is_ip {
        client.check_ip(target).await
    } else {
        client.check_domain(target).await
    }
}
