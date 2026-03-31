use crate::cli::{PacketCaptureArgs, PacketDumpArgs, PacketSendArgs, PacketTracerouteArgs};
use crate::packet::capture::list_interfaces;
use crate::packet::craft::{PacketBuilder, TcpFlags};
use crate::packet::hexdump;
use crate::packet::traceroute::{Traceroute, TracerouteResult};
use crate::packet::{CaptureConfig, PacketCapture, PacketInfo};
use anyhow::anyhow;
use std::fs::File;
use std::io::{BufReader, Read};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::Path;
use std::time::Duration;
use tokio::sync::mpsc;

#[cfg(all(feature = "packet-inspection", unix))]
use pnet::datalink::{self, Channel::Ethernet, Config, NetworkInterface};

pub async fn handle_packet_capture(
    args: PacketCaptureArgs,
    json: bool,
) -> Result<(), anyhow::Error> {
    #[cfg(not(all(feature = "packet-inspection", unix)))]
    {
        anyhow::bail!("Packet capture requires the 'packet-inspection' feature and Unix OS");
    }

    #[cfg(all(feature = "packet-inspection", unix))]
    {
        println!("Starting packet capture...");

        if args.interface.is_none() {
            println!("Available interfaces:");
            for iface in list_interfaces() {
                println!(
                    "  - {} ({}): {:?}",
                    iface.name,
                    iface.ips.join(", "),
                    iface.mac
                );
            }
            return Ok(());
        }

        let config = CaptureConfig {
            interface: args.interface.expect("checked for None above"),
            filter: args.filter,
            promiscuous: args.promiscuous,
            snapshot_len: 65535,
            timeout: Duration::from_secs(1),
            max_packets: args.max,
            save_to_file: args.output,
            validate_checksums: false,
        };

        let (tx, mut rx) = mpsc::channel(100);

        let mut capture = PacketCapture::new(config);
        let running = capture.running();

        let capture_handle = tokio::spawn(async move {
            let mut cap = capture;
            cap.start(tx).await
        });

        let max_packets = args.max.unwrap_or(100);
        let mut count = 0;

        println!("Capturing packets (Ctrl+C to stop)...\n");

        while let Some(packet) = rx.recv().await {
            print_packet(&packet);
            count += 1;

            if count >= max_packets {
                break;
            }
        }

        running.store(false, std::sync::atomic::Ordering::SeqCst);
        let stats = capture_handle.await??;

        println!("\n--- Capture Statistics ---");
        println!("Packets captured: {}", stats.packets_captured);
        println!("Bytes captured: {}", stats.bytes_captured);
        println!("Runtime: {} ms", stats.runtime_ms);

        Ok(())
    }
}

pub async fn handle_packet_send(args: PacketSendArgs, _json: bool) -> Result<(), anyhow::Error> {
    use crate::utils::is_root;
    use std::net::UdpSocket;

    let target: SocketAddr = args.target.parse()?;
    let src_ip: IpAddr = args
        .src_ip
        .map(|s| s.parse())
        .unwrap_or(Ok(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0))))?;

    if args.icmp {
        #[cfg(all(feature = "packet-inspection", unix))]
        {
            if is_root() {
                send_raw_icmp(&target, args.ttl.unwrap_or(64)).await?;
                return Ok(());
            } else {
                println!("Warning: ICMP with raw sockets requires root privileges.");
                println!("Falling back to UDP socket (ICMP packet data will be sent as UDP).");
            }
        }
        #[cfg(not(all(feature = "packet-inspection", unix)))]
        {
            println!("Warning: ICMP requires 'packet-inspection' feature and Unix OS.");
            println!("Falling back to UDP socket.");
        }
    }

    let packet_data = if args.icmp {
        build_icmp_packet()
    } else if args.udp {
        build_udp_packet(args.src_port.unwrap_or(40000), target.port())
    } else {
        build_tcp_packet(
            args.src_port.unwrap_or(40000),
            target.port(),
            args.flags.as_deref(),
        )
    };

    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.connect(target)?;

    socket.send(&packet_data)?;

    println!("Packet sent to {}", target);
    println!("Hex dump:");
    print!("{}", hexdump(&packet_data));

    Ok(())
}

