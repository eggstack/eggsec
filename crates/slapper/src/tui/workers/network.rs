use crate::tui::workers::TaskResult;
#[cfg(feature = "stress-testing")]
use std::net::SocketAddr;
use std::time::Duration;

#[cfg(all(feature = "packet-inspection", unix))]
const PACKET_CAPTURE_IDLE_TIMEOUT: Duration = Duration::from_secs(5);

pub async fn run_load_test(
    target: String,
    requests: u64,
    concurrency: usize,
    timeout: Duration,
    progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    result_tx: tokio::sync::mpsc::Sender<TaskResult>,
) -> anyhow::Result<()> {
    use crate::loadtest::runner::LoadTestRunner;

    let runner =
        LoadTestRunner::new_with_tui_mode(target.clone(), requests, concurrency, timeout, true)?;

    if let Err(e) = progress_tx.send((0, requests)).await {
        tracing::warn!("Failed to send initial progress: {}", e);
    }

    let load_test_timeout = Duration::from_secs(300);
    let results = match tokio::time::timeout(load_test_timeout, runner.run()).await {
        Ok(Ok(r)) => r,
        Ok(Err(e)) => {
            tracing::error!("Load test failed: {}", e);
            return Err(anyhow::anyhow!("{}", e));
        }
        Err(_) => {
            tracing::error!("Load test timed out after {:?}", load_test_timeout);
            return Err(anyhow::anyhow!("Load test timed out after 300 seconds"));
        }
    };

    if let Err(e) = result_tx.send(TaskResult::LoadTest(results)).await {
        tracing::warn!("Failed to send load test results: {}", e);
    }
    if let Err(e) = progress_tx.send((requests, requests)).await {
        tracing::warn!("Failed to send progress: {}", e);
    }
    Ok(())
}

#[cfg(feature = "stress-testing")]
pub async fn run_stress_test(
    target: String,
    stress_type: String,
    rate: u64,
    duration: u64,
    concurrency: usize,
    progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    result_tx: tokio::sync::mpsc::Sender<TaskResult>,
) -> anyhow::Result<()> {
    use crate::stress::{StressConfig, StressTest, StressType};

    let stress_type = match stress_type.as_str() {
        "syn" => StressType::Syn,
        "udp" => StressType::Udp,
        "tcp" => StressType::Tcp,
        "icmp" => StressType::Icmp,
        _ => StressType::Http,
    };

    let (host, port) = parse_target_host_port(&target);

    let config = StressConfig {
        target: host,
        port,
        stress_type,
        rate_pps: rate,
        duration_secs: duration,
        concurrency,
        spoof_source: false,
        spoof_range: None,
        random_source_port: true,
        payload_size: 64,
        use_proxies: false,
        proxy_pool: None,
    };

    let test = StressTest::new(config)?;

    let stress_test_timeout = Duration::from_secs(600);
    let stats = match tokio::time::timeout(stress_test_timeout, test.run_non_interactive()).await {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => {
            tracing::error!("Stress test failed: {}", e);
            return Err(anyhow::anyhow!("{}", e));
        }
        Err(_) => {
            tracing::error!("Stress test timed out after {:?}", stress_test_timeout);
            return Err(anyhow::anyhow!("Stress test timed out after 600 seconds"));
        }
    };

    if let Err(e) = result_tx
        .send(TaskResult::StressTest {
            target: target.clone(),
            stats: stats.clone(),
        })
        .await
    {
        tracing::warn!("Failed to send stress test results: {}", e);
    }
    if let Err(e) = progress_tx.send((duration, duration)).await {
        tracing::warn!("Failed to send progress: {}", e);
    }
    Ok(())
}

#[cfg(not(feature = "stress-testing"))]
pub async fn run_stress_test(
    _target: String,
    _stress_type: String,
    _rate: u64,
    _duration: u64,
    _concurrency: usize,
    _progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    _result_tx: tokio::sync::mpsc::Sender<TaskResult>,
) -> anyhow::Result<()> {
    anyhow::bail!("Stress testing not enabled. Compile with --features stress-testing");
}

