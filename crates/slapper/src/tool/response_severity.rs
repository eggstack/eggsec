use crate::types::Severity;
use serde::{Deserialize, Serialize};

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

    pub fn to_agent_severity(&self) -> Severity {
        match self {
            ResponseSeverity::Critical => Severity::Critical,
            ResponseSeverity::High => Severity::High,
            ResponseSeverity::Medium => Severity::Medium,
            ResponseSeverity::Low => Severity::Low,
            ResponseSeverity::Info => Severity::Info,
            ResponseSeverity::None => Severity::Info,
        }
    }

    pub fn to_option(&self) -> Option<Severity> {
        match self {
            ResponseSeverity::Critical => Some(Severity::Critical),
            ResponseSeverity::High => Some(Severity::High),
            ResponseSeverity::Medium => Some(Severity::Medium),
            ResponseSeverity::Low => Some(Severity::Low),
            ResponseSeverity::Info => Some(Severity::Info),
            ResponseSeverity::None => None,
        }
    }

    pub fn from_option(opt: Option<Severity>) -> Self {
        match opt {
            Some(Severity::Critical) => ResponseSeverity::Critical,
            Some(Severity::High) => ResponseSeverity::High,
            Some(Severity::Medium) => ResponseSeverity::Medium,
            Some(Severity::Low) => ResponseSeverity::Low,
            Some(Severity::Info) => ResponseSeverity::Info,
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
        match s.to_lowercase().as_str() {
            "critical" => Ok(ResponseSeverity::Critical),
            "high" => Ok(ResponseSeverity::High),
            "medium" | "moderate" => Ok(ResponseSeverity::Medium),
            "low" => Ok(ResponseSeverity::Low),
            "info" | "informational" => Ok(ResponseSeverity::Info),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_response_severity_ordering() {
        assert!(ResponseSeverity::Critical > ResponseSeverity::High);
        assert!(ResponseSeverity::High > ResponseSeverity::Medium);
        assert!(ResponseSeverity::Medium > ResponseSeverity::Low);
        assert!(ResponseSeverity::Low > ResponseSeverity::Info);
        assert!(ResponseSeverity::Info > ResponseSeverity::None);
    }

    #[test]
    fn test_response_severity_from_severity() {
        assert_eq!(ResponseSeverity::from(Severity::Critical), ResponseSeverity::Critical);
        assert_eq!(ResponseSeverity::from(Severity::High), ResponseSeverity::High);
        assert_eq!(ResponseSeverity::from(Severity::Medium), ResponseSeverity::Medium);
        assert_eq!(ResponseSeverity::from(Severity::Low), ResponseSeverity::Low);
        assert_eq!(ResponseSeverity::from(Severity::Info), ResponseSeverity::Info);
    }

    #[test]
    fn test_response_severity_to_option() {
        assert_eq!(ResponseSeverity::Critical.to_option(), Some(Severity::Critical));
        assert_eq!(ResponseSeverity::None.to_option(), None);
    }

    #[test]
    fn test_response_severity_from_option() {
        assert_eq!(ResponseSeverity::from_option(Some(Severity::Critical)), ResponseSeverity::Critical);
        assert_eq!(ResponseSeverity::from_option(None), ResponseSeverity::None);
    }

    #[test]
    fn test_response_severity_parse() {
        assert_eq!("critical".parse::<ResponseSeverity>().unwrap(), ResponseSeverity::Critical);
        assert_eq!("high".parse::<ResponseSeverity>().unwrap(), ResponseSeverity::High);
        assert_eq!("medium".parse::<ResponseSeverity>().unwrap(), ResponseSeverity::Medium);
        assert_eq!("moderate".parse::<ResponseSeverity>().unwrap(), ResponseSeverity::Medium);
        assert_eq!("low".parse::<ResponseSeverity>().unwrap(), ResponseSeverity::Low);
        assert_eq!("info".parse::<ResponseSeverity>().unwrap(), ResponseSeverity::Info);
        assert_eq!("unknown".parse::<ResponseSeverity>().unwrap(), ResponseSeverity::None);
    }

    #[test]
    fn test_response_severity_display() {
        assert_eq!(ResponseSeverity::Critical.to_string(), "critical");
        assert_eq!(ResponseSeverity::High.to_string(), "high");
        assert_eq!(ResponseSeverity::Medium.to_string(), "medium");
        assert_eq!(ResponseSeverity::Low.to_string(), "low");
        assert_eq!(ResponseSeverity::Info.to_string(), "info");
        assert_eq!(ResponseSeverity::None.to_string(), "none");
    }
}
