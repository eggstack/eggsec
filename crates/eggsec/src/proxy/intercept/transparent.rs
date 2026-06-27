//! Transparent proxy mode using iptables/nftables REDIRECT.
//!
//! On Linux, this module provides transparent HTTP/HTTPS interception by
//! configuring iptables rules to redirect traffic to the proxy port.
//! On non-Linux platforms, the types compile but operations return errors.
//!
//! Gated behind the `transparent-proxy` feature flag.

use std::net::SocketAddr;

/// Configuration for transparent proxy mode.
#[derive(Debug, Clone)]
pub struct TransparentProxyConfig {
    /// Address to listen on (proxy redirect target).
    pub listen_addr: SocketAddr,
    /// Network interface to intercept (e.g., "eth0").
    pub interface: String,
    /// Ports to redirect (default: 80, 443).
    pub redirect_ports: Vec<u16>,
    /// Whether to also intercept DNS (port 53).
    pub intercept_dns: bool,
    /// Cleanup iptables rules on drop.
    pub cleanup_on_drop: bool,
}

impl Default for TransparentProxyConfig {
    fn default() -> Self {
        Self {
            listen_addr: "127.0.0.1:8080".parse().unwrap(),
            interface: "eth0".to_string(),
            redirect_ports: vec![80, 443],
            intercept_dns: false,
            cleanup_on_drop: true,
        }
    }
}

/// Result of iptables rule management operations.
#[derive(Debug, Clone)]
pub struct IptablesResult {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Output from iptables/nftables command.
    pub output: String,
    /// Rules that were inserted (for cleanup).
    pub inserted_rules: Vec<String>,
}

/// Manages iptables rules for transparent proxy interception.
pub struct TransparentProxy {
    config: TransparentProxyConfig,
    rules_active: bool,
}

impl TransparentProxy {
    /// Create a new transparent proxy manager.
    pub fn new(config: TransparentProxyConfig) -> Self {
        Self {
            config,
            rules_active: false,
        }
    }

    /// Insert iptables REDIRECT rules for transparent interception.
    ///
    /// On non-Linux platforms, returns an error.
    pub fn setup(&mut self) -> Result<IptablesResult, TransparentProxyError> {
        #[cfg(target_os = "linux")]
        {
            self.setup_linux()
        }
        #[cfg(not(target_os = "linux"))]
        {
            Err(TransparentProxyError::UnsupportedPlatform(
                "Transparent proxy requires Linux with iptables/nftables".to_string(),
            ))
        }
    }

    /// Remove all iptables rules that were inserted.
    pub fn cleanup(&mut self) -> Result<IptablesResult, TransparentProxyError> {
        #[cfg(target_os = "linux")]
        {
            self.cleanup_linux()
        }
        #[cfg(not(target_os = "linux"))]
        {
            Ok(IptablesResult {
                success: true,
                output: "No-op on non-Linux".to_string(),
                inserted_rules: vec![],
            })
        }
    }

    /// Check if iptables rules are currently active.
    pub fn is_active(&self) -> bool {
        self.rules_active
    }

    /// Get the configuration.
    pub fn config(&self) -> &TransparentProxyConfig {
        &self.config
    }

    #[cfg(target_os = "linux")]
    fn setup_linux(&mut self) -> Result<IptablesResult, TransparentProxyError> {
        let mut inserted_rules = Vec::new();
        let proxy_port = self.config.listen_addr.port();

        // First, check if iptables is available
        Self::check_iptables_available()?;

        for port in &self.config.redirect_ports {
            let rule_args = [
                "-t", "nat", "-A", "PREROUTING",
                "-i", &self.config.interface,
                "-p", "tcp",
                "--dport", &port.to_string(),
                "-j", "REDIRECT",
                "--to-port", &proxy_port.to_string(),
            ];

            let output = std::process::Command::new("iptables")
                .args(&rule_args)
                .output()
                .map_err(|e| TransparentProxyError::IptablesFailed(format!("Failed to execute iptables: {}", e)))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(TransparentProxyError::IptablesFailed(format!(
                    "iptables failed for port {}: {}",
                    port, stderr
                )));
            }

            let rule_str = format!(
                "-t nat -A PREROUTING -i {} -p tcp --dport {} -j REDIRECT --to-port {}",
                self.config.interface, port, proxy_port
            );
            inserted_rules.push(rule_str);
        }

        if self.config.intercept_dns {
            let dns_rule_args = [
                "-t", "nat", "-A", "PREROUTING",
                "-i", &self.config.interface,
                "-p", "udp",
                "--dport", "53",
                "-j", "REDIRECT",
                "--to-port", &proxy_port.to_string(),
            ];

            let output = std::process::Command::new("iptables")
                .args(&dns_rule_args)
                .output()
                .map_err(|e| TransparentProxyError::IptablesFailed(format!("Failed to execute iptables for DNS: {}", e)))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(TransparentProxyError::IptablesFailed(format!(
                    "iptables DNS redirect failed: {}",
                    stderr
                )));
            }

