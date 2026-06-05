use smallvec::SmallVec;

pub fn format_ipv6(bytes: &[u8]) -> String {
    let parts: Vec<String> = (0..8)
        .map(|i| format!("{:x}", u16::from_be_bytes([bytes[i * 2], bytes[i * 2 + 1]])))
        .collect();
    parts.join(":")
}

pub fn parse_dns_name(data: &[u8], offset: usize) -> Option<(String, usize)> {
    let mut name = SmallVec::<[u8; 128]>::new();
    let mut pos = offset;
    let mut jumped = false;
    let mut jumps = 0;
    let original_offset = offset;

    while pos < data.len() {
        let length = data[pos] as usize;

        if length == 0 {
            return Some((
                String::from_utf8_lossy(&name).to_string(),
                if !jumped { pos + 1 } else { original_offset },
            ));
        }

        if (length & 0xc0) == 0xc0 {
            if pos + 1 >= data.len() {
                return None;
            }
            let new_offset = ((length & 0x3f) as usize) << 8 | data[pos + 1] as usize;
            if jumps == 0 {
                jumps = pos - original_offset + 2;
            }
            pos = new_offset;
            jumped = true;
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

        name.extend_from_slice(&data[label_start..label_end]);
        pos = label_end;
    }

    Some((String::from_utf8_lossy(&name).to_string(), pos))
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
        2 | 5 | 12 | 15 | 16 => {
            if let Some((name, _)) = parse_dns_name(data, offset) {
                name
            } else {
                String::new()
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
