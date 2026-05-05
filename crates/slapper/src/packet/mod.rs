pub mod capture;
pub mod craft;
pub mod hexdump;
pub mod parse_impl;
pub mod traceroute;
pub mod types;
pub mod validation;

pub use capture::{CaptureBuilder, CaptureConfig, CaptureError, CaptureStats, PacketCapture};
pub use craft::PacketBuilder;
pub use hexdump::{hexdump, hexdump_with_offset};
pub use traceroute::{TracerouteConfig, TracerouteError, TracerouteHop, TracerouteResult};
pub use types::ParsedPacket;
pub use types::{
    AppLayer, DnsAnswer, DnsQuestion, DnsRecord, EthernetFrame, HttpHeader, HttpRequest,
    HttpResponse, IcmpHeader, IpFlags, IpOption, IpPacket, TcpFlags, TcpHeader, TcpOption,
    TlsClientHello, TlsHandshake, TlsServerHello, TransportProtocol, UdpHeader,
};

#[cfg(feature = "packet-inspection")]
pub mod cli;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacketInfo {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub ethernet: Option<EthernetFrame>,
    pub ip: Option<IpPacket>,
    pub transport: Option<TransportProtocol>,
    pub app: Option<AppLayer>,
    pub raw_size: usize,
    pub hex_dump: String,
}

impl PacketInfo {
    pub fn summary(&self) -> String {
        let mut parts = Vec::new();

        if let Some(ref eth) = self.ethernet {
            parts.push(format!("{} → {}", eth.src_mac, eth.dst_mac));
        }

        if let Some(ref ip) = self.ip {
            parts.push(format!("{} → {}", ip.src_ip(), ip.dst_ip()));
        }

        if let Some(ref trans) = self.transport {
            match trans {
                TransportProtocol::Tcp(tcp) => {
                    parts.push(format!("TCP {} → {}", tcp.src_port, tcp.dst_port));
                    if tcp.flags.syn {
                        parts.push("SYN".to_string());
                    }
                    if tcp.flags.ack {
                        parts.push("ACK".to_string());
                    }
                    if tcp.flags.fin {
                        parts.push("FIN".to_string());
                    }
                    if tcp.flags.rst {
                        parts.push("RST".to_string());
                    }
                }
                TransportProtocol::Udp(udp) => {
                    parts.push(format!("UDP {} → {}", udp.src_port, udp.dst_port));
                }
                TransportProtocol::Icmp(icmp) => {
                    parts.push(format!("ICMP type:{}", icmp.icmp_type));
                }
                TransportProtocol::Unknown(_) => {}
            }
        }

        if let Some(ref app) = self.app {
            match app {
                AppLayer::Http(req) => {
                    parts.push(format!("HTTP {}", req.method));
                }
                AppLayer::Dns(dns) => {
                    parts.push(format!("DNS {} {}", dns.transaction_id, dns.query_type));
                }
                AppLayer::Tls(tls) => {
                    parts.push(format!("TLS {}", tls.handshake_type));
                }
                AppLayer::Unknown => {}
            }
        }

        parts.join(" | ")
    }
}
