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
                | (FindingStatus::FalsePositive, FindingStatus::Open)
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
        assert!(StatusWorkflow::can_transition(
            &FindingStatus::FalsePositive,
            &FindingStatus::Open
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
        assert!(!StatusWorkflow::can_transition(
            &FindingStatus::Verified,
            &FindingStatus::InProgress
        ));
    }

    #[test]
    fn test_validate_transition_ok() {
        assert!(StatusWorkflow::validate_transition(
            &FindingStatus::Open,
            &FindingStatus::InProgress
        )
        .is_ok());
    }

    #[test]
    fn test_validate_transition_error() {
        let result = StatusWorkflow::validate_transition(
            &FindingStatus::Open,
            &FindingStatus::Verified,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_all_invalid_transitions() {
        let invalid = [
            (FindingStatus::Open, FindingStatus::Resolved),
            (FindingStatus::Open, FindingStatus::Verified),
            (FindingStatus::InProgress, FindingStatus::Verified),
            (FindingStatus::InProgress, FindingStatus::FalsePositive),
            (FindingStatus::Resolved, FindingStatus::InProgress),
            (FindingStatus::Resolved, FindingStatus::FalsePositive),
            (FindingStatus::Verified, FindingStatus::InProgress),
            (FindingStatus::Verified, FindingStatus::Resolved),
            (FindingStatus::Verified, FindingStatus::FalsePositive),
            (FindingStatus::FalsePositive, FindingStatus::InProgress),
            (FindingStatus::FalsePositive, FindingStatus::Resolved),
            (FindingStatus::FalsePositive, FindingStatus::Verified),
        ];

        for (from, to) in invalid {
            assert!(
                !StatusWorkflow::can_transition(&from, &to),
                "Transition from {:?} to {:?} should be invalid",
                from,
                to
            );
        }
    }

    #[test]
    fn test_all_valid_transitions() {
        let valid = [
            (FindingStatus::Open, FindingStatus::InProgress),
            (FindingStatus::Open, FindingStatus::FalsePositive),
            (FindingStatus::InProgress, FindingStatus::Resolved),
            (FindingStatus::InProgress, FindingStatus::Open),
            (FindingStatus::Resolved, FindingStatus::Verified),
            (FindingStatus::Resolved, FindingStatus::Open),
            (FindingStatus::Verified, FindingStatus::Open),
            (FindingStatus::FalsePositive, FindingStatus::Open),
        ];

        for (from, to) in valid {
            assert!(
                StatusWorkflow::can_transition(&from, &to),
                "Transition from {:?} to {:?} should be valid",
                from,
                to
            );
        }
    }
}
