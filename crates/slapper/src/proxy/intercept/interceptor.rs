//! Request/response interception logic
//!
//! Handles the interception of HTTP/HTTPS requests and responses
//! with pause, modification, and continuation capabilities.

use super::rules::{InterceptRule, RuleAction, RuleSet};
use crate::error::Result;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InterceptMode {
    Monitor,
    Intercept,
    Allow,
}

impl Default for InterceptMode {
    fn default() -> Self {
        Self::Monitor
    }
}

#[derive(Debug, Clone)]
pub struct InterceptConfig {
    pub mode: InterceptMode,
    pub pause_on_match: bool,
    pub timeout: Duration,
    pub buffer_size: usize,
}

impl Default for InterceptConfig {
    fn default() -> Self {
        Self {
            mode: InterceptMode::Monitor,
            pause_on_match: false,
            timeout: Duration::from_secs(30),
            buffer_size: 8192,
        }
    }
}

pub struct InterceptProxy {
    config: InterceptConfig,
    rules: Arc<RwLock<RuleSet>>,
    event_tx: Option<mpsc::Sender<InterceptEvent>>,
}

impl InterceptProxy {
    pub fn new(config: InterceptConfig) -> Self {
        Self {
            config,
            rules: Arc::new(RwLock::new(RuleSet::default())),
            event_tx: None,
        }
    }

    pub fn with_rules(mut self, rules: RuleSet) -> Self {
        *self.rules.write() = rules;
        self
    }

    pub fn with_event_channel(mut self, tx: mpsc::Sender<InterceptEvent>) -> Self {
        self.event_tx = Some(tx);
        self
    }

    pub fn add_rule(&self, rule: InterceptRule) {
        self.rules.write().add(rule);
    }

    pub fn should_intercept(&self, host: &str, path: &str) -> bool {
        let rules = self.rules.read();
        matches!(rules.evaluate(host, path, ""), RuleAction::Intercept)
    }

    pub fn should_monitor(&self, host: &str, path: &str) -> bool {
        let rules = self.rules.read();
        matches!(rules.evaluate(host, path, ""), RuleAction::Monitor | RuleAction::Intercept)
    }

    pub async fn wait_for_decision(&self, _event: &InterceptEvent) -> Result<InterceptDecision> {
        if self.event_tx.is_none() {
            return Ok(InterceptDecision::Allow);
        }

        tokio::time::timeout(self.config.timeout, async {
            tokio::time::sleep(Duration::MAX).await
        })
        .await
        .map_err(|_| crate::error::SlapperError::Proxy("Intercept timeout".to_string()))?;

        Ok(InterceptDecision::Allow)
    }

    pub fn modify_request(&self, request: &mut InterceptRequest, modification: &RequestModification) {
        if let Some(ref headers) = modification.headers {
            for (k, v) in headers {
                if !validate_header_value(&k) || !validate_header_value(v) {
                    tracing::warn!("Blocked CRLF injection attempt in header: {}={}", k, v);
                    continue;
                }
                request.headers.insert(k.clone(), v.clone());
            }
        }

        if let Some(ref path) = modification.path {
            request.path = path.clone();
        }

        if let Some(ref body) = modification.body {
            request.body = Some(body.clone());
        }
    }

    pub fn modify_response(&self, response: &mut InterceptResponse, modification: &ResponseModification) {
        if let Some(ref headers) = modification.headers {
            for (k, v) in headers {
                response.headers.insert(k.clone(), v.clone());
            }
        }

        if let Some(ref body) = modification.body {
            response.body = Some(body.clone());
        }

        if let Some(status) = modification.status_code {
            response.status_code = status;
        }
    }
}

#[derive(Debug, Clone)]
pub struct InterceptRequest {
    pub method: String,
    pub path: String,
    pub headers: std::collections::HashMap<String, String>,
    pub body: Option<String>,
    pub host: String,
}

#[derive(Debug, Clone)]
pub struct InterceptResponse {
    pub status_code: u16,
    pub headers: std::collections::HashMap<String, String>,
    pub body: Option<String>,
}

#[derive(Debug, Clone)]
pub enum InterceptEvent {
    Request(InterceptRequest),
    Response(InterceptResponse, InterceptRequest),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterceptDecision {
    Allow,
    Block,
    Drop,
}

#[derive(Debug, Clone)]
pub struct RequestModification {
    pub headers: Option<std::collections::HashMap<String, String>>,
    pub path: Option<String>,
    pub body: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResponseModification {
    pub headers: Option<std::collections::HashMap<String, String>>,
    pub body: Option<String>,
    pub status_code: Option<u16>,
}

impl Default for RequestModification {
    fn default() -> Self {
        Self {
            headers: None,
            path: None,
            body: None,
        }
    }
}

impl Default for ResponseModification {
    fn default() -> Self {
        Self {
            headers: None,
            body: None,
            status_code: None,
        }
    }
}

fn validate_header_value(value: &str) -> bool {
    !value.contains('\r') && !value.contains('\n') && !value.contains('\0')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intercept_config_default() {
        let config = InterceptConfig::default();
        assert!(matches!(config.mode, InterceptMode::Monitor));
        assert_eq!(config.buffer_size, 8192);
    }

    #[test]
    fn test_should_intercept() {
        let proxy = InterceptProxy::new(InterceptConfig::default());
        proxy.add_rule(InterceptRule::new(
            "example.com".to_string(),
            Some("/admin".to_string()),
            RuleAction::Intercept,
        ));

        assert!(proxy.should_intercept("example.com", "/admin"));
        assert!(!proxy.should_intercept("example.com", "/public"));
    }

    #[test]
    fn test_validate_header_value() {
        assert!(validate_header_value("normal value"));
        assert!(validate_header_value(""));
        assert!(validate_header_value("value with spaces"));
    }

    #[test]
    fn test_validate_header_value_rejects_crlf() {
        assert!(!validate_header_value("value\r\nmalicious"));
        assert!(!validate_header_value("value\rnext"));
        assert!(!validate_header_value("value\nnext"));
    }

    #[test]
    fn test_validate_header_value_rejects_null() {
        assert!(!validate_header_value("value\0null"));
        assert!(!validate_header_value("\0start"));
    }
}
