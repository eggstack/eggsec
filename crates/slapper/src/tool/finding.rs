use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

use crate::fuzzer::FuzzResult;
use crate::types::Severity;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub id: String,
    pub finding_type: FindingType,
    pub severity: ResponseSeverity,
    pub title: String,
    pub description: String,
    pub location: String,
    pub evidence: Option<String>,
    pub cve_ids: Vec<String>,
    pub remediation: Option<String>,
    pub references: Vec<String>,
    pub metadata: FxHashMap<String, serde_json::Value>,
}

impl Finding {
    pub fn new(
        finding_type: FindingType,
        severity: ResponseSeverity,
        title: impl Into<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            finding_type,
            severity,
            title: title.into(),
            description: String::new(),
            location: String::new(),
            evidence: None,
            cve_ids: vec![],
            remediation: None,
            references: vec![],
            metadata: FxHashMap::default(),
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn at_location(mut self, location: impl Into<String>) -> Self {
        self.location = location.into();
        self
    }

    pub fn with_evidence(mut self, evidence: impl Into<String>) -> Self {
        self.evidence = Some(evidence.into());
        self
    }

    pub fn with_cve(mut self, cve: impl Into<String>) -> Self {
        self.cve_ids.push(cve.into());
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FindingType {
    Vulnerability,
    Information,
    Weakness,
    Configuration,
    Misconfiguration,
    SensitiveData,
    Banner,
    Technology,
    Service,
    Endpoint,
    Subdomain,
    OpenPort,
}

impl std::fmt::Display for FindingType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FindingType::Vulnerability => write!(f, "vulnerability"),
            FindingType::Information => write!(f, "information"),
            FindingType::Weakness => write!(f, "weakness"),
            FindingType::Configuration => write!(f, "configuration"),
            FindingType::Misconfiguration => write!(f, "misconfiguration"),
            FindingType::SensitiveData => write!(f, "sensitive_data"),
            FindingType::Banner => write!(f, "banner"),
            FindingType::Technology => write!(f, "technology"),
            FindingType::Service => write!(f, "service"),
            FindingType::Endpoint => write!(f, "endpoint"),
            FindingType::Subdomain => write!(f, "subdomain"),
            FindingType::OpenPort => write!(f, "open_port"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResponseSeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
    None,
}

impl ResponseSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            ResponseSeverity::Critical => "critical",
            ResponseSeverity::High => "high",
            ResponseSeverity::Medium => "medium",
            ResponseSeverity::Low => "low",
            ResponseSeverity::Info => "info",
            ResponseSeverity::None => "none",
        }
    }

    fn as_int(&self) -> u8 {
        match self {
            ResponseSeverity::Critical => 5,
            ResponseSeverity::High => 4,
            ResponseSeverity::Medium => 3,
            ResponseSeverity::Low => 2,
            ResponseSeverity::Info => 1,
            ResponseSeverity::None => 0,
        }
    }

    pub fn to_agent_severity(&self) -> crate::types::Severity {
        match self {
            ResponseSeverity::Critical => crate::types::Severity::Critical,
            ResponseSeverity::High => crate::types::Severity::High,
            ResponseSeverity::Medium => crate::types::Severity::Medium,
            ResponseSeverity::Low => crate::types::Severity::Low,
            ResponseSeverity::Info => crate::types::Severity::Info,
            ResponseSeverity::None => crate::types::Severity::Info,
        }
    }

    pub fn to_option(&self) -> Option<crate::types::Severity> {
        match self {
            ResponseSeverity::Critical => Some(crate::types::Severity::Critical),
            ResponseSeverity::High => Some(crate::types::Severity::High),
            ResponseSeverity::Medium => Some(crate::types::Severity::Medium),
            ResponseSeverity::Low => Some(crate::types::Severity::Low),
            ResponseSeverity::Info => Some(crate::types::Severity::Info),
            ResponseSeverity::None => None,
        }
    }

