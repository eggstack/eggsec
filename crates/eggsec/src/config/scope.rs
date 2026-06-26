use ipnetwork::IpNetwork;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::str::FromStr;
use url::Url;

/// Provenance of a loaded scope manifest.
///
/// Used by [`LoadedScope`] to distinguish between "no scope provided" and
/// "user explicitly supplied an empty scope". Strict execution profiles
/// (MCP, agent, CI) require an explicit manifest for networked operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ScopeSource {
    /// No scope file was found or provided.
    DefaultEmpty,
    /// Scope loaded from the config file's `[scope]` section or profile.
    ConfigFile,
    /// Scope loaded from a CLI `--scope` argument.
    CliScopeFile,
    /// Scope generated from a preset or template.
    GeneratedPreset,
}

/// A scope with provenance metadata.
///
/// Wraps [`Scope`] with information about where it was loaded from, enabling
/// strict execution paths to distinguish "no scope" from "explicit empty scope".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadedScope {
    pub scope: Scope,
    pub source: ScopeSource,
    pub path: Option<String>,
}

impl LoadedScope {
    /// Returns `true` if this scope came from an explicit manifest
    /// (config file, CLI path, or generated preset).
    pub fn is_explicit_manifest(&self) -> bool {
        matches!(
            self.source,
            ScopeSource::ConfigFile | ScopeSource::CliScopeFile | ScopeSource::GeneratedPreset
        )
    }

    /// Create a default empty scope (no manifest provided).
    pub fn default_empty() -> Self {
        Self {
            scope: Scope::default(),
            source: ScopeSource::DefaultEmpty,
            path: None,
        }
    }

    /// Create from an explicit scope with provenance.
    pub fn explicit(scope: Scope, source: ScopeSource, path: Option<String>) -> Self {
        Self {
            scope,
            source,
            path,
        }
    }
}

impl Default for LoadedScope {
    fn default() -> Self {
        Self::default_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Scope {
    #[serde(default)]
    pub allowed_targets: Vec<ScopeRule>,

    #[serde(default)]
    pub excluded_targets: Vec<ScopeRule>,

    #[serde(default)]
    pub allowed_ports: Option<Vec<u16>>,

    #[serde(default)]
    pub excluded_ports: Vec<u16>,

    #[serde(default)]
    pub max_requests_per_second: Option<u32>,

    #[serde(default)]
    pub require_explicit_scope: bool,

    #[serde(default)]
    pub scope_file: Option<String>,
}

impl Scope {
    pub fn new() -> Self {
        Self::default()
    }

    /// Validates the scope configuration.
    ///
    /// Checks:
    /// - `allowed_targets` is not empty when `require_explicit_scope` is true
    /// - No duplicate ports in `allowed_ports`
    /// - `max_requests_per_second` is in range 1..=10000 (if set)
    pub fn validate(&self) -> Result<(), ScopeError> {
        if self.allowed_targets.is_empty() && self.require_explicit_scope {
            return Err(ScopeError::Validation(
                "At least one allowed target is required when require_explicit_scope is true"
                    .to_string(),
            ));
        }

        if let Some(ref ports) = self.allowed_ports {
            let mut seen = rustc_hash::FxHashSet::default();
            for &port in ports {
                if !seen.insert(port) {
                    return Err(ScopeError::Validation(format!(
                        "Duplicate port {} in allowed_ports",
                        port
                    )));
                }
            }
        }

        if let Some(rate) = self.max_requests_per_second {
            if rate == 0 {
                return Err(ScopeError::Validation(
                    "max_requests_per_second must be greater than 0".to_string(),
                ));
            }
            if rate > crate::constants::MAX_REQUESTS_PER_SECOND_LIMIT {
                return Err(ScopeError::Validation(format!(
                    "max_requests_per_second exceeds reasonable limit ({})",
                    crate::constants::MAX_REQUESTS_PER_SECOND_LIMIT
                )));
            }
        }

        Ok(())
    }

    pub fn from_file(path: &str) -> Result<Self, ScopeError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| ScopeError::FileRead(path.to_string(), e.to_string()))?;

        let scope: Scope = if path.ends_with(".yaml") || path.ends_with(".yml") {
            serde_yaml_neo::from_str(&content)
                .map_err(|e| ScopeError::Parse(path.to_string(), e.to_string()))?
        } else {
            toml::from_str(&content)
                .map_err(|e| ScopeError::Parse(path.to_string(), e.to_string()))?
        };

        Ok(scope)
    }

