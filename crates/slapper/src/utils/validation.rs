use crate::constants::{http, scan};
use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};

pub fn validate_path(base: &Path, user_path: &Path) -> Result<PathBuf> {
    let canonical = user_path
        .canonicalize()
        .map_err(|e| anyhow!("Failed to canonicalize path: {}", e))?;
    let base_canonical = base
        .canonicalize()
        .map_err(|e| anyhow!("Failed to canonicalize base path: {}", e))?;
    if !canonical.starts_with(&base_canonical) {
        return Err(anyhow!(
            "Path traversal detected: {} is not within {}",
            user_path.display(),
            base.display()
        ));
    }
    Ok(canonical)
}

pub fn validate_path_string(base: &Path, user_path: &str) -> Result<PathBuf> {
    validate_path(base, Path::new(user_path))
}

pub fn validate_url(url: &str) -> Result<()> {
    if url.is_empty() {
        return Err(anyhow!("URL cannot be empty"));
    }
    crate::utils::parsing::parse_url_validated(url)?;
    Ok(())
}

pub fn validate_concurrency(concurrency: usize) -> Result<()> {
    if concurrency == 0 {
        return Err(anyhow!("Concurrency must be greater than 0"));
    }
    if concurrency > scan::DEFAULT_PORT_CONCURRENCY {
        return Err(anyhow!(
            "Concurrency cannot exceed {}",
            scan::DEFAULT_PORT_CONCURRENCY
        ));
    }
    Ok(())
}

pub fn validate_timeout(timeout: u64) -> Result<()> {
    if timeout == 0 {
        return Err(anyhow!("Timeout must be greater than 0"));
    }
    if timeout > http::DEFAULT_TIMEOUT_SECS * 10 {
        return Err(anyhow!(
            "Timeout cannot exceed {} seconds",
            http::DEFAULT_TIMEOUT_SECS * 10
        ));
    }
    Ok(())
}

pub fn validate_rate_limit(rps: u32) -> Result<()> {
    if rps == 0 {
        return Err(anyhow!("Rate limit must be greater than 0"));
    }
    if rps > 10000 {
        return Err(anyhow!(
            "Rate limit cannot exceed 10000 requests per second"
        ));
    }
    Ok(())
}

pub fn validate_git_repo_path(repo_path: &str) -> Result<()> {
    let path = Path::new(repo_path);

    if !path.exists() {
        return Err(anyhow!("Path does not exist: {}", repo_path));
    }

    let canonical = path
        .canonicalize()
        .map_err(|e| anyhow!("Failed to canonicalize path: {} - {}", repo_path, e))?;

    if !canonical.exists() {
        return Err(anyhow!(
            "Path does not exist after canonicalization: {}",
            repo_path
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_validate_url_valid() {
        assert!(validate_url("https://example.com").is_ok());
    }

    #[test]
    fn test_validate_url_empty() {
        assert!(validate_url("").is_err());
    }

    #[test]
    fn test_validate_url_invalid_scheme() {
        assert!(validate_url("ftp://example.com").is_err());
    }

    #[test]
    fn test_validate_concurrency_valid() {
        assert!(validate_concurrency(10).is_ok());
    }

    #[test]
    fn test_validate_concurrency_zero() {
        assert!(validate_concurrency(0).is_err());
    }

    #[test]
    fn test_validate_concurrency_too_high() {
        assert!(validate_concurrency(scan::DEFAULT_PORT_CONCURRENCY + 1).is_err());
    }

    #[test]
    fn test_validate_timeout_valid() {
        assert!(validate_timeout(30).is_ok());
    }

    #[test]
    fn test_validate_timeout_zero() {
        assert!(validate_timeout(0).is_err());
    }

    #[test]
    fn test_validate_timeout_too_high() {
        assert!(validate_timeout(http::DEFAULT_TIMEOUT_SECS * 10 + 1).is_err());
    }

    #[test]
    fn test_validate_rate_limit_valid() {
        assert!(validate_rate_limit(100).is_ok());
    }

    #[test]
    fn test_validate_rate_limit_zero() {
        assert!(validate_rate_limit(0).is_err());
    }

    proptest! {
        #[test]
        fn test_validate_concurrency_in_range_passes(val in 1usize..scan::DEFAULT_PORT_CONCURRENCY) {
            prop_assert!(validate_concurrency(val).is_ok());
        }

        #[test]
        fn test_validate_timeout_in_range_passes(val in 1u64..http::DEFAULT_TIMEOUT_SECS * 10) {
            prop_assert!(validate_timeout(val).is_ok());
        }

        #[test]
        fn test_validate_rate_limit_in_range_passes(val in 1u32..10000) {
            prop_assert!(validate_rate_limit(val).is_ok());
        }
    }
}
