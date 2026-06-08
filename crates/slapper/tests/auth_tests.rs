mod common;

use common::create_test_server;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn test_session_tester_fixation_detected() {
    let server = create_test_server().await;
    let cookie = "session=abc123; HttpOnly; Secure; SameSite=Strict";

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Set-Cookie", cookie)
                .set_body_string("OK"),
        )
        .mount(&server)
        .await;

    let tester = slapper::auth::SessionTester::new(5).unwrap();
    let result = tester.test(&server.uri()).await.unwrap();

    assert!(result.session_fixation_possible);
    assert!(result.findings.iter().any(|f| f.contains("reused")));
}

#[tokio::test]
async fn test_session_tester_cookie_issues() {
    let server = create_test_server().await;

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Set-Cookie", "session=abc123")
                .set_body_string("OK"),
        )
        .mount(&server)
        .await;

    let tester = slapper::auth::SessionTester::new(5).unwrap();
    let result = tester.test(&server.uri()).await.unwrap();

    assert!(!result.session_cookie_issues.is_empty());
    let issues = result.session_cookie_issues.join(", ");
    assert!(issues.contains("HttpOnly"));
    assert!(issues.contains("Secure"));
    assert!(issues.contains("SameSite"));
}

#[tokio::test]
async fn test_session_tester_good_cookies() {
    let server = create_test_server().await;

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header(
                    "Set-Cookie",
                    "session=abc123; HttpOnly; Secure; SameSite=Strict",
                )
                .set_body_string("OK"),
        )
        .mount(&server)
        .await;

    let tester = slapper::auth::SessionTester::new(5).unwrap();
    let result = tester.test(&server.uri()).await.unwrap();

    assert!(result.session_cookie_issues.is_empty());
}

#[tokio::test]
async fn test_mfa_tester_no_mfa() {
    let server = create_test_server().await;

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_string("Welcome back!"))
        .mount(&server)
        .await;

    let tester = slapper::auth::MfaTester::new(5).unwrap();
    let result = tester.test(&server.uri()).await.unwrap();

    assert!(!result.mfa_enabled);
    assert!(!result.mfa_bypass_possible);
    assert!(result.bypass_methods.is_empty());
}

#[tokio::test]
async fn test_mfa_tester_mfa_detected() {
    let server = create_test_server().await;

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string("Please enter your TOTP verification code"),
        )
        .mount(&server)
        .await;

    let tester = slapper::auth::MfaTester::new(5).unwrap();
    let result = tester.test(&server.uri()).await.unwrap();

    assert!(result.mfa_enabled);
    assert!(result.findings.iter().any(|f| f.contains("MFA")));
}

#[tokio::test]
async fn test_mfa_tester_bypass_weak_code() {
    let server = create_test_server().await;

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string("Please enter your verification code"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(
            ResponseTemplate::new(302)
                .append_header("Location", "/dashboard")
                .set_body_string("Redirecting"),
        )
        .mount(&server)
        .await;

    let tester = slapper::auth::MfaTester::new(5).unwrap();
    let result = tester.test(&server.uri()).await.unwrap();

    assert!(result.mfa_enabled);
    assert!(result.mfa_bypass_possible);
    assert!(result
        .bypass_methods
        .iter()
        .any(|m| m.method.contains("Weak MFA Code")));
}

#[tokio::test]
async fn test_rate_limit_tester_no_limit() {
    let server = create_test_server().await;

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_string("OK"))
        .mount(&server)
        .await;

    let tester = slapper::auth::RateLimitTester::new(5).unwrap();
    let result = tester.test(&server.uri()).await.unwrap();

    assert!(!result.rate_limited);
}

#[tokio::test]
async fn test_rate_limit_tester_detected() {
    let server = create_test_server().await;

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(429).set_body_string("Too Many Requests"))
        .mount(&server)
        .await;

    let tester = slapper::auth::RateLimitTester::new(5).unwrap();
    let result = tester.test(&server.uri()).await.unwrap();

    assert!(result.rate_limited);
    assert_eq!(result.requests_until_limited, 1);
}

#[tokio::test]
async fn test_password_policy_tester_no_policy() {
    let server = create_test_server().await;

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_string("Login page"))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_string("error: invalid credentials"))
        .mount(&server)
        .await;

    let tester = slapper::auth::PasswordPolicyTester::new(5).unwrap();
    let result = tester.test(&server.uri()).await.unwrap();

    assert!(!result.policy_detected);
    assert!(!result.accepts_weak_passwords);
}

#[tokio::test]
async fn test_password_policy_tester_strict_policy() {
    let server = create_test_server().await;

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(
                "Password policy: minimum 12 characters, requires uppercase, lowercase, digit, and special symbol",
            ),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_string("error: weak password"))
        .mount(&server)
        .await;

    let tester = slapper::auth::PasswordPolicyTester::new(5).unwrap();
    let result = tester.test(&server.uri()).await.unwrap();

    assert!(result.policy_detected);
    assert!(result.requires_uppercase);
    assert!(result.requires_lowercase);
    assert!(result.requires_digit);
    assert!(result.requires_special);
    assert!(!result.accepts_weak_passwords);
}

