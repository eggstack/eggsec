use rustc_hash::FxHashSet;
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
    let prev_set: FxHashSet<_> = previous
        .iter()
        .map(|r| format!("{}:{}:{}", r.record_type, r.name, r.value))
        .collect();

    let curr_set: FxHashSet<_> = current
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
    match dns_lookup::lookup_host(domain) {
        Ok(addrs) => Some(addrs),
        Err(e) => {
            tracing::debug!("DNS lookup failed for {}: {}", domain, e);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dns_record_serialization() {
        let record = DnsRecord {
            record_type: "A".to_string(),
            name: "example.com".to_string(),
            value: "93.184.216.34".to_string(),
            ttl: Some(300),
        };
        let json = serde_json::to_string(&record).unwrap();
        assert!(json.contains("A"));
        assert!(json.contains("93.184.216.34"));
        let decoded: DnsRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.value, "93.184.216.34");
    }

    #[test]
    fn test_dns_enum_result_serialization() {
        let result = DnsEnumResult {
            domain: "example.com".to_string(),
            records: vec![DnsRecord {
                record_type: "A".to_string(),
                name: "example.com".to_string(),
                value: "1.2.3.4".to_string(),
                ttl: None,
            }],
            nameservers: vec!["ns1.example.com".to_string()],
            mail_servers: vec!["mx.example.com".to_string()],
            txt_records: vec!["v=spf1 include:_spf.example.com ~all".to_string()],
            dmarc: Some("v=DMARC1; p=quarantine".to_string()),
            spf: Some("v=SPFv1".to_string()),
        };
        let json = serde_json::to_string(&result).unwrap();
        let decoded: DnsEnumResult = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.domain, "example.com");
        assert_eq!(decoded.records.len(), 1);
        assert!(decoded.dmarc.is_some());
    }

    #[test]
    fn test_subdomain_result_serialization() {
        let result = SubdomainResult {
            subdomain: "www.example.com".to_string(),
            ips: vec!["93.184.216.34".to_string()],
            has_http: true,
            has_https: true,
        };
        let json = serde_json::to_string(&result).unwrap();
        let decoded: SubdomainResult = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.subdomain, "www.example.com");
        assert!(decoded.has_https);
    }

    #[test]
    fn test_dns_enumerator_default_wordlist() {
        let enumerator = DnsEnumerator::new();
        assert!(!enumerator.wordlist.is_empty());
        assert!(enumerator.wordlist.contains(&"www".to_string()));
        assert!(enumerator.wordlist.contains(&"mail".to_string()));
        assert!(enumerator.wordlist.contains(&"api".to_string()));
    }

    #[test]
    fn test_dns_enumerator_with_custom_wordlist() {
        let enumerator =
            DnsEnumerator::new().with_wordlist(vec!["custom1".to_string(), "custom2".to_string()]);
        assert_eq!(enumerator.wordlist.len(), 2);
        assert!(enumerator.wordlist.contains(&"custom1".to_string()));
    }

    #[test]
    fn test_compare_dns_records_added() {
        let previous = vec![DnsRecord {
            record_type: "A".to_string(),
            name: "example.com".to_string(),
            value: "1.2.3.4".to_string(),
            ttl: None,
        }];
        let current = vec![
            DnsRecord {
                record_type: "A".to_string(),
                name: "example.com".to_string(),
                value: "1.2.3.4".to_string(),
                ttl: None,
            },
            DnsRecord {
                record_type: "A".to_string(),
                name: "example.com".to_string(),
                value: "5.6.7.8".to_string(),
                ttl: None,
            },
        ];
        let comparison = compare_dns_records(&previous, &current);
        assert_eq!(comparison.added.len(), 1);
        assert!(comparison.added[0].value.contains("5.6.7.8"));
    }

    #[test]
    fn test_compare_dns_records_removed() {
        let previous = vec![
            DnsRecord {
                record_type: "A".to_string(),
                name: "example.com".to_string(),
                value: "1.2.3.4".to_string(),
                ttl: None,
            },
            DnsRecord {
                record_type: "A".to_string(),
                name: "example.com".to_string(),
                value: "5.6.7.8".to_string(),
                ttl: None,
            },
        ];
        let current = vec![DnsRecord {
            record_type: "A".to_string(),
            name: "example.com".to_string(),
            value: "1.2.3.4".to_string(),
            ttl: None,
        }];
        let comparison = compare_dns_records(&previous, &current);
        assert_eq!(comparison.removed.len(), 1);
        assert!(comparison.removed[0].value.contains("5.6.7.8"));
    }

    #[test]
    fn test_compare_dns_records_no_changes() {
        let records = vec![DnsRecord {
            record_type: "A".to_string(),
            name: "example.com".to_string(),
            value: "1.2.3.4".to_string(),
            ttl: None,
        }];
        let comparison = compare_dns_records(&records, &records);
        assert!(comparison.added.is_empty());
        assert!(comparison.removed.is_empty());
    }

    #[test]
    fn test_dns_record_comparison_unchanged() {
        let record = DnsRecord {
            record_type: "A".to_string(),
            name: "example.com".to_string(),
            value: "1.2.3.4".to_string(),
            ttl: Some(3600),
        };
        let comparison = compare_dns_records(&[record.clone()], &[record.clone()]);
        assert!(comparison.added.is_empty());
        assert!(comparison.removed.is_empty());
    }

    #[test]
    fn test_dns_record_comparison_empty_previous() {
        let current = vec![DnsRecord {
            record_type: "A".to_string(),
            name: "example.com".to_string(),
            value: "1.2.3.4".to_string(),
            ttl: None,
        }];
        let comparison = compare_dns_records(&[], &current);
        assert_eq!(comparison.added.len(), 1);
        assert!(comparison.removed.is_empty());
    }

    #[test]
    fn test_dns_record_comparison_empty_current() {
        let previous = vec![DnsRecord {
            record_type: "A".to_string(),
            name: "example.com".to_string(),
            value: "1.2.3.4".to_string(),
            ttl: None,
        }];
        let comparison = compare_dns_records(&previous, &[]);
        assert!(comparison.added.is_empty());
        assert_eq!(comparison.removed.len(), 1);
    }
}