#[cfg(all(feature = "packet-inspection", unix))]
async fn send_raw_icmp(target: &SocketAddr, ttl: u8) -> Result<(), anyhow::Error> {
    use pnet::datalink::{self, Channel::Ethernet, Config, NetworkInterface};
    use pnet::packet::icmp::echo_request::MutableEchoRequestPacket;
    use pnet::packet::icmp::{IcmpCode, IcmpTypes};
    use pnet::packet::ip::IpNextHeaderProtocols;
    use pnet_packet::ipv4::MutableIpv4Packet;
    use rand::Rng;

    let target_ip = match target.ip() {
        IpAddr::V4(ip) => ip,
        IpAddr::V6(_) => anyhow::bail!("IPv6 not supported for ICMP"),
    };

    let interface = find_network_interface()?;
    let (mut tx, _rx) = create_datalink_channel(&interface)?;

    let src_ip = get_interface_ip(&interface)?;

    let payload_size = 56;
    let mut rng = rand::thread_rng();
    let payload: Vec<u8> = (0..payload_size).map(|_| rng.gen()).collect();

    let icmp_len = 8 + payload.len();
    let total_len = 20 + icmp_len;
    let mut buffer = vec![0u8; total_len];

    let mut ipv4_packet = MutableIpv4Packet::new(&mut buffer[..20])
        .ok_or_else(|| anyhow!("Failed to create IPv4 packet"))?;
    ipv4_packet.set_version(4);
    ipv4_packet.set_header_length(5);
    ipv4_packet.set_total_length(total_len as u16);
    ipv4_packet.set_ttl(ttl);
    ipv4_packet.set_next_level_protocol(IpNextHeaderProtocols::Icmp);
    ipv4_packet.set_source(src_ip);
    ipv4_packet.set_destination(target_ip);

    let mut icmp_packet = MutableEchoRequestPacket::new(&mut buffer[20..])
        .ok_or_else(|| anyhow!("Failed to create ICMP packet"))?;
    icmp_packet.set_icmp_type(IcmpTypes::EchoRequest);
    icmp_packet.set_icmp_code(IcmpCode(0));
    icmp_packet.set_identifier(rng.gen());
    icmp_packet.set_sequence_number(1);
    icmp_packet.set_payload(payload.as_slice());
    icmp_packet.set_checksum(0);

    match tx.send_to(&buffer, Some(interface.clone())) {
        Some(Ok(_)) => {
            println!("Raw ICMP packet sent to {}", target);
            println!("Hex dump:");
            print!("{}", hexdump(&buffer));
        }
        Some(Err(e)) => anyhow::bail!("Failed to send ICMP packet: {}", e),
        None => anyhow::bail!("Failed to send ICMP packet"),
    }

    Ok(())
}

#[cfg(all(feature = "packet-inspection", unix))]
fn find_network_interface() -> Result<NetworkInterface, anyhow::Error> {
    use pnet::datalink::interfaces;

    interfaces()
        .into_iter()
        .find(|iface| iface.is_up() && !iface.is_loopback() && !iface.ips.is_empty())
        .ok_or_else(|| anyhow!("No suitable network interface found"))
}

#[cfg(all(feature = "packet-inspection", unix))]
fn create_datalink_channel(
    interface: &NetworkInterface,
) -> Result<
    (
        Box<dyn datalink::DataLinkSender>,
        Box<dyn datalink::DataLinkReceiver>,
    ),
    anyhow::Error,
> {
    use pnet::datalink::Channel::Ethernet;

    let config = Config::default();
    match datalink::channel(interface, config) {
        Ok(Ethernet(tx, rx)) => Ok((tx, rx)),
        Ok(_) => Err(anyhow!("Unsupported channel type")),
        Err(e) => Err(anyhow!("Failed to create channel: {}", e)),
    }
}

#[cfg(all(feature = "packet-inspection", unix))]
fn get_interface_ip(interface: &NetworkInterface) -> Result<Ipv4Addr, anyhow::Error> {
    interface
        .ips
        .iter()
        .find_map(|ip| match ip.ip() {
            IpAddr::V4(ip) => Some(ip),
            _ => None,
        })
        .ok_or_else(|| anyhow!("No IPv4 address found on interface"))
}

