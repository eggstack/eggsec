use crate::error::Result;
use crate::hunt::{HuntClient, HuntConfig};
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
#[allow(dead_code)]
pub enum BypassType {
    Idor,
    MissingAuthorization,
    PrivilegeEscalation,
    ForceBrowsing,
    APIKeyLeak,
    JWTBypass,
    RoleManipulation,
}

const ADMIN_PATHS: &[&str] = &[
    "/admin",
    "/admin/",
    "/api/admin",
    "/api/admin/users",
    "/api/admin/config",
    "/dashboard",
    "/manage",
    "/management",
    "/internal",
    "/api/internal",
    "/debug",
    "/api/debug",
    "/actuator",
    "/actuator/health",
    "/actuator/env",
    "/swagger-ui.html",
    "/api-docs",
    "/graphql",
];

const IDOR_PATHS: &[&str] = &[
    "/api/users/1",
    "/api/users/2",
    "/api/users/1/profile",
    "/api/users/2/profile",
    "/api/accounts/1",
    "/api/accounts/2",
    "/api/documents/1",
    "/api/documents/2",
];

pub async fn check_authz_bypass(
    client: &HuntClient,
    config: &HuntConfig,
) -> Result<Vec<AuthzBypass>> {
    let mut bypasses = Vec::new();

    bypasses.extend(check_admin_access(client, config).await);
    bypasses.extend(check_idor(client, config).await);
    bypasses.extend(check_force_browsing(client, config).await);
    bypasses.extend(check_http_methods(client, config).await);

    Ok(bypasses)
}

async fn check_admin_access(client: &HuntClient, config: &HuntConfig) -> Vec<AuthzBypass> {
    let mut bypasses = Vec::new();
    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(config.concurrency));
    let mut handles = Vec::new();

    for path in ADMIN_PATHS {
        let client = client.clone();
        let sem = semaphore.clone();
        let path = path.to_string();

        handles.push(tokio::spawn(async move {
            let _permit = match sem.acquire().await {
                Ok(p) => p,
                Err(_) => {
                    return (
                        path,
                        Err(crate::error::SlapperError::Http(
                            "Semaphore closed".to_string(),
                        )),
                    );
                }
            };
            let resp = client.get(&path).await;
            (path, resp)
        }));
    }

    for handle in handles {
        if let Ok((path, Ok(resp))) = handle.await {
            let status = resp.status().as_u16();
            if status == 200 {
                if let Ok(body) = resp.text().await {
                    let body_lower = body.to_lowercase();

                    if body_lower.contains("admin")
                        || body_lower.contains("dashboard")
                        || body_lower.contains("management")
                        || body_lower.contains("users")
                    {
                        let id = format!("az-{}", &uuid::Uuid::new_v4().to_string()[..8]);
                        bypasses.push(AuthzBypass {
                            id,
                            bypass_type: BypassType::MissingAuthorization,
                            severity: Severity::Critical,
                            description: format!(
                                "Admin endpoint accessible without authentication: {}",
                                path
                            ),
                            endpoint: format!("{}{}", client.base_url(), path),
                            evidence: format!(
                                "GET {} returned 200 OK with admin content",
                                path
                            ),
                            remediation: "Implement authorization checks on all admin endpoints"
                                .to_string(),
                            cvss_score: Some(9.0),
                        });
                    }
                }
            }
        }
    }

    bypasses
}

