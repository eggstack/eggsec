pub mod api;
pub mod apache;
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
