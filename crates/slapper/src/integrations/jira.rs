use crate::error::{Result, SlapperError};
use crate::integrations::{Issue, IssueTracker, IssueUpdate};
use crate::types::SensitiveString;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraConfig {
    pub url: String,
    pub username: String,
    pub api_token: SensitiveString,
    pub project_key: String,
}

pub struct JiraClient {
    config: JiraConfig,
    client: reqwest::blocking::Client,
}

impl JiraClient {
    pub fn new(config: JiraConfig) -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::blocking::Client::new());
        Self { config, client }
    }
}

impl IssueTracker for JiraClient {
    fn create_issue(&self, issue: &Issue) -> Result<String> {
        let url = format!("{}/rest/api/3/issue", self.config.url);

        let severity_label = issue
            .severity
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Medium".to_string());

        let body = serde_json::json!({
            "fields": {
                "project": { "key": self.config.project_key },
                "summary": issue.title,
                "description": {
                    "type": "doc",
                    "version": 1,
                    "content": [{
                        "type": "paragraph",
                        "content": [{
                            "type": "text",
                            "text": issue.description
                        }]
                    }]
                },
                "issuetype": { "name": "Task" },
                "labels": issue.labels,
                "priority": { "name": severity_label }
            }
        });

        let response = self
            .client
            .post(&url)
            .basic_auth(
                &self.config.username,
                Some(self.config.api_token.expose_secret()),
            )
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .map_err(|e| SlapperError::Network(e.to_string()))?;

        if response.status().is_success() {
            let json: serde_json::Value = response
                .json()
                .map_err(|e| SlapperError::Network(e.to_string()))?;
            Ok(json["key"].as_str().unwrap_or("JIRA-1").to_string())
        } else {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            Err(SlapperError::Network(format!(
                "Jira API error {}: {}",
                status, body
            )))
        }
    }

    fn update_issue(&self, id: &str, update: &IssueUpdate) -> Result<()> {
        let url = format!("{}/rest/api/3/issue/{}", self.config.url, id);

        let mut fields = serde_json::Map::new();

        if let Some(title) = &update.title {
            fields.insert("summary".to_string(), serde_json::json!(title));
        }
        if let Some(description) = &update.description {
            fields.insert(
                "description".to_string(),
                serde_json::json!({
                    "type": "doc",
                    "version": 1,
                    "content": [{
                        "type": "paragraph",
                        "content": [{
                            "type": "text",
                            "text": description
                        }]
                    }]
                }),
            );
        }
        if let Some(labels) = &update.labels {
            fields.insert("labels".to_string(), serde_json::json!(labels));
        }

        let body = serde_json::json!({ "fields": fields });

        let response = self
            .client
            .post(&url)
            .basic_auth(
                &self.config.username,
                Some(self.config.api_token.expose_secret()),
            )
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
                "Jira API error {}: {}",
                status, body
            )))
        }
    }

    fn add_comment(&self, issue_id: &str, comment: &str) -> Result<()> {
        let url = format!("{}/rest/api/3/issue/{}/comment", self.config.url, issue_id);

        let body = serde_json::json!({
            "body": {
                "type": "doc",
                "version": 1,
                "content": [{
                    "type": "paragraph",
                    "content": [{
                        "type": "text",
                        "text": comment
                    }]
                }]
            }
        });

        let response = self
            .client
            .post(&url)
            .basic_auth(
                &self.config.username,
                Some(self.config.api_token.expose_secret()),
            )
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
                "Jira API error {}: {}",
                status, body
            )))
        }
    }

    fn get_issue(&self, id: &str) -> Result<Issue> {
        let url = format!("{}/rest/api/3/issue/{}", self.config.url, id);

        let response = self
            .client
            .get(&url)
            .basic_auth(
                &self.config.username,
                Some(self.config.api_token.expose_secret()),
            )
            .send()
            .map_err(|e| SlapperError::Network(e.to_string()))?;

        if response.status().is_success() {
            let json: serde_json::Value = response
                .json()
                .map_err(|e| SlapperError::Network(e.to_string()))?;

            let fields = &json["fields"];
            let description = fields["description"]["content"][0]["content"][0]["text"]
                .as_str()
                .unwrap_or("")
                .to_string();

            let labels: Vec<String> = fields["labels"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            Ok(Issue {
                title: fields["summary"].as_str().unwrap_or("").to_string(),
                description,
                labels,
                severity: None,
                assignees: vec![],
            })
        } else {
            let status = response.status();
            Err(SlapperError::Network(format!("Jira API error: {}", status)))
        }
    }

    fn search_issues(&self, query: &str) -> Result<Vec<Issue>> {
        let url = format!("{}/rest/api/3/search", self.config.url);

        let body = serde_json::json!({
            "jql": query,
            "maxResults": 100
        });

        let response = self
            .client
            .post(&url)
            .basic_auth(
                &self.config.username,
                Some(self.config.api_token.expose_secret()),
            )
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .map_err(|e| SlapperError::Network(e.to_string()))?;

        if response.status().is_success() {
            let json: serde_json::Value = response
                .json()
                .map_err(|e| SlapperError::Network(e.to_string()))?;

            let issues: Vec<Issue> = json["issues"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|item| {
                            let fields = item.get("fields")?;
                            let description = fields["description"]["content"][0]["content"][0]
                                ["text"]
                                .as_str()
                                .unwrap_or("")
                                .to_string();
                            Some(Issue {
                                title: fields["summary"].as_str().unwrap_or("").to_string(),
                                description,
                                labels: fields["labels"]
                                    .as_array()
                                    .map(|a| {
                                        a.iter()
                                            .filter_map(|v| v.as_str().map(String::from))
                                            .collect()
                                    })
                                    .unwrap_or_default(),
                                severity: None,
                                assignees: vec![],
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();

            Ok(issues)
        } else {
            let status = response.status();
            Err(SlapperError::Network(format!("Jira API error: {}", status)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jira_client_config() {
        let config = JiraConfig {
            url: "https://example.atlassian.net".to_string(),
            username: "user@example.com".to_string(),
            api_token: SensitiveString::new("token123".to_string()),
            project_key: "SEC".to_string(),
        };
        let _client = JiraClient::new(config);
    }
}
