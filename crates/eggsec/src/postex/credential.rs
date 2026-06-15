use super::{PostexCategory, PostexDetection, PostexRisk, PostexTechnique};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CredentialTechnique {
    LsassDump,
    TokenImpersonation,
    PasswordSpray,
    Kerberoasting,
    Dcsync,
    LdapQuery,
}

impl CredentialTechnique {
    pub fn to_technique(&self) -> PostexTechnique {
        let (id, name, mitre_id, risk, desc, reversible) = match self {
            Self::LsassDump => (
                "cred-lsass".to_string(),
                "LSASS Memory Dump".to_string(),
                "T1003.001".to_string(),
                PostexRisk::Critical,
                "Detection of LSASS process memory access for credential extraction".to_string(),
                false,
            ),
            Self::TokenImpersonation => (
                "cred-token".to_string(),
                "Token Impersonation".to_string(),
                "T1134".to_string(),
                PostexRisk::High,
                "Detection of access token manipulation for privilege escalation".to_string(),
                true,
            ),
            Self::PasswordSpray => (
                "cred-spray".to_string(),
                "Password Spraying".to_string(),
                "T1110.003".to_string(),
                PostexRisk::High,
                "Detection of password spraying against authentication endpoints".to_string(),
                true,
            ),
            Self::Kerberoasting => (
                "cred-kerberoast".to_string(),
                "Kerberoasting".to_string(),
                "T1558.003".to_string(),
                PostexRisk::Critical,
                "Detection of Kerberos service ticket extraction for offline cracking".to_string(),
                true,
            ),
            Self::Dcsync => (
                "cred-dcsync".to_string(),
                "DCSync Attack".to_string(),
                "T1003.006".to_string(),
                PostexRisk::Critical,
                "Detection of DCSync replication request for credential extraction".to_string(),
                true,
            ),
            Self::LdapQuery => (
                "cred-ldap".to_string(),
                "LDAP Query for Credentials".to_string(),
                "T1087.002".to_string(),
                PostexRisk::Medium,
                "Detection of LDAP queries targeting credential objects".to_string(),
                true,
            ),
        };
        PostexTechnique {
            id,
            name,
            mitre_id,
            category: PostexCategory::CredentialAccess,
            risk,
            description: desc,
            reversible,
        }
    }
}

pub fn simulate_credential(technique: CredentialTechnique, target: &str) -> PostexDetection {
    let tech = technique.to_technique();
    let rate_limit_note = match technique {
        CredentialTechnique::PasswordSpray => {
            " (rate-limited: max 1 attempt per 10 minutes in lab mode)".to_string()
        }
        _ => String::new(),
    };
    PostexDetection {
        technique: tech,
        simulated: true,
        confidence: 0.6,
        evidence: format!(
            "dry-run: {:?} against {} would be simulated{}",
            technique, target, rate_limit_note
        ),
        recommendations: vec![
            "Enable credential guard / LSA protection".to_string(),
            "Implement tiered administration model".to_string(),
            "Monitor for anomalous authentication patterns".to_string(),
        ],
    }
}
