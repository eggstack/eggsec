//! Shared types used across the crate.

use serde::{Deserialize, Serialize};
use subtle::ConstantTimeEq;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Canonical severity rating for findings and vulnerabilities.
///
/// Used by the fuzzer, WAF detector, recon, output, and tool modules.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Default,
)]
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
    #[deprecated(since = "0.0.0", note = "Use `str.parse::<Severity>()` instead")]
    pub fn from_str(s: &str) -> Self {
        s.parse().unwrap_or(Severity::Info)
    }

    /// Derive severity from a CVSS score.
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
            _ => Ok(Severity::Info),
        }
    }
}

/// A string wrapper that zeroizes its contents on drop.
///
/// Use for passwords, API keys, tokens, and other sensitive credentials.
/// Serializes/deserializes transparently as a plain string.
/// Comparison is constant-time to prevent timing attacks.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct SensitiveString(String);

impl SensitiveString {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Borrow the inner secret.
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
    /// When `redact_logs` is enabled, logs "[REDACTED]" instead of the actual value.
    pub fn log_secret(&self, logger: impl FnOnce(&str), redact: bool) {
        if redact {
            logger("[REDACTED]");
        } else {
            logger(self.expose_secret());
        }
    }

    /// Create a display-safe version for logging.
    ///
    /// Returns "[REDACTED]" when `redact` is true, otherwise returns the actual value.
    /// WARNING: Only use `redact=true` when you control the logging output destination.
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
        // Declaration order: Critical(0) < High(1) < Medium(2) < Low(3) < Info(4)
        // Semantic order (as_int): Critical(4) > High(3) > Medium(2) > Low(1) > Info(0)
        // Use as_int() for severity comparisons, not the derived Ord.
        assert_eq!(Severity::Critical.as_int(), 4);
        assert_eq!(Severity::Info.as_int(), 0);
        assert!(Severity::Critical.as_int() > Severity::High.as_int());
    }

    #[test]
    fn severity_from_str() {
        assert_eq!(Severity::from_str("critical"), Severity::Critical);
        assert_eq!(Severity::from_str("HIGH"), Severity::High);
        assert_eq!(Severity::from_str("moderate"), Severity::Medium);
        assert_eq!(Severity::from_str("unknown"), Severity::Info);
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
}
