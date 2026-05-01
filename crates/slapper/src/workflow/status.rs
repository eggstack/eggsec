use crate::error::Result;
use crate::workflow::finding::FindingStatus;

pub struct StatusWorkflow;

impl StatusWorkflow {
    pub fn can_transition(from: &FindingStatus, to: &FindingStatus) -> bool {
        matches!(
            (from, to),
            (FindingStatus::Open, FindingStatus::InProgress)
                | (FindingStatus::Open, FindingStatus::FalsePositive)
                | (FindingStatus::InProgress, FindingStatus::Resolved)
                | (FindingStatus::InProgress, FindingStatus::Open)
                | (FindingStatus::Resolved, FindingStatus::Verified)
                | (FindingStatus::Resolved, FindingStatus::Open)
                | (FindingStatus::Verified, FindingStatus::Open)
        )
    }

    pub fn validate_transition(from: &FindingStatus, to: &FindingStatus) -> Result<()> {
        if Self::can_transition(from, to) {
            Ok(())
        } else {
            Err(crate::error::SlapperError::Validation(format!(
                "Invalid transition from {:?} to {:?}",
                from, to
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_transitions() {
        assert!(StatusWorkflow::can_transition(
            &FindingStatus::Open,
            &FindingStatus::InProgress
        ));
        assert!(StatusWorkflow::can_transition(
            &FindingStatus::InProgress,
            &FindingStatus::Resolved
        ));
        assert!(StatusWorkflow::can_transition(
            &FindingStatus::Resolved,
            &FindingStatus::Verified
        ));
    }

    #[test]
    fn test_invalid_transitions() {
        assert!(!StatusWorkflow::can_transition(
            &FindingStatus::Open,
            &FindingStatus::Verified
        ));
        assert!(!StatusWorkflow::can_transition(
            &FindingStatus::FalsePositive,
            &FindingStatus::InProgress
        ));
    }
}
