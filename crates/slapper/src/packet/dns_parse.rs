use crate::packet::types::*;
use crate::packet::validation;

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
            if let Some((name, new_offset)) = validation::parse_dns_name(data, offset) {
                if new_offset + 4 > data.len() {
                    break;
                }
                let qtype = u16::from_be_bytes([data[new_offset], data[new_offset + 1]]);
                let qclass = u16::from_be_bytes([data[new_offset + 2], data[new_offset + 3]]);

                questions.push(DnsQuestion {
                    name,
                    query_type: validation::dns_type_to_string(qtype),
                    class: format!("{}", qclass),
                });
                offset = new_offset + 4;
            } else {
                break;
            }
        }

        for _ in 0..answers_count {
            if let Some((name, new_offset)) = validation::parse_dns_name(data, offset) {
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
                    validation::parse_dns_rdata(data, new_offset + 10, atype, rdlen as usize)
                } else {
                    String::new()
                };

                answers.push(DnsAnswer {
                    name,
                    record_type: validation::dns_type_to_string(atype),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_data() {
        assert!(DnsRecord::parse(&[]).is_none());
        assert!(DnsRecord::parse(&[0u8; 11]).is_none());
    }

    #[test]
    fn test_parse_dns_query() {
        let mut data = vec![0u8; 32];
        data[0..2].copy_from_slice(&1u16.to_be_bytes());
        data[2..4].copy_from_slice(&0u16.to_be_bytes());
        data[4..6].copy_from_slice(&1u16.to_be_bytes());
        data[6..8].copy_from_slice(&0u16.to_be_bytes());
        data[12] = 3;
        data[13] = b'w';
        data[14] = b'w';
        data[15] = b'w';
        data[16] = 7;
        data[17..22].copy_from_slice(b"example");
        data[22] = 3;
        data[23..27].copy_from_slice(b"com");
        data[27] = 0;
        data[28..30].copy_from_slice(&1u16.to_be_bytes());
        data[30..32].copy_from_slice(&1u16.to_be_bytes());

        let record = DnsRecord::parse(&data);
        assert!(record.is_some());
        let record = record.unwrap();
        assert_eq!(record.transaction_id, 1);
        assert_eq!(record.questions.len(), 1);
        assert_eq!(record.answers.len(), 0);
    }
}
