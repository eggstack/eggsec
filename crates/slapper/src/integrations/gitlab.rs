use crate::error::{Result, SlapperError};
use crate::integrations::{Issue, IssueTracker, IssueUpdate};
use crate::types::SensitiveString;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitLabConfig {
    pub url: String,
    pub project_id: String,
    pub api_token: SensitiveString,
}

pub struct GitLabClient {
    config: GitLabConfig,
    client: reqwest::blocking::Client,
}

impl GitLabClient {
    pub fn new(config: GitLabConfig) -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::blocking::Client::new());
        Self { config, client }
    }

    fn api_url(&self, path: &str) -> String {
        format!(
            "{}/api/v4/projects/{}{}",
            self.config.url.trim_end_matches('/'),
            self.config.project_id,
            path
        )
    }
}

impl IssueTracker for GitLabClient {
    fn create_issue(&self, issue: &Issue) -> Result<String> {
        let url = self.api_url("/issues");

        let labels = if issue.labels.is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::json!(issue.labels)
        };

        let body = serde_json::json!({
            "title": issue.title,
            "description": issue.description,
            "labels": labels
        });

        let response = self
            .client
            .post(&url)
            .header("PRIVATE-TOKEN", self.config.api_token.expose_secret())
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .map_err(|e| SlapperError::Network(e.to_string()))?;

        if response.status().is_success() {
            let json: serde_json::Value = response
                .json()
                .map_err(|e| SlapperError::Network(e.to_string()))?;
            let iid = json["iid"].as_i64().unwrap_or(1);
            Ok(format!("!{}", iid))
        } else {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            Err(SlapperError::Network(format!(
                "GitLab API error {}: {}",
                status, body
            )))
        }
    }

    fn update_issue(&self, id: &str, update: &IssueUpdate) -> Result<()> {
        let issue_iid = id.trim_start_matches('!');
        let url = self.api_url(&format!("/issues/{}", issue_iid));

        let mut body = serde_json::Map::new();

        if let Some(title) = &update.title {
            body.insert("title".to_string(), serde_json::json!(title));
        }
        if let Some(description) = &update.description {
            body.insert("description".to_string(), serde_json::json!(description));
        }
        if let Some(labels) = &update.labels {
            body.insert("labels".to_string(), serde_json::json!(labels));
        }
        if let Some(state) = &update.status {
            let state_value = if state.to_lowercase() == "closed" {
                "close".to_string()
            } else {
                state.to_string()
            };
            body.insert("state_event".to_string(), serde_json::json!(state_value));
        }

        let response = self
            .client
            .put(&url)
            .header("PRIVATE-TOKEN", self.config.api_token.expose_secret())
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
                "GitLab API error {}: {}",
                status, body
            )))
        }
    }

    fn add_comment(&self, issue_id: &str, comment: &str) -> Result<()> {
        let issue_iid = issue_id.trim_start_matches('!');
        let url = self.api_url(&format!("/issues/{}/notes", issue_iid));

        let body = serde_json::json!({
            "body": comment
        });

        let response = self
            .client
            .post(&url)
            .header("PRIVATE-TOKEN", self.config.api_token.expose_secret())
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
                "GitLab API error {}: {}",
                status, body
            )))
        }
    }

    fn get_issue(&self, id: &str) -> Result<Issue> {
        let issue_iid = id.trim_start_matches('!');
        let url = self.api_url(&format!("/issues/{}", issue_iid));

        let response = self
            .client
            .get(&url)
            .header("PRIVATE-TOKEN", self.config.api_token.expose_secret())
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
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            Ok(Issue {
                id: json["iid"].as_i64().map(|n| n.to_string()),
                title: json["title"].as_str().unwrap_or("").to_string(),
                description: json["description"].as_str().unwrap_or("").to_string(),
                labels,
                severity: None,
                assignees: json["assignees"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v["username"].as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default(),
                status: json["state"].as_str().map(String::from),
                url: json["web_url"].as_str().map(String::from),
                created_at: json["created_at"].as_str().and_then(|s| {
                    chrono::DateTime::parse_from_rfc3339(s)
                        .ok()
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                }),
            })
        } else {
            let status = response.status();
            Err(SlapperError::Network(format!(
                "GitLab API error: {}",
                status
            )))
        }
    }

    fn search_issues(&self, query: &str) -> Result<Vec<Issue>> {
        let url = self.api_url(&format!("/issues?search={}", query));

        let response = self
            .client
            .get(&url)
            .header("PRIVATE-TOKEN", self.config.api_token.expose_secret())
            .send()
            .map_err(|e| SlapperError::Network(e.to_string()))?;

        if response.status().is_success() {
            let json: serde_json::Value = response
                .json()
                .map_err(|e| SlapperError::Network(e.to_string()))?;

            let issues: Vec<Issue> = json
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .map(|item| {
                            let labels: Vec<String> = item["labels"]
                                .as_array()
                                .map(|a| {
                                    a.iter()
                                        .filter_map(|v| v.as_str().map(String::from))
                                        .collect()
                                })
                                .unwrap_or_default();
                            Issue {
                                id: item["iid"].as_i64().map(|n| n.to_string()),
                                title: item["title"].as_str().unwrap_or("").to_string(),
                                description: item["description"].as_str().unwrap_or("").to_string(),
                                labels,
                                severity: None,
                                assignees: item["assignees"]
                                    .as_array()
                                    .map(|a| {
                                        a.iter()
                                            .filter_map(|v| {
                                                v["username"].as_str().map(String::from)
                                            })
                                            .collect()
                                    })
                                    .unwrap_or_default(),
                                status: item["state"].as_str().map(String::from),
                                url: item["web_url"].as_str().map(String::from),
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
                "GitLab API error: {}",
                status
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gitlab_client_config() {
        let config = GitLabConfig {
            url: "https://gitlab.com".to_string(),
            project_id: "12345".to_string(),
            api_token: SensitiveString::new("glpat-token".to_string()),
        };
        let client = GitLabClient::new(config.clone());
        assert_eq!(client.config.url, "https://gitlab.com");
    }

    #[test]
    fn test_api_url() {
        let config = GitLabConfig {
            url: "https://gitlab.com".to_string(),
            project_id: "12345".to_string(),
            api_token: SensitiveString::new("glpat-token".to_string()),
        };
        let client = GitLabClient::new(config);
        let url = client.api_url("/issues");
        assert!(url.contains("gitlab.com"));
        assert!(url.contains("12345"));
    }
}
