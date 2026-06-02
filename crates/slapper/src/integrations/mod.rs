//! Issue tracker integration module
//!
//! Provides integration with external issue trackers like Jira, GitHub, and GitLab.
//!
//! ## Modules
//!
//! - [`common`] - Common traits and types for all issue trackers
//! - [`jira`] - Jira issue creation and management
//! - [`github`] - GitHub Issues integration
//! - [`gitlab`] - GitLab Issues integration

pub mod common;
pub mod github;
pub mod gitlab;
pub mod jira;

use crate::error::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IntegrationConfig {
    pub jira: Option<jira::JiraConfig>,
    pub github: Option<github::GitHubConfig>,
    pub gitlab: Option<gitlab::GitLabConfig>,
}

#[async_trait::async_trait]
pub trait IssueTracker: Send + Sync {
    async fn create_issue(&self, issue: &Issue) -> Result<String>;
    async fn update_issue(&self, id: &str, update: &IssueUpdate) -> Result<()>;
    async fn add_comment(&self, issue_id: &str, comment: &str) -> Result<()>;
    async fn get_issue(&self, id: &str) -> Result<Issue>;
    async fn search_issues(&self, query: &str) -> Result<Vec<Issue>>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub id: Option<String>,
    pub title: String,
    pub description: String,
    pub labels: Vec<String>,
    pub severity: Option<crate::types::Severity>,
    pub assignees: Vec<String>,
    pub status: Option<String>,
    pub url: Option<String>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueUpdate {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub labels: Option<Vec<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_issue_creation() {
        let issue = Issue {
            id: Some("123".to_string()),
            title: "Test Issue".to_string(),
            description: "Test description".to_string(),
            labels: vec!["security".to_string()],
            severity: Some(crate::types::Severity::High),
            assignees: vec![],
            status: Some("Open".to_string()),
            url: None,
            created_at: None,
        };
        assert_eq!(issue.title, "Test Issue");
        assert_eq!(issue.id, Some("123".to_string()));
    }
}
