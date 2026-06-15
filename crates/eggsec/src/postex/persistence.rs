use super::{PostexCategory, PostexDetection, PostexRisk, PostexTechnique};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PersistenceType {
    RegistryRunKey,
    ScheduledTask,
    ServiceCreation,
    DllHijack,
    StartupFolder,
    WmiEventSubscription,
}

impl PersistenceType {
    pub fn to_technique(&self) -> PostexTechnique {
        let (id, name, mitre_id, risk, desc) = match self {
            Self::RegistryRunKey => (
                "persist-registry".to_string(),
                "Registry Run Key Persistence".to_string(),
                "T1547.001".to_string(),
                PostexRisk::High,
                "Detection of registry-based persistence via Run/RunOnce keys".to_string(),
            ),
            Self::ScheduledTask => (
                "persist-scheduled-task".to_string(),
                "Scheduled Task Persistence".to_string(),
                "T1053.005".to_string(),
                PostexRisk::High,
                "Detection of scheduled task creation for persistence".to_string(),
            ),
            Self::ServiceCreation => (
                "persist-service".to_string(),
                "Service Creation Persistence".to_string(),
                "T1543.003".to_string(),
                PostexRisk::Critical,
                "Detection of Windows service creation for persistence".to_string(),
            ),
            Self::DllHijack => (
                "persist-dll-hijack".to_string(),
                "DLL Side-Loading".to_string(),
                "T1574.002".to_string(),
                PostexRisk::Critical,
                "Detection of DLL side-loading via search order hijacking".to_string(),
            ),
            Self::StartupFolder => (
                "persist-startup".to_string(),
                "Startup Folder Persistence".to_string(),
                "T1547.001".to_string(),
                PostexRisk::Medium,
                "Detection of startup folder file placement for persistence".to_string(),
            ),
            Self::WmiEventSubscription => (
                "persist-wmi".to_string(),
                "WMI Event Subscription".to_string(),
                "T1546.003".to_string(),
                PostexRisk::High,
                "Detection of WMI event subscription for persistence".to_string(),
            ),
        };
        PostexTechnique {
            id,
            name,
            mitre_id,
            category: PostexCategory::Persistence,
            risk,
            description: desc,
            reversible: true,
        }
    }
}

pub fn simulate_persistence(persistence: PersistenceType, target: &str) -> PostexDetection {
    let technique = persistence.to_technique();
    let cleanup = generate_cleanup_command(persistence);
    PostexDetection {
        technique,
        simulated: true,
        confidence: 0.8,
        evidence: format!(
            "dry-run: {:?} persistence would be simulated against {}. Cleanup: {}",
            persistence, target, cleanup
        ),
        recommendations: vec![
            "Verify persistence mechanism detection in lab environment".to_string(),
            format!("Cleanup command: {}", cleanup),
            "Ensure reversibility in lab mode".to_string(),
        ],
    }
}

pub fn generate_cleanup_command(persistence: PersistenceType) -> String {
    match persistence {
        PersistenceType::RegistryRunKey => {
            "reg delete HKLM\\Software\\Microsoft\\Windows\\CurrentVersion\\Run /v EggsecLab /f"
                .to_string()
        }
        PersistenceType::ScheduledTask => {
            "schtasks /delete /tn EggsecLab /f".to_string()
        }
        PersistenceType::ServiceCreation => "sc delete EggsecLabService".to_string(),
        PersistenceType::DllHijack => {
            "Remove-Item -Path \"$env:TEMP\\eggsec_lab.dll\" -Force".to_string()
        }
        PersistenceType::StartupFolder => {
            "Remove-Item -Path \"$env:APPDATA\\Microsoft\\Windows\\Start Menu\\Programs\\Startup\\eggsec_lab.lnk\" -Force"
                .to_string()
        }
        PersistenceType::WmiEventSubscription => {
            "Get-WmiObject -Query \"SELECT * FROM __EventFilter WHERE Name='EggsecLabFilter'\" | Remove-WmiObject"
                .to_string()
        }
    }
}
