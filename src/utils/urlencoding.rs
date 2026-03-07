#![allow(dead_code)]

pub fn encode(s: &str) -> String {
    let mut encoded = String::new();
    for c in s.chars() {
        match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => {
                encoded.push(c);
            }
            _ => {
                for byte in c.to_string().as_bytes() {
                    encoded.push_str(&format!("%{:02X}", byte));
                }
            }
        }
    }
    encoded
}

pub fn decode(s: &str) -> Result<String, String> {
    let mut decoded = String::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if hex.len() != 2 {
                return Err("Invalid percent encoding".to_string());
            }
            match u8::from_str_radix(&hex, 16) {
                Ok(byte) => decoded.push(byte as char),
                Err(_) => return Err(format!("Invalid hex: {}", hex)),
            }
        } else if c == '+' {
            decoded.push(' ');
        } else {
            decoded.push(c);
        }
    }

    Ok(decoded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_simple() {
        assert_eq!(encode("hello"), "hello");
    }

    #[test]
    fn test_encode_space() {
        assert_eq!(encode("hello world"), "hello%20world");
    }

    #[test]
    fn test_encode_special() {
        assert_eq!(encode("hello/world"), "hello%2Fworld");
    }

    #[test]
    fn test_encode_unicode() {
        let encoded = encode("hello\u{1F600}");
        assert!(encoded.starts_with("hello%F0%9F%98%80"));
    }

    #[test]
    fn test_decode_simple() {
        assert_eq!(decode("hello").unwrap(), "hello");
    }

    #[test]
    fn test_decode_space() {
        assert_eq!(decode("hello%20world").unwrap(), "hello world");
    }

    #[test]
    fn test_decode_plus() {
        assert_eq!(decode("hello+world").unwrap(), "hello world");
    }
}