    fn has_ip_based_rules(&self) -> bool {
        self.allowed_targets
            .iter()
            .chain(self.excluded_targets.iter())
            .any(|rule| rule.cidr.is_some())
    }

    pub fn is_target_allowed(&self, target: &str) -> Result<bool, ScopeError> {
        let target_scope = if self.has_ip_based_rules() {
            let scope = TargetScope::parse(target)?;
            if scope.ip.is_none() {
                return Err(ScopeError::DnsResolution(
                    target.to_string(),
                    "DNS resolution failed with CIDR rules configured".to_string(),
                ));
            }
            scope
        } else {
            TargetScope::parse_hostname_only(target)?
        };

        if self.is_explicitly_excluded(&target_scope) {
            tracing::warn!(
                target = %target,
                "Target is explicitly excluded from scope"
            );
            return Ok(false);
        }

        if self.allowed_targets.is_empty() {
            if self.require_explicit_scope {
                tracing::warn!(
                    target = %target,
                    "No scope defined and explicit scope required"
                );
                return Ok(false);
            }
            // Block private IPs even when no scope rules are defined
            if let Some(ref ip) = target_scope.ip {
                if is_private_ip(ip) {
                    tracing::warn!(
                        target = %target,
                        "Private IP address blocked by security policy"
                    );
                    return Err(ScopeError::DnsResolution(
                        target.to_string(),
                        "Private IP address blocked by security policy".to_string(),
                    ));
                }
            }
            return Ok(true);
        }

        let allowed = self
            .allowed_targets
            .iter()
            .any(|rule| rule.matches(&target_scope));

        if !allowed {
            if let Some(ref ip) = target_scope.ip {
                if is_private_ip(ip) {
                    tracing::warn!(
                        target = %target,
                        "Private IP address not in allowed scope"
                    );
                    return Ok(false);
                }
            }
            tracing::warn!(
                target = %target,
                "Target is not in allowed scope"
            );
        }

        Ok(allowed)
    }

    /// Returns true if the target string matches any explicit exclusion rule.
    ///
    /// Used by policy enforcement to classify ExplicitExclusion denials separately
    /// from general "not in scope" denials, enabling precise downgrade logic in
    /// permissive profiles.
    pub fn is_excluded(&self, target: &str) -> bool {
        match TargetScope::parse_hostname_only(target) {
            Ok(ts) => self.is_explicitly_excluded(&ts),
            Err(_) => false,
        }
    }

    pub fn is_port_allowed(&self, port: u16) -> bool {
        if self.excluded_ports.contains(&port) {
            return false;
        }

        if let Some(ref allowed) = self.allowed_ports {
            return allowed.contains(&port);
        }

        true
    }

    fn is_explicitly_excluded(&self, target: &TargetScope) -> bool {
        self.excluded_targets
            .iter()
            .any(|rule| rule.matches(target))
    }

