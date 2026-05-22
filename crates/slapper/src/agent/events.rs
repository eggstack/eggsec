//! Event system for the security agent.
//!
//! Provides event handling for security events like scan completions,
//! new findings, threshold violations, and external webhooks.

use rustc_hash::FxHashMap;
use std::future::Future;
use std::pin::Pin;

use anyhow::Result;
use chrono::{DateTime, Utc};

use crate::output::AgentFinding;

#[derive(Clone, Debug, PartialEq)]
pub enum EventType {
    ScanComplete,
    FindingDetected,
    ThresholdExceeded,
    ScheduleTriggered,
    ExternalWebhook,
    ManualTrigger,
}

#[derive(Clone, Debug)]
pub struct ScanCompleteEvent {
    pub scan_id: String,
    pub target: String,
    pub scan_type: String,
    pub timestamp: DateTime<Utc>,
    pub findings_count: usize,
    pub severity_counts: FxHashMap<String, usize>,
}

#[derive(Clone, Debug)]
pub struct FindingDetectedEvent {
    pub finding: AgentFinding,
    pub target: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct ThresholdEvent {
    pub threshold_type: String,
    pub target: String,
    pub current_value: f64,
    pub threshold_value: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct ScheduleEvent {
    pub schedule_id: String,
    pub target: String,
    pub cron_expression: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct WebhookEvent {
    pub source: String,
    pub payload: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub enum SecurityEvent {
    ScanComplete(ScanCompleteEvent),
    FindingDetected(FindingDetectedEvent),
    ThresholdExceeded(ThresholdEvent),
    ScheduleTriggered(ScheduleEvent),
    ExternalWebhook(WebhookEvent),
}

impl SecurityEvent {
    pub fn event_type(&self) -> EventType {
        match self {
            SecurityEvent::ScanComplete(_) => EventType::ScanComplete,
            SecurityEvent::FindingDetected(_) => EventType::FindingDetected,
            SecurityEvent::ThresholdExceeded(_) => EventType::ThresholdExceeded,
            SecurityEvent::ScheduleTriggered(_) => EventType::ScheduleTriggered,
            SecurityEvent::ExternalWebhook(_) => EventType::ExternalWebhook,
        }
    }

    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            SecurityEvent::ScanComplete(e) => e.timestamp,
            SecurityEvent::FindingDetected(e) => e.timestamp,
            SecurityEvent::ThresholdExceeded(e) => e.timestamp,
            SecurityEvent::ScheduleTriggered(e) => e.timestamp,
            SecurityEvent::ExternalWebhook(e) => e.timestamp,
        }
    }
}

pub trait EventHandler: Send + Sync {
    fn handles(&self, event: &SecurityEvent) -> bool;

    fn handle<'a>(
        &'a self,
        event: &'a SecurityEvent,
        agent: &'a mut crate::agent::Agent,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + 'a>>;
}

pub struct FnEventHandler<F> {
    pub handler: F,
}

impl<F> FnEventHandler<F>
where
    F: Fn(SecurityEvent, &mut crate::agent::Agent) -> Result<()> + Send + Sync + 'static,
{
    pub fn new(handler: F) -> Self {
        Self { handler }
    }
}

impl<F> EventHandler for FnEventHandler<F>
where
    F: Fn(SecurityEvent, &mut crate::agent::Agent) -> Result<()> + Send + Sync + 'static,
{
    fn handles(&self, _event: &SecurityEvent) -> bool {
        true
    }

    fn handle<'a>(
        &'a self,
        event: &'a SecurityEvent,
        agent: &'a mut crate::agent::Agent,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + 'a>> {
        let handler = &self.handler;
        Box::pin(async move { handler(event.clone(), agent) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type() {
        let event = SecurityEvent::ScanComplete(ScanCompleteEvent {
            scan_id: "test-1".to_string(),
            target: "https://example.com".to_string(),
            scan_type: "recon".to_string(),
            timestamp: Utc::now(),
            findings_count: 5,
            severity_counts: FxHashMap::default(),
        });

        assert_eq!(event.event_type(), EventType::ScanComplete);
    }
}
