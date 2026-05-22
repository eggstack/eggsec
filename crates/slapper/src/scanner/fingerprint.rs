use crate::error::Result;
use crate::utils::parsing::{parse_ports, resolve_host};
use crate::utils::strip_controls;
use dashmap::DashMap;
use futures::future::join_all;
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;

use super::udp_fingerprint::{fingerprint_udp_services, get_default_udp_ports};
use crate::cli::FingerprintArgs;
use crate::config::SlapperConfig;

const MAX_SCAN_RESULTS: usize = 100_000;

static PROBES: &[(&str, &[u8], &str)] = &[
    ("HTTP", b"HEAD / HTTP/1.0\r\n\r\n", "HTTP"),
    ("SSH", b"", "SSH"),
    ("SMTP", b"EHLO test\r\n", "220|250|EHLO"),
    ("FTP", b"", "220"),
    ("MySQL", b"\x00\x00\x00\x00\x0a\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00", "\\x00\\x00\\xff"),
    ("Redis", b"PING\r\n", "+PONG"),
    ("MongoDB", b"\x3a\x00\x00\x00\xa8\x3d\x00\x00\x00\x00\x00\x00\xd4\x07\x00\x00\x00\x00\x00\x00admin.$cmd\x00\x00\x00\x00\x00\xff\xff\xff\xff\x1b\x00\x00\x00\x01ismaster\x00\x00\x00\x00\x00\x00\x00\xf0\x3f\x00", "ismaster"),
    ("PostgreSQL", b"\x00\x00\x00\x08\x04\xd2\x16\x2f", "\\x00\\x00\\x00"),
    ("Memcached", b"stats\r\n", "STAT|END"),
    ("RDP", b"\x03\x00\x00\x13\x0e\xe0\x00\x00\x00\x00\x00\x01\x00\x08\x00\x0b\x00\x00\x00", "\\x03\\x00"),
    ("VNC", b"", "RFB"),
    ("Telnet", b"", "login:|telnet"),
    ("XMPP", b"<?xml version='1.0'?><stream:stream xmlns='jabber:client' xmlns:stream='http://etherx.jabber.org/streams' to='test'><starttls xmlns='urn:ietf:params:xml:ns:xmpp-tls'/></stream:stream>", "<stream:stream"),
    ("LDAP", b"\x30\x0c\x02\x01\x01\x60\x07\x02\x01\x03\x04\x00\x80\x00", "\\x30"),
    ("SMB", b"\x00\x00\x00\x85\xff\x53\x4d\x42\x72\x00\x00\x00\x00\x18\x53\xc8\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\xff\xfe\x00\x00\x00\x00\x00\x62\x00\x02\x50\x43\x20\x4e\x45\x54\x57\x4f\x52\x4b\x20\x50\x52\x4f\x47\x52\x41\x4d\x20\x31\x2e\x30\x00\x02\x4c\x41\x4e\x4d\x41\x4e\x31\x2e\x30\x00\x02\x57\x69\x6e\x64\x6f\x77\x73\x20\x66\x6f\x72\x20\x57\x6f\x72\x6b\x67\x72\x6f\x75\x70\x73\x20\x33\x2e\x31\x61\x00\x02\x4c\x4d\x31\x2e\x32\x58\x30\x30\x32\x00\x02\x4c\x41\x4e\x4d\x41\x4e\x32\x2e\x31\x00\x02\x4e\x54\x20\x4c\x4d\x20\x30\x2e\x31\x32\x00", "\\x00\\x00\\x00\\xff\\x53\\x4d\\x42"),
    ("Elasticsearch", b"GET / HTTP/1.0\r\n\r\n", "\"name\"|\"cluster_name\"|lucene"),
    ("Kafka", b"\x00\x00\x00\x1c\x00\x01\x00\x00\x00\x00\x00\x03api\x00\x00\x00\x01\x00", "\\x00\\x00\\x00"),
    ("Zookeeper", b"ruok", "imok"),
    ("RabbitMQ", b"AMQP\x00\x00\x09\x01", "AMQP"),
    ("Cassandra", b"\x00\x00\x00\x00", "\\x00\\x00\\x00"),
    ("CouchDB", b"GET / HTTP/1.0\r\n\r\n", "CouchDB"),
    ("Docker", b"GET /version HTTP/1.0\r\nHost: localhost\r\n\r\n", "ApiVersion|Docker"),
    ("Kubernetes", b"GET /api/v1 HTTP/1.0\r\nHost: localhost\r\n\r\n", "\"kind\":|k8s"),
    ("Etcd", b"GET /version HTTP/1.0\r\nHost: localhost\r\n\r\n", "etcdserver"),
    ("Consul", b"GET /v1/agent/self HTTP/1.0\r\nHost: localhost\r\n\r\n", "Consul"),
    ("Nats", b"INFO\r\n", "INFO"),
    ("InfluxDB", b"GET /ping HTTP/1.0\r\nHost: localhost\r\n\r\n", "InfluxDB|X-Influxdb"),
    ("MSSQL", b"\x12\x01\x00\x34\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x01\x00\x00\x00", "\\x04\\x00"),
    ("Oracle", b"\x00\x7c\x00\x00\x06\x00\x00\x00\x00\x00\x00\x00\x00", "\\x00\\x7c"),
    ("Rsyncd", b"", "@RSYNCD:"),
    ("Memcached", b"version\r\n", "VERSION"),
    ("Couchbase", b"\x80\x00\x00\x05\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00", "\\x80"),
    ("OpenVPN", b"\x00\x01\x00\x00\x00\x00\x00\x00", "\\x00\\x01\\x00\\x00"),
    ("WinRM", b"<?xml version=\"1.0\"?><s:Envelope xmlns:s=\"http://www.w3.org/2003/05/soap-envelope\"><s:Body/></s:Envelope>", "soap"),
    ("Jenkins", b"GET /api/json HTTP/1.0\r\n\r\n", "\"jobs\"|Crumb"),
    ("ActiveMQ", b"CONNECT\r\n", "ActiveMQ"),
    ("WebSocket", b"GET / HTTP/1.1\r\nHost: localhost\r\nUpgrade: websocket\r\n\r\n", "101|Upgrade"),
    ("gRPC", b"\x00\x00\x00\x00\x10\x00\x00\x00\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00", "\\x00\\x00\\x00\\x00\\x10"),
    ("Caddy", b"GET / HTTP/1.0\r\n\r\n", "Caddy"),
    ("Harbor", b"GET /api/v2.0/ping HTTP/1.0\r\nHost: localhost\r\n\r\n", "\"ping\""),
    ("GitLab", b"GET /api/v4/version HTTP/1.0\r\n\r\n", "\"version\""),
    ("MinIO", b"GET /minio/health/live HTTP/1.0\r\n\r\n", "\"ready\""),
    ("Nginx", b"GET / HTTP/1.0\r\n\r\n", "nginx"),
    ("Apache", b"GET / HTTP/1.0\r\n\r\n", "Apache"),
    ("IIS", b"GET / HTTP/1.0\r\n\r\n", "IIS|Server"),
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceFingerprint {
    pub port: u16,
    pub service: String,
    pub banner: Option<String>,
    pub version: Option<String>,
    pub product: Option<String>,
    pub extra: Option<String>,
    pub confidence: u8,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FingerprintResults {
    pub host: String,
    pub ports_scanned: usize,
    pub services_identified: usize,
    pub duration_ms: u64,
    pub results: Vec<ServiceFingerprint>,
}

impl std::fmt::Display for FingerprintResults {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Service Fingerprint Results")?;
        writeln!(f, "Host: {}", strip_controls(&self.host, 65))?;
        writeln!(f, "Ports Scanned: {}", self.ports_scanned)?;
        writeln!(f, "Services Identified: {}", self.services_identified)?;
        writeln!(f, "Duration: {}ms", self.duration_ms)?;

        if self.results.is_empty() {
            writeln!(f, "No services identified")?;
        } else {
            writeln!(
                f,
                "{:>6} {:<15} {:<20} {:<30}",
                "PORT", "SERVICE", "PRODUCT/VERSION", "BANNER"
            )?;
            for fp in &self.results {
                let product_version = match (&fp.product, &fp.version) {
                    (Some(p), Some(v)) => format!("{} {}", p, v),
                    (Some(p), None) => p.clone(),
                    (None, Some(v)) => v.clone(),
                    (None, None) => "-".to_string(),
                };
                let banner = fp
                    .banner
                    .as_deref()
                    .unwrap_or("-")
                    .lines()
                    .next()
                    .unwrap_or("-");
                writeln!(
                    f,
                    "{:>6} {:<15} {:<20} {:<30}",
                    fp.port,
                    strip_controls(&fp.service, 15),
                    strip_controls(&product_version, 20),
                    strip_controls(banner, 30)
                )?;
            }
        }
        Ok(())
    }
}

pub async fn run_cli(args: FingerprintArgs, config: &SlapperConfig) -> Result<()> {
    let timeout_secs = if args.timeout == 5 {
        config.scan.port_timeout_secs
    } else {
        args.timeout
    };

    if args.udp {
        let ports = if args.ports == "80,443,22,21,25,3306,5432,6379,27017" {
            get_default_udp_ports()
        } else {
            parse_ports(&args.ports)?
        };

        let results =
            fingerprint_udp_services(&args.host, ports, Duration::from_secs(timeout_secs)).await?;

        if args.json {
            println!("{}", serde_json::to_string_pretty(&results)?);
        } else {
            println!("{}", results);
        }
    } else {
        let ports = parse_ports(&args.ports)?;
        let results = fingerprint_services(
            &args.host,
            ports,
            Duration::from_secs(timeout_secs),
            false,
            args.concurrency,
            None,
            None,
        )
        .await?;

        if args.json {
            println!("{}", serde_json::to_string_pretty(&results)?);
        } else {
            println!("{}", results);
        }
    }

    Ok(())
}

#[cfg(feature = "tool-api")]
pub async fn run_cli_with_callback<F>(
    args: FingerprintArgs,
    config: &SlapperConfig,
    mut callback: F,
) -> Result<()>
where
    F: FnMut(crate::tool::response::Finding) + Send + 'static,
{
    let timeout_secs = if args.timeout == 5 {
        config.scan.port_timeout_secs
    } else {
        args.timeout
    };

    if args.udp {
        let ports = if args.ports == "80,443,22,21,25,3306,5432,6379,27017" {
            get_default_udp_ports()
        } else {
            parse_ports(&args.ports)?
        };

        let results =
            fingerprint_udp_services(&args.host, ports, Duration::from_secs(timeout_secs)).await?;

        for fp in &results.results {
            callback(crate::tool::response::Finding::from(fp.clone()));
        }

        if args.json {
            println!("{}", serde_json::to_string_pretty(&results)?);
        } else {
            println!("{}", results);
        }
    } else {
        let ports = parse_ports(&args.ports)?;
        let results = fingerprint_services(
            &args.host,
            ports,
            Duration::from_secs(timeout_secs),
            false,
            args.concurrency,
            None,
            None,
        )
        .await?;

        for fp in &results.results {
            callback(crate::tool::response::Finding::from(fp.clone()));
        }

        if args.json {
            println!("{}", serde_json::to_string_pretty(&results)?);
        } else {
            println!("{}", results);
        }
    }

    Ok(())
}

pub async fn fingerprint_services(
    host: &str,
    ports: Vec<u16>,
    timeout_duration: Duration,
    tui_mode: bool,
    concurrency: usize,
    progress_tx: Option<tokio::sync::mpsc::Sender<(u64, u64)>>,
    max_results: Option<usize>,
) -> Result<FingerprintResults> {
    let resolved_ip = resolve_host(host)?;
    let results: Arc<DashMap<u16, ServiceFingerprint>> = Arc::new(DashMap::new());
    let scanned_count = Arc::new(tokio::sync::Mutex::new(0u64));
    let results_count = Arc::new(AtomicU64::new(0));
    let total_ports = ports.len() as u64;

    let progress = if tui_mode {
        None
    } else {
        let pb = Arc::new(ProgressBar::new(ports.len() as u64));
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ports ({eta})")
                .unwrap_or_else(|_| ProgressStyle::default_bar())
                .progress_chars("#>-"),
        );
        Some(pb)
    };

    let semaphore = Arc::new(tokio::sync::Semaphore::new(concurrency));
    let mut handles = Vec::with_capacity(ports.len());
    let start = std::time::Instant::now();
    let ports_count = ports.len();

    for port in ports {
        let permit = semaphore.clone().acquire_owned().await?;
        let resolved_ip = resolved_ip;
        let results = results.clone();
        let progress = progress.clone();
        let timeout_dur = timeout_duration;
        let scanned_count = scanned_count.clone();
        let progress_tx = progress_tx.clone();
        let results_count = results_count.clone();

        let handle = tokio::spawn(async move {
            if let Some(fp) = fingerprint_port(resolved_ip, port, timeout_dur).await {
                let should_insert = match max_results {
                    Some(limit) => {
                        let old = results_count.fetch_add(1, Ordering::Relaxed);
                        old < limit as u64
                    }
                    None => true,
                };
                if should_insert {
                    results.insert(port, fp);
                }
            }
            if let Some(ref pb) = progress {
                pb.inc(1);
            }
            if let Some(ref tx) = progress_tx {
                let count = {
                    let mut c = scanned_count.lock().await;
                    *c += 1;
                    *c
                };
                let _ = tx.send((count, total_ports)).await;
            }
            drop(permit);
        });

        handles.push(handle);
    }

    join_all(handles).await;
    if let Some(ref pb) = progress {
        pb.finish_and_clear();
    }

    let mut results: Vec<ServiceFingerprint> = Arc::try_unwrap(results)
        .expect("all workers completed")
        .into_iter()
        .map(|(_, v)| v)
        .collect();
    results.sort_by_key(|p| p.port);

    if results.len() > MAX_SCAN_RESULTS {
        results.truncate(MAX_SCAN_RESULTS);
    }

    let identified = results.len();

    Ok(FingerprintResults {
        host: host.to_string(),
        ports_scanned: ports_count,
        services_identified: identified,
        duration_ms: start.elapsed().as_millis() as u64,
        results,
    })
}

