use crate::packet::types::*;

impl EthernetFrame {
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 14 {
            return None;
        }

        let dst_mac = format!(
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            data[0], data[1], data[2], data[3], data[4], data[5]
        );
        let src_mac = format!(
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            data[6], data[7], data[8], data[9], data[10], data[11]
        );
        let ether_type = u16::from_be_bytes([data[12], data[13]]);

        let ether_type_name = match ether_type {
            0x0800 => "IPv4".to_string(),
            0x86DD => "IPv6".to_string(),
            0x0806 => "ARP".to_string(),
            0x8100 => "VLAN".to_string(),
            _ => format!("0x{:04x}", ether_type),
        };

        Some(Self {
            dst_mac,
            src_mac,
            ether_type,
            ether_type_name,
        })
    }
}

impl IpPacket {
    pub fn parse_ipv4(data: &[u8]) -> Option<Self> {
        if data.len() < 20 {
            return None;
        }

        let version = (data[0] >> 4) & 0x0f;
        if version != 4 {
            return None;
        }

        let header_len = (data[0] & 0x0f) * 4;
        if data.len() < header_len as usize {
            return None;
        }

        let total_len = u16::from_be_bytes([data[2], data[3]]);
        let identification = u16::from_be_bytes([data[4], data[5]]);
        let flags_fragment = u16::from_be_bytes([data[6], data[7]]);
        let ttl = data[8];
        let protocol = data[9];
        let checksum = u16::from_be_bytes([data[10], data[11]]);

        let src_ip = format!("{}.{}.{}.{}", data[12], data[13], data[14], data[15]);
        let dst_ip = format!("{}.{}.{}.{}", data[16], data[17], data[18], data[19]);

        let flags = IpFlags {
            reserved: (flags_fragment & 0x8000) != 0,
            dont_fragment: (flags_fragment & 0x4000) != 0,
            more_fragments: (flags_fragment & 0x2000) != 0,
        };

        let protocol_name = match protocol {
            1 => "ICMP".to_string(),
            6 => "TCP".to_string(),
            17 => "UDP".to_string(),
            47 => "GRE".to_string(),
            50 => "ESP".to_string(),
            51 => "AH".to_string(),
            _ => format!("{}", protocol),
        };

        let options = if header_len > 20 {
            Self::parse_ip_options(&data[20..header_len as usize])
        } else {
            vec![]
        };

        let payload = data[header_len as usize..].to_vec();

        Some(Self {
            version,
            header_len,
            total_len,
            ttl,
            protocol,
            protocol_name,
            src_ip,
            dst_ip,
            payload,
            options,
            identification,
            flags,
            checksum,
        })
    }

    fn parse_ip_options(data: &[u8]) -> Vec<IpOption> {
        let mut options = Vec::new();
        let mut i = 0;

        while i < data.len() {
            let code = data[i];

            if code == 0 {
                break;
            }

            if code == 1 {
                options.push(IpOption {
                    code,
                    name: "NOP".to_string(),
                    length: None,
                    data: None,
                });
                i += 1;
                continue;
            }

            if i + 1 >= data.len() {
                break;
            }

            let len = data[i + 1] as usize;
            let name = match code {
                7 => "RR".to_string(),
                68 => "TS".to_string(),
                131 => "LSRR".to_string(),
                137 => "SSRR".to_string(),
                148 => "RTRALT".to_string(),
                _ => format!("Unknown({})", code),
            };

            let opt_data = if len > 2 && i + len <= data.len() {
                Some(data[i + 2..i + len].to_vec())
            } else {
                None
            };

            options.push(IpOption {
                code,
                name,
                length: Some(data[i + 1]),
                data: opt_data,
            });

            if len == 0 {
                break;
            }

            i += len;
        }

        options
    }

    pub fn parse_ipv6(data: &[u8]) -> Option<Self> {
        if data.len() < 40 {
            return None;
        }

        let version = (data[0] >> 4) & 0x0f;
        if version != 6 {
            return None;
        }

        let payload_len = u16::from_be_bytes([data[4], data[5]]);
        let next_header = data[6];
        let hop_limit = data[7];

        let src_ip = super::validation::format_ipv6(&data[8..24]);
        let dst_ip = super::validation::format_ipv6(&data[24..40]);

        let protocol_name = match next_header {
            6 => "TCP".to_string(),
            17 => "UDP".to_string(),
            58 => "ICMPv6".to_string(),
            _ => format!("{}", next_header),
        };

        let payload = if data.len() > 40 {
            data[40..].to_vec()
        } else {
            vec![]
        };

        Some(Self {
            version,
            header_len: 40,
            total_len: payload_len + 40,
            ttl: hop_limit,
            protocol: next_header,
            protocol_name,
            src_ip,
            dst_ip,
            payload,
            options: vec![],
            identification: 0,
            flags: IpFlags::default(),
            checksum: 0,
        })
    }
}

