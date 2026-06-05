use crate::error::Result;
use crate::hunt::{HuntClient, HuntConfig};
use crate::types::Severity;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusinessLogicFlaw {
    pub id: String,
    pub flaw_type: FlawType,
    pub severity: Severity,
    pub description: String,
    pub location: String,
    pub evidence: String,
    pub remediation: String,
    pub cvss_score: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[allow(dead_code)]
pub enum FlawType {
    PriceManipulation,
    PrivilegeEscalation,
    RateLimitBypass,
    CartManipulation,
    CreditOverflow,
    WorkflowBypass,
    InsufficientValidation,
    TrustBoundaryViolation,
    TimeTravel,
    IntegerOverflow,
}

const API_PATHS: &[&str] = &[
    "/api",
    "/api/v1",
    "/api/v2",
    "/graphql",
    "/api/users",
    "/api/products",
    "/api/orders",
    "/api/cart",
    "/api/checkout",
    "/api/payment",
    "/api/config",
    "/api/settings",
    "/api/health",
    "/api/status",
    "/api/info",
];

const SENSITIVE_PATHS: &[&str] = &[
    "/.env",
    "/.git/config",
    "/.git/HEAD",
    "/config.json",
    "/config.yml",
    "/config.yaml",
    "/database.yml",
    "/wp-config.php",
    "/.htaccess",
    "/web.config",
    "/appsettings.json",
    "/application.properties",
    "/settings.py",
    "/config.py",
    "/.aws/credentials",
    "/credentials.json",
    "/secrets.json",
    "/private_key.pem",
    "/id_rsa",
    "/.ssh/authorized_keys",
    "/backup",
    "/backups",
    "/dump",
    "/export",
    "/api/debug",
    "/debug/vars",
    "/debug/pprof",
    "/actuator",
    "/actuator/env",
    "/actuator/heapdump",
    "/metrics",
    "/prometheus",
];

#[tracing::instrument(skip(client, config), fields(target = %client.base_url()))]
pub async fn check_business_logic(
    client: &HuntClient,
    config: &HuntConfig,
) -> Result<Vec<BusinessLogicFlaw>> {
    tracing::info!("Checking business logic");
    let mut flaws = Vec::new();

    flaws.extend(check_api_discovery(client, config).await);
    flaws.extend(check_sensitive_files(client, config).await);
    flaws.extend(check_error_handling(client, config).await);
    flaws.extend(check_rate_limiting(client, config).await);

    Ok(flaws)
}

async fn check_api_discovery(client: &HuntClient, _config: &HuntConfig) -> Vec<BusinessLogicFlaw> {
    let mut flaws = Vec::new();

    for path in API_PATHS {
        if let Ok(resp) = client.get(path).await {
            let status = resp.status().as_u16();
            if let Ok(body) = resp.text().await {
                let body_lower = body.to_lowercase();

                if status == 200 && body.len() > 100 {
                    let has_api_info = body_lower.contains("api")
                        || body_lower.contains("version")
                        || body_lower.contains("endpoint")
                        || body_lower.contains("documentation")
                        || body_lower.contains("swagger");

                    if has_api_info {
                        let id = format!("bl-{}", &uuid::Uuid::new_v4().to_string()[..8]);
                        flaws.push(BusinessLogicFlaw {
                            id,
                            flaw_type: FlawType::InsufficientValidation,
                            severity: Severity::Low,
                            description: format!("API documentation/info exposed at {}", path),
                            location: format!("{}{}", client.base_url(), path),
                            evidence: format!(
                                "GET {} returned 200 with {} bytes containing API documentation",
                                path,
                                body.len()
                            ),
                            remediation: "Disable API documentation in production environments"
                                .to_string(),
                            cvss_score: Some(3.0),
                        });
                    }
                }

                if status == 401 || status == 403 {
                    let id = format!("bl-{}", &uuid::Uuid::new_v4().to_string()[..8]);
                    flaws.push(BusinessLogicFlaw {
                        id,
                        flaw_type: FlawType::PrivilegeEscalation,
                        severity: Severity::Info,
                        description: format!("Authentication required at {}", path),
                        location: format!("{}{}", client.base_url(), path),
                        evidence: format!("GET {} returned {} (auth required)", path, status),
                        remediation: "Ensure proper authentication is implemented".to_string(),
                        cvss_score: None,
                    });
                }
            }
        }
    }

    flaws
}

async fn check_sensitive_files(
    client: &HuntClient,
    config: &HuntConfig,
) -> Vec<BusinessLogicFlaw> {
    let mut flaws = Vec::new();
    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(config.concurrency));
    let mut handles = Vec::new();
    let timeout = std::time::Duration::from_millis(config.timeout_ms);

    for path in SENSITIVE_PATHS {
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
            let result = tokio::time::timeout(timeout, client.get(&path)).await;
            let resp = match result {
                Ok(r) => r,
                Err(_) => Err(crate::error::SlapperError::Http("Request timed out".to_string())),
            };
            (path, resp)
        }));
    }

    for handle in handles {
        if let Ok((path, Ok(resp))) = handle.await {
            let status = resp.status().as_u16();
            if status == 200 {
                if let Ok(body) = resp.text().await {
                    if !body.is_empty() && body.len() > 10 {
                        let (severity, flaw_type) = classify_sensitive_file(&path, &body);

                        let id = format!("bl-{}", &uuid::Uuid::new_v4().to_string()[..8]);
                        flaws.push(BusinessLogicFlaw {
                            id,
                            flaw_type,
                            severity,
                            description: format!("Sensitive file accessible: {}", path),
                            location: format!("{}{}", client.base_url(), path),
                            evidence: format!(
                                "GET {} returned 200 with {} bytes",
                                path,
                                body.len()
                            ),
                            remediation: "Restrict access to sensitive files and configuration"
                                .to_string(),
                            cvss_score: severity_cvss(severity),
                        });
                    }
                }
            }
        }
    }

    flaws
}

