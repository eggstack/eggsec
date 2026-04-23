//! Autonomous security agent for Slapper.
//!
//! This module provides an event-driven agent that:
//! - Monitors configured targets on schedules
//! - Executes security scans based on events
//! - Maintains longitudinal memory of scan results
//! - Routes alerts to configured channels
//! - Provides skills to guide AI assistants

pub mod portfolio;
pub mod memory;
pub mod alerts;
pub mod channels;
pub mod events;

#[cfg(feature = "ai-integration")]
pub mod skills;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use chrono::{DateTime, Utc};
use tokio::sync::RwLock;
use tokio::time::interval;

use crate::output::schedule::CronScheduler;
use crate::tool::{
    create_default_registry, ToolDispatcher, ToolRegistry, ToolRequest,
    ToolResponse,
};

#[cfg(feature = "ai-integration")]
use crate::ai::AiClient;

pub use portfolio::{Priority, ScanRecord, TargetConfig, TargetPortfolio};
pub use memory::LongitudinalMemory;
pub use alerts::{
    AggregatedAlert, Alert, AlertChannel, AlertRouter, AlertRoutingRules, AlertTemplate, EmailChannel,
    EmailFormattedAlert, EmailTemplate, EscalationLevel, PagerDutyChannel, PagerDutyTemplate,
    ReportSummary, ScanReport, SlackChannel, SlackFormattedAlert, SlackTemplate, TimeBasedRouting, TimeRange,
    WebhookConfig,
};
pub use events::{EventHandler, SecurityEvent};

#[cfg(feature = "ai-integration")]
pub use skills::{Skill, SkillLoader, SkillRegistry};

#[derive(Clone)]
pub struct AgentConfig {
    pub portfolio_path: Option<PathBuf>,
    pub memory_dir: PathBuf,
    pub poll_interval_secs: u64,
    pub ai_config: Option<crate::config::AiConfig>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        let memory_dir = directories::ProjectDirs::from("com", "slapper", "slapper")
            .map(|d| d.config_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("~/.config/slapper"));

        Self {
            portfolio_path: None,
            memory_dir,
            poll_interval_secs: 60,
            ai_config: None,
        }
    }
}

pub struct Agent {
    config: AgentConfig,
    registry: ToolRegistry,
    dispatcher: ToolDispatcher,
    #[cfg(feature = "ai-integration")]
    ai_client: Option<AiClient>,
    scheduler: CronScheduler,
    portfolio: TargetPortfolio,
    memory: LongitudinalMemory,
    alert_router: AlertRouter,
    event_handlers: Vec<Box<dyn EventHandler>>,
    running: Arc<RwLock<bool>>,
}

impl Agent {
    pub async fn new(config: AgentConfig) -> Result<Self> {
        let registry = create_default_registry();
        let dispatcher = ToolDispatcher::new(registry.clone());

        let portfolio = if let Some(ref path) = config.portfolio_path {
            TargetPortfolio::load_from_file(path)?
        } else {
            TargetPortfolio::new()
        };

        let memory_dir = config.memory_dir.join("memory");
        let memory = LongitudinalMemory::new(memory_dir)?;

        let alert_router = AlertRouter::new();

        Ok(Self {
            config,
            registry,
            dispatcher,
            #[cfg(feature = "ai-integration")]
            ai_client: None,
            scheduler: CronScheduler::new(),
            portfolio,
            memory,
            alert_router,
            event_handlers: Vec::new(),
            running: Arc::new(RwLock::new(false)),
        })
    }

    #[cfg(feature = "ai-integration")]
    pub async fn with_ai_client(mut self, ai_config: crate::config::AiConfig) -> Self {
        let ai_client = AiClient::new(ai_config.clone());
        self.ai_client = Some(ai_client);
        self
    }

    pub fn register_handler(&mut self, handler: Box<dyn EventHandler>) {
        self.event_handlers.push(handler);
    }

    pub async fn run(&mut self) -> Result<()> {
        {
            let mut running = self.running.write().await;
            if *running {
                return Ok(());
            }
            *running = true;
        }

        tracing::info!("Starting autonomous security agent");

        let mut poll_interval = interval(Duration::from_secs(self.config.poll_interval_secs));

        loop {
            tokio::select! {
                _ = poll_interval.tick() => {
                    if self.process_scheduled_scans().await.is_ok() {
                        tracing::debug!("Processed scheduled scans");
                    }
                }
            }

            let running = self.running.read().await;
            if !*running {
                break;
            }
        }

        tracing::info!("Agent stopped");
        Ok(())
    }

