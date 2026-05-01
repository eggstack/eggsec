use crate::proxy::{ProxyEntry, ProxyType};
use crate::types::SensitiveString;
use anyhow::Result;

#[allow(clippy::type_complexity)]
pub fn parse_proxy_url(
    proxy_url: &str,
) -> Result<(String, u16, Option<String>, Option<String>, ProxyType)> {
    let proxy_type = if proxy_url.starts_with("socks5://") || proxy_url.starts_with("socks://") {
        ProxyType::Socks5
    } else if proxy_url.starts_with("socks4://") {
        ProxyType::Socks4
    } else if proxy_url.starts_with("https://") {
        ProxyType::Https
    } else {
        ProxyType::Http
    };

    let remainder = proxy_url
        .trim_start_matches("socks5://")
        .trim_start_matches("socks://")
        .trim_start_matches("socks4://")
        .trim_start_matches("https://")
        .trim_start_matches("http://");

    let (auth, host_port) = if remainder.contains('@') {
        let parts: Vec<&str> = remainder.splitn(2, '@').collect();
        (Some(parts[0].to_string()), parts[1].to_string())
    } else {
        (None, remainder.to_string())
    };

    let (username, password) = if let Some(auth_str) = auth {
        let parts: Vec<&str> = auth_str.splitn(2, ':').collect();
        if parts.len() == 2 {
            (Some(parts[0].to_string()), Some(parts[1].to_string()))
        } else {
            (Some(parts[0].to_string()), None)
        }
    } else {
        (None, None)
    };

    let parts: Vec<&str> = host_port.rsplitn(2, ':').collect();
    let (address, port) = if parts.len() == 2 {
        (parts[1].to_string(), parts[0].parse()?)
    } else {
        anyhow::bail!("Invalid proxy format: {}", proxy_url);
    };

    Ok((address, port, username, password, proxy_type))
}

pub fn create_proxy_entry(proxy_url: &str) -> Result<ProxyEntry> {
    let (address, port, username, password, proxy_type) = parse_proxy_url(proxy_url)?;

    Ok(ProxyEntry {
        name: None,
        proxy_type,
        address,
        port,
        username,
        password: password.map(SensitiveString::new),
        weight: 1,
        priority: 0,
        timeout_ms: 10000,
        enabled: true,
        tags: Vec::new(),
    })
}