impl TcpHeader {
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 20 {
            return None;
        }

        let src_port = u16::from_be_bytes([data[0], data[1]]);
        let dst_port = u16::from_be_bytes([data[2], data[3]]);
        let seq_num = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
        let ack_num = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);
        let data_offset = (data[12] >> 4) * 4;
        let flags = TcpFlags::from_bits(data[13]);
        let window_size = u16::from_be_bytes([data[14], data[15]]);
        let checksum = u16::from_be_bytes([data[16], data[17]]);
        let urgent_ptr = u16::from_be_bytes([data[18], data[19]]);

        if data.len() < data_offset as usize {
            return None;
        }

        let options = if data_offset > 20 {
            Self::parse_tcp_options(&data[20..data_offset as usize])
        } else {
            vec![]
        };

        let payload = data[data_offset as usize..].to_vec();

        Some(Self {
            src_port,
            dst_port,
            seq_num,
            ack_num,
            data_offset,
            flags,
            window_size,
            checksum,
            urgent_ptr,
            payload,
            options,
        })
    }

    fn parse_tcp_options(data: &[u8]) -> Vec<TcpOption> {
        let mut options = Vec::new();
        let mut i = 0;

        while i < data.len() {
            let kind = data[i];

            if kind == 0 {
                options.push(TcpOption {
                    kind,
                    name: "EOL".to_string(),
                    length: None,
                    data: None,
                });
                break;
            }

            if kind == 1 {
                options.push(TcpOption {
                    kind,
                    name: "NOP".to_string(),
                    length: None,
                    data: None,
                });
                i += 1;
                continue;
            }

            if i + 1 >= data.len() {
                break;
            }

            let len = data[i + 1] as usize;
            let name = match kind {
                2 => "MSS".to_string(),
                3 => "WS".to_string(),
                4 => "SACK Permitted".to_string(),
                5 => "SACK".to_string(),
                8 => "TS".to_string(),
                14 => "TCP Alternate Checksum Request".to_string(),
                15 => "TCP Alternate Checksum Data".to_string(),
                19 => "MD5".to_string(),
                28 => "UT".to_string(),
                29 => "TCP Quick Start Response".to_string(),
                34 => "TCP Connection Recording".to_string(),
                _ => format!("Unknown({})", kind),
            };

            let opt_data = if len > 2 && i + len <= data.len() {
                Some(data[i + 2..i + len].to_vec())
            } else {
                None
            };

            options.push(TcpOption {
                kind,
                name,
                length: Some(data[i + 1]),
                data: opt_data,
            });

            if len == 0 {
                break;
            }

            i += len;
        }

        options
    }
}

impl UdpHeader {
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 8 {
            return None;
        }

        let src_port = u16::from_be_bytes([data[0], data[1]]);
        let dst_port = u16::from_be_bytes([data[2], data[3]]);
        let length = u16::from_be_bytes([data[4], data[5]]);
        let checksum = u16::from_be_bytes([data[6], data[7]]);

        let payload = data[8..].to_vec();

        Some(Self {
            src_port,
            dst_port,
            length,
            checksum,
            payload,
        })
    }
}

impl IcmpHeader {
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 8 {
            return None;
        }

        let icmp_type = data[0];
        let icmp_code = data[1];
        let checksum = u16::from_be_bytes([data[2], data[3]]);

        let type_name = match icmp_type {
            0 => "Echo Reply",
            3 => "Destination Unreachable",
            4 => "Source Quench",
            5 => "Redirect",
            8 => "Echo Request",
            11 => "Time Exceeded",
            12 => "Parameter Problem",
            13 => "Timestamp Request",
            14 => "Timestamp Reply",
            _ => "Unknown",
        };

        tracing::debug!("ICMP type: {} ({})", icmp_type, type_name);

        let payload = data[8..].to_vec();

        Some(Self {
            icmp_type,
            icmp_code,
            checksum,
            payload,
        })
    }
}

