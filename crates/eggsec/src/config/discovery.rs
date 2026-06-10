use serde::{Deserialize, Serialize};

/// Status of a target discovered during agent/MCP execution.
///
/// Discovered targets in strict modes (MCP/agent) must not silently become
/// authorized. Only explicit scope rules or human approval may promote a
/// candidate to approved scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DiscoveredTargetStatus {
    /// Newly discovered, not yet evaluated.
    Candidate,
    /// Awaiting human approval before scope expansion.
    PendingApproval,
    /// Explicitly approved via scope rules or human action.
    ApprovedInScope,
    /// Rejected as out-of-scope.
    RejectedOutOfScope,
}

impl DiscoveredTargetStatus {
    /// Returns `true` if this status allows the target to be scanned.
    pub fn is_scannable(&self) -> bool {
        matches!(self, Self::ApprovedInScope)
    }
}

impl std::fmt::Display for DiscoveredTargetStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Candidate => write!(f, "candidate"),
            Self::PendingApproval => write!(f, "pending-approval"),
            Self::ApprovedInScope => write!(f, "approved-in-scope"),
            Self::RejectedOutOfScope => write!(f, "rejected-out-of-scope"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_approved_is_scannable() {
        assert!(DiscoveredTargetStatus::ApprovedInScope.is_scannable());
        assert!(!DiscoveredTargetStatus::Candidate.is_scannable());
        assert!(!DiscoveredTargetStatus::PendingApproval.is_scannable());
        assert!(!DiscoveredTargetStatus::RejectedOutOfScope.is_scannable());
    }

    #[test]
    fn display_format() {
        assert_eq!(format!("{}", DiscoveredTargetStatus::Candidate), "candidate");
        assert_eq!(
            format!("{}", DiscoveredTargetStatus::PendingApproval),
            "pending-approval"
        );
    }

    #[test]
    fn serialization_roundtrip() {
        let status = DiscoveredTargetStatus::PendingApproval;
        let json = serde_json::to_string(&status).unwrap();
        let deserialized: DiscoveredTargetStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(status, deserialized);
    }
}