    pub fn from_option(opt: Option<crate::types::Severity>) -> Self {
        match opt {
            Some(crate::types::Severity::Critical) => ResponseSeverity::Critical,
            Some(crate::types::Severity::High) => ResponseSeverity::High,
            Some(crate::types::Severity::Medium) => ResponseSeverity::Medium,
            Some(crate::types::Severity::Low) => ResponseSeverity::Low,
            Some(crate::types::Severity::Info) => ResponseSeverity::Info,
            None => ResponseSeverity::None,
        }
    }
}

impl Ord for ResponseSeverity {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_int().cmp(&other.as_int())
    }
}

impl PartialOrd for ResponseSeverity {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::str::FromStr for ResponseSeverity {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            s if s.eq_ignore_ascii_case("critical") => Ok(ResponseSeverity::Critical),
            s if s.eq_ignore_ascii_case("high") => Ok(ResponseSeverity::High),
            s if s.eq_ignore_ascii_case("medium") || s.eq_ignore_ascii_case("moderate") => {
                Ok(ResponseSeverity::Medium)
            }
            s if s.eq_ignore_ascii_case("low") => Ok(ResponseSeverity::Low),
            s if s.eq_ignore_ascii_case("info") || s.eq_ignore_ascii_case("informational") => {
                Ok(ResponseSeverity::Info)
            }
            _ => Ok(ResponseSeverity::None),
        }
    }
}

impl std::fmt::Display for ResponseSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<Severity> for ResponseSeverity {
    fn from(severity: Severity) -> Self {
        match severity {
            Severity::Critical => ResponseSeverity::Critical,
            Severity::High => ResponseSeverity::High,
            Severity::Medium => ResponseSeverity::Medium,
            Severity::Low => ResponseSeverity::Low,
            Severity::Info => ResponseSeverity::Info,
        }
    }
}

impl From<FuzzResult> for Finding {
    fn from(result: FuzzResult) -> Self {
        let severity = ResponseSeverity::from(result.detected_severity);
        let description = if result.leaks_found.is_empty() {
            String::new()
        } else {
            result.leaks_found.join(", ")
        };
        let location = format!(
            "{} - {}",
            result.payload.payload_type, result.payload.payload
        );
        let mut metadata = FxHashMap::default();
        metadata.insert(
            "status_code".to_string(),
            serde_json::Value::Number(result.status_code.into()),
        );
        metadata.insert(
            "response_time_ms".to_string(),
            serde_json::Value::Number(result.response_time_ms.into()),
        );
        metadata.insert(
            "is_waf_blocked".to_string(),
            serde_json::Value::Bool(result.is_waf_blocked),
        );
        metadata.insert(
            "is_anomaly".to_string(),
            serde_json::Value::Bool(result.is_anomaly),
        );
        metadata.insert(
            "payload".to_string(),
            serde_json::to_value(&result.payload)
                .inspect_err(|e| {
                    tracing::debug!(error = %e, "Failed to serialize payload metadata");
                })
                .unwrap_or_default(),
        );

        Finding {
            id: uuid::Uuid::new_v4().to_string(),
            finding_type: FindingType::Vulnerability,
            severity,
            title: result.payload.description,
            description,
            location,
            evidence: None,
            cve_ids: vec![],
            remediation: None,
            references: vec![],
            metadata,
        }
    }
}

impl From<crate::scanner::ports::PortResult> for Finding {
    fn from(result: crate::scanner::ports::PortResult) -> Self {
        Finding {
            id: uuid::Uuid::new_v4().to_string(),
            finding_type: FindingType::OpenPort,
            severity: ResponseSeverity::Info,
            title: format!("Open port: {}/tcp ({})", result.port, result.service),
            description: format!(
                "Port {} is open running service: {}",
                result.port, result.service
            ),
            location: format!("{}:{}", result.port, result.service),
            evidence: None,
            cve_ids: vec![],
            remediation: None,
            references: vec![],
            metadata: {
                let mut m = FxHashMap::default();
                m.insert(
                    "port".to_string(),
                    serde_json::Value::Number(result.port.into()),
                );
                m.insert(
                    "service".to_string(),
                    serde_json::Value::String(result.service),
                );
                m.insert(
                    "status".to_string(),
                    serde_json::Value::String(result.status),
                );
                m
            },
        }
    }
}

