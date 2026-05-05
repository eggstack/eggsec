use chrono::Timelike;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::types::Severity;

pub mod routing;

pub use routing::AlertRouter;

pub use crate::agent::channels::{
    AggregatedAlert, Alert, AlertChannel, AlertTemplate, EmailChannel, EmailFormattedAlert,
    EmailTemplate, EscalationLevel, PagerDutyChannel, PagerDutyTemplate, ReportSummary, ScanReport,
    SlackChannel, SlackFormattedAlert, SlackTemplate, WebhookConfig,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeBasedRouting {
    pub time_ranges: Vec<TimeRange>,
    pub channel_assignments: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub start_hour: u8,
    pub end_hour: u8,
    pub timezone: String,
}

impl TimeRange {
    pub fn is_active(&self) -> bool {
        let now = chrono::Local::now().naive_local();
        let current_hour = now.hour() as u8;
        current_hour >= self.start_hour && current_hour < self.end_hour
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRoutingRules {
    pub by_severity: HashMap<Severity, Vec<String>>,
    pub by_time: Option<TimeBasedRouting>,
    pub by_vulnerability_type: HashMap<String, Vec<String>>,
    #[serde(skip)]
    channel_cache: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl AlertRoutingRules {
    pub fn new() -> Self {
        Self {
            by_severity: HashMap::new(),
            by_time: None,
            by_vulnerability_type: HashMap::new(),
            channel_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_severity_routing(mut self, severity: Severity, channels: Vec<String>) -> Self {
        self.by_severity.insert(severity, channels);
        self
    }

    pub fn with_time_routing(mut self, time_routing: TimeBasedRouting) -> Self {
        self.by_time = Some(time_routing);
        self
    }

    pub fn with_vulnerability_routing(mut self, vuln_type: String, channels: Vec<String>) -> Self {
        self.by_vulnerability_type.insert(vuln_type, channels);
        self
    }

    pub fn get_channels_for_alert(
        &self,
        severity: &Severity,
        vulnerability_type: Option<&str>,
    ) -> Vec<String> {
        let mut channels = Vec::new();

        if let Some(time_routing) = &self.by_time {
            for time_range in &time_routing.time_ranges {
                if time_range.is_active() {
                    if let Some(time_channels) = time_routing
                        .channel_assignments
                        .get(&format!("{:02}:00", time_range.start_hour))
                    {
                        channels.extend(time_channels.clone());
                    }
                }
            }
        }

        if let Some(sev_channels) = self.by_severity.get(severity) {
            channels.extend(sev_channels.clone());
        }

        if let Some(vuln_type) = vulnerability_type {
            if let Some(vuln_channels) = self.by_vulnerability_type.get(vuln_type) {
                channels.extend(vuln_channels.clone());
            }
        }

        channels.sort();
        channels.dedup();
        channels
    }

    pub fn clear_cache(&self) {
        self.channel_cache.write().clear();
    }
}

impl Default for AlertRoutingRules {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_severity_routing() {
        let rules = AlertRoutingRules::new()
            .with_severity_routing(Severity::Critical, vec!["pagerduty".to_string()])
            .with_severity_routing(Severity::High, vec!["slack".to_string()]);

        let channels = rules.get_channels_for_alert(&Severity::Critical, None);
        assert!(channels.contains(&"pagerduty".to_string()));
        assert!(!channels.contains(&"slack".to_string()));
    }

    #[test]
    fn test_vulnerability_routing() {
        let rules = AlertRoutingRules::new().with_vulnerability_routing(
            "SQL Injection".to_string(),
            vec!["sql_injection_channel".to_string()],
        );

        let channels = rules.get_channels_for_alert(&Severity::High, Some("SQL Injection"));
        assert!(channels.contains(&"sql_injection_channel".to_string()));
    }

    #[test]
    fn test_combined_routing() {
        let rules = AlertRoutingRules::new()
            .with_severity_routing(Severity::Critical, vec!["critical_channel".to_string()])
            .with_vulnerability_routing("XSS".to_string(), vec!["xss_channel".to_string()]);

        let channels = rules.get_channels_for_alert(&Severity::Critical, Some("XSS"));
        assert!(channels.contains(&"critical_channel".to_string()));
        assert!(channels.contains(&"xss_channel".to_string()));
    }
}
