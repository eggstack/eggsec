use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::DiffSummary;

/// Structured summary of a single assessment or defense-lab run.
///
/// `RunManifest` is a metadata envelope that wraps run-level provenance
/// (identity, scope, profile, feature flags) with observations, findings,
/// and an optional baseline diff. It is designed for regression-oriented
/// workflows where runs must be comparable and reproducible.
///
/// The manifest is intentionally minimal and serde-only. It is not yet
/// wired into existing output/report generation paths — that integration
/// is future work.
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
    pub probe_intents: Vec<String>,
    /// Maximum risk tier allowed (e.g. "safe-active", "intrusive").
    pub risk_budget: String,
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
            risk_budget: "safe-active".to_string(),
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
