use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayloadInfo {
    pub name: String,
    pub module_type: String,
    pub fullname: String,
    pub description: String,
    pub platform: Option<Vec<String>>,
    pub arch: Option<Vec<String>>,
    pub options: HashMap<String, PayloadOption>,
    pub size: Option<usize>,
    pub staged: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayloadOption {
    #[serde(rename = "type")]
    pub opt_type: String,
    pub required: bool,
    pub default: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayloadConfig {
    pub payload: String,
    pub lhost: String,
    pub lport: u16,
    pub encoder: Option<String>,
    pub iterations: Option<u32>,
    pub badchars: Option<String>,
    pub format: PayloadFormat,
    pub options: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub enum PayloadFormat {
    #[default]
    #[serde(rename = "raw")]
    Raw,
    #[serde(rename = "exe")]
    Exe,
    #[serde(rename = "dll")]
    Dll,
    #[serde(rename = "elf")]
    Elf,
    #[serde(rename = "macho")]
    Macho,
    #[serde(rename = "asp")]
    Asp,
    #[serde(rename = "jsp")]
    Jsp,
    #[serde(rename = "war")]
    War,
    #[serde(rename = "psh")]
    PowerShell,
    #[serde(rename = "vbs")]
    Vbs,
    #[serde(rename = "rb")]
    Ruby,
    #[serde(rename = "py")]
    Python,
    #[serde(rename = "c")]
    C,
    #[serde(rename = "java")]
    Java,
}

impl std::fmt::Display for PayloadFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PayloadFormat::Raw => write!(f, "raw"),
            PayloadFormat::Exe => write!(f, "exe"),
            PayloadFormat::Dll => write!(f, "dll"),
            PayloadFormat::Elf => write!(f, "elf"),
            PayloadFormat::Macho => write!(f, "macho"),
            PayloadFormat::Asp => write!(f, "asp"),
            PayloadFormat::Jsp => write!(f, "jsp"),
            PayloadFormat::War => write!(f, "war"),
            PayloadFormat::PowerShell => write!(f, "psh"),
            PayloadFormat::Vbs => write!(f, "vbs"),
            PayloadFormat::Ruby => write!(f, "rb"),
            PayloadFormat::Python => write!(f, "py"),
            PayloadFormat::C => write!(f, "c"),
            PayloadFormat::Java => write!(f, "java"),
        }
    }
}

impl std::str::FromStr for PayloadFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "raw" => Ok(PayloadFormat::Raw),
            "exe" => Ok(PayloadFormat::Exe),
            "dll" => Ok(PayloadFormat::Dll),
            "elf" => Ok(PayloadFormat::Elf),
            "macho" => Ok(PayloadFormat::Macho),
            "asp" => Ok(PayloadFormat::Asp),
            "jsp" => Ok(PayloadFormat::Jsp),
            "war" => Ok(PayloadFormat::War),
            "psh" | "powershell" => Ok(PayloadFormat::PowerShell),
            "vbs" => Ok(PayloadFormat::Vbs),
            "rb" | "ruby" => Ok(PayloadFormat::Ruby),
            "py" | "python" => Ok(PayloadFormat::Python),
            "c" => Ok(PayloadFormat::C),
            "java" => Ok(PayloadFormat::Java),
            _ => Err(format!("Unknown payload format: {}", s)),
        }
    }
}

impl PayloadConfig {
    pub fn new(payload: String, lhost: String, lport: u16) -> Self {
        Self {
            payload,
            lhost,
            lport,
            encoder: None,
            iterations: None,
            badchars: None,
            format: PayloadFormat::Raw,
            options: HashMap::new(),
        }
    }

    pub fn with_encoder(mut self, encoder: String) -> Self {
        self.encoder = Some(encoder);
        self
    }

    pub fn with_format(mut self, format: PayloadFormat) -> Self {
        self.format = format;
        self
    }

    pub fn with_option(mut self, key: String, value: String) -> Self {
        self.options.insert(key, value);
        self
    }
}
