use crate::error::{EggsecError, Result};
use hickory_resolver::config::{ResolverConfig, ResolverOpts};
use hickory_resolver::TokioResolver;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReverseDnsResult {
    pub hostname: Option<String>,
    pub ip_address: String,
    pub asn: Option<String>,
    pub organization: Option<String>,
}

fn create_resolver_opts() -> ResolverOpts {
    let mut opts = ResolverOpts::default();
    opts.timeout = Duration::from_secs(10);
    opts.attempts = 2;
    opts
}

pub async fn reverse_dns_lookup(ip: &str) -> Result<ReverseDnsResult> {
    let ip_addr: IpAddr = ip.parse()?;

    let resolver = TokioResolver::builder_with_config(
        ResolverConfig::default(),
        hickory_resolver::net::runtime::TokioRuntimeProvider::default(),
    )
    .with_options(create_resolver_opts())
    .build()?;

    let lookup = resolver.reverse_lookup(ip_addr).await?;

    let hostname = lookup.answers().first().map(|record| match &record.data {
        hickory_resolver::proto::rr::RData::PTR(ptr) => ptr.to_string(),
        data => data.to_string(),
    });

    let hostname_str = hostname.clone().unwrap_or_else(|| {
        tracing::debug!("reverse DNS hostname missing");
        String::new()
    });

    let (asn, organization) = if !hostname_str.is_empty() {
        extract_asn_from_hostname(&hostname_str)
    } else {
        (None, None)
    };

    Ok(ReverseDnsResult {
        hostname,
        ip_address: ip.to_string(),
        asn,
        organization,
    })
}

pub async fn asn_lookup(ip: &str) -> Result<(Option<String>, Option<String>)> {
    let resolver = TokioResolver::builder_with_config(
        ResolverConfig::default(),
        hickory_resolver::net::runtime::TokioRuntimeProvider::default(),
    )
    .with_options(create_resolver_opts())
    .build()?;

    let ip_addr: IpAddr = ip.parse()?;

    let lookup = resolver.reverse_lookup(ip_addr).await?;

    let hostname = lookup.answers().first().map(|record| match &record.data {
        hickory_resolver::proto::rr::RData::PTR(ptr) => ptr.to_string(),
        data => data.to_string(),
    });

    let (asn, org) = if let Some(h) = hostname {
        extract_asn_from_hostname(&h)
    } else {
        (None, None)
    };

    Ok((asn, org))
}

fn extract_asn_from_hostname(hostname: &str) -> (Option<String>, Option<String>) {
    let hostname_lower = hostname.to_lowercase();

    let asn_patterns = [
        ("as15169", "Google"),
        ("as13335", "Cloudflare"),
        ("as20940", "Akamai"),
        ("as54113", "Fastly"),
        ("as16509", "Amazon"),
        ("as8075", "Microsoft Azure"),
        ("as396982", "Google Cloud"),
        ("as45671", "StackPath"),
        ("as20473", "Choopa"),
        ("as49332", "M247"),
    ];

    for (pattern, name) in asn_patterns {
        if hostname_lower.contains(pattern)
            || hostname_lower.contains(&name.to_lowercase().replace(" ", "-"))
        {
            return (
                Some(format!("AS{}", pattern.trim_start_matches("as"))),
                Some(name.to_string()),
            );
        }
    }

    let asn_from_hostname = hostname_lower
        .split('.')
        .find(|part| part.starts_with("as") && part[2..].chars().all(|c| c.is_ascii_digit()));

    if let Some(asn) = asn_from_hostname {
        return (Some(format!("AS{}", &asn[2..])), None);
    }

    (None, None)
}

pub async fn resolve_domain(domain: &str) -> Result<Vec<String>> {
    let resolver = TokioResolver::builder_with_config(
        ResolverConfig::default(),
        hickory_resolver::net::runtime::TokioRuntimeProvider::default(),
    )
    .with_options(create_resolver_opts())
    .build()?;

    let lookup = resolver.lookup_ip(domain).await?;

    let ips: Vec<String> = lookup.iter().map(|ip| ip.to_string()).collect();

    Ok(ips)
}

pub async fn lookup_domain_info(target: &str) -> Result<ReverseDnsResult> {
    let _ip_or_domain = target.to_string();

    let ip_addr: Option<IpAddr> = target.parse().ok();

    let ip = if let Some(ip) = ip_addr {
        ip.to_string()
    } else {
        let ips = resolve_domain(target).await?;
        if ips.is_empty() {
            return Err(EggsecError::Network(format!(
                "Could not resolve domain: {}",
                target
            )));
        }
        ips[0].clone()
    };

    reverse_dns_lookup(&ip).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_asn_google() {
        let (asn, org) = extract_asn_from_hostname("google-public-dns-a.google.com");
        assert!(asn.is_some() || org.is_some());
    }

    #[test]
    fn test_extract_asn_cloudflare() {
        let (asn, org) = extract_asn_from_hostname("1dot1dot1dot1.cloudflare-dns.com");
        assert!(asn.is_some() || org.is_some());
    }

    #[test]
    fn test_extract_asn_generic_pattern() {
        let (asn, _org) = extract_asn_from_hostname("as15169.google.com");
        assert_eq!(asn, Some("AS15169".to_string()));
    }

    #[test]
    fn test_extract_asn_no_match() {
        let (asn, org) = extract_asn_from_hostname("example.com");
        assert!(asn.is_none());
        assert!(org.is_none());
    }
}