#[tokio::test]
async fn test_timing_tester() {
    let server = create_test_server().await;

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_string("OK"))
        .mount(&server)
        .await;

    let tester = slapper::auth::TimingTester::new(5).unwrap();
    let result = tester.test(&server.uri()).await.unwrap();

    assert_eq!(result.measurements.len(), 7);
    assert!(!result.analysis.is_empty());
}

#[tokio::test]
async fn test_brute_force_tester_weak_credential() {
    let server = create_test_server().await;

    Mock::given(method("POST"))
        .and(path("/login"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("Welcome to the dashboard! Your session token: abc123"),
        )
        .mount(&server)
        .await;

    let tester = slapper::auth::BruteForceTester::new(10, 1, 5).unwrap();
    let result = tester
        .test(
            &format!("{}/login", server.uri()),
            "admin",
            &["password123".to_string()],
        )
        .await
        .unwrap();

    assert_eq!(result.attempts_made, 1);
    assert_eq!(result.successful_logins, 1);
    assert!(!result.weak_credentials.is_empty());
    assert!(result
        .weak_credentials
        .iter()
        .any(|c| c.password == "password123"));
}

#[tokio::test]
async fn test_brute_force_tester_lockout() {
    let server = create_test_server().await;

    Mock::given(method("POST"))
        .and(path("/login"))
        .respond_with(
            ResponseTemplate::new(423).set_body_string("Account locked: too many attempts"),
        )
        .mount(&server)
        .await;

    let tester = slapper::auth::BruteForceTester::new(10, 1, 5).unwrap();
    let result = tester
        .test(
            &format!("{}/login", server.uri()),
            "admin",
            &["pass1".to_string(), "pass2".to_string()],
        )
        .await
        .unwrap();

    assert!(result.lockout_detected);
}

#[tokio::test]
async fn test_lockout_detector_hard_lockout() {
    let server = create_test_server().await;

    Mock::given(method("POST"))
        .and(path("/login"))
        .respond_with(ResponseTemplate::new(200).set_body_string("Invalid credentials"))
        .up_to_n_times(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/login"))
        .respond_with(ResponseTemplate::new(423).set_body_string("Account has been locked"))
        .mount(&server)
        .await;

    let detector = slapper::auth::LockoutDetector::new(5).unwrap();
    let result = detector
        .detect(&format!("{}/login", server.uri()), "admin", 5)
        .await
        .unwrap();

    assert_eq!(
        result.lockout_type,
        slapper::auth::lockout::LockoutType::HardLockout
    );
    assert!(result.lockout_threshold.is_some());
}

#[tokio::test]
async fn test_lockout_detector_soft_lockout() {
    let server = create_test_server().await;

    Mock::given(method("POST"))
        .and(path("/login"))
        .respond_with(ResponseTemplate::new(429).set_body_string("Rate limit exceeded"))
        .mount(&server)
        .await;

    let detector = slapper::auth::LockoutDetector::new(5).unwrap();
    let result = detector
        .detect(&format!("{}/login", server.uri()), "admin", 5)
        .await
        .unwrap();

    assert_eq!(
        result.lockout_type,
        slapper::auth::lockout::LockoutType::SoftLockout
    );
}

#[tokio::test]
async fn test_lockout_detector_no_lockout() {
    let server = create_test_server().await;

    Mock::given(method("POST"))
        .and(path("/login"))
        .respond_with(ResponseTemplate::new(200).set_body_string("Welcome back!"))
        .mount(&server)
        .await;

    let detector = slapper::auth::LockoutDetector::new(5).unwrap();
    let result = detector
        .detect(&format!("{}/login", server.uri()), "admin", 5)
        .await
        .unwrap();

    assert_eq!(
        result.lockout_type,
        slapper::auth::lockout::LockoutType::None
    );
}

#[tokio::test]
async fn test_credential_stuffer() {
    let server = create_test_server().await;

    Mock::given(method("POST"))
        .and(path("/login"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string("Welcome! Your session token: xyz"),
        )
        .mount(&server)
        .await;

    let stuffer = slapper::auth::CredentialStuffer::new(10, 1, 5).unwrap();
    let credentials = vec![
        slapper::auth::CredentialPair {
            username: "admin".to_string(),
            password: "admin".to_string(),
        },
        slapper::auth::CredentialPair {
            username: "user".to_string(),
            password: "pass".to_string(),
        },
    ];
    let result = stuffer
        .test(&format!("{}/login", server.uri()), &credentials)
        .await
        .unwrap();

    assert_eq!(result.credentials_tested, 2);
    assert_eq!(result.successful_logins, 2);
    assert_eq!(result.compromised_accounts.len(), 2);
}
