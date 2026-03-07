use serde::{Deserialize, Serialize};
use regex::Regex;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
}

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

pub struct SecretScanner {
    patterns: Vec<SecretPattern>,
}

struct SecretPattern {
    pattern: Regex,
    secret_type: SecretType,
    confidence: Confidence,
    severity: Severity,
    description: String,
}

impl SecretScanner {
    pub fn new() -> Self {
        let patterns = vec![
            SecretPattern {
                pattern: Regex::new(r"(?i)(A3T[A-Z0-9]|AKIA|AGPA|AIDA|AROA|AIPA|ANPA|ANVA|ASIA)[A-Z0-9]{16}").unwrap(),
                secret_type: SecretType::AwsAccessKey,
                confidence: Confidence::High,
                severity: Severity::Critical,
                description: "AWS Access Key ID".to_string(),
            },
            SecretPattern {
                pattern: Regex::new(r"(?i)aws(.{0,20})?(?-i)['\"][0-9a-zA-Z/+]{40}['\"]").unwrap(),
                secret_type: SecretType::AwsSecretKey,
                confidence: Confidence::High,
                severity: Severity::Critical,
                description: "AWS Secret Access Key".to_string(),
            },
            SecretPattern {
                pattern: Regex::new(r"(?i)aws_session_token[=\s]['\"]?[A-Za-z0-9+/=]+").unwrap(),
                secret_type: SecretType::AwsSessionToken,
                confidence: Confidence::High,
                severity: Severity::Critical,
                description: "AWS Session Token".to_string(),
            },
            SecretPattern {
                pattern: Regex::new(r"(?i)github(.{0,20})?['\"][0-9a-zA-Z]{35,40}['\"]").unwrap(),
                secret_type: SecretType::GithubToken,
                confidence: Confidence::High,
                severity: Severity::Critical,
                description: "GitHub Personal Access Token".to_string(),
            },
            SecretPattern {
                pattern: Regex::new(r"(?i)ghp_[0-9a-zA-Z]{36}").unwrap(),
                secret_type: SecretType::GithubToken,
                confidence: Confidence::High,
                severity: Severity::Critical,
                description: "GitHub OAuth Token".to_string(),
            },
            SecretPattern {
                pattern: Regex::new(r"(?i)glpat-[0-9a-zA-Z\-_]{20,}").unwrap(),
                secret_type: SecretType::GitlabToken,
                confidence: Confidence::High,
                severity: Severity::Critical,
                description: "GitLab Personal Access Token".to_string(),
            },
            SecretPattern {
                pattern: Regex::new(r"(?i)xox[baprs]-[0-9a-zA-Z]{10,48}").unwrap(),
                secret_type: SecretType::SlackToken,
                confidence: Confidence::High,
                severity: Severity::Critical,
                description: "Slack Token".to_string(),
            },
            SecretPattern {
                pattern: Regex::new(r"https://hooks\.slack\.com/services/T[a-zA-Z0-9_]+/B[a-zA-Z0-9_]+/[a-zA-Z0-9_]+").unwrap(),
                secret_type: SecretType::SlackWebhook,
                confidence: Confidence::High,
                severity: Severity::High,
                description: "Slack Webhook URL".to_string(),
            },
            SecretPattern {
                pattern: Regex::new(r"(?i)sk-[0-9a-zA-Z]{48}").unwrap(),
                secret_type: SecretType::OpenAiKey,
                confidence: Confidence::High,
                severity: Severity::Critical,
                description: "OpenAI API Key".to_string(),
            },
            SecretPattern {
                pattern: Regex::new(r"(?i)sk_live_[0-9a-zA-Z]{24,}").unwrap(),
                secret_type: SecretType::StripeKey,
                confidence: Confidence::High,
                severity: Severity::Critical,
                description: "Stripe Live API Key".to_string(),
            },
            SecretPattern {
                pattern: Regex::new(r"sk_live_[0-9a-zA-Z]{24,}").unwrap(),
                secret_type: SecretType::StripeKey,
                confidence: Confidence::High,
                severity: Severity::Critical,
                description: "Stripe Secret Key".to_string(),
            },
            SecretPattern {
                pattern: Regex::new(r"(?i)AIza[0-9A-Za-z\\-_]{35}").unwrap(),
                secret_type: SecretType::GcpApiKey,
                confidence: Confidence::High,
                severity: Severity::Critical,
                description: "Google API Key".to_string(),
            },
            SecretPattern {
                pattern: Regex::new(r"-----BEGIN (RSA |EC |DSA |OPENSSH )?PRIVATE KEY-----").unwrap(),
                secret_type: SecretType::PrivateKey,
                confidence: Confidence::High,
                severity: Severity::Critical,
                description: "Private Key".to_string(),
            },
            SecretPattern {
                pattern: Regex::new(r"(?i)bearer\s+[a-zA-Z0-9\-_\.=]+").unwrap(),
                secret_type: SecretType::BearerToken,
                confidence: Confidence::Medium,
                severity: Severity::High,
                description: "Bearer Token".to_string(),
            },
            SecretPattern {
                pattern: Regex::new(r"(?i)basic\s+[a-zA-Z0-9+/=]+").unwrap(),
                secret_type: SecretType::BasicAuth,
                confidence: Confidence::Medium,
                severity: Severity::High,
                description: "Basic Authentication Header".to_string(),
            },
            SecretPattern {
                pattern: Regex::new(r"(?i)connection_string[=\s]['\"]?Server=[^;]+;Database=[^;]+;(User Id=|UID=|User=|Password=|PWD=)[^;]+").unwrap(),
                secret_type: SecretType::DatabaseConnectionString,
                confidence: Confidence::High,
                severity: Severity::Critical,
                description: "Database Connection String".to_string(),
            },
            SecretPattern {
                pattern: Regex::new(r"mongodb(\+srv)?://[^:]+:[^@]+@").unwrap(),
                secret_type: SecretType::DatabaseConnectionString,
                confidence: Confidence::High,
                severity: Severity::Critical,
                description: "MongoDB Connection String".to_string(),
            },
            SecretPattern {
                pattern: Regex::new(r"postgres(ql)?://[^:]+:[^@]+@").unwrap(),
                secret_type: SecretType::DatabaseConnectionString,
                confidence: Confidence::High,
                severity: Severity::Critical,
                description: "PostgreSQL Connection String".to_string(),
            },
            SecretPattern {
                pattern: Regex::new(r"mysql://[^:]+:[^@]+@").unwrap(),
                secret_type: SecretType::DatabaseConnectionString,
                confidence: Confidence::High,
                severity: Severity::Critical,
                description: "MySQL Connection String".to_string(),
            },
            SecretPattern {
                pattern: Regex::new(r"(?i)apikey[=\s]['\"]?[a-zA-Z0-9]{20,}['\"]?").unwrap(),
                secret_type: SecretType::GenericApiKey,
                confidence: Confidence::Low,
                severity: Severity::Medium,
                description: "Generic API Key".to_string(),
            },
            SecretPattern {
                pattern: Regex::new(r"(?i)password[=\s]['\"]?[^\s'\"]{8,}['\"]?").unwrap(),
                secret_type: SecretType::PasswordInUrl,
                confidence: Confidence::Medium,
                severity: Severity::High,
                description: "Password in configuration".to_string(),
            },
            SecretPattern {
                pattern: Regex::new(r"[a-zA-Z0-9_-]*:[a-zA-Z0-9_-]+@github\.com").unwrap(),
                secret_type: SecretType::BasicAuth,
                confidence: Confidence::High,
                severity: Severity::Critical,
                description: "GitHub credentials in URL".to_string(),
            },
            SecretPattern {
                pattern: Regex::new(r"xox[baprs]-[0-9]{10,12}-[0-9]{10,12}[a-zA-Z0-9-]*").unwrap(),
                secret_type: SecretType::DiscordToken,
                confidence: Confidence::High,
                severity: Severity::Critical,
                description: "Discord Bot Token".to_string(),
            },
            SecretPattern {
                pattern: Regex::new(r"SK[0-9a-fA-F]{32}").unwrap(),
                secret_type: SecretType::TwilioKey,
                confidence: Confidence::High,
                severity: Severity::Critical,
                description: "Twilio API Key".to_string(),
            },
            SecretPattern {
                pattern: Regex::new(r"SG\.[0-9A-Za-z\-_]{22}\.[0-9A-Za-z\-_]{43}").unwrap(),
                secret_type: SecretType::SendGridKey,
                confidence: Confidence::High,
                severity: Severity::Critical,
                description: "SendGrid API Key".to_string(),
            },
            SecretPattern {
                pattern: Regex::new(r"key-[0-9a-zA-Z]{32}").unwrap(),
                secret_type: SecretType::MailchimpKey,
                confidence: Confidence::High,
                severity: Severity::Critical,
                description: "Mailchimp API Key".to_string(),
            },
        ];

        Self { patterns }
    }

    pub fn scan(&self, content: &str) -> Vec<SecretFinding> {
        let mut findings = Vec::new();

        for pattern in &self.patterns {
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
                    description: pattern.description.clone(),
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
