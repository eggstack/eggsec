use serde::{Deserialize, Serialize};
use std::fmt::Write;
use tokio::io::{AsyncWrite, AsyncWriteExt, BufWriter};

/// Output format for CSV export.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, Default,
)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    #[default]
    Pretty,
    Json,
    Compact,
    Html,
    Csv,
    Sarif,
    Junit,
    Markdown,
}

pub struct CsvExporter;

impl CsvExporter {
    pub fn export_findings(findings: &[FindingCsv]) -> Result<String, std::fmt::Error> {
        if findings.is_empty() {
            return Ok(String::new());
        }

        let mut output = String::new();
        writeln!(output, "Severity,Target,Path,Description,CVE,Remediation")?;

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
            )?;
        }

        Ok(output)
    }

    pub async fn export_findings_streaming(
        findings: &[FindingCsv],
        writer: &mut (impl AsyncWrite + Unpin),
    ) -> std::io::Result<()> {
        let mut buf = BufWriter::new(writer);

        if findings.is_empty() {
            return buf.flush().await;
        }

        buf.write_all(b"Severity,Target,Path,Description,CVE,Remediation\n")
            .await?;

        for f in findings {
            buf.write_all(super::escape::escape_csv(&f.severity).as_bytes())
                .await?;
            buf.write_all(b",").await?;
            buf.write_all(super::escape::escape_csv(&f.target).as_bytes())
                .await?;
            buf.write_all(b",").await?;
            buf.write_all(super::escape::escape_csv(&f.path).as_bytes())
                .await?;
            buf.write_all(b",").await?;
            buf.write_all(super::escape::escape_csv(&f.description).as_bytes())
                .await?;
            buf.write_all(b",").await?;
            buf.write_all(super::escape::escape_csv(f.cve.as_deref().unwrap_or("")).as_bytes())
                .await?;
            buf.write_all(b",").await?;
            buf.write_all(
                super::escape::escape_csv(f.remediation.as_deref().unwrap_or("")).as_bytes(),
            )
            .await?;
            buf.write_all(b"\n").await?;
        }

        buf.flush().await
    }

    pub fn export_ports(ports: &[PortCsv]) -> Result<String, std::fmt::Error> {
        if ports.is_empty() {
            return Ok(String::new());
        }

        let mut output = String::new();
        writeln!(output, "Host,Port,Protocol,Service,Version,State")?;

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
            )?;
        }

        Ok(output)
    }

    pub fn export_endpoints(endpoints: &[EndpointCsv]) -> Result<String, std::fmt::Error> {
        if endpoints.is_empty() {
            return Ok(String::new());
        }

        let mut output = String::new();
        writeln!(output, "URL,Method,Status,Content-Type,Content-Length")?;

        for e in endpoints {
            writeln!(
                output,
                "{},{},{},{},{}",
                super::escape::escape_csv(&e.url),
                super::escape::escape_csv(&e.method),
                e.status,
                super::escape::escape_csv(e.content_type.as_deref().unwrap_or("")),
                e.content_length,
            )?;
        }

        Ok(output)
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
