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

pub fn add_comment(request: &CommentRequest, user_id: &str) -> Comment {
    Comment::new(
        &request.finding_id,
        user_id,
        &request.content,
        request.is_internal,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comment_creation() {
        let comment = Comment::new("finding-1", "user-1", "This is a comment", false);
        assert_eq!(comment.finding_id, "finding-1");
        assert_eq!(comment.user_id, "user-1");
        assert_eq!(comment.content, "This is a comment");
        assert!(!comment.is_internal);
    }

    #[test]
    fn test_comment_internal() {
        let comment = Comment::new("finding-1", "user-1", "Internal note", true);
        assert!(comment.is_internal);
    }

    #[test]
    fn test_add_comment() {
        let request = CommentRequest {
            finding_id: "finding-1".to_string(),
            content: "Looks like SQLi".to_string(),
            is_internal: false,
        };
        let comment = add_comment(&request, "analyst-1");
        assert_eq!(comment.finding_id, "finding-1");
        assert_eq!(comment.user_id, "analyst-1");
        assert_eq!(comment.content, "Looks like SQLi");
    }

    #[test]
    fn test_comment_has_uuid() {
        let comment = Comment::new("f1", "u1", "c1", false);
        assert!(!comment.id.is_empty());
        let comment2 = Comment::new("f1", "u1", "c1", false);
        assert_ne!(comment.id, comment2.id);
    }
}
