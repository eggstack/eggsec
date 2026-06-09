//! Tests for secret detection regex patterns.
//!
//! Tests the secret scanner's ability to detect API keys, tokens,
//! and other sensitive data in content.

use eggsec::recon::secrets::{scan_content, Confidence, SecretType, Severity};

#[test]
fn test_detect_aws_access_key() {
    let content = "AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE";
    let findings = scan_content(content);

    let aws: Vec<_> = findings
        .iter()
        .filter(|f| f.secret_type == SecretType::AwsAccessKey)
        .collect();
    assert!(!aws.is_empty(), "Should detect AWS access key");
    assert_eq!(aws[0].severity, Severity::Critical);
    assert_eq!(aws[0].confidence, Confidence::High);
    assert!(
        !aws[0].value_preview.is_empty(),
        "Should have value preview"
    );
}

#[test]
fn test_detect_github_token() {
    // ghp_ followed by exactly 36 alphanumeric chars
    let content = "GITHUB_TOKEN=ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdef1234";
    let findings = scan_content(content);

    let gh: Vec<_> = findings
        .iter()
        .filter(|f| f.secret_type == SecretType::GithubToken)
        .collect();
    assert!(!gh.is_empty(), "Should detect GitHub token");
    assert_eq!(gh[0].severity, Severity::Critical);
    assert!(
        gh[0].value_preview.contains("ghp_"),
        "Preview should contain token prefix"
    );
}

#[test]
fn test_detect_private_key() {
    let content =
        "-----BEGIN RSA PRIVATE KEY-----\nMIIEpAIBAAKCAQEA...\n-----END RSA PRIVATE KEY-----";
    let findings = scan_content(content);

    let pk: Vec<_> = findings
        .iter()
        .filter(|f| f.secret_type == SecretType::PrivateKey)
        .collect();
    assert!(!pk.is_empty(), "Should detect private key");
    assert_eq!(pk[0].severity, Severity::Critical);
}

#[test]
fn test_detect_slack_token() {
    let content = "SLACK_TOKEN=xoxb-123456789012-123456789012-ABCDEFGHIJKLMNOPQRSTUV";
    let findings = scan_content(content);

    let slack: Vec<_> = findings
        .iter()
        .filter(|f| f.secret_type == SecretType::SlackToken)
        .collect();
    assert!(!slack.is_empty(), "Should detect Slack token");
    assert_eq!(slack[0].severity, Severity::Critical);
    assert!(
        slack[0].value_preview.contains("xoxb"),
        "Preview should contain token prefix"
    );
}

#[test]
fn test_detect_stripe_key() {
    let content = "STRIPE_KEY=sk_live_abcdefghijklmnopqrstuvwxyz0123456789";
    let findings = scan_content(content);

    let stripe: Vec<_> = findings
        .iter()
        .filter(|f| f.secret_type == SecretType::StripeKey)
        .collect();
    assert!(!stripe.is_empty(), "Should detect Stripe key");
    assert_eq!(stripe[0].severity, Severity::Critical);
}

#[test]
fn test_detect_openai_key() {
    // sk- followed by exactly 48 alphanumeric chars
    let content = "OPENAI_KEY=sk-abcdefghijklmnopqrstuvwxyz0123456789ABCDEFGHIJkl";
    let findings = scan_content(content);

    let openai: Vec<_> = findings
        .iter()
        .filter(|f| f.secret_type == SecretType::OpenAiKey)
        .collect();
    assert!(!openai.is_empty(), "Should detect OpenAI key");
    assert_eq!(openai[0].severity, Severity::Critical);
    assert!(
        openai[0].value_preview.contains("sk-"),
        "Preview should contain key prefix"
    );
}

#[test]
fn test_detect_bearer_token() {
    let content = "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9";
    let findings = scan_content(content);

    let bearer: Vec<_> = findings
        .iter()
        .filter(|f| f.secret_type == SecretType::BearerToken)
        .collect();
    assert!(!bearer.is_empty(), "Should detect bearer token");
    assert_eq!(bearer[0].severity, Severity::High);
    assert!(
        bearer[0].value_preview.contains("Bearer"),
        "Preview should contain bearer prefix"
    );
}

#[test]
fn test_detect_database_connection_string() {
    let content = "postgres://user:p4ssw0rd@db.example.com:5432/mydb";
    let findings = scan_content(content);

    let db: Vec<_> = findings
        .iter()
        .filter(|f| f.secret_type == SecretType::DatabaseConnectionString)
        .collect();
    assert!(!db.is_empty(), "Should detect database connection string");
    assert_eq!(db[0].severity, Severity::Critical);
    assert!(
        db[0].value_preview.contains("postgres"),
        "Preview should contain scheme"
    );
}

#[test]
fn test_no_false_positives_clean_content() {
    let content = "This is a normal document about security practices. \
                   It mentions API keys and tokens but doesn't contain any. \
                   The quick brown fox jumps over the lazy dog.";
    let findings = scan_content(content);

    let high_confidence: Vec<_> = findings
        .iter()
        .filter(|f| f.confidence == Confidence::High)
        .collect();
    assert!(
        high_confidence.is_empty(),
        "Clean content should not have high-confidence findings. Got: {:?}",
        high_confidence
            .iter()
            .map(|f| &f.description)
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_empty_content() {
    let findings = scan_content("");
    assert!(findings.is_empty(), "Empty content should have no findings");
}

#[test]
fn test_multiple_secrets() {
    let content = r#"
        AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE
        GITHUB_TOKEN=ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdef1234
        SLACK_TOKEN=xoxb-123456789012-123456789012-ABCDEFGHIJKLMNOPQRSTUV
    "#;
    let findings = scan_content(content);

    let types: Vec<_> = findings.iter().map(|f| f.secret_type).collect();
    assert!(
        types.contains(&SecretType::AwsAccessKey),
        "Should find AWS key. Found: {:?}",
        types
    );
    assert!(
        types.contains(&SecretType::GithubToken),
        "Should find GitHub token. Found: {:?}",
        types
    );
    assert!(
        types.contains(&SecretType::SlackToken),
        "Should find Slack token. Found: {:?}",
        types
    );
}