fn classify_sensitive_file(path: &str, body: &str) -> (Severity, FlawType) {
    let path_lower = path.to_lowercase();
    let body_lower = body.to_lowercase();

    if path_lower.contains(".env")
        || path_lower.contains("credentials")
        || path_lower.contains("private_key")
        || path_lower.contains("id_rsa")
        || body_lower.contains("password")
        || body_lower.contains("secret")
        || body_lower.contains("api_key")
    {
        (Severity::Critical, FlawType::TrustBoundaryViolation)
    } else if path_lower.contains(".git")
        || path_lower.contains("config")
        || path_lower.contains("backup")
        || path_lower.contains("dump")
    {
        (Severity::High, FlawType::TrustBoundaryViolation)
    } else {
        (Severity::Medium, FlawType::TrustBoundaryViolation)
    }
}

fn severity_cvss(severity: Severity) -> Option<f32> {
    match severity {
        Severity::Critical => Some(9.0),
        Severity::High => Some(7.5),
        Severity::Medium => Some(5.0),
        Severity::Low => Some(3.0),
        Severity::Info => None,
    }
}

async fn check_error_handling(client: &HuntClient, _config: &HuntConfig) -> Vec<BusinessLogicFlaw> {
    let mut flaws = Vec::new();

    let test_paths = [
        "/api/test%00",
        "/api/test%0d%0a",
        "/api/test'or'1'='1",
        "/api/test<script>alert(1)</script>",
        "/api/../../../etc/passwd",
        "/api/test%ff%ff",
    ];

    for path in &test_paths {
        if let Ok(resp) = client.get(path).await {
            let status = resp.status().as_u16();
            if let Ok(body) = resp.text().await {
                let body_lower = body.to_lowercase();

                if body_lower.contains("stack trace")
                    || body_lower.contains("exception")
                    || body_lower.contains("error in")
                    || body_lower.contains("traceback")
                    || body_lower.contains("syntax error")
                    || body_lower.contains("undefined")
                    || body_lower.contains("null pointer")
                {
                    let id = format!("bl-{}", &uuid::Uuid::new_v4().to_string()[..8]);
                    flaws.push(BusinessLogicFlaw {
                        id,
                        flaw_type: FlawType::InsufficientValidation,
                        severity: Severity::Medium,
                        description: "Verbose error messages expose internal details".to_string(),
                        location: format!("{}{}", client.base_url(), path),
                        evidence: format!(
                            "Error response at {}: {}...",
                            path,
                            &body[..200.min(body.len())]
                        ),
                        remediation: "Implement custom error pages; disable debug mode in production"
                            .to_string(),
                        cvss_score: Some(5.0),
                    });
                }

                if status == 200 && body_lower.contains("root:") {
                    let id = format!("bl-{}", &uuid::Uuid::new_v4().to_string()[..8]);
                    flaws.push(BusinessLogicFlaw {
                        id,
                        flaw_type: FlawType::TrustBoundaryViolation,
                        severity: Severity::Critical,
                        description: "Path traversal allows reading system files".to_string(),
                        location: format!("{}{}", client.base_url(), path),
                        evidence: format!("GET {} returned /etc/passwd content", path),
                        remediation: "Validate and sanitize file path inputs".to_string(),
                        cvss_score: Some(9.0),
                    });
                }
            }
        }
    }

    flaws
}

async fn check_rate_limiting(client: &HuntClient, _config: &HuntConfig) -> Vec<BusinessLogicFlaw> {
    let mut flaws = Vec::new();

    // Find a valid path to test rate limiting against
    let test_path = if let Ok(resp) = client.get("/api").await {
        if resp.status().is_success() {
            "/api"
        } else {
            "/"
        }
    } else {
        "/"
    };

    let mut status_429_count = 0;
    let mut total_requests = 0;

    for _ in 0..20 {
        if let Ok(resp) = client.get(test_path).await {
            total_requests += 1;
            if resp.status().as_u16() == 429 {
                status_429_count += 1;
            }
        }
    }

    if status_429_count == 0 && total_requests >= 10 {
        let id = format!("bl-{}", &uuid::Uuid::new_v4().to_string()[..8]);
        flaws.push(BusinessLogicFlaw {
            id,
            flaw_type: FlawType::RateLimitBypass,
            severity: Severity::Medium,
            description: "No rate limiting detected after rapid requests".to_string(),
            location: client.base_url().to_string(),
            evidence: format!(
                "Sent {} requests without receiving a single 429 response",
                total_requests
            ),
            remediation: "Implement rate limiting to prevent brute force and abuse".to_string(),
            cvss_score: Some(5.0),
        });
    }

    flaws
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flaw_creation() {
        let flaw = BusinessLogicFlaw {
            id: "test-456".to_string(),
            flaw_type: FlawType::PriceManipulation,
            severity: Severity::Critical,
            description: "Test".to_string(),
            location: "test".to_string(),
            evidence: "test".to_string(),
            remediation: "test".to_string(),
            cvss_score: Some(8.0),
        };

        assert_eq!(flaw.flaw_type, FlawType::PriceManipulation);
    }
}