impl From<crate::scanner::fingerprint::ServiceFingerprint> for Finding {
    fn from(fp: crate::scanner::fingerprint::ServiceFingerprint) -> Self {
        let service_info = fp
            .product
            .as_ref()
            .or(fp.version.as_ref())
            .map(|s| s.as_str())
            .unwrap_or("unknown");
        let banner_snippet = fp.banner.as_ref().map(|b| {
            let trimmed = b.chars().take(200).collect::<String>();
            if b.len() > 200 {
                format!("{}...", trimmed)
            } else {
                trimmed
            }
        });

        Finding {
            id: uuid::Uuid::new_v4().to_string(),
            finding_type: FindingType::Service,
            severity: ResponseSeverity::Info,
            title: format!("Service detected: {} on port {}", service_info, fp.port),
            description: format!(
                "Detected {} (confidence: {}){}",
                service_info,
                fp.confidence,
                fp.version
                    .as_ref()
                    .map(|v| format!(" version {}", v))
                    .unwrap_or_default()
            ),
            location: format!("port {}", fp.port),
            evidence: banner_snippet,
            cve_ids: vec![],
            remediation: None,
            references: vec![],
            metadata: {
                let mut m = FxHashMap::default();
                m.insert(
                    "port".to_string(),
                    serde_json::Value::Number(fp.port.into()),
                );
                m.insert(
                    "service".to_string(),
                    serde_json::Value::String(fp.service.clone()),
                );
                m.insert(
                    "product".to_string(),
                    serde_json::to_value(&fp.product)
                        .inspect_err(|e| {
                            tracing::debug!(error = %e, "Failed to serialize product metadata");
                        })
                        .unwrap_or_default(),
                );
                m.insert(
                    "version".to_string(),
                    serde_json::to_value(&fp.version)
                        .inspect_err(|e| {
                            tracing::debug!(error = %e, "Failed to serialize version metadata");
                        })
                        .unwrap_or_default(),
                );
                m.insert(
                    "confidence".to_string(),
                    serde_json::Value::Number(fp.confidence.into()),
                );
                m.insert(
                    "banner".to_string(),
                    serde_json::to_value(&fp.banner)
                        .inspect_err(|e| {
                            tracing::debug!(error = %e, "Failed to serialize banner metadata");
                        })
                        .unwrap_or_default(),
                );
                m
            },
        }
    }
}

impl From<crate::scanner::udp_fingerprint::UdpServiceFingerprint> for Finding {
    fn from(fp: crate::scanner::udp_fingerprint::UdpServiceFingerprint) -> Self {
        let banner_snippet = fp.banner.as_ref().map(|b| {
            let trimmed = b.chars().take(200).collect::<String>();
            if b.len() > 200 {
                format!("{}...", trimmed)
            } else {
                trimmed
            }
        });

        Finding {
            id: uuid::Uuid::new_v4().to_string(),
            finding_type: FindingType::Service,
            severity: ResponseSeverity::Info,
            title: format!("UDP service detected: {} on port {}", fp.service, fp.port),
            description: format!("Detected {} (confidence: {})", fp.service, fp.confidence),
            location: format!("port {}", fp.port),
            evidence: banner_snippet,
            cve_ids: vec![],
            remediation: None,
            references: vec![],
            metadata: {
                let mut m = FxHashMap::default();
                m.insert(
                    "port".to_string(),
                    serde_json::Value::Number(fp.port.into()),
                );
                m.insert("service".to_string(), serde_json::Value::String(fp.service));
                m.insert(
                    "response".to_string(),
                    serde_json::to_value(&fp.response)
                        .inspect_err(|e| {
                            tracing::debug!(error = %e, "Failed to serialize response metadata");
                        })
                        .unwrap_or_default(),
                );
                m.insert(
                    "banner".to_string(),
                    serde_json::to_value(&fp.banner)
                        .inspect_err(|e| {
                            tracing::debug!(error = %e, "Failed to serialize banner metadata");
                        })
                        .unwrap_or_default(),
                );
                m.insert(
                    "confidence".to_string(),
                    serde_json::Value::Number(fp.confidence.into()),
                );
                m
            },
        }
    }
}

