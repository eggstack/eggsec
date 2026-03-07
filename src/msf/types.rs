use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct MsfResponse {
    #[serde(default)]
    pub token: Option<String>,

    #[serde(default)]
    pub result: Option<String>,

    #[serde(default)]
    pub version: Option<String>,

    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MsfError {
    #[serde(default)]
    pub error: bool,

    #[serde(default)]
    pub error_class: Option<String>,

    #[serde(default)]
    pub error_message: String,
}

impl std::fmt::Display for MsfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.error_message)
    }
}

impl std::error::Error for MsfError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModuleType {
    #[serde(rename = "exploit")]
    Exploit,
    #[serde(rename = "auxiliary")]
    Auxiliary,
    #[serde(rename = "post")]
    Post,
    #[serde(rename = "payload")]
    Payload,
    #[serde(rename = "encoder")]
    Encoder,
    #[serde(rename = "nop")]
    Nop,
}

impl std::fmt::Display for ModuleType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModuleType::Exploit => write!(f, "exploit"),
            ModuleType::Auxiliary => write!(f, "auxiliary"),
            ModuleType::Post => write!(f, "post"),
            ModuleType::Payload => write!(f, "payload"),
            ModuleType::Encoder => write!(f, "encoder"),
            ModuleType::Nop => write!(f, "nop"),
        }
    }
}

impl std::str::FromStr for ModuleType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "exploit" => Ok(ModuleType::Exploit),
            "auxiliary" => Ok(ModuleType::Auxiliary),
            "post" => Ok(ModuleType::Post),
            "payload" => Ok(ModuleType::Payload),
            "encoder" => Ok(ModuleType::Encoder),
            "nop" => Ok(ModuleType::Nop),
            _ => Err(format!("Unknown module type: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionType {
    #[serde(rename = "shell")]
    Shell,
    #[serde(rename = "meterpreter")]
    Meterpreter,
    #[serde(rename = "vnc")]
    Vnc,
}

impl std::fmt::Display for SessionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionType::Shell => write!(f, "shell"),
            SessionType::Meterpreter => write!(f, "meterpreter"),
            SessionType::Vnc => write!(f, "vnc"),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Platform {
    pub name: String,
    pub arch: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleOption {
    #[serde(rename = "type")]
    pub opt_type: String,
    pub required: bool,
    pub default: Option<String>,
    pub description: Option<String>,
    pub enums: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedOption {
    pub default: Option<String>,
    pub desc: Option<String>,
    pub name: String,
    #[serde(rename = "type")]
    pub opt_type: Option<String>,
}
