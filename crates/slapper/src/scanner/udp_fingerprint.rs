use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use crate::error::Result;
use crate::utils::parsing::resolve_host;
use serde::{Deserialize, Serialize};
use tokio::net::UdpSocket;
use tokio::sync::Semaphore;
use tokio::time::timeout;

use crate::utils::strip_controls;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UdpServiceFingerprint {
    pub port: u16,
    pub service: String,
    pub response: Option<String>,
    pub banner: Option<String>,
    pub confidence: u8,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UdpFingerprintResults {
    pub host: String,
    pub ports_scanned: usize,
    pub services_identified: usize,
    pub duration_ms: u64,
    pub results: Vec<UdpServiceFingerprint>,
}

impl std::fmt::Display for UdpFingerprintResults {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "UDP Service Fingerprint Results")?;
        writeln!(f, "Host: {}", strip_controls(&self.host, 65))?;
        writeln!(f, "Ports Scanned: {}", self.ports_scanned)?;
        writeln!(f, "Services Identified: {}", self.services_identified)?;
        writeln!(f, "Duration: {}ms", self.duration_ms)?;

        if self.results.is_empty() {
            writeln!(f, "No UDP services identified")?;
        } else {
            writeln!(f, "{:>6} {:<15} {:<50}", "PORT", "SERVICE", "RESPONSE")?;
            for fp in &self.results {
                let response = fp
                    .response
                    .as_deref()
                    .unwrap_or("-")
                    .chars()
                    .take(50)
                    .collect::<String>();
                writeln!(
                    f,
                    "{:>6} {:<15} {:<50}",
                    fp.port,
                    strip_controls(&fp.service, 15),
                    strip_controls(&response, 50)
                )?;
            }
        }
        Ok(())
    }
}

static UDP_PROBES: &[(&str, u16, &[u8], &str)] = &[
    ("DNS", 53, b"\x12\x34\x01\x00\x00\x01\x00\x00\x00\x00\x00\x00", "NRS"),
    ("SNMP", 161, b"\x30\x00\x04\x01\x02", "SNMP"),
    ("SNMP-Trap", 162, b"\x30\x00\x04\x01\x03", "SNMP"),
    ("NTP", 123, b"\x23\x00\x04\x00\x00\x00\x00", "NTP"),
    ("SIP", 5060, b"OPTIONS sip:test SIP/2.0\r\n", "SIP"),
    ("SIP", 5061, b"OPTIONS sip:test SIP/2.0\r\n", "SIP"),
    ("mDNS", 5353, b"\x00\x00\x00\x00", "mDNS"),
    ("MDNS", 5353, b"\x00\x00\x00\x00", "mDNS"),
    ("Quake3", 27960, b"\xff\xff\xff\xff\x02getstatus", "Quake"),
    ("Quake3", 27961, b"\xff\xff\xff\xff\x02getstatus", "Quake"),
    ("TeamSpeak", 8767, b"\x00\x00\x00\x00", "TeamSpeak"),
    ("RADIUS", 1812, b"\x01\x00\x00\x14\x00\x01\x00\x00", "RADIUS"),
    ("RADIUS", 1813, b"\x01\x00\x00\x14\x00\x01\x00\x00", "RADIUS"),
    ("Kerberos", 88, b"\x00\x00\x00\x30", "Kerberos"),
    ("LDAP", 389, b"\x30\x0c\x02\x01\x01\x60\x07\x02\x01\x03\x04\x00\x80\x00", "\\x30"),
    ("ISAKMP", 500, b"\x00\x00\x00\x00\x00\x00\x00\x00", "ISAKMP"),
    ("IPSec", 4500, b"\x00\x00\x00\x00\x00\x00\x00\x00", "IPSec"),
    ("Syslog", 514, b"<134>Oct 11 22:14:15 test: test", "syslog"),
    ("DHCP", 67, b"\x01\x01\x06\x00", "DHCP"),
    ("TFTP", 69, b"\x00\x01\x00\x00", "TFTP"),
    ("NBNS", 137, b"\x81\x8d\x01\x10\x00\x01\x00\x00\x00\x00", "NBNS"),
    ("NBDGM", 138, b"\x81\x00\x00\x01", "NBDGM"),
    ("Steam", 27015, b"\xff\xff\xff\xff\x54\x53\x6f\x75\x72\x63\x65\x20\x45\x6e\x67\x69\x6e\x65\x20\x51\x75\x65\x72\x79\x00", "Steam"),
    ("Half-Life", 27015, b"\xff\xff\xff\xff\x54\x00", "Half-Life"),
    ("Minecraft", 25565, b"\x00\xfe\x01\x00", "Minecraft"),
    ("Memcached", 11211, b"stats\r\n", "STAT"),
    ("Etcd", 2379, b"GET /version HTTP/1.0\r\nHost: localhost\r\n\r\n", "etcd"),
    ("Vault", 8200, b"GET /v1/sys/health HTTP/1.0\r\nHost: localhost\r\n\r\n", "Vault"),
    ("Consul", 8500, b"GET /v1/agent/self HTTP/1.0\r\nHost: localhost\r\n\r\n", "Consul"),
    ("WireGuard", 51820, b"\x04\x00\x00\x00\x00\x00\x00\x00", "WireGuard"),
    ("OpenVPN", 1194, b"\x00\x01\x00\x00\x00\x00\x00", "OpenVPN"),
    ("IPMI", 623, b"\x00\x00\x00\x00", "IPMI"),
    ("Hadoop", 50030, b"GET / HTTP/1.0\r\nHost: localhost\r\n\r\n", "Hadoop"),
    ("Elasticsearch", 9200, b"GET / HTTP/1.0\r\n\r\n", "Elasticsearch"),
    ("JMX", 9010, b"\x00\x00\x00\x00", "JMX"),
    ("BACnet", 47808, b"\x81\x0a\x00\x11\x01\x00", "BACnet"),
    ("Modbus", 502, b"\x00\x00\x00\x05\x00\x00\x00\x00\x00\x39", "Modbus"),
    ("PROFINET", 102, b"\x02\x00\x00\x01\x00\xc8", "PROFINET"),
    ("S7comm", 102, b"\x03\x00\x00\x01\x00\xc8", "S7comm"),
    ("DNP3", 20000, b"\x05\x64\x01", "DNP3"),
    ("AMQP", 5672, b"AMQP\x00\x00\x09\x01", "AMQP"),
    ("MQTT", 1883, b"\x10\x0e\x00\x04MQTT\x04\x02\x00\x3c\x00\x00", "MQTT"),
];

