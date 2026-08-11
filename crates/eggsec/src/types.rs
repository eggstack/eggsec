//! Shared types used across the crate.
//!
//! `Severity` and `SensitiveString` are defined in `eggsec-core` and
//! re-exported here. `OutputFormat` and `check_config_file_permissions`
//! remain in this crate because they depend on `clap` or have other
//! main-crate concerns.

use std::fs;
use std::path::Path;

#[cfg(feature = "cli")]
use clap::ValueEnum;
use serde::{Deserialize, Serialize};

// Re-export core types
pub use eggsec_core::types::{SensitiveString, Severity};

/// Check if a config file has overly permissive permissions.
///
/// Logs a warning if the file is writable by group or other, or readable by
/// group or other (mode bits outside of `0o600`). Config files containing
/// `SensitiveString` values should have restrictive permissions to protect
/// secrets from unauthorized access.
///
/// Checks write permissions first (more dangerous), then read permissions.
///
/// # Arguments
///
/// * `path` - Path to the config file to check
///
/// # Example
///
/// ```ignore
/// use eggsec::types::check_config_file_permissions;
///
/// if let Some(path) = config_path {
///     check_config_file_permissions(&path);
/// }
/// ```
#[cfg(unix)]
pub fn check_config_file_permissions(path: &Path) {
    use std::os::unix::fs::PermissionsExt;

    let metadata = match fs::metadata(path) {
        Ok(m) => m,
        Err(e) => {
            tracing::debug!("Could not read config file permissions: {}", e);
            return;
        }
    };

    let mode = metadata.permissions().mode();
    let world_readable = mode & 0o004;
    let group_readable = mode & 0o040;
    let world_writable = mode & 0o002;
    let group_writable = mode & 0o020;

    if world_writable != 0 {
        tracing::warn!(
            "Config file '{}' has world-writable permissions ({:o}). \
             Secrets may be modified or deleted by other users. Consider running: \
             chmod 600 '{}'",
            path.display(),
            mode,
            path.display()
        );
    } else if group_writable != 0 {
        tracing::warn!(
            "Config file '{}' has group-writable permissions ({:o}). \
             Secrets may be modified by other users on multi-user systems. \
             Consider running: chmod 600 '{}'",
            path.display(),
            mode,
            path.display()
        );
    } else if world_readable != 0 {
        tracing::warn!(
            "Config file '{}' has world-readable permissions ({:o}). \
             Secrets may be accessible to other users. Consider running: \
             chmod 600 '{}'",
            path.display(),
            mode,
            path.display()
        );
    } else if group_readable != 0 {
        tracing::warn!(
            "Config file '{}' has group-readable permissions ({:o}). \
             Secrets may be accessible to other users on multi-user systems. \
             Consider running: chmod 600 '{}'",
            path.display(),
            mode,
            path.display()
        );
    }
}

#[cfg(not(unix))]
pub fn check_config_file_permissions(_path: &Path) {}

/// Canonical output format for reports and CLI output.
///
/// Used by both CLI argument parsing and configuration deserialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "cli", derive(ValueEnum))]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    #[default]
    Pretty,
    Json,
    Compact,
    Html,
    Csv,
    Sarif,
    Junit,
    Markdown,
}

/// Scan profile controlling pipeline stage selection and risk budget.
///
/// Used by both CLI argument parsing and pipeline/engine configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "cli", derive(ValueEnum))]
pub enum ScanProfile {
    Quick,
    Endpoint,
    Web,
    Waf,
    Full,
    Api,
    Recon,
    Stealth,
    Deep,
    Vuln,
    Auth,
    DefenseLab,
    SynvoidLocal,
    WafRegression,
    ProtocolEdge,
    NseSafe,
    DbRegression,
    WebProxy,
}

/// Common HTTP client configuration shared across scan operations.
///
/// Used by both CLI argument parsing and engine HTTP client construction.
#[derive(Debug, Clone, Default)]
pub struct CommonHttpArgs {
    pub insecure: bool,
    pub proxy: Option<String>,
    pub proxy_auth: Option<String>,
    pub auth: Option<String>,
    pub bearer: Option<String>,
    pub cookie: Option<String>,
    pub api_key: Option<String>,
    pub user_agent: Option<String>,
    pub stealth: bool,
    pub rate_limit: Option<u32>,
    pub jitter: Option<String>,
    pub auth_context: Option<String>,
    pub auth_role: Option<String>,
}

