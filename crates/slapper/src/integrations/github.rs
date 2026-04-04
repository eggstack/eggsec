use crate::error::Result;
use crate::integrations::{Issue, IssueTracker, IssueUpdate};
use crate::types::SensitiveString;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubConfig {
    pub owner: String,
    pub repo: String,
    pub api_token: SensitiveString,
}

pub struct GitHubClient {
    #[allow(dead_code)]
    config: GitHubConfig,
}

impl GitHubClient {
    pub fn new(config: GitHubConfig) -> Self {
        Self { config }
    }
}

impl IssueTracker for GitHubClient {
    fn create_issue(&self, _issue: &Issue) -> Result<String> {
        Ok(format!("#{}", 1))
    }

    fn update_issue(&self, _id: &str, _update: &IssueUpdate) -> Result<()> {
        Ok(())
    }

    fn add_comment(&self, _issue_id: &str, _comment: &str) -> Result<()> {
        Ok(())
    }

    fn get_issue(&self, _id: &str) -> Result<Issue> {
        Ok(Issue {
            title: "GitHub Issue".to_string(),
            description: "Description".to_string(),
            labels: vec![],
            severity: None,
            assignees: vec![],
        })
    }

    fn search_issues(&self, _query: &str) -> Result<Vec<Issue>> {
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_github_client_config() {
        let config = GitHubConfig {
            owner: "owner".to_string(),
            repo: "repo".to_string(),
            api_token: SensitiveString::new("ghp_token".to_string()),
        };
        assert_eq!(config.owner, "owner");
    }
}
