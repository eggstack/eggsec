use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

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

impl From<slapper_core::types::Severity> for ResponseSeverity {
    fn from(severity: slapper_core::types::Severity) -> Self {
        match severity {
            slapper_core::types::Severity::Critical => ResponseSeverity::Critical,
            slapper_core::types::Severity::High => ResponseSeverity::High,
            slapper_core::types::Severity::Medium => ResponseSeverity::Medium,
            slapper_core::types::Severity::Low => ResponseSeverity::Low,
            slapper_core::types::Severity::Info => ResponseSeverity::Info,
        }
    }
}