async fn fingerprint_port(ip: IpAddr, port: u16, timeout_duration: Duration) -> Option<ServiceFingerprint> {
    let addr = SocketAddr::new(ip, port);

    #[allow(unreachable_patterns)]
    let probes_to_try: Vec<(&str, &[u8], &str)> = match port {
        8080 | 8090 | 8180 => vec![("Jenkins", b"GET /api/json HTTP/1.0\r\n\r\n", "\"jobs\"|Crumb")],
        6443 | 8443 | 10443 => vec![("Kubernetes API", b"GET /api/v1 HTTP/1.0\r\nHost: localhost\r\n\r\n", "\"kind\":|k8s")],
        8086 | 8087 | 9092 => vec![("InfluxDB", b"GET /ping HTTP/1.0\r\nHost: localhost\r\n\r\n", "InfluxDB|X-Influxdb")],
        8081 | 8000 | 8001 => vec![("Caddy", b"GET / HTTP/1.0\r\n\r\n", "Caddy")],
        9090 | 3000 | 3001 => vec![("Prometheus/Grafana", b"GET /api/v1/status/config HTTP/1.0\r\n\r\n", "Prometheus|Grafana")],
        80 | 8080 | 8000 | 3000 | 5000 | 443 | 8443 | 8888 | 9000 | 9090 | 32768 | 49152 | 49153 | 49154 => vec![
            ("HTTP", b"HEAD / HTTP/1.0\r\n\r\n", "HTTP"),
        ],
        22 | 2222 => vec![("SSH", b"", "SSH")],
        21 | 2121 => vec![("FTP", b"", "220")],
        25 | 587 | 465 | 2525 => vec![("SMTP", b"EHLO test\r\n", "220|250|EHLO")],
        3306 | 3307 | 33060 => vec![("MySQL", b"\x00\x00\x00\x00\x0a\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00", "\\x00\\x00\\xff")],
        5432 | 5433 => vec![("PostgreSQL", b"\x00\x00\x00\x08\x04\xd2\x16\x2f", "\\x00\\x00\\x00")],
        6379 | 6380 => vec![("Redis", b"PING\r\n", "+PONG")],
        27017 | 27018 | 27019 | 28017 => vec![("MongoDB", b"\x3a\x00\x00\x00\xa8\x3d\x00\x00\x00\x00\x00\x00\xd4\x07\x00\x00\x00\x00\x00\x00admin.$cmd\x00\x00\x00\x00\x00\xff\xff\xff\xff\x1b\x00\x00\x00\x01ismaster\x00\x00\x00\x00\x00\x00\x00\xf0\x3f\x00", "ismaster")],
        11211 | 11212 => vec![("Memcached", b"stats\r\n", "STAT|END")],
        3389 => vec![("RDP", b"\x03\x00\x00\x13\x0e\xe0\x00\x00\x00\x00\x00\x01\x00\x08\x00\x0b\x00\x00\x00", "\\x03\\x00")],
        5900..=5903 => vec![("VNC", b"", "RFB")],
        23 => vec![("Telnet", b"", "login:|telnet")],
        5672 | 5671 => vec![("RabbitMQ", b"AMQP\x00\x00\x09\x01", "AMQP")],
        2181..=2183 => vec![("Zookeeper", b"ruok", "imok")],
        9200 | 9300 | 9243 => vec![("Elasticsearch", b"GET / HTTP/1.0\r\n\r\n", "\"name\"|\"cluster_name\"|lucene")],
        9092..=9094 => vec![("Kafka", b"\x00\x00\x00\x1c\x00\x01\x00\x00\x00\x00\x00\x03api\x00\x00\x00\x01\x00", "\\x00\\x00\\x00")],
        1433 | 1434 | 14330 => vec![("MSSQL", b"\x12\x01\x00\x34\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x01\x00\x00\x00", "\\x04\\x00")],
        1521 | 1522 | 1526 => vec![("Oracle", b"\x00\x7c\x00\x00\x06\x00\x00\x00\x00\x00\x00\x00\x00", "\\x00\\x7c")],
        873 => vec![("Rsyncd", b"", "@RSYNCD:")],
        11210 => vec![("Couchbase", b"\x80\x00\x00\x05\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00", "\\x80")],
        1194 => vec![("OpenVPN", b"\x00\x01\x00\x00\x00\x00\x00", "\\x00\\x01\\x00\\x00")],
        5985 | 5986 => vec![("WinRM", b"<?xml version=\"1.0\"?><s:Envelope xmlns:s=\"http://www.w3.org/2003/05/soap-envelope\"><s:Body/></s:Envelope>", "soap")],
        61613 | 61614 => vec![("ActiveMQ", b"CONNECT\r\n", "ActiveMQ")],
        8091 | 8092 => vec![("MinIO", b"GET /minio/health/live HTTP/1.0\r\n\r\n", "\"ready\"")],
        5984 => vec![("CouchDB", b"GET / HTTP/1.0\r\n\r\n", "CouchDB")],
        8500 => vec![("Consul", b"GET /v1/agent/self HTTP/1.0\r\nHost: localhost\r\n\r\n", "Consul")],
        2375 | 2376 => vec![("Docker", b"GET /version HTTP/1.0\r\nHost: localhost\r\n\r\n", "ApiVersion|Docker")],
        4222 | 8222 | 9222 | 6222 => vec![("Nats", b"INFO\r\n", "INFO")],
        2379 | 2380 => vec![("Etcd", b"GET /version HTTP/1.0\r\nHost: localhost\r\n\r\n", "etcdserver")],
        4444 => vec![("gRPC", b"\x00\x00\x00\x00\x10\x00\x00\x00\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00", "\\x00\\x00\\x00\\x00\\x10")],
        5555 => vec![("ZeroMQ", b"", "READY|DATA")],
        4150..=4152 => vec![("NSQ", b"  V2", "\\x20\\x56\\x32")],
        5666 => vec![("Nagios", b"", "nagios|NRPE")],
        502 | 102 | 44818 => vec![("Modbus/ICS", b"\x00\x00\x00\x05\x00\x00\x00\x00\x00\x39\x00\x03\x00\x00\x00\x05", "\\x00\\x00")],
        47808 => vec![("BACnet", b"\x81\x0a\x00\x11\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00", "\\x81")],
        _ => PROBES.to_vec(),
    };

    for (probe_name, probe_data, match_pattern) in probes_to_try {
        let stream = match crate::utils::network::connect_with_nodelay_timeout(
            &addr,
            timeout_duration,
        )
        .await
        {
            Ok(s) => s,
            Err(_) => continue,
        };

        let mut stream = stream;
        if let Some(fp) = probe_service(
            &mut stream,
            probe_name,
            probe_data,
            match_pattern,
            port,
            timeout_duration,
        )
        .await
        {
            return Some(fp);
        }
    }

    None
}

