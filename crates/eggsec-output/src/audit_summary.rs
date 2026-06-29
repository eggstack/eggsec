use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuditSummary {
    pub total_events: usize,
    pub allow_count: usize,
    pub warn_count: usize,
    pub confirmed_count: usize,
    pub deny_count: usize,
    pub confirmation_required_count: usize,
    pub manual_override_ignored_count: usize,
    pub surfaces_used: Vec<String>,
}

impl AuditSummary {
    pub fn from_serde_value(events_json: &str) -> Result<Self, serde_json::Error> {
        let events: Vec<serde_json::Value> = serde_json::from_str(events_json)?;
        Ok(Self::from_values(&events))
    }

    pub fn from_values(events: &[serde_json::Value]) -> Self {
        let mut summary = Self::default();
        let mut surfaces = std::collections::HashSet::new();
        for event in events {
            summary.total_events += 1;
            if let Some(outcome) = event.get("outcome").and_then(|v| v.as_str()) {
                match outcome {
                    "allow" => summary.allow_count += 1,
                    "warn" => summary.warn_count += 1,
                    "confirmed" => summary.confirmed_count += 1,
                    "deny" => summary.deny_count += 1,
                    "confirmation-required" => summary.confirmation_required_count += 1,
                    _ => {}
                }
            }
            if event
                .get("manual_override_ignored")
                .and_then(|v| v.as_bool())
                == Some(true)
            {
                summary.manual_override_ignored_count += 1;
            }
            if let Some(surface) = event.get("surface").and_then(|v| v.as_str()) {
                surfaces.insert(surface.to_string());
            }
        }
        summary.surfaces_used = surfaces.into_iter().collect();
        summary.surfaces_used.sort();
        summary
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_summary() {
        let s = AuditSummary::from_values(&[]);
        assert_eq!(s.total_events, 0);
        assert_eq!(s.allow_count, 0);
    }

    #[test]
    fn summary_from_events() {
        let events = serde_json::json!([
            {"outcome": "allow", "surface": "CliManual", "manual_override_ignored": false},
            {"outcome": "deny", "surface": "McpServer", "manual_override_ignored": true},
            {"outcome": "warn", "surface": "RestApi", "manual_override_ignored": false},
            {"outcome": "confirmed", "surface": "CliManual", "manual_override_ignored": false}
        ]);
        let arr = events.as_array().unwrap();
        let s = AuditSummary::from_values(arr);
        assert_eq!(s.total_events, 4);
        assert_eq!(s.allow_count, 1);
        assert_eq!(s.deny_count, 1);
        assert_eq!(s.warn_count, 1);
        assert_eq!(s.confirmed_count, 1);
        assert_eq!(s.manual_override_ignored_count, 1);
        assert_eq!(s.surfaces_used, vec!["CliManual", "McpServer", "RestApi"]);
    }
}
