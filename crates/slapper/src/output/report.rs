use crate::error::Result;
use serde::{Deserialize, Serialize};

pub trait Report {
    fn title(&self) -> &str;
    fn to_json(&self) -> Result<String>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReportTemplate {
    Executive,
    Technical,
    Developer,
    Compliance,
}

impl ReportTemplate {
    pub fn from_string(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "executive" => Some(Self::Executive),
            "technical" => Some(Self::Technical),
            "developer" => Some(Self::Developer),
            "compliance" => Some(Self::Compliance),
            _ => None,
        }
    }

    pub fn description(&self) -> &str {
        match self {
            Self::Executive => "High-level summary for management",
            Self::Technical => "Detailed technical findings for security team",
            Self::Developer => "Actionable items for developers",
            Self::Compliance => "Compliance-focused report (PCI-DSS, OWASP)",
        }
    }

    pub fn file_extension(&self) -> &str {
        match self {
            Self::Executive => "exec.html",
            Self::Technical => "tech.html",
            Self::Developer => "dev.html",
            Self::Compliance => "compliance.html",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportMetadata {
    pub target: String,
    pub scan_date: String,
    pub scan_type: String,
    pub template: ReportTemplate,
    pub severity_counts: SeverityCounts,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SeverityCounts {
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
    pub info: usize,
}

impl SeverityCounts {
    pub fn total(&self) -> usize {
        self.critical + self.high + self.medium + self.low + self.info
    }

    pub fn risk_score(&self) -> f64 {
        (self.critical as f64 * 10.0)
            + (self.high as f64 * 7.0)
            + (self.medium as f64 * 4.0)
            + (self.low as f64 * 1.0)
    }
}
