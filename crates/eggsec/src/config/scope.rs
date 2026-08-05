use ipnetwork::IpNetwork;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::str::FromStr;
use std::sync::Arc;
use url::Url;

/// Classification of an IP address based on RFC definitions and Eggsec policy.
///
/// Used by scope evaluation to determine authorization for each resolved address.
/// The resolver reports facts; policy decides whether they are authorized.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AddressClass {
    /// Public routable address (e.g. 8.8.8.8, 2001:4860:4860::8888).
    Public,
    /// Loopback address (127.0.0.0/8, ::1, IPv4-mapped loopback).
    Loopback,
    /// RFC 1918 private (10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16) or IPv6 ULA (fc00::/7).
    Private,
    /// Link-local address (169.254.0.0/16, fe80::/10).
    LinkLocal,
    /// IPv4-mapped IPv6 loopback (::ffff:127.0.0.1).
    IPv4MappedLoopback,
    /// Unspecified address (0.0.0.0, ::).
    Unspecified,
    /// Multicast address (224.0.0.0/4, ff00::/8).
    Multicast,
}

impl AddressClass {
    /// Stable kebab-case string for audit and decision records.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Loopback => "loopback",
            Self::Private => "private",
            Self::LinkLocal => "link-local",
            Self::IPv4MappedLoopback => "ipv4-mapped-loopback",
            Self::Unspecified => "unspecified",
            Self::Multicast => "multicast",
        }
    }

    /// Returns `true` if this address class is non-public (loopback, private, link-local,
    /// IPv4-mapped loopback, unspecified, or multicast).
    ///
    /// Used by scope authorization to determine if an address requires explicit scope rules.
    /// Public addresses are allowed by default; non-public addresses require explicit authorization.
    pub fn is_non_public(&self) -> bool {
        !matches!(self, Self::Public)
    }
}

impl std::fmt::Display for AddressClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Classify an IP address into its [`AddressClass`].
///
/// This function reports facts only — it does not authorize or reject.
/// Policy evaluation uses the class to determine scope compliance.
pub fn classify_address(ip: &IpAddr) -> AddressClass {
    match ip {
        IpAddr::V4(v4) => {
            let octets = v4.octets();
            if v4.is_loopback() || octets[0] == 127 {
                AddressClass::Loopback
            } else if octets[0] == 0 {
                AddressClass::Unspecified
            } else if octets[0] >= 224 && octets[0] <= 239 {
                AddressClass::Multicast
            } else if octets[0] == 10
                || (octets[0] == 172 && (16..=31).contains(&octets[1]))
                || (octets[0] == 192 && octets[1] == 168)
            {
                AddressClass::Private
            } else if octets[0] == 169 && octets[1] == 254 {
                AddressClass::LinkLocal
            } else {
                AddressClass::Public
            }
        }
        IpAddr::V6(v6) => {
            // Check for IPv4-mapped addresses first (before loopback check)
            if let Some(v4) = v6.to_ipv4_mapped() {
                // IPv4-mapped IPv6: ::ffff:a.b.c.d — classify the embedded v4
                // but use IPv4MappedLoopback for mapped loopback addresses
                if v4.is_loopback() {
                    AddressClass::IPv4MappedLoopback
                } else {
                    classify_address(&IpAddr::V4(v4))
                }
            } else if v6.is_loopback() {
                AddressClass::Loopback
            } else if v6.is_unspecified() {
                AddressClass::Unspecified
            } else if (v6.segments()[0] & 0xff00) == 0xff00 {
                AddressClass::Multicast
            } else if (v6.segments()[0] & 0xfe00) == 0xfc00 {
                AddressClass::Private
            } else if (v6.segments()[0] & 0xffc0) == 0xfe80 {
                AddressClass::LinkLocal
            } else {
                AddressClass::Public
            }
        }
    }
}

/// Result of resolving a hostname to IP addresses.
///
/// The resolver reports facts; policy decides whether they are authorized.
#[derive(Debug, Clone)]
pub struct ResolutionResult {
    /// The normalized hostname that was resolved.
    pub hostname: String,
    /// All unique IP addresses returned by the resolver, in deterministic order.
    pub addresses: Vec<IpAddr>,
    /// Resolution error, if the resolver failed partially or completely.
    pub error: Option<String>,
}

impl ResolutionResult {
    /// Returns `true` if the resolver returned at least one address.
    pub fn has_addresses(&self) -> bool {
        !self.addresses.is_empty()
    }

