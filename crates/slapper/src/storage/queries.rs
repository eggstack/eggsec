pub struct QueryBuilder;

impl QueryBuilder {
    pub fn find_open_findings_by_severity() -> &'static str {
        "SELECT * FROM findings WHERE status = 'new' AND finding->>'severity' = $1 ORDER BY created_at DESC"
    }

    pub fn find_recent_scans() -> &'static str {
        "SELECT * FROM scans ORDER BY started_at DESC LIMIT $1"
    }

    pub fn find_findings_by_cve() -> &'static str {
        "SELECT * FROM findings WHERE finding->>'cve' = $1"
    }

    pub fn count_findings_by_status() -> &'static str {
        "SELECT status, COUNT(*) as count FROM findings GROUP BY status"
    }

    pub fn find_duplicate_findings() -> &'static str {
        "SELECT f1.id, f2.id as duplicate_id
         FROM findings f1
         JOIN findings f2 ON f1.id < f2.id
         WHERE similarity(f1.finding->>'title', f2.finding->>'title') > $1::float"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_builder() {
        let query = QueryBuilder::find_open_findings_by_severity();
        assert!(query.contains("finding->>'severity'"));
        assert!(query.contains("$1"));
    }

    #[test]
    fn test_find_recent_scans() {
        let query = QueryBuilder::find_recent_scans();
        assert!(query.contains("LIMIT $1"));
    }

    #[test]
    fn test_find_findings_by_cve() {
        let query = QueryBuilder::find_findings_by_cve();
        assert!(query.contains("finding->>'cve'"));
        assert!(query.contains("$1"));
    }

    #[test]
    fn test_count_findings_by_status() {
        let query = QueryBuilder::count_findings_by_status();
        assert!(query.contains("GROUP BY status"));
    }

    #[test]
    fn test_find_duplicate_findings() {
        let query = QueryBuilder::find_duplicate_findings();
        assert!(query.contains("similarity"));
        assert!(query.contains("$1"));
    }
}
