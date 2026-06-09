use crate::error::{Result, EggsecError};
use crate::integrations::common::send_with_retry;
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
    client: reqwest::Client,
}

impl JiraClient {
    pub fn new(config: JiraConfig) -> Self {
        let client = match reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("Jira: failed to build HTTP client, using default: {}", e);
                reqwest::Client::new()
            }
        };
        Self { config, client }
    }

    async fn transition_issue(&self, id: &str, target_status: &str) -> Result<()> {
        let transitions_url = format!("{}/rest/api/3/issue/{}/transitions", self.config.url, id);

        let req = self.client.get(&transitions_url).basic_auth(
            &self.config.username,
            Some(self.config.api_token.expose_secret()),
        );

        let response = send_with_retry(req, "Jira").await?;
        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| EggsecError::Network(e.to_string()))?;

        let transitions = json["transitions"].as_array().ok_or_else(|| {
            EggsecError::Network(
                "Jira: transitions response missing 'transitions' array".to_string(),
            )
        })?;

        let target_lower = target_status.to_lowercase();
        let transition_id = transitions.iter().find_map(|t| {
            let to_name = t["to"]["name"].as_str().unwrap_or("");
            let to_lower = to_name.to_lowercase();
            if to_lower == target_lower {
                t["id"].as_str().map(String::from)
            } else {
                None
            }
        });

        let transition_id = match transition_id {
            Some(id) => id,
            None => {
                let available: Vec<&str> = transitions
                    .iter()
                    .filter_map(|t| t["to"]["name"].as_str())
                    .collect();
                return Err(EggsecError::Network(format!(
                    "Jira: no transition to '{}' found. Available: {:?}",
                    target_status, available
                )));
            }
        };

        let body = serde_json::json!({
            "transition": { "id": transition_id }
        });

        let req = self
            .client
            .post(&transitions_url)
            .basic_auth(
                &self.config.username,
                Some(self.config.api_token.expose_secret()),
            )
            .header("Content-Type", "application/json")
            .json(&body);

        send_with_retry(req, "Jira").await?;
        Ok(())
    }

    fn parse_issue(json: &serde_json::Value) -> Issue {
        let fields = &json["fields"];
        let description = fields["description"]["content"][0]["content"][0]["text"]
            .as_str()
            .or_else(|| fields["description"].as_str())
            .unwrap_or("");
        if description.is_empty() {
            tracing::warn!(
                "Jira: could not parse description from issue {}",
                json["key"].as_str().unwrap_or("<unknown>")
            );
        }

        let labels: Vec<String> = fields["labels"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let severity =
            fields["priority"]["name"]
                .as_str()
                .and_then(|s| match s.to_lowercase().as_str() {
                    "highest" | "blocker" | "critical" => Some(crate::types::Severity::Critical),
                    "high" => Some(crate::types::Severity::High),
                    "medium" | "major" | "normal" => Some(crate::types::Severity::Medium),
                    "low" | "minor" | "trivial" => Some(crate::types::Severity::Low),
                    other => {
                        tracing::warn!("Jira: unknown severity level '{}'", other);
                        None
                    }
                });

        let assignees: Vec<String> = fields["assignee"]
            .as_object()
            .and_then(|a| a["displayName"].as_str())
            .map(|s| vec![s.to_string()])
            .unwrap_or_default();

        let status = fields["status"]["name"]
            .as_str()
            .or_else(|| {
                tracing::warn!(
                    "Jira: could not parse status from issue {}",
                    json["key"].as_str().unwrap_or("<unknown>")
                );
                None
            })
            .map(String::from);

        let url = json["self"].as_str().map(String::from);

        let created_at = fields["created"].as_str().and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(s)
                .ok()
                .map(|dt| dt.with_timezone(&chrono::Utc))
        });

        Issue {
            id: json["key"].as_str().map(String::from),
            title: fields["summary"].as_str().unwrap_or("").to_string(),
            description: description.to_string(),
            labels,
            severity,
            assignees,
            status,
            url,
            created_at,
        }
    }
}

