use smallvec::SmallVec;

pub fn format_ipv6(bytes: &[u8]) -> String {
    bytes
        .get(..16)
        .and_then(|bytes| <[u8; 16]>::try_from(bytes).ok())
        .map_or_else(String::new, |bytes| {
            std::net::Ipv6Addr::from(bytes).to_string()
        })
}

pub fn parse_dns_name(data: &[u8], offset: usize) -> Option<(String, usize)> {
    let mut name = SmallVec::<[u8; 128]>::new();
    let mut pos = offset;
    let mut jump_end = None;
    let mut jumps = 0;

    while pos < data.len() {
        let length = data[pos] as usize;

        if length == 0 {
            return Some((
                String::from_utf8_lossy(&name).to_string(),
                jump_end.unwrap_or(pos + 1),
            ));
        }

        if (length & 0xc0) == 0xc0 {
            if pos + 1 >= data.len() {
                return None;
            }
            let new_offset = ((length & 0x3f) as usize) << 8 | data[pos + 1] as usize;
            if new_offset >= data.len() {
                return None;
            }
            jump_end.get_or_insert(pos + 2);
            pos = new_offset;
            jumps += 1;
            if jumps > 100 {
                return None;
            }
            continue;
        }

        if !name.is_empty() {
            name.push(b'.');
        }

        let label_start = pos + 1;
        let label_end = label_start + length;
        if label_end > data.len() {
            return None;
        }

        let separator_len = if name.is_empty() { 0 } else { 1 };
        if name.len() + separator_len + length > 255 {
            return None;
        }
        name.extend_from_slice(&data[label_start..label_end]);
        pos = label_end;
    }

    None
}

#[cfg(test)]
mod tests {
    use super::{format_ipv6, parse_dns_name};

    #[test]
    fn compressed_name_returns_offset_after_pointer() {
        let data = [3, b'w', b'w', b'w', 0, 0xc0, 0];
        let (name, next) = parse_dns_name(&data, 5).unwrap();
        assert_eq!(name, "www");
        assert_eq!(next, 7);
    }

    #[test]
    fn unterminated_name_is_rejected() {
        assert_eq!(parse_dns_name(&[3, b'w', b'w', b'w'], 0), None);
    }

    #[test]
    fn ipv6_uses_rfc5952_compression() {
        assert_eq!(format_ipv6(&[0; 16]), "::");
        assert_eq!(
            format_ipv6(&[0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]),
            "2001:db8::1"
        );
    }
}

pub fn parse_dns_rdata(data: &[u8], offset: usize, rtype: u16, _rdlen: usize) -> String {
    match rtype {
        1 => {
            if offset + 4 <= data.len() {
                format!(
                    "{}.{}.{}.{}",
                    data[offset],
                    data[offset + 1],
                    data[offset + 2],
                    data[offset + 3]
                )
            } else {
                String::new()
            }
        }
        2 | 5 | 12 => {
            if let Some((name, _)) = parse_dns_name(data, offset) {
                name
            } else {
                String::new()
            }
        }
        15 => {
            // MX: 2-byte preference + domain name
            if offset + 2 >= data.len() {
                return String::new();
            }
            let preference = u16::from_be_bytes([data[offset], data[offset + 1]]);
            if let Some((name, _)) = parse_dns_name(data, offset + 2) {
                format!("{} {}", preference, name)
            } else {
                String::new()
            }
        }
        16 => {
            // TXT: one or more <1-byte len><text> blocks
            let mut texts = Vec::new();
            let mut pos = offset;
            while pos < data.len() {
                let txt_len = data[pos] as usize;
                pos += 1;
                if pos + txt_len > data.len() {
                    break;
                }
                texts.push(String::from_utf8_lossy(&data[pos..pos + txt_len]).to_string());
                pos += txt_len;
            }
            if texts.is_empty() {
                String::new()
            } else {
                texts.join(" ")
            }
        }
        28 => {
            if offset + 16 <= data.len() {
                format_ipv6(&data[offset..offset + 16])
            } else {
                String::new()
            }
        }
        _ => {
            format!("{} bytes", _rdlen)
        }
    }
}

pub fn dns_type_to_string(qtype: u16) -> String {
    match qtype {
        1 => "A".to_string(),
        2 => "NS".to_string(),
        5 => "CNAME".to_string(),
        6 => "SOA".to_string(),
        12 => "PTR".to_string(),
        15 => "MX".to_string(),
        16 => "TXT".to_string(),
        28 => "AAAA".to_string(),
        33 => "SRV".to_string(),
        _ => format!("TYPE{}", qtype),
    }
}