impl OutputFormat {
    /// Parse an output format from a string, defaulting to `Pretty` for unknown values.
    ///
    /// Prefer `s.parse::<OutputFormat>()` for new code that wants error handling.
    #[must_use]
    pub fn parse_or_default(s: &str) -> Self {
        s.parse().unwrap_or(OutputFormat::Pretty)
    }
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Pretty => write!(f, "pretty"),
            OutputFormat::Json => write!(f, "json"),
            OutputFormat::Compact => write!(f, "compact"),
            OutputFormat::Html => write!(f, "html"),
            OutputFormat::Csv => write!(f, "csv"),
            OutputFormat::Sarif => write!(f, "sarif"),
            OutputFormat::Junit => write!(f, "junit"),
            OutputFormat::Markdown => write!(f, "markdown"),
        }
    }
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pretty" => Ok(OutputFormat::Pretty),
            "json" => Ok(OutputFormat::Json),
            "compact" => Ok(OutputFormat::Compact),
            "html" => Ok(OutputFormat::Html),
            "csv" => Ok(OutputFormat::Csv),
            "sarif" => Ok(OutputFormat::Sarif),
            "junit" => Ok(OutputFormat::Junit),
            "markdown" => Ok(OutputFormat::Markdown),
            _ => Err(format!("unknown output format: {}", s)),
        }
    }
}

impl std::fmt::Display for ScanProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScanProfile::Quick => write!(f, "quick"),
            ScanProfile::Endpoint => write!(f, "endpoint"),
            ScanProfile::Web => write!(f, "web"),
            ScanProfile::Waf => write!(f, "waf"),
            ScanProfile::Full => write!(f, "full"),
            ScanProfile::Api => write!(f, "api"),
            ScanProfile::Recon => write!(f, "recon"),
            ScanProfile::Stealth => write!(f, "stealth"),
            ScanProfile::Deep => write!(f, "deep"),
            ScanProfile::Vuln => write!(f, "vuln"),
            ScanProfile::Auth => write!(f, "auth"),
            ScanProfile::DefenseLab => write!(f, "defense-lab"),
            ScanProfile::SynvoidLocal => write!(f, "synvoid-local"),
            ScanProfile::WafRegression => write!(f, "waf-regression"),
            ScanProfile::ProtocolEdge => write!(f, "protocol-edge"),
            ScanProfile::NseSafe => write!(f, "nse-safe"),
            ScanProfile::DbRegression => write!(f, "db-regression"),
            ScanProfile::WebProxy => write!(f, "web-proxy"),
        }
    }
}

impl std::str::FromStr for ScanProfile {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "quick" => Ok(ScanProfile::Quick),
            "endpoint" => Ok(ScanProfile::Endpoint),
            "web" => Ok(ScanProfile::Web),
            "waf" => Ok(ScanProfile::Waf),
            "full" => Ok(ScanProfile::Full),
            "api" => Ok(ScanProfile::Api),
            "recon" => Ok(ScanProfile::Recon),
            "stealth" => Ok(ScanProfile::Stealth),
            "deep" => Ok(ScanProfile::Deep),
            "vuln" => Ok(ScanProfile::Vuln),
            "auth" => Ok(ScanProfile::Auth),
            "defense-lab" => Ok(ScanProfile::DefenseLab),
            "synvoid-local" => Ok(ScanProfile::SynvoidLocal),
            "waf-regression" => Ok(ScanProfile::WafRegression),
            "protocol-edge" => Ok(ScanProfile::ProtocolEdge),
            "nse-safe" => Ok(ScanProfile::NseSafe),
            "db-regression" | "db_regression" | "dbregression" => Ok(ScanProfile::DbRegression),
            "web-proxy" | "webproxy" | "proxy" => Ok(ScanProfile::WebProxy),
            _ => Err(format!("Unknown scan profile: {}", s)),
        }
    }
}

impl ScanProfile {
    /// Returns `true` if this profile is a defense-lab variant that requires
    /// local/private-scope targets only.
    pub fn requires_private_scope(&self) -> bool {
        matches!(
            self,
            ScanProfile::DefenseLab
                | ScanProfile::SynvoidLocal
                | ScanProfile::DbRegression
                | ScanProfile::WafRegression
                | ScanProfile::ProtocolEdge
                | ScanProfile::NseSafe
                | ScanProfile::WebProxy
        )
    }