#[cfg(all(feature = "packet-inspection", unix))]
pub async fn run_packet_capture(
    interface: String,
    filter: String,
    max_packets: usize,
    output_file: Option<String>,
    progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    result_tx: tokio::sync::mpsc::Sender<TaskResult>,
) -> anyhow::Result<()> {
    use crate::packet::capture::CaptureBuilder;
    use pnet::datalink;

    let interfaces = datalink::interfaces();
    let iface = interfaces
        .into_iter()
        .find(|i| i.name == interface)
        .ok_or_else(|| anyhow::anyhow!("Interface not found: {}", interface))?;

    let mut builder = CaptureBuilder::new()
        .interface(iface.name.clone())
        .filter(filter)
        .promiscuous(true)
        .snapshot_len(65535)
        .timeout(std::time::Duration::from_secs(1))
        .max_packets(max_packets);

    if let Some(path) = output_file.clone() {
        builder = builder.save_to_file(path);
    }

    let capture = builder.build();

    let mut captured = 0;
    if let Err(e) = progress_tx.send((0, max_packets as u64)).await {
        tracing::warn!("Failed to send initial progress: {}", e);
    }

    let mut capture = capture;
    let running = capture.running();
    let (pkt_tx, mut pkt_rx) = tokio::sync::mpsc::channel(100);
    let handle = tokio::spawn(async move {
        tokio::time::timeout(std::time::Duration::from_secs(300), capture.start(pkt_tx)).await
    });

    loop {
        tokio::select! {
            packet = pkt_rx.recv() => {
                match packet {
                    Some(_packet) => {
                        captured += 1;
                        if let Err(e) = progress_tx
                            .send((captured as u64, max_packets as u64))
                            .await
                        {
                            tracing::warn!("Failed to send packet capture progress: {}", e);
                        }
                        if captured >= max_packets {
                            break;
                        }
                    }
                    None => break,
                }
            }
            _ = tokio::time::sleep(PACKET_CAPTURE_IDLE_TIMEOUT) => {
                tracing::warn!(
                    "Packet capture idle timeout - no packets received for {} seconds",
                    PACKET_CAPTURE_IDLE_TIMEOUT.as_secs()
                );
                running.store(false, std::sync::atomic::Ordering::SeqCst);
                break;
            }
        }
    }

running.store(false, std::sync::atomic::Ordering::SeqCst);
    let handle_result = tokio::time::timeout(Duration::from_secs(2), handle).await;
    match handle_result {
        Err(e) => {
            tracing::warn!("Packet capture handle timed out: {}", e);
            handle.abort();
        }
        Ok(Err(e)) => {
            if e.is_panic() {
                tracing::warn!("Packet capture task panicked: {:?}", e);
            } else {
                tracing::warn!("Packet capture task failed: {:?}", e);
            }
        }
        Ok(Ok(())) => {
            tracing::debug!("Packet capture task completed successfully");
        }
    }

    if let Err(e) = result_tx
        .send(TaskResult::PacketCapture {
            packets_captured: captured,
            output_file,
        })
        .await
    {
        tracing::warn!("Failed to send packet capture results: {}", e);
    }

    Ok(())
}

#[cfg(not(all(feature = "packet-inspection", unix)))]
pub async fn run_packet_capture(
    _interface: String,
    _filter: String,
    _max_packets: usize,
    _output_file: Option<String>,
    _progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    _result_tx: tokio::sync::mpsc::Sender<TaskResult>,
) -> anyhow::Result<()> {
    anyhow::bail!("Packet capture not available. Compile with --features packet-inspection");
}

#[cfg(all(feature = "stress-testing", unix))]
pub async fn run_packet_traceroute(
    target: String,
    max_hops: u8,
    progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    result_tx: tokio::sync::mpsc::Sender<TaskResult>,
) -> anyhow::Result<()> {
    use crate::packet::traceroute::{Traceroute, TracerouteConfig};
    use crate::tui::workers::TracerouteHopResult;
    let _socket_addr = tokio::net::lookup_host((target.as_str(), 80))
        .await
        .map_err(|e| anyhow::anyhow!("Invalid target: {}", e))?
        .next()
        .ok_or_else(|| anyhow::anyhow!("Could not resolve target"))?;

    let config = TracerouteConfig {
        target: target.clone(),
        max_hops,
        timeout: std::time::Duration::from_secs(3),
        max_retries: 3,
        first_ttl: 1,
        port: 33434,
        use_icmp: false,
        packet_size: 32,
        parallel_probes: true,
        resolve_names: true,
    };

    if let Err(e) = progress_tx.send((0, max_hops as u64)).await {
        tracing::warn!("Failed to send initial progress: {}", e);
    }

    let traceroute = Traceroute::new(config);
    let result = traceroute
        .run()
        .await
        .map_err(|e| anyhow::anyhow!("Traceroute failed: {}", e))?;

    let hops: Vec<TracerouteHopResult> = result
        .hops
        .iter()
        .map(|h| TracerouteHopResult {
            hop: h.hop,
            address: h.address.clone(),
            rtt_ms: h.rtt.map(|d| d.as_secs_f64() * 1000.0),
        })
        .collect();

    if let Err(e) = progress_tx.send((max_hops as u64, max_hops as u64)).await {
        tracing::warn!("Failed to send progress: {}", e);
    }

    if let Err(e) = result_tx.send(TaskResult::PacketTraceroute { hops }).await {
        tracing::warn!("Failed to send traceroute results: {}", e);
    }

    Ok(())
}

