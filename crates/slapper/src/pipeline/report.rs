use serde::{Deserialize, Serialize};

use super::executor::StageResult;
use crate::output::escape::{escape_html, escape_xml};
use crate::scanner::endpoints::EndpointResult;
use crate::scanner::fingerprint::ServiceFingerprint;
use crate::scanner::ports::PortResult;

fn escape_csv(s: &str) -> String {
    let formula_chars = ['=', '+', '-', '@', '\t', '\r'];
    let starts_with_formula = s
        .chars()
        .next()
        .map(|c| formula_chars.contains(&c))
        .unwrap_or(false);
    if s.contains(',') || s.contains('"') || s.contains('\n') || starts_with_formula {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PipelineReport {
    pub target: String,
    pub total_duration_ms: u64,
    pub stage_results: Vec<StageResult>,
    pub open_ports: Vec<PortResult>,
    pub services: Vec<ServiceFingerprint>,
    pub endpoints: Vec<EndpointResult>,
}

impl std::fmt::Display for PipelineReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Security Assessment Report")?;
        writeln!(f, "target: {}", truncate(&self.target, 60))?;
        writeln!(f, "duration: {}ms", self.total_duration_ms)?;

        let _ = writeln!(f, "stages");
        for result in &self.stage_results {
            let status = if result.success { "✓" } else { "✗" };
            writeln!(f, "\t{} {} {}ms", status, result.stage, result.duration_ms)?;
        }

        if !self.open_ports.is_empty() {
            let _ = writeln!(f, "open ports");
            for port in self.open_ports.iter().take(10) {
                writeln!(f, "\t{}/{}\t{}", port.port, port.status, port.service)?;
            }
            if self.open_ports.len() > 10 {
                writeln!(f, "\t... and {} more", self.open_ports.len() - 10)?;
            }
        }

        if !self.services.is_empty() {
            let _ = writeln!(f, "services");
            for service in self.services.iter().take(5) {
                let product = service.product.as_deref().unwrap_or("-");
                writeln!(f, "\t{}\t{}\t{}", service.port, service.service, product)?;
            }
            if self.services.len() > 5 {
                writeln!(f, "\t... and {} more", self.services.len() - 5)?;
            }
        }

        let interesting_endpoints: Vec<_> =
            self.endpoints.iter().filter(|e| e.interesting).collect();
        if !interesting_endpoints.is_empty() {
            let _ = writeln!(f, "interesting endpoints");
            for endpoint in interesting_endpoints.iter().take(10) {
                writeln!(f, "\t[!] {}", endpoint.path)?;
            }
            if interesting_endpoints.len() > 10 {
                writeln!(f, "\t... and {} more", interesting_endpoints.len() - 10)?;
            }
        } else if !self.endpoints.is_empty() {
            writeln!(f, "endpoints: {} found", self.endpoints.len())?;
        }

        Ok(())
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    let cleaned: String = s
        .chars()
        .filter(|c| c.is_ascii_graphic() || *c == ' ')
        .collect();
    if cleaned.len() > max_len {
        format!("{}...", &cleaned[..max_len.saturating_sub(3)])
    } else {
        format!("{:<width$}", cleaned, width = max_len)
    }
}

pub fn generate_html(report: &PipelineReport) -> crate::error::Result<String> {
    let mut html = String::new();
    html.push_str("<!DOCTYPE html>\n<html>\n<head>\n");
    html.push_str("<title>Security Assessment Report</title>\n");
    html.push_str("<style>\n");
    html.push_str("body { font-family: Arial, sans-serif; margin: 40px; background: #1a1a2e; color: #eee; }\n");
    html.push_str("h1 { color: #00d9ff; }\n");
    html.push_str("h2 { color: #00ff88; border-bottom: 1px solid #333; padding-bottom: 10px; }\n");
    html.push_str(
        ".section { background: #16213e; padding: 20px; margin: 20px 0; border-radius: 8px; }\n",
    );
    html.push_str("table { width: 100%; border-collapse: collapse; }\n");
    html.push_str("th, td { padding: 10px; text-align: left; border-bottom: 1px solid #333; }\n");
    html.push_str("th { color: #00d9ff; }\n");
    html.push_str(".success { color: #00ff88; }\n");
    html.push_str(".fail { color: #ff4444; }\n");
    html.push_str(".interesting { color: #ffaa00; font-weight: bold; }\n");
    html.push_str("</style>\n</head>\n<body>\n");

    html.push_str("<h1>Security Assessment Report</h1>\n");
    html.push_str(&format!(
        "<p><strong>Target:</strong> {}</p>\n",
        escape_html(&report.target)
    ));
    html.push_str(&format!(
        "<p><strong>Duration:</strong> {}ms</p>\n",
        report.total_duration_ms
    ));

    html.push_str("<div class='section'>\n<h2>Stages Completed</h2>\n<table>\n");
    html.push_str("<tr><th>Stage</th><th>Duration</th><th>Status</th></tr>\n");
    for result in &report.stage_results {
        let status = if result.success {
            "<span class='success'>Success</span>"
        } else {
            "<span class='fail'>Failed</span>"
        };
        html.push_str(&format!(
            "<tr><td>{}</td><td>{}ms</td><td>{}</td></tr>\n",
            escape_html(&format!("{}", result.stage)),
            result.duration_ms,
            status
        ));
    }
    html.push_str("</table>\n</div>\n");

    if !report.open_ports.is_empty() {
        html.push_str("<div class='section'>\n<h2>Open Ports</h2>\n<table>\n");
        html.push_str("<tr><th>Port</th><th>Status</th><th>Service</th></tr>\n");
        for port in &report.open_ports {
            html.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td></tr>\n",
                port.port,
                escape_xml(&port.status),
                escape_html(&port.service)
            ));
        }
        html.push_str("</table>\n</div>\n");
    }

    if !report.services.is_empty() {
        html.push_str("<div class='section'>\n<h2>Services Identified</h2>\n<table>\n");
        html.push_str("<tr><th>Port</th><th>Service</th><th>Product</th><th>Version</th></tr>\n");
        for service in &report.services {
            html.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>\n",
                service.port,
                escape_html(&service.service),
                escape_html(service.product.as_deref().unwrap_or("-")),
                escape_html(service.version.as_deref().unwrap_or("-"))
            ));
        }
        html.push_str("</table>\n</div>\n");
    }

    let interesting_endpoints: Vec<_> = report.endpoints.iter().filter(|e| e.interesting).collect();
    if !interesting_endpoints.is_empty() {
        html.push_str("<div class='section'>\n<h2>Interesting Endpoints</h2>\n<table>\n");
        html.push_str("<tr><th>Path</th><th>Status</th><th>Size</th></tr>\n");
        for endpoint in interesting_endpoints {
            let size = endpoint
                .content_length
                .map(|l| l.to_string())
                .unwrap_or_else(|| "-".to_string());
            html.push_str(&format!(
                "<tr><td class='interesting'>{}</td><td>{}</td><td>{}</td></tr>\n",
                escape_html(&endpoint.path),
                endpoint.status_code,
                size
            ));
        }
        html.push_str("</table>\n</div>\n");
    }

    html.push_str("</body>\n</html>\n");
    Ok(html)
}

