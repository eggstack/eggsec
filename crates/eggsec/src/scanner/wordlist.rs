use crate::error::{EggsecError, Result};
use std::path::Path;

/// Maximum allowed length for a single endpoint path.
const MAX_ENDPOINT_LENGTH: usize = 2048;

/// A parsed and validated endpoint wordlist.
#[derive(Debug, Clone)]
pub struct Wordlist {
    endpoints: Vec<String>,
}

impl Wordlist {
    /// Parse a wordlist from a file path.
    ///
    /// Reads the file, splits by lines, trims whitespace, skips empty lines
    /// and comments (`#`), normalizes paths to start with `/`, and validates
    /// each endpoint.
    pub async fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = tokio::fs::read_to_string(path).await.map_err(|e| {
            EggsecError::Config(format!("failed to read wordlist {}: {}", path.display(), e))
        })?;
        Self::parse(&content)
    }

    /// Parse a wordlist from a string.
    pub fn parse(content: &str) -> Result<Self> {
        let endpoints: Vec<String> = content
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                if let Some(err) = validate_endpoint(line) {
                    return Err(EggsecError::Config(format!(
                        "invalid endpoint '{}': {}",
                        line, err
                    )));
                }
                Ok(normalize_path(line))
            })
            .collect::<Result<Vec<_>>>()?;

        if endpoints.is_empty() {
            return Err(EggsecError::Config(
                "wordlist contains no valid endpoints".into(),
            ));
        }

        Ok(Self { endpoints })
    }

    /// Return the list of validated endpoint paths.
    pub fn endpoints(&self) -> &[String] {
        &self.endpoints
    }

    /// Consume the wordlist and return the inner vector.
    pub fn into_endpoints(self) -> Vec<String> {
        self.endpoints
    }

    /// Return the number of endpoints in the wordlist.
    pub fn len(&self) -> usize {
        self.endpoints.len()
    }

    /// Return true if the wordlist is empty.
    pub fn is_empty(&self) -> bool {
        self.endpoints.is_empty()
    }
}

/// Normalize a path to start with `/`.
///
/// - Strips leading `/` then re-adds it to collapse duplicates.
/// - Leaves paths that already start with `/` as-is (after dedup).
fn normalize_path(line: &str) -> String {
    let trimmed = line.trim_start_matches('/');
    format!("/{trimmed}")
}

/// Validate a single endpoint string.
///
/// Returns `None` if valid, or `Some(reason)` if invalid.
fn validate_endpoint(line: &str) -> Option<&'static str> {
    if line.len() > MAX_ENDPOINT_LENGTH {
        return Some("exceeds maximum length");
    }
    if line.contains(' ') {
        return Some("contains whitespace");
    }
    if line.bytes().any(|b| b < 0x20 || b == 0x7f) {
        return Some("contains control characters");
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic_wordlist() {
        let input = "/admin\n/api/v1\n/login\n";
        let wl = Wordlist::parse(input).unwrap();
        assert_eq!(wl.len(), 3);
        assert_eq!(wl.endpoints()[0], "/admin");
        assert_eq!(wl.endpoints()[1], "/api/v1");
        assert_eq!(wl.endpoints()[2], "/login");
    }

    #[test]
    fn parse_skips_empty_lines_and_comments() {
        let input = "# comment\n\n/admin\n  \n# another comment\n/api\n";
        let wl = Wordlist::parse(input).unwrap();
        assert_eq!(wl.len(), 2);
    }

    #[test]
    fn parse_normalizes_paths_without_leading_slash() {
        let input = "admin\napi/v1\nlogin\n";
        let wl = Wordlist::parse(input).unwrap();
        assert_eq!(wl.endpoints()[0], "/admin");
        assert_eq!(wl.endpoints()[1], "/api/v1");
        assert_eq!(wl.endpoints()[2], "/login");
    }

    #[test]
    fn parse_normalizes_double_leading_slash() {
        let input = "//admin\n";
        let wl = Wordlist::parse(input).unwrap();
        assert_eq!(wl.endpoints()[0], "/admin");
    }

    #[test]
    fn parse_trims_whitespace() {
        let input = "  /admin  \n\t/api\t\n";
        let wl = Wordlist::parse(input).unwrap();
        assert_eq!(wl.endpoints()[0], "/admin");
        assert_eq!(wl.endpoints()[1], "/api");
    }

    #[test]
    fn parse_rejects_empty_wordlist() {
        let input = "# only comments\n\n";
        let err = Wordlist::parse(input).unwrap_err();
        assert!(err.to_string().contains("no valid endpoints"));
    }

    #[test]
    fn parse_rejects_endpoint_with_spaces() {
        let input = "/admin panel\n";
        let err = Wordlist::parse(input).unwrap_err();
        assert!(err.to_string().contains("contains whitespace"));
    }

    #[test]
    fn parse_rejects_endpoint_with_control_chars() {
        let input = "/admin\x01panel\n";
        let err = Wordlist::parse(input).unwrap_err();
        assert!(err.to_string().contains("control characters"));
    }

    #[test]
    fn validate_endpoint_valid() {
        assert!(validate_endpoint("/admin").is_none());
        assert!(validate_endpoint("/api/v1/users").is_none());
        assert!(validate_endpoint("/.env").is_none());
    }

    #[test]
    fn validate_endpoint_too_long() {
        let long = "/".repeat(MAX_ENDPOINT_LENGTH + 1);
        assert!(validate_endpoint(&long).is_some());
    }

    #[test]
    fn normalize_path_strips_duplicates() {
        assert_eq!(normalize_path("/admin"), "/admin");
        assert_eq!(normalize_path("//admin"), "/admin");
        assert_eq!(normalize_path("///admin"), "/admin");
        assert_eq!(normalize_path("admin"), "/admin");
    }

    #[test]
    fn wordlist_len_and_is_empty() {
        let wl = Wordlist::parse("/admin\n/api\n").unwrap();
        assert_eq!(wl.len(), 2);
        assert!(!wl.is_empty());
    }

    #[test]
    fn into_endpoints_consumes() {
        let wl = Wordlist::parse("/admin\n/api\n").unwrap();
        let eps = wl.into_endpoints();
        assert_eq!(eps.len(), 2);
    }

    #[test]
    fn parse_example_endpoints_file() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("examples")
            .join("endpoints.txt");
        if !path.exists() {
            eprintln!("skipping: {} not found", path.display());
            return;
        }
        let content = std::fs::read_to_string(&path).unwrap();
        let wl = Wordlist::parse(&content).unwrap();
        assert!(wl.len() > 100, "expected 100+ endpoints, got {}", wl.len());
        for ep in wl.endpoints() {
            assert!(ep.starts_with('/'), "endpoint missing leading /: {ep}");
            assert!(!ep.contains(' '), "endpoint has space: {ep}");
        }
    }
}
