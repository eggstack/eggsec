use serde::{Deserialize, Serialize};

pub use crate::types::OutputFormat;

pub struct CsvExporter;

impl CsvExporter {
    pub fn export_findings(findings: &[FindingCsv]) -> String {
        if findings.is_empty() {
            return String::new();
        }

        use std::fmt::Write;
        let mut output = String::new();
        writeln!(output, "Severity,Target,Path,Description,CVE,Remediation").unwrap();

        for f in findings {
            writeln!(
                output,
                "{},{},{},{},{},{}",
                super::escape::escape_csv(&f.severity),
                super::escape::escape_csv(&f.target),
                super::escape::escape_csv(&f.path),
                super::escape::escape_csv(&f.description),
                super::escape::escape_csv(f.cve.as_deref().unwrap_or("")),
                super::escape::escape_csv(f.remediation.as_deref().unwrap_or("")),
            )
            .unwrap();
        }

        output
    }

    pub fn export_ports(ports: &[PortCsv]) -> String {
        if ports.is_empty() {
            return String::new();
        }

        use std::fmt::Write;
        let mut output = String::new();
        writeln!(output, "Host,Port,Protocol,Service,Version,State").unwrap();

        for p in ports {
            writeln!(
                output,
                "{},{},{},{},{},{}",
                super::escape::escape_csv(&p.host),
                p.port,
                super::escape::escape_csv(&p.protocol),
                super::escape::escape_csv(p.service.as_deref().unwrap_or("")),
                super::escape::escape_csv(p.version.as_deref().unwrap_or("")),
                super::escape::escape_csv(&p.state),
            )
            .unwrap();
        }

        output
    }

    pub fn export_endpoints(endpoints: &[EndpointCsv]) -> String {
        if endpoints.is_empty() {
            return String::new();
        }

        use std::fmt::Write;
        let mut output = String::new();
        writeln!(output, "URL,Method,Status,Content-Type,Content-Length").unwrap();

        for e in endpoints {
            writeln!(
                output,
                "{},{},{},{},{}",
                super::escape::escape_csv(&e.url),
                super::escape::escape_csv(&e.method),
                e.status,
                super::escape::escape_csv(e.content_type.as_deref().unwrap_or("")),
                e.content_length,
            )
            .unwrap();
        }

        output
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
