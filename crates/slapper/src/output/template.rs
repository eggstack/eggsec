//! Report templating system using Handlebars.
//!
//! Provides customizable report generation with support for compliance
//! templates (PCI-DSS, SOC2, HIPAA) and custom templates.

use handlebars::{Handlebars, RenderError};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::error::{Result, SlapperError};
use crate::output::report::{ReportMetadata, SeverityCounts};

#[derive(Debug, Clone)]
pub struct ReportTemplateEngine {
    registry: Handlebars<'static>,
    custom_templates: FxHashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceTemplate {
    pub name: String,
    pub standard: ComplianceStandard,
    pub sections: Vec<TemplateSection>,
    pub styling: TemplateStyling,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ComplianceStandard {
    PCIDSS,
    SOC2,
    HIPAA,
    GDPR,
    OWASP,
    NIST,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateSection {
    pub id: String,
    pub title: String,
    pub content: String,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateStyling {
    pub primary_color: String,
    pub secondary_color: String,
    pub logo_path: Option<String>,
    pub footer_text: Option<String>,
}

impl Default for TemplateStyling {
    fn default() -> Self {
        Self {
            primary_color: "#1a73e8".to_string(),
            secondary_color: "#f8f9fa".to_string(),
            logo_path: None,
            footer_text: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateRenderContext {
    pub metadata: ReportMetadata,
    pub findings: Vec<TemplateFinding>,
    pub summary: TemplateSummary,
    pub compliance: Option<ComplianceReport>,
    pub custom_data: FxHashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateFinding {
    pub id: String,
    pub title: String,
    pub severity: String,
    pub description: String,
    pub location: String,
    pub remediation: String,
    pub cvss: Option<f32>,
    pub cwe_ids: Vec<String>,
    pub evidence: Vec<TemplateEvidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateEvidence {
    pub type_field: String,
    pub content: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateSummary {
    pub total_findings: usize,
    pub critical_count: usize,
    pub high_count: usize,
    pub medium_count: usize,
    pub low_count: usize,
    pub info_count: usize,
    pub risk_score: f64,
    pub scan_duration_seconds: u64,
    pub targets_scanned: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceReport {
    pub standard: String,
    pub requirements: Vec<ComplianceRequirement>,
    pub overall_status: ComplianceStatus,
    pub gaps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceRequirement {
    pub id: String,
    pub description: String,
    pub status: ComplianceStatus,
    pub evidence_refs: Vec<String>,
    pub remediation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ComplianceStatus {
    Compliant,
    NonCompliant,
    PartiallyCompliant,
    NotApplicable,
}

impl ReportTemplateEngine {
    pub fn new() -> Self {
        let mut registry = Handlebars::new();
        registry.set_strict_mode(true);

        registry
            .register_template_string("executive", EXECUTIVE_TEMPLATE)
            .unwrap();
        registry
            .register_template_string("technical", TECHNICAL_TEMPLATE)
            .unwrap();
        registry
            .register_template_string("developer", DEVELOPER_TEMPLATE)
            .unwrap();
        registry
            .register_template_string("compliance", COMPLIANCE_TEMPLATE)
            .unwrap();

        Self {
            registry,
            custom_templates: FxHashMap::default(),
        }
    }

    pub fn register_template(&mut self, name: &str, template: &str) -> Result<()> {
        self.registry
            .register_template_string(name, template)
            .map_err(|e| SlapperError::Template(e.to_string()))?;
        self.custom_templates
            .insert(name.to_string(), template.to_string());
        Ok(())
    }

    pub fn register_template_from_file(&mut self, name: &str, path: &Path) -> Result<()> {
        let content = fs::read_to_string(path)
            .map_err(|e| SlapperError::Io(e.to_string()))?;
        self.register_template(name, &content)?;
        Ok(())
    }

    pub fn render(&self, template_name: &str, context: &TemplateRenderContext) -> Result<String> {
        self.registry
            .render(template_name, context)
            .map_err(|e| SlapperError::Template(e.to_string()))
    }

    pub fn render_with_styling(
        &self,
        template_name: &str,
        context: &TemplateRenderContext,
        styling: &TemplateStyling,
    ) -> Result<String> {
        let mut styled_context = context.clone();
        if let Ok(styling_value) = serde_json::to_value(styling) {
            styled_context.custom_data.insert("styling".to_string(), styling_value);
        }
        self.render(template_name, &styled_context)
    }

    pub fn list_templates(&self) -> Vec<String> {
        let mut templates = vec![
            "executive".to_string(),
            "technical".to_string(),
            "developer".to_string(),
            "compliance".to_string(),
        ];
        templates.extend(self.custom_templates.keys().cloned());
        templates
    }

    pub fn get_compliance_template(
        &self,
        standard: ComplianceStandard,
    ) -> ComplianceTemplate {
        match standard {
            ComplianceStandard::PCIDSS => pcidss_template(),
            ComplianceStandard::SOC2 => soc2_template(),
            ComplianceStandard::HIPAA => hipaa_template(),
            ComplianceStandard::GDPR => gdpr_template(),
            ComplianceStandard::OWASP => owasp_template(),
            ComplianceStandard::NIST => nist_template(),
        }
    }
}

impl Default for ReportTemplateEngine {
    fn default() -> Self {
        Self::new()
    }
}

const EXECUTIVE_TEMPLATE: &str = r#"
<!DOCTYPE html>
<html>
<head>
    <title>Security Scan Report - {{metadata.target}}</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 40px; }
        .header { background: {{custom_data.styling.primary_color}}; color: white; padding: 20px; }
        .summary { display: grid; grid-template-columns: repeat(5, 1fr); gap: 10px; margin: 20px 0; }
        .severity-box { padding: 15px; text-align: center; border-radius: 5px; }
        .critical { background: #dc3545; color: white; }
        .high { background: #fd7e14; color: white; }
        .medium { background: #ffc107; color: black; }
        .low { background: #0dcaf0; color: black; }
        .info { background: #6c757d; color: white; }
        .finding { border: 1px solid #ddd; padding: 15px; margin: 10px 0; border-radius: 5px; }
    </style>
</head>
<body>
    <div class="header">
        <h1>Security Scan Report</h1>
        <p>Target: {{metadata.target}}</p>
        <p>Scan Date: {{metadata.scan_date}}</p>
        <p>Scan Type: {{metadata.scan_type}}</p>
    </div>

    <h2>Executive Summary</h2>
    <div class="summary">
        <div class="severity-box critical">{{summary.critical_count}} Critical</div>
        <div class="severity-box high">{{summary.high_count}} High</div>
        <div class="severity-box medium">{{summary.medium_count}} Medium</div>
        <div class="severity-box low">{{summary.low_count}} Low</div>
        <div class="severity-box info">{{summary.info_count}} Info</div>
    </div>

    <p><strong>Risk Score:</strong> {{summary.risk_score}}</p>
    <p><strong>Total Findings:</strong> {{summary.total_findings}}</p>
    <p><strong>Scan Duration:</strong> {{summary.scan_duration_seconds}} seconds</p>

    {{#if compliance}}
    <h2>Compliance Status</h2>
    <p><strong>Standard:</strong> {{compliance.standard}}</p>
    <p><strong>Overall Status:</strong> {{compliance.overall_status}}</p>
    {{/if}}

    <h2>Key Findings</h2>
    {{#each findings}}
    {{#if (or (eq severity "critical") (eq severity "high"))}}
    <div class="finding">
        <h3>[{{severity}}] {{title}}</h3>
        <p><strong>Location:</strong> {{location}}</p>
        <p>{{description}}</p>
        <p><strong>Remediation:</strong> {{remediation}}</p>
    </div>
    {{/if}}
    {{/each}}
</body>
</html>
"#;

const TECHNICAL_TEMPLATE: &str = r#"
<!DOCTYPE html>
<html>
<head>
    <title>Technical Security Report - {{metadata.target}}</title>
    <style>
        body { font-family: 'Courier New', monospace; margin: 40px; }
        .header { border-bottom: 3px solid #333; padding-bottom: 10px; }
        .finding { border-left: 4px solid; padding: 15px; margin: 15px 0; background: #f5f5f5; }
        .critical { border-color: #dc3545; }
        .high { border-color: #fd7e14; }
        .medium { border-color: #ffc107; }
        .low { border-color: #0dcaf0; }
        .info { border-color: #6c757d; }
        .meta { color: #666; font-size: 0.9em; }
    </style>
</head>
<body>
    <div class="header">
        <h1>Technical Security Report</h1>
        <div class="meta">
            <p>Target: {{metadata.target}}</p>
            <p>Date: {{metadata.scan_date}}</p>
            <p>Type: {{metadata.scan_type}}</p>
        </div>
    </div>

    <h2>Summary</h2>
    <pre>
Total: {{summary.total_findings}}
Critical: {{summary.critical_count}}
High: {{summary.high_count}}
Medium: {{summary.medium_count}}
Low: {{summary.low_count}}
Info: {{summary.info_count}}
Risk Score: {{summary.risk_score}}
    </pre>

    <h2>Detailed Findings</h2>
    {{#each findings}}
    <div class="finding {{severity}}">
        <h3>{{title}}</h3>
        <p class="meta">ID: {{id}} | Severity: {{severity}}</p>
        <p><strong>Location:</strong> <code>{{location}}</code></p>
        <p><strong>Description:</strong></p>
        <p>{{description}}</p>
        <p><strong>Remediation:</strong></p>
        <p>{{remediation}}</p>
        {{#if cwe_ids}}
        <p><strong>CWE:</strong> {{cwe_ids}}</p>
        {{/if}}
        {{#each evidence}}
        <pre>{{content}}</pre>
        {{/each}}
    </div>
    {{/each}}
</body>
</html>
"#;

const DEVELOPER_TEMPLATE: &str = r#"
<!DOCTYPE html>
<html>
<head>
    <title>Developer Security Report - {{metadata.target}}</title>
    <style>
        body { font-family: system-ui; margin: 40px; max-width: 900px; }
        .finding { border: 1px solid #e0e0e0; border-radius: 8px; padding: 20px; margin: 15px 0; }
        .severity { display: inline-block; padding: 4px 12px; border-radius: 4px; color: white; font-size: 0.85em; text-transform: uppercase; }
        .critical { background: #dc3545; }
        .high { background: #fd7e14; }
        .medium { background: #ffc107; color: black; }
        .low { background: #0dcaf0; }
        .info { background: #6c757d; }
        .code { background: #f4f4f4; padding: 10px; border-radius: 4px; font-family: monospace; overflow-x: auto; }
        .steps { counter-reset: step; }
        .step { margin: 10px 0; padding-left: 30px; position: relative; }
        .step::before { content: counter(step); counter-increment: step; position: absolute; left: 0; }
    </style>
</head>
<body>
    <h1>Developer Security Report</h1>
    <p><strong>Target:</strong> {{metadata.target}}</p>

    <h2>Findings Requiring Immediate Attention</h2>
    {{#each findings}}
    {{#if (or (eq severity "critical") (eq severity "high"))}}
    <div class="finding">
        <span class="severity {{severity}}">{{severity}}</span>
        <h3>{{title}}</h3>
        <p><strong>File/Location:</strong> <code>{{location}}</code></p>
        <p>{{description}}</p>
        <h4>Remediation Steps:</h4>
        <div class="steps">
            {{#each (split remediation "//")}}
            <div class="step">{{this}}</div>
            {{/each}}
        </div>
        {{#if cwe_ids}}
        <p><strong>Related CWE:</strong> {{cwe_ids}}</p>
        {{/if}}
    </div>
    {{/if}}
    {{/each}}

    <h2>All Findings</h2>
    {{#each findings}}
    <div class="finding">
        <span class="severity {{severity}}">{{severity}}</span>
        <h3>{{title}}</h3>
        <p><code>{{location}}</code></p>
        <p>{{description}}</p>
    </div>
    {{/each}}
</body>
</html>
"#;

const COMPLIANCE_TEMPLATE: &str = r#"
<!DOCTYPE html>
<html>
<head>
    <title>Compliance Report - {{metadata.target}}</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 40px; }
        .status { padding: 20px; margin: 10px 0; border-radius: 5px; }
        .compliant { background: #d4edda; border: 1px solid #c3e6cb; }
        .non-compliant { background: #f8d7da; border: 1px solid #f5c6cb; }
        .partial { background: #fff3cd; border: 1px solid #ffeeba; }
        .na { background: #e2e3e5; border: 1px solid #d6d8db; }
        .requirement { padding: 15px; margin: 10px 0; border-left: 4px solid #ccc; }
    </style>
</head>
<body>
    <h1>Compliance Assessment Report</h1>
    <p><strong>Target:</strong> {{metadata.target}}</p>
    <p><strong>Assessment Date:</strong> {{metadata.scan_date}}</p>
    <p><strong>Standard:</strong> {{compliance.standard}}</p>

    <h2>Overall Status</h2>
    <div class="status {{compliance.overall_status}}">
        <strong>{{compliance.overall_status}}</strong>
    </div>

    <h2>Requirements Assessment</h2>
    {{#each compliance.requirements}}
    <div class="requirement">
        <h3>{{id}}: {{description}}</h3>
        <p><strong>Status:</strong> <span class="{{status}}">{{status}}</span></p>
        {{#if remediation}}
        <p><strong>Remediation:</strong> {{remediation}}</p>
        {{/if}}
    </div>
    {{/each}}

    {{#if compliance.gaps}}
    <h2>Identified Gaps</h2>
    <ul>
    {{#each compliance.gaps}}
    <li>{{this}}</li>
    {{/each}}
    </ul>
    {{/if}}
</body>
</html>
"#;

fn pcidss_template() -> ComplianceTemplate {
    ComplianceTemplate {
        name: "PCI-DSS Compliance".to_string(),
        standard: ComplianceStandard::PCIDSS,
        sections: vec![
            TemplateSection {
                id: "1".to_string(),
                title: "Install and maintain network security controls".to_string(),
                content: " firewalls, VLANs".to_string(),
                required: true,
            },
            TemplateSection {
                id: "2".to_string(),
                title: "Apply secure configurations to all system components".to_string(),
                content: "default passwords, unnecessary services".to_string(),
                required: true,
            },
        ],
        styling: TemplateStyling::default(),
    }
}

fn soc2_template() -> ComplianceTemplate {
    ComplianceTemplate {
        name: "SOC 2 Compliance".to_string(),
        standard: ComplianceStandard::SOC2,
        sections: vec![
            TemplateSection {
                id: "CC1".to_string(),
                title: "Control Environment".to_string(),
                content: "Security policies and procedures".to_string(),
                required: true,
            },
        ],
        styling: TemplateStyling::default(),
    }
}

fn hipaa_template() -> ComplianceTemplate {
    ComplianceTemplate {
        name: "HIPAA Compliance".to_string(),
        standard: ComplianceStandard::HIPAA,
        sections: vec![TemplateSection {
            id: "164".to_string(),
            title: "Security and Privacy of PHI".to_string(),
            content: "Administrative, physical, and technical safeguards".to_string(),
            required: true,
        }],
        styling: TemplateStyling::default(),
    }
}

fn gdpr_template() -> ComplianceTemplate {
    ComplianceTemplate {
        name: "GDPR Compliance".to_string(),
        standard: ComplianceStandard::GDPR,
        sections: vec![TemplateSection {
            id: "Art-32".to_string(),
            title: "Security of Processing".to_string(),
            content: "Appropriate technical and organizational measures".to_string(),
            required: true,
        }],
        styling: TemplateStyling::default(),
    }
}

fn owasp_template() -> ComplianceTemplate {
    ComplianceTemplate {
        name: "OWASP Compliance".to_string(),
        standard: ComplianceStandard::OWASP,
        sections: vec![TemplateSection {
            id: "A1".to_string(),
            title: "Injection".to_string(),
            content: "SQL, NoSQL, OS, LDAP injection".to_string(),
            required: true,
        }],
        styling: TemplateStyling::default(),
    }
}

fn nist_template() -> ComplianceTemplate {
    ComplianceTemplate {
        name: "NIST Compliance".to_string(),
        standard: ComplianceStandard::NIST,
        sections: vec![TemplateSection {
            id: "ID".to_string(),
            title: "Identification and Authentication".to_string(),
            content: "Identify and authenticate organizational users".to_string(),
            required: true,
        }],
        styling: TemplateStyling::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_engine_new() {
        let engine = ReportTemplateEngine::new();
        let templates = engine.list_templates();
        assert!(templates.contains(&"executive".to_string()));
        assert!(templates.contains(&"technical".to_string()));
    }

    #[test]
    fn test_register_custom_template() {
        let mut engine = ReportTemplateEngine::new();
        let result = engine.register_template("custom", "<html>{{test}}</html>");
        assert!(result.is_ok());
        let templates = engine.list_templates();
        assert!(templates.contains(&"custom".to_string()));
    }

    #[test]
    fn test_compliance_templates() {
        let engine = ReportTemplateEngine::new();
        let pcidss = engine.get_compliance_template(ComplianceStandard::PCIDSS);
        assert_eq!(pcidss.standard, ComplianceStandard::PCIDSS);

        let soc2 = engine.get_compliance_template(ComplianceStandard::SOC2);
        assert_eq!(soc2.standard, ComplianceStandard::SOC2);

        let hipaa = engine.get_compliance_template(ComplianceStandard::HIPAA);
        assert_eq!(hipaa.standard, ComplianceStandard::HIPAA);
    }

    #[test]
    fn test_template_styling_default() {
        let styling = TemplateStyling::default();
        assert_eq!(styling.primary_color, "#1a73e8");
        assert_eq!(styling.secondary_color, "#f8f9fa");
    }
}