async fn probe_service(
    stream: &mut TcpStream,
    probe_name: &str,
    probe_data: &[u8],
    match_pattern: &str,
    port: u16,
    timeout_duration: Duration,
) -> Option<ServiceFingerprint> {
    if !probe_data.is_empty() {
        let _ = stream.write_all(probe_data).await;
    }

    let mut buffer: SmallVec<[u8; 256]> = SmallVec::new();
    buffer.resize(4096, 0);
    let read_result = timeout(timeout_duration, stream.read(&mut buffer)).await;

    match read_result {
        Ok(Ok(n)) if n > 0 => {
            let response = &buffer[..n];
            let response_str = String::from_utf8_lossy(response);
            let response_lower = response_str.to_lowercase();

            let matches = match_pattern.split('|').any(|pattern| {
                if pattern.starts_with("\\x") {
                    hex_match(pattern, response)
                } else {
                    response_lower.contains(&pattern.to_lowercase())
                }
            });

            if matches {
                let banner = extract_banner(&response_str);
                let (product, version) = extract_product_version(&response_str, probe_name);

                return Some(ServiceFingerprint {
                    port,
                    service: probe_name.to_string(),
                    banner: if !banner.is_empty() {
                        Some(banner)
                    } else {
                        None
                    },
                    version,
                    product,
                    extra: None,
                    confidence: 90,
                });
            }
        }
        _ => {}
    }

    None
}