    pub fn validate_url(&self, url: &str) -> Result<bool, ScopeError> {
        let parsed =
            Url::parse(url).map_err(|e| ScopeError::InvalidUrl(url.to_string(), e.to_string()))?;

        let host = parsed
            .host_str()
            .ok_or_else(|| ScopeError::InvalidUrl(url.to_string(), "No host".to_string()))?;

        self.is_target_allowed(host)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeRule {
    #[serde(default)]
    pub pattern: String,

    #[serde(default)]
    pub cidr: Option<String>,

    #[serde(default)]
    pub description: Option<String>,
}

impl ScopeRule {
    pub fn new(pattern: String) -> Self {
        Self {
            pattern,
            cidr: None,
            description: None,
        }
    }

    pub fn with_cidr(cidr: String) -> Result<Self, ScopeError> {
        IpNetwork::from_str(&cidr)
            .map_err(|e| ScopeError::InvalidCidr(cidr.clone(), e.to_string()))?;

        Ok(Self {
            pattern: String::new(),
            cidr: Some(cidr),
            description: None,
        })
    }

    pub fn matches(&self, target: &TargetScope) -> bool {
        if let Some(ref cidr) = self.cidr {
            if let Some(ref ip) = target.ip {
                match IpNetwork::from_str(cidr) {
                    Ok(network) => return network.contains(*ip),
                    Err(e) => {
                        tracing::warn!(
                            cidr = %cidr,
                            error = %e,
                            "Failed to parse CIDR in scope rule, skipping match"
                        );
                        return false;
                    }
                }
            }
        }

        if !self.pattern.is_empty() {
            if self.pattern == "*" {
                return true;
            }

            if self.pattern.contains('/') {
                if let Some(ref ip) = target.ip {
                    match IpNetwork::from_str(&self.pattern) {
                        Ok(network) => return network.contains(*ip),
                        Err(e) => {
                            tracing::warn!(
                                pattern = %self.pattern,
                                error = %e,
                                "Failed to parse CIDR pattern in scope rule, skipping match"
                            );
                            return false;
                        }
                    }
                }
            }

            if self.pattern.starts_with("*.") {
                let suffix = &self.pattern[1..];
                return target.host.ends_with(suffix) || target.host == self.pattern[2..];
            }

            return target.host == self.pattern;
        }

        false
    }
}

#[derive(Debug, Clone)]
pub struct TargetScope {
    pub host: String,
    pub ip: Option<IpAddr>,
}

impl TargetScope {
    pub fn parse(target: &str) -> Result<Self, ScopeError> {
        let target = target.trim();

        if target.is_empty() {
            return Err(ScopeError::InvalidTarget(target.to_string()));
        }

        if let Ok(ip) = IpAddr::from_str(target) {
            // Private IP check is deferred to scope rule evaluation in is_target_allowed()
            return Ok(Self {
                host: target.to_string(),
                ip: Some(ip),
            });
        }

        if let Ok(url) = Url::parse(target) {
            let host = url
                .host_str()
                .ok_or_else(|| ScopeError::InvalidTarget(target.to_string()))?
                .to_string();

            let ip = Some(Self::resolve_host(&host).map_err(|e| {
                ScopeError::InvalidTarget(format!("DNS resolution failed for '{}': {}", host, e))
            })?);

            return Ok(Self { host, ip });
        }

        if target.contains('/') || target.contains(' ') {
            return Err(ScopeError::InvalidTarget(target.to_string()));
        }

        let host = target.split(':').next().unwrap_or(target).to_string();

        if host.is_empty() {
            return Err(ScopeError::InvalidTarget(target.to_string()));
        }

        let ip = match Self::resolve_host(&host) {
            Ok(ip) => Some(ip),
            Err(e) => {
                tracing::debug!("DNS resolution failed for '{}': {}", host, e);
                None
            }
        };

        Ok(Self { host, ip })
    }

    pub fn parse_hostname_only(target: &str) -> Result<Self, ScopeError> {
        let target = target.trim();

        if target.is_empty() {
            return Err(ScopeError::InvalidTarget(target.to_string()));
        }

        if let Ok(ip) = IpAddr::from_str(target) {
            // Private IP check is deferred to scope rule evaluation in is_target_allowed()
            return Ok(Self {
                host: target.to_string(),
                ip: Some(ip),
            });
        }

        if let Ok(url) = Url::parse(target) {
            let host = url
                .host_str()
                .ok_or_else(|| ScopeError::InvalidTarget(target.to_string()))?
                .to_string();

            let ip = Self::resolve_host(&host).ok();
            return Ok(Self { host, ip });
        }

        if target.contains('/') || target.contains(' ') {
            return Err(ScopeError::InvalidTarget(target.to_string()));
        }

        let host = target.split(':').next().unwrap_or(target).to_string();

        if host.is_empty() {
            return Err(ScopeError::InvalidTarget(target.to_string()));
        }

        let ip = Self::resolve_host(&host).ok();

        Ok(Self { host, ip })
    }

    fn resolve_host(host: &str) -> Result<IpAddr, ScopeError> {
        use std::net::ToSocketAddrs;

        let addrs: Vec<_> = (host, 0)
            .to_socket_addrs()
            .map_err(|e| ScopeError::DnsResolution(host.to_string(), e.to_string()))?
            .collect();

        let ip = addrs.first().map(|a| a.ip()).ok_or_else(|| {
            ScopeError::DnsResolution(host.to_string(), "No addresses found".to_string())
        })?;

        if ip.is_loopback() {
            return Err(ScopeError::DnsResolution(
                host.to_string(),
                "Resolved to loopback address blocked by security policy".to_string(),
            ));
        }

        Ok(ip)
    }
}

pub fn is_private_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(ipv4) => {
            let octets = ipv4.octets();
            octets[0] == 10
                || (octets[0] == 172 && (16..=31).contains(&octets[1]))
                || (octets[0] == 192 && octets[1] == 168)
                || (octets[0] == 169 && octets[1] == 254)
                || (octets[0] == 127)
        }
        IpAddr::V6(ipv6) => {
            ipv6.is_loopback()
                || (ipv6.segments()[0] & 0xfe00) == 0xfc00
                || (ipv6.segments()[0] & 0xffc0) == 0xfe80
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ScopeError {
    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Failed to read scope file '{0}': {1}")]
    FileRead(String, String),

    #[error("Failed to parse scope file '{0}': {1}")]
    Parse(String, String),

    #[error("Invalid URL '{0}': {1}")]
    InvalidUrl(String, String),

    #[error("Invalid CIDR '{0}': {1}")]
    InvalidCidr(String, String),

    #[error("Invalid target '{0}'")]
    InvalidTarget(String),

    #[error("DNS resolution failed for '{0}': {1}")]
    DnsResolution(String, String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_rule_wildcard() {
        let rule = ScopeRule::new("*.example.com".to_string());

        assert!(rule.matches(&TargetScope {
            host: "sub.example.com".to_string(),
            ip: None
        }));
        assert!(rule.matches(&TargetScope {
            host: "example.com".to_string(),
            ip: None
        }));
        assert!(!rule.matches(&TargetScope {
            host: "other.com".to_string(),
            ip: None
        }));
    }

    #[test]
    fn test_scope_allow() {
        let mut scope = Scope::new();
        scope
            .allowed_targets
            .push(ScopeRule::new("example.com".to_string()));

        assert!(scope.is_target_allowed("example.com").unwrap());
        assert!(!scope.is_target_allowed("other.com").unwrap());
    }

    #[test]
    fn test_scope_exclude() {
        let mut scope = Scope::new();
        scope
            .excluded_targets
            .push(ScopeRule::new("internal.example.com".to_string()));

        assert!(!scope.is_target_allowed("internal.example.com").unwrap());
    }

    #[test]
    fn test_scope_rule_cidr_from_pattern() {
        let rule = ScopeRule::new("10.0.0.0/8".to_string());

        let target1 = TargetScope {
            host: "10.255.255.255".to_string(),
            ip: Some("10.255.255.255".parse().unwrap()),
        };
        assert!(
            rule.matches(&target1),
            "10.255.255.255 should be in 10.0.0.0/8"
        );

        let target2 = TargetScope {
            host: "11.0.0.1".to_string(),
            ip: Some("11.0.0.1".parse().unwrap()),
        };
        assert!(
            !rule.matches(&target2),
            "11.0.0.1 should NOT be in 10.0.0.0/8"
        );
    }

    #[test]
    fn test_scope_rule_cidr_explicit() {
        let rule = ScopeRule::with_cidr("10.0.0.0/8".to_string()).unwrap();

        let target1 = TargetScope {
            host: "10.255.255.255".to_string(),
            ip: Some("10.255.255.255".parse().unwrap()),
        };
        assert!(
            rule.matches(&target1),
            "10.255.255.255 should be in 10.0.0.0/8"
        );

        let target2 = TargetScope {
            host: "11.0.0.1".to_string(),
            ip: Some("11.0.0.1".parse().unwrap()),
        };
        assert!(
            !rule.matches(&target2),
            "11.0.0.1 should NOT be in 10.0.0.0/8"
        );
    }

    #[test]
    fn test_is_private_ip_ipv6_ranges() {
        let ula_fc00: IpAddr = "fc00::1".parse().unwrap();
        let ula_fd00: IpAddr = "fd00::1".parse().unwrap();
        let link_local: IpAddr = "fe80::1".parse().unwrap();
        let global: IpAddr = "2001:4860:4860::8888".parse().unwrap();

        assert!(is_private_ip(&ula_fc00));
        assert!(is_private_ip(&ula_fd00));
        assert!(is_private_ip(&link_local));
        assert!(!is_private_ip(&global));
    }

    #[test]
    fn test_scope_toml_parse_sample() {
        let toml_str = r#"
require_explicit_scope = true
max_requests_per_second = 100
excluded_ports = [22, 3389]

[[allowed_targets]]
pattern = "*.example.com"
description = "Production web applications"

[[allowed_targets]]
cidr = "10.0.0.0/8"
description = "Internal network"

[[allowed_targets]]
pattern = "localhost"
description = "Local development"

[[excluded_targets]]
pattern = "admin.example.com"
description = "Admin panel - excluded by policy"

[[excluded_targets]]
cidr = "10.0.0.1/32"
description = "Critical database server"
"#;

        let scope: Scope = toml::from_str(toml_str).unwrap();

        assert!(scope.require_explicit_scope);
        assert_eq!(scope.max_requests_per_second, Some(100));
        assert_eq!(scope.allowed_targets.len(), 3);
        assert_eq!(scope.excluded_targets.len(), 2);
        assert_eq!(scope.excluded_ports, vec![22, 3389]);

        // Verify allowed target fields
        assert_eq!(scope.allowed_targets[0].pattern, "*.example.com");
        assert!(scope.allowed_targets[0].cidr.is_none());
        assert_eq!(scope.allowed_targets[1].cidr.as_deref(), Some("10.0.0.0/8"));
        assert!(scope.allowed_targets[1].pattern.is_empty());
        assert_eq!(scope.allowed_targets[2].pattern, "localhost");

        // Verify excluded target fields
        assert_eq!(scope.excluded_targets[0].pattern, "admin.example.com");
        assert_eq!(
            scope.excluded_targets[1].cidr.as_deref(),
            Some("10.0.0.1/32")
        );
    }

    #[test]
    fn test_loaded_scope_default_empty_is_not_explicit() {
        let loaded = LoadedScope::default_empty();
        assert!(!loaded.is_explicit_manifest());
        assert_eq!(loaded.source, ScopeSource::DefaultEmpty);
    }

    #[test]
    fn test_loaded_scope_explicit_is_explicit() {
        let scope = Scope::default();
        let loaded = LoadedScope::explicit(scope, ScopeSource::CliScopeFile, None);
        assert!(loaded.is_explicit_manifest());
        assert_eq!(loaded.source, ScopeSource::CliScopeFile);
    }

    #[test]
    fn test_scope_toml_parse_empty() {
        let toml_str = r#"
require_explicit_scope = false
"#;

        let scope: Scope = toml::from_str(toml_str).unwrap();
        assert!(!scope.require_explicit_scope);
        assert!(scope.allowed_targets.is_empty());
        assert!(scope.excluded_targets.is_empty());
    }
}