pub async fn fingerprint_udp_services(
    host: &str,
    ports: Vec<u16>,
    timeout_duration: Duration,
) -> Result<UdpFingerprintResults> {
    let start = std::time::Instant::now();
    let ports_count = ports.len();
    if ports_count == 0 {
        return Ok(UdpFingerprintResults {
            host: host.to_string(),
            ports_scanned: 0,
            services_identified: 0,
            duration_ms: start.elapsed().as_millis() as u64,
            results: Vec::new(),
        });
    }

    let resolved_ip = resolve_host(host)?;
    let socket = match UdpSocket::bind("0.0.0.0:0").await {
        Ok(s) => Arc::new(s),
        Err(_) => {
            return Ok(UdpFingerprintResults {
                host: host.to_string(),
                ports_scanned: ports_count,
                services_identified: 0,
                duration_ms: start.elapsed().as_millis() as u64,
                results: Vec::new(),
            });
        }
    };
    let semaphore = Arc::new(Semaphore::new(50));
    let mut handles = Vec::new();

    for port in ports {
        let ip = resolved_ip;
        let socket = socket.clone();
        let semaphore = semaphore.clone();
        let handle = tokio::spawn(async move {
            let _permit = semaphore.acquire_owned().await.ok();
            let result = fingerprint_udp_port(ip, port, timeout_duration, Some(socket)).await;
            result
        });
        handles.push(handle);
    }

    let mut results: Vec<UdpServiceFingerprint> = Vec::new();
    for handle in handles {
        match handle.await {
            Ok(Some(fp)) => results.push(fp),
            Ok(None) => {}
            Err(e) => tracing::debug!("UDP fingerprint task panicked: {}", e),
        }
    }

    let identified = results.len();

    Ok(UdpFingerprintResults {
        host: host.to_string(),
        ports_scanned: ports_count,
        services_identified: identified,
        duration_ms: start.elapsed().as_millis() as u64,
        results,
    })
}