pub fn handle_packet_dump(args: PacketDumpArgs, json: bool) -> Result<(), anyhow::Error> {
    let path = Path::new(&args.file);

    if !path.exists() {
        anyhow::bail!("File not found: {}", args.file);
    }

    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    if let Some(ext) = path.extension() {
        if ext == "pcap" {
            return dump_pcap(&mut reader, args, json);
        }
    }

    let mut data = Vec::new();
    reader.read_to_end(&mut data)?;

    dump_raw_packet(&data, args, json)
}

fn dump_pcap(
    reader: &mut BufReader<File>,
    args: PacketDumpArgs,
    json: bool,
) -> Result<(), anyhow::Error> {
    use std::io::Seek;

    let mut header = [0u8; 24];
    reader.read_exact(&mut header)?;

    let magic = u32::from_le_bytes([header[0], header[1], header[2], header[3]]);
    let is_nano = magic == 0xa1b23c4d || magic == 0xa1b2c3d4;

    let packets: Vec<(u32, Vec<u8>)> = if is_nano { vec![] } else { vec![] };

    reader.seek(std::io::SeekFrom::Start(24))?;

    let bytes_per_line = args.bytes_per_line.unwrap_or(16);
    let max_count = args.count.unwrap_or(usize::MAX);
    let index = args.index;

    let mut packet_data = Vec::new();
    let mut count = 0;

    loop {
        let mut pkt_header = [0u8; 16];
        match reader.read_exact(&mut pkt_header) {
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e.into()),
        }

        let ts_sec =
            u32::from_le_bytes([pkt_header[0], pkt_header[1], pkt_header[2], pkt_header[3]]);
        let incl_len = u32::from_le_bytes([
            pkt_header[12],
            pkt_header[13],
            pkt_header[14],
            pkt_header[15],
        ]);

        if incl_len as usize > 65535 {
            break;
        }

        let mut payload = vec![0u8; incl_len as usize];
        reader.read_exact(&mut payload)?;

        if let Some(idx) = index {
            if count == idx {
                packet_data = payload;
                break;
            }
        } else if count < max_count {
            print!("\n=== Packet {} ===\n", count);
            print!("Timestamp: {}\n", ts_sec);

            if !args.hex_only {
                if let Some(info) = parse_and_print_packet(&payload) {
                    if args.headers_only {
                        println!("{}", info);
                    } else {
                        println!("{}\n", info);
                        println!(
                            "{}",
                            hexdump::hexdump_with_offset(&payload, 0, bytes_per_line)
                        );
                    }
                } else {
                    println!(
                        "{}",
                        hexdump::hexdump_with_offset(&payload, 0, bytes_per_line)
                    );
                }
            } else {
                println!(
                    "{}",
                    hexdump::hexdump_with_offset(&payload, 0, bytes_per_line)
                );
            }
        }

        count += 1;
    }

    if let Some(idx) = index {
        if !packet_data.is_empty() {
            if !args.hex_only {
                if let Some(info) = parse_and_print_packet(&packet_data) {
                    println!("{}", info);
                    println!(
                        "\n{}",
                        hexdump::hexdump_with_offset(&packet_data, 0, bytes_per_line)
                    );
                }
            } else {
                println!(
                    "{}",
                    hexdump::hexdump_with_offset(&packet_data, 0, bytes_per_line)
                );
            }
        }
    }

    println!("\nTotal packets: {}", count);
    Ok(())
}

