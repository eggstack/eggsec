use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WafBehavior {
    Blocked,
    Allowed,
    Challenged,
    Tarpitted,
    Errored,
    Skipped,
}

impl std::fmt::Display for WafBehavior {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WafBehavior::Blocked => write!(f, "blocked"),
            WafBehavior::Allowed => write!(f, "allowed"),
            WafBehavior::Challenged => write!(f, "challenged"),
            WafBehavior::Tarpitted => write!(f, "tarpitted"),
            WafBehavior::Errored => write!(f, "errored"),
            WafBehavior::Skipped => write!(f, "skipped"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WafRegressionCase {
    pub payload_family: String,
    pub payload_type: String,
    pub request_summary: String,
    pub status_code: u16,
    pub behavior: WafBehavior,
    pub response_time_ms: u64,
    pub baseline_behavior: Option<WafBehavior>,
    pub regression: bool,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WafBehaviorSummary {
    pub total_cases: usize,
    pub blocked: usize,
    pub allowed: usize,
    pub challenged: usize,
    pub tarpitted: usize,
    pub errored: usize,
    pub skipped: usize,
    pub regression_count: usize,
    pub new_bypass_count: usize,
}

impl WafBehaviorSummary {
    pub fn from_cases(cases: &[WafRegressionCase]) -> Self {
        let total_cases = cases.len();
        let blocked = cases.iter().filter(|c| c.behavior == WafBehavior::Blocked).count();
        let allowed = cases.iter().filter(|c| c.behavior == WafBehavior::Allowed).count();
        let challenged = cases.iter().filter(|c| c.behavior == WafBehavior::Challenged).count();
        let tarpitted = cases.iter().filter(|c| c.behavior == WafBehavior::Tarpitted).count();
        let errored = cases.iter().filter(|c| c.behavior == WafBehavior::Errored).count();
        let skipped = cases.iter().filter(|c| c.behavior == WafBehavior::Skipped).count();
        let regression_count = cases.iter().filter(|c| c.regression).count();
        let new_bypass_count = cases
            .iter()
            .filter(|c| {
                c.behavior == WafBehavior::Allowed
                    && c.baseline_behavior == Some(WafBehavior::Blocked)
            })
            .count();

        Self {
            total_cases,
            blocked,
            allowed,
            challenged,
            tarpitted,
            errored,
            skipped,
            regression_count,
            new_bypass_count,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WafRegressionReport {
    pub target: String,
    pub profile: String,
    pub scope_file: Option<String>,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
    pub payload_families_tested: Vec<String>,
    pub cases: Vec<WafRegressionCase>,
    pub summary: WafBehaviorSummary,
    pub baseline_id: Option<String>,
    pub budget_consumed: Option<f64>,
    pub termination_reason: Option<String>,
}

impl WafRegressionReport {
    pub fn to_human_readable(&self) -> String {
        let mut out = format!("WAF Regression Report for {}\n", self.target);
        out.push_str(&format!("Profile: {}\n", self.profile));
        if let Some(ref baseline) = self.baseline_id {
            out.push_str(&format!("Baseline: {}\n", baseline));
        }
        out.push_str(&format!(
            "Period: {} to {}\n",
            self.started_at.format("%Y-%m-%d %H:%M:%S UTC"),
            self.ended_at.format("%Y-%m-%d %H:%M:%S UTC")
        ));
        out.push_str(&format!("Families tested: {}\n", self.payload_families_tested.len()));
        out.push_str("\nBehavior Summary:\n");
        out.push_str(&format!("  Total cases: {}\n", self.summary.total_cases));
        out.push_str(&format!("  Blocked: {}\n", self.summary.blocked));
        out.push_str(&format!("  Allowed: {}\n", self.summary.allowed));
        out.push_str(&format!("  Challenged: {}\n", self.summary.challenged));
        out.push_str(&format!("  Tarpitted: {}\n", self.summary.tarpitted));
        out.push_str(&format!("  Errored: {}\n", self.summary.errored));
        out.push_str(&format!("  Skipped: {}\n", self.summary.skipped));
        out.push_str(&format!("  Regressions: {}\n", self.summary.regression_count));
        out.push_str(&format!("  New bypasses: {}\n", self.summary.new_bypass_count));

        if self.summary.regression_count > 0 {
            out.push_str("\nRegressions detected:\n");
            for case in &self.cases {
                if case.regression {
                    out.push_str(&format!(
                        "  [{}] {} ({}) - baseline {:?} -> {}\n",
                        case.payload_family,
                        case.request_summary,
                        case.payload_type,
                        case.baseline_behavior,
                        case.behavior,
                    ));
                }
            }
        }

        if let Some(ref reason) = self.termination_reason {
            out.push_str(&format!("\nTermination: {}\n", reason));
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_case(
        family: &str,
        behavior: WafBehavior,
        baseline: Option<WafBehavior>,
        regression: bool,
    ) -> WafRegressionCase {
        WafRegressionCase {
            payload_family: family.to_string(),
            payload_type: "sqli".to_string(),
            request_summary: format!("GET /test?param={}", family),
            status_code: if behavior == WafBehavior::Blocked { 403 } else { 200 },
            behavior,
            response_time_ms: 50,
            baseline_behavior: baseline,
            regression,
            confidence: 0.95,
        }
    }

    #[test]
    fn summary_counts_behaviors() {
        let cases = vec![
            make_case("sqli", WafBehavior::Blocked, None, false),
            make_case("xss", WafBehavior::Allowed, None, false),
            make_case("ssrf", WafBehavior::Challenged, None, false),
        ];
        let summary = WafBehaviorSummary::from_cases(&cases);
        assert_eq!(summary.total_cases, 3);
        assert_eq!(summary.blocked, 1);
        assert_eq!(summary.allowed, 1);
        assert_eq!(summary.challenged, 1);
        assert_eq!(summary.tarpitted, 0);
        assert_eq!(summary.errored, 0);
        assert_eq!(summary.skipped, 0);
    }

    #[test]
    fn regression_detected() {
        let cases = vec![
            make_case("sqli", WafBehavior::Allowed, Some(WafBehavior::Blocked), true),
            make_case("xss", WafBehavior::Blocked, None, false),
        ];
        let summary = WafBehaviorSummary::from_cases(&cases);
        assert_eq!(summary.regression_count, 1);
    }

    #[test]
    fn new_bypass_detected() {
        let cases = vec![
            make_case("sqli", WafBehavior::Allowed, Some(WafBehavior::Blocked), true),
            make_case("xss", WafBehavior::Allowed, Some(WafBehavior::Allowed), false),
            make_case("ssrf", WafBehavior::Blocked, None, false),
        ];
        let summary = WafBehaviorSummary::from_cases(&cases);
        assert_eq!(summary.new_bypass_count, 1);
    }

    #[test]
    fn report_human_readable() {
        let cases = vec![
            make_case("sqli", WafBehavior::Blocked, None, false),
            make_case("xss", WafBehavior::Allowed, Some(WafBehavior::Blocked), true),
        ];
        let summary = WafBehaviorSummary::from_cases(&cases);
        let report = WafRegressionReport {
            target: "https://example.com".to_string(),
            profile: "cloudflare".to_string(),
            scope_file: None,
            started_at: Utc::now(),
            ended_at: Utc::now(),
            payload_families_tested: vec!["sqli".to_string(), "xss".to_string()],
            cases,
            summary,
            baseline_id: Some("run-001".to_string()),
            budget_consumed: None,
            termination_reason: None,
        };

        let readable = report.to_human_readable();
        assert!(readable.contains("https://example.com"));
        assert!(readable.contains("cloudflare"));
        assert!(readable.contains("run-001"));
        assert!(readable.contains("Regressions detected"));
        assert!(readable.contains("sqli"));
        assert!(readable.contains("xss"));
    }
}