impl From<crate::scanner::endpoints::EndpointResult> for Finding {
    fn from(result: crate::scanner::endpoints::EndpointResult) -> Self {
        let severity = if result.interesting {
            ResponseSeverity::Low
        } else {
            ResponseSeverity::Info
        };
        let title = if result.interesting {
            format!(
                "Interesting endpoint: {} ({})",
                result.path, result.status_code
            )
        } else {
            format!(
                "Endpoint discovered: {} ({})",
                result.path, result.status_code
            )
        };

        Finding {
            id: uuid::Uuid::new_v4().to_string(),
            finding_type: FindingType::Endpoint,
            severity,
            title,
            description: format!(
                "Path: {}, Status: {} ({}){}",
                result.path,
                result.status_code,
                result.status_text,
                result
                    .content_length
                    .map(|l| format!(", {} bytes", l))
                    .unwrap_or_default()
            ),
            location: result.path.clone(),
            evidence: None,
            cve_ids: vec![],
            remediation: None,
            references: vec![],
            metadata: {
                let mut m = FxHashMap::default();
                m.insert("path".to_string(), serde_json::Value::String(result.path));
                m.insert(
                    "status_code".to_string(),
                    serde_json::Value::Number(result.status_code.into()),
                );
                m.insert(
                    "status_text".to_string(),
                    serde_json::Value::String(result.status_text),
                );
                m.insert(
                    "content_length".to_string(),
                    serde_json::to_value(result.content_length).inspect_err(|e| {
                        tracing::debug!(error = %e, "Failed to serialize content_length metadata");
                    }).unwrap_or_default(),
                );
                m.insert(
                    "response_time_ms".to_string(),
                    serde_json::Value::Number(result.response_time_ms.into()),
                );
                m.insert(
                    "interesting".to_string(),
                    serde_json::Value::Bool(result.interesting),
                );
                m
            },
        }
    }
}

impl From<crate::recon::cve::VulnerabilityInfo> for Finding {
    fn from(v: crate::recon::cve::VulnerabilityInfo) -> Self {
        let severity = match v.severity.to_lowercase().as_str() {
            "critical" => ResponseSeverity::Critical,
            "high" => ResponseSeverity::High,
            "medium" | "moderate" => ResponseSeverity::Medium,
            "low" => ResponseSeverity::Low,
            _ => ResponseSeverity::Info,
        };

        Finding {
            id: uuid::Uuid::new_v4().to_string(),
            finding_type: FindingType::Vulnerability,
            severity,
            title: format!(
                "{}: {}",
                v.cve_id,
                v.description.split('.').next().unwrap_or(&v.description)
            ),
            description: v.description.clone(),
            location: v.affected_product.clone(),
            evidence: None,
            cve_ids: vec![v.cve_id.clone()],
            remediation: None,
            references: v.references.clone(),
            metadata: {
                let mut m = FxHashMap::default();
                m.insert(
                    "cvss_score".to_string(),
                    serde_json::Value::Number(
                        serde_json::Number::from_f64(v.cvss_score as f64)
                            .unwrap_or(serde_json::Number::from(0)),
                    ),
                );
                m.insert(
                    "severity".to_string(),
                    serde_json::Value::String(v.severity),
                );
                m.insert(
                    "affected_product".to_string(),
                    serde_json::Value::String(v.affected_product),
                );
                m.insert(
                    "published_date".to_string(),
                    serde_json::to_value(&v.published_date).inspect_err(|e| {
                        tracing::debug!(error = %e, "Failed to serialize published_date metadata");
                    }).unwrap_or_default(),
                );
                m
            },
        }
    }
}
