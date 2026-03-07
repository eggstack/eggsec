#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExportFormat {
    Json,
    Csv,
    Html,
    Markdown,
    Sarif,
    Junit,
}

impl ExportFormat {
    pub fn from_string(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "json" => Some(Self::Json),
            "csv" => Some(Self::Csv),
            "html" => Some(Self::Html),
            "md" | "markdown" => Some(Self::Markdown),
            "sarif" => Some(Self::Sarif),
            "junit" | "xml" => Some(Self::Junit),
            _ => None,
        }
    }

    pub fn all() -> Vec<(&'static str, &'static str)> {
        vec![
            ("json", "JSON - Full data export"),
            ("csv", "CSV - Spreadsheet compatible"),
            ("html", "HTML - Web report"),
            ("md", "Markdown - Documentation"),
            ("sarif", "SARIF - Dev integration"),
            ("junit", "JUnit - CI/CD integration"),
        ]
    }
}

pub struct CsvExporter;

impl CsvExporter {
    pub fn export_findings(findings: &[FindingCsv]) -> String {
        if findings.is_empty() {
            return String::new();
        }

        let mut output = String::new();
        output.push_str("Severity,Target,Path,Description,CVE,Remediation\n");

        for f in findings {
            output.push_str(&format!(
                "{},{},{},{},{},{}\n",
                escape_csv(&f.severity),
                escape_csv(&f.target),
                escape_csv(&f.path),
                escape_csv(&f.description),
                escape_csv(f.cve.as_deref().unwrap_or("")),
                escape_csv(f.remediation.as_deref().unwrap_or("")),
            ));
        }

        output
    }

    pub fn export_ports(ports: &[PortCsv]) -> String {
        if ports.is_empty() {
            return String::new();
        }

        let mut output = String::new();
        output.push_str("Host,Port,Protocol,Service,Version,State\n");

        for p in ports {
            output.push_str(&format!(
                "{},{},{},{},{},{}\n",
                escape_csv(&p.host),
                p.port,
                escape_csv(&p.protocol),
                escape_csv(p.service.as_deref().unwrap_or("")),
                escape_csv(p.version.as_deref().unwrap_or("")),
                escape_csv(&p.state),
            ));
        }

        output
    }

    pub fn export_endpoints(endpoints: &[EndpointCsv]) -> String {
        if endpoints.is_empty() {
            return String::new();
        }

        let mut output = String::new();
        output.push_str("URL,Method,Status,Content-Type,Content-Length\n");

        for e in endpoints {
            output.push_str(&format!(
                "{},{},{},{},{}\n",
                escape_csv(&e.url),
                escape_csv(&e.method),
                e.status,
                escape_csv(e.content_type.as_deref().unwrap_or("")),
                e.content_length,
            ));
        }

        output
    }
}

fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingCsv {
    pub severity: String,
    pub target: String,
    pub path: String,
    pub description: String,
    pub cve: Option<String>,
    pub remediation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortCsv {
    pub host: String,
    pub port: u16,
    pub protocol: String,
    pub service: Option<String>,
    pub version: Option<String>,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointCsv {
    pub url: String,
    pub method: String,
    pub status: u16,
    pub content_type: Option<String>,
    pub content_length: u64,
}
