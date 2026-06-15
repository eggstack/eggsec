use super::{PostexCategory, PostexDetection, PostexRisk, PostexTechnique};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LotlCommand {
    PowerShell,
    Wmic,
    Certutil,
    Rundll32,
    Msiexec,
    Mshta,
    Regsvr32,
    Bash,
    Curl,
    Wget,
}

impl LotlCommand {
    pub fn mitre_id(&self) -> &'static str {
        match self {
            Self::PowerShell => "T1059.001",
            Self::Wmic => "T1047",
            Self::Certutil => "T1105",
            Self::Rundll32 => "T1218.011",
            Self::Msiexec => "T1218.007",
            Self::Mshta => "T1218.005",
            Self::Regsvr32 => "T1218.010",
            Self::Bash => "T1059.004",
            Self::Curl => "T1105",
            Self::Wget => "T1105",
        }
    }

    pub fn risk(&self) -> PostexRisk {
        match self {
            Self::PowerShell | Self::Certutil => PostexRisk::High,
            Self::Rundll32 | Self::Mshta | Self::Regsvr32 => PostexRisk::Medium,
            Self::Wmic | Self::Msiexec => PostexRisk::Medium,
            Self::Bash | Self::Curl | Self::Wget => PostexRisk::Low,
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::PowerShell => "PowerShell-based command execution for defense evasion",
            Self::Wmic => "WMIC process creation and query operations",
            Self::Certutil => "certutil.exe used for file download/decode",
            Self::Rundll32 => "rundll32.exe loading malicious DLLs",
            Self::Msiexec => "msiexec.exe for software installation abuse",
            Self::Mshta => "mshta.exe for HTML Application execution",
            Self::Regsvr32 => "regsvr32.exe for script execution",
            Self::Bash => "Bash shell command execution",
            Self::Curl => "curl for file download and C2 communication",
            Self::Wget => "wget for file download and C2 communication",
        }
    }

    pub fn to_technique(&self) -> PostexTechnique {
        PostexTechnique {
            id: format!("lotl-{}", self.mitre_id().replace('.', "-")),
            name: format!("{:?}", self),
            mitre_id: self.mitre_id().to_string(),
            category: PostexCategory::Lotl,
            risk: self.risk(),
            description: self.description().to_string(),
            reversible: true,
        }
    }
}

pub fn simulate_lotl(command: LotlCommand, target: &str) -> PostexDetection {
    let technique = command.to_technique();
    PostexDetection {
        technique,
        simulated: true,
        confidence: 0.7,
        evidence: format!(
            "dry-run: {:?} execution would be simulated against {}",
            command, target
        ),
        recommendations: vec![
            format!("Monitor {:?} execution in production", command),
            "Implement application whitelisting".to_string(),
            "Enable command-line audit logging".to_string(),
        ],
    }
}
