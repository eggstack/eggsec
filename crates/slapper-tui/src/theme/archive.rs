use std::io::Cursor;
use std::path::PathBuf;

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ThemeArchiveError {
    #[error("invalid archive magic bytes")]
    InvalidMagic,
    #[error("unsupported archive version: {0}")]
    UnsupportedVersion(u16),
    #[error("unsafe path in archive: {0}")]
    UnsafePath(String),
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("lzma decompression failed: {0}")]
    LzmaError(String),
    #[error("base64 decoding failed: {0}")]
    Base64Error(#[from] base64::DecodeError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackagedThemeFile {
    pub relative_path: PathBuf,
    pub content: Vec<u8>,
}

const MAGIC: &[u8; 9] = b"SLPTHEME\0";
const CURRENT_VERSION: u16 = 1;

fn validate_path(path: &str) -> Result<(), ThemeArchiveError> {
    if path.is_empty() {
        return Err(ThemeArchiveError::UnsafePath("empty path".to_string()));
    }
    if PathBuf::from(path).is_absolute() {
        return Err(ThemeArchiveError::UnsafePath(format!(
            "absolute path: {path}"
        )));
    }
    if path.contains("..") {
        return Err(ThemeArchiveError::UnsafePath(format!(
            "path traversal: {path}"
        )));
    }
    if !path.ends_with(".toml") {
        return Err(ThemeArchiveError::UnsafePath(format!(
            "non-.toml file: {path}"
        )));
    }
    Ok(())
}

pub fn decode_packaged_archive(bytes: &[u8]) -> Result<Vec<PackagedThemeFile>, ThemeArchiveError> {
    let mut cursor = Cursor::new(bytes);

    let mut magic = [0u8; 9];
    std::io::Read::read_exact(&mut cursor, &mut magic)?;
    if &magic != MAGIC {
        return Err(ThemeArchiveError::InvalidMagic);
    }

    let mut version_buf = [0u8; 2];
    std::io::Read::read_exact(&mut cursor, &mut version_buf)?;
    let version = u16::from_le_bytes(version_buf);
    if version != CURRENT_VERSION {
        return Err(ThemeArchiveError::UnsupportedVersion(version));
    }

    let mut file_count_buf = [0u8; 4];
    std::io::Read::read_exact(&mut cursor, &mut file_count_buf)?;
    let file_count = u32::from_le_bytes(file_count_buf);

    let mut files = Vec::with_capacity(file_count as usize);

    for _ in 0..file_count {
        let mut path_len_buf = [0u8; 2];
        std::io::Read::read_exact(&mut cursor, &mut path_len_buf)?;
        let path_len = u16::from_le_bytes(path_len_buf) as usize;

        let mut path_buf = vec![0u8; path_len];
        std::io::Read::read_exact(&mut cursor, &mut path_buf)?;
        let path_str = std::str::from_utf8(&path_buf)
            .map_err(|e| ThemeArchiveError::UnsafePath(format!("invalid UTF-8: {e}")))?;

        validate_path(path_str)?;

        let mut content_len_buf = [0u8; 8];
        std::io::Read::read_exact(&mut cursor, &mut content_len_buf)?;
        let content_len = u64::from_le_bytes(content_len_buf) as usize;

        let mut content = vec![0u8; content_len];
        std::io::Read::read_exact(&mut cursor, &mut content)?;

        files.push(PackagedThemeFile {
            relative_path: PathBuf::from(path_str),
            content,
        });
    }

    Ok(files)
}

pub fn decode_lzma_base64(encoded: &str) -> Result<Vec<PackagedThemeFile>, ThemeArchiveError> {
    let compressed = STANDARD.decode(encoded)?;

    let mut decompressed = Vec::new();
    lzma_rs::xz_decompress(&mut Cursor::new(compressed), &mut decompressed)
        .map_err(|e| ThemeArchiveError::LzmaError(format!("{e}")))?;

    decode_packaged_archive(&decompressed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_archive(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(b"SLPTHEME\0");
        buf.extend_from_slice(&1u16.to_le_bytes());
        buf.extend_from_slice(&(files.len() as u32).to_le_bytes());
        for (path, content) in files {
            let path_bytes = path.as_bytes();
            buf.extend_from_slice(&(path_bytes.len() as u16).to_le_bytes());
            buf.extend_from_slice(path_bytes);
            buf.extend_from_slice(&(content.len() as u64).to_le_bytes());
            buf.extend_from_slice(content);
        }
        buf
    }

    #[test]
    fn rejects_bad_magic() {
        let mut archive = make_test_archive(&[("test.toml", b"content")]);
        archive[0] = b'X';
        assert!(matches!(
            decode_packaged_archive(&archive),
            Err(ThemeArchiveError::InvalidMagic)
        ));
    }

    #[test]
    fn rejects_absolute_path() {
        let archive = make_test_archive(&[("/etc/passwd.toml", b"content")]);
        assert!(matches!(
            decode_packaged_archive(&archive),
            Err(ThemeArchiveError::UnsafePath(_))
        ));
    }

    #[test]
    fn rejects_path_traversal() {
        let archive = make_test_archive(&[("../escape.toml", b"content")]);
        assert!(matches!(
            decode_packaged_archive(&archive),
            Err(ThemeArchiveError::UnsafePath(_))
        ));
    }

    #[test]
    fn rejects_non_toml() {
        let archive = make_test_archive(&[("theme.txt", b"content")]);
        assert!(matches!(
            decode_packaged_archive(&archive),
            Err(ThemeArchiveError::UnsafePath(_))
        ));
    }

    #[test]
    fn accepts_valid_archive() {
        let archive = make_test_archive(&[
            ("colors.toml", b"[palette]\nprimary = \"#ff0000\""),
            ("styles.toml", b"[header]\nbold = true"),
        ]);
        let files = decode_packaged_archive(&archive).unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].relative_path, PathBuf::from("colors.toml"));
        assert_eq!(files[0].content, b"[palette]\nprimary = \"#ff0000\"");
        assert_eq!(files[1].relative_path, PathBuf::from("styles.toml"));
        assert_eq!(files[1].content, b"[header]\nbold = true");
    }
}
