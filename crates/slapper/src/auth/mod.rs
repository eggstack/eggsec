//! Authentication security testing module
//!
//! Provides credential stuffing, brute force, and authentication bypass testing
//! with built-in safety mechanisms.

pub mod brute_force;
pub mod credential_stuffing;
pub mod lockout;
pub mod mfa;
pub mod password_policy;
pub mod rate_limit;
pub mod session;
pub mod timing;

#[cfg(feature = "nse-ssh2")]
pub mod multi_protocol;

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

pub use brute_force::BruteForceTester;
pub use credential_stuffing::{CredentialPair, CredentialStuffer};
pub use lockout::LockoutDetector;
pub use mfa::MfaTester;
pub use password_policy::{PasswordPolicyResult, PasswordPolicyTester};
pub use rate_limit::RateLimitTester;
pub use session::SessionTester;
pub use timing::TimingTester;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthTestReport {
    pub target: String,
    pub tests_run: Vec<AuthTestType>,
    pub brute_force: Option<brute_force::BruteForceResult>,
    pub credential_stuffing: Option<credential_stuffing::CredentialStuffingResult>,
    pub lockout_detection: Option<lockout::LockoutDetectionResult>,
    pub rate_limit: Option<rate_limit::RateLimitResult>,
    pub mfa: Option<mfa::MfaTestResult>,
    pub session: Option<session::SessionTestResult>,
    pub timing: Option<timing::TimingTestResult>,
    pub password_policy: Option<PasswordPolicyResult>,
    pub total_attempts: usize,
    pub findings: Vec<AuthFinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuthTestType {
    BruteForce,
    CredentialStuffing,
    AccountLockout,
    RateLimitBypass,
    MfaBypass,
    SessionFixation,
    TimingAttack,
    PasswordPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthFinding {
    pub test_type: AuthTestType,
    pub severity: Severity,
    pub title: String,
    pub description: String,
    pub recommendation: String,
}

pub use crate::types::Severity;

pub struct AuthEngine {
    pub max_attempts: usize,
    pub stop_on_lockout: bool,
    pub concurrency: usize,
    pub timeout_secs: u64,
    pub username_list: Option<Vec<String>>,
    pub password_list: Option<Vec<String>>,
    pub stop_flag: Arc<AtomicBool>,
    pub attempt_counter: Arc<AtomicUsize>,
}

impl AuthEngine {
    pub fn new(
        max_attempts: usize,
        concurrency: usize,
        timeout_secs: u64,
        stop_on_lockout: bool,
    ) -> Result<Self> {
        Ok(Self {
            max_attempts,
            stop_on_lockout,
            concurrency,
            timeout_secs,
            username_list: None,
            password_list: None,
            stop_flag: Arc::new(AtomicBool::new(false)),
            attempt_counter: Arc::new(AtomicUsize::new(0)),
        })
    }

    pub fn load_wordlists(&mut self, usernames: Vec<String>, passwords: Vec<String>) {
        self.username_list = Some(usernames);
        self.password_list = Some(passwords);
    }

    pub fn increment_attempts(&self) -> bool {
        let current = self.attempt_counter.fetch_add(1, Ordering::SeqCst) + 1;
        if current >= self.max_attempts {
            self.stop_flag.store(true, Ordering::SeqCst);
            false
        } else {
            !self.stop_flag.load(Ordering::SeqCst)
        }
    }

    pub fn should_stop(&self) -> bool {
        self.stop_flag.load(Ordering::SeqCst)
    }

    pub fn stop(&self) {
        self.stop_flag.store(true, Ordering::SeqCst);
    }

    pub async fn run_full_test(&self, target: &str) -> Result<AuthTestReport> {
        let mut report = AuthTestReport {
            target: target.to_string(),
            tests_run: Vec::new(),
            brute_force: None,
            credential_stuffing: None,
            lockout_detection: None,
            rate_limit: None,
            mfa: None,
            session: None,
            timing: None,
            password_policy: None,
            total_attempts: 0,
            findings: Vec::new(),
        };

        report.tests_run.push(AuthTestType::BruteForce);
        let brute_tester =
            BruteForceTester::new(self.max_attempts, self.concurrency, self.timeout_secs)?;
        if let (Some(ref usernames), Some(ref passwords)) =
            (&self.username_list, &self.password_list)
        {
            if let Some(username) = usernames.first() {
                if let Ok(result) = brute_tester.test(target, username, passwords).await {
                    report.brute_force = Some(result);
                }
            }
        }

        report.tests_run.push(AuthTestType::CredentialStuffing);
        let stuffer =
            CredentialStuffer::new(self.max_attempts, self.concurrency, self.timeout_secs)?;
        let credentials: Vec<CredentialPair> = self
            .username_list
            .as_ref()
            .zip(self.password_list.as_ref())
            .map(|(users, passes)| {
                users
                    .iter()
                    .flat_map(|u| {
                        passes.iter().map(move |p| CredentialPair {
                            username: u.clone(),
                            password: p.clone(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        if let Ok(result) = stuffer.test(target, &credentials).await {
            report.credential_stuffing = Some(result);
        }

        report.tests_run.push(AuthTestType::AccountLockout);
        let lockout_detector = LockoutDetector::new(self.timeout_secs)?;
        let username = self
            .username_list
            .as_ref()
            .and_then(|u| u.first().map(|s| s.as_str()))
            .unwrap_or("admin");
        if let Ok(result) = lockout_detector
            .detect(target, username, self.max_attempts)
            .await
        {
            report.lockout_detection = Some(result);
        }

        report.tests_run.push(AuthTestType::RateLimitBypass);
        let rate_tester = RateLimitTester::new(self.timeout_secs)?;
        if let Ok(result) = rate_tester.test(target).await {
            report.rate_limit = Some(result);
        }

        report.tests_run.push(AuthTestType::MfaBypass);
        let mfa_tester = MfaTester::new(self.timeout_secs)?;
        if let Ok(result) = mfa_tester.test(target).await {
            report.mfa = Some(result);
        }

        report.tests_run.push(AuthTestType::SessionFixation);
        let session_tester = SessionTester::new(self.timeout_secs)?;
        if let Ok(result) = session_tester.test(target).await {
            report.session = Some(result);
        }

        report.tests_run.push(AuthTestType::TimingAttack);
        let timing_tester = TimingTester::new(self.timeout_secs)?;
        if let Ok(result) = timing_tester.test(target).await {
            report.timing = Some(result);
        }

        report.tests_run.push(AuthTestType::PasswordPolicy);
        let policy_tester = PasswordPolicyTester::new(self.timeout_secs)?;
        if let Ok(result) = policy_tester.test(target).await {
            report.password_policy = Some(result);
        }

        report.total_attempts = self.attempt_counter.load(Ordering::SeqCst);

        Ok(report)
    }
}

pub const AUTH_BANNER: &str = r#"
╔══════════════════════════════════════════════════════════╗
║  ⚠️  AUTHORIZED USE ONLY  ⚠️                            ║
║                                                          ║
║  This tool performs authentication security testing.     ║
║  Only use against systems you have explicit permission   ║
║  to test. Unauthorized access is illegal.                ║
╚══════════════════════════════════════════════════════════╝
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_engine_creation() {
        let engine = AuthEngine::new(100, 10, 30, true);
        assert!(engine.is_ok());
    }

    #[test]
    fn test_auth_engine_stop_flag() {
        let engine = AuthEngine::new(100, 10, 30, true).unwrap();
        assert!(!engine.should_stop());
        engine.stop();
        assert!(engine.should_stop());
    }

    #[test]
    fn test_auth_engine_attempt_counter() {
        let engine = AuthEngine::new(5, 10, 30, true).unwrap();
        assert!(engine.increment_attempts());
        assert!(engine.increment_attempts());
        assert!(engine.increment_attempts());
        assert!(engine.increment_attempts());
        assert!(!engine.increment_attempts());
        assert!(engine.should_stop());
    }

    #[test]
    fn test_auth_engine_load_wordlists() {
        let mut engine = AuthEngine::new(100, 10, 30, true).unwrap();
        engine.load_wordlists(
            vec!["admin".to_string(), "user".to_string()],
            vec!["password".to_string(), "123456".to_string()],
        );
        assert!(engine.username_list.is_some());
        assert!(engine.password_list.is_some());
        assert_eq!(engine.username_list.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_auth_finding_creation() {
        let finding = AuthFinding {
            test_type: AuthTestType::BruteForce,
            severity: Severity::High,
            title: "Weak password policy".to_string(),
            description: "System accepts common passwords".to_string(),
            recommendation: "Enforce stronger password requirements".to_string(),
        };
        assert_eq!(finding.test_type, AuthTestType::BruteForce);
        assert_eq!(finding.severity, Severity::High);
    }

    #[test]
    fn test_auth_test_type_variants() {
        assert_eq!(AuthTestType::BruteForce, AuthTestType::BruteForce);
        assert_eq!(
            AuthTestType::CredentialStuffing,
            AuthTestType::CredentialStuffing
        );
        assert_eq!(AuthTestType::AccountLockout, AuthTestType::AccountLockout);
        assert_eq!(AuthTestType::RateLimitBypass, AuthTestType::RateLimitBypass);
        assert_eq!(AuthTestType::MfaBypass, AuthTestType::MfaBypass);
        assert_eq!(AuthTestType::SessionFixation, AuthTestType::SessionFixation);
        assert_eq!(AuthTestType::TimingAttack, AuthTestType::TimingAttack);
        assert_eq!(AuthTestType::PasswordPolicy, AuthTestType::PasswordPolicy);
    }

    #[test]
    fn test_auth_banner_not_empty() {
        assert!(!AUTH_BANNER.is_empty());
    }
}
