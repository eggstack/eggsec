use std::sync::LazyLock;

#[cfg(feature = "python-plugins")]
use regex::Regex;

const MAX_PLUGIN_SIZE_BYTES: usize = 1_000_000;

#[cfg(feature = "python-plugins")]
static SUSPICIOUS_PYTHON_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r"(?i)os\.system").unwrap(),
        Regex::new(r"(?i)subprocess").unwrap(),
        Regex::new(r"(?i)socket").unwrap(),
        Regex::new(r"(?i)eval\(").unwrap(),
        Regex::new(r"(?i)\bfork\b").unwrap(),
        Regex::new(r"(?i)__import__").unwrap(),
        Regex::new(r"(?i)\bopen\(").unwrap(),
        Regex::new(r"(?i)pty\.spawn").unwrap(),
        Regex::new(r"(?i)os\.popen").unwrap(),
        Regex::new(r"(?i)multiprocessing\.Process").unwrap(),
        Regex::new(r"(?i)ctypes").unwrap(),
        Regex::new(r"(?i)importlib").unwrap(),
        Regex::new(r"(?i)getattr\(").unwrap(),
        Regex::new(r"(?i)chr\(").unwrap(),
        Regex::new(r"(?i)\\x[0-9a-fA-F]{2}").unwrap(),
        Regex::new(r"(?i)\\u[0-9a-fA-F]{4}").unwrap(),
        Regex::new(r"(?i)\\[0-7]{3,}").unwrap(),
    ]
});

pub fn get_max_plugin_size_bytes() -> usize {
    MAX_PLUGIN_SIZE_BYTES
}

#[cfg(feature = "python-plugins")]
pub fn validate_python_plugin(content: &str, block_suspicious_plugins: bool) -> anyhow::Result<()> {
    if content.len() > MAX_PLUGIN_SIZE_BYTES {
        anyhow::bail!(
            "Plugin exceeds maximum size of {} bytes",
            MAX_PLUGIN_SIZE_BYTES
        );
    }

    let mut suspicious_found = Vec::new();
    for pattern in SUSPICIOUS_PYTHON_PATTERNS.iter() {
        if pattern.is_match(content) {
            suspicious_found.push(pattern.as_str());
        }
    }

    if !suspicious_found.is_empty() {
        if block_suspicious_plugins {
            anyhow::bail!(
                "Plugin contains suspicious patterns and blocking is enabled: {}",
                suspicious_found.join(", ")
            );
        } else {
            tracing::warn!(
                "Plugin contains suspicious patterns (allowing due to config): {}",
                suspicious_found.join(", ")
            );
        }
    }

    Ok(())
}

#[cfg(not(feature = "python-plugins"))]
pub fn validate_python_plugin(_content: &str, _block_suspicious_plugins: bool) -> anyhow::Result<()> {
    anyhow::bail!("Python plugins support is not enabled");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_python_plugin_size() {
        let large_content = "a".repeat(MAX_PLUGIN_SIZE_BYTES + 1);
        let result = validate_python_plugin(&large_content, true);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("maximum size"));
    }

    #[test]
    fn test_validate_python_plugin_clean() {
        let content = "def run_check(target):\n    return []\n";
        let result = validate_python_plugin(content, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_python_plugin_suspicious() {
        let content = "import os\nos.system('ls')\n";
        let result = validate_python_plugin(content, true);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("suspicious patterns"));
    }

    #[test]
    fn test_validate_python_plugin_suspicious_no_block() {
        let content = "import os\nos.system('ls')\n";
        let result = validate_python_plugin(content, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_max_plugin_size_constant() {
        assert_eq!(MAX_PLUGIN_SIZE_BYTES, 1_000_000);
    }
}