    pub async fn stop(&self) {
        let mut running = self.running.write().await;
        *running = false;
    }

    async fn process_scheduled_scans(&mut self) -> Result<()> {
        let now = Utc::now();
        let targets = self.portfolio.get_all_targets();

        for (target_id, config) in targets {
            if let Some(ref schedule) = config.schedule {
                if self.scheduler.should_run_for(schedule, &now) {
                    if let Some(ref window) = config.off_peak_window {
                        if !window.is_in_window(&now) {
                            tracing::debug!(
                                "Skipping {} - outside off-peak window",
                                target_id
                            );
                            continue;
                        }
                    }

                    tracing::info!(
                        "Triggering {} scan for {}",
                        config.scan_depth.as_str(),
                        target_id
                    );

                    let result = self
                        .execute_scan_with_depth(&config.target, "pipeline", config.scan_depth)
                        .await;

                    if let Ok(ref response) = result {
                        self.memory.store_scan_results(&config.target, response)?;

                        let findings = self.process_findings(response);
                        if !findings.is_empty() {
                            self.handle_findings(&config.target, findings).await;
                        }
                    }

                    self.portfolio.update_last_scan(&target_id, &now);
                }
            }
        }

        Ok(())
    }

    pub async fn execute_scan(
        &self,
        target: &str,
        scan_type: &str,
    ) -> Result<ToolResponse> {
        self.execute_scan_with_depth(target, scan_type, crate::agent::portfolio::ScanDepth::Shallow)
            .await
    }

    pub async fn execute_scan_with_depth(
        &self,
        target: &str,
        scan_type: &str,
        depth: crate::agent::portfolio::ScanDepth,
    ) -> Result<ToolResponse> {
        let params = match depth {
            crate::agent::portfolio::ScanDepth::Shallow => {
                serde_json::json!({
                    "concurrency": 5,
                    "timeout_ms": 30000,
                    "payload_types": "xss,sqli",
                })
            }
            crate::agent::portfolio::ScanDepth::Deep => {
                serde_json::json!({
                    "concurrency": 20,
                    "timeout_ms": 120000,
                    "payload_types": "xss,sqli,ssrf,command,ssti,xxe,nosql,ldap",
                    "mutate": true,
                    "mutation_count": 5,
                })
            }
        };

        let request = ToolRequest {
            id: uuid::Uuid::new_v4().to_string(),
            tool: scan_type.to_string(),
            target: crate::tool::Target {
                value: target.to_string(),
                target_type: crate::tool::TargetType::Url,
                scope: None,
            },
            params,
            options: Default::default(),
            cancellation_token: None,
        };

        self.dispatcher
            .dispatch(request)
            .await
            .map_err(|e| anyhow::anyhow!("{:?}", e))
    }

    fn process_findings(&self, response: &ToolResponse) -> Vec<crate::tool::response::Finding> {
        response.findings.clone()
    }

    async fn handle_findings(&mut self, target: &str, findings: Vec<crate::tool::response::Finding>) {
        let critical_findings: Vec<_> = findings.iter()
            .filter(|f| matches!(f.severity, crate::tool::response::ResponseSeverity::Critical))
            .collect();

        if !critical_findings.is_empty() {
            let critical_count = critical_findings.len();
            let alert_severity = critical_findings.first()
                .map(|f| f.severity.to_agent_severity())
                .unwrap_or(crate::types::Severity::Critical);
            let alert = Alert {
                severity: alert_severity,
                title: format!("{} critical findings on {}", critical_count, target),
                message: format!(
                    "Detected {} critical severity findings during scan of {}",
                    critical_count, target
                ),
                target: target.to_string(),
                finding_ids: findings.iter().map(|f| f.id.clone()).collect(),
                recommended_actions: vec![
                    "Review findings immediately".to_string(),
                    "Consider initiating emergency response".to_string(),
                ],
            };

            if let Err(e) = self.alert_router.send(&alert).await {
                tracing::error!("Failed to send alert: {}", e);
            }
        }
    }

