use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::DiffSummary;
use crate::pipeline::report::PipelineReport;
use crate::probe::{ProbeIntent, ProbeRisk};
use crate::types::Severity;

/// Structured summary of a single assessment or defense-lab run.
///
/// `RunManifest` is a metadata envelope that wraps run-level provenance
/// (identity, scope, profile, feature flags) with observations, findings,
/// and an optional baseline diff. It is designed for regression-oriented
/// workflows where runs must be comparable and reproducible.
///
/// The manifest is integrated into the pipeline output path via
/// [`PipelineReport::manifest`]. Use [`RunManifest::from_report`] to
/// construct one from a completed pipeline run.
///
/// # Schema Direction
///
/// A baseline run produces a manifest with `baseline_id: None`.
/// Subsequent runs reference the baseline via `baseline_id` and populate
/// `diff_summary` with the delta.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunManifest {
    /// Schema version for forward compatibility (e.g. "1.0.0").
    pub schema_version: String,
    /// Unique identifier for this run (UUID or similar).
    pub run_id: String,
    /// When the run started.
    pub started_at: DateTime<Utc>,
    /// When the run ended.
    pub ended_at: DateTime<Utc>,
    /// Slapper version used for this run.
    pub slapper_version: String,
    /// Target scope specification (e.g. "localhost", "10.0.0.0/8").
    pub target_scope: String,
    /// Profile name (e.g. "defense-lab", "waf-regression").
    pub profile: String,
    /// Probe intent categories exercised during the run.
    pub probe_intents: Vec<ProbeIntent>,
    /// Maximum risk tier allowed for this run.
    pub risk_budget: ProbeRisk,
    /// Feature flags enabled for this run.
    pub feature_flags: Vec<String>,
    /// Raw probe observations (response codes, latencies, payloads).
    pub observations: Vec<serde_json::Value>,
    /// Interpreted findings produced by this run.
    pub findings: Vec<serde_json::Value>,
    /// Paths or identifiers of output artifacts (JSON, HTML, CSV, etc.).
    pub artifacts: Vec<String>,
    /// Reference to the baseline run this run is compared against, if any.
    pub baseline_id: Option<String>,
    /// Summary of differences against the baseline, if computed.
    pub diff_summary: Option<DiffSummary>,
}

