//! Pure scope/authorization data model (no I/O).
//!
//! This module owns the serializable scope data types and the deterministic
//! matching algorithms over **already-supplied** destination facts. It
//! performs no DNS resolution, no filesystem access, and no transport work:
//!
//! - DNS acquisition lives in the engine resolver bridge
//!   (`eggsec::config::scope_resolver` + `eggsec::policy_bridge::resolver`);
//! - the `NetworkAuthority` transport checkpoint lives in
//!   `eggsec::policy_bridge::transport` (`ScopeAuthority`);
//! - `ScopeSpec` conversion lives in `eggsec::config::scope_spec`.
//!
//! Engine bridges resolve a target string into a [`TargetScope`] fact record
//! first, then call [`Scope::evaluate_facts`] / [`TargetScope::evaluate_addresses`].

use ipnetwork::IpNetwork;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::str::FromStr;

pub use super::address::{classify_address, is_private_ip, AddressClass};

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
            let mut seen = std::collections::HashSet::new();
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
            // Mirrors `eggsec_core::constants::MAX_REQUESTS_PER_SECOND_LIMIT`.
            // Kept as a literal so this leaf crate stays dependency-light.
            if rate > 10_000 {
                return Err(ScopeError::Validation(format!(
                    "max_requests_per_second exceeds reasonable limit ({})",
                    10_000
                )));
            }
        }

        Ok(())
    }

    pub fn has_ip_based_rules(&self) -> bool {
        self.allowed_targets
            .iter()
            .chain(self.excluded_targets.iter())
            .any(|rule| rule.cidr.is_some())
    }

    /// Pure authorization over already-resolved destination facts.
    ///
    /// The caller (engine resolver bridge or transport authority) supplies a
    /// [`TargetScope`] built from DNS facts it acquired; this function only
    /// decides. Mirrors the post-resolution branch of the historical
    /// `is_target_allowed_with_resolver` without performing any I/O:
    ///
    /// - explicit exclusions deny first;
    /// - empty allowlists fall back to default policy (explicit scope required
    ///   denies; otherwise non-public non-loopback addresses deny);
    /// - otherwise every resolved address must be allowed and none excluded
    ///   (mixed authorized/unauthorized answers deny); with no resolved
    ///   addresses, hostname-pattern matching decides.
    pub fn evaluate_facts(&self, target: &TargetScope) -> Result<bool, ScopeError> {
        if self.is_explicitly_excluded(target) {
            return Ok(false);
        }

        if self.allowed_targets.is_empty() {
            if self.require_explicit_scope {
                return Ok(false);
            }
            // Block non-public addresses even when no scope rules are defined.
            // Loopback addresses are exempt — they are inherently local and
            // represent no scope violation on any machine.
            let any_blocked = target
                .resolved_addresses
                .iter()
                .map(classify_address)
                .any(|class| class != AddressClass::Loopback && class.is_non_public());
            if any_blocked {
                return Ok(false);
            }
            return Ok(true);
        }

        // Use all-address evaluation when facts are available.
        let allowed = if !target.resolved_addresses.is_empty() {
            let (all_allowed, any_excluded, _) =
                target.evaluate_addresses(&self.allowed_targets, &self.excluded_targets);
            if any_excluded {
                return Ok(false);
            }
            all_allowed
        } else {
            self.allowed_targets.iter().any(|rule| rule.matches(target))
        };

        if !allowed {
            if let Some(ref ip) = target.ip {
                let class = classify_address(ip);
                if class.is_non_public() {
                    return Ok(false);
                }
            }
        }

        Ok(allowed)
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

    /// Pure exclusion check over supplied facts (hostname patterns + CIDR).
    pub fn is_explicitly_excluded(&self, target: &TargetScope) -> bool {
        // Check hostname pattern exclusions
        let hostname_excluded = self
            .excluded_targets
            .iter()
            .any(|rule| rule.matches(target));

        if hostname_excluded {
            return true;
        }

        // Check all resolved addresses against exclusion rules
        if !target.resolved_addresses.is_empty() {
            for addr in &target.resolved_addresses {
                let excluded = self.excluded_targets.iter().any(|rule| {
                    rule.cidr
                        .as_ref()
                        .and_then(|cidr| {
                            IpNetwork::from_str(cidr)
                                .ok()
                                .map(|net| net.contains(*addr))
                        })
                        .unwrap_or(false)
                        || {
                            !rule.pattern.is_empty()
                                && rule.pattern.contains('/')
                                && IpNetwork::from_str(&rule.pattern)
                                    .ok()
                                    .map(|net| net.contains(*addr))
                                    .unwrap_or(false)
                        }
                });
                if excluded {
                    return true;
                }
            }
        }

        false
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
        // Check CIDR rules against all resolved addresses
        if let Some(ref cidr) = self.cidr {
            if let Ok(network) = IpNetwork::from_str(cidr) {
                // Check all resolved addresses, not just the first
                for addr in &target.resolved_addresses {
                    if network.contains(*addr) {
                        return true;
                    }
                }
                // Fallback to single ip for backward compatibility
                if let Some(ip) = target.ip {
                    if network.contains(ip) {
                        return true;
                    }
                }
            }
            // Unparseable CIDR: skip (no log in the I/O-free kernel;
            // engine bridges may log on their side).
        }

        if !self.pattern.is_empty() {
            if self.pattern == "*" {
                return true;
            }

            if self.pattern.contains('/') {
                // CIDR pattern — check all resolved addresses
                if let Ok(network) = IpNetwork::from_str(&self.pattern) {
                    for addr in &target.resolved_addresses {
                        if network.contains(*addr) {
                            return true;
                        }
                    }
                    if let Some(ip) = target.ip {
                        if network.contains(ip) {
                            return true;
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

    /// Check whether a single IP address matches this rule's CIDR or pattern.
    ///
    /// Used by [`TargetScope::evaluate_addresses`] for per-address evaluation.
    fn matches_address(&self, addr: IpAddr) -> bool {
        if let Some(ref cidr) = self.cidr {
            if let Ok(network) = IpNetwork::from_str(cidr) {
                if network.contains(addr) {
                    return true;
                }
            }
        }

        if !self.pattern.is_empty() {
            if self.pattern == "*" {
                return true;
            }

            if self.pattern.contains('/') {
                if let Ok(network) = IpNetwork::from_str(&self.pattern) {
                    if network.contains(addr) {
                        return true;
                    }
                }
            }
        }

        false
    }
}

/// Resolved destination facts supplied by the caller.
///
/// The resolver (engine bridge) reports facts; policy decides authorization.
/// Construct directly from already-known addresses — no DNS is performed here.
#[derive(Debug, Clone)]
pub struct TargetScope {
    pub host: String,
    pub ip: Option<IpAddr>,
    /// All unique addresses resolved for this target (empty when no resolution needed).
    pub resolved_addresses: Vec<IpAddr>,
}

impl TargetScope {
    /// Facts for a literal IP (no resolution needed).
    pub fn for_ip(ip: IpAddr) -> Self {
        Self {
            host: ip.to_string(),
            ip: Some(ip),
            resolved_addresses: vec![ip],
        }
    }

    /// Facts for a hostname with caller-supplied resolved addresses.
    pub fn for_host_with_addresses(host: impl Into<String>, addresses: Vec<IpAddr>) -> Self {
        let host = host.into();
        let ip = addresses.first().copied();
        Self {
            host,
            ip,
            resolved_addresses: addresses,
        }
    }

    /// Facts for a hostname with no resolved addresses (pattern-only matching).
    pub fn for_host_without_addresses(host: impl Into<String>) -> Self {
        Self {
            host: host.into(),
            ip: None,
            resolved_addresses: Vec::new(),
        }
    }

    /// Evaluate whether all resolved addresses match the given scope rules.
    ///
    /// For strict surfaces: every address must match at least one allowed rule
    /// and no address may match an exclusion rule.
    /// Returns (all_allowed, any_excluded, evaluated_classes).
    pub fn evaluate_addresses(
        &self,
        allowed_rules: &[ScopeRule],
        excluded_rules: &[ScopeRule],
    ) -> (bool, bool, Vec<AddressClass>) {
        if self.resolved_addresses.is_empty() {
            // No addresses to evaluate — caller decides how to handle
            return (false, false, Vec::new());
        }

        let classes: Vec<AddressClass> = self
            .resolved_addresses
            .iter()
            .map(classify_address)
            .collect();

        let any_excluded = self.resolved_addresses.iter().any(|addr| {
            excluded_rules.iter().any(|rule| {
                rule.cidr
                    .as_ref()
                    .and_then(|cidr| {
                        IpNetwork::from_str(cidr)
                            .ok()
                            .map(|net| net.contains(*addr))
                    })
                    .unwrap_or(false)
                    || {
                        // Also check pattern-based CIDR rules
                        !rule.pattern.is_empty()
                            && rule.pattern.contains('/')
                            && IpNetwork::from_str(&rule.pattern)
                                .ok()
                                .map(|net| net.contains(*addr))
                                .unwrap_or(false)
                    }
            })
        });

        let all_allowed = if allowed_rules.is_empty() {
            // No allowed rules — everything is allowed (unless excluded)
            !any_excluded
        } else {
            // Check if ALL addresses match at least one rule.
            // For hostname patterns, the host must match; for CIDR rules, addresses must match.
            self.resolved_addresses.iter().all(|addr| {
                allowed_rules.iter().any(|rule| {
                    rule.matches_address(*addr) || {
                        // For hostname patterns, check if the host matches
                        let temp = TargetScope {
                            host: self.host.clone(),
                            ip: Some(*addr),
                            resolved_addresses: vec![],
                        };
                        rule.matches(&temp)
                    }
                })
            })
        };

        (all_allowed, any_excluded, classes)
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
            ip: None,
            resolved_addresses: Vec::new(),
        }));
        assert!(rule.matches(&TargetScope {
            host: "example.com".to_string(),
            ip: None,
            resolved_addresses: Vec::new(),
        }));
        assert!(!rule.matches(&TargetScope {
            host: "other.com".to_string(),
            ip: None,
            resolved_addresses: Vec::new(),
        }));
    }

    #[test]
    fn test_scope_rule_cidr_from_pattern() {
        let rule = ScopeRule::new("10.0.0.0/8".to_string());

        let target1 = TargetScope {
            host: "10.255.255.255".to_string(),
            ip: Some("10.255.255.255".parse().unwrap()),
            resolved_addresses: vec!["10.255.255.255".parse().unwrap()],
        };
        assert!(
            rule.matches(&target1),
            "10.255.255.255 should be in 10.0.0.0/8"
        );

        let target2 = TargetScope {
            host: "11.0.0.1".to_string(),
            ip: Some("11.0.0.1".parse().unwrap()),
            resolved_addresses: vec!["11.0.0.1".parse().unwrap()],
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
            resolved_addresses: vec!["10.255.255.255".parse().unwrap()],
        };
        assert!(
            rule.matches(&target1),
            "10.255.255.255 should be in 10.0.0.0/8"
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
    fn test_evaluate_addresses_all_public_allowed() {
        let scope = Scope {
            allowed_targets: vec![ScopeRule::new("*".to_string())],
            ..Default::default()
        };
        let ts = TargetScope {
            host: "example.com".to_string(),
            ip: Some("93.184.216.34".parse::<IpAddr>().unwrap()),
            resolved_addresses: vec![
                "93.184.216.34".parse::<IpAddr>().unwrap(),
                "93.184.216.35".parse::<IpAddr>().unwrap(),
            ],
        };
        let (all_allowed, any_excluded, classes) =
            ts.evaluate_addresses(&scope.allowed_targets, &scope.excluded_targets);
        assert!(all_allowed);
        assert!(!any_excluded);
        assert!(classes.iter().all(|c| *c == AddressClass::Public));
    }

    #[test]
    fn test_evaluate_addresses_mixed_public_private_cidr() {
        let scope = Scope {
            allowed_targets: vec![ScopeRule::new("10.0.0.0/8".to_string())],
            ..Default::default()
        };
        let ts = TargetScope {
            host: "mixed.example.com".to_string(),
            ip: Some("10.0.0.1".parse::<IpAddr>().unwrap()),
            resolved_addresses: vec![
                "10.0.0.1".parse::<IpAddr>().unwrap(),
                "93.184.216.34".parse::<IpAddr>().unwrap(),
            ],
        };
        let (all_allowed, any_excluded, _classes) =
            ts.evaluate_addresses(&scope.allowed_targets, &scope.excluded_targets);
        // Not all addresses are in 10.0.0.0/8
        assert!(!all_allowed);
        assert!(!any_excluded);
    }

    #[test]
    fn test_evaluate_addresses_exclusion_wins() {
        let scope = Scope {
            allowed_targets: vec![ScopeRule::new("*".to_string())],
            excluded_targets: vec![ScopeRule::new("10.0.0.0/8".to_string())],
            ..Default::default()
        };
        let ts = TargetScope {
            host: "excluded.example.com".to_string(),
            ip: Some("10.0.0.1".parse::<IpAddr>().unwrap()),
            resolved_addresses: vec![
                "10.0.0.1".parse::<IpAddr>().unwrap(),
                "93.184.216.34".parse::<IpAddr>().unwrap(),
            ],
        };
        let (all_allowed, any_excluded, _classes) =
            ts.evaluate_addresses(&scope.allowed_targets, &scope.excluded_targets);
        // All addresses are allowed by wildcard
        assert!(all_allowed);
        // But one is excluded
        assert!(any_excluded);
    }

    #[test]
    fn test_evaluate_addresses_empty_returns_false() {
        let ts = TargetScope {
            host: "unresolvable.host".to_string(),
            ip: None,
            resolved_addresses: Vec::new(),
        };
        let (all_allowed, any_excluded, classes) = ts.evaluate_addresses(&[], &[]);
        assert!(!all_allowed);
        assert!(!any_excluded);
        assert!(classes.is_empty());
    }

    #[test]
    fn evaluate_facts_explicit_exclusion_denies() {
        let mut scope = Scope::new();
        scope
            .excluded_targets
            .push(ScopeRule::new("internal.example.com".to_string()));
        let facts = TargetScope::for_host_without_addresses("internal.example.com");
        assert_eq!(scope.evaluate_facts(&facts).unwrap(), false);
    }

    #[test]
    fn evaluate_facts_hostname_allowlist() {
        let mut scope = Scope::new();
        scope
            .allowed_targets
            .push(ScopeRule::new("example.com".to_string()));
        let allowed = TargetScope::for_host_without_addresses("example.com");
        let denied = TargetScope::for_host_without_addresses("other.com");
        assert_eq!(scope.evaluate_facts(&allowed).unwrap(), true);
        assert_eq!(scope.evaluate_facts(&denied).unwrap(), false);
    }

    #[test]
    fn evaluate_facts_mixed_answers_deny() {
        let mut scope = Scope::new();
        scope
            .allowed_targets
            .push(ScopeRule::with_cidr("93.184.216.0/24".to_string()).unwrap());
        let mixed = TargetScope::for_host_with_addresses(
            "mixed.example",
            vec![
                "93.184.216.34".parse().unwrap(),
                "203.0.113.99".parse().unwrap(),
            ],
        );
        assert_eq!(scope.evaluate_facts(&mixed).unwrap(), false);
    }

    #[test]
    fn evaluate_facts_default_policy_blocks_private_allows_loopback() {
        let scope = Scope::new();
        let private = TargetScope::for_ip("10.0.0.1".parse().unwrap());
        let loopback = TargetScope::for_ip("127.0.0.1".parse().unwrap());
        let public = TargetScope::for_ip("8.8.8.8".parse().unwrap());
        assert_eq!(scope.evaluate_facts(&private).unwrap(), false);
        assert_eq!(scope.evaluate_facts(&loopback).unwrap(), true);
        assert_eq!(scope.evaluate_facts(&public).unwrap(), true);
    }

    #[test]
    fn evaluate_facts_direct_ip_cidr() {
        let mut scope = Scope::new();
        scope
            .allowed_targets
            .push(ScopeRule::with_cidr("10.0.0.0/8".to_string()).unwrap());
        let inside = TargetScope::for_ip("10.0.0.5".parse().unwrap());
        let outside = TargetScope::for_ip("11.0.0.5".parse().unwrap());
        assert_eq!(scope.evaluate_facts(&inside).unwrap(), true);
        assert_eq!(scope.evaluate_facts(&outside).unwrap(), false);
    }

    #[test]
    fn scope_validate_rejects_duplicates_and_bad_rate() {
        let mut scope = Scope::new();
        scope.allowed_ports = Some(vec![80, 80]);
        assert!(scope.validate().is_err());
        let mut scope = Scope::new();
        scope.max_requests_per_second = Some(0);
        assert!(scope.validate().is_err());
        let mut scope = Scope::new();
        scope.max_requests_per_second = Some(100);
        assert!(scope.validate().is_ok());
    }
}
