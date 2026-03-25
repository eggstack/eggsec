
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsnInfo {
    pub asn: String,
    pub prefix: String,
    pub name: String,
    pub description: String,
    pub country: String,
    pub registry: String,
    pub allocated: String,
    pub updated: String,
    pub abuse_contacts: Vec<String>,
    pub routing_policy: Option<String>,
    pub traffic_estimate: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpRange {
    pub prefix: String,
    pub start_address: String,
    pub end_address: String,
    pub asn: String,
    pub name: String,
    pub country: String,
}

pub struct AsnLookup;

impl AsnLookup {
    pub fn lookup(asn: &str) -> Result<AsnInfo, Box<dyn std::error::Error + Send + Sync>> {
        let asn_number = asn.trim_start_matches("AS");

        let url = format!("https://rdap.arin.net/rest/asn/AS{}", asn_number);

        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()?;

        let response = client
            .get(&url)
            .header("Accept", "application/json")
            .send()?;

        if !response.status().is_success() {
            return Err(format!("ASN lookup failed: {}", response.status()).into());
        }

        let json: serde_json::Value = response.json()?;

        let name = json
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string();

        let description = json
            .get("remarks")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.get("description"))
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let abuse_contacts = json
            .get("entities")
            .and_then(|v| v.as_array())
            .map(|entities| {
                entities
                    .iter()
                    .filter(|e| {
                        e.get("roles")
                            .and_then(|r| r.as_array())
                            .map(|roles| {
                                roles
                                    .iter()
                                    .any(|r| r.as_str().map(|s| s == "abuse").unwrap_or(false))
                            })
                            .unwrap_or(false)
                    })
                    .filter_map(|e| {
                        e.get("vcardArray")
                            .and_then(|v| v.as_array())
                            .and_then(|arr| arr.get(1))
                            .and_then(|v| v.as_array())
                            .and_then(|props| {
                                props.iter().find(|p| {
                                    p.as_array()
                                        .and_then(|a| a.first())
                                        .and_then(|t| t.as_str())
                                        .map(|s| s == "fn")
                                        .unwrap_or(false)
                                })
                            })
                            .and_then(|p| p.as_array())
                            .and_then(|a| a.get(3))
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(AsnInfo {
            asn: format!("AS{}", asn_number),
            prefix: String::new(),
            name,
            description,
            country: String::new(),
            registry: "ARIN".to_string(),
            allocated: String::new(),
            updated: String::new(),
            abuse_contacts,
            routing_policy: None,
            traffic_estimate: None,
        })
    }

    pub fn lookup_by_ip(ip: &str) -> Result<AsnInfo, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("https://rdap.arin.net/rest/ip/{}", ip);

        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()?;

        let response = client
            .get(&url)
            .header("Accept", "application/json")
            .send()?;

        if !response.status().is_success() {
            return Err(format!("IP lookup failed: {}", response.status()).into());
        }

        let json: serde_json::Value = response.json()?;

        let asn = json
            .get("asn")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        let prefix = json
            .get("network")
            .and_then(|n| n.get("cidr"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if asn == "Unknown" {
            return Err("No ASN found for IP".into());
        }

        let asn_info = Self::lookup(&asn)?;

        Ok(AsnInfo {
            asn,
            prefix,
            ..asn_info
        })
    }

    pub fn get_prefixes(
        asn: &str,
    ) -> Result<Vec<IpRange>, Box<dyn std::error::Error + Send + Sync>> {
        let asn_number = asn.trim_start_matches("AS");

        let url = format!("https://rdap.arin.net/rest/asn/AS{}/prefixes", asn_number);

        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()?;

        let response = client
            .get(&url)
            .header("Accept", "application/json")
            .send()?;

        if !response.status().is_success() {
            return Ok(vec![]);
        }

        let json: serde_json::Value = response.json()?;

        let mut prefixes = Vec::new();

        if let Some(prefix_list) = json.get("prefixes").and_then(|v| v.as_array()) {
            for prefix in prefix_list {
                if let (Some(cidr), Some(start), Some(end)) = (
                    prefix.get("cidr").and_then(|v| v.as_str()),
                    prefix.get("startAddress").and_then(|v| v.as_str()),
                    prefix.get("endAddress").and_then(|v| v.as_str()),
                ) {
                    prefixes.push(IpRange {
                        prefix: cidr.to_string(),
                        start_address: start.to_string(),
                        end_address: end.to_string(),
                        asn: format!("AS{}", asn_number),
                        name: String::new(),
                        country: String::new(),
                    });
                }
            }
        }

        Ok(prefixes)
    }
}

pub fn get_asn_info(target: &str) -> Result<AsnInfo, Box<dyn std::error::Error + Send + Sync>> {
    if target.starts_with("AS") {
        AsnLookup::lookup(target)
    } else if target.parse::<std::net::IpAddr>().is_ok() {
        AsnLookup::lookup_by_ip(target)
    } else {
        Err("Invalid target: must be ASN (AS12345) or IP address".into())
    }
}