    /// Returns the first resolved address, if any.
    pub fn first_address(&self) -> Option<IpAddr> {
        self.addresses.first().copied()
    }
}

/// Trait for DNS resolution, enabling deterministic unit tests and shared engine behavior.
///
/// The default implementation uses `std::net::ToSocketAddrs`. Tests can provide
/// a fake resolver that returns predetermined addresses without network access.
pub trait HostResolver: Send + Sync {
    /// Resolve a hostname to all unique IP addresses.
    ///
    /// Returns all addresses the system resolver returns, preserving deterministic
    /// ordering (sorted for audit/testing). Does not reject any address classes —
    /// policy decides authorization.
    fn resolve_all(&self, host: &str) -> ResolutionResult;
}

/// Default resolver using `std::net::ToSocketAddrs`.
///
/// Collects all unique addresses and returns them sorted for deterministic
/// ordering. Does not reject loopback, private, or other special addresses —
/// those are policy concerns, not resolver concerns.
pub struct SystemResolver;

impl HostResolver for SystemResolver {
    fn resolve_all(&self, host: &str) -> ResolutionResult {
        use std::collections::BTreeSet;
        use std::net::ToSocketAddrs;

        let hostname = host.to_string();

        let addrs: Vec<_> = match (host, 0u16).to_socket_addrs() {
            Ok(iter) => iter.collect(),
            Err(e) => {
                return ResolutionResult {
                    hostname,
                    addresses: Vec::new(),
                    error: Some(e.to_string()),
                };
            }
        };

        let mut unique: BTreeSet<IpAddr> = BTreeSet::new();
        for sock_addr in &addrs {
            unique.insert(sock_addr.ip());
        }

        ResolutionResult {
            hostname,
            addresses: unique.into_iter().collect(),
            error: None,
        }
    }
}

/// Create the default host resolver (system DNS).
pub fn default_resolver() -> Arc<dyn HostResolver> {
    Arc::new(SystemResolver)
}

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

