use crate::error::Result;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assignment {
    pub id: String,
    pub finding_id: String,
    pub user_id: String,
    pub assigned_at: chrono::DateTime<chrono::Utc>,
    pub assigned_by: String,
    pub notes: Option<String>,
}

impl Assignment {
    pub fn new(finding_id: &str, user_id: &str, assigned_by: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            finding_id: finding_id.to_string(),
            user_id: user_id.to_string(),
            assigned_at: chrono::Utc::now(),
            assigned_by: assigned_by.to_string(),
            notes: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignmentRequest {
    pub finding_id: String,
    pub user_id: String,
    pub notes: Option<String>,
}

pub fn assign_finding(request: &AssignmentRequest, assigned_by: &str) -> Result<Assignment> {
    Ok(Assignment::new(
        &request.finding_id,
        &request.user_id,
        assigned_by,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assignment_creation() {
        let assignment = Assignment::new("finding-1", "user-1", "admin");
        assert_eq!(assignment.finding_id, "finding-1");
        assert_eq!(assignment.user_id, "user-1");
    }
}
