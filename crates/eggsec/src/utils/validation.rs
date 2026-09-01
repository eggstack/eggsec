use crate::constants::http;
use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};

pub fn validate_path(base: &Path, user_path: &Path) -> Result<PathBuf> {
    let base_canonical = base
        .canonicalize()
        .map_err(|e| anyhow!("Failed to canonicalize base path: {}", e))?;

    // Try to canonicalize the user path if it exists
    let canonical = if user_path.exists() {
        user_path
            .canonicalize()
            .map_err(|e| anyhow!("Failed to canonicalize path: {}", e))?
    } else {
        // For non-existent paths, walk up to the nearest existing ancestor,
        // canonicalize it, then re-append the remaining tail components and
        // lexically normalize `..` / `.` before the prefix check.
        let (ancestor, tail) = find_existing_ancestor(user_path);
        if let Some(ancestor) = ancestor {
            let ancestor_canonical = ancestor
                .canonicalize()
                .map_err(|e| anyhow!("Failed to canonicalize ancestor path: {}", e))?;
            let mut candidate = ancestor_canonical;
            for comp in tail.iter() {
                candidate.push(comp);
            }
            normalize_lexically(&candidate)
        } else {
            return Err(anyhow!(
                "Path traversal detected: {} is not within {} (no existing ancestor)",
                user_path.display(),
                base.display()
            ));
        }
    };

    if !canonical.starts_with(&base_canonical) {
        return Err(anyhow!(
            "Path traversal detected: {} is not within {}",
            user_path.display(),
            base.display()
        ));
    }
    Ok(canonical)
}

fn find_existing_ancestor(path: &Path) -> (Option<PathBuf>, Vec<std::ffi::OsString>) {
    use std::path::Component;
    for ancestor in path.ancestors() {
        if ancestor.as_os_str().is_empty() {
            continue;
        }
        if ancestor.exists() {
            let tail_path = path
                .strip_prefix(ancestor)
                .unwrap_or_else(|_| Path::new(""));
            let mut tail = Vec::new();
            for comp in tail_path.components() {
                match comp {
                    Component::Normal(os) => tail.push(os.to_os_string()),
                    Component::ParentDir => tail.push(std::ffi::OsString::from("..")),
                    Component::CurDir => tail.push(std::ffi::OsString::from(".")),
                    Component::RootDir | Component::Prefix(_) => {}
                }
            }
            return (Some(ancestor.to_path_buf()), tail);
        }
    }
    // No ancestor found; for relative paths try "." as fallback ancestor if it exists
    if !path.is_absolute() && Path::new(".").exists() {
        let mut tail = Vec::new();
        for comp in path.components() {
            match comp {
                Component::Normal(os) => tail.push(os.to_os_string()),
                Component::ParentDir => tail.push(std::ffi::OsString::from("..")),
                Component::CurDir => tail.push(std::ffi::OsString::from(".")),
                Component::RootDir | Component::Prefix(_) => {}
            }
        }
        return (Some(PathBuf::from(".")), tail);
    }
    (None, Vec::new())
}

fn normalize_lexically(path: &Path) -> PathBuf {
    use std::path::Component;
    let mut components: Vec<Component> = Vec::new();
    for comp in path.components() {
        match comp {
            Component::ParentDir => {
                // Pop last Normal if possible, otherwise keep ParentDir unless at root
                if let Some(last) = components.last() {
                    match last {
                        Component::Normal(_) => {
                            components.pop();
                        }
                        Component::RootDir | Component::Prefix(_) => {
                            // Cannot go above root
                        }
                        _ => components.push(comp),
                    }
                } else {
                    components.push(comp);
                }
            }
            Component::CurDir => {}
            _ => components.push(comp),
        }
    }
    let mut out = PathBuf::new();
    for comp in components {
        out.push(comp.as_os_str());
    }
    if out.as_os_str().is_empty() {
        out.push(".");
    }
    out
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
    // Shared validator is used by port scans (default 100) and by fuzzer/endpoint
    // scans which may legitimately use higher concurrency. Keep the hard cap
    // above the port-scan default to avoid rejecting valid CLI values.
    const MAX_CONCURRENCY: usize = 1000;
    if concurrency == 0 {
        return Err(anyhow!("Concurrency must be greater than 0"));
    }
    if concurrency > MAX_CONCURRENCY {
        return Err(anyhow!("Concurrency cannot exceed {}", MAX_CONCURRENCY));
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
    if rps > crate::constants::MAX_REQUESTS_PER_SECOND_LIMIT {
        return Err(anyhow!(
            "Rate limit cannot exceed {} requests per second",
            crate::constants::MAX_REQUESTS_PER_SECOND_LIMIT
        ));
    }
    Ok(())
}

pub fn validate_git_repo_path(repo_path: &str) -> Result<()> {
    let path = Path::new(repo_path);

    if !path.exists() {
        return Err(anyhow!("Path does not exist: {}", repo_path));
    }

    path.canonicalize()
        .map_err(|e| anyhow!("Failed to canonicalize path: {} - {}", repo_path, e))?;

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
        assert!(validate_concurrency(1001).is_err());
    }

    #[test]
    fn test_validate_concurrency_port_default_plus_one_still_allowed() {
        // Shared validator intentionally allows values above the port-scan default
        // (100) for fuzzer/endpoint scans up to the hard cap (1000).
        assert!(validate_concurrency(crate::constants::scan::DEFAULT_PORT_CONCURRENCY + 1).is_ok());
        assert!(validate_concurrency(200).is_ok());
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
        fn test_validate_concurrency_in_range_passes(val in 1usize..1000usize) {
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
