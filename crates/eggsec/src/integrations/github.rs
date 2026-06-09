use crate::error::{Result, EggsecError};
use crate::integrations::common::send_with_retry;
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
    client: reqwest::Client,
}

impl GitHubClient {
    pub fn new(config: GitHubConfig) -> Self {
        let client = match reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("GitHub: failed to build HTTP client, using default: {}", e);
                reqwest::Client::new()
            }
        };
        Self { config, client }
    }

    fn api_url(&self, path: &str) -> String {
        format!(
            "https://api.github.com/repos/{}/{}{}",
            urlencoding::encode(&self.config.owner),
            urlencoding::encode(&self.config.repo),
            path
        )
    }

    fn parse_issue(json: &serde_json::Value) -> Issue {
        let labels: Vec<String> = json["labels"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v["name"].as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let assignees: Vec<String> = json["assignees"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v["login"].as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let status = json["state"]
            .as_str()
            .or_else(|| {
                tracing::warn!(
                    "GitHub: could not parse state from issue #{}",
                    json["number"].as_i64().unwrap_or(0)
                );
                None
            })
            .map(String::from);

        let created_at = json["created_at"].as_str().and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(s)
                .ok()
                .map(|dt| dt.with_timezone(&chrono::Utc))
        });

        Issue {
            id: json["number"].as_i64().map(|n| n.to_string()),
            title: json["title"].as_str().unwrap_or("").to_string(),
            description: json["body"].as_str().unwrap_or("").to_string(),
            labels,
            severity: None,
            assignees,
            status,
            url: json["html_url"].as_str().map(String::from),
            created_at,
        }
    }
}

#[async_trait::async_trait]
impl IssueTracker for GitHubClient {
    async fn create_issue(&self, issue: &Issue) -> Result<String> {
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

        let req = self
            .client
            .post(&url)
            .header(
                "Authorization",
                format!("Bearer {}", self.config.api_token.expose_secret()),
            )
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("Content-Type", "application/json")
            .json(&body);

        let response = send_with_retry(req, "GitHub").await?;
        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| EggsecError::Network(e.to_string()))?;
        let number = match json["number"].as_i64() {
            Some(n) => n,
            None => {
                tracing::warn!("GitHub: create_issue response missing 'number' field");
                return Err(EggsecError::Network(
                    "GitHub: create_issue response missing 'number' field".to_string(),
                ));
            }
        };
        Ok(format!("#{}", number))
    }

    async fn update_issue(&self, id: &str, update: &IssueUpdate) -> Result<()> {
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

        let req = self
            .client
            .patch(&url)
            .header(
                "Authorization",
                format!("Bearer {}", self.config.api_token.expose_secret()),
            )
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("Content-Type", "application/json")
            .json(&body);

        send_with_retry(req, "GitHub").await?;
        Ok(())
    }

    async fn add_comment(&self, issue_id: &str, comment: &str) -> Result<()> {
        let issue_number = issue_id.trim_start_matches('#');
        let url = self.api_url(&format!("/issues/{}/comments", issue_number));

        let body = serde_json::json!({
            "body": comment
        });

        let req = self
            .client
            .post(&url)
            .header(
                "Authorization",
                format!("Bearer {}", self.config.api_token.expose_secret()),
            )
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("Content-Type", "application/json")
            .json(&body);

        send_with_retry(req, "GitHub").await?;
        Ok(())
    }

    async fn get_issue(&self, id: &str) -> Result<Issue> {
        let issue_number = id.trim_start_matches('#');
        let url = self.api_url(&format!("/issues/{}", issue_number));

        let req = self
            .client
            .get(&url)
            .header(
                "Authorization",
                format!("Bearer {}", self.config.api_token.expose_secret()),
            )
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28");

        let response = send_with_retry(req, "GitHub").await?;
        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| EggsecError::Network(e.to_string()))?;

        Ok(Self::parse_issue(&json))
    }

    async fn search_issues(&self, query: &str) -> Result<Vec<Issue>> {
        let mut all_issues = Vec::new();
        let mut page = 1;
        const PER_PAGE: u32 = 100;
        const MAX_PAGES: u32 = 10;

        loop {
            let scoped_query = format!("repo:{}/{} {}", self.config.owner, self.config.repo, query);
            let url = format!(
                "https://api.github.com/search/issues?q={}&per_page={}&page={}",
                urlencoding::encode(&scoped_query),
                PER_PAGE,
                page
            );

            let req = self
                .client
                .get(&url)
                .header(
                    "Authorization",
                    format!("Bearer {}", self.config.api_token.expose_secret()),
                )
                .header("Accept", "application/vnd.github+json")
                .header("X-GitHub-Api-Version", "2022-11-28");

            let response = send_with_retry(req, "GitHub").await?;
            let json: serde_json::Value = response
                .json()
                .await
                .map_err(|e| EggsecError::Network(e.to_string()))?;

            let items: Vec<Issue> = json["items"]
                .as_array()
                .map(|arr| arr.iter().map(Self::parse_issue).collect())
                .unwrap_or_default();

            let count = items.len();
            all_issues.extend(items);

            if count < PER_PAGE as usize || page >= MAX_PAGES {
                break;
            }
            page += 1;
        }

        Ok(all_issues)
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