async fn fingerprint_udp_port(
    ip: IpAddr,
    port: u16,
    timeout_duration: Duration,
    socket: Option<Arc<UdpSocket>>,
) -> Option<UdpServiceFingerprint> {
    let addr = SocketAddr::new(ip, port);

    let socket = match socket {
        Some(s) => s,
        None => match UdpSocket::bind("0.0.0.0:0").await {
            Ok(s) => Arc::new(s),
            Err(_) => return None,
        },
    };

    let probes_to_try: Vec<(&str, &[u8], &str)> = UDP_PROBES
        .iter()
        .filter(|(_service, p, _, _)| *p == port)
        .map(|(service, _, probe, response)| (*service, *probe, *response))
        .collect();

    if probes_to_try.is_empty() {
        return None;
    }

    for (service, probe_data, expected_response) in probes_to_try {
        let send_result = timeout(timeout_duration, socket.send_to(probe_data, addr)).await;
        if send_result.is_err() {
            continue;
        }

        let mut buf = vec![0u8; 4096];
        match timeout(timeout_duration, socket.recv_from(&mut buf)).await {
            Ok(Ok((n, _))) if n > 0 => {
                let response = &buf[..n];
                let response_str = String::from_utf8_lossy(response);

                if expected_response.starts_with("\\x") {
                    if hex_contains(response, expected_response) {
                        return Some(UdpServiceFingerprint {
                            port,
                            service: service.to_string(),
                            response: Some(response_str.chars().take(100).collect()),
                            banner: Some(
                                response_str
                                    .lines()
                                    .next()
                                    .unwrap_or("")
                                    .chars()
                                    .take(50)
                                    .collect(),
                            ),
                            confidence: 80,
                        });
                    }
                } else if response_str
                    .to_lowercase()
                    .contains(&expected_response.to_lowercase())
                {
                    return Some(UdpServiceFingerprint {
                        port,
                        service: service.to_string(),
                        response: Some(response_str.chars().take(100).collect()),
                        banner: Some(
                            response_str
                                .lines()
                                .next()
                                .unwrap_or("")
                                .chars()
                                .take(50)
                                .collect(),
                        ),
                        confidence: 80,
                    });
                }
            }
            _ => {}
        }
    }

    None
}

fn hex_contains(data: &[u8], pattern: &str) -> bool {
    let hex_bytes: Vec<u8> = pattern
        .split("\\x")
        .filter(|s| !s.is_empty())
        .filter_map(|s| u8::from_str_radix(s, 16).ok())
        .collect();

    if hex_bytes.is_empty() {
        return false;
    }

    data.windows(hex_bytes.len())
        .any(|window| window == hex_bytes.as_slice())
}

