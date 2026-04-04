use crate::tui::workers::TaskResult;
use std::time::Duration;

pub async fn run_load_test(
    target: String,
    requests: u64,
    concurrency: usize,
    timeout: Duration,
    progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    result_tx: tokio::sync::mpsc::Sender<TaskResult>,
) -> anyhow::Result<()> {
    use crate::loadtest::runner::LoadTestRunner;

    let runner = LoadTestRunner::new_with_tui_mode(
        target.clone(),
        requests,
        concurrency,
        timeout,
        true,
    )?;

    let results = runner.run().await?;
    let _ = result_tx.send(TaskResult::LoadTest(results)).await;
    let _ = progress_tx.send((requests, requests)).await;
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

    let (host, port) = if target.contains(':') {
        let parts: Vec<&str> = target.splitn(2, ':').collect();
        (parts[0].to_string(), parts[1].parse().unwrap_or(80))
    } else {
        (target.clone(), 80)
    };

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
    let stats = test.run().await?;

    let _ = result_tx
        .send(TaskResult::StressTest {
            target: target.clone(),
            stats: stats.clone(),
        })
        .await;
    let _ = progress_tx.send((duration, duration)).await;
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

    let capture = CaptureBuilder::new()
        .interface(iface.name.clone())
        .filter(filter)
        .promiscuous(true)
        .snapshot_len(65535)
        .timeout(std::time::Duration::from_secs(1))
        .max_packets(max_packets)
        .build();

    let mut captured = 0;
    let _ = progress_tx.send((0, max_packets as u64)).await;

    let mut capture = capture;
    let (pkt_tx, mut pkt_rx) = tokio::sync::mpsc::channel(100);
    let handle = tokio::spawn(async move {
        capture.start(pkt_tx).await
    });

    let timeout_duration = Duration::from_secs(5);
    loop {
        tokio::select! {
            packet = pkt_rx.recv() => {
                match packet {
                    Some(_packet) => {
                        captured += 1;
                        let _ = progress_tx
                            .send((captured as u64, max_packets as u64))
                            .await;
                        if captured >= max_packets {
                            break;
                        }
                    }
                    None => break,
                }
            }
            _ = tokio::time::sleep(timeout_duration) => {
                tracing::warn!("Packet capture timeout - no packets received for {} seconds", timeout_duration.as_secs());
                break;
            }
        }
    }

    let _ = handle.await;

    let _ = result_tx
        .send(TaskResult::PacketCapture {
            packets_captured: captured,
            output_file,
        })
        .await;

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
    let addr = format!("{}:80", target);
    let _socket_addr = tokio::net::lookup_host(addr.as_str())
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

    let _ = progress_tx.send((0, max_hops as u64)).await;

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

    let _ = progress_tx.send((max_hops as u64, max_hops as u64)).await;

    let _ = result_tx.send(TaskResult::PacketTraceroute { hops }).await;

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

#[cfg(all(feature = "stress-testing", unix))]
pub async fn run_packet_send(
    target: String,
    port: u16,
    count: u32,
    packet_size: usize,
    progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    result_tx: tokio::sync::mpsc::Sender<TaskResult>,
) -> anyhow::Result<()> {
    let _ = progress_tx.send((0, count as u64)).await;

    let mut sent = 0u32;
    let mut bytes = 0u64;

    for _ in 0..count {
        sent += 1;
        bytes += packet_size as u64;
        let _ = progress_tx.send((sent as u64, count as u64)).await;
    }

    let _ = result_tx
        .send(TaskResult::PacketSend {
            packets_sent: sent,
            bytes_sent: bytes,
        })
        .await;

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