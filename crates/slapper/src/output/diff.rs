use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffSummary {
    pub total_new: usize,
    pub total_resolved: usize,
    pub total_escalated: usize,
    pub total_deescalated: usize,
    pub net_change: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_summary() {
        let summary = DiffSummary {
            total_new: 5,
            total_resolved: 3,
            total_escalated: 1,
            total_deescalated: 2,
            net_change: 2,
        };
        assert_eq!(summary.net_change, 2);
    }
}
