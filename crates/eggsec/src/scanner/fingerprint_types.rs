use serde::{Deserialize, Serialize};

/// Confidence level for a service fingerprint match.
///
/// Variants are ordered from lowest to highest confidence so that
/// derived `Ord` produces the expected semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FingerprintConfidence {
    /// Unknown service or version
    Unknown,
    /// Low confidence, could be misidentified
    Low,
    /// Medium confidence, likely correct
    Medium,
    /// High confidence based on strong indicators
    High,
    /// Service and version confirmed by multiple independent signals
    Confirmed,
}

impl std::fmt::Display for FingerprintConfidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Confirmed => write!(f, "confirmed"),
            Self::High => write!(f, "high"),
            Self::Medium => write!(f, "medium"),
            Self::Low => write!(f, "low"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

/// Evidence captured during fingerprinting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FingerprintEvidence {
    pub kind: EvidenceType,
    pub raw_value: Option<String>,
    pub redacted_value: Option<String>,
    pub confidence_contribution: FingerprintConfidence,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceType {
    Banner,
    TlsCertificate,
    TlsAlpn,
    HttpHeader,
    HttpResponse,
    ProtocolNegotiation,
    DnsRecord,
    PortState,
}

/// Normalized service identity from fingerprinting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceIdentity {
    pub service_name: String,
    pub version: Option<String>,
    pub product: Option<String>,
    pub vendor: Option<String>,
    pub protocol: String,
    pub transport: String,
    pub port: u16,
    pub confidence: FingerprintConfidence,
    pub evidence: Vec<FingerprintEvidence>,
    pub cpe: Option<String>,
    pub possible_cves: Vec<String>,
}

impl ServiceIdentity {
    /// Check if version information is reliable enough for CVE mapping
    pub fn is_version_reliable(&self) -> bool {
        matches!(
            self.confidence,
            FingerprintConfidence::Confirmed | FingerprintConfidence::High
        ) && self.version.is_some()
    }

    /// Get a normalized service key for deduplication
    pub fn service_key(&self) -> String {
        format!(
            "{}:{}:{}:{}",
            self.service_name.to_lowercase(),
            self.protocol.to_lowercase(),
            self.transport.to_lowercase(),
            self.port
        )
    }
}

/// Enhanced fingerprint result with confidence and evidence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedFingerprint {
    pub identity: ServiceIdentity,
    pub all_alternatives: Vec<ServiceIdentity>,
    pub raw_banner: Option<String>,
    pub scan_timestamp: chrono::DateTime<chrono::Utc>,
}

impl EnhancedFingerprint {
    /// Get the best matching service identity
    pub fn best_match(&self) -> &ServiceIdentity {
        &self.identity
    }

    /// Check if there are conflicting fingerprints
    pub fn has_conflicts(&self) -> bool {
        !self.all_alternatives.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_identity(name: &str, confidence: FingerprintConfidence) -> ServiceIdentity {
        ServiceIdentity {
            service_name: name.to_string(),
            version: Some("1.0.0".to_string()),
            product: None,
            vendor: None,
            protocol: "tcp".to_string(),
            transport: "ssl".to_string(),
            port: 443,
            confidence,
            evidence: vec![],
            cpe: None,
            possible_cves: vec![],
        }
    }

    #[test]
    fn confirmed_identity_is_version_reliable() {
        let id = test_identity("nginx", FingerprintConfidence::Confirmed);
        assert!(id.is_version_reliable());
    }

    #[test]
    fn low_confidence_not_reliable() {
        let id = test_identity("nginx", FingerprintConfidence::Low);
        assert!(!id.is_version_reliable());
    }

    #[test]
    fn unknown_version_not_reliable() {
        let mut id = test_identity("nginx", FingerprintConfidence::High);
        id.version = None;
        assert!(!id.is_version_reliable());
    }

    #[test]
    fn service_key_is_stable() {
        let id = test_identity("Nginx", FingerprintConfidence::High);
        assert_eq!(id.service_key(), "nginx:tcp:ssl:443");
    }

    #[test]
    fn confidence_ordering() {
        assert!(FingerprintConfidence::Confirmed > FingerprintConfidence::High);
        assert!(FingerprintConfidence::High > FingerprintConfidence::Medium);
        assert!(FingerprintConfidence::Medium > FingerprintConfidence::Low);
        assert!(FingerprintConfidence::Low > FingerprintConfidence::Unknown);
    }

    #[test]
    fn enhanced_fingerprint_no_conflicts() {
        let fp = EnhancedFingerprint {
            identity: test_identity("nginx", FingerprintConfidence::High),
            all_alternatives: vec![],
            raw_banner: None,
            scan_timestamp: chrono::Utc::now(),
        };
        assert!(!fp.has_conflicts());
    }

    #[test]
    fn enhanced_fingerprint_with_conflicts() {
        let fp = EnhancedFingerprint {
            identity: test_identity("nginx", FingerprintConfidence::Medium),
            all_alternatives: vec![test_identity("apache", FingerprintConfidence::Low)],
            raw_banner: None,
            scan_timestamp: chrono::Utc::now(),
        };
        assert!(fp.has_conflicts());
    }
}