    /// Returns `true` if this profile requires the `packet-inspection` feature.
    pub fn requires_packet_inspection(&self) -> bool {
        matches!(self, ScanProfile::ProtocolEdge)
    }

    /// Returns `true` if this profile requires the `nse` feature.
    pub fn requires_nse(&self) -> bool {
        matches!(self, ScanProfile::NseSafe)
    }

    /// Returns the maximum `ProbeRisk` level allowed for this profile.
    pub fn max_risk_budget(&self) -> crate::probe::ProbeRisk {
        match self {
            ScanProfile::Quick | ScanProfile::ProtocolEdge | ScanProfile::NseSafe => {
                crate::probe::ProbeRisk::SafeActive
            }
            ScanProfile::Stealth => crate::probe::ProbeRisk::Passive,
            ScanProfile::DefenseLab
            | ScanProfile::SynvoidLocal
            | ScanProfile::WafRegression
            | ScanProfile::DbRegression
            | ScanProfile::WebProxy => crate::probe::ProbeRisk::Intrusive,
            ScanProfile::Endpoint
            | ScanProfile::Web
            | ScanProfile::Waf
            | ScanProfile::Recon
            | ScanProfile::Vuln
            | ScanProfile::Auth => crate::probe::ProbeRisk::Intrusive,
            ScanProfile::Full | ScanProfile::Api | ScanProfile::Deep => {
                crate::probe::ProbeRisk::Stress
            }
        }
    }

    /// Returns the operating mode for this profile.
    pub fn operation_mode(&self) -> crate::config::OperationMode {
        match self {
            ScanProfile::Quick
            | ScanProfile::Endpoint
            | ScanProfile::Web
            | ScanProfile::Waf
            | ScanProfile::Full
            | ScanProfile::Api
            | ScanProfile::Recon
            | ScanProfile::Stealth
            | ScanProfile::Deep
            | ScanProfile::Vuln
            | ScanProfile::Auth => crate::config::OperationMode::StandardAssessment,
            ScanProfile::DefenseLab
            | ScanProfile::SynvoidLocal
            | ScanProfile::WafRegression
            | ScanProfile::ProtocolEdge
            | ScanProfile::NseSafe
            | ScanProfile::DbRegression
            | ScanProfile::WebProxy => crate::config::OperationMode::DefenseLab,
        }
    }

    /// Returns the intended use cases for this profile.
    pub fn intended_uses(&self) -> Vec<crate::config::IntendedUse> {
        match self {
            ScanProfile::Quick | ScanProfile::Endpoint | ScanProfile::Web => {
                vec![crate::config::IntendedUse::WebAssessment]
            }
            ScanProfile::Api => vec![crate::config::IntendedUse::ApiAssessment],
            ScanProfile::Waf | ScanProfile::WafRegression => {
                vec![crate::config::IntendedUse::WafRegression]
            }
            ScanProfile::Full | ScanProfile::Deep => vec![
                crate::config::IntendedUse::WebAssessment,
                crate::config::IntendedUse::ApiAssessment,
            ],
            ScanProfile::Recon => vec![crate::config::IntendedUse::WebAssessment],
            ScanProfile::Stealth => vec![crate::config::IntendedUse::WebAssessment],
            ScanProfile::Vuln | ScanProfile::Auth => {
                vec![crate::config::IntendedUse::WebAssessment]
            }
            ScanProfile::DefenseLab => vec![
                crate::config::IntendedUse::WafRegression,
                crate::config::IntendedUse::SynvoidRegression,
            ],
            ScanProfile::SynvoidLocal => {
                vec![crate::config::IntendedUse::SynvoidRegression]
            }
            ScanProfile::ProtocolEdge => {
                vec![crate::config::IntendedUse::ProtocolEdgeValidation]
            }
            ScanProfile::NseSafe => vec![crate::config::IntendedUse::CodingAgentVerification],
            ScanProfile::DbRegression => vec![
                crate::config::IntendedUse::WafRegression,
                crate::config::IntendedUse::SynvoidRegression,
            ],
            ScanProfile::WebProxy => vec![
                crate::config::IntendedUse::WafRegression,
                crate::config::IntendedUse::SynvoidRegression,
            ],
        }
    }

