//! Alert routing system for the security agent.
//!
//! Routes alerts to configured channels (webhooks, email, Slack, PagerDuty)
//! with rate limiting and deduplication.

pub use crate::agent::channels::{
    AggregatedAlert, Alert, AlertChannel, AlertTemplate, EmailChannel, EmailFormattedAlert,
    EmailTemplate, EscalationLevel, PagerDutyChannel, PagerDutyTemplate, ReportSummary,
    ScanReport, SlackChannel, SlackFormattedAlert, SlackTemplate, WebhookConfig,
};
pub use crate::agent::routing::AlertRouter;
