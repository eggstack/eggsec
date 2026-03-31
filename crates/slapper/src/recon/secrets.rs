use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretFinding {
    pub secret_type: SecretType,
    pub value_preview: String,
    pub location: String,
    pub confidence: Confidence,
    pub severity: Severity,
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecretType {
    AwsAccessKey,
    AwsSecretKey,
    AwsSessionToken,
    AzureKey,
    GcpApiKey,
    GcpServiceAccount,
    GithubToken,
    GitlabToken,
    BitbucketToken,
    SlackToken,
    DiscordToken,
    SlackWebhook,
    GenericApiKey,
    PrivateKey,
    JwtToken,
    BasicAuth,
    BearerToken,
    OpenAiKey,
    StripeKey,
    TwilioKey,
    SendGridKey,
    MailchimpKey,
    PasswordInUrl,
    DatabaseConnectionString,
    NpmToken,
    PyPiToken,
    HerokuKey,
    NetlifyToken,
    DockerhubToken,
    KubernetesSecret,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Confidence {
    High,
    Medium,
    Low,
}

pub use crate::types::Severity;

impl std::fmt::Display for SecretType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecretType::AwsAccessKey => write!(f, "AWS Access Key"),
            SecretType::AwsSecretKey => write!(f, "AWS Secret Key"),
            SecretType::AwsSessionToken => write!(f, "AWS Session Token"),
            SecretType::AzureKey => write!(f, "Azure Key"),
            SecretType::GcpApiKey => write!(f, "GCP API Key"),
            SecretType::GcpServiceAccount => write!(f, "GCP Service Account"),
            SecretType::GithubToken => write!(f, "GitHub Token"),
            SecretType::GitlabToken => write!(f, "GitLab Token"),
            SecretType::BitbucketToken => write!(f, "Bitbucket Token"),
            SecretType::SlackToken => write!(f, "Slack Token"),
            SecretType::DiscordToken => write!(f, "Discord Token"),
            SecretType::SlackWebhook => write!(f, "Slack Webhook"),
            SecretType::GenericApiKey => write!(f, "Generic API Key"),
            SecretType::PrivateKey => write!(f, "Private Key"),
            SecretType::JwtToken => write!(f, "JWT Token"),
            SecretType::BasicAuth => write!(f, "Basic Authentication"),
            SecretType::BearerToken => write!(f, "Bearer Token"),
            SecretType::OpenAiKey => write!(f, "OpenAI API Key"),
            SecretType::StripeKey => write!(f, "Stripe API Key"),
            SecretType::TwilioKey => write!(f, "Twilio Key"),
            SecretType::SendGridKey => write!(f, "SendGrid Key"),
            SecretType::MailchimpKey => write!(f, "Mailchimp Key"),
            SecretType::PasswordInUrl => write!(f, "Password in URL"),
            SecretType::DatabaseConnectionString => write!(f, "Database Connection String"),
            SecretType::NpmToken => write!(f, "NPM Token"),
            SecretType::PyPiToken => write!(f, "PyPI Token"),
            SecretType::HerokuKey => write!(f, "Heroku API Key"),
            SecretType::NetlifyToken => write!(f, "Netlify Token"),
            SecretType::DockerhubToken => write!(f, "Docker Hub Token"),
            SecretType::KubernetesSecret => write!(f, "Kubernetes Secret"),
        }
    }
}

struct SecretPattern {
    pattern: Regex,
    secret_type: SecretType,
    confidence: Confidence,
    severity: Severity,
    description: &'static str,
}

