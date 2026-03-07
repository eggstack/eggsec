use anyhow::Result;
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
    use trust_dns_resolver::config::{ResolverConfig, ResolverOpts};
    use trust_dns_resolver::TokioAsyncResolver;

    let resolver = TokioAsyncResolver::tokio(
        ResolverConfig::default(),
        ResolverOpts::default(),
    );

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
        for mx in lookup.iter() {
            records.mx.push(MxRecord {
                preference: mx.preference(),
                exchange: mx.exchange().to_string(),
            });
        }
    }

    if let Ok(lookup) = resolver.txt_lookup(domain).await {
        for txt in lookup.iter() {
            records.txt.push(txt.to_string());
        }
    }

    if let Ok(lookup) = resolver.ns_lookup(domain).await {
        for ns in lookup.iter() {
            records.ns.push(ns.to_string());
        }
    }

    Ok(records)
}
