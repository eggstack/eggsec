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
pub mod logging;
pub mod config_watcher;
pub mod constraints;

#[cfg(feature = "ai-integration")]
pub mod skills;

use std::path::PathBuf;
use std::pin::Pin;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use chrono::{DateTime, Utc, Timelike};
use tokio::sync::RwLock;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;

use crate::config::SlapperConfig;
use crate::output::schedule::CronScheduler;
use crate::tool::{
    create_default_registry, ToolDispatcher, ToolRegistry, ToolRequest,
    ToolResponse,
};

pub use config_watcher::{ConfigReloader, ConfigWatcher, SlapperConfigReloader};

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
pub use constraints::{
    ConstraintChecker, OperationalConstraints, ForbiddenAction, DoNotDoList, OffPeakConfig,
};

#[cfg(feature = "ai-integration")]
pub use skills::{Skill, SkillLoader, SkillRegistry};

// Crate-private traits for testable seams (Phase 2)
trait ScanDispatcherTrait: Send + Sync {
    fn dispatch<'a>(
        &'a self,
        request: ToolRequest,
    ) -> Pin<Box<dyn Future<Output = std::result::Result<ToolResponse, crate::error::SlapperError>> + Send + 'a>>;
}

trait AlertSenderTrait: Send + Sync {
    fn send(
        &self,
        alert: Alert,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;
}

// Implement traits for existing types
impl ScanDispatcherTrait for ToolDispatcher {
    fn dispatch<'a>(
        &'a self,
        request: ToolRequest,
    ) -> Pin<Box<dyn Future<Output = std::result::Result<ToolResponse, crate::error::SlapperError>> + Send + 'a>> {
        Box::pin(self.dispatch(request))
    }
}

impl AlertSenderTrait for AlertRouter {
    fn send(
        &self,
        alert: Alert,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        let alert = std::sync::Arc::new(alert);
        Box::pin(async move {
            self.send(&alert).await
        })
    }
}

#[derive(Clone)]
pub struct AgentConfig {
    pub portfolio_path: Option<PathBuf>,
    pub memory_dir: PathBuf,
    pub poll_interval_secs: u64,
    pub ai_config: Option<crate::config::AiConfig>,
    pub operational_constraints: Option<crate::agent::constraints::OperationalConstraints>,
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
            operational_constraints: None,
        }
    }
}

pub struct Agent {
    config: AgentConfig,
    #[allow(dead_code)]
    registry: ToolRegistry,
    constraint_checker: ConstraintChecker,
    dispatcher: Box<dyn ScanDispatcherTrait + Send + Sync>,
    #[cfg(feature = "ai-integration")]
    ai_client: Option<AiClient>,
    scheduler: CronScheduler,
    portfolio: TargetPortfolio,
    memory: LongitudinalMemory,
    alert_router: Box<dyn AlertSenderTrait + Send + Sync>,
    event_handlers: Vec<Box<dyn EventHandler>>,
    running: Arc<RwLock<bool>>,
    #[allow(dead_code)]
    config_watcher: Option<ConfigWatcher>,
}

impl Agent {
    pub async fn new(config: AgentConfig) -> Result<Self> {
        let registry = create_default_registry();
        let dispatcher = ToolDispatcher::new(registry.clone());
        let dispatcher: Box<dyn ScanDispatcherTrait + Send + Sync> = Box::new(dispatcher);

        let portfolio = if let Some(ref path) = config.portfolio_path {
            TargetPortfolio::load_from_file(path)?
        } else {
            TargetPortfolio::new()
        };

        let memory_dir = config.memory_dir.join("memory");
        let memory = LongitudinalMemory::new(memory_dir).await?;
        memory.warm_cache().await.ok();

        let alert_router = AlertRouter::new()?;
        let alert_router: Box<dyn AlertSenderTrait + Send + Sync> = Box::new(alert_router);

        let config_paths = std::iter::once(config.portfolio_path.clone())
            .flatten()
            .chain(SlapperConfig::default_path())
            .collect::<Vec<_>>();
        let reloader = Arc::new(SlapperConfigReloader::new(
            config.portfolio_path.clone(),
            SlapperConfig::default_path(),
        ));
        let config_watcher = ConfigWatcher::new(config_paths, reloader).ok();

        let constraint_checker = if let Some(constraints) = config.operational_constraints.clone() {
            ConstraintChecker::new(constraints)
        } else {
            ConstraintChecker::new(OperationalConstraints::default())
        };

        Ok(Self {
            config,
            registry,
            constraint_checker,
            dispatcher,
            #[cfg(feature = "ai-integration")]
            ai_client: None,
            scheduler: CronScheduler::new(),
            portfolio,
            memory,
            alert_router,
            event_handlers: Vec::new(),
            running: Arc::new(RwLock::new(false)),
            config_watcher,
        })
    }

