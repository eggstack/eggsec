//! Shared types used across the crate.
//!
//! Only dependency-light types live here. `OutputFormat` and other
//! CLI-specific types remain in the main `eggsec` crate.

use serde::{Deserialize, Serialize};
use subtle::ConstantTimeEq;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Canonical severity rating for findings and vulnerabilities.
///
/// Used by the fuzzer, WAF detector, recon, output, and tool modules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    #[default]
    Info,
}

impl Severity {
    /// Parse a severity from a string, defaulting to `Info` for unknown values.
    ///
    /// Prefer `s.parse::<Severity>()` for new code.
    #[must_use]
    pub fn parse_or_default(s: &str) -> Self {
        s.parse().unwrap_or(Severity::Info)
    }

    /// Derive severity from a CVSS score.
    ///
    /// Score ranges: `>=9.0` → Critical, `>=7.0` → High, `>=4.0` → Medium,
    /// `>=0.1` → Low, otherwise → Info. NaN and negative values map to `Info`.
    #[must_use]
    pub fn from_cvss(score: f32) -> Self {
        match score {
            s if s >= 9.0 => Severity::Critical,
            s if s >= 7.0 => Severity::High,
            s if s >= 4.0 => Severity::Medium,
            s if s >= 0.1 => Severity::Low,
            _ => Severity::Info,
        }
    }

    /// Return the lowercase string representation.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Critical => "critical",
            Severity::High => "high",
            Severity::Medium => "medium",
            Severity::Low => "low",
            Severity::Info => "info",
        }
    }

    /// Return an integer ranking (higher = more severe).
    #[must_use]
    pub fn as_int(&self) -> i32 {
        match self {
            Severity::Critical => 4,
            Severity::High => 3,
            Severity::Medium => 2,
            Severity::Low => 1,
            Severity::Info => 0,
        }
    }

    /// Return a color emoji for terminal display.
    #[must_use]
    pub fn cvss_color(&self) -> &'static str {
        match self {
            Severity::Critical => "\u{1f534}",
            Severity::High => "\u{1f7e0}",
            Severity::Medium => "\u{1f7e1}",
            Severity::Low => "\u{1f535}",
            Severity::Info => "\u{26aa}",
        }
    }
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Critical => write!(f, "CRITICAL"),
            Severity::High => write!(f, "HIGH"),
            Severity::Medium => write!(f, "MEDIUM"),
            Severity::Low => write!(f, "LOW"),
            Severity::Info => write!(f, "INFO"),
        }
    }
}

impl std::str::FromStr for Severity {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "critical" => Ok(Severity::Critical),
            "high" => Ok(Severity::High),
            "medium" | "moderate" => Ok(Severity::Medium),
            "low" => Ok(Severity::Low),
            "info" => Ok(Severity::Info),
            _ => Ok(Severity::Info),
        }
    }
}

impl PartialOrd for Severity {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Severity {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_int().cmp(&other.as_int())
    }
}

/// A string wrapper that zeroizes its contents on drop.
///
/// Use for passwords, API keys, tokens, and other sensitive credentials.
/// Serializes/deserializes transparently as a plain string.
/// Comparison is constant-time to prevent timing attacks.
///
/// # Intentionally Not Implemented
///
/// `Hash` is intentionally **not** implemented. Hashing a secret would expose
/// a stable fingerprint that could be used for correlation attacks. If you need
/// to use a credential as a map key, derive a non-secret identifier instead.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct SensitiveString(String);

impl SensitiveString {
    #[must_use]
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Borrow the inner secret.
    #[must_use]
    pub fn expose_secret(&self) -> &str {
        &self.0
    }

    /// Consume and return the inner secret.
    ///
    /// The inner value is replaced with an empty string before returning
    /// so that `ZeroizeOnDrop` does not panic on the moved-out field.
    pub fn into_secret(mut self) -> String {
        std::mem::take(&mut self.0)
    }

    /// Log the secret value safely using tracing.
    ///
    /// When `redact_logs` is enabled, logs `"[REDACTED]"` instead of the actual value.
    pub fn log_secret(&self, logger: impl FnOnce(&str), redact: bool) {
        if redact {
            logger("[REDACTED]");
        } else {
            logger(self.expose_secret());
        }
    }

    /// Create a display-safe version for logging.
    ///
    /// Returns `"[REDACTED]"` when `redact` is true, otherwise returns the actual value.
    /// WARNING: Only use `redact=true` when you control the logging output destination.
    #[must_use]
    pub fn for_logging(&self, redact: bool) -> impl std::fmt::Display + '_ {
        struct SecretViewer<'a> {
            value: &'a str,
            redact: bool,
        }
        impl std::fmt::Display for SecretViewer<'_> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                if self.redact {
                    write!(f, "[REDACTED]")
                } else {
                    write!(f, "{}", self.value)
                }
            }
        }
        SecretViewer {
            value: self.expose_secret(),
            redact,
        }
    }
}

impl std::fmt::Debug for SensitiveString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SensitiveString([REDACTED])")
    }
}

impl std::fmt::Display for SensitiveString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("[REDACTED]")
    }
}

impl serde::Serialize for SensitiveString {
    /// WARNING: SensitiveString serializes secrets in plaintext!
    ///
    /// This is intentional for config file compatibility. The secret value is
    /// serialized directly without encryption or redaction.
    ///
    /// # Security Warning
    ///
    /// Ensure that config files containing `SensitiveString` values have
    /// appropriate access controls (e.g., filesystem permissions, encrypted
    /// storage) to protect the secrets from unauthorized access.
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> serde::Deserialize<'de> for SensitiveString {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(SensitiveString)
    }
}

impl PartialEq for SensitiveString {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_bytes().ct_eq(other.0.as_bytes()).into()
    }
}

impl Eq for SensitiveString {}

impl PartialEq<str> for SensitiveString {
    fn eq(&self, other: &str) -> bool {
        self.0.as_bytes().ct_eq(other.as_bytes()).into()
    }
}

impl<'a> PartialEq<&'a str> for SensitiveString {
    fn eq(&self, other: &&'a str) -> bool {
        self.0.as_bytes().ct_eq(other.as_bytes()).into()
    }
}

impl PartialEq<String> for SensitiveString {
    fn eq(&self, other: &String) -> bool {
        self.0.as_bytes().ct_eq(other.as_bytes()).into()
    }
}

impl From<String> for SensitiveString {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for SensitiveString {
    fn from(s: &str) -> Self {
        Self(s.to_string())
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
}