fn build_patterns() -> Vec<SecretPattern> {
    // All regex literals are compile-time validated. If any pattern is invalid,
    // this function will panic at first access (once via Lazy), making bugs
    // immediately visible rather than silently skipping matches.
    vec![
        SecretPattern {
            pattern: Regex::new(r#"(?i)(A3T[A-Z0-9]|AKIA|AGPA|AIDA|AROA|AIPA|ANPA|ANVA|ASIA)[A-Z0-9]{16}"#)
                .expect("invalid AWS access key regex"),
            secret_type: SecretType::AwsAccessKey,
            confidence: Confidence::High,
            severity: Severity::Critical,
            description: "AWS Access Key ID",
        },
        SecretPattern {
            pattern: Regex::new(r#"(?i)aws(.{0,20})?(?-i)['\"][0-9a-zA-Z/+]{40}['\"]"#)
                .expect("invalid AWS secret key regex"),
            secret_type: SecretType::AwsSecretKey,
            confidence: Confidence::High,
            severity: Severity::Critical,
            description: "AWS Secret Access Key",
        },
        SecretPattern {
            pattern: Regex::new(r#"(?i)aws_session_token[=\s]["']?[A-Za-z0-9+/=]+"#)
                .expect("invalid AWS session token regex"),
            secret_type: SecretType::AwsSessionToken,
            confidence: Confidence::High,
            severity: Severity::Critical,
            description: "AWS Session Token",
        },
        SecretPattern {
            pattern: Regex::new(r#"(?i)github(.{0,20})?['\"][0-9a-zA-Z]{35,40}['\"]"#)
                .expect("invalid GitHub PAT regex"),
            secret_type: SecretType::GithubToken,
            confidence: Confidence::High,
            severity: Severity::Critical,
            description: "GitHub Personal Access Token",
        },
        SecretPattern {
            pattern: Regex::new(r#"(?i)ghp_[0-9a-zA-Z]{36}"#)
                .expect("invalid GitHub OAuth regex"),
            secret_type: SecretType::GithubToken,
            confidence: Confidence::High,
            severity: Severity::Critical,
            description: "GitHub OAuth Token",
        },
        SecretPattern {
            pattern: Regex::new(r#"(?i)glpat-[0-9a-zA-Z\-_]{20,}"#)
                .expect("invalid GitLab PAT regex"),
            secret_type: SecretType::GitlabToken,
            confidence: Confidence::High,
            severity: Severity::Critical,
            description: "GitLab Personal Access Token",
        },
        SecretPattern {
            pattern: Regex::new(r#"(?i)xox[baprs]-[0-9a-zA-Z]{10,48}"#)
                .expect("invalid Slack token regex"),
            secret_type: SecretType::SlackToken,
            confidence: Confidence::High,
            severity: Severity::Critical,
            description: "Slack Token",
        },
        SecretPattern {
            pattern: Regex::new(r#"https://hooks\.slack\.com/services/T[a-zA-Z0-9_]+/B[a-zA-Z0-9_]+/[a-zA-Z0-9_]+"#)
                .expect("invalid Slack webhook regex"),
            secret_type: SecretType::SlackWebhook,
            confidence: Confidence::High,
            severity: Severity::High,
            description: "Slack Webhook URL",
        },
        SecretPattern {
            pattern: Regex::new(r#"(?i)sk-[0-9a-zA-Z]{48}"#)
                .expect("invalid OpenAI key regex"),
            secret_type: SecretType::OpenAiKey,
            confidence: Confidence::High,
            severity: Severity::Critical,
            description: "OpenAI API Key",
        },
        SecretPattern {
            pattern: Regex::new(r#"(?i)sk_live_[0-9a-zA-Z]{24,}"#)
                .expect("invalid Stripe key regex"),
            secret_type: SecretType::StripeKey,
            confidence: Confidence::High,
            severity: Severity::Critical,
            description: "Stripe API Key",
        },
        SecretPattern {
            pattern: Regex::new(r#"(?i)AIza[0-9A-Za-z_-]{35}"#)
                .expect("invalid GCP API key regex"),
            secret_type: SecretType::GcpApiKey,
            confidence: Confidence::High,
            severity: Severity::Critical,
            description: "Google API Key",
        },
        SecretPattern {
            pattern: Regex::new(r#"-----BEGIN (RSA |EC |DSA |OPENSSH )?PRIVATE KEY-----"#)
                .expect("invalid private key regex"),
            secret_type: SecretType::PrivateKey,
            confidence: Confidence::High,
            severity: Severity::Critical,
            description: "Private Key",
        },
        SecretPattern {
            pattern: Regex::new(r#"(?i)bearer\s+[a-zA-Z0-9\-_\.=]+"#)
                .expect("invalid bearer token regex"),
            secret_type: SecretType::BearerToken,
            confidence: Confidence::Medium,
            severity: Severity::High,
            description: "Bearer Token",
        },
        SecretPattern {
            pattern: Regex::new(r#"(?i)basic\s+[a-zA-Z0-9+/=]+"#)
                .expect("invalid basic auth regex"),
            secret_type: SecretType::BasicAuth,
            confidence: Confidence::Medium,
            severity: Severity::High,
            description: "Basic Authentication Header",
        },
        SecretPattern {
            pattern: Regex::new(r#"(?i)connection_string[=\s]["']?Server=[^;]+;Database=[^;]+;(User Id=|UID=|User=|Password=|PWD=)[^;]+"#)
                .expect("invalid connection string regex"),
            secret_type: SecretType::DatabaseConnectionString,
            confidence: Confidence::High,
            severity: Severity::Critical,
            description: "Database Connection String",
        },
        SecretPattern {
            pattern: Regex::new(r#"mongodb(\+srv)?://[^:]+:[^@]+@"#)
                .expect("invalid MongoDB URI regex"),
            secret_type: SecretType::DatabaseConnectionString,
            confidence: Confidence::High,
            severity: Severity::Critical,
            description: "MongoDB Connection String",
        },
        SecretPattern {
            pattern: Regex::new(r#"postgres(ql)?://[^:]+:[^@]+@"#)
                .expect("invalid PostgreSQL URI regex"),
            secret_type: SecretType::DatabaseConnectionString,
            confidence: Confidence::High,
            severity: Severity::Critical,
            description: "PostgreSQL Connection String",
        },
        SecretPattern {
            pattern: Regex::new(r#"mysql://[^:]+:[^@]+@"#)
                .expect("invalid MySQL URI regex"),
            secret_type: SecretType::DatabaseConnectionString,
            confidence: Confidence::High,
            severity: Severity::Critical,
            description: "MySQL Connection String",
        },
        SecretPattern {
            pattern: Regex::new(r#"(?i)apikey[=\s]["']?[a-zA-Z0-9]{20,}["']?"#)
                .expect("invalid generic API key regex"),
            secret_type: SecretType::GenericApiKey,
            confidence: Confidence::Low,
            severity: Severity::Medium,
            description: "Generic API Key",
        },
        SecretPattern {
            pattern: Regex::new(r#"(?i)password[=\s]["']?[^\s"']{8,}["']?"#)
                .expect("invalid password regex"),
            secret_type: SecretType::PasswordInUrl,
            confidence: Confidence::Medium,
            severity: Severity::High,
            description: "Password in configuration",
        },
        SecretPattern {
            pattern: Regex::new(r#"[a-zA-Z0-9_-]*:[a-zA-Z0-9_-]+@github\.com"#)
                .expect("invalid GitHub credentials regex"),
            secret_type: SecretType::BasicAuth,
            confidence: Confidence::High,
            severity: Severity::Critical,
            description: "GitHub credentials in URL",
        },
        SecretPattern {
            pattern: Regex::new(r#"xox[baprs]-[0-9]{10,12}-[0-9]{10,12}[a-zA-Z0-9-]*"#)
                .expect("invalid Discord token regex"),
            secret_type: SecretType::DiscordToken,
            confidence: Confidence::High,
            severity: Severity::Critical,
            description: "Discord Bot Token",
        },
        SecretPattern {
            pattern: Regex::new(r#"SK[0-9a-fA-F]{32}"#)
                .expect("invalid Twilio key regex"),
            secret_type: SecretType::TwilioKey,
            confidence: Confidence::High,
            severity: Severity::Critical,
            description: "Twilio API Key",
        },
        SecretPattern {
            pattern: Regex::new(r#"SG\.[0-9A-Za-z\-_]{22}\.[0-9A-Za-z\-_]{43}"#)
                .expect("invalid SendGrid key regex"),
            secret_type: SecretType::SendGridKey,
            confidence: Confidence::High,
            severity: Severity::Critical,
            description: "SendGrid API Key",
        },
        SecretPattern {
            pattern: Regex::new(r#"key-[0-9a-zA-Z]{32}"#)
                .expect("invalid Mailchimp key regex"),
            secret_type: SecretType::MailchimpKey,
            confidence: Confidence::High,
            severity: Severity::Critical,
            description: "Mailchimp API Key",
        },
    ]
}

static PATTERNS: LazyLock<Vec<SecretPattern>> = LazyLock::new(build_patterns);

pub struct SecretScanner;

impl SecretScanner {
    pub fn new() -> Self {
        // Force lazy initialization so any pattern errors surface immediately
        LazyLock::force(&PATTERNS);
        Self
    }

    pub fn scan(&self, content: &str) -> Vec<SecretFinding> {
        let mut findings = Vec::new();

        for pattern in PATTERNS.iter() {
            for mat in pattern.pattern.find_iter(content) {
                let value = mat.as_str();
                let preview = if value.len() > 20 {
                    format!("{}...", &value[..20])
                } else {
                    value.to_string()
                };

                findings.push(SecretFinding {
                    secret_type: pattern.secret_type,
                    value_preview: preview,
                    location: format!("position {}", mat.start()),
                    confidence: pattern.confidence,
                    severity: pattern.severity,
                    description: pattern.description.to_string(),
                });
            }
        }

        findings
    }

    pub fn scan_file(&self, path: &str) -> Result<Vec<SecretFinding>, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        Ok(self.scan(&content))
    }
}

pub fn scan_content(content: &str) -> Vec<SecretFinding> {
    let scanner = SecretScanner::new();
    scanner.scan(content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scanner_creation() {
        let scanner = SecretScanner::new();
        let findings = scanner.scan("no secrets here");
        assert!(findings.is_empty());
    }

    #[test]
    fn test_aws_access_key_detected() {
        let content = "key = AKIAIOSFODNN7EXAMPLE";
        let findings = scan_content(content);
        assert!(!findings.is_empty());
        assert_eq!(findings[0].secret_type, SecretType::AwsAccessKey);
        assert_eq!(findings[0].severity, Severity::Critical);
    }

    #[test]
    fn test_github_token_detected() {
        let content = "token = ghp_1234567890abcdefghijklmnopqrstuvwxyz1234";
        let findings = scan_content(content);
        assert!(!findings.is_empty());
        assert_eq!(findings[0].secret_type, SecretType::GithubToken);
    }

    #[test]
    fn test_private_key_detected() {
        let content = "-----BEGIN RSA PRIVATE KEY-----\nMIIEpAIBAAKC...";
        let findings = scan_content(content);
        assert!(!findings.is_empty());
        assert_eq!(findings[0].secret_type, SecretType::PrivateKey);
    }

    #[test]
    fn test_bearer_token_detected() {
        let content = "Authorization: Bearer eyJhbGciOiJIUzI1NiJ9.payload.sig";
        let findings = scan_content(content);
        assert!(!findings.is_empty());
        assert_eq!(findings[0].secret_type, SecretType::BearerToken);
    }

    #[test]
    fn test_slack_webhook_detected() {
        let content = "webhook = https://hooks.slack.com/services/T0123/B456/abcDEF";
        let findings = scan_content(content);
        assert!(!findings.is_empty());
        assert_eq!(findings[0].secret_type, SecretType::SlackWebhook);
    }

    #[test]
    fn test_multiple_secrets_detected() {
        let content = r#"
            AWS_KEY=AKIAIOSFODNN7EXAMPLE
            github_token=ghp_1234567890abcdefghijklmnopqrstuvwxyz1234
            Authorization: Bearer eyJhbGciOiJIUzI1NiJ9
        "#;
        let findings = scan_content(content);
        assert!(findings.len() >= 2);
    }

    #[test]
    fn test_mongodb_uri_detected() {
        let content = "MONGO_URI=mongodb://admin:password123@localhost:27017/db";
        let findings = scan_content(content);
        assert!(!findings.is_empty());
        assert_eq!(
            findings[0].secret_type,
            SecretType::DatabaseConnectionString
        );
    }

    #[test]
    fn test_value_preview_truncation() {
        let long_key = format!("AKIAIOSFODNN7{}", "X".repeat(100));
        let content = format!("key = {}", long_key);
        let findings = scan_content(&content);
        assert!(!findings.is_empty());
        assert!(findings[0].value_preview.len() <= 23); // 20 chars + "..."
    }

    #[test]
    fn test_sendgrid_key_detected() {
        let content =
            "api_key = SG.1234567890123456789012.123456789012345678901234567890123456789012345";
        let findings = scan_content(content);
        assert!(!findings.is_empty());
        assert_eq!(findings[0].secret_type, SecretType::SendGridKey);
    }
}