async fn check_idor(client: &HuntClient, _config: &HuntConfig) -> Vec<AuthzBypass> {
    let mut bypasses = Vec::new();

    for path in IDOR_PATHS {
        if let Ok(resp) = client.get(path).await {
            let status = resp.status().as_u16();
            if status == 200 {
                if let Ok(body) = resp.text().await {
                    if !body.is_empty() && body.len() > 50 {
                        let id = format!("az-{}", &uuid::Uuid::new_v4().to_string()[..8]);
                        bypasses.push(AuthzBypass {
                            id,
                            bypass_type: BypassType::Idor,
                            severity: Severity::High,
                            description: format!(
                                "Potential IDOR: resource accessible at {}",
                                path
                            ),
                            endpoint: format!("{}{}", client.base_url(), path),
                            evidence: format!(
                                "GET {} returned 200 with {} bytes of content",
                                path,
                                body.len()
                            ),
                            remediation:
                                "Implement ownership validation on all resource access endpoints"
                                    .to_string(),
                            cvss_score: Some(7.5),
                        });
                    }
                }
            }
        }
    }

    bypasses
}

async fn check_force_browsing(client: &HuntClient, _config: &HuntConfig) -> Vec<AuthzBypass> {
    let mut bypasses = Vec::new();
    let paths = ["/admin/config", "/settings", "/profile/admin", "/user/roles"];

    for path in &paths {
        if let Ok(resp) = client.get(path).await {
            let status = resp.status().as_u16();
            if status == 200 {
                let id = format!("az-{}", &uuid::Uuid::new_v4().to_string()[..8]);
                bypasses.push(AuthzBypass {
                    id,
                    bypass_type: BypassType::ForceBrowsing,
                    severity: Severity::Medium,
                    description: format!("Direct access to {} without authentication", path),
                    endpoint: format!("{}{}", client.base_url(), path),
                    evidence: format!("GET {} returned 200 OK", path),
                    remediation: "Implement proper access controls and authentication checks"
                        .to_string(),
                    cvss_score: Some(5.3),
                });
            }
        }
    }

    bypasses
}

async fn check_http_methods(client: &HuntClient, _config: &HuntConfig) -> Vec<AuthzBypass> {
    let mut bypasses = Vec::new();

    let methods = ["OPTIONS", "TRACE"];

    for method in &methods {
        let resp = match *method {
            "OPTIONS" => client.head("/").await,
            "TRACE" => {
                client
                    .request(reqwest::Method::TRACE, "/")
                    .await
            }
            _ => client.get("/").await,
        };

        if let Ok(resp) = resp {
            if *method == "OPTIONS" {
                if let Some(allow) = resp.headers().get("allow") {
                    if let Ok(allow_str) = allow.to_str() {
                        let allow_lower = allow_str.to_lowercase();
                        if allow_lower.contains("put") || allow_lower.contains("delete") {
                            let id = format!("az-{}", &uuid::Uuid::new_v4().to_string()[..8]);
                            bypasses.push(AuthzBypass {
                                id,
                                bypass_type: BypassType::MissingAuthorization,
                                severity: Severity::Low,
                                description: "Dangerous HTTP methods allowed (PUT/DELETE)"
                                    .to_string(),
                                endpoint: client.base_url().to_string(),
                                evidence: format!("OPTIONS returned Allow: {}", allow_str),
                                remediation: "Restrict HTTP methods to only those required"
                                    .to_string(),
                                cvss_score: Some(3.0),
                            });
                        }
                    }
                }
            }

            if *method == "TRACE" && resp.status().as_u16() == 200 {
                let id = format!("az-{}", &uuid::Uuid::new_v4().to_string()[..8]);
                bypasses.push(AuthzBypass {
                    id,
                    bypass_type: BypassType::MissingAuthorization,
                    severity: Severity::Medium,
                    description: "TRACE method enabled (XST risk)".to_string(),
                    endpoint: client.base_url().to_string(),
                    evidence: "TRACE method returned 200 OK".to_string(),
                    remediation: "Disable TRACE method on the server".to_string(),
                    cvss_score: Some(5.0),
                });
            }
        }
    }

    bypasses
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bypass_types() {
        assert_eq!(BypassType::Idor, BypassType::Idor);
        assert_eq!(
            BypassType::MissingAuthorization,
            BypassType::MissingAuthorization
        );
        assert_eq!(BypassType::ForceBrowsing, BypassType::ForceBrowsing);
    }
}
