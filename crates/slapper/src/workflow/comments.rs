use crate::error::Result;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: String,
    pub finding_id: String,
    pub user_id: String,
    pub content: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub is_internal: bool,
}

impl Comment {
    pub fn new(finding_id: &str, user_id: &str, content: &str, is_internal: bool) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            finding_id: finding_id.to_string(),
            user_id: user_id.to_string(),
            content: content.to_string(),
            created_at: chrono::Utc::now(),
            is_internal,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentRequest {
    pub finding_id: String,
    pub content: String,
    pub is_internal: bool,
}

pub fn add_comment(request: &CommentRequest, user_id: &str) -> Result<Comment> {
    Ok(Comment::new(
        &request.finding_id,
        user_id,
        &request.content,
        request.is_internal,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comment_creation() {
        let comment = Comment::new("finding-1", "user-1", "This is a comment", false);
        assert_eq!(comment.finding_id, "finding-1");
        assert!(!comment.is_internal);
    }
}