    // Crate-private constructor for testing with custom dispatch/alert sender
    #[cfg(test)]
    pub(crate) async fn new_for_test(
        config: AgentConfig,
        dispatcher: Box<dyn ScanDispatcherTrait + Send + Sync>,
        alert_router: Box<dyn AlertSenderTrait + Send + Sync>,
    ) -> Result<Self> {
        let registry = create_default_registry();
        let portfolio = if let Some(ref path) = config.portfolio_path {
            TargetPortfolio::load_from_file(path)?
        } else {
            TargetPortfolio::new()
        };
        let memory_dir = config.memory_dir.join("memory");
        let memory = LongitudinalMemory::new(memory_dir).await?;
        let constraint_checker = if let Some(constraints) = config.operational_constraints.clone() {
            ConstraintChecker::new(constraints)
        } else {
            ConstraintChecker::new(OperationalConstraints::default())
        };
        Ok(Self {
            config,
            registry,
            constraint_checker,
            dispatcher,
            #[cfg(feature = "ai-integration")]
            ai_client: None,
            scheduler: CronScheduler::new(),
            portfolio,
            memory,
            alert_router,
            event_handlers: Vec::new(),
            running: Arc::new(RwLock::new(false)),
            config_watcher: None,
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

    pub fn get_snapshot_path(&self) -> std::path::PathBuf {
        self.memory.storage_dir().join("portfolio_snapshot.json")
    }

    pub async fn run(&mut self) -> Result<()> {
        {
            let mut running = self.running.write().await;
            if *running {
                return Ok(());
            }
            *running = true;
        }

        let log_dir = self.config.memory_dir.join("logs");
        let _logger = logging::AgentLogger::init(log_dir)?;

        tracing::info!("Starting autonomous security agent");

        let token = CancellationToken::new();
        let token_clone = token.clone();

        tokio::spawn(async move {
            tokio::signal::ctrl_c().await.ok();
            tracing::info!("Received shutdown signal");
            token_clone.cancel();
        });

        let mut poll_interval = interval(Duration::from_secs(self.config.poll_interval_secs));

        loop {
            tokio::select! {
                _ = token.cancelled() => break,
                _ = poll_interval.tick() => {
                    if let Err(e) = self.process_scheduled_scans().await {
                        tracing::warn!(error = %e, "Scheduled scan failed");
                    } else {
                        tracing::debug!("Processed scheduled scans");
                    }
                }
            }

            let running = self.running.read().await;
            if !*running {
                drop(running);
                token.cancel();
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
                if self.scheduler.should_run_target(schedule, config.last_scan, &now) {
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

                    // Check operational constraints for scheduled scan
                    if let Err(e) = self.constraint_checker.evaluate_action("scan", &config.target) {
                        tracing::warn!("Scheduled scan blocked for {}: {:?}", target_id, e);
                        continue;
                    }
                    if let Err(e) = self.constraint_checker.evaluate_target(&config.target) {
                        tracing::warn!("Scheduled scan target forbidden for {}: {:?}", target_id, e);
                        continue;
                    }
                    if let Err(e) = self.constraint_checker.evaluate_scan_depth(config.scan_depth) {
                        tracing::warn!("Scheduled scan depth not allowed for {}: {:?}", target_id, e);
                        continue;
                    }
                    if let Err(e) = self.constraint_checker.evaluate_rate_limit(&config.target) {
                        tracing::warn!("Scheduled scan rate limit exceeded for {}: {:?}", target_id, e);
                        continue;
                    }

                    let result = self
                        .execute_scan_with_depth(&config.target, "pipeline", config.scan_depth, None)
                        .await;

                    if let Ok(ref response) = result {
                        // Only update last_scan after successful dispatch
                        self.portfolio.update_last_scan(&target_id, &now);

                        if let Err(e) = self.memory.store_scan_results(&config.target, response).await {
                            tracing::warn!("Failed to store scan results: {}", e);
                        }

                        let findings = self.process_findings(response);
                        if !findings.is_empty() {
                            let baseline_comparison = self.memory.compare_with_baseline(&config.target, &findings).await?;
                            let new_findings = baseline_comparison.new_findings;

                            if !new_findings.is_empty() {
                                let (to_alert, deduplicated) = self.memory.deduplicate_findings(new_findings).await?;
                                if !to_alert.is_empty() {
                                    tracing::debug!(
                                        "Alerting on {} new findings ({} deduplicated from previous scans)",
                                        to_alert.len(),
                                        deduplicated.len()
                                    );
                                    self.handle_findings(&config.target, to_alert).await?;
                                } else {
                                    tracing::debug!(
                                        "Skipped alerting on {} findings - already alerted in previous scans",
                                        deduplicated.len()
                                    );
                                }
                            }

                            if !baseline_comparison.resolved_findings.is_empty() {
                                tracing::info!(
                                    "{} previously known findings resolved on {}",
                                    baseline_comparison.resolved_findings.len(),
                                    config.target
                                );
                            }
                        }
                    }
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
        self.execute_scan_with_depth(target, scan_type, crate::agent::portfolio::ScanDepth::Shallow, None)
            .await
    }

    pub async fn execute_scan_with_depth(
        &self,
        target: &str,
        scan_type: &str,
        depth: crate::agent::portfolio::ScanDepth,
        cancellation_token: Option<CancellationToken>,
    ) -> Result<ToolResponse> {
        // Check operational constraints before dispatch
        self.constraint_checker.evaluate_action("scan", target)
            .map_err(|e| anyhow::anyhow!("Action forbidden: {:?}", e))?;
        self.constraint_checker.evaluate_target(target)
            .map_err(|e| anyhow::anyhow!("Target forbidden: {:?}", e))?;
        self.constraint_checker.evaluate_scan_depth(depth)
            .map_err(|e| anyhow::anyhow!("Scan depth not allowed: {:?}", e))?;
        self.constraint_checker.evaluate_rate_limit(target)
            .map_err(|e| anyhow::anyhow!("Rate limit exceeded: {:?}", e))?;

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

        let token_handle = cancellation_token.map(|tokio_token| {
            // Bridge Tokio cancellation token to tool cancellation token
            let tool_token = crate::tool::request::CancellationToken::new();
            let tool_token_clone = tool_token.clone();
            tokio::spawn(async move {
                tokio_token.cancelled().await;
                tool_token_clone.cancel();
            });
            tool_token.wrap()
        });
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
            cancellation_token: token_handle,
        };

        self.dispatcher
            .dispatch(request)
            .await
            .map_err(|e| anyhow::anyhow!("{:?}", e))
    }

    fn process_findings(&self, response: &ToolResponse) -> Vec<crate::tool::response::Finding> {
        response.findings.clone()
    }

    async fn handle_findings(&mut self, target: &str, findings: Vec<crate::tool::response::Finding>) -> Result<()> {
        #[cfg(feature = "ai-integration")]
        if let Some(ref client) = self.ai_client {
            let finding_values: Vec<serde_json::Value> = findings
                .iter()
                .map(|f| serde_json::to_value(f).unwrap_or_default())
                .collect();

            if let Ok(analysis) = client.analyze_findings_typed(&finding_values).await {
                tracing::debug!(
                    "AI analysis: reassessed_severity={}, confidence={}, exploitability={}",
                    analysis.reassessed_severity,
                    analysis.confidence,
                    analysis.exploitability
                );
            }
        }

        self.process_findings_by_severity(target, &findings).await
    }

    async fn process_findings_by_severity(
        &mut self,
        target: &str,
        findings: &[crate::tool::response::Finding],
    ) -> Result<()> {
        use crate::tool::response::ResponseSeverity;

        let critical_findings: Vec<_> = findings.iter()
            .filter(|f| matches!(f.severity, ResponseSeverity::Critical))
            .collect();

        let high_findings: Vec<_> = findings.iter()
            .filter(|f| matches!(f.severity, ResponseSeverity::High))
            .collect();

        let medium_findings: Vec<_> = findings.iter()
            .filter(|f| matches!(f.severity, ResponseSeverity::Medium))
            .collect();

        let low_findings: Vec<_> = findings.iter()
            .filter(|f| matches!(f.severity, ResponseSeverity::Low))
            .collect();

        let info_findings: Vec<_> = findings.iter()
            .filter(|f| matches!(f.severity, ResponseSeverity::Info))
            .collect();

        if !critical_findings.is_empty() {
            let count = critical_findings.len();
            let alert_severity = crate::types::Severity::Critical;
            let alert = Alert {
                severity: alert_severity,
                title: format!("{} critical findings on {}", count, target),
                message: format!("Detected {} critical severity findings during scan of {}", count, target),
                target: target.to_string(),
                finding_ids: findings.iter().map(|f| f.id.clone()).collect(),
                recommended_actions: vec![
                    "Review immediately".to_string(),
                    "Consider emergency response".to_string(),
                ],
            };
            self.alert_router.send(alert).await?;
        }

        if !high_findings.is_empty() {
            let count = high_findings.len();
            let alert = Alert {
                severity: crate::types::Severity::High,
                title: format!("{} high-severity findings on {}", count, target),
                message: format!("Detected {} high-severity findings during scan of {}", count, target),
                target: target.to_string(),
                finding_ids: high_findings.iter().map(|f| f.id.clone()).collect(),
                recommended_actions: vec!["Review within 24 hours".to_string()],
            };
            self.alert_router.send(alert).await?;
        }

        if !medium_findings.is_empty() {
            let count = medium_findings.len();
            let alert = Alert {
                severity: crate::types::Severity::Medium,
                title: format!("{} medium-severity findings on {}", count, target),
                message: format!("Detected {} medium-severity findings during scan of {}", count, target),
                target: target.to_string(),
                finding_ids: medium_findings.iter().map(|f| f.id.clone()).collect(),
                recommended_actions: vec!["Review in weekly triage".to_string()],
            };
            self.alert_router.send(alert).await?;
        }

        if !low_findings.is_empty() {
            let count = low_findings.len();
            let alert = Alert {
                severity: crate::types::Severity::Low,
                title: format!("{} low-severity findings on {}", count, target),
                message: format!("Detected {} low-severity findings during scan of {}", count, target),
                target: target.to_string(),
                finding_ids: low_findings.iter().map(|f| f.id.clone()).collect(),
                recommended_actions: vec!["Add to remediation backlog".to_string()],
            };
            self.alert_router.send(alert).await?;
        }

        if !info_findings.is_empty() {
            let count = info_findings.len();
            tracing::info!("{} info-level findings on {} - no alert triggered", count, target);
        }

        Ok(())
    }

    pub async fn trigger_scan(&mut self, target: &str, scan_type: &str) -> Result<ToolResponse> {
        tracing::info!("Manually triggered scan for {} (type: {})", target, scan_type);

        let result = self.execute_scan(target, scan_type).await?;

        if let Err(e) = self.memory.store_scan_results(target, &result).await {
            tracing::warn!("Failed to store scan results: {}", e);
        }

        Ok(result)
    }

    pub async fn trigger_event(&mut self, event: SecurityEvent) -> Result<()> {
        tracing::debug!("Event triggered: {:?}", event.event_type());

        let handlers = std::mem::take(&mut self.event_handlers);
        let result = async {
            for handler in handlers.iter() {
                if handler.handles(&event) {
                    handler.handle(&event, self).await?;
                }
            }
            Ok(())
        }
        .await;
        self.event_handlers = handlers;
        result
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

    /// Check if a scheduled target should run, considering last_scan to prevent duplicates in same window
    pub fn should_run_target(
        &self,
        schedule: &str,
        last_scan: Option<DateTime<Utc>>,
        now: &DateTime<Utc>,
    ) -> bool {
        // First check if cron matches now
        if !self.should_run_for(schedule, now) {
            return false;
        }

        // If no last scan, run
        let Some(last) = last_scan else {
            return true;
        };

        // If last scan is in the same minute as now, don't run again (cron triggers at minute granularity)
        if last.minute() == now.minute() && last.hour() == now.hour() && last.date() == now.date() {
            return false;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::constraints::{OperationalConstraints, ForbiddenAction};
    use super::events::ScanCompleteEvent;
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
    async fn test_trigger_event_restores_handlers_on_success() {
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
        let initial_count = agent.event_handlers.len();

        let event = SecurityEvent::ScanComplete(ScanCompleteEvent {
            scan_id: "test-1".to_string(),
            target: "https://example.com".to_string(),
            scan_type: "recon".to_string(),
            timestamp: Utc::now(),
            findings_count: 0,
            severity_counts: std::collections::HashMap::new(),
        });
        agent.trigger_event(event).await.unwrap();

        assert_eq!(agent.event_handlers.len(), initial_count, "Handlers should be restored after successful event");
    }

    #[tokio::test]
    async fn test_trigger_event_restores_handlers_on_error() {
        let mut agent = Agent::new(AgentConfig::default()).await.unwrap();

        struct FailingHandler;
        impl EventHandler for FailingHandler {
            fn handles(&self, _event: &SecurityEvent) -> bool {
                true
            }
            fn handle<'a>(
                &'a self,
                _event: &'a SecurityEvent,
                _agent: &'a mut Agent,
            ) -> Pin<Box<dyn Future<Output = Result<()>> + 'a>> {
                Box::pin(async { Err(anyhow::anyhow!("handler failed")) })
            }
        }
        agent.register_handler(Box::new(FailingHandler));
        let initial_count = agent.event_handlers.len();

        let event = SecurityEvent::ScanComplete(ScanCompleteEvent {
            scan_id: "test-2".to_string(),
            target: "https://example.com".to_string(),
            scan_type: "recon".to_string(),
            timestamp: Utc::now(),
            findings_count: 0,
            severity_counts: std::collections::HashMap::new(),
        });
        let result = agent.trigger_event(event).await;

        assert!(result.is_err(), "Handler error should propagate");
        assert_eq!(agent.event_handlers.len(), initial_count, "Handlers should be restored even after handler error");
    }

    #[test]
    fn test_cron_scheduler_should_run_for_valid_expression() {
        let scheduler = CronScheduler::new();
        let test_time = chrono::NaiveDate::from_ymd_opt(2024, 1, 1)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap()
            .and_utc();
        assert!(scheduler.should_run_for("0 * * * *", &test_time), "At minute 0 should match");
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

    // Phase 2: Testable seams tests
    struct MockDispatcher {
        response: std::sync::Arc<std::sync::Mutex<Option<ToolResponse>>>,
    }

    impl ScanDispatcherTrait for MockDispatcher {
        fn dispatch<'a>(
            &'a self,
            _request: ToolRequest,
        ) -> Pin<Box<dyn Future<Output = std::result::Result<ToolResponse, crate::error::SlapperError>> + Send + 'a>> {
            let response = self.response.lock().unwrap().clone();
            Box::pin(async move {
                response.ok_or_else(|| crate::error::SlapperError::Network("Mock no response".into()))
            })
        }
    }

    struct MockAlertSender {
        sent_alerts: std::sync::Arc<std::sync::Mutex<Vec<Alert>>>,
    }

    impl AlertSenderTrait for MockAlertSender {
        fn send(
            &self,
            alert: Alert,
        ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
            self.sent_alerts.lock().unwrap().push(alert);
            Box::pin(async { Ok(()) })
        }
    }

    #[tokio::test]
    async fn test_mock_dispatcher_success_scan() {
        use tempfile::TempDir;
        let temp_dir = TempDir::new().unwrap();
        let mut config = AgentConfig::default();
        config.memory_dir = temp_dir.path().to_path_buf();

        let mock_response = ToolResponse {
            request_id: "test-1".to_string(),
            tool_id: "mock-tool".to_string(),
            status: crate::tool::response::ResponseStatus::Success,
            results: serde_json::Value::Null,
            metadata: crate::tool::response::ResponseMetadata {
                started_at: chrono::Utc::now(),
                completed_at: chrono::Utc::now(),
                duration_ms: 0,
                targets_scanned: 1,
                findings_count: 0,
            },
            errors: vec![],
            findings: vec![],
        };
        let dispatcher = MockDispatcher {
            response: std::sync::Arc::new(std::sync::Mutex::new(Some(mock_response))),
        };
        let alert_sender = MockAlertSender {
            sent_alerts: std::sync::Arc::new(std::sync::Mutex::new(vec![])),
        };

        let agent = Agent::new_for_test(
            config,
            Box::new(dispatcher),
            Box::new(alert_sender),
        ).await.unwrap();

        // Verify agent was created with test doubles (no network calls)
        assert_eq!(agent.event_handlers.len(), 0);
    }

    #[tokio::test]
    async fn test_mock_dispatcher_failed_scan() {
        use tempfile::TempDir;
        let temp_dir = TempDir::new().unwrap();
        let mut config = AgentConfig::default();
        config.memory_dir = temp_dir.path().to_path_buf();

        let dispatcher = MockDispatcher {
            response: std::sync::Arc::new(std::sync::Mutex::new(None)),
        };
        let alert_sender = MockAlertSender {
            sent_alerts: std::sync::Arc::new(std::sync::Mutex::new(vec![])),
        };

        let agent = Agent::new_for_test(
            config,
            Box::new(dispatcher),
            Box::new(alert_sender),
        ).await.unwrap();

        // Attempt to execute scan, should fail with mock error
        let result = agent.execute_scan("https://example.com", "recon").await;
        assert!(result.is_err());
    }

    // Phase3: Constraint enforcement tests
    #[tokio::test]
    async fn test_manual_scan_blocked_by_forbidden_target() {
        use tempfile::TempDir;
        let temp_dir = TempDir::new().unwrap();
        let mut config = AgentConfig::default();
        config.memory_dir = temp_dir.path().to_path_buf();

        // Add forbidden target to constraints
        let mut constraints = OperationalConstraints::default();
        constraints.do_not_do_list.forbidden_targets.push("https://forbidden.com".to_string());
        config.operational_constraints = Some(constraints);

        let dispatcher = MockDispatcher {
            response: std::sync::Arc::new(std::sync::Mutex::new(None)),
        };
        let alert_sender = MockAlertSender {
            sent_alerts: std::sync::Arc::new(std::sync::Mutex::new(vec![])),
        };

        let agent = Agent::new_for_test(
            config,
            Box::new(dispatcher),
            Box::new(alert_sender),
        ).await.unwrap();

        // Attempt to scan forbidden target, should be blocked
        let result = agent.execute_scan("https://forbidden.com", "recon").await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        // Error may be ActionForbidden (since evaluate_action checks target allowance too)
        assert!(err.to_string().contains("forbidden") || err.to_string().contains("Forbidden"));
    }

    #[tokio::test]
    async fn test_manual_scan_blocked_by_action() {
        use tempfile::TempDir;
        let temp_dir = TempDir::new().unwrap();
        let mut config = AgentConfig::default();
        config.memory_dir = temp_dir.path().to_path_buf();

        // Forbid "scan" action
        let mut constraints = OperationalConstraints::default();
        constraints.do_not_do_list.forbidden_actions.push(ForbiddenAction::new("scan", "Testing"));
        config.operational_constraints = Some(constraints);

        let dispatcher = MockDispatcher {
            response: std::sync::Arc::new(std::sync::Mutex::new(None)),
        };
        let alert_sender = MockAlertSender {
            sent_alerts: std::sync::Arc::new(std::sync::Mutex::new(vec![])),
        };

        let agent = Agent::new_for_test(
            config,
            Box::new(dispatcher),
            Box::new(alert_sender),
        ).await.unwrap();

        let result = agent.execute_scan("https://example.com", "recon").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Action forbidden"));
    }

    // Phase 4: Idempotent scheduling tests
    #[test]
    fn test_should_run_target_first_time() {
        let scheduler = CronScheduler::new();
        // Fixed time with minute 0 to match cron expression
        let now = chrono::NaiveDate::from_ymd_opt(2024, 1, 1)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap()
            .and_utc();
        let schedule = "0 * * * *"; // Minute 0
        let last_scan = None;

        assert!(scheduler.should_run_target(schedule, last_scan, &now));
    }

    #[test]
    fn test_should_run_target_same_minute() {
        let scheduler = CronScheduler::new();
        let now = Utc::now();
        let schedule = "* * * * *";
        let last_scan = Some(now); // Same time

        assert!(!scheduler.should_run_target(schedule, last_scan, &now));
    }

    #[test]
    fn test_should_run_target_next_minute() {
        let scheduler = CronScheduler::new();
        let now = Utc::now();
        let schedule = "* * * * *";
        let last_scan = Some(now - chrono::Duration::minutes(1));

        assert!(scheduler.should_run_target(schedule, last_scan, &now));
    }

    #[tokio::test]
    async fn test_scheduled_scan_idempotent() {
        use tempfile::TempDir;
        use crate::tool::response::ToolResponse;
        use directories::ProjectDirs;
        use chrono::TimeZone;

        let temp_dir = TempDir::new().unwrap();
        let mut config = AgentConfig::default();
        config.memory_dir = temp_dir.path().to_path_buf();

        // Use portfolio path within allowed base config directory
        let base_dir = ProjectDirs::from("com", "slapper", "slapper")
            .map(|d| d.config_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("~/.config/slapper"));
        let portfolio_path = base_dir.join("test_portfolio.json");
        config.portfolio_path = Some(portfolio_path.clone());

        let target_config = crate::agent::portfolio::TargetConfig::new("https://example.com");
        let mut target_config = target_config; // Make mutable
        // Schedule matches current minute to ensure trigger
        let current_minute = chrono::Utc::now().minute();
        target_config.schedule = Some(format!("{} * * * *", current_minute));

        let mut portfolio = TargetPortfolio::load_from_file(&portfolio_path).unwrap();
        portfolio.add_target("https://example.com".to_string(), target_config);
        portfolio.save().unwrap();

        let dispatcher = MockDispatcher {
            response: std::sync::Arc::new(std::sync::Mutex::new(
                Some(ToolResponse::success("req-1", "pipeline", serde_json::json!({"status": "success"})))
            )),
        };
        let alert_sender = MockAlertSender {
            sent_alerts: std::sync::Arc::new(std::sync::Mutex::new(vec![])),
        };

        let mut agent = Agent::new_for_test(
            config,
            Box::new(dispatcher),
            Box::new(alert_sender),
        ).await.unwrap();

        // First poll: should run (fixed time with minute 0)
        // We can't control Utc::now() in process_scheduled_scans, so we rely on the mock
        agent.process_scheduled_scans().await.unwrap();
        // Check that last_scan is set in-memory
        let portfolio = &agent.portfolio;
        let target = portfolio.get_target("https://example.com").unwrap();
        assert!(target.last_scan.is_some());
    }
}