    pub async fn trigger_scan(&mut self, target: &str, scan_type: &str) -> Result<ToolResponse> {
        tracing::info!("Manually triggered scan for {} (type: {})", target, scan_type);

        let result = self.execute_scan(target, scan_type).await?;

        if let Err(e) = self.memory.store_scan_results(target, &result) {
            tracing::warn!("Failed to store scan results: {}", e);
        }

        Ok(result)
    }

    pub async fn trigger_event(&mut self, event: SecurityEvent) -> Result<()> {
        tracing::debug!("Event triggered: {:?}", event.event_type());

        let handlers: Vec<Box<dyn EventHandler>> = std::mem::take(&mut self.event_handlers);

        for handler in &handlers {
            if handler.handles(&event) {
                handler.handle(&event, self).await?;
            }
        }

        self.event_handlers = handlers;

        Ok(())
    }
}

impl CronScheduler {
    pub fn should_run_for(&self, schedule: &str, now: &DateTime<Utc>) -> bool {
        if let Ok(expr) = crate::output::schedule::CronExpression::parse(schedule) {
            expr.matches(now)
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::pin::Pin;
    use std::future::Future;

    #[tokio::test]
    async fn test_agent_creation() {
        let config = AgentConfig::default();
        let agent = Agent::new(config).await;
        assert!(agent.is_ok());
    }

    #[cfg(feature = "ai-integration")]
    #[tokio::test]
    async fn test_agent_with_ai_client() {
        let config = AgentConfig::default();
        let agent = Agent::new(config).await.unwrap();
        let ai_config = crate::config::AiConfig {
            provider: "openai".to_string(),
            model: Some("gpt-4".to_string()),
            api_key: Some(crate::types::SensitiveString::from("test-key".to_string())),
            base_url: Some("https://api.openai.com/v1/chat/completions".to_string()),
            max_tokens: Some(2048),
            temperature: Some(0.7),
            max_payloads: 50,
            max_bypasses: 10,
        };
        let agent_with_ai = agent.with_ai_client(ai_config).await;
        assert!(agent_with_ai.ai_client.is_some());
    }

    #[cfg(feature = "ai-integration")]
    #[tokio::test]
    async fn test_agent_execute_scan_returns_result() {
        let config = AgentConfig::default();
        let agent = Agent::new(config).await.unwrap();
        let result = agent.execute_scan("https://example.com", "recon").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_agent_stop() {
        let config = AgentConfig::default();
        let agent = Agent::new(config).await.unwrap();
        agent.stop().await;
    }

    #[tokio::test]
    async fn test_agent_portfolio_operations() {
        let config = AgentConfig::default();
        let agent = Agent::new(config).await.unwrap();

        let targets = agent.portfolio.get_all_targets();
        assert!(targets.is_empty());

        let config = TargetConfig {
            target: "https://example.com".to_string(),
            schedule: Some("0 0 * * *".to_string()),
            ..Default::default()
        };
        agent.portfolio.add_target("example.com".to_string(), config);

        let targets = agent.portfolio.get_all_targets();
        assert_eq!(targets.len(), 1);
    }

    #[tokio::test]
    async fn test_agent_register_event_handler() {
        let mut agent = Agent::new(AgentConfig::default()).await.unwrap();
        struct TestHandler;
        impl EventHandler for TestHandler {
            fn handles(&self, _event: &SecurityEvent) -> bool {
                true
            }
            fn handle<'a>(
                &'a self,
                _event: &'a SecurityEvent,
                _agent: &'a mut Agent,
            ) -> Pin<Box<dyn Future<Output = Result<()>> + 'a>> {
                Box::pin(async { Ok(()) })
            }
        }
        agent.register_handler(Box::new(TestHandler));
        assert_eq!(agent.event_handlers.len(), 1);
    }

    #[test]
    fn test_cron_scheduler_should_run_for_valid_expression() {
        let scheduler = CronScheduler::new();
        let now = chrono::Utc::now();
        assert!(scheduler.should_run_for("0 0 * * *", &now));
    }

    #[test]
    fn test_cron_scheduler_should_not_run_for_invalid_expression() {
        let scheduler = CronScheduler::new();
        let now = chrono::Utc::now();
        assert!(!scheduler.should_run_for("invalid", &now));
    }

    #[test]
    fn test_agent_config_default() {
        let config = AgentConfig::default();
        assert!(config.portfolio_path.is_none());
        assert!(config.ai_config.is_none());
        assert_eq!(config.poll_interval_secs, 60);
    }
}