impl From<&LoadedScope> for eggsec_runtime::SessionScope {
    fn from(loaded: &LoadedScope) -> Self {
        let source = match loaded.source {
            ScopeSource::DefaultEmpty => "default-empty",
            ScopeSource::ConfigFile => "config",
            ScopeSource::CliScopeFile => "cli",
            ScopeSource::GeneratedPreset => "preset",
        };
        Self {
            is_explicit: loaded.is_explicit_manifest(),
            source: source.to_string(),
            path: loaded.path.clone(),
        }
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

    pub fn has_ip_based_rules(&self) -> bool {
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
            // Block non-public addresses even when no scope rules are defined.
            // Loopback addresses are exempt — they are inherently local and
            // represent no scope violation on any machine.
            if let Some(ref ip) = target_scope.ip {
                let class = classify_address(ip);
                if class != AddressClass::Loopback && class.is_non_public() {
                    tracing::warn!(
                        target = %target,
                        address_class = %class,
                        "Non-public IP address blocked by security policy"
                    );
                    return Ok(false);
                }
            }
            return Ok(true);
        }

        // Use all-address evaluation when available
        let allowed = if !target_scope.resolved_addresses.is_empty() {
            let (all_allowed, any_excluded, classes) =
                target_scope.evaluate_addresses(&self.allowed_targets, &self.excluded_targets);
            if any_excluded {
                tracing::warn!(
                    target = %target,
                    "One or more resolved addresses match exclusion rules"
                );
                return Ok(false);
            }
            if !all_allowed {
                // Log the address classes for diagnostics
                let class_summary: Vec<&str> = classes.iter().map(|c| c.as_str()).collect();
                tracing::warn!(
                    target = %target,
                    resolved_classes = ?class_summary,
                    "Not all resolved addresses are in allowed scope"
                );
            }
            all_allowed
        } else {
            self.allowed_targets
                .iter()
                .any(|rule| rule.matches(&target_scope))
        };

        if !allowed {
            if let Some(ref ip) = target_scope.ip {
                let class = classify_address(ip);
                if class.is_non_public() {
                    tracing::warn!(
                        target = %target,
                        address_class = %class,
                        "Non-public IP address not in allowed scope"
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
            } else {
                tracing::warn!(
                    cidr = %cidr,
                    "Failed to parse CIDR in scope rule, skipping match"
                );
            }
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
                } else {
                    tracing::warn!(
                        pattern = %self.pattern,
                        "Failed to parse CIDR pattern in scope rule, skipping match"
                    );
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

#[derive(Debug, Clone)]
pub struct TargetScope {
    pub host: String,
    pub ip: Option<IpAddr>,
    /// All unique addresses resolved for this target (empty when no resolution needed).
    pub resolved_addresses: Vec<IpAddr>,
}

impl TargetScope {
    /// Parse a target with full DNS resolution.
    ///
    /// Returns all unique resolved addresses. DNS resolution errors for
    /// hostnames (not literals) result in `ip: None` with an empty
    /// `resolved_addresses` — callers decide whether to fail.
    pub fn parse(target: &str) -> Result<Self, ScopeError> {
        Self::parse_with_resolver(target, &SystemResolver)
    }

    /// Parse a target with full DNS resolution using a custom resolver.
    pub fn parse_with_resolver(
        target: &str,
        resolver: &dyn HostResolver,
    ) -> Result<Self, ScopeError> {
        let target = target.trim();

        if target.is_empty() {
            return Err(ScopeError::InvalidTarget(target.to_string()));
        }

        // Literal IP — no resolution needed
        if let Ok(ip) = IpAddr::from_str(target) {
            return Ok(Self {
                host: target.to_string(),
                ip: Some(ip),
                resolved_addresses: vec![ip],
            });
        }

        // URL form
        if let Ok(url) = Url::parse(target) {
            let host = url
                .host_str()
                .ok_or_else(|| ScopeError::InvalidTarget(target.to_string()))?
                .to_string();

            let result = resolver.resolve_all(&host);
            let addresses = result.addresses;

            let ip = if addresses.is_empty() {
                if let Some(ref err) = result.error {
                    tracing::debug!(
                        host = %host,
                        error = %err,
                        "DNS resolution failed for URL host"
                    );
                }
                None
            } else {
                Some(addresses[0])
            };

            return Ok(Self {
                host,
                ip,
                resolved_addresses: addresses,
            });
        }

        // Reject paths and other ambiguous forms
        if target.contains('/') || target.contains(' ') {
            return Err(ScopeError::InvalidTarget(target.to_string()));
        }

        let host = target.split(':').next().unwrap_or(target).to_string();

        if host.is_empty() {
            return Err(ScopeError::InvalidTarget(target.to_string()));
        }

        let result = resolver.resolve_all(&host);
        let addresses = result.addresses;

        let ip = if addresses.is_empty() {
            if let Some(ref err) = result.error {
                tracing::debug!(
                    host = %host,
                    error = %err,
                    "DNS resolution failed for hostname"
                );
            }
            None
        } else {
            Some(addresses[0])
        };

        Ok(Self {
            host,
            ip,
            resolved_addresses: addresses,
        })
    }

    /// Parse a target for hostname-only matching (no DNS required for scope checks
    /// when CIDR rules are absent).
    ///
    /// Resolution is attempted but failures are non-fatal — `ip` and
    /// `resolved_addresses` may be empty.
    pub fn parse_hostname_only(target: &str) -> Result<Self, ScopeError> {
        Self::parse_hostname_only_with_resolver(target, &SystemResolver)
    }

    /// Parse a target for hostname-only matching using a custom resolver.
    pub fn parse_hostname_only_with_resolver(
        target: &str,
        resolver: &dyn HostResolver,
    ) -> Result<Self, ScopeError> {
        let target = target.trim();

        if target.is_empty() {
            return Err(ScopeError::InvalidTarget(target.to_string()));
        }

        if let Ok(ip) = IpAddr::from_str(target) {
            return Ok(Self {
                host: target.to_string(),
                ip: Some(ip),
                resolved_addresses: vec![ip],
            });
        }

        if let Ok(url) = Url::parse(target) {
            let host = url
                .host_str()
                .ok_or_else(|| ScopeError::InvalidTarget(target.to_string()))?
                .to_string();

            let result = resolver.resolve_all(&host);
            let addresses = result.addresses;
            let ip = addresses.first().copied();

            return Ok(Self {
                host,
                ip,
                resolved_addresses: addresses,
            });
        }

        if target.contains('/') || target.contains(' ') {
            return Err(ScopeError::InvalidTarget(target.to_string()));
        }

        let host = target.split(':').next().unwrap_or(target).to_string();

        if host.is_empty() {
            return Err(ScopeError::InvalidTarget(target.to_string()));
        }

        let result = resolver.resolve_all(&host);
        let addresses = result.addresses;
        let ip = addresses.first().copied();

        Ok(Self {
            host,
            ip,
            resolved_addresses: addresses,
        })
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

    // ========== Phase B: AddressClass tests ==========

    #[test]
    fn test_classify_address_public() {
        assert_eq!(
            classify_address(&"8.8.8.8".parse().unwrap()),
            AddressClass::Public
        );
        assert_eq!(
            classify_address(&"2001:4860:4860::8888".parse().unwrap()),
            AddressClass::Public
        );
    }

    #[test]
    fn test_classify_address_loopback() {
        assert_eq!(
            classify_address(&"127.0.0.1".parse().unwrap()),
            AddressClass::Loopback
        );
        assert_eq!(
            classify_address(&"::1".parse().unwrap()),
            AddressClass::Loopback
        );
        assert_eq!(
            classify_address(&"127.255.255.255".parse().unwrap()),
            AddressClass::Loopback
        );
    }

    #[test]
    fn test_classify_address_private() {
        assert_eq!(
            classify_address(&"10.0.0.1".parse().unwrap()),
            AddressClass::Private
        );
        assert_eq!(
            classify_address(&"172.16.0.1".parse().unwrap()),
            AddressClass::Private
        );
        assert_eq!(
            classify_address(&"192.168.1.1".parse().unwrap()),
            AddressClass::Private
        );
        assert_eq!(
            classify_address(&"fc00::1".parse().unwrap()),
            AddressClass::Private
        );
        assert_eq!(
            classify_address(&"fd00::1".parse().unwrap()),
            AddressClass::Private
        );
    }

    #[test]
    fn test_classify_address_link_local() {
        assert_eq!(
            classify_address(&"169.254.1.1".parse().unwrap()),
            AddressClass::LinkLocal
        );
        assert_eq!(
            classify_address(&"fe80::1".parse().unwrap()),
            AddressClass::LinkLocal
        );
    }

    #[test]
    fn test_classify_address_unspecified() {
        assert_eq!(
            classify_address(&"0.0.0.0".parse().unwrap()),
            AddressClass::Unspecified
        );
        assert_eq!(
            classify_address(&"::".parse().unwrap()),
            AddressClass::Unspecified
        );
    }

    #[test]
    fn test_classify_address_multicast() {
        assert_eq!(
            classify_address(&"224.0.0.1".parse().unwrap()),
            AddressClass::Multicast
        );
        assert_eq!(
            classify_address(&"ff02::1".parse().unwrap()),
            AddressClass::Multicast
        );
    }

    #[test]
    fn test_address_class_as_str() {
        assert_eq!(AddressClass::Public.as_str(), "public");
        assert_eq!(AddressClass::Loopback.as_str(), "loopback");
        assert_eq!(AddressClass::Private.as_str(), "private");
        assert_eq!(AddressClass::LinkLocal.as_str(), "link-local");
        assert_eq!(AddressClass::Unspecified.as_str(), "unspecified");
        assert_eq!(AddressClass::Multicast.as_str(), "multicast");
    }

    // ========== Phase B: Fake resolver tests ==========

    struct FakeResolver {
        responses: std::collections::HashMap<String, Vec<IpAddr>>,
    }

    impl FakeResolver {
        fn new() -> Self {
            Self {
                responses: std::collections::HashMap::new(),
            }
        }

        fn with_response(mut self, host: &str, addrs: Vec<IpAddr>) -> Self {
            self.responses.insert(host.to_string(), addrs);
            self
        }
    }

    impl HostResolver for FakeResolver {
        fn resolve_all(&self, host: &str) -> ResolutionResult {
            let addresses = self.responses.get(host).cloned().unwrap_or_default();
            ResolutionResult {
                hostname: host.to_string(),
                addresses,
                error: None,
            }
        }
    }

    #[test]
    fn test_target_scope_parse_literal_ip() {
        let resolver = FakeResolver::new();
        let ts = TargetScope::parse_with_resolver("10.0.0.1", &resolver).unwrap();
        assert_eq!(ts.host, "10.0.0.1");
        assert_eq!(ts.ip, Some("10.0.0.1".parse::<IpAddr>().unwrap()));
        assert_eq!(
            ts.resolved_addresses,
            vec!["10.0.0.1".parse::<IpAddr>().unwrap()]
        );
    }

    #[test]
    fn test_target_scope_parse_hostname_resolves_all() {
        let resolver = FakeResolver::new().with_response(
            "example.com",
            vec![
                "93.184.216.34".parse::<IpAddr>().unwrap(),
                "93.184.216.35".parse::<IpAddr>().unwrap(),
            ],
        );
        let ts = TargetScope::parse_with_resolver("example.com", &resolver).unwrap();
        assert_eq!(ts.host, "example.com");
        assert_eq!(
            ts.resolved_addresses,
            vec![
                "93.184.216.34".parse::<IpAddr>().unwrap(),
                "93.184.216.35".parse::<IpAddr>().unwrap()
            ]
        );
    }

    #[test]
    fn test_target_scope_parse_url_resolves_all() {
        let resolver = FakeResolver::new().with_response(
            "example.com",
            vec![
                "93.184.216.34".parse().unwrap(),
                "93.184.216.35".parse().unwrap(),
            ],
        );
        let ts = TargetScope::parse_with_resolver("https://example.com/path", &resolver).unwrap();
        assert_eq!(ts.host, "example.com");
        assert_eq!(ts.resolved_addresses.len(), 2);
    }

    #[test]
    fn test_target_scope_parse_no_resolution_returns_empty() {
        let resolver = FakeResolver::new();
        let ts = TargetScope::parse_with_resolver("unknown.host", &resolver).unwrap();
        assert_eq!(ts.host, "unknown.host");
        assert!(ts.ip.is_none());
        assert!(ts.resolved_addresses.is_empty());
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
    fn test_evaluate_addresses_loopback_in_allowed_cidr() {
        let scope = Scope {
            allowed_targets: vec![ScopeRule::new("127.0.0.0/8".to_string())],
            ..Default::default()
        };
        let ts = TargetScope {
            host: "localhost".to_string(),
            ip: Some("127.0.0.1".parse::<IpAddr>().unwrap()),
            resolved_addresses: vec!["127.0.0.1".parse::<IpAddr>().unwrap()],
        };
        let (all_allowed, any_excluded, classes) =
            ts.evaluate_addresses(&scope.allowed_targets, &scope.excluded_targets);
        assert!(all_allowed);
        assert!(!any_excluded);
        assert_eq!(classes, vec![AddressClass::Loopback]);
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

    // ========== Phase B: Integration tests ==========

    #[test]
    fn test_scope_with_cidr_allows_loopback_when_explicit() {
        let mut scope = Scope::new();
        scope
            .allowed_targets
            .push(ScopeRule::new("127.0.0.0/8".to_string()));
        assert!(scope.is_target_allowed("127.0.0.1").unwrap());
    }

    #[test]
    fn test_scope_with_cidr_blocks_loopback_when_not_explicit() {
        let mut scope = Scope::new();
        scope
            .allowed_targets
            .push(ScopeRule::new("10.0.0.0/8".to_string()));
        // 127.0.0.1 is not in 10.0.0.0/8
        assert!(!scope.is_target_allowed("127.0.0.1").unwrap());
    }

    #[test]
    fn test_scope_hostname_resolves_to_private_blocked_by_fallback() {
        let mut scope = Scope::new();
        scope
            .allowed_targets
            .push(ScopeRule::new("public.example.com".to_string()));
        // When scope rules exist but don't match, private IPs get specific warning
        // This tests the code path where is_target_allowed returns false for non-matching
        // private targets with explicit rules
        assert!(!scope.is_target_allowed("10.0.0.1").unwrap());
    }

    #[test]
    fn test_is_private_ip_includes_all_ranges() {
        // IPv4
        assert!(is_private_ip(&"10.0.0.1".parse().unwrap()));
        assert!(is_private_ip(&"172.16.0.1".parse().unwrap()));
        assert!(is_private_ip(&"192.168.1.1".parse().unwrap()));
        assert!(is_private_ip(&"169.254.1.1".parse().unwrap()));
        assert!(is_private_ip(&"127.0.0.1".parse().unwrap()));

        // IPv6
        assert!(is_private_ip(&"::1".parse().unwrap()));
        assert!(is_private_ip(&"fc00::1".parse().unwrap()));
        assert!(is_private_ip(&"fd00::1".parse().unwrap()));
        assert!(is_private_ip(&"fe80::1".parse().unwrap()));

        // Public
        assert!(!is_private_ip(&"8.8.8.8".parse().unwrap()));
        assert!(!is_private_ip(&"2001:4860:4860::8888".parse().unwrap()));
    }

    #[test]
    fn test_resolution_result_methods() {
        let empty = ResolutionResult {
            hostname: "host".to_string(),
            addresses: Vec::new(),
            error: None,
        };
        assert!(!empty.has_addresses());
        assert!(empty.first_address().is_none());

        let with_addrs = ResolutionResult {
            hostname: "host".to_string(),
            addresses: vec!["1.2.3.4".parse().unwrap()],
            error: None,
        };
        assert!(with_addrs.has_addresses());
        assert_eq!(with_addrs.first_address(), Some("1.2.3.4".parse().unwrap()));
    }

    // ========== Phase B.1: AddressClass-based authorization tests ==========

    #[test]
    fn test_address_class_is_non_public() {
        assert!(!AddressClass::Public.is_non_public());
        assert!(AddressClass::Loopback.is_non_public());
        assert!(AddressClass::Private.is_non_public());
        assert!(AddressClass::LinkLocal.is_non_public());
        assert!(AddressClass::IPv4MappedLoopback.is_non_public());
        assert!(AddressClass::Unspecified.is_non_public());
        assert!(AddressClass::Multicast.is_non_public());
    }

    #[test]
    fn test_scope_no_rules_allows_public_targets() {
        let scope = Scope::new();
        // Public targets are allowed when no rules are defined
        assert!(scope.is_target_allowed("8.8.8.8").unwrap());
        assert!(scope.is_target_allowed("example.com").unwrap());
    }

    #[test]
    fn test_scope_no_rules_blocks_private_targets() {
        let scope = Scope::new();
        // Private targets are blocked when no rules are defined
        assert!(!scope.is_target_allowed("10.0.0.1").unwrap());
        assert!(!scope.is_target_allowed("192.168.1.1").unwrap());
        assert!(!scope.is_target_allowed("172.16.0.1").unwrap());
    }

    #[test]
    fn test_scope_no_rules_allows_loopback_targets() {
        let scope = Scope::new();
        // Loopback is exempt from private IP blocking
        assert!(scope.is_target_allowed("127.0.0.1").unwrap());
        assert!(scope.is_target_allowed("127.0.0.2").unwrap());
    }

    #[test]
    fn test_scope_no_rules_blocks_link_local_targets() {
        let scope = Scope::new();
        // Link-local addresses are blocked
        assert!(!scope.is_target_allowed("169.254.1.1").unwrap());
    }

    #[test]
    fn test_scope_with_cidr_allows_matching_private_targets() {
        let mut scope = Scope::new();
        scope
            .allowed_targets
            .push(ScopeRule::new("10.0.0.0/8".to_string()));
        // Private IP in allowed CIDR is permitted
        assert!(scope.is_target_allowed("10.0.0.1").unwrap());
        assert!(scope.is_target_allowed("10.255.255.255").unwrap());
    }

    #[test]
    fn test_scope_with_cidr_blocks_non_matching_private_targets() {
        let mut scope = Scope::new();
        scope
            .allowed_targets
            .push(ScopeRule::new("10.0.0.0/8".to_string()));
        // Private IP not in allowed CIDR is blocked
        assert!(!scope.is_target_allowed("192.168.1.1").unwrap());
        assert!(!scope.is_target_allowed("172.16.0.1").unwrap());
    }

    #[test]
    fn test_evaluate_addresses_uses_address_class() {
        let scope = Scope {
            allowed_targets: vec![ScopeRule::new("*".to_string())],
            ..Default::default()
        };
        // Mixed public and private addresses
        let ts = TargetScope {
            host: "mixed.example.com".to_string(),
            ip: Some("8.8.8.8".parse::<IpAddr>().unwrap()),
            resolved_addresses: vec![
                "8.8.8.8".parse::<IpAddr>().unwrap(),
                "10.0.0.1".parse::<IpAddr>().unwrap(),
            ],
        };
        let (all_allowed, _any_excluded, classes) =
            ts.evaluate_addresses(&scope.allowed_targets, &scope.excluded_targets);
        // Wildcard allows all addresses
        assert!(all_allowed);
        // Classes correctly identify address types
        assert!(classes.contains(&AddressClass::Public));
        assert!(classes.contains(&AddressClass::Private));
    }

    #[test]
    fn test_classify_address_ipv4_mapped_loopback() {
        // ::ffff:127.0.0.1 should be classified as IPv4MappedLoopback
        let mapped_loopback: IpAddr = "::ffff:127.0.0.1".parse().unwrap();
        assert_eq!(
            classify_address(&mapped_loopback),
            AddressClass::IPv4MappedLoopback
        );

        // ::ffff:10.0.0.1 should be classified as Private (via embedded v4)
        let mapped_private: IpAddr = "::ffff:10.0.0.1".parse().unwrap();
        assert_eq!(classify_address(&mapped_private), AddressClass::Private);
    }
}
