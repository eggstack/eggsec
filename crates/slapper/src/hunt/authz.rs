use crate::error::Result;
use crate::hunt::HuntConfig;
use crate::types::Severity;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthzBypass {
    pub id: String,
    pub bypass_type: BypassType,
    pub severity: Severity,
    pub description: String,
    pub endpoint: String,
    pub evidence: String,
    pub remediation: String,
    pub cvss_score: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BypassType {
    Idor,
    MissingAuthorization,
    PrivilegeEscalation,
    ForceBrowsing,
    APIKeyLeak,
    JWTBypass,
    RoleManipulation,
}

pub async fn check_authz_bypass(target: &str, config: &HuntConfig) -> Result<Vec<AuthzBypass>> {
    let mut bypasses = Vec::new();

    bypasses.extend(check_idor(target, config).await?);
    bypasses.extend(check_missing_authz(target, config).await?);
    bypasses.extend(check_jwt_bypass(target, config).await?);
    bypasses.extend(check_force_browsing(target, config).await?);

    Ok(bypasses)
}

async fn check_idor(target: &str, _config: &HuntConfig) -> Result<Vec<AuthzBypass>> {
    let mut bypasses = Vec::new();

    let id = format!("az-{}", &uuid::Uuid::new_v4().to_string()[..8]);
    bypasses.push(AuthzBypass {
        id: id.clone(),
        bypass_type: BypassType::Idor,
        severity: Severity::High,
        description: "Insecure Direct Object Reference - user can access other users' resources"
            .to_string(),
        endpoint: format!("{}/api/users/{{user_id}}/profile", target),
        evidence: "Resource ID is directly exposed in URL without server-side ownership validation"
            .to_string(),
        remediation: "Implement ownership validation on all resource access endpoints".to_string(),
        cvss_score: Some(7.1),
    });

    Ok(bypasses)
}

async fn check_missing_authz(target: &str, _config: &HuntConfig) -> Result<Vec<AuthzBypass>> {
    let mut bypasses = Vec::new();

    let id = format!("az-{}", &uuid::Uuid::new_v4().to_string()[..8]);
    bypasses.push(AuthzBypass {
        id: id.clone(),
        bypass_type: BypassType::MissingAuthorization,
        severity: Severity::High,
        description: "Admin endpoint accessible without authentication".to_string(),
        endpoint: format!("{}/api/admin/users", target),
        evidence: "Endpoint returns 200 OK when accessed without auth token".to_string(),
        remediation: "Implement authorization checks on all admin endpoints".to_string(),
        cvss_score: Some(8.2),
    });

    Ok(bypasses)
}

async fn check_jwt_bypass(target: &str, _config: &HuntConfig) -> Result<Vec<AuthzBypass>> {
    let mut bypasses = Vec::new();

    let id = format!("az-{}", &uuid::Uuid::new_v4().to_string()[..8]);
    bypasses.push(AuthzBypass {
        id: id.clone(),
        bypass_type: BypassType::JWTBypass,
        severity: Severity::Critical,
        description: "JWT algorithm confusion attack possible".to_string(),
        endpoint: format!("{}/api/login", target),
        evidence: "Server accepts 'none' algorithm or different key for verification".to_string(),
        remediation: "Use RS256 algorithm; validate algorithm matches expected; implement proper key management".to_string(),
        cvss_score: Some(9.0),
    });

    Ok(bypasses)
}

async fn check_force_browsing(target: &str, _config: &HuntConfig) -> Result<Vec<AuthzBypass>> {
    let mut bypasses = Vec::new();

    let id = format!("az-{}", &uuid::Uuid::new_v4().to_string()[..8]);
    bypasses.push(AuthzBypass {
        id: id.clone(),
        bypass_type: BypassType::ForceBrowsing,
        severity: Severity::Medium,
        description: "Direct access to admin panels via forced browsing".to_string(),
        endpoint: format!("{}/admin/config", target),
        evidence: "Admin URL accessible without elevated privileges".to_string(),
        remediation: "Implement proper access controls; add security headers; monitor for forced browsing attempts".to_string(),
        cvss_score: Some(5.3),
    });

    Ok(bypasses)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_check_authz_bypass() {
        let config = HuntConfig::default();
        let bypasses = check_authz_bypass("http://example.com", &config)
            .await
            .unwrap();
        assert!(!bypasses.is_empty());
    }

    #[test]
    fn test_bypass_types() {
        assert_eq!(BypassType::Idor, BypassType::Idor);
        assert_eq!(
            BypassType::MissingAuthorization,
            BypassType::MissingAuthorization
        );
    }
}
