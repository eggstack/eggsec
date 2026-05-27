use crate::error::{Result, SlapperError};
use crate::utils::connect_with_nodelay;
use crate::utils::extract_target_from_url;
use serde::{Deserialize, Serialize};
use std::net::ToSocketAddrs;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WhoisResult {
    pub domain: String,
    pub registrar: Option<String>,
    pub created_date: Option<String>,
    pub expires_date: Option<String>,
    pub updated_date: Option<String>,
    pub nameservers: Vec<String>,
    pub status: Vec<String>,
    pub registrant: Option<String>,
    pub raw_data: Option<String>,
}

pub struct WhoisConfig {
    pub timeout_secs: u64,
    pub max_retries: u32,
    pub retry_delay_ms: u64,
}

impl Default for WhoisConfig {
    fn default() -> Self {
        Self {
            timeout_secs: 10,
            max_retries: 3,
            retry_delay_ms: 1000,
        }
    }
}

pub async fn whois_lookup(domain: &str) -> Result<WhoisResult> {
    whois_lookup_with_config(domain, &WhoisConfig::default()).await
}

pub async fn whois_lookup_with_config(domain: &str, config: &WhoisConfig) -> Result<WhoisResult> {
    let domain = clean_domain(domain);

    let whois_server = get_whois_server(&domain);

    let mut last_error = None;

    for attempt in 0..config.max_retries {
        match lookup_whois(&domain, whois_server).await {
            Ok(result) => {
                return Ok(parse_whois_response(&domain, &result));
            }
            Err(e) => {
                last_error = Some(e);
                if attempt < config.max_retries - 1 {
                    tokio::time::sleep(Duration::from_millis(
                        config.retry_delay_ms * (attempt + 1) as u64,
                    ))
                    .await;
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| SlapperError::Network("WHOIS lookup failed".to_string())))
}

fn clean_domain(domain: &str) -> String {
    let domain = domain.to_lowercase();
    let domain = domain.trim_end_matches('.');

    extract_target_from_url(domain).unwrap_or_else(|| domain.to_string())
}

fn get_whois_server(domain: &str) -> &'static str {
    let domain_lower = domain.to_lowercase();

    if domain_lower.ends_with(".com")
        || domain_lower.ends_with(".net")
        || domain_lower.ends_with(".org")
        || domain_lower.ends_with(".info")
        || domain_lower.ends_with(".biz")
        || domain_lower.ends_with(".io")
        || domain_lower.ends_with(".co")
        || domain_lower.ends_with(".me")
        || domain_lower.ends_with(".tv")
        || domain_lower.ends_with(".us")
        || domain_lower.ends_with(".cc")
        || domain_lower.ends_with(".app")
        || domain_lower.ends_with(".dev")
        || domain_lower.ends_with(".cloud")
        || domain_lower.ends_with(".xyz")
        || domain_lower.ends_with(".online")
    {
        "whois.internic.net"
    } else if domain_lower.ends_with(".uk") {
        "whois.nic.uk"
    } else if domain_lower.ends_with(".de") {
        "whois.denic.de"
    } else if domain_lower.ends_with(".nl") {
        "whois.domain-registry.nl"
    } else if domain_lower.ends_with(".eu") {
        "whois.eu"
    } else if domain_lower.ends_with(".ru") || domain_lower.ends_with(".su") {
        "whois.tcinet.ru"
    } else if domain_lower.ends_with(".cn") {
        "whois.cnnic.cn"
    } else if domain_lower.ends_with(".jp") {
        "whois.jprs.jp"
    } else if domain_lower.ends_with(".br") {
        "whois.registro.br"
    } else if domain_lower.ends_with(".au") {
        "whois.auda.org.au"
    } else if domain_lower.ends_with(".ca") {
        "whois.cira.ca"
    } else if domain_lower.ends_with(".nz") {
        "whois.irs.net.nz"
    } else if domain_lower.ends_with(".pl") {
        "whois.dns.pl"
    } else if domain_lower.ends_with(".ch") {
        "whois.nic.ch"
    } else if domain_lower.ends_with(".fr") {
        "whois.afnic.fr"
    } else if domain_lower.ends_with(".es") {
        "whois.dn.es"
    } else if domain_lower.ends_with(".se") {
        "whois.iis.se"
    } else if domain_lower.ends_with(".nu") {
        "whois.nic.nu"
    } else {
        "whois.internic.net"
    }
}

async fn lookup_whois(domain: &str, server: &str) -> Result<String> {
    let addr = format!("{}:43", server);

    let mut addrs = addr.to_socket_addrs()?;
    let socket_addr = addrs
        .next()
        .ok_or_else(|| SlapperError::Network(format!("No address found for {}", server)))?;

    let stream = connect_with_nodelay(&socket_addr).await?;

    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let mut stream = stream;

    let query = format!("{}\r\n", domain);
    stream.write_all(query.as_bytes()).await?;
    stream.flush().await?;

    let mut response = Vec::new();
    let mut buf = [0u8; 4096];

    let read_timeout = tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            match stream.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    response.extend_from_slice(&buf[..n]);
                    if response.len() > 10 * 1024 * 1024 {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    if let Err(e) = read_timeout.await {
        tracing::warn!(target: "recon", "WHOIS read timeout: {}", e);
    }

    let response_str = String::from_utf8_lossy(&response).to_string();

    if response_str.contains("Throttled") || response_str.contains("Rate limit") {
        return Err(SlapperError::RateLimited("WHOIS rate limited".to_string()));
    }

    if response_str.is_empty() {
        return Err(SlapperError::Network("Empty WHOIS response".to_string()));
    }

    Ok(response_str)
}

fn parse_whois_response(domain: &str, response: &str) -> WhoisResult {
    let mut result = WhoisResult {
        domain: domain.to_string(),
        ..Default::default()
    };

    for line in response.lines() {
        let line = line.trim();
        let line_lower = line.to_lowercase();

        if line_lower.starts_with("registrar:") {
            result.registrar = Some(line[10..].trim().to_string());
        } else if line_lower.starts_with("creation date:")
            || line_lower.starts_with("created date:")
        {
            result.created_date = Some(extract_value(line));
        } else if line_lower.starts_with("expiry date:")
            || line_lower.starts_with("expiration date:")
        {
            result.expires_date = Some(extract_value(line));
        } else if line_lower.starts_with("updated date:")
            || line_lower.starts_with("modified date:")
        {
            result.updated_date = Some(extract_value(line));
        } else if line_lower.starts_with("name server:") || line_lower.starts_with("nserver:") {
            let ns = extract_value(line);
            if !ns.is_empty() && !result.nameservers.contains(&ns) {
                result.nameservers.push(ns);
            }
        } else if line_lower.starts_with("domain status:") || line_lower.starts_with("status:") {
            let status = extract_value(line);
            if !status.is_empty() && !result.status.contains(&status) {
                result.status.push(status);
            }
        } else if line_lower.starts_with("registrant:") {
            result.registrant = Some(extract_value(line));
        }
    }

    result.raw_data = Some(response.to_string());

    result
}

fn extract_value(line: &str) -> String {
    if let Some(pos) = line.find(':') {
        line[pos + 1..].trim().to_string()
    } else {
        String::new()
    }
}

pub async fn get_domain_creation_date(domain: &str) -> Result<Option<String>> {
    let result = whois_lookup(domain).await?;
    Ok(result.created_date)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_domain() {
        assert_eq!(clean_domain("example.com"), "example.com");
        assert_eq!(clean_domain("http://example.com"), "example.com");
        assert_eq!(clean_domain("https://example.com/path"), "example.com");
        assert_eq!(clean_domain("EXAMPLE.COM"), "example.com");
        assert_eq!(clean_domain("example.com."), "example.com");
    }

    #[test]
    fn test_whois_server_selection() {
        assert_eq!(get_whois_server("example.com"), "whois.internic.net");
        assert_eq!(get_whois_server("example.org"), "whois.internic.net");
        assert_eq!(get_whois_server("example.co.uk"), "whois.nic.uk");
        assert_eq!(get_whois_server("example.de"), "whois.denic.de");
    }
}