impl RunManifest {
    /// Create a new manifest with the current timestamp and default schema version.
    pub fn new(
        run_id: String,
        slapper_version: String,
        target_scope: String,
        profile: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            schema_version: "1.0.0".to_string(),
            run_id,
            started_at: now,
            ended_at: now,
            slapper_version,
            target_scope,
            profile,
            probe_intents: Vec::new(),
            risk_budget: ProbeRisk::SafeActive,
            feature_flags: Vec::new(),
            observations: Vec::new(),
            findings: Vec::new(),
            artifacts: Vec::new(),
            baseline_id: None,
            diff_summary: None,
        }
    }

    /// Mark the run as ended at the current time.
    pub fn finish(&mut self) {
        self.ended_at = Utc::now();
    }

    /// Set the baseline reference and diff summary for regression comparison.
    pub fn with_baseline(mut self, baseline_id: String, diff_summary: DiffSummary) -> Self {
        self.baseline_id = Some(baseline_id);
        self.diff_summary = Some(diff_summary);
        self
    }

    /// Create a manifest from a completed pipeline report.
    ///
    /// Populates `started_at` from now minus the run duration, `ended_at` as now,
    /// and derives `observations` from open ports/services/endpoints.
    /// `findings` are derived from interesting endpoints (as a simple heuristic).
    pub fn from_report(report: &PipelineReport, profile: &str, risk_budget: ProbeRisk) -> Self {
        let now = Utc::now();
        let started_at = now - chrono::Duration::milliseconds(report.total_duration_ms as i64);

        let run_id = uuid::Uuid::new_v4().to_string();

        let observations: Vec<serde_json::Value> = report
            .open_ports
            .iter()
            .map(|p| {
                serde_json::json!({
                    "type": "port",
                    "port": p.port,
                    "status": &p.status,
                    "service": &p.service,
                })
            })
            .chain(report.services.iter().map(|s| {
                serde_json::json!({
                    "type": "service",
                    "port": s.port,
                    "service": &s.service,
                    "product": s.product,
                    "version": s.version,
                })
            }))
            .chain(report.endpoints.iter().filter(|e| e.interesting).map(|e| {
                serde_json::json!({
                    "type": "endpoint",
                    "path": &e.path,
                    "status_code": e.status_code,
                    "content_length": e.content_length,
                })
            }))
            .collect();

        let probe_intents: Vec<ProbeIntent> = report
            .stage_results
            .iter()
            .filter(|r| r.success)
            .map(|r| r.stage.to_probe_intent())
            .collect();

        let feature_flags: Vec<String> = report
            .stage_results
            .iter()
            .map(|r| {
                if r.success {
                    format!("stage:{}", r.stage)
                } else {
                    format!("stage:{}:failed", r.stage)
                }
            })
            .collect();

        Self {
            schema_version: "1.0.0".to_string(),
            run_id,
            started_at,
            ended_at: now,
            slapper_version: env!("CARGO_PKG_VERSION").to_string(),
            target_scope: report.target.clone(),
            profile: profile.to_string(),
            probe_intents,
            risk_budget,
            feature_flags,
            observations,
            findings: Vec::new(),
            artifacts: Vec::new(),
            baseline_id: None,
            diff_summary: None,
        }
    }

    /// Populate findings from the report's interesting endpoints as a basic heuristic.
    /// Each interesting endpoint becomes a finding observation.
    pub fn populate_findings_from_report(&mut self, report: &PipelineReport) {
        self.findings = report
            .endpoints
            .iter()
            .filter(|e| e.interesting)
            .map(|e| {
                serde_json::json!({
                    "title": format!("Interesting endpoint: {}", e.path),
                    "severity": Severity::Info.as_str(),
                    "category": "endpoint_discovery",
                    "description": format!("Endpoint {} returned status {}", e.path, e.status_code),
                    "location": e.path,
                })
            })
            .collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_serializes_to_json() {
        let manifest = RunManifest::new(
            "run-001".to_string(),
            "0.1.0".to_string(),
            "localhost".to_string(),
            "defense-lab".to_string(),
        );
        let json = serde_json::to_string(&manifest).expect("should serialize");
        assert!(json.contains("\"run_id\":\"run-001\""));
        assert!(json.contains("\"schema_version\":\"1.0.0\""));
        assert!(json.contains("\"profile\":\"defense-lab\""));
    }

    #[test]
    fn manifest_deserializes_from_json() {
        let manifest = RunManifest::new(
            "run-002".to_string(),
            "0.1.0".to_string(),
            "10.0.0.0/8".to_string(),
            "waf-regression".to_string(),
        );
        let json = serde_json::to_string(&manifest).expect("should serialize");
        let decoded: RunManifest = serde_json::from_str(&json).expect("should deserialize");
        assert_eq!(decoded.run_id, "run-002");
        assert_eq!(decoded.target_scope, "10.0.0.0/8");
        assert_eq!(decoded.profile, "waf-regression");
        assert!(decoded.baseline_id.is_none());
        assert!(decoded.diff_summary.is_none());
    }

    #[test]
    fn manifest_with_baseline() {
        let diff_summary = DiffSummary {
            total_new: 1,
            total_resolved: 0,
            total_escalated: 0,
            total_deescalated: 0,
            net_change: 1,
        };
        let manifest = RunManifest::new(
            "run-003".to_string(),
            "0.1.0".to_string(),
            "localhost".to_string(),
            "defense-lab".to_string(),
        )
        .with_baseline("baseline-001".to_string(), diff_summary);

        assert_eq!(manifest.baseline_id.as_deref(), Some("baseline-001"));
        assert!(manifest.diff_summary.is_some());
        assert_eq!(manifest.diff_summary.unwrap().total_new, 1);
    }

    #[test]
    fn manifest_finish_sets_end_time() {
        let mut manifest = RunManifest::new(
            "run-004".to_string(),
            "0.1.0".to_string(),
            "localhost".to_string(),
            "defense-lab".to_string(),
        );
        let before = Utc::now();
        manifest.finish();
        let after = Utc::now();
        assert!(manifest.ended_at >= before);
        assert!(manifest.ended_at <= after);
    }
}
