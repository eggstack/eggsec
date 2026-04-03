//! Intelligent vulnerability hunting module
//!
//! Provides advanced vulnerability detection capabilities including attack chain
//! analysis, business logic flaw detection, race condition testing, authorization
//! bypass testing, and session security analysis.
//!
//! ## Modules
//!
//! - [`chain`] - Attack chain detection (privilege escalation, data exfiltration, RCE)
//! - [`business`] - Business logic vulnerability detection
//! - [`race`] - Race condition and concurrency testing
//! - [`authz`] - Authorization bypass testing
//! - [`session`] - Session management security testing

pub mod authz;
pub mod business;
pub mod chain;
pub mod race;
pub mod session;

use crate::error::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HuntReport {
    pub target: String,
    pub attack_chains: Vec<chain::AttackChain>,
    pub business_logic: Vec<business::BusinessLogicFlaw>,
    pub race_conditions: Vec<race::RaceCondition>,
    pub authz_bypasses: Vec<authz::AuthzBypass>,
    pub session_issues: Vec<session::SessionIssue>,
    pub total_findings: usize,
}

impl HuntReport {
    pub fn new(target: &str) -> Self {
        Self {
            target: target.to_string(),
            ..Default::default()
        }
    }

    pub fn add_chain(&mut self, chain: chain::AttackChain) {
        self.total_findings += chain.steps.len();
        self.attack_chains.push(chain);
    }

    pub fn add_business_flaw(&mut self, flaw: business::BusinessLogicFlaw) {
        self.total_findings += 1;
        self.business_logic.push(flaw);
    }

    pub fn add_race_condition(&mut self, race: race::RaceCondition) {
        self.total_findings += 1;
        self.race_conditions.push(race);
    }

    pub fn add_authz_bypass(&mut self, bypass: authz::AuthzBypass) {
        self.total_findings += 1;
        self.authz_bypasses.push(bypass);
    }

    pub fn add_session_issue(&mut self, issue: session::SessionIssue) {
        self.total_findings += 1;
        self.session_issues.push(issue);
    }
}

pub async fn run_hunt(target: &str, config: HuntConfig) -> Result<HuntReport> {
    let mut report = HuntReport::new(target);

    if config.check_attack_chains {
        let chains = chain::detect_attack_chains(target, &config).await?;
        for c in chains {
            report.add_chain(c);
        }
    }

    if config.check_business_logic {
        let flaws = business::check_business_logic(target, &config).await?;
        for f in flaws {
            report.add_business_flaw(f);
        }
    }

    if config.check_race_conditions {
        let races = race::check_race_conditions(target, &config).await?;
        for r in races {
            report.add_race_condition(r);
        }
    }

    if config.check_authz_bypass {
        let bypasses = authz::check_authz_bypass(target, &config).await?;
        for b in bypasses {
            report.add_authz_bypass(b);
        }
    }

    if config.check_session {
        let issues = session::check_session_security(target, &config).await?;
        for i in issues {
            report.add_session_issue(i);
        }
    }

    Ok(report)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HuntConfig {
    pub check_attack_chains: bool,
    pub check_business_logic: bool,
    pub check_race_conditions: bool,
    pub check_authz_bypass: bool,
    pub check_session: bool,
    pub concurrency: usize,
    pub timeout_ms: u64,
}

impl Default for HuntConfig {
    fn default() -> Self {
        Self {
            check_attack_chains: true,
            check_business_logic: true,
            check_race_conditions: true,
            check_authz_bypass: true,
            check_session: true,
            concurrency: 10,
            timeout_ms: 30000,
        }
    }
}
