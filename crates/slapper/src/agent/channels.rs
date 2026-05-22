use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Alert {
    pub severity: crate::types::Severity,
    pub title: String,
    pub message: String,
    pub target: String,
    pub finding_ids: Vec<String>,
    pub recommended_actions: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WebhookConfig {
    pub url: String,
    pub secret: Option<crate::types::SensitiveString>,
    pub headers: FxHashMap<String, String>,
}

impl Default for WebhookConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            secret: None,
            headers: FxHashMap::default(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmailChannel {
    pub smtp_host: String,
    pub smtp_port: u16,
    pub from: String,
    pub to: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SlackChannel {
    pub webhook_url: String,
    pub channel: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PagerDutyChannel {
    pub routing_key: crate::types::SensitiveString,
    pub severity: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AlertChannel {
    Webhook(WebhookConfig),
    Email(EmailChannel),
    Slack(SlackChannel),
    PagerDuty(PagerDutyChannel),
}

#[derive(Debug, Clone)]
pub struct AggregatedAlert {
    pub total_count: usize,
    pub severity_counts: FxHashMap<String, usize>,
    pub all_finding_ids: Vec<String>,
    pub affected_targets: Vec<String>,
    pub max_severity: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct ScanReport {
    pub id: String,
    pub target: String,
    pub generated_at: chrono::DateTime<chrono::Utc>,
    pub summary: ReportSummary,
    pub findings: Vec<crate::tool::response::Finding>,
    pub alerts: Vec<Alert>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ReportSummary {
    pub total_findings: usize,
    pub critical_count: usize,
    pub high_count: usize,
    pub medium_count: usize,
    pub low_count: usize,
    pub info_count: usize,
    pub risk_score: f64,
}

#[derive(Debug, Clone, Copy)]
pub enum EscalationLevel {
    Warning,
    Urgent,
    Critical,
}

pub struct AlertTemplate {
    pub name: String,
    pub slack_template: SlackTemplate,
    pub pagerduty_template: PagerDutyTemplate,
    pub email_template: EmailTemplate,
}

#[derive(Debug, Clone)]
pub struct SlackTemplate {
    pub title_format: String,
    pub body_format: String,
    pub color_by_severity: FxHashMap<String, String>,
}

impl SlackTemplate {
    pub fn default_templates() -> Self {
        let mut colors = FxHashMap::default();
        colors.insert("critical".to_string(), "#dc3545".to_string());
        colors.insert("high".to_string(), "#fd7e14".to_string());
        colors.insert("medium".to_string(), "#ffc107".to_string());
        colors.insert("low".to_string(), "#0dcaf0".to_string());
        colors.insert("info".to_string(), "#6c757d".to_string());

        Self {
            title_format: "[{{severity}}] {{title}}".to_string(),
            body_format: "Target: {{target}}\n{{message}}\nFindings: {{finding_count}}".to_string(),
            color_by_severity: colors,
        }
    }

    pub fn format(&self, alert: &Alert, finding_count: usize) -> SlackFormattedAlert {
        let title = self
            .title_format
            .replace("{{severity}}", &alert.severity.as_str().to_uppercase())
            .replace("{{title}}", &alert.title);

        let body = self
            .body_format
            .replace("{{target}}", &alert.target)
            .replace("{{message}}", &alert.message)
            .replace("{{finding_count}}", &finding_count.to_string());

        let color = self
            .color_by_severity
            .get(alert.severity.as_str())
            .cloned()
            .unwrap_or_else(|| "#6c757d".to_string());

        SlackFormattedAlert { title, body, color }
    }
}

#[derive(Debug, Clone)]
pub struct SlackFormattedAlert {
    pub title: String,
    pub body: String,
    pub color: String,
}

#[derive(Debug, Clone)]
pub struct PagerDutyTemplate {
    pub severity_mapping: FxHashMap<String, String>,
}

impl PagerDutyTemplate {
    pub fn default_template() -> Self {
        let mut mapping = FxHashMap::default();
        mapping.insert("critical".to_string(), "critical".to_string());
        mapping.insert("high".to_string(), "error".to_string());
        mapping.insert("medium".to_string(), "warning".to_string());
        mapping.insert("low".to_string(), "info".to_string());
        mapping.insert("info".to_string(), "info".to_string());

        Self {
            severity_mapping: mapping,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EmailTemplate {
    pub subject_format: String,
    pub body_format: String,
}

impl EmailTemplate {
    pub fn default_template() -> Self {
        Self {
            subject_format: "[Slapper] {{severity}}: {{title}}".to_string(),
            body_format: "Security Alert: {{title}}\n\n\
                Target: {{target}}\n\
                Severity: {{severity}}\n\n\
                {{message}}\n\n\
                Recommended Actions:\n\
                {{actions}}"
                .to_string(),
        }
    }

    pub fn format(&self, alert: &Alert) -> EmailFormattedAlert {
        let subject = self
            .subject_format
            .replace("{{severity}}", &alert.severity.as_str().to_uppercase())
            .replace("{{title}}", &alert.title);

        let body = self
            .body_format
            .replace("{{target}}", &alert.target)
            .replace("{{title}}", &alert.title)
            .replace("{{message}}", &alert.message)
            .replace("{{severity}}", &alert.severity.as_str().to_uppercase())
            .replace("{{actions}}", &alert.recommended_actions.join("\n"));

        EmailFormattedAlert { subject, body }
    }
}

#[derive(Debug, Clone)]
pub struct EmailFormattedAlert {
    pub subject: String,
    pub body: String,
}
