use crate::error::Result;
use crate::storage::models::*;
use crate::types::Severity;

pub struct QueryBuilder;

impl QueryBuilder {
    pub fn find_open_findings_by_severity(severity: Severity) -> String {
        format!(
            "SELECT * FROM findings WHERE status = 'Open' AND severity = '{}'",
            severity.as_str()
        )
    }

    pub fn find_recent_scans(limit: usize) -> String {
        format!(
            "SELECT * FROM scans ORDER BY started_at DESC LIMIT {}",
            limit
        )
    }

    pub fn find_findings_by_cve(cve_id: &str) -> String {
        format!("SELECT * FROM findings WHERE cve_ids @> '[\"{}\"]'", cve_id)
    }

    pub fn count_findings_by_status() -> String {
        "SELECT status, COUNT(*) FROM findings GROUP BY status".to_string()
    }

    pub fn find_duplicate_findings(similarity_threshold: f32) -> String {
        format!(
            "SELECT * FROM findings WHERE similarity(title, description) > {}",
            similarity_threshold
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_builder() {
        let query = QueryBuilder::find_open_findings_by_severity(Severity::Critical);
        assert!(query.contains("Critical"));
    }
}
