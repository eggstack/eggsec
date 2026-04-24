use crate::packet::types::*;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tls_client_hello() {
        let data = vec![
            0x16, 0x03, 0x01, 0x01, 0x00, 0x01
        ];
        let tls = TlsHandshake::parse(&data);
        assert!(tls.is_some());
        let tls = tls.unwrap();
        assert_eq!(tls.handshake_type, "ClientHello");
        assert_eq!(tls.version, "TLS 1.2");
    }

    #[test]
    fn test_parse_tls_server_hello() {
        let data = vec![
            0x16, 0x03, 0x03, 0x00, 0x51, 0x02
        ];
        let tls = TlsHandshake::parse(&data);
        assert!(tls.is_some());
        let tls = tls.unwrap();
        assert_eq!(tls.handshake_type, "ServerHello");
        assert_eq!(tls.version, "TLS 1.2");
    }

    #[test]
    fn test_parse_tls_empty_data() {
        assert!(TlsHandshake::parse(&[]).is_none());
        assert!(TlsHandshake::parse(&[0x16]).is_none());
        assert!(TlsHandshake::parse(&[0x16, 0x03]).is_none());
    }

    #[test]
    fn test_parse_tls_invalid_version() {
        let data = vec![0x16, 0x04, 0x01, 0x01, 0x00, 0x01];
        assert!(TlsHandshake::parse(&data).is_none());
    }

    #[test]
    fn test_parse_tls_1_0() {
        let data = vec![0x16, 0x03, 0x01, 0x01, 0x00, 0x01];
        let tls = TlsHandshake::parse(&data).unwrap();
        assert_eq!(tls.version, "TLS 1.0");
    }

    #[test]
    fn test_parse_tls_1_3() {
        let data = vec![0x16, 0x03, 0x04, 0x01, 0x00, 0x01];
        let tls = TlsHandshake::parse(&data).unwrap();
        assert_eq!(tls.version, "TLS 1.3");
    }
}
