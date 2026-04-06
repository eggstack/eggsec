use crate::error::{Result, SlapperError};
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
    config: GitHubConfig,
    client: reqwest::blocking::Client,
}

impl GitHubClient {
    pub fn new(config: GitHubConfig) -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::blocking::Client::new());
        Self { config, client }
    }

    fn api_url(&self, path: &str) -> String {
        format!(
            "https://api.github.com/repos/{}/{}{}",
            self.config.owner, self.config.repo, path
        )
    }
}

impl IssueTracker for GitHubClient {
    fn create_issue(&self, issue: &Issue) -> Result<String> {
        let url = self.api_url("/issues");

        let labels = if issue.labels.is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::json!(issue.labels)
        };

        let body = serde_json::json!({
            "title": issue.title,
            "body": issue.description,
            "labels": labels
        });

        let response = self
            .client
            .post(&url)
            .header(
                "Authorization",
                format!("Bearer {}", self.config.api_token.expose_secret()),
            )
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .map_err(|e| SlapperError::Network(e.to_string()))?;

        if response.status().is_success() {
            let json: serde_json::Value = response
                .json()
                .map_err(|e| SlapperError::Network(e.to_string()))?;
            let number = json["number"].as_i64().unwrap_or(1);
            Ok(format!("#{}", number))
        } else {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            Err(SlapperError::Network(format!(
                "GitHub API error {}: {}",
                status, body
            )))
        }
    }

    fn update_issue(&self, id: &str, update: &IssueUpdate) -> Result<()> {
        let issue_number = id.trim_start_matches('#');
        let url = self.api_url(&format!("/issues/{}", issue_number));

        let mut body = serde_json::Map::new();

        if let Some(title) = &update.title {
            body.insert("title".to_string(), serde_json::json!(title));
        }
        if let Some(description) = &update.description {
            body.insert("body".to_string(), serde_json::json!(description));
        }
        if let Some(labels) = &update.labels {
            body.insert("labels".to_string(), serde_json::json!(labels));
        }
        if let Some(state) = &update.status {
            body.insert("state".to_string(), serde_json::json!(state));
        }

        let response = self
            .client
            .patch(&url)
            .header(
                "Authorization",
                format!("Bearer {}", self.config.api_token.expose_secret()),
            )
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .map_err(|e| SlapperError::Network(e.to_string()))?;

        if response.status().is_success() {
            Ok(())
        } else {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            Err(SlapperError::Network(format!(
                "GitHub API error {}: {}",
                status, body
            )))
        }
    }

    fn add_comment(&self, issue_id: &str, comment: &str) -> Result<()> {
        let issue_number = issue_id.trim_start_matches('#');
        let url = self.api_url(&format!("/issues/{}/comments", issue_number));

        let body = serde_json::json!({
            "body": comment
        });

        let response = self
            .client
            .post(&url)
            .header(
                "Authorization",
                format!("Bearer {}", self.config.api_token.expose_secret()),
            )
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .map_err(|e| SlapperError::Network(e.to_string()))?;

        if response.status().is_success() {
            Ok(())
        } else {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            Err(SlapperError::Network(format!(
                "GitHub API error {}: {}",
                status, body
            )))
        }
    }

    fn get_issue(&self, id: &str) -> Result<Issue> {
        let issue_number = id.trim_start_matches('#');
        let url = self.api_url(&format!("/issues/{}", issue_number));

        let response = self
            .client
            .get(&url)
            .header(
                "Authorization",
                format!("Bearer {}", self.config.api_token.expose_secret()),
            )
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send()
            .map_err(|e| SlapperError::Network(e.to_string()))?;

        if response.status().is_success() {
            let json: serde_json::Value = response
                .json()
                .map_err(|e| SlapperError::Network(e.to_string()))?;

            let labels: Vec<String> = json["labels"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v["name"].as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            Ok(Issue {
                id: json["number"].as_i64().map(|n| n.to_string()),
                title: json["title"].as_str().unwrap_or("").to_string(),
                description: json["body"].as_str().unwrap_or("").to_string(),
                labels,
                severity: None,
                assignees: json["assignees"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v["login"].as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default(),
                status: None,
                url: json["html_url"].as_str().map(String::from),
                created_at: json["created_at"].as_str().and_then(|s| {
                    chrono::DateTime::parse_from_rfc3339(s)
                        .ok()
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                }),
            })
        } else {
            let status = response.status();
            Err(SlapperError::Network(format!(
                "GitHub API error: {}",
                status
            )))
        }
    }

    fn search_issues(&self, query: &str) -> Result<Vec<Issue>> {
        let url = format!("https://api.github.com/search/issues?q={}", query);

        let response = self
            .client
            .get(&url)
            .header(
                "Authorization",
                format!("Bearer {}", self.config.api_token.expose_secret()),
            )
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send()
            .map_err(|e| SlapperError::Network(e.to_string()))?;

        if response.status().is_success() {
            let json: serde_json::Value = response
                .json()
                .map_err(|e| SlapperError::Network(e.to_string()))?;

            let issues: Vec<Issue> = json["items"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .map(|item| {
                            let labels: Vec<String> = item["labels"]
                                .as_array()
                                .map(|a| {
                                    a.iter()
                                        .filter_map(|v| v["name"].as_str().map(String::from))
                                        .collect()
                                })
                                .unwrap_or_default();
                            Issue {
                                id: item["number"].as_i64().map(|n| n.to_string()),
                                title: item["title"].as_str().unwrap_or("").to_string(),
                                description: item["body"].as_str().unwrap_or("").to_string(),
                                labels,
                                severity: None,
                                assignees: item["assignees"]
                                    .as_array()
                                    .map(|a| {
                                        a.iter()
                                            .filter_map(|v| v["login"].as_str().map(String::from))
                                            .collect()
                                    })
                                    .unwrap_or_default(),
                                status: None,
                                url: item["html_url"].as_str().map(String::from),
                                created_at: item["created_at"].as_str().and_then(|s| {
                                    chrono::DateTime::parse_from_rfc3339(s)
                                        .ok()
                                        .map(|dt| dt.with_timezone(&chrono::Utc))
                                }),
                            }
                        })
                        .collect()
                })
                .unwrap_or_default();

            Ok(issues)
        } else {
            let status = response.status();
            Err(SlapperError::Network(format!(
                "GitHub API error: {}",
                status
            )))
        }
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
        let client = GitHubClient::new(config.clone());
        assert_eq!(client.config.owner, "owner");
    }

    #[test]
    fn test_api_url() {
        let config = GitHubConfig {
            owner: "owner".to_string(),
            repo: "repo".to_string(),
            api_token: SensitiveString::new("ghp_token".to_string()),
        };
        let client = GitHubClient::new(config);
        let url = client.api_url("/issues");
        assert!(url.contains("github.com"));
        assert!(url.contains("owner/repo"));
    }
}
