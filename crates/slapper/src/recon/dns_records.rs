use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DnsRecords {
    pub domain: String,
    pub a: Vec<String>,
    pub aaaa: Vec<String>,
    pub cname: Vec<String>,
    pub mx: Vec<MxRecord>,
    pub txt: Vec<String>,
    pub ns: Vec<String>,
    pub soa: Option<SoaRecord>,
    pub caa: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MxRecord {
    pub preference: u16,
    pub exchange: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoaRecord {
    pub mname: String,
    pub rname: String,
    pub serial: u32,
    pub refresh: u32,
    pub retry: u32,
    pub expire: u32,
    pub minimum: u32,
}

pub async fn enumerate_dns_records(domain: &str) -> Result<DnsRecords> {
    use hickory_resolver::config::{ResolverConfig, ResolverOpts};
    use hickory_resolver::TokioResolver;

    let resolver = TokioResolver::builder_with_config(
        ResolverConfig::default(),
        hickory_resolver::net::runtime::TokioRuntimeProvider::default(),
    )
    .with_options(ResolverOpts::default())
    .build()?;

    let mut records = DnsRecords {
        domain: domain.to_string(),
        ..Default::default()
    };

    if let Ok(lookup) = resolver.lookup_ip(domain).await {
        for ip in lookup.iter() {
            match ip {
                IpAddr::V4(ipv4) => records.a.push(ipv4.to_string()),
                IpAddr::V6(ipv6) => records.aaaa.push(ipv6.to_string()),
            }
        }
    }

    if let Ok(lookup) = resolver.mx_lookup(domain).await {
        for record in lookup.answers() {
            if let hickory_resolver::proto::rr::RData::MX(mx) = &record.data {
                records.mx.push(MxRecord {
                    preference: mx.preference,
                    exchange: mx.exchange.to_string(),
                });
            }
        }
    }

    if let Ok(lookup) = resolver.txt_lookup(domain).await {
        for record in lookup.answers() {
            records.txt.push(record.data.to_string());
        }
    }

    if let Ok(lookup) = resolver.ns_lookup(domain).await {
        for record in lookup.answers() {
            records.ns.push(record.data.to_string());
        }
    }

    if let Ok(lookup) = resolver
        .lookup(domain, hickory_resolver::proto::rr::RecordType::CNAME)
        .await
    {
        for record in lookup.answers() {
            if let hickory_resolver::proto::rr::RData::CNAME(cname) = &record.data {
                records.cname.push(cname.to_string());
            }
        }
    }

    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dns_records_default() {
        let records = DnsRecords::default();
        assert!(records.a.is_empty());
        assert!(records.mx.is_empty());
        assert!(records.soa.is_none());
    }

    #[test]
    fn test_dns_records_serialization() {
        let records = DnsRecords {
            domain: "example.com".to_string(),
            a: vec!["93.184.216.34".to_string()],
            aaaa: vec![],
            cname: vec![],
            mx: vec![MxRecord {
                preference: 10,
                exchange: "mail.example.com".to_string(),
            }],
            txt: vec!["v=spf1 include:example.com ~all".to_string()],
            ns: vec!["ns1.example.com".to_string()],
            soa: Some(SoaRecord {
                mname: "ns1.example.com".to_string(),
                rname: "admin.example.com".to_string(),
                serial: 2024010101,
                refresh: 3600,
                retry: 900,
                expire: 604800,
                minimum: 86400,
            }),
            caa: vec![],
        };

        let json = serde_json::to_string(&records).unwrap();
        assert!(json.contains("example.com"));
        assert!(json.contains("mail.example.com"));

        let deserialized: DnsRecords = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.domain, "example.com");
        assert_eq!(deserialized.a.len(), 1);
        assert_eq!(deserialized.mx.len(), 1);
    }

    #[test]
    fn test_mx_record_serialization() {
        let mx = MxRecord {
            preference: 20,
            exchange: "backup.example.com".to_string(),
        };
        let json = serde_json::to_string(&mx).unwrap();
        let parsed: MxRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.preference, 20);
    }
}