fn dump_raw_packet(data: &[u8], args: PacketDumpArgs, json: bool) -> Result<(), anyhow::Error> {
    let bytes_per_line = args.bytes_per_line.unwrap_or(16);

    if let Some(idx) = args.index {
        let start = idx * bytes_per_line;
        let end = start + bytes_per_line;
        let chunk = &data[start.min(data.len())..end.min(data.len())];

        println!("Packet at index {}:", idx);
        if !args.hex_only {
            if let Some(info) = parse_and_print_packet(chunk) {
                println!("{}", info);
            }
        }
        println!(
            "{}",
            hexdump::hexdump_with_offset(chunk, start, bytes_per_line)
        );
    } else {
        let chunks: Vec<&[u8]> = data.chunks(bytes_per_line).collect();
        for (i, chunk) in chunks.iter().enumerate() {
            let offset = i * bytes_per_line;
            print!("\n=== Packet {} ===\n", i);

            if !args.hex_only {
                if let Some(info) = parse_and_print_packet(chunk) {
                    if args.headers_only {
                        println!("{}", info);
                    } else {
                        println!("{}\n", info);
                        println!(
                            "{}",
                            hexdump::hexdump_with_offset(chunk, offset, bytes_per_line)
                        );
                    }
                } else {
                    println!(
                        "{}",
                        hexdump::hexdump_with_offset(chunk, offset, bytes_per_line)
                    );
                }
            } else {
                println!(
                    "{}",
                    hexdump::hexdump_with_offset(chunk, offset, bytes_per_line)
                );
            }
        }
    }

    Ok(())
}

pub async fn handle_packet_traceroute(
    args: PacketTracerouteArgs,
    json: bool,
) -> Result<(), anyhow::Error> {
    use crate::utils::is_root;

    let use_icmp = args.icmp;

    if use_icmp {
        #[cfg(any(feature = "stress-testing", feature = "packet-inspection"))]
        {
            #[cfg(any(
                all(feature = "packet-inspection", unix),
                all(feature = "stress-testing", unix)
            ))]
            {
                if !is_root() {
                    println!("Warning: ICMP traceroute requires root privileges.");
                    println!("Falling back to UDP traceroute.");
                } else {
                    println!("Using ICMP Echo Request for traceroute (requires root).");
                }
            }
            #[cfg(not(any(
                all(feature = "packet-inspection", unix),
                all(feature = "stress-testing", unix)
            )))]
            {
                println!("Warning: ICMP traceroute requires root/sudo on Unix.");
                println!("Falling back to UDP traceroute.");
            }
        }
        #[cfg(not(any(feature = "stress-testing", feature = "packet-inspection")))]
        {
            println!("Warning: ICMP traceroute requires 'stress-testing' or 'packet-inspection' feature.");
            println!("Falling back to UDP traceroute.");
        }
    }

    let config = crate::packet::traceroute::TracerouteConfig {
        target: args.target.clone(),
        max_hops: args.max_hops,
        timeout: Duration::from_secs(args.timeout.unwrap_or(3)),
        max_retries: args.probes,
        first_ttl: args.first_ttl.unwrap_or(1),
        port: 33434,
        use_icmp: use_icmp && is_root(),
        packet_size: 32,
        parallel_probes: args.parallel,
        resolve_names: !args.no_resolve,
    };

    let traceroute = Traceroute::new(config);
    let result = traceroute.run().await?;

    print_traceroute_result(&result, json);

    Ok(())
}

pub fn handle_packet_interfaces() -> Result<(), anyhow::Error> {
    let interfaces = list_interfaces();

    if interfaces.is_empty() {
        println!("No network interfaces found (requires packet-inspection feature and root)");
        return Ok(());
    }

    println!("Available network interfaces:\n");
    for iface in interfaces {
        println!("  {}", iface.name);
        println!("    IPs: {}", iface.ips.join(", "));
        if let Some(mac) = iface.mac {
            println!("    MAC: {}", mac);
        }
        println!(
            "    Status: {} {}",
            if iface.is_up { "UP" } else { "DOWN" },
            if iface.is_loopback { "(loopback)" } else { "" }
        );
        println!();
    }

    Ok(())
}

fn print_packet(info: &PacketInfo) {
    println!("\n{}", info.summary());
    println!("  Size: {} bytes", info.raw_size);
    println!("{}", info.hex_dump);
}

