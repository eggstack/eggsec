use serde::{Deserialize, Serialize};

use super::executor::StageResult;
use crate::loadtest::metrics::LoadTestResults;
use crate::output::escape::{escape_csv, escape_html, escape_xml};
use crate::output::RunManifest;
use crate::scanner::endpoints::EndpointResult;
use crate::scanner::fingerprint::ServiceFingerprint;
use crate::scanner::ports::PortResult;

#[derive(Debug, Serialize, Deserialize)]
pub struct PipelineReport {
    pub target: String,
    pub total_duration_ms: u64,
    pub stage_results: Vec<StageResult>,
    pub open_ports: Vec<PortResult>,
    pub services: Vec<ServiceFingerprint>,
    pub endpoints: Vec<EndpointResult>,
    #[serde(skip)]
    pub checkpoint_error: Option<String>,
    /// Run manifest providing structured metadata for regression workflows.
    /// Populated after pipeline execution completes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manifest: Option<RunManifest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vuln_assessment: Option<crate::vuln::VulnAssessment>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load_test_results: Option<LoadTestResults>,
    #[cfg(feature = "web-proxy")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub web_proxy_report: Option<crate::proxy::intercept::types::WebProxySessionReport>,
}

impl std::fmt::Display for PipelineReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Security Assessment Report")?;
        writeln!(f, "target: {}", truncate(&self.target, 60))?;
        writeln!(f, "duration: {}ms", self.total_duration_ms)?;

        writeln!(f, "stages")?;
        for result in &self.stage_results {
            let status = if result.success { "✓" } else { "✗" };
            writeln!(f, "\t{} {} {}ms", status, result.stage, result.duration_ms)?;
        }

        if !self.open_ports.is_empty() {
            writeln!(f, "open ports")?;
            for port in self.open_ports.iter().take(10) {
                writeln!(f, "\t{}/{}\t{}", port.port, port.status, port.service)?;
            }
            if self.open_ports.len() > 10 {
                writeln!(f, "\t... and {} more", self.open_ports.len() - 10)?;
            }
        }

        if !self.services.is_empty() {
            writeln!(f, "services")?;
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
            writeln!(f, "interesting endpoints")?;
            for endpoint in interesting_endpoints.iter().take(10) {
                writeln!(f, "\t[!] {}", endpoint.path)?;
            }
            if interesting_endpoints.len() > 10 {
                writeln!(f, "\t... and {} more", interesting_endpoints.len() - 10)?;
            }
        } else if !self.endpoints.is_empty() {
            writeln!(f, "endpoints: {} found", self.endpoints.len())?;
        }

        if let Some(ref vuln) = self.vuln_assessment {
            writeln!(f, "vulnerability assessment")?;
            for line in &vuln.summary {
                writeln!(f, "\t{}", line)?;
            }
        }

        if let Some(ref load) = self.load_test_results {
            writeln!(f, "load test")?;
            writeln!(
                f,
                "\t{} requests, {:.2} rps, {:.2}ms mean latency (p95: {:.2}ms)",
                load.total_requests,
                load.requests_per_second,
                load.latency_mean_ms,
                load.latency_p95_ms
            )?;
            writeln!(
                f,
                "\t{} successful, {} failed",
                load.successful_requests, load.failed_requests
            )?;
        }

        #[cfg(feature = "web-proxy")]
        if let Some(ref proxy) = self.web_proxy_report {
            writeln!(f, "web proxy intercept")?;
            writeln!(
                f,
                "\t{} flows (HTTPS: {}, HTTP: {}, Blocked: {}, Redacted: {})",
                proxy.flows.len(),
                proxy.https_intercepted,
                proxy.http_logged,
                proxy.blocked,
                proxy.redacted
            )?;
            if !proxy.ws_sessions.is_empty() {
                let ws_msgs: usize = proxy.ws_sessions.iter().map(|s| s.messages.len()).sum();
                writeln!(f, "\t{} WebSocket sessions ({} messages)", proxy.ws_sessions.len(), ws_msgs)?;
            }
            if !proxy.http2_sessions.is_empty() {
                let h2_streams: usize = proxy.http2_sessions.iter().map(|s| s.streams.len()).sum();
                writeln!(f, "\t{} HTTP/2 sessions ({} streams)", proxy.http2_sessions.len(), h2_streams)?;
            }
            if !proxy.grpc_sessions.is_empty() {
                let grpc_calls: usize = proxy.grpc_sessions.iter().map(|s| s.calls.len()).sum();
                writeln!(f, "\t{} gRPC sessions ({} calls)", proxy.grpc_sessions.len(), grpc_calls)?;
            }
        }

        Ok(())
    }
}

impl PipelineReport {
    pub fn has_failures(&self) -> bool {
        self.stage_results.iter().any(|r| !r.success)
    }

