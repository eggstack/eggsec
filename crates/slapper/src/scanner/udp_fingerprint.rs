
use std::net::SocketAddr;
use std::time::Duration;

use crate::error::Result;
use serde::{Deserialize, Serialize};
use tokio::net::UdpSocket;
use tokio::time::timeout;

use crate::utils::truncate;

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
        writeln!(f, "Host: {}", truncate(&self.host, 65))?;
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
                    truncate(&fp.service, 15),
                    truncate(&response, 50)
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
    ("Kafka", 9092, b"\x00\x00\x00\x1c\x00\x01\x00\x00\x00\x00\x00\x03api\x00\x00\x00\x01\x00", "Kafka"),
    ("Memcached", 11211, b"stats\r\n", "STAT"),
    ("Redis", 6379, b"PING\r\n", "PONG"),
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
    let mut results: Vec<UdpServiceFingerprint> = Vec::new();
    let start = std::time::Instant::now();
    let ports_count = ports.len();

    for port in ports {
        if let Some(fp) = fingerprint_udp_port(host, port, timeout_duration).await {
            results.push(fp);
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
    host: &str,
    port: u16,
    timeout_duration: Duration,
) -> Option<UdpServiceFingerprint> {
    let addr: SocketAddr = format!("{}:{}", host, port).parse().ok()?;

    let socket = match UdpSocket::bind("0.0.0.0:0").await {
        Ok(s) => s,
        Err(_) => return None,
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
        let _ = socket.send_to(probe_data, addr).await;

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
