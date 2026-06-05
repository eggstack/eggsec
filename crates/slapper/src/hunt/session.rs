use crate::error::Result;
use crate::hunt::{HuntClient, HuntConfig};
use crate::types::Severity;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionIssue {
    pub id: String,
    pub issue_type: SessionIssueType,
    pub severity: Severity,
    pub description: String,
    pub evidence: String,
    pub remediation: String,
    pub cvss_score: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[allow(dead_code)]
pub enum SessionIssueType {
    SessionFixation,
    SessionTimeout,
    TokenPrediction,
    InsufficientEntropy,
    MissingHttpOnly,
    MissingSecure,
    MissingSameSite,
    Csrf,
    ConcurrentSessions,
}

pub async fn check_session_security(
    client: &HuntClient,
    config: &HuntConfig,
) -> Result<Vec<SessionIssue>> {
    let mut issues = Vec::new();

    match client.get("/").await {
        Ok(resp) => {
            let headers = resp.headers().clone();

            issues.extend(check_cookie_flags(&headers, client.base_url()));
            issues.extend(check_security_headers(&headers, client.base_url()));
            issues.extend(check_session_token_entropy(&headers, client.base_url()));

            issues.extend(check_session_fixation(client, config).await);
        }
        Err(e) => {
            tracing::warn!("Failed to connect to target for session analysis: {}", e);
        }
    }

    Ok(issues)
}

fn check_cookie_flags(
    headers: &reqwest::header::HeaderMap,
    target: &str,
) -> Vec<SessionIssue> {
    let mut issues = Vec::new();
    let cookie_headers: Vec<_> = headers
        .get_all(reqwest::header::SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .collect();

    for cookie_str in cookie_headers {
        let cookie_lower = cookie_str.to_lowercase();
        let cookie_name = cookie_str.split('=').next().unwrap_or("unknown").trim();

        if !cookie_lower.contains("httponly") {
            let id = format!("ss-{}", &uuid::Uuid::new_v4().to_string()[..8]);
            issues.push(SessionIssue {
                id,
                issue_type: SessionIssueType::MissingHttpOnly,
                severity: Severity::Medium,
                description: format!("Cookie '{}' missing HttpOnly flag", cookie_name),
                evidence: format!("Set-Cookie header: {}", cookie_str),
                remediation: "Set HttpOnly flag on session cookies to prevent JavaScript access"
                    .to_string(),
                cvss_score: Some(5.0),
            });
        }

        if !cookie_lower.contains("secure") && target.starts_with("https") {
            let id = format!("ss-{}", &uuid::Uuid::new_v4().to_string()[..8]);
            issues.push(SessionIssue {
                id,
                issue_type: SessionIssueType::MissingSecure,
                severity: Severity::Medium,
                description: format!("Cookie '{}' missing Secure flag on HTTPS site", cookie_name),
                evidence: format!("Set-Cookie header: {}", cookie_str),
                remediation: "Set Secure flag on cookies for HTTPS sites to prevent HTTP transmission"
                    .to_string(),
                cvss_score: Some(5.0),
            });
        }

        if !cookie_lower.contains("samesite") {
            let id = format!("ss-{}", &uuid::Uuid::new_v4().to_string()[..8]);
            issues.push(SessionIssue {
                id,
                issue_type: SessionIssueType::MissingSameSite,
                severity: Severity::Low,
                description: format!("Cookie '{}' missing SameSite attribute", cookie_name),
                evidence: format!("Set-Cookie header: {}", cookie_str),
                remediation: "Set SameSite=Strict or SameSite=Lax on cookies to prevent CSRF"
                    .to_string(),
                cvss_score: Some(3.0),
            });
        }
    }

    issues
}

fn check_security_headers(
    headers: &reqwest::header::HeaderMap,
    _target: &str,
) -> Vec<SessionIssue> {
    let mut issues = Vec::new();

    if !headers.contains_key("x-frame-options") {
        let id = format!("ss-{}", &uuid::Uuid::new_v4().to_string()[..8]);
        issues.push(SessionIssue {
            id,
            issue_type: SessionIssueType::Csrf,
            severity: Severity::Low,
            description: "Missing X-Frame-Options header (clickjacking risk)".to_string(),
            evidence: "Response does not include X-Frame-Options header".to_string(),
            remediation: "Add X-Frame-Options: DENY or SAMEORIGIN header".to_string(),
            cvss_score: Some(3.0),
        });
    }

    if !headers.contains_key("content-security-policy") {
        let id = format!("ss-{}", &uuid::Uuid::new_v4().to_string()[..8]);
        issues.push(SessionIssue {
            id,
            issue_type: SessionIssueType::Csrf,
            severity: Severity::Low,
            description: "Missing Content-Security-Policy header".to_string(),
            evidence: "Response does not include CSP header".to_string(),
            remediation: "Implement Content-Security-Policy header".to_string(),
            cvss_score: Some(3.0),
        });
    }

    issues
}

fn check_session_token_entropy(
    headers: &reqwest::header::HeaderMap,
    _target: &str,
) -> Vec<SessionIssue> {
    let mut issues = Vec::new();

    if let Some(set_cookie) = headers.get(reqwest::header::SET_COOKIE) {
        if let Ok(cookie_str) = set_cookie.to_str() {
            let parts: Vec<&str> = cookie_str.split(';').collect();
            if let Some(name_value) = parts.first() {
                let mut name_value_iter = name_value.splitn(2, '=');
                if let (Some(name), Some(value)) =
                    (name_value_iter.next(), name_value_iter.next())
                {
                    let name = name.trim().to_lowercase();
                    let value = value.trim();

                    if (name.contains("session") || name.contains("sid") || name.contains("token"))
                        && value.len() < 16
                    {
                        let id = format!("ss-{}", &uuid::Uuid::new_v4().to_string()[..8]);
                        issues.push(SessionIssue {
                            id,
                            issue_type: SessionIssueType::InsufficientEntropy,
                            severity: Severity::High,
                            description: format!(
                                "Session token '{}' appears to have insufficient entropy ({} chars)",
                                name,
                                value.len()
                            ),
                            evidence: format!("Token length: {} chars", value.len()),
                            remediation: "Use at least 128-bit cryptographically random session tokens"
                                .to_string(),
                            cvss_score: Some(7.0),
                        });
                    }
                }
            }
        }
    }

    issues
}

async fn check_session_fixation(
    client: &HuntClient,
    _config: &HuntConfig,
) -> Vec<SessionIssue> {
    let mut issues = Vec::new();

    let request_count = 5;
    let mut all_cookies: Vec<Vec<String>> = Vec::new();

    for _ in 0..request_count {
        if let Ok(resp) = client.get("/").await {
            let cookies: Vec<String> = resp
                .headers()
                .get_all(reqwest::header::SET_COOKIE)
                .iter()
                .filter_map(|v| v.to_str().ok().map(|s| s.to_string()))
                .collect();
            all_cookies.push(cookies);
        }
    }

    if all_cookies.is_empty() || all_cookies.iter().all(|c| c.is_empty()) {
        return issues;
    }

    let first = &all_cookies[0];
    if first.is_empty() {
        return issues;
    }

    let all_identical = all_cookies.iter().all(|c| c == first);

    let has_session_cookie = first.iter().any(|cookie| {
        let lower = cookie.to_lowercase();
        lower.contains("session")
            || lower.contains("sid")
            || lower.contains("token")
            || lower.contains("auth")
            || lower.contains("jwt")
    });

    if all_identical && has_session_cookie {
        let id = format!("ss-{}", &uuid::Uuid::new_v4().to_string()[..8]);
        issues.push(SessionIssue {
            id,
            issue_type: SessionIssueType::SessionFixation,
            severity: Severity::High,
            description: "Session ID appears static across multiple requests (potential fixation)"
                .to_string(),
            evidence: format!(
                "Received identical session cookie across {} consecutive requests",
                request_count
            ),
            remediation: "Regenerate session ID after authentication and periodically"
                .to_string(),
            cvss_score: Some(6.5),
        });
    }

    issues
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_issue_types() {
        assert_eq!(
            SessionIssueType::SessionFixation,
            SessionIssueType::SessionFixation
        );
        assert_eq!(SessionIssueType::Csrf, SessionIssueType::Csrf);
        assert_eq!(
            SessionIssueType::MissingHttpOnly,
            SessionIssueType::MissingHttpOnly
        );
    }
}