pub fn generate_csv(report: &PipelineReport) -> crate::error::Result<String> {
    let mut csv = String::new();

    csv.push_str("Security Assessment Report\n");
    csv.push_str(&format!("Target,{}\n", escape_csv(&report.target)));
    csv.push_str(&format!("Duration (ms),{}\n\n", report.total_duration_ms));

    if !report.open_ports.is_empty() {
        csv.push_str("Open Ports\n");
        csv.push_str("Port,Status,Service\n");
        for port in &report.open_ports {
            csv.push_str(&format!(
                "{},{},{}\n",
                port.port,
                port.status,
                escape_csv(&port.service)
            ));
        }
        csv.push('\n');
    }

    if !report.services.is_empty() {
        csv.push_str("Services\n");
        csv.push_str("Port,Service,Product,Version\n");
        for service in &report.services {
            csv.push_str(&format!(
                "{},{},{},{}\n",
                service.port,
                escape_csv(&service.service),
                escape_csv(service.product.as_deref().unwrap_or("-")),
                escape_csv(service.version.as_deref().unwrap_or("-"))
            ));
        }
        csv.push('\n');
    }

    if !report.endpoints.is_empty() {
        csv.push_str("Endpoints\n");
        csv.push_str("Path,Status,Size,Interesting\n");
        for endpoint in &report.endpoints {
            let size = endpoint
                .content_length
                .map(|l| l.to_string())
                .unwrap_or_else(|| "-".to_string());
            csv.push_str(&format!(
                "{},{},{},{}\n",
                escape_csv(&endpoint.path),
                endpoint.status_code,
                size,
                endpoint.interesting
            ));
        }
    }

    Ok(csv)
}