            let dns_rule_str = format!(
                "-t nat -A PREROUTING -i {} -p udp --dport 53 -j REDIRECT --to-port {}",
                self.config.interface, proxy_port
            );
            inserted_rules.push(dns_rule_str);
        }

        self.rules_active = true;

        tracing::info!(
            "Transparent proxy: {} iptables rules inserted on interface {}",
            inserted_rules.len(),
            self.config.interface
        );

        Ok(IptablesResult {
            success: true,
            output: format!("{} iptables rules inserted successfully", inserted_rules.len()),
            inserted_rules,
        })
    }

    #[cfg(target_os = "linux")]
    fn cleanup_linux(&mut self) -> Result<IptablesResult, TransparentProxyError> {
        if !self.rules_active {
            return Ok(IptablesResult {
                success: true,
                output: "No rules to clean up".to_string(),
                inserted_rules: vec![],
            });
        }

        let mut removed_rules = Vec::new();
        let proxy_port = self.config.listen_addr.port();

        // Remove rules in reverse order
        for port in self.config.redirect_ports.iter().rev() {
            let rule_args = [
                "-t", "nat", "-D", "PREROUTING",
                "-i", &self.config.interface,
                "-p", "tcp",
                "--dport", &port.to_string(),
                "-j", "REDIRECT",
                "--to-port", &proxy_port.to_string(),
            ];

            let output = std::process::Command::new("iptables")
                .args(&rule_args)
                .output()
                .map_err(|e| TransparentProxyError::IptablesFailed(format!("Failed to execute iptables cleanup: {}", e)))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                tracing::warn!("iptables cleanup failed for port {}: {}", port, stderr);
            } else {
                removed_rules.push(format!("Removed redirect for port {}", port));
            }
        }

        if self.config.intercept_dns {
            let dns_rule_args = [
                "-t", "nat", "-D", "PREROUTING",
                "-i", &self.config.interface,
                "-p", "udp",
                "--dport", "53",
                "-j", "REDIRECT",
                "--to-port", &proxy_port.to_string(),
            ];

            let output = std::process::Command::new("iptables")
                .args(&dns_rule_args)
                .output()
                .map_err(|e| TransparentProxyError::IptablesFailed(format!("Failed to execute iptables DNS cleanup: {}", e)))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                tracing::warn!("iptables DNS cleanup failed: {}", stderr);
            } else {
                removed_rules.push("Removed DNS redirect".to_string());
            }
        }

        self.rules_active = false;

        tracing::info!("Transparent proxy: {} iptables rules removed", removed_rules.len());

        Ok(IptablesResult {
            success: true,
            output: format!("{} iptables rules removed", removed_rules.len()),
            inserted_rules: removed_rules,
        })
    }

    #[cfg(target_os = "linux")]
    fn check_iptables_available() -> Result<(), TransparentProxyError> {
        let output = std::process::Command::new("iptables")
            .arg("--version")
            .output()
            .map_err(|e| TransparentProxyError::PermissionDenied(format!(
                "iptables not found or not executable: {}. Ensure iptables is installed and accessible.",
                e
            )))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(TransparentProxyError::PermissionDenied(format!(
                "iptables check failed: {}",
                stderr
            )));
        }

        // Check if we have root/CAP_NET_ADMIN permissions by trying a dry-run
        let test_output = std::process::Command::new("iptables")
            .args(["-t", "nat", "-L", "PREROUTING", "-n"])
            .output()
            .map_err(|e| TransparentProxyError::PermissionDenied(format!(
                "Failed to check iptables permissions: {}",
                e
            )))?;

        if !test_output.status.success() {
            let stderr = String::from_utf8_lossy(&test_output.stderr);
            return Err(TransparentProxyError::PermissionDenied(format!(
                "Insufficient permissions for iptables (requires root or CAP_NET_ADMIN): {}",
                stderr
            )));
        }

        Ok(())
    }
}

impl Drop for TransparentProxy {
    fn drop(&mut self) {
        if self.config.cleanup_on_drop && self.rules_active {
            if let Err(e) = self.cleanup() {
                tracing::warn!("transparent proxy cleanup failed: {}", e);
            }
        }
    }
}

/// Errors that can occur during transparent proxy operations.
#[derive(Debug, Clone)]
pub enum TransparentProxyError {
    /// Platform does not support transparent proxy.
    UnsupportedPlatform(String),
    /// iptables/nftables command failed.
    IptablesFailed(String),
    /// Permission denied (requires root/CAP_NET_ADMIN).
    PermissionDenied(String),
}

impl std::fmt::Display for TransparentProxyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedPlatform(msg) => write!(f, "Unsupported platform: {}", msg),
            Self::IptablesFailed(msg) => write!(f, "iptables failed: {}", msg),
            Self::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
        }
    }
}

impl std::error::Error for TransparentProxyError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = TransparentProxyConfig::default();
        assert_eq!(config.listen_addr.port(), 8080);
        assert_eq!(config.redirect_ports, vec![80, 443]);
        assert!(!config.intercept_dns);
        assert!(config.cleanup_on_drop);
    }

    #[test]
    fn test_transparent_proxy_new() {
        let config = TransparentProxyConfig::default();
        let proxy = TransparentProxy::new(config);
        assert!(!proxy.is_active());
    }

    #[test]
    fn test_cleanup_noop_when_inactive() {
        let config = TransparentProxyConfig::default();
        let mut proxy = TransparentProxy::new(config);
        let result = proxy.cleanup().unwrap();
        assert!(result.success);
    }

    #[test]
    fn test_transparent_proxy_error_display() {
        let err = TransparentProxyError::UnsupportedPlatform("test".to_string());
        assert!(err.to_string().contains("test"));
    }
}
