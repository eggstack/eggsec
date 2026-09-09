//! Target normalization and target-policy types (Phase D WS6).
//!
//! Cohesive module extracted from `config/policy.rs`: target representation,
//! normalization, and target-policy validation only. No execution policy,
//! operation catalog, or evaluation logic here.
//!
//! Stable facade: `config/policy.rs` re-exports everything here, so
//! `crate::config::policy::{TargetHint, OperationTarget, normalize_target,
//! TargetPolicyKind, DescriptorError}` and the corresponding
//! `crate::config::{...}` paths keep working.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Operation Metadata — single source of truth for OperationDescriptor generation
// ---------------------------------------------------------------------------

/// Hint about the semantic kind of a target string, used by
/// [`OperationMetadata::try_descriptor_for_target`] to improve normalization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetHint {
    /// Treat as a URL (scheme://host/path).
    Url,
    /// Treat as a bare hostname or domain.
    Host,
    /// Treat as an IP address (v4 or v6).
    Ip,
    /// Treat as a CIDR network.
    Cidr,
    /// Treat as a local file or resource path.
    Resource,
}

/// Normalized representation of an operation target.
///
/// Created by [`OperationMetadata::try_descriptor_for_target`] to provide
/// a deterministic, comparable form for authorization binding. Two targets
/// that are semantically identical will produce the same `OperationTarget`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OperationTarget {
    /// No target — operation does not operate against an external entity.
    #[default]
    None,
    /// Normalized URL target (lowercase host, default-port stripped, path normalized).
    Url(String),
    /// Normalized hostname (lowercase).
    Host(String),
    /// Parsed IP address.
    Ip(std::net::IpAddr),
    /// Parsed CIDR network.
    Cidr(ipnetwork::IpNetwork),
    /// Local file or resource path (normalized to canonical form).
    Resource(String),
}

impl std::fmt::Display for OperationTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "(none)"),
            Self::Url(s) | Self::Host(s) | Self::Resource(s) => write!(f, "{}", s),
            Self::Ip(addr) => write!(f, "{}", addr),
            Self::Cidr(net) => write!(f, "{}", net),
        }
    }
}

impl OperationTarget {
    /// Returns `true` if this is the `None` variant (no target).
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    /// Returns `true` if this target carries a meaningful identity (not `None`).
    pub fn is_present(&self) -> bool {
        !self.is_none()
    }
}

/// Normalize a raw target string into an [`OperationTarget`], optionally guided
/// by a [`TargetHint`].
///
/// When no hint is provided, the function attempts auto-detection: IP addresses
/// and CIDR networks are tried first, then URLs (if a scheme is present), then
/// bare hostnames.
pub fn normalize_target(raw: &str, hint: Option<TargetHint>) -> OperationTarget {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return OperationTarget::None;
    }

    match hint {
        Some(TargetHint::Url) => normalize_url(trimmed),
        Some(TargetHint::Host) => OperationTarget::Host(trimmed.to_lowercase()),
        Some(TargetHint::Ip) => {
            if let Ok(addr) = trimmed.parse::<std::net::IpAddr>() {
                OperationTarget::Ip(addr)
            } else {
                OperationTarget::Host(trimmed.to_lowercase())
            }
        }
        Some(TargetHint::Cidr) => {
            if let Ok(net) = trimmed.parse::<ipnetwork::IpNetwork>() {
                OperationTarget::Cidr(net)
            } else if let Ok(addr) = trimmed.parse::<std::net::IpAddr>() {
                OperationTarget::Ip(addr)
            } else {
                OperationTarget::Host(trimmed.to_lowercase())
            }
        }
        Some(TargetHint::Resource) => OperationTarget::Resource(trimmed.to_string()),
        None => auto_detect_target(trimmed),
    }
}

/// Auto-detect target kind and normalize.
fn auto_detect_target(raw: &str) -> OperationTarget {
    // Try bare IP first (before CIDR, since "10.0.0.2" parses as /32 CIDR).
    if let Ok(addr) = raw.parse::<std::net::IpAddr>() {
        return OperationTarget::Ip(addr);
    }
    // Try CIDR (must have explicit /prefix to avoid matching bare IPs).
    if let Ok(net) = raw.parse::<ipnetwork::IpNetwork>() {
        return OperationTarget::Cidr(net);
    }
    // If it looks like a URL (has a scheme), parse as URL.
    if raw.contains("://") {
        return normalize_url(raw);
    }
    // Otherwise treat as a hostname.
    OperationTarget::Host(raw.to_lowercase())
}

/// Normalize a URL string: lowercase host, strip default port, normalize path.
fn normalize_url(raw: &str) -> OperationTarget {
    match url::Url::parse(raw) {
        Ok(mut parsed) => {
            // Lowercase the host.
            if let Some(host) = parsed.host_str() {
                let lower_host = host.to_lowercase();
                if let Err(error) = parsed.set_host(Some(&lower_host)) {
                    tracing::debug!(%error, "Failed to normalize URL host");
                }
            }
            // Strip default port.
            let needs_port_strip = matches!(
                (parsed.scheme(), parsed.port()),
                ("http", Some(80)) | ("https", Some(443))
            );
            if needs_port_strip && parsed.set_port(None).is_err() {
                tracing::debug!("Failed to remove default URL port");
            }
            // Normalize path: remove trailing slash unless root.
            let path = parsed.path().to_string();
            if path.len() > 1 && path.ends_with('/') {
                let normalized = path.trim_end_matches('/');
                parsed.set_path(normalized);
            }
            OperationTarget::Url(parsed.to_string())
        }
        Err(_) => {
            // If URL parsing fails, treat as a hostname.
            OperationTarget::Host(raw.to_lowercase())
        }
    }
}

/// Target policy requirement for operation metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetPolicyKind {
    NoTarget,
    OptionalTarget,
    TargetRequired,
    ExplicitScopeRequired,
    PrivateOrLocalRequired,
}

impl std::fmt::Display for TargetPolicyKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoTarget => write!(f, "no-target"),
            Self::OptionalTarget => write!(f, "optional-target"),
            Self::TargetRequired => write!(f, "target-required"),
            Self::ExplicitScopeRequired => write!(f, "explicit-scope-required"),
            Self::PrivateOrLocalRequired => write!(f, "private-or-local-required"),
        }
    }
}

/// Error returned by [`OperationMetadata::try_descriptor_for_target`] when the
/// supplied target violates the operation's target policy.
#[derive(Debug, Clone, thiserror::Error)]
pub enum DescriptorError {
    /// A target was supplied for an operation with `NoTarget` policy.
    #[error(
        "operation '{operation_id}' has target policy '{target_policy}' but received target '{target}'"
    )]
    UnexpectedTarget {
        operation_id: String,
        target_policy: TargetPolicyKind,
        target: String,
    },

    /// A required target is missing (None or empty) for a target-bearing operation.
    #[error(
        "operation '{operation_id}' has target policy '{target_policy}' but no target was provided"
    )]
    MissingTarget {
        operation_id: String,
        target_policy: TargetPolicyKind,
    },

    /// The operation ID is not registered in the operation metadata registry.
    ///
    /// Returned by strict-surface constructors (e.g. agent scan descriptors)
    /// that must fail closed instead of synthesizing policy for unknown
    /// operations.
    #[error("unknown operation '{operation_id}': no registered operation metadata")]
    UnknownOperation { operation_id: String },
}