impl HttpRequest {
    pub fn parse(data: &[u8]) -> Option<Self> {
        let text = String::from_utf8_lossy(data);
        let lines: Vec<&str> = text.lines().collect();

        if lines.is_empty() {
            return None;
        }

        let request_line = lines[0];
        let parts: Vec<&str> = request_line.splitn(3, ' ').collect();
        if parts.len() < 3 {
            return None;
        }

        let method = parts[0].to_string();
        let uri = parts[1].to_string();
        let version = parts[2].to_string();

        let mut headers = Vec::new();
        let mut body_start = None;

        for (i, line) in lines.iter().skip(1).enumerate() {
            if line.is_empty() {
                body_start = Some(i + 1);
                break;
            }
            if let Some(colon_pos) = line.find(':') {
                let name = line[..colon_pos].trim().to_string();
                let value = line[colon_pos + 1..].trim().to_string();
                headers.push(HttpHeader { name, value });
            }
        }

        let body = body_start.and_then(|start| {
            let body_lines: Vec<&str> = lines.iter().skip(start).copied().collect();
            if body_lines.is_empty() {
                None
            } else {
                Some(body_lines.join("\n").into_bytes())
            }
        });

        Some(Self {
            method,
            uri,
            version,
            headers,
            body,
        })
    }
}

impl HttpResponse {
    pub fn parse(data: &[u8]) -> Option<Self> {
        let text = String::from_utf8_lossy(data);
        let lines: Vec<&str> = text.lines().collect();

        if lines.is_empty() {
            return None;
        }

        let status_line = lines[0];
        let parts: Vec<&str> = status_line.splitn(3, ' ').collect();
        if parts.len() < 3 {
            return None;
        }

        let version = parts[0].to_string();
        let status_code = parts[1].parse().ok()?;
        let reason_phrase = parts[2].to_string();

        let mut headers = Vec::new();
        let mut body_start = None;

        for (i, line) in lines.iter().skip(1).enumerate() {
            if line.is_empty() {
                body_start = Some(i + 1);
                break;
            }
            if let Some(colon_pos) = line.find(':') {
                let name = line[..colon_pos].trim().to_string();
                let value = line[colon_pos + 1..].trim().to_string();
                headers.push(HttpHeader { name, value });
            }
        }

        let body = body_start.and_then(|start| {
            let body_lines: Vec<&str> = lines.iter().skip(start).copied().collect();
            if body_lines.is_empty() {
                None
            } else {
                Some(body_lines.join("\n").into_bytes())
            }
        });

        Some(Self {
            version,
            status_code,
            reason_phrase,
            headers,
            body,
        })
    }
}

impl DnsRecord {
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 12 {
            return None;
        }

        let transaction_id = u16::from_be_bytes([data[0], data[1]]);
        let flags = u16::from_be_bytes([data[2], data[3]]);

        let is_response = (flags & 0x8000) != 0;
        let opcode = (flags >> 11) & 0x0f;
        let rcode = flags & 0x0f;

        let opcode_str = match opcode {
            0 => "QUERY",
            1 => "IQUERY",
            2 => "STATUS",
            _ => "UNKNOWN",
        };

        let flags_str = if is_response {
            format!("QR={}", opcode_str)
        } else {
            format!("QUERY, RCODE={}", rcode)
        };

        let questions_count = u16::from_be_bytes([data[4], data[5]]);
        let answers_count = u16::from_be_bytes([data[6], data[7]]);

        let query_type = if is_response { "RESPONSE" } else { "QUERY" };

        let mut questions = Vec::new();
        let mut answers = Vec::new();
        let mut offset = 12;

        for _ in 0..questions_count {
            if let Some((name, new_offset)) = super::validation::parse_dns_name(data, offset) {
                if new_offset + 4 > data.len() {
                    break;
                }
                let qtype = u16::from_be_bytes([data[new_offset], data[new_offset + 1]]);
                let qclass = u16::from_be_bytes([data[new_offset + 2], data[new_offset + 3]]);

                questions.push(DnsQuestion {
                    name,
                    query_type: super::validation::dns_type_to_string(qtype),
                    class: format!("{}", qclass),
                });
                offset = new_offset + 4;
            } else {
                break;
            }
        }

        for _ in 0..answers_count {
            if let Some((name, new_offset)) = super::validation::parse_dns_name(data, offset) {
                if new_offset + 10 > data.len() {
                    break;
                }
                let atype = u16::from_be_bytes([data[new_offset], data[new_offset + 1]]);
                let _aclass = u16::from_be_bytes([data[new_offset + 2], data[new_offset + 3]]);
                let ttl = u32::from_be_bytes([
                    data[new_offset + 4],
                    data[new_offset + 5],
                    data[new_offset + 6],
                    data[new_offset + 7],
                ]);
                let rdlen = u16::from_be_bytes([data[new_offset + 8], data[new_offset + 9]]);

                let rdata = if new_offset + 10 + rdlen as usize <= data.len() {
                    super::validation::parse_dns_rdata(data, new_offset + 10, atype, rdlen as usize)
                } else {
                    String::new()
                };

                answers.push(DnsAnswer {
                    name,
                    record_type: super::validation::dns_type_to_string(atype),
                    ttl,
                    data: rdata,
                });
                offset = new_offset + 10 + rdlen as usize;
            } else {
                break;
            }
        }