    pub fn first_failed_stage(&self) -> Option<&StageResult> {
        self.stage_results.iter().find(|r| !r.success)
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

    if let Some(ref load) = report.load_test_results {
        html.push_str("<div class='section'>\n<h2>Load Test Results</h2>\n<table>\n");
        html.push_str("<tr><th>Metric</th><th>Value</th></tr>\n");
        html.push_str(&format!(
            "<tr><td>Total Requests</td><td>{}</td></tr>\n",
            load.total_requests
        ));
        html.push_str(&format!(
            "<tr><td>Successful</td><td>{}</td></tr>\n",
            load.successful_requests
        ));
        html.push_str(&format!(
            "<tr><td>Failed</td><td>{}</td></tr>\n",
            load.failed_requests
        ));
        html.push_str(&format!(
            "<tr><td>Requests/sec</td><td>{:.2}</td></tr>\n",
            load.requests_per_second
        ));
        html.push_str(&format!(
            "<tr><td>Mean Latency</td><td>{:.2}ms</td></tr>\n",
            load.latency_mean_ms
        ));
        html.push_str(&format!(
            "<tr><td>P95 Latency</td><td>{:.2}ms</td></tr>\n",
            load.latency_p95_ms
        ));
        html.push_str(&format!(
            "<tr><td>P99 Latency</td><td>{:.2}ms</td></tr>\n",
            load.latency_p99_ms
        ));
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

    if let Some(ref load) = report.load_test_results {
        csv.push_str("Load Test Results\n");
        csv.push_str("Metric,Value\n");
        csv.push_str(&format!("Total Requests,{}\n", load.total_requests));
        csv.push_str(&format!("Successful,{}\n", load.successful_requests));
        csv.push_str(&format!("Failed,{}\n", load.failed_requests));
        csv.push_str(&format!("Requests/sec,{:.2}\n", load.requests_per_second));
        csv.push_str(&format!("Mean Latency (ms),{:.2}\n", load.latency_mean_ms));
        csv.push_str(&format!("P95 Latency (ms),{:.2}\n", load.latency_p95_ms));
        csv.push_str(&format!("P99 Latency (ms),{:.2}\n", load.latency_p99_ms));
    }

    Ok(csv)
}

pub fn generate_markdown(report: &PipelineReport) -> crate::error::Result<String> {
    let mut md = String::new();
    md.push_str("# Security Assessment Report\n\n");
    md.push_str(&format!("**Target:** `{}`\n\n", &report.target));
    md.push_str(&format!("**Duration:** {}ms\n\n", report.total_duration_ms));

    md.push_str("## Stages\n\n");
    md.push_str("| Stage | Status | Duration |\n");
    md.push_str("|-------|--------|----------|\n");
    for result in &report.stage_results {
        let status = if result.success { "✓" } else { "✗" };
        md.push_str(&format!(
            "| {} | {} | {}ms |\n",
            result.stage, status, result.duration_ms
        ));
    }
    md.push('\n');

    if !report.open_ports.is_empty() {
        md.push_str("## Open Ports\n\n");
        md.push_str("| Port | Status | Service |\n");
        md.push_str("|------|--------|--------|\n");
        for port in &report.open_ports {
            md.push_str(&format!(
                "| {} | {} | {} |\n",
                port.port, port.status, &port.service
            ));
        }
        md.push('\n');
    }

    if !report.services.is_empty() {
        md.push_str("## Services\n\n");
        md.push_str("| Port | Service | Product | Version |\n");
        md.push_str("|------|---------|---------|--------|\n");
        for service in &report.services {
            let product = service.product.as_deref().unwrap_or("-");
            let version = service.version.as_deref().unwrap_or("-");
            md.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                service.port, &service.service, product, version
            ));
        }
        md.push('\n');
    }

    let interesting_endpoints: Vec<_> = report.endpoints.iter().filter(|e| e.interesting).collect();
    if !interesting_endpoints.is_empty() {
        md.push_str("## Interesting Endpoints\n\n");
        for endpoint in &interesting_endpoints {
            let size = endpoint
                .content_length
                .map(|l| l.to_string())
                .unwrap_or_else(|| "-".to_string());
            md.push_str(&format!(
                "- `{}` (status: {}, size: {})\n",
                &endpoint.path, endpoint.status_code, size
            ));
        }
        md.push('\n');
    }

    if let Some(ref vuln) = report.vuln_assessment {
        md.push_str("## Vulnerability Assessment\n\n");
        for line in &vuln.summary {
            md.push_str(&format!("{}\n", line));
        }
        md.push('\n');
    }

    if let Some(ref load) = report.load_test_results {
        md.push_str("## Load Test Results\n\n");
        md.push_str(&format!("- **Total Requests:** {}\n", load.total_requests));
        md.push_str(&format!("- **Successful:** {}\n", load.successful_requests));
        md.push_str(&format!("- **Failed:** {}\n", load.failed_requests));
        md.push_str(&format!(
            "- **Requests/sec:** {:.2}\n",
            load.requests_per_second
        ));
        md.push_str(&format!(
            "- **Mean Latency:** {:.2}ms\n",
            load.latency_mean_ms
        ));
        md.push_str(&format!(
            "- **P95 Latency:** {:.2}ms\n",
            load.latency_p95_ms
        ));
        md.push_str(&format!(
            "- **P99 Latency:** {:.2}ms\n",
            load.latency_p99_ms
        ));
    }

    Ok(md)
}