    /// Returns a human-readable description of this profile's mode and risk.
    pub fn mode_description(&self) -> String {
        let mode = self.operation_mode();
        let risk = self.max_risk_budget();
        format!(
            "{} mode (max risk: {})",
            mode.label(),
            format!("{:?}", risk).to_lowercase()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_ordering() {
        assert_eq!(Severity::Critical.as_int(), 4);
        assert_eq!(Severity::Info.as_int(), 0);
        assert!(Severity::Critical > Severity::High);
    }

    #[test]
    fn severity_from_str() {
        assert_eq!("critical".parse::<Severity>().unwrap(), Severity::Critical);
        assert_eq!("HIGH".parse::<Severity>().unwrap(), Severity::High);
        assert_eq!("moderate".parse::<Severity>().unwrap(), Severity::Medium);
        assert_eq!("unknown".parse::<Severity>().unwrap(), Severity::Info);
    }

    #[test]
    fn severity_from_cvss() {
        assert_eq!(Severity::from_cvss(9.5), Severity::Critical);
        assert_eq!(Severity::from_cvss(7.0), Severity::High);
        assert_eq!(Severity::from_cvss(4.0), Severity::Medium);
        assert_eq!(Severity::from_cvss(0.5), Severity::Low);
        assert_eq!(Severity::from_cvss(0.0), Severity::Info);
    }

    #[test]
    fn severity_display_is_uppercase() {
        assert_eq!(format!("{}", Severity::Critical), "CRITICAL");
        assert_eq!(format!("{}", Severity::Info), "INFO");
    }

    #[test]
    fn severity_as_str_is_lowercase() {
        assert_eq!(Severity::Critical.as_str(), "critical");
        assert_eq!(Severity::Info.as_str(), "info");
    }

    #[test]
    fn sensitive_string_expose() {
        let s = SensitiveString::new("secret123");
        assert_eq!(s.expose_secret(), "secret123");
    }

    #[test]
    fn sensitive_string_into_secret() {
        let s = SensitiveString::new("secret123");
        assert_eq!(s.into_secret(), "secret123");
    }

    #[test]
    fn sensitive_string_debug_redacted() {
        let s = SensitiveString::new("hunter2");
        let debug = format!("{:?}", s);
        assert_eq!(debug, "SensitiveString([REDACTED])");
        assert!(!debug.contains("hunter2"));
    }

    #[test]
    fn sensitive_string_display_redacted() {
        let s = SensitiveString::new("hunter2");
        assert_eq!(format!("{}", s), "[REDACTED]");
    }

    #[test]
    fn sensitive_string_serialize_deserialize() {
        let s = SensitiveString::new("api-key-123");
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(json, "\"api-key-123\"");
        let deserialized: SensitiveString = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.expose_secret(), "api-key-123");
    }

    #[test]
    fn sensitive_string_equality() {
        let a = SensitiveString::new("same");
        let b = SensitiveString::new("same");
        let c = SensitiveString::new("different");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn sensitive_string_from_conversions() {
        let from_str: SensitiveString = "hello".into();
        let from_string: SensitiveString = String::from("hello").into();
        assert_eq!(from_str, from_string);
    }

    #[test]
    fn severity_ord_matches_semantic_order() {
        assert!(Severity::Critical > Severity::High);
        assert!(Severity::High > Severity::Medium);
        assert!(Severity::Medium > Severity::Low);
        assert!(Severity::Low > Severity::Info);
    }

    #[test]
    fn severity_sorts_correctly() {
        let mut sevs = vec![
            Severity::Info,
            Severity::Low,
            Severity::Critical,
            Severity::Medium,
            Severity::High,
        ];
        sevs.sort();
        assert_eq!(
            sevs,
            vec![
                Severity::Info,
                Severity::Low,
                Severity::Medium,
                Severity::High,
                Severity::Critical,
            ]
        );
    }

    #[test]
    fn severity_cvss_boundary_values() {
        assert_eq!(Severity::from_cvss(8.99), Severity::High);
        assert_eq!(Severity::from_cvss(9.0), Severity::Critical);
        assert_eq!(Severity::from_cvss(6.99), Severity::Medium);
        assert_eq!(Severity::from_cvss(7.0), Severity::High);
        assert_eq!(Severity::from_cvss(3.99), Severity::Low);
        assert_eq!(Severity::from_cvss(4.0), Severity::Medium);
        assert_eq!(Severity::from_cvss(0.09), Severity::Info);
        assert_eq!(Severity::from_cvss(0.1), Severity::Low);
    }

    #[test]
    fn sensitive_string_empty() {
        let s = SensitiveString::new("");
        assert!(s.is_empty());
        assert_eq!(s.len(), 0);
        assert_eq!(s.expose_secret(), "");
        assert_eq!(s.as_bytes(), b"");
    }

    #[test]
    fn sensitive_string_into_secret_leaves_owned() {
        let s = SensitiveString::new("token");
        let secret = s.into_secret();
        assert_eq!(secret, "token");
    }

    #[test]
    fn sensitive_string_for_logging_redacted() {
        let s = SensitiveString::new("supersecret");
        assert_eq!(format!("{}", s.for_logging(true)), "[REDACTED]");
        assert_eq!(format!("{}", s.for_logging(false)), "supersecret");
    }

    #[test]
    fn sensitive_string_len() {
        let s = SensitiveString::new("abcde");
        assert_eq!(s.len(), 5);
    }

    #[test]
    fn output_format_display() {
        assert_eq!(format!("{}", OutputFormat::Pretty), "pretty");
        assert_eq!(format!("{}", OutputFormat::Json), "json");
        assert_eq!(format!("{}", OutputFormat::Sarif), "sarif");
        assert_eq!(format!("{}", OutputFormat::Markdown), "markdown");
    }

    #[test]
    fn output_format_default_is_pretty() {
        assert_eq!(OutputFormat::default(), OutputFormat::Pretty);
    }

    #[test]
    fn output_format_from_str() {
        assert_eq!("json".parse::<OutputFormat>().unwrap(), OutputFormat::Json);
        assert_eq!("HTML".parse::<OutputFormat>().unwrap(), OutputFormat::Html);
        assert_eq!(
            "SARIF".parse::<OutputFormat>().unwrap(),
            OutputFormat::Sarif
        );
        assert_eq!(
            "markdown".parse::<OutputFormat>().unwrap(),
            OutputFormat::Markdown
        );
        assert!("unknown".parse::<OutputFormat>().is_err());
    }

    #[test]
    fn severity_default_is_info() {
        assert_eq!(Severity::default(), Severity::Info);
    }

    #[test]
    fn severity_display_from_str_roundtrip() {
        for sev in [
            Severity::Critical,
            Severity::High,
            Severity::Medium,
            Severity::Low,
            Severity::Info,
        ] {
            let s = format!("{}", sev);
            let parsed: Severity = s.parse().unwrap();
            assert_eq!(parsed, sev, "round-trip failed for {:?}", sev);
        }
    }

    #[test]
    fn output_format_serde_roundtrip() {
        for fmt in [
            OutputFormat::Pretty,
            OutputFormat::Json,
            OutputFormat::Compact,
            OutputFormat::Html,
            OutputFormat::Csv,
            OutputFormat::Sarif,
            OutputFormat::Junit,
            OutputFormat::Markdown,
        ] {
            let json = serde_json::to_string(&fmt).unwrap();
            let deserialized: OutputFormat = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, fmt, "serde round-trip failed for {:?}", fmt);
        }
    }

    #[test]
    fn sensitive_string_eq_str() {
        let s = SensitiveString::new("secret");
        assert_eq!(s, "secret");
        assert_ne!(s, "other");
    }

    #[test]
    fn sensitive_string_eq_string() {
        let s = SensitiveString::new("secret");
        assert_eq!(s, String::from("secret"));
        assert_ne!(s, String::from("other"));
    }

    #[test]
    fn output_format_parse_or_default() {
        assert_eq!(OutputFormat::parse_or_default("json"), OutputFormat::Json);
        assert_eq!(OutputFormat::parse_or_default("HTML"), OutputFormat::Html);
        assert_eq!(
            OutputFormat::parse_or_default("unknown"),
            OutputFormat::Pretty
        );
        assert_eq!(OutputFormat::parse_or_default(""), OutputFormat::Pretty);
    }

    #[test]
    fn severity_from_cvss_nan() {
        assert_eq!(Severity::from_cvss(f32::NAN), Severity::Info);
    }

    #[test]
    fn severity_from_cvss_negative() {
        assert_eq!(Severity::from_cvss(-1.0), Severity::Info);
    }
}