#[async_trait::async_trait]
impl IssueTracker for JiraClient {
    async fn create_issue(&self, issue: &Issue) -> Result<String> {
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

        let req = self
            .client
            .post(&url)
            .basic_auth(
                &self.config.username,
                Some(self.config.api_token.expose_secret()),
            )
            .header("Content-Type", "application/json")
            .json(&body);

        let response = send_with_retry(req, "Jira").await?;
        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| EggsecError::Network(e.to_string()))?;
        let key = match json["key"].as_str() {
            Some(k) => k.to_string(),
            None => {
                tracing::warn!("Jira: create_issue response missing 'key' field");
                return Err(EggsecError::Network(
                    "Jira: create_issue response missing 'key' field".to_string(),
                ));
            }
        };
        Ok(key)
    }

    async fn update_issue(&self, id: &str, update: &IssueUpdate) -> Result<()> {
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

        if !fields.is_empty() {
            let url = format!("{}/rest/api/3/issue/{}", self.config.url, id);
            let body = serde_json::json!({ "fields": fields });
            let req = self
                .client
                .put(&url)
                .basic_auth(
                    &self.config.username,
                    Some(self.config.api_token.expose_secret()),
                )
                .header("Content-Type", "application/json")
                .json(&body);
            send_with_retry(req, "Jira").await?;
        }

        if let Some(status) = &update.status {
            self.transition_issue(id, status).await?;
        }

        Ok(())
    }

    async fn add_comment(&self, issue_id: &str, comment: &str) -> Result<()> {
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

        let req = self
            .client
            .post(&url)
            .basic_auth(
                &self.config.username,
                Some(self.config.api_token.expose_secret()),
            )
            .header("Content-Type", "application/json")
            .json(&body);

        send_with_retry(req, "Jira").await?;
        Ok(())
    }

    async fn get_issue(&self, id: &str) -> Result<Issue> {
        let url = format!("{}/rest/api/3/issue/{}", self.config.url, id);

        let req = self.client.get(&url).basic_auth(
            &self.config.username,
            Some(self.config.api_token.expose_secret()),
        );

        let response = send_with_retry(req, "Jira").await?;
        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| EggsecError::Network(e.to_string()))?;

        Ok(Self::parse_issue(&json))
    }

    async fn search_issues(&self, query: &str) -> Result<Vec<Issue>> {
        let mut all_issues = Vec::new();
        let mut start_at: u64 = 0;
        const PAGE_SIZE: u64 = 100;
        const MAX_RESULTS: u64 = 1000;

        loop {
            let url = format!(
                "{}/rest/api/3/search?jql={}&maxResults={}&startAt={}",
                self.config.url,
                urlencoding::encode(query),
                PAGE_SIZE,
                start_at
            );

            let req = self.client.get(&url).basic_auth(
                &self.config.username,
                Some(self.config.api_token.expose_secret()),
            );

            let response = send_with_retry(req, "Jira").await?;
            let json: serde_json::Value = response
                .json()
                .await
                .map_err(|e| EggsecError::Network(e.to_string()))?;

            let total = json["total"].as_u64().unwrap_or(0);
            let issues: Vec<Issue> = json["issues"]
                .as_array()
                .map(|arr| arr.iter().map(Self::parse_issue).collect())
                .unwrap_or_default();

            let fetched = issues.len() as u64;
            all_issues.extend(issues);
            start_at += fetched;

            if start_at >= total || fetched == 0 || start_at >= MAX_RESULTS {
                break;
            }
        }

        Ok(all_issues)
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

    #[test]
    fn test_transition_matching_case_insensitive() {
        let transitions = serde_json::json!([
            {"id": "21", "to": {"name": "In Progress"}},
            {"id": "31", "to": {"name": "Done"}},
            {"id": "11", "to": {"name": "To Do"}}
        ]);

        let find_transition = |target: &str| -> Option<String> {
            let target_lower = target.to_lowercase();
            transitions.as_array().unwrap().iter().find_map(|t| {
                let to_lower = t["to"]["name"].as_str().unwrap_or("").to_lowercase();
                if to_lower == target_lower {
                    t["id"].as_str().map(String::from)
                } else {
                    None
                }
            })
        };

        assert_eq!(find_transition("done"), Some("31".to_string()));
        assert_eq!(find_transition("DONE"), Some("31".to_string()));
        assert_eq!(find_transition("Done"), Some("31".to_string()));
        assert_eq!(find_transition("in progress"), Some("21".to_string()));
        assert_eq!(find_transition("In Progress"), Some("21".to_string()));
        assert_eq!(find_transition("to do"), Some("11".to_string()));
        assert_eq!(find_transition("nonexistent"), None);
    }
}
