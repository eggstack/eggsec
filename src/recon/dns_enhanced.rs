use serde::{Deserialize, Serialize};
use std::net::{IpAddr, ToSocketAddrs};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsRecord {
    pub record_type: String,
    pub name: String,
    pub value: String,
    pub ttl: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsEnumResult {
    pub domain: String,
    pub records: Vec<DnsRecord>,
    pub nameservers: Vec<String>,
    pub mail_servers: Vec<String>,
    pub txt_records: Vec<String>,
    pub dmarc: Option<String>,
    pub spf: Option<String>,
}

pub struct DnsEnumerator {
    wordlist: Vec<String>,
}

impl DnsEnumerator {
    pub fn new() -> Self {
        Self {
            wordlist: Self::default_wordlist(),
        }
    }

    fn default_wordlist() -> Vec<String> {
        vec![
            "www",
            "mail",
            "ftp",
            "localhost",
            "webmail",
            "smtp",
            "pop",
            "ns1",
            "webdisk",
            "ns2",
            "cpanel",
            "whm",
            "autodiscover",
            "autoconfig",
            "imap",
            "test",
            "ns",
            "blog",
            "pop3",
            "dev",
            "www2",
            "admin",
            "forum",
            "news",
            "vpn",
            "ns3",
            "mail2",
            "new",
            "mysql",
            "old",
            "lists",
            "support",
            "mobile",
            "mx",
            "static",
            "docs",
            "beta",
            "shop",
            "sql",
            "secure",
            "demo",
            "artha",
            "cdn",
            "wiki",
            "web",
            "dns2",
            "cloud",
            "git",
            "stats",
            "dns",
            "monitor",
            "server",
            "ns1",
            "correo",
            "m",
            "ftp2",
            "tmp",
            "cp",
            "ns11",
            "ns12",
            "ns13",
            "ns14",
            "staging",
            "dns1",
            "dns3",
            "api",
            "apps",
            "bbs",
            "web1",
            "db",
            "dns4",
            "ns15",
            "ns16",
            "ns17",
            "ns18",
            "ns19",
            "ns20",
            "v2",
            "mx1",
            "mx2",
            "mx3",
            "owa",
            "svn",
            "git1",
            "git2",
            "jira",
            "erp",
            "crm",
            "cms",
            "phpmyadmin",
            "email",
            "images",
            "img",
            "cdn1",
            "cdn2",
            "cdn3",
            "s3",
            "storage",
            "backup",
            "proxy",
            "router",
            "gateway",
            "fw",
            "firewall",
            "waf",
            "lb",
            "loadbalancer",
            "ssh",
            "telnet",
            "vnc",
            "rdp",
        ]
    }

    pub fn with_wordlist(mut self, wordlist: Vec<String>) -> Self {
        self.wordlist = wordlist;
        self
    }

    pub fn enumerate(&self, domain: &str) -> DnsEnumResult {
        let mut records = Vec::new();
        let mut nameservers = Vec::new();
        let mut mail_servers = Vec::new();
        let mut txt_records = Vec::new();

        if let Ok(addrs) = format!("{}.", domain).to_socket_addrs() {
            for addr in addrs {
                records.push(DnsRecord {
                    record_type: "A".to_string(),
                    name: domain.to_string(),
                    value: addr.ip().to_string(),
                    ttl: None,
                });
            }
        }

        if let Ok(host) = dns_lookup::lookup_host(&format!("ns1.{}", domain)) {
            for ip in host {
                nameservers.push(ip.to_string());
            }
        }

        if let Ok(host) = dns_lookup::lookup_host(&format!("ns2.{}", domain)) {
            for ip in host {
                nameservers.push(ip.to_string());
            }
        }

        if let Ok(host) = dns_lookup::lookup_host(&format!("mail.{}", domain)) {
            for ip in host {
                mail_servers.push(ip.to_string());
            }
        }

        DnsEnumResult {
            domain: domain.to_string(),
            records,
            nameservers,
            mail_servers,
            txt_records,
            dmarc: None,
            spf: None,
        }
    }

    pub fn enumerate_subdomains(&self, domain: &str) -> Vec<SubdomainResult> {
        let mut results = Vec::new();

        for prefix in &self.wordlist {
            let subdomain = format!("{}.{}", prefix, domain);

            if let Ok(ips) = dns_lookup::lookup_host(&subdomain) {
                if !ips.is_empty() {
                    results.push(SubdomainResult {
                        subdomain: subdomain.clone(),
                        ips: ips.iter().map(|ip| ip.to_string()).collect(),
                        has_http: false,
                        has_https: false,
                    });
                }
            }
        }

        results.sort_by(|a, b| b.ips.len().cmp(&a.ips.len()));

        results
    }

    pub fn check_zone_transfer(&self, domain: &str, nameserver: &str) -> Vec<DnsRecord> {
        vec![]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubdomainResult {
    pub subdomain: String,
    pub ips: Vec<String>,
    pub has_http: bool,
    pub has_https: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsRecordComparison {
    pub domain: String,
    pub previous: Vec<DnsRecord>,
    pub current: Vec<DnsRecord>,
    pub added: Vec<DnsRecord>,
    pub removed: Vec<DnsRecord>,
}

pub fn compare_dns_records(previous: &[DnsRecord], current: &[DnsRecord]) -> DnsRecordComparison {
    let prev_set: std::collections::HashSet<_> = previous
        .iter()
        .map(|r| format!("{}:{}:{}", r.record_type, r.name, r.value))
        .collect();

    let curr_set: std::collections::HashSet<_> = current
        .iter()
        .map(|r| format!("{}:{}:{}", r.record_type, r.name, r.value))
        .collect();

    let added: Vec<_> = current
        .iter()
        .filter(|r| !prev_set.contains(&format!("{}:{}:{}", r.record_type, r.name, r.value)))
        .cloned()
        .collect();

    let removed: Vec<_> = previous
        .iter()
        .filter(|r| !curr_set.contains(&format!("{}:{}:{}", r.record_type, r.name, r.value)))
        .cloned()
        .collect();

    DnsRecordComparison {
        domain: String::new(),
        previous: previous.to_vec(),
        current: current.to_vec(),
        added,
        removed,
    }
}

pub fn resolve_domain(domain: &str) -> Option<Vec<IpAddr>> {
    dns_lookup::lookup_host(domain).ok()
}