        Some(Self {
            transaction_id,
            flags: flags_str,
            query_type: query_type.to_string(),
            questions,
            answers,
        })
    }
}

impl TlsHandshake {
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 5 {
            return None;
        }

        if data[0] != 0x16 {
            return None;
        }

        if data[1] != 0x03 {
            return None;
        }

        let version = match data[3] {
            0x01 => "TLS 1.0",
            0x02 => "TLS 1.1",
            0x03 => "TLS 1.2",
            0x04 => "TLS 1.3",
            _ => "Unknown",
        };

        let handshake_type = match data[5] {
            0x01 => "ClientHello",
            0x02 => "ServerHello",
            0x0b => "Certificate",
            0x0c => "ServerKeyExchange",
            0x0d => "CertificateRequest",
            0x0e => "ServerHelloDone",
            0x0f => "CertificateVerify",
            0x10 => "ClientKeyExchange",
            0x14 => "Finished",
            _ => "Unknown",
        };

        Some(Self {
            handshake_type: handshake_type.to_string(),
            version: version.to_string(),
            client_hello: None,
            server_hello: None,
        })
    }
}

impl ParsedPacket {
    pub fn parse(data: &[u8]) -> Option<Self> {
        let mut offset = 0;

        let ethernet = if data.len() >= 14 {
            EthernetFrame::parse(&data[offset..]).map(|eth| {
                offset += EthernetFrame::header_len();
                eth
            })
        } else {
            None
        };

        let ip = if data.len() > offset {
            IpPacket::parse(&data[offset..]).map(|mut ip| {
                let ip_header_len = ip.header_len as usize;
                let payload_len = ip.payload.len();
                offset += ip_header_len;
                if payload_len > 0 && offset + payload_len <= data.len() {
                    ip.payload = data[offset..offset + payload_len].to_vec();
                } else {
                    ip.payload = data[offset..].to_vec();
                }
                offset += payload_len;
                ip
            })
        } else {
            None
        };

        let transport = if data.len() > offset {
            match ip.as_ref() {
                Some(ip_packet) => match ip_packet.protocol {
                    6 => TcpHeader::parse(&data[offset..]).and_then(|tcp| {
                        let tcp_len = tcp.data_offset as usize;
                        if offset + tcp_len > data.len() {
                            return None;
                        }
                        Some(TransportProtocol::Tcp(TcpHeader {
                            payload: data[offset + tcp_len..].to_vec(),
                            ..tcp
                        }))
                    }),
                    17 => UdpHeader::parse(&data[offset..]).map(|udp| TransportProtocol::Udp(udp)),
                    1 => {
                        IcmpHeader::parse(&data[offset..]).map(|icmp| TransportProtocol::Icmp(icmp))
                    }
                    _ => Some(TransportProtocol::Unknown(data[offset..].to_vec())),
                },
                None => Some(TransportProtocol::Unknown(data[offset..].to_vec())),
            }
        } else {
            None
        };

        let app = Self::parse_app_layer(&ip, &transport);

        Some(Self {
            ethernet,
            ip,
            transport,
            app,
        })
    }

    fn parse_app_layer(
        ip: &Option<IpPacket>,
        transport: &Option<TransportProtocol>,
    ) -> Option<AppLayer> {
        let payload = match transport {
            Some(TransportProtocol::Tcp(tcp)) => &tcp.payload,
            Some(TransportProtocol::Udp(udp)) => &udp.payload,
            _ => return None,
        };

        if payload.is_empty() {
            return None;
        }

        if let Some(ip_pkt) = ip {
            match ip_pkt.protocol {
                6 => {
                    if payload.len() > 20 {
                        let src_port = u16::from_be_bytes([payload[0], payload[1]]);
                        let dst_port = u16::from_be_bytes([payload[2], payload[3]]);

                        if dst_port == 80 || src_port == 80 || dst_port == 8080 || src_port == 8080
                        {
                            if let Some(http) = HttpRequest::parse(payload) {
                                return Some(AppLayer::Http(http));
                            }
                        }
                    }
                }
                17 => {
                    let dns_payload: Vec<u8> = match transport {
                        Some(TransportProtocol::Udp(udp)) => udp.payload.clone(),
                        _ => vec![],
                    };

                    if !dns_payload.is_empty() {
                        if let Some(dns) = DnsRecord::parse(&dns_payload) {
                            return Some(AppLayer::Dns(dns));
                        }
                    }
                }
                _ => {}
            }
        }

        if payload.len() >= 3 && payload[0] == 0x16 && payload[1] == 0x03 {
            if let Some(tls) = TlsHandshake::parse(payload) {
                return Some(AppLayer::Tls(tls));
            }
        }

        None
    }
}