fn hex_match(pattern: &str, data: &[u8]) -> bool {
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

fn extract_banner(response: &str) -> String {
    response
        .lines()
        .take(3)
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(200)
        .collect()
}

fn extract_product_version(response: &str, service: &str) -> (Option<String>, Option<String>) {
    match service {
        "HTTP" => {
            let server = response
                .lines()
                .find(|l| l.to_lowercase().starts_with("server:"))
                .and_then(|l| l.split(':').nth(1).map(|s| s.trim().to_string()));

            if let Some(server) = server {
                let parts: Vec<&str> = server.split('/').collect();
                if parts.len() == 2 {
                    return (Some(parts[0].to_string()), Some(parts[1].to_string()));
                }
                return (Some(server), None);
            }
        }
        "SSH" => {
            if let Some(version_line) = response.lines().next() {
                if version_line.starts_with("SSH-") {
                    let parts: Vec<&str> = version_line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        let version_parts: Vec<&str> = parts[0].split('-').collect();
                        if version_parts.len() >= 3 {
                            return (
                                Some(parts[1].to_string()),
                                Some(version_parts[2].to_string()),
                            );
                        }
                    }
                }
            }
        }
        _ => {}
    }

    (None, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_match_exact() {
        let data = [0x00, 0x01, 0x02, 0x03, 0x04];
        assert!(hex_match("\\x00\\x01\\x02", &data));
    }

    #[test]
    fn test_hex_match_offset() {
        let data = [0xFF, 0x00, 0x01, 0x02, 0xFE];
        assert!(hex_match("\\x00\\x01\\x02", &data));
    }

    #[test]
    fn test_hex_match_not_found() {
        let data = [0x00, 0x02, 0x04, 0x06];
        assert!(!hex_match("\\x01\\x03", &data));
    }

    #[test]
    fn test_hex_match_empty_pattern() {
        let data = [0x00, 0x01];
        assert!(!hex_match("", &data));
    }

    #[test]
    fn test_hex_match_empty_data() {
        assert!(!hex_match("\\x00\\x01", &[]));
    }

    #[test]
    fn test_hex_match_single_byte() {
        let data = [0xFF, 0x00, 0xFF];
        assert!(hex_match("\\x00", &data));
        assert!(!hex_match("\\x01", &data));
    }

    #[test]
    fn test_hex_match_end_of_data() {
        let data = [0x01, 0x02, 0x03, 0x04];
        assert!(hex_match("\\x03\\x04", &data));
    }

    #[test]
    fn test_extract_banner_single_line() {
        let response = "SSH-2.0-OpenSSH_8.2p1";
        let banner = extract_banner(response);
        assert_eq!(banner, "SSH-2.0-OpenSSH_8.2p1");
    }

    #[test]
    fn test_extract_banner_multi_line() {
        let response = "HTTP/1.1 200 OK\r\nServer: nginx/1.18\r\nContent-Length: 0\r\n";
        let banner = extract_banner(response);
        assert!(banner.contains("HTTP/1.1 200 OK"));
        assert!(banner.contains("Server: nginx/1.18"));
    }

    #[test]
    fn test_extract_banner_truncates() {
        let response = "A".repeat(300);
        let banner = extract_banner(&response);
        assert!(banner.len() <= 200);
    }

    #[test]
    fn test_extract_banner_empty() {
        let banner = extract_banner("");
        assert!(banner.is_empty());
    }

    #[test]
    fn test_extract_banner_joins_lines() {
        let response = "line1\nline2\nline3\nline4";
        let banner = extract_banner(response);
        assert_eq!(banner, "line1 line2 line3");
    }

    #[test]
    fn test_extract_product_version_http_with_version() {
        let response = "HTTP/1.1 200 OK\r\nServer: Apache/2.4.41\r\n";
        let (product, version) = extract_product_version(response, "HTTP");
        assert_eq!(product, Some("Apache".to_string()));
        assert_eq!(version, Some("2.4.41".to_string()));
    }

    #[test]
    fn test_extract_product_version_http_without_slash() {
        let response = "HTTP/1.1 200 OK\r\nServer: Microsoft-IIS\r\n";
        let (product, version) = extract_product_version(response, "HTTP");
        assert_eq!(product, Some("Microsoft-IIS".to_string()));
        assert_eq!(version, None);
    }

    #[test]
    fn test_extract_product_version_http_no_server() {
        let response = "HTTP/1.1 200 OK\r\nContent-Length: 0\r\n";
        let (product, version) = extract_product_version(response, "HTTP");
        assert_eq!(product, None);
        assert_eq!(version, None);
    }

    #[test]
    fn test_extract_product_version_ssh() {
        let response = "SSH-2.0-OpenSSH_8.2p1 Ubuntu-4ubuntu0.5";
        let (product, version) = extract_product_version(response, "SSH");
        assert_eq!(product, Some("Ubuntu-4ubuntu0.5".to_string()));
        assert_eq!(version, Some("OpenSSH_8.2p1".to_string()));
    }

    #[test]
    fn test_extract_product_version_unknown_service() {
        let response = "some response data";
        let (product, version) = extract_product_version(response, "Redis");
        assert_eq!(product, None);
        assert_eq!(version, None);
    }

    #[test]
    fn test_extract_product_version_http_case_insensitive() {
        let response = "HTTP/1.1 200 OK\r\nserver: nginx/1.18.0\r\n";
        let (product, version) = extract_product_version(response, "HTTP");
        assert_eq!(product, Some("nginx".to_string()));
        assert_eq!(version, Some("1.18.0".to_string()));
    }

    #[test]
    fn test_service_fingerprint_serialization() {
        let fp = ServiceFingerprint {
            port: 80,
            service: "HTTP".to_string(),
            banner: Some("HTTP/1.1 200 OK".to_string()),
            version: Some("1.18.0".to_string()),
            product: Some("nginx".to_string()),
            extra: None,
            confidence: 90,
        };
        let json = serde_json::to_string(&fp).unwrap();
        let deserialized: ServiceFingerprint = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.port, 80);
        assert_eq!(deserialized.service, "HTTP");
        assert_eq!(deserialized.confidence, 90);
    }

    #[test]
    fn test_fingerprint_results_display_empty() {
        let results = FingerprintResults {
            host: "example.com".to_string(),
            ports_scanned: 100,
            services_identified: 0,
            duration_ms: 5000,
            results: vec![],
        };
        let output = format!("{}", results);
        assert!(output.contains("No services identified"));
        assert!(output.contains("example.com"));
    }

    #[test]
    fn test_fingerprint_results_display_with_results() {
        let results = FingerprintResults {
            host: "example.com".to_string(),
            ports_scanned: 100,
            services_identified: 2,
            duration_ms: 5000,
            results: vec![
                ServiceFingerprint {
                    port: 22,
                    service: "SSH".to_string(),
                    banner: Some("SSH-2.0-OpenSSH_8.2".to_string()),
                    version: Some("OpenSSH_8.2".to_string()),
                    product: Some("OpenSSH".to_string()),
                    extra: None,
                    confidence: 90,
                },
                ServiceFingerprint {
                    port: 80,
                    service: "HTTP".to_string(),
                    banner: Some("HTTP/1.1 200 OK".to_string()),
                    version: None,
                    product: None,
                    extra: None,
                    confidence: 90,
                },
            ],
        };
        let output = format!("{}", results);
        assert!(output.contains("Services Identified: 2"));
        assert!(output.contains("SSH"));
        assert!(output.contains("HTTP"));
        assert!(output.contains("OpenSSH_8.2"));
    }

    #[test]
    fn test_probes_not_empty() {
        assert!(!PROBES.is_empty());
        assert!(PROBES.iter().any(|(name, _, _)| *name == "HTTP"));
        assert!(PROBES.iter().any(|(name, _, _)| *name == "SSH"));
    }
}