#[cfg(not(all(feature = "stress-testing", unix)))]
pub async fn run_packet_traceroute(
    _target: String,
    _max_hops: u8,
    _progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    _result_tx: tokio::sync::mpsc::Sender<TaskResult>,
) -> anyhow::Result<()> {
    anyhow::bail!("Traceroute not available. Compile with --features stress-testing");
}

#[cfg(feature = "stress-testing")]
fn parse_target_host_port(target: &str) -> (String, u16) {
    if let Ok(addr) = target.parse::<SocketAddr>() {
        return (addr.ip().to_string(), addr.port());
    }

    if target.contains("]:") {
        if let Some((host_part, port_part)) = target.rsplit_once("]:") {
            let host = host_part.trim_start_matches('[').to_string();
            let port = port_part.parse().unwrap_or_else(|_| 80);
            return (host, port);
        }
    }

    if target.matches(':').count() == 1 {
        if let Some((host, port_part)) = target.split_once(':') {
            return (host.to_string(), port_part.parse().unwrap_or_else(|_| 80));
        }
    }

    (target.to_string(), 80)
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "stress-testing")]
    use super::parse_target_host_port;

    #[cfg(feature = "stress-testing")]
    #[test]
    fn parse_target_host_port_ipv4() {
        let (host, port) = parse_target_host_port("127.0.0.1:8443");
        assert_eq!(host, "127.0.0.1");
        assert_eq!(port, 8443);
    }

    #[cfg(feature = "stress-testing")]
    #[test]
    fn parse_target_host_port_hostname() {
        let (host, port) = parse_target_host_port("example.com:443");
        assert_eq!(host, "example.com");
        assert_eq!(port, 443);
    }

    #[cfg(feature = "stress-testing")]
    #[test]
    fn parse_target_host_port_ipv6_with_port() {
        let (host, port) = parse_target_host_port("[2001:db8::1]:8080");
        assert_eq!(host, "2001:db8::1");
        assert_eq!(port, 8080);
    }

    #[cfg(feature = "stress-testing")]
    #[test]
    fn parse_target_host_port_ipv6_without_port() {
        let (host, port) = parse_target_host_port("2001:db8::1");
        assert_eq!(host, "2001:db8::1");
        assert_eq!(port, 80);
    }
}

#[cfg(all(feature = "stress-testing", unix))]
pub async fn run_packet_send(
    target: String,
    port: u16,
    count: u32,
    packet_size: usize,
    progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    result_tx: tokio::sync::mpsc::Sender<TaskResult>,
) -> anyhow::Result<()> {
    use std::time::Duration;

    tracing::warn!("Packet sending is a stub - raw socket implementation required for actual packet transmission");

    if let Err(e) = progress_tx.send((0, count as u64)).await {
        tracing::warn!("Failed to send initial progress: {}", e);
    }

    let mut sent = 0u32;
    let mut bytes = 0u64;

    for i in 0..count {
        tokio::time::sleep(Duration::from_millis(10)).await;
        sent += 1;
        bytes += packet_size as u64;
        if let Err(e) = progress_tx.send((sent as u64, count as u64)).await {
            tracing::warn!("Failed to send packet send progress: {}", e);
        }
        tracing::trace!("[STUB] Would send packet {} to {}:{}", i + 1, target, port);
    }

    if let Err(e) = result_tx
        .send(TaskResult::PacketSend {
            packets_sent: sent,
            bytes_sent: bytes,
        })
        .await
    {
        tracing::warn!("Failed to send packet send results: {}", e);
    }

    Ok(())
}

#[cfg(not(all(feature = "stress-testing", unix)))]
pub async fn run_packet_send(
    _target: String,
    _port: u16,
    _count: u32,
    _packet_size: usize,
    _progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    _result_tx: tokio::sync::mpsc::Sender<TaskResult>,
) -> anyhow::Result<()> {
    anyhow::bail!("Packet send not available. Compile with --features stress-testing");
}
