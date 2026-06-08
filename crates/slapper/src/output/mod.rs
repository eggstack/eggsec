//! Output and report generation module
//!
//! Provides report generation, format conversion, trend analysis, and scan session management.
//!
//! Most output types and renderers live in the `slapper-output` crate and are
//! re-exported here for backward compatibility. Modules that depend on
//! engine-internal types (`pdf`, `report`, `report_summary`, `run_manifest`,
//! `attack_graph`) remain in this crate.

// Re-export everything from slapper-output (agent, ai_schema, baseline, convert,
// csv, dedup, diff, escape, html, junit, markdown, sarif, schedule, session,
// trend and all their public types).
pub use slapper_output::*;

// Local modules that depend on engine-internal types and could not be moved.
#[cfg(feature = "advanced-hunting")]
pub mod attack_graph;
pub mod pdf;
pub mod report;
pub mod report_summary;
pub mod run_manifest;

// Re-export local module types for backward compatibility.
#[cfg(feature = "advanced-hunting")]
pub use attack_graph::{
    AttackGraph, AttackGraphBuilder, EdgeType, GraphCluster, GraphEdge, GraphNode, NodeType,
};
pub use pdf::{PdfConfig, PdfGenerator};
pub use report::{Report, ReportMetadata, ReportTemplate, SeverityCounts};
pub use report_summary::ReportSummary;
pub use run_manifest::RunManifest;

/// Extension traits that add `with_report` convenience methods to builder types
/// from slapper-output. These methods consume engine-internal `PipelineReport`.
pub mod extensions {
    use crate::pipeline::PipelineReport;

    /// Extension trait for [`SarifBuilder`] to populate from a [`PipelineReport`].
    pub trait SarifBuilderExt {
        fn with_report(self, report: &PipelineReport) -> Self;
    }

    impl SarifBuilderExt for super::SarifBuilder {
        fn with_report(mut self, report: &PipelineReport) -> Self {
            for port in &report.open_ports {
                if port.status == "open" {
                    let rule_id = format!("PORT-{}", port.port);
                    self = self.add_rule(
                        &rule_id,
                        &format!("Open Port {}", port.port),
                        "note",
                        &format!(
                            "Open port {} detected on {}",
                            port.port, report.target
                        ),
                    );
                    self = self.add_result(
                        &rule_id,
                        "note",
                        &format!("Port {} is open", port.port),
                        &format!("{}:{}", report.target, port.port),
                    );
                }
            }

            for service in &report.services {
                let rule_id = format!("SERVICE-{}", service.service);
                self = self.add_rule(
                    &rule_id,
                    &service.service.clone(),
                    "note",
                    &format!(
                        "Detected {} service version {}",
                        service.service,
                        service.version.as_deref().unwrap_or("unknown")
                    ),
                );
                self = self.add_result(
                    &rule_id,
                    "note",
                    &format!(
                        "Service: {} {}",
                        service.service,
                        service.version.as_deref().unwrap_or("")
                    ),
                    &format!("{}:{}", report.target, service.port),
                );
            }

            if !report.endpoints.is_empty() {
                self = self.add_rule(
                    "ENDPOINT",
                    "Discovered Endpoint",
                    "note",
                    "Found endpoint during scan",
                );
            }

            self
        }
    }

    /// Extension trait for [`JUnitBuilder`] to populate from a [`PipelineReport`].
    pub trait JUnitBuilderExt {
        fn with_report(self, report: &PipelineReport) -> Self;
    }

    impl JUnitBuilderExt for super::JUnitBuilder {
        fn with_report(self, report: &PipelineReport) -> Self {
            let suite_name = format!("slapper-scan-{}", report.target);
            let mut builder = self;

            for port in &report.open_ports {
                if port.status == "open" {
                    builder = builder.add_test_case(
                        &suite_name,
                        &format!("port_{}_open", port.port),
                        "port_scan",
                        0.0,
                        super::JUnitTestResult::Passed,
                    );
                }
            }

            for service in &report.services {
                let service_name = format!(
                    "{}_v{}",
                    service.service,
                    service.version.as_deref().unwrap_or("unknown")
                );
                builder = builder.add_test_case(
                    &suite_name,
                    &service_name,
                    "fingerprint",
                    0.0,
                    super::JUnitTestResult::Passed,
                );
            }

            for endpoint in &report.endpoints {
                let test_name = format!(
                    "{} {} - {}",
                    endpoint.path, endpoint.status_code, endpoint.status_text
                );
                let result = if endpoint.status_code >= 400 {
                    super::JUnitTestResult::Failed {
                        message: format!(
                            "Endpoint returned error status: {}",
                            endpoint.status_code
                        ),
                        failure_type: "HttpError".to_string(),
                        text: Some(format!("Path: {}", endpoint.path)),
                    }
                } else {
                    super::JUnitTestResult::Passed
                };
                builder = builder.add_test_case(
                    &suite_name,
                    &test_name,
                    "endpoints",
                    endpoint.response_time_ms as f64 / 1000.0,
                    result,
                );
            }

            builder
        }
    }
}
