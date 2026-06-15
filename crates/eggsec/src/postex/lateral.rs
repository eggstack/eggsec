use super::{PostexCategory, PostexDetection, PostexRisk, PostexTechnique};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LateralTechnique {
    SmbShare,
    RdpSession,
    PortForward,
    SocksProxy,
    WinRm,
    PsExec,
}

impl LateralTechnique {
    pub fn to_technique(&self) -> PostexTechnique {
        let (id, name, mitre_id, risk, desc) = match self {
            Self::SmbShare => (
                "lateral-smb".to_string(),
                "SMB Lateral Movement".to_string(),
                "T1021.002".to_string(),
                PostexRisk::High,
                "Detection of SMB-based lateral movement techniques".to_string(),
            ),
            Self::RdpSession => (
                "lateral-rdp".to_string(),
                "RDP Lateral Movement".to_string(),
                "T1021.001".to_string(),
                PostexRisk::High,
                "Detection of RDP-based lateral movement".to_string(),
            ),
            Self::PortForward => (
                "lateral-port-forward".to_string(),
                "Port Forwarding".to_string(),
                "T1090".to_string(),
                PostexRisk::Medium,
                "Detection of network port forwarding for pivoting".to_string(),
            ),
            Self::SocksProxy => (
                "lateral-socks".to_string(),
                "SOCKS Proxy".to_string(),
                "T1090.002".to_string(),
                PostexRisk::Medium,
                "Detection of SOCKS proxy setup for traffic relay".to_string(),
            ),
            Self::WinRm => (
                "lateral-winrm".to_string(),
                "WinRM Remote Execution".to_string(),
                "T1021.006".to_string(),
                PostexRisk::High,
                "Detection of WinRM-based remote command execution".to_string(),
            ),
            Self::PsExec => (
                "lateral-psexec".to_string(),
                "PsExec Remote Execution".to_string(),
                "T1021.002".to_string(),
                PostexRisk::Critical,
                "Detection of PsExec-style remote service execution".to_string(),
            ),
        };
        PostexTechnique {
            id,
            name,
            mitre_id,
            category: PostexCategory::LateralMovement,
            risk,
            description: desc,
            reversible: true,
        }
    }
}

pub fn simulate_lateral(
    technique: LateralTechnique,
    source: &str,
    target: &str,
) -> PostexDetection {
    let tech = technique.to_technique();
    PostexDetection {
        technique: tech,
        simulated: true,
        confidence: 0.65,
        evidence: format!(
            "dry-run: {:?} from {} to {} would be simulated",
            technique, source, target
        ),
        recommendations: vec![
            "Monitor lateral movement attempts in production".to_string(),
            "Implement network segmentation".to_string(),
            "Enable SMB/RDP audit logging".to_string(),
        ],
    }
}
