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
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone)]
pub struct HuntClient {
    pub client: Client,
    pub target: String,
    pub timeout: Duration,
}

impl HuntClient {
    pub fn new(target: &str, config: &HuntConfig) -> Result<Self> {
        if !target.starts_with("http://") && !target.starts_with("https://") {
            return Err(crate::error::SlapperError::Http(format!(
                "Invalid target URL: must start with http:// or https://, got: {}",
                target
            )));
        }

        let timeout = Duration::from_millis(config.timeout_ms);
        let client = Client::builder()
            .timeout(timeout)
            .cookie_store(true)
            .redirect(reqwest::redirect::Policy::limited(10))
            .pool_max_idle_per_host(crate::constants::DEFAULT_POOL_MAX_IDLE_PER_HOST)
            .pool_idle_timeout(Duration::from_secs(
                crate::constants::DEFAULT_POOL_IDLE_TIMEOUT_SECS,
            ))
            .tcp_nodelay(true)
            .danger_accept_invalid_certs(true)
            .build()
            .map_err(|e| crate::error::SlapperError::Http(e.to_string()))?;

        Ok(Self {
            client,
            target: target.to_string(),
            timeout,
        })
    }

    fn build_url(&self, path: &str) -> String {
        if path.starts_with("http") {
            path.to_string()
        } else {
            format!(
                "{}{}",
                self.target.trim_end_matches('/'),
                if path.starts_with('/') {
                    path.to_string()
                } else {
                    format!("/{}", path)
                }
            )
        }
    }

    pub async fn get(&self, path: &str) -> Result<reqwest::Response> {
        let url = self.build_url(path);

        self.client
            .get(&url)
            .header("User-Agent", "Slapper/1.0 Security Testing")
            .send()
            .await
            .map_err(|e| crate::error::SlapperError::Http(e.to_string()))
    }

    pub async fn post_json(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<reqwest::Response> {
        let url = self.build_url(path);

        self.client
            .post(&url)
            .header("User-Agent", "Slapper/1.0 Security Testing")
            .header("Content-Type", "application/json")
            .json(body)
            .send()
            .await
            .map_err(|e| crate::error::SlapperError::Http(e.to_string()))
    }

    pub async fn head(&self, path: &str) -> Result<reqwest::Response> {
        let url = self.build_url(path);

        self.client
            .head(&url)
            .header("User-Agent", "Slapper/1.0 Security Testing")
            .send()
            .await
            .map_err(|e| crate::error::SlapperError::Http(e.to_string()))
    }

    pub async fn request(&self, method: reqwest::Method, path: &str) -> Result<reqwest::Response> {
        let url = self.build_url(path);

        self.client
            .request(method, &url)
            .header("User-Agent", "Slapper/1.0 Security Testing")
            .send()
            .await
            .map_err(|e| crate::error::SlapperError::Http(e.to_string()))
    }

    pub fn base_url(&self) -> &str {
        &self.target
    }
}

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
        self.total_findings += 1;
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

#[tracing::instrument(skip(config), fields(target = %target))]
pub async fn run_hunt(target: &str, config: HuntConfig) -> Result<HuntReport> {
    tracing::info!("Starting vulnerability hunt");
    let mut report = HuntReport::new(target);
    let client = HuntClient::new(target, &config)?;

    if config.check_session {
        let issues = session::check_session_security(&client, &config).await?;
        for i in issues {
            report.add_session_issue(i);
        }
    }

    if config.check_authz_bypass {
        let bypasses = authz::check_authz_bypass(&client, &config).await?;
        for b in bypasses {
            report.add_authz_bypass(b);
        }
    }

    if config.check_race_conditions {
        let races = race::check_race_conditions(&client, &config).await?;
        for r in races {
            report.add_race_condition(r);
        }
    }

    if config.check_business_logic {
        let flaws = business::check_business_logic(&client, &config).await?;
        for f in flaws {
            report.add_business_flaw(f);
        }
    }

    if config.check_attack_chains {
        let chains = chain::detect_attack_chains(&report).await?;
        for c in chains {
            report.add_chain(c);
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
            timeout_ms: crate::constants::DEFAULT_TOOL_TIMEOUT_MS,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hunt_client_creation() {
        let config = HuntConfig::default();
        let client = HuntClient::new("http://example.com", &config).unwrap();
        assert_eq!(client.target, "http://example.com");
    }

    #[test]
    fn test_hunt_config_defaults() {
        let config = HuntConfig::default();
        assert!(config.check_attack_chains);
        assert!(config.check_business_logic);
        assert!(config.check_race_conditions);
        assert!(config.check_authz_bypass);
        assert!(config.check_session);
        assert_eq!(config.concurrency, 10);
        assert_eq!(config.timeout_ms, crate::constants::DEFAULT_TOOL_TIMEOUT_MS);
    }

    #[test]
    fn test_hunt_report_new() {
        let report = HuntReport::new("http://test.com");
        assert_eq!(report.target, "http://test.com");
        assert_eq!(report.total_findings, 0);
    }
}