pub fn get_default_udp_ports() -> Vec<u16> {
    vec![
        53, 123, 161, 162, 389, 500, 514, 5353, 5060, 5061, 1194, 1812, 1813, 4500, 623, 67, 69,
        137, 138, 27015, 27016, 8767, 25565, 1883, 5672, 2379, 8200, 8500, 51820, 9092, 9200,
        11211, 6379, 9010, 47808, 502, 102, 20000, 27960, 27961, 50030,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_udp_probes_not_empty() {
        assert!(!UDP_PROBES.is_empty());
    }

    #[test]
    fn test_udp_probes_have_valid_ports() {
        for (_service, port, _probe, _response) in UDP_PROBES {
            assert!(*port > 0);
        }
    }

    #[test]
    fn test_udp_probes_have_non_empty_probes() {
        for (_service, _port, probe, _response) in UDP_PROBES {
            assert!(!probe.is_empty());
        }
    }

    #[test]
    fn test_udp_probes_have_non_empty_response_patterns() {
        for (_service, _port, _probe, response) in UDP_PROBES {
            assert!(!response.is_empty());
        }
    }

    #[test]
    fn test_hex_contains_basic() {
        let data = b"\x30\x0c\x02\x01\x01\x60\x07\x02";
        assert!(hex_contains(data, "\\x30"));
    }

    #[test]
    fn test_hex_contains_multi_byte() {
        let data = b"\x12\x34\x56\x78";
        assert!(hex_contains(data, "\\x34\\x56"));
    }

    #[test]
    fn test_hex_contains_not_found() {
        let data = b"\x00\x01\x02\x03";
        assert!(!hex_contains(data, "\\xff"));
    }

    #[test]
    fn test_hex_contains_empty_pattern() {
        let data = b"\x00\x01\x02";
        assert!(!hex_contains(data, ""));
    }

    #[test]
    fn test_hex_contains_invalid_hex() {
        let data = b"\x00\x01\x02";
        assert!(!hex_contains(data, "\\xZZ"));
    }

    #[test]
    fn test_hex_contains_partial_match() {
        let data = b"hello\x30world";
        assert!(hex_contains(data, "\\x30"));
    }

    #[test]
    fn test_get_default_udp_ports_not_empty() {
        let ports = get_default_udp_ports();
        assert!(!ports.is_empty());
    }

    #[test]
    fn test_get_default_udp_ports_contains_common_ports() {
        let ports = get_default_udp_ports();
        assert!(ports.contains(&53));
        assert!(ports.contains(&123));
        assert!(ports.contains(&161));
    }

    #[test]
    fn test_udp_fingerprint_struct_default_values() {
        let fp = UdpServiceFingerprint {
            port: 53,
            service: "DNS".to_string(),
            response: Some("response".to_string()),
            banner: Some("banner".to_string()),
            confidence: 80,
        };
        assert_eq!(fp.port, 53);
        assert_eq!(fp.service, "DNS");
        assert_eq!(fp.confidence, 80);
    }

    #[test]
    fn test_udp_fingerprint_results_display_empty() {
        let results = UdpFingerprintResults {
            host: "127.0.0.1".to_string(),
            ports_scanned: 10,
            services_identified: 0,
            duration_ms: 100,
            results: vec![],
        };
        let display = format!("{}", results);
        assert!(display.contains("UDP Service Fingerprint Results"));
        assert!(display.contains("No UDP services identified"));
    }

    #[test]
    fn test_udp_fingerprint_results_display_with_results() {
        let results = UdpFingerprintResults {
            host: "127.0.0.1".to_string(),
            ports_scanned: 1,
            services_identified: 1,
            duration_ms: 50,
            results: vec![UdpServiceFingerprint {
                port: 53,
                service: "DNS".to_string(),
                response: Some("dns response".to_string()),
                banner: Some("dns banner".to_string()),
                confidence: 80,
            }],
        };
        let display = format!("{}", results);
        assert!(display.contains("DNS"));
        assert!(display.contains("Ports Scanned: 1"));
    }

    #[test]
    fn test_udp_fingerprint_results_display_strips_controls() {
        let results = UdpFingerprintResults {
            host: "host\x01with\x02controls".to_string(),
            ports_scanned: 1,
            services_identified: 1,
            duration_ms: 10,
            results: vec![UdpServiceFingerprint {
                port: 53,
                service: "DNS\x03".to_string(),
                response: Some("resp\x04".to_string()),
                banner: None,
                confidence: 80,
            }],
        };
        let display = format!("{}", results);
        assert!(!display.contains('\x01'));
    }

    #[tokio::test]
    async fn test_fingerprint_udp_port_invalid_host() {
        let invalid_ip: std::net::IpAddr = "192.0.2.255".parse().unwrap();
        let result = fingerprint_udp_port(
            invalid_ip,
            53,
            Duration::from_millis(10),
            None,
        )
        .await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_fingerprint_udp_services_empty_ports() {
        let result = fingerprint_udp_services("127.0.0.1", vec![], Duration::from_millis(10)).await;
        assert!(result.is_ok());
        let results = result.unwrap();
        assert_eq!(results.ports_scanned, 0);
        assert_eq!(results.services_identified, 0);
    }

    #[tokio::test]
    async fn test_fingerprint_udp_services_unreachable_host() {
        let result =
            fingerprint_udp_services("192.0.2.1", vec![53], Duration::from_millis(10)).await;
        assert!(result.is_ok());
        let results = result.unwrap();
        assert_eq!(results.ports_scanned, 1);
        assert_eq!(results.services_identified, 0);
    }

    #[test]
    fn test_udp_fingerprint_results_serde() {
        let results = UdpFingerprintResults {
            host: "127.0.0.1".to_string(),
            ports_scanned: 5,
            services_identified: 2,
            duration_ms: 200,
            results: vec![UdpServiceFingerprint {
                port: 53,
                service: "DNS".to_string(),
                response: None,
                banner: None,
                confidence: 80,
            }],
        };
        let json = serde_json::to_string(&results).unwrap();
        let parsed: UdpFingerprintResults = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.host, "127.0.0.1");
        assert_eq!(parsed.ports_scanned, 5);
    }
}
