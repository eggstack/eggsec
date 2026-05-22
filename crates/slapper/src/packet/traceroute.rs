use serde::{Deserialize, Serialize};
use hickory_resolver::config::{ResolverConfig, ResolverOpts};
use hickory_resolver::TokioResolver;
use std::net::{IpAddr, SocketAddr};
use std::time::{Duration, Instant};

#[cfg(all(feature = "stress-testing", unix))]
use surge_ping;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracerouteConfig {
    pub target: String,
    pub max_hops: u8,
    pub timeout: Duration,
    pub max_retries: u8,
    pub first_ttl: u8,
    pub port: u16,
    pub use_icmp: bool,
    pub packet_size: usize,
    pub parallel_probes: bool,
    pub resolve_names: bool,
}

impl Default for TracerouteConfig {
    fn default() -> Self {
        Self {
            target: String::new(),
            max_hops: 30,
            timeout: Duration::from_secs(3),
            max_retries: 3,
            first_ttl: 1,
            port: 33434,
            use_icmp: false,
            packet_size: 32,
            parallel_probes: true,
            resolve_names: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracerouteHop {
    pub hop: u8,
    pub address: Option<String>,
    pub rtt: Option<Duration>,
    pub rtt_ms: Option<f64>,
    pub name: Option<String>,
    pub is_final: bool,
    pub probes: Vec<HopProbe>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HopProbe {
    pub address: Option<String>,
    pub rtt: Option<Duration>,
    pub success: bool,
}

impl TracerouteHop {
    pub fn new(hop: u8) -> Self {
        Self {
            hop,
            address: None,
            rtt: None,
            rtt_ms: None,
            name: None,
            is_final: false,
            probes: Vec::new(),
        }
    }

    pub fn add_probe(&mut self, address: Option<String>, rtt: Option<Duration>) {
        let probe = HopProbe {
            address: address.clone(),
            rtt,
            success: address.is_some(),
        };
        self.probes.push(probe);

        if self.address.is_none() {
            self.address = address;
            self.rtt = rtt;
            self.rtt_ms = rtt.map(|d| d.as_secs_f64() * 1000.0);
        } else if let Some(r) = rtt {
            if let Some(existing) = self.rtt {
                if r < existing {
                    self.address = address;
                    self.rtt = Some(r);
                    self.rtt_ms = Some(r.as_secs_f64() * 1000.0);
                }
            }
        }
    }

    pub fn success(&self) -> bool {
        self.address.is_some()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracerouteResult {
    pub target: String,
    pub resolved_address: String,
    pub hops: Vec<TracerouteHop>,
    pub total_hops: usize,
    pub success: bool,
}

pub struct Traceroute {
    config: TracerouteConfig,
}

impl Traceroute {
    pub fn new(config: TracerouteConfig) -> Self {
        Self { config }
    }

    pub async fn run(&self) -> Result<TracerouteResult, TracerouteError> {
        if self.config.use_icmp {
            return Err(TracerouteError::Unsupported(
                "ICMP traceroute is currently disabled because hop TTL controls are not applied correctly. Use UDP mode.".to_string(),
            ));
        }

        let target_ip = self.resolve_target(&self.config.target)?;
        let target_str = target_ip.to_string();

        tracing::info!(
            target = %self.config.target,
            ip = %target_ip,
            use_udp = !self.config.use_icmp,
            parallel = self.config.parallel_probes,
            "Starting traceroute"
        );

        let mut hops = Vec::new();
        let mut final_reached = false;

        let use_icmp = self.config.use_icmp;

        if self.config.parallel_probes {
            for ttl in self.config.first_ttl..=self.config.max_hops {
                let hop = if use_icmp {
                    self.probe_hop_icmp_parallel(target_ip, ttl).await
                } else {
                    self.probe_hop_udp_parallel(target_ip, ttl).await
                };

                let hop_addr = hop.address.clone();
                if let Some(ref addr) = hop_addr {
                    if self.config.resolve_names {
                        let mut hop_with_name = hop;
                        hop_with_name.name = Self::reverse_dns(addr).await.ok();
                        hops.push(hop_with_name);
                    } else {
                        hops.push(hop);
                    }
                } else {
                    hops.push(hop);
                }

                if hops.last().map(|h| h.is_final).unwrap_or(false) {
                    final_reached = true;
                    break;
                }

                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        } else {
            for ttl in self.config.first_ttl..=self.config.max_hops {
                let mut hop = TracerouteHop::new(ttl);

                for _ in 0..self.config.max_retries {
                    let start = Instant::now();

                    let probe_result = if use_icmp {
                        self.probe_icmp(target_ip, ttl).await
                    } else {
                        self.probe_udp(target_ip, ttl).await
                    };

                    match probe_result {
                        Ok(response_ip) => {
                            let rtt = start.elapsed();
                            let ip_str = response_ip.to_string();
                            hop.add_probe(Some(ip_str.clone()), Some(rtt));

                            if response_ip == target_ip {
                                hop.is_final = true;
                                final_reached = true;
                                break;
                            }
                        }
                        Err(ProbeError::Timeout) => {
                            hop.add_probe(None, None);
                        }
                        Err(ProbeError::PortUnreachable) => {
                            let rtt = start.elapsed();
                            hop.add_probe(Some(target_str.clone()), Some(rtt));
                            hop.is_final = true;
                            final_reached = true;
                            break;
                        }
                        Err(e) => {
                            tracing::debug!("Probe error at hop {}: {}", ttl, e);
                            hop.add_probe(None, None);
                        }
                    }
                }

                if self.config.resolve_names {
                    let hop_addr = hop.address.clone();
                    if let Some(ref addr) = hop_addr {
                        let mut hop_with_name = hop;
                        hop_with_name.name = Self::reverse_dns(addr).await.ok();
                        hops.push(hop_with_name);
                    } else {
                        hops.push(hop);
                    }
                } else {
                    hops.push(hop);
                }

                if final_reached {
                    break;
                }

                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        }

        let total_hops = hops.len();

        tracing::info!(hops = total_hops, final_reached, "Traceroute completed");

        Ok(TracerouteResult {
            target: self.config.target.clone(),
            resolved_address: target_str,
            hops,
            total_hops,
            success: final_reached,
        })
    }

    async fn probe_hop_udp_parallel(&self, target: IpAddr, ttl: u8) -> TracerouteHop {
        let mut hop = TracerouteHop::new(ttl);
        let target_port = self.config.port + ttl as u16 - 1;

        let probes: Vec<_> = (0..self.config.max_retries)
            .map(|_| {
                let target = target;
                let ttl = ttl;
                let timeout = self.config.timeout;
                let port = target_port;
                let packet_size = self.config.packet_size;

                tokio::spawn(async move {
                    let start = Instant::now();
                    let socket = std::net::UdpSocket::bind("0.0.0.0:0")
                        .map_err(|e| ProbeError::SocketError(e.to_string()))?;
                    socket
                        .set_read_timeout(Some(timeout))
                        .map_err(|e| ProbeError::SocketError(e.to_string()))?;
                    socket
                        .set_ttl(ttl as u32)
                        .map_err(|e| ProbeError::SocketError(e.to_string()))?;

                    let packet = vec![0u8; packet_size];
                    let dst = SocketAddr::new(target, port);

                    socket
                        .send_to(&packet, dst)
                        .map_err(|e| ProbeError::SendError(e.to_string()))?;

                    let mut buf = [0u8; 1024];
                    match socket.recv_from(&mut buf) {
                        Ok((_, addr)) => {
                            let rtt = start.elapsed();
                            Ok((addr.ip(), rtt))
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
                            Err(ProbeError::Timeout)
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::ConnectionRefused => {
                            let rtt = start.elapsed();
                            Ok((target, rtt))
                        }
                        Err(e) => Err(ProbeError::ReceiveError(e.to_string())),
                    }
                })
            })
            .collect();

        for probe in probes {
            match probe.await {
                Ok(Ok((ip, rtt))) => {
                    let ip_str = ip.to_string();
                    hop.add_probe(Some(ip_str), Some(rtt));

                    if ip == target {
                        hop.is_final = true;
                        break;
                    }
                }
                Ok(Err(ProbeError::Timeout)) => {
                    hop.add_probe(None, None);
                }
                Ok(Err(ProbeError::PortUnreachable)) => {
                    hop.add_probe(Some(target.to_string()), Some(Duration::ZERO));
                    hop.is_final = true;
                    break;
                }
                Err(_) => {
                    hop.add_probe(None, None);
                }
                _ => {}
            }
        }

        hop
    }

    #[cfg(all(feature = "stress-testing", unix))]
    async fn probe_hop_icmp_parallel(&self, target: IpAddr, ttl: u8) -> TracerouteHop {
        use surge_ping::{Client, Config};

        let mut hop = TracerouteHop::new(ttl);
        let timeout = self.config.timeout;
        let payload = vec![0u8; self.config.packet_size];

        let config = Config::default();
        let client = match Client::new(&config) {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("Failed to create ICMP client: {}", e);
                return hop;
            }
        };

        let mut handles = Vec::new();

        for _ in 0..self.config.max_retries {
            let target = target;
            let payload = payload.clone();

            let handle: tokio::task::JoinHandle<(Option<IpAddr>, Option<Duration>)> =
                tokio::spawn(async move {
                    match surge_ping::ping(target, &payload).await {
                        Ok((_, rtt)) => (Some(target), Some(rtt)),
                        Err(e) => {
                            tracing::debug!("ICMP ping failed: {}", e);
                            (None, None)
                        }
                    }
                });

            handles.push(handle);
        }

        for handle in handles {
            match handle.await {
                Ok((Some(ip), Some(rtt))) => {
                    hop.add_probe(Some(ip.to_string()), Some(rtt));
                    if ip == target {
                        hop.is_final = true;
                    }
                }
                Ok((None, None)) => {
                    hop.add_probe(None, None);
                }
                _ => {}
            }
        }

        hop
    }

    #[cfg(not(all(feature = "stress-testing", unix)))]
    async fn probe_hop_icmp_parallel(&self, _target: IpAddr, _ttl: u8) -> TracerouteHop {
        TracerouteHop::new(_ttl)
    }

    async fn reverse_dns(addr: &str) -> Result<String, TracerouteError> {
        let ip: IpAddr = addr
            .parse()
            .map_err(|e| TracerouteError::ResolveError(format!("Invalid IP '{addr}': {e}")))?;

        let mut opts = ResolverOpts::default();
        opts.timeout = Duration::from_secs(2);
        opts.attempts = 1;

        let resolver = TokioResolver::builder_with_config(
            ResolverConfig::default(),
            hickory_resolver::net::runtime::TokioRuntimeProvider::default(),
        )
        .with_options(opts)
        .build()
        .map_err(|e| TracerouteError::ResolveError(e.to_string()))?;

        let lookup = resolver
            .reverse_lookup(ip)
            .await
            .map_err(|e| TracerouteError::ResolveError(e.to_string()))?;

        let name = lookup
            .answers()
            .first()
            .map(|record| match &record.data {
                hickory_resolver::proto::rr::RData::PTR(ptr) => ptr.to_string(),
                data => data.to_string(),
            })
            .ok_or_else(|| TracerouteError::ResolveError("No PTR record found".to_string()))?;

        Ok(normalize_ptr_name(&name))
    }

    fn resolve_target(&self, target: &str) -> Result<IpAddr, TracerouteError> {
        if let Ok(ip) = target.parse::<IpAddr>() {
            return Ok(ip);
        }

        use std::net::ToSocketAddrs;
        let addrs: Vec<_> = (target, 0)
            .to_socket_addrs()
            .map_err(|e| TracerouteError::ResolveError(e.to_string()))?
            .collect();

        addrs
            .first()
            .map(|a| a.ip())
            .ok_or_else(|| TracerouteError::ResolveError(format!("Failed to resolve: {}", target)))
    }

    async fn probe_udp(&self, target: IpAddr, ttl: u8) -> Result<IpAddr, ProbeError> {
        use std::net::UdpSocket;

        let socket =
            UdpSocket::bind("0.0.0.0:0").map_err(|e| ProbeError::SocketError(e.to_string()))?;

        socket
            .set_read_timeout(Some(self.config.timeout))
            .map_err(|e| ProbeError::SocketError(e.to_string()))?;

        socket
            .set_ttl(ttl as u32)
            .map_err(|e| ProbeError::SocketError(e.to_string()))?;

        let packet = vec![0u8; self.config.packet_size];
        let dst = SocketAddr::new(target, self.config.port + ttl as u16 - 1);

        socket
            .send_to(&packet, dst)
            .map_err(|e| ProbeError::SendError(e.to_string()))?;

        let mut buf = [0u8; 1024];
        match socket.recv_from(&mut buf) {
            Ok((len, addr)) => {
                tracing::debug!("UDP response from {} ({} bytes)", addr, len);
                if addr.port() == self.config.port + ttl as u16 - 1 {
                    if buf[0] == 3 {
                        return Err(ProbeError::PortUnreachable);
                    }
                }
                Ok(addr.ip())
            }
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => Err(ProbeError::Timeout),
            Err(e) if e.kind() == std::io::ErrorKind::ConnectionRefused => Ok(target),
            Err(e) => Err(ProbeError::ReceiveError(e.to_string())),
        }
    }

    async fn probe_icmp(&self, target: IpAddr, ttl: u8) -> Result<IpAddr, ProbeError> {
        #[cfg(all(feature = "stress-testing", unix))]
        {
            let timeout = self.config.timeout;
            let payload = vec![0u8; self.config.packet_size];

            let result =
                tokio::time::timeout(timeout, async { surge_ping::ping(target, &payload).await })
                    .await;

            match result {
                Ok(Ok((_, rtt))) => {
                    tracing::debug!("ICMP response from {} in {:?}", target, rtt);
                    Ok(target)
                }
                Ok(Err(e)) => {
                    tracing::debug!("ICMP probe failed: {}", e);
                    Err(ProbeError::Timeout)
                }
                Err(_) => Err(ProbeError::Timeout),
            }
        }

        #[cfg(not(all(feature = "stress-testing", unix)))]
        {
            tracing::debug!("ICMP probe not available, using UDP fallback");
            self.probe_udp(target, ttl).await
        }
    }
}

fn normalize_ptr_name(name: &str) -> String {
    name.trim_end_matches('.').to_string()
}

#[derive(Debug, thiserror::Error)]
pub enum TracerouteError {
    #[error("Failed to resolve target: {0}")]
    ResolveError(String),
    #[error("Probe failed: {0}")]
    ProbeError(#[from] ProbeError),
    #[error("Requires root privileges for ICMP traceroute")]
    RequiresRoot,
    #[error("{0}")]
    Unsupported(String),
}

#[derive(Debug, thiserror::Error)]
pub enum ProbeError {
    #[error("Socket error: {0}")]
    SocketError(String),
    #[error("Send error: {0}")]
    SendError(String),
    #[error("Receive error: {0}")]
    ReceiveError(String),
    #[error("Timeout")]
    Timeout,
    #[error("Port unreachable")]
    PortUnreachable,
}

pub struct TracerouteBuilder {
    config: TracerouteConfig,
}

impl TracerouteBuilder {
    pub fn new() -> Self {
        Self {
            config: TracerouteConfig::default(),
        }
    }

    pub fn target(mut self, target: impl Into<String>) -> Self {
        self.config.target = target.into();
        self
    }

    pub fn max_hops(mut self, hops: u8) -> Self {
        self.config.max_hops = hops;
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = timeout;
        self
    }

    pub fn max_retries(mut self, retries: u8) -> Self {
        self.config.max_retries = retries;
        self
    }

    pub fn first_ttl(mut self, ttl: u8) -> Self {
        self.config.first_ttl = ttl;
        self
    }

    pub fn port(mut self, port: u16) -> Self {
        self.config.port = port;
        self
    }

    pub fn use_icmp(mut self, icmp: bool) -> Self {
        self.config.use_icmp = icmp;
        self
    }

    pub fn packet_size(mut self, size: usize) -> Self {
        self.config.packet_size = size;
        self
    }

    pub fn parallel(mut self, parallel: bool) -> Self {
        self.config.parallel_probes = parallel;
        self
    }

    pub fn resolve_names(mut self, resolve: bool) -> Self {
        self.config.resolve_names = resolve;
        self
    }

    pub fn build(self) -> Traceroute {
        Traceroute::new(self.config)
    }
}

impl Default for TracerouteBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn run_returns_unsupported_for_icmp_mode() {
        let traceroute = Traceroute::new(TracerouteConfig {
            target: "127.0.0.1".to_string(),
            use_icmp: true,
            ..TracerouteConfig::default()
        });

        let result = traceroute.run().await;
        match result {
            Err(TracerouteError::Unsupported(msg)) => {
                assert!(msg.contains("ICMP traceroute"));
            }
            other => unreachable!("expected Unsupported error, got: {other:?}"),
        }
    }

    #[test]
    fn normalize_ptr_name_trims_trailing_dot() {
        assert_eq!(normalize_ptr_name("example.com."), "example.com");
        assert_eq!(normalize_ptr_name("router.local"), "router.local");
    }
}