fn parse_and_print_packet(data: &[u8]) -> Option<String> {
    use crate::packet::parse::ParsedPacket;

    let parsed = ParsedPacket::parse(data)?;
    let mut output = String::new();

    if let Some(ref eth) = parsed.ethernet {
        output.push_str(&format!("Ethernet: {} → {}\n", eth.src_mac, eth.dst_mac));
    }

    if let Some(ref ip) = parsed.ip {
        output.push_str(&format!(
            "{}: {} → {}\n",
            ip.protocol_name,
            ip.src_ip(),
            ip.dst_ip()
        ));
        output.push_str(&format!("  TTL: {}\n", ip.ttl));
    }

    if let Some(ref trans) = parsed.transport {
        match trans {
            crate::packet::TransportProtocol::Tcp(tcp) => {
                output.push_str(&format!("TCP: {} → {}\n", tcp.src_port, tcp.dst_port));
                output.push_str(&format!("  Seq: {}, Ack: {}\n", tcp.seq_num, tcp.ack_num));
                output.push_str(&format!("  Flags: {}\n", tcp.flags.to_string()));
            }
            crate::packet::TransportProtocol::Udp(udp) => {
                output.push_str(&format!("UDP: {} → {}\n", udp.src_port, udp.dst_port));
            }
            crate::packet::TransportProtocol::Icmp(icmp) => {
                output.push_str(&format!(
                    "ICMP: type={}, code={}\n",
                    icmp.icmp_type, icmp.icmp_code
                ));
            }
            crate::packet::TransportProtocol::Unknown(_) => {}
        }
    }

    if let Some(ref app) = parsed.app {
        match app {
            crate::packet::AppLayer::Http(req) => {
                output.push_str(&format!(
                    "HTTP: {} {} {}\n",
                    req.method, req.uri, req.version
                ));
            }
            crate::packet::AppLayer::Dns(dns) => {
                output.push_str(&format!(
                    "DNS: {} ({})\n",
                    dns.transaction_id, dns.query_type
                ));
            }
            crate::packet::AppLayer::Tls(tls) => {
                output.push_str(&format!("TLS: {} {}\n", tls.handshake_type, tls.version));
            }
            crate::packet::AppLayer::Unknown => {}
        }
    }

    Some(output)
}

fn build_tcp_packet(src_port: u16, dst_port: u16, flags_str: Option<&str>) -> Vec<u8> {
    let mut flags = TcpFlags::syn();

    if let Some(f) = flags_str {
        for part in f.split(',') {
            match part.to_lowercase().as_str() {
                "syn" => flags.syn = true,
                "ack" => flags.ack = true,
                "rst" => flags.rst = true,
                "fin" => flags.fin = true,
                "psh" => flags.psh = true,
                "urg" => flags.urg = true,
                _ => {}
            }
        }
    }

    let packet = PacketBuilder::new()
        .ipv4(Ipv4Addr::new(0, 0, 0, 0), Ipv4Addr::new(0, 0, 0, 0), 6, 64)
        .tcp(src_port, dst_port, 1000, 0, flags, 65535)
        .build();

    packet
}

fn build_udp_packet(src_port: u16, dst_port: u16) -> Vec<u8> {
    let packet = PacketBuilder::new()
        .ipv4(Ipv4Addr::new(0, 0, 0, 0), Ipv4Addr::new(0, 0, 0, 0), 17, 64)
        .udp(src_port, dst_port)
        .build();

    packet
}

fn build_icmp_packet() -> Vec<u8> {
    let packet = PacketBuilder::new()
        .ipv4(Ipv4Addr::new(0, 0, 0, 0), Ipv4Addr::new(0, 0, 0, 0), 1, 64)
        .icmp(8, 0, 1, 1)
        .build();

    packet
}

fn print_traceroute_result(result: &TracerouteResult, json: bool) {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(result).unwrap_or_default()
        );
        return;
    }

    println!(
        "traceroute to {} ({}), {} hops max\n",
        result.target,
        result.resolved_address,
        result.hops.len()
    );

    for hop in &result.hops {
        print!("{:2}  ", hop.hop);

        if let Some(ref addr) = hop.address {
            if hop.is_final {
                print!("{}  ", addr);
            } else {
                print!("{}  ", addr);
            }
        } else {
            print!("*  ");
        }

        if let Some(rtt) = hop.rtt_ms {
            print!("{:.2} ms", rtt);
        }

        if hop.is_final {
            print!(" [final]");
        }

        println!();
    }

    if result.success {
        println!("\nTrace complete.");
    } else {
        println!("\nTrace incomplete (destination not reached).");
    }
}
