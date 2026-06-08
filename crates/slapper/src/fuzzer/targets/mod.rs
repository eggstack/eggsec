pub mod apache;
pub mod api;
pub mod generic;
pub mod nginx;
pub mod php;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TargetType {
    Api,
    Nginx,
    Apache,
    PHP,
    Generic,
}

impl std::fmt::Display for TargetType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TargetType::Api => write!(f, "api"),
            TargetType::Nginx => write!(f, "nginx"),
            TargetType::Apache => write!(f, "apache"),
            TargetType::PHP => write!(f, "php"),
            TargetType::Generic => write!(f, "generic"),
        }
    }
}

impl std::str::FromStr for TargetType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "api" => Ok(TargetType::Api),
            "nginx" => Ok(TargetType::Nginx),
            "apache" => Ok(TargetType::Apache),
            "php" => Ok(TargetType::PHP),
            "generic" | "default" => Ok(TargetType::Generic),
            _ => Err(format!("Unknown target type: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetPayload {
    pub payload: String,
    pub description: String,
    pub category: String,
}

pub fn get_target_payloads(target: TargetType) -> Vec<TargetPayload> {
    match target {
        TargetType::Api => api::get_payloads(),
        TargetType::Nginx => nginx::get_payloads(),
        TargetType::Apache => apache::get_payloads(),
        TargetType::PHP => php::get_payloads(),
        TargetType::Generic => generic::get_payloads(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_type_display_roundtrip() {
        let types = [
            TargetType::Api,
            TargetType::Nginx,
            TargetType::Apache,
            TargetType::PHP,
            TargetType::Generic,
        ];
        for t in &types {
            let s = t.to_string();
            let parsed: TargetType = s.parse().unwrap();
            assert_eq!(*t, parsed);
        }
    }

    #[test]
    fn test_target_type_from_str_default() {
        assert_eq!(
            "default".parse::<TargetType>().unwrap(),
            TargetType::Generic
        );
    }

    #[test]
    fn test_target_type_from_str_invalid() {
        assert!("unknown".parse::<TargetType>().is_err());
    }

    #[test]
    fn test_get_target_payloads_all_non_empty() {
        let types = [
            TargetType::Api,
            TargetType::Nginx,
            TargetType::Apache,
            TargetType::PHP,
            TargetType::Generic,
        ];
        for t in &types {
            let payloads = get_target_payloads(*t);
            assert!(
                !payloads.is_empty(),
                "payloads for {:?} should not be empty",
                t
            );
        }
    }

    #[test]
    fn test_payload_fields_are_non_empty() {
        let all = [
            get_target_payloads(TargetType::Api),
            get_target_payloads(TargetType::Nginx),
            get_target_payloads(TargetType::Apache),
            get_target_payloads(TargetType::PHP),
            get_target_payloads(TargetType::Generic),
        ];
        for payloads in &all {
            for p in payloads {
                assert!(!p.description.is_empty(), "description should not be empty");
                assert!(!p.category.is_empty(), "category should not be empty");
            }
        }
    }
}
