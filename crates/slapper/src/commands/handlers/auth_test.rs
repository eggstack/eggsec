use crate::auth::{AuthEngine, AuthFinding, AuthTestReport, AuthTestType, AUTH_BANNER};
use crate::cli::AuthTestArgs;
use crate::types::Severity;
use anyhow::Result;

pub async fn handle_auth_test(
    _ctx: &crate::commands::CommandContext,
    args: AuthTestArgs,
) -> Result<()> {
    eprintln!("{}", AUTH_BANNER);

    let engine = AuthEngine::new(args.max_attempts, args.concurrency, args.timeout)?;

    let mut report = AuthTestReport {
        target: args.target.clone(),
        tests_run: Vec::new(),
        brute_force: None,
        credential_stuffing: None,
        lockout_detection: None,
        rate_limit: None,
        mfa: None,
        session: None,
        timing: None,
        total_attempts: 0,
        findings: Vec::new(),
    };

    let run_test = args.all;

    if run_test || args.rate_limit_bypass {
        report.tests_run.push(AuthTestType::RateLimitBypass);
        if args.verbose {
            eprintln!("[*] Testing rate limiting...");
        }
        let rate_tester = crate::auth::RateLimitTester::new(args.timeout)?;
        if let Ok(result) = rate_tester.test(&args.target).await {
            if result.rate_limited {
                report.findings.push(AuthFinding {
                    test_type: AuthTestType::RateLimitBypass,
                    severity: Severity::Medium,
                    title: "Rate limiting detected".to_string(),
                    description: format!(
                        "Rate limit enforced after {} requests",
                        result.requests_until_limited
                    ),
                    recommendation:
                        "Ensure rate limiting is properly configured with appropriate thresholds"
                            .to_string(),
                });
            }
            if !result.bypass_techniques.is_empty() {
                let bypassed: Vec<_> = result
                    .bypass_techniques
                    .iter()
                    .filter(|b| b.successful)
                    .collect();
                if !bypassed.is_empty() {
                    report.findings.push(AuthFinding {
                        test_type: AuthTestType::RateLimitBypass,
                        severity: Severity::High,
                        title: "Rate limit bypass possible".to_string(),
                        description: format!("{} bypass technique(s) successful", bypassed.len()),
                        recommendation:
                            "Implement IP-based rate limiting that cannot be bypassed via headers"
                                .to_string(),
                    });
                }
            }
            report.rate_limit = Some(result);
        }
    }

    if run_test || args.timing_attack {
        report.tests_run.push(AuthTestType::TimingAttack);
        if args.verbose {
            eprintln!("[*] Testing for timing vulnerabilities...");
        }
        let timing_tester = crate::auth::TimingTester::new(args.timeout)?;
        if let Ok(result) = timing_tester.test(&args.target).await {
            if result.timing_vulnerable {
                report.findings.push(AuthFinding {
                    test_type: AuthTestType::TimingAttack,
                    severity: Severity::Medium,
                    title: "Timing attack vulnerability detected".to_string(),
                    description: result.analysis.clone(),
                    recommendation: "Use constant-time string comparison for credential validation"
                        .to_string(),
                });
            }
            report.timing = Some(result);
        }
    }

    if run_test || args.session_fixation {
        report.tests_run.push(AuthTestType::SessionFixation);
        if args.verbose {
            eprintln!("[*] Testing session security...");
        }
        let session_tester = crate::auth::SessionTester::new(args.timeout)?;
        if let Ok(result) = session_tester.test(&args.target).await {
            if result.session_fixation_possible {
                report.findings.push(AuthFinding {
                    test_type: AuthTestType::SessionFixation,
                    severity: Severity::High,
                    title: "Session fixation possible".to_string(),
                    description: "Session tokens are reused across requests".to_string(),
                    recommendation: "Regenerate session tokens after authentication".to_string(),
                });
            }
            if !result.session_cookie_issues.is_empty() {
                for issue in &result.session_cookie_issues {
                    report.findings.push(AuthFinding {
                        test_type: AuthTestType::SessionFixation,
                        severity: Severity::Medium,
                        title: "Session cookie issue".to_string(),
                        description: issue.clone(),
                        recommendation: "Set HttpOnly and Secure flags on all session cookies"
                            .to_string(),
                    });
                }
            }
            report.session = Some(result);
        }
    }

    if run_test || args.mfa_bypass {
        report.tests_run.push(AuthTestType::MfaBypass);
        if args.verbose {
            eprintln!("[*] Testing MFA security...");
        }
        let mfa_tester = crate::auth::MfaTester::new(args.timeout)?;
        if let Ok(result) = mfa_tester.test(&args.target).await {
            if result.mfa_bypass_possible {
                report.findings.push(AuthFinding {
                    test_type: AuthTestType::MfaBypass,
                    severity: Severity::Critical,
                    title: "MFA bypass possible".to_string(),
                    description: format!("{} bypass method(s) found", result.bypass_methods.len()),
                    recommendation: "Implement proper MFA validation and prevent skip parameters"
                        .to_string(),
                });
            }
            report.mfa = Some(result);
        }
    }

    if run_test || args.brute_force {
        if let Some(ref username) = args.username {
            report.tests_run.push(AuthTestType::BruteForce);
            if args.verbose {
                eprintln!(
                    "[*] Testing brute force resistance for user '{}'...",
                    username
                );
            }
            let passwords = load_passwords(&args.wordlist)?;
            let bf_tester = crate::auth::BruteForceTester::new(
                args.max_attempts,
                args.concurrency,
                args.timeout,
            )?;
            if let Ok(result) = bf_tester.test(&args.target, username, &passwords).await {
                if result.successful_logins > 0 {
                    report.findings.push(AuthFinding {
                        test_type: AuthTestType::BruteForce,
                        severity: Severity::Critical,
                        title: "Weak credentials found".to_string(),
                        description: format!(
                            "{} weak password(s) discovered for user '{}'",
                            result.successful_logins, username
                        ),
                        recommendation:
                            "Enforce strong password policy and implement account lockout"
                                .to_string(),
                    });
                }
                report.brute_force = Some(result);
            }
        }
    }

    if run_test || args.credential_stuffing {
        if let Some(ref cred_file) = args.credential_file {
            report.tests_run.push(AuthTestType::CredentialStuffing);
            if args.verbose {
                eprintln!("[*] Testing credential stuffing...");
            }
            let stuffer = crate::auth::CredentialStuffer::new(
                args.max_attempts,
                args.concurrency,
                args.timeout,
            )?;
            let credentials = stuffer.load_breach_list(cred_file)?;
            if let Ok(result) = stuffer.test(&args.target, &credentials).await {
                if result.successful_logins > 0 {
                    report.findings.push(AuthFinding {
                        test_type: AuthTestType::CredentialStuffing,
                        severity: Severity::Critical,
                        title: "Compromised accounts found".to_string(),
                        description: format!(
                            "{} compromised account(s) discovered",
                            result.successful_logins
                        ),
                        recommendation: "Implement credential stuffing detection and MFA"
                            .to_string(),
                    });
                }
                report.credential_stuffing = Some(result);
            }
        }
    }

    if run_test || args.lockout_detection {
        if let Some(ref username) = args.username {
            report.tests_run.push(AuthTestType::AccountLockout);
            if args.verbose {
                eprintln!("[*] Testing account lockout...");
            }
            let detector = crate::auth::LockoutDetector::new(args.timeout)?;
            if let Ok(result) = detector
                .detect(&args.target, username, args.max_attempts.min(20))
                .await
            {
                if result.lockout_type != crate::auth::lockout::LockoutType::None {
                    report.findings.push(AuthFinding {
                        test_type: AuthTestType::AccountLockout,
                        severity: Severity::Info,
                        title: "Account lockout detected".to_string(),
                        description: format!(
                            "Lockout after {} attempts",
                            result.attempts_before_lockout
                        ),
                        recommendation:
                            "Ensure lockout duration is appropriate and does not enable DoS"
                                .to_string(),
                    });
                } else {
                    report.findings.push(AuthFinding {
                        test_type: AuthTestType::AccountLockout,
                        severity: Severity::High,
                        title: "No account lockout".to_string(),
                        description: "No account lockout mechanism detected".to_string(),
                        recommendation: "Implement account lockout after repeated failed attempts"
                            .to_string(),
                    });
                }
                report.lockout_detection = Some(result);
            }
        }
    }

    report.total_attempts = engine
        .attempt_counter
        .load(std::sync::atomic::Ordering::SeqCst);

    let output = if args.json {
        serde_json::to_string_pretty(&report)?
    } else {
        format_auth_report(&report)
    };

    if let Some(ref output_file) = args.output {
        tokio::fs::write(output_file, &output).await?;
        eprintln!("Results written to {}", output_file);
    } else {
        println!("{}", output);
    }

    Ok(())
}

fn load_passwords(wordlist_path: &Option<String>) -> Result<Vec<String>> {
    if let Some(path) = wordlist_path {
        let content = std::fs::read_to_string(path)?;
        Ok(content
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect())
    } else {
        Ok(vec![
            "password".to_string(),
            "123456".to_string(),
            "12345678".to_string(),
            "admin".to_string(),
            "admin123".to_string(),
            "password123".to_string(),
            "letmein".to_string(),
            "welcome".to_string(),
            "monkey".to_string(),
            "dragon".to_string(),
        ])
    }
}

fn format_auth_report(report: &AuthTestReport) -> String {
    let mut s = String::new();
    s.push_str(&format!("Auth Test Report: {}\n", report.target));
    s.push_str(&format!("Tests run: {}\n", report.tests_run.len()));
    s.push_str(&format!("Total attempts: {}\n", report.total_attempts));
    s.push_str(&format!("Findings: {}\n", report.findings.len()));
    s.push('\n');

    for finding in &report.findings {
        s.push_str(&format!(
            "[{}] {}: {}\n",
            finding.severity.as_str().to_uppercase(),
            finding.title,
            finding.description
        ));
        s.push_str(&format!("  Recommendation: {}\n", finding.recommendation));
    }

    s
}
