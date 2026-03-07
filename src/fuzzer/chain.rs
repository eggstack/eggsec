#![allow(dead_code)]

use reqwest::{Client, Method};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChainAction {
    Request(RequestTemplate),
    ExtractVar(ExtractRule),
    Conditional(Condition),
    Sleep(u64),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestTemplate {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub follow_redirects: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractRule {
    pub from: ExtractionSource,
    pub field: String,
    pub pattern: String,
    pub group: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExtractionSource {
    ResponseBody,
    ResponseHeader(String),
    ResponseStatus,
    Cookie(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    pub check: ConditionCheck,
    pub then: Vec<ChainAction>,
    pub else_: Option<Vec<ChainAction>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConditionCheck {
    StatusCode(u16),
    StatusCodeRange(u16, u16),
    Contains(String),
    RegexMatch(String),
    VariableExists(String),
    VariableEquals(String, String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainResult {
    pub action_index: usize,
    pub success: bool,
    pub status_code: Option<u16>,
    pub response_time_ms: u64,
    pub extracted_vars: HashMap<String, String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainExecutionResult {
    pub success: bool,
    pub total_actions: usize,
    pub successful_actions: usize,
    pub chain_results: Vec<ChainResult>,
    pub final_variables: HashMap<String, String>,
}

pub struct ChainExecutor {
    client: Client,
    variables: HashMap<String, String>,
    results: Vec<ChainResult>,
}

impl ChainExecutor {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            variables: HashMap::new(),
            results: Vec::new(),
        }
    }

    pub async fn execute(&mut self, actions: Vec<ChainAction>) -> ChainExecutionResult {
        let total_actions = actions.len();
        let mut successful = 0;
        
        let mut action_queue: Vec<ChainAction> = actions.into_iter().collect();
        
        while let Some(action) = action_queue.pop() {
            match action {
                ChainAction::Conditional(cond) => {
                    let check_result = self.check_condition(&cond.check).await;
                    
                    if check_result {
                        let mut new_actions: Vec<_> = cond.then.into_iter().collect();
                        new_actions.reverse();
                        action_queue.extend(new_actions);
                    } else if let Some(else_actions) = cond.else_ {
                        let mut new_actions: Vec<_> = else_actions.into_iter().collect();
                        new_actions.reverse();
                        action_queue.extend(new_actions);
                    }
                }
                _ => {
                    let result = self.execute_single_action(action).await;
                    
                    if result.success {
                        successful += 1;
                    }
                    
                    self.results.push(result);
                }
            }
        }

        ChainExecutionResult {
            success: successful == total_actions,
            total_actions,
            successful_actions: successful,
            chain_results: self.results.clone(),
            final_variables: self.variables.clone(),
        }
    }

    async fn execute_single_action(&mut self, action: ChainAction) -> ChainResult {
        match action {
            ChainAction::Request(template) => {
                self.execute_request(template).await
            }
            ChainAction::ExtractVar(rule) => {
                self.execute_extract(rule).await
            }
            ChainAction::Sleep(duration) => {
                tokio::time::sleep(tokio::time::Duration::from_millis(duration)).await;
                ChainResult {
                    action_index: self.results.len(),
                    success: true,
                    status_code: None,
                    response_time_ms: duration,
                    extracted_vars: HashMap::new(),
                    error: None,
                }
            }
            ChainAction::Conditional(_) => {
                unreachable!("Conditional should be handled in execute()")
            }
        }
    }

    async fn execute_action(&mut self, action: ChainAction) -> ChainResult {
        self.execute_single_action(action).await
    }

    async fn execute_request(&mut self, template: RequestTemplate) -> ChainResult {
        let start = std::time::Instant::now();
        
        let url = self.interpolate_string(&template.url);
        let body = template.body.map(|b| self.interpolate_string(&b));
        
        let method = match template.method.to_uppercase().as_str() {
            "GET" => Method::GET,
            "POST" => Method::POST,
            "PUT" => Method::PUT,
            "DELETE" => Method::DELETE,
            "PATCH" => Method::PATCH,
            "HEAD" => Method::HEAD,
            "OPTIONS" => Method::OPTIONS,
            _ => Method::GET,
        };

        let mut request = self.client.request(method, &url);
        
        for (key, value) in &template.headers {
            let interpolated = self.interpolate_string(value);
            request = request.header(key, interpolated);
        }

        if let Some(body_content) = body {
            request = request.body(body_content);
        }

        match request.send().await {
            Ok(response) => {
                let elapsed = start.elapsed().as_millis() as u64;
                let status = response.status().as_u16();
                let is_success = response.status().is_success();
                
                let body_text = response.text().await.unwrap_or_default();
                
                self.variables.insert("_last_status".to_string(), status.to_string());
                self.variables.insert("_last_body".to_string(), body_text.clone());
                
                ChainResult {
                    action_index: self.results.len(),
                    success: is_success,
                    status_code: Some(status),
                    response_time_ms: elapsed,
                    extracted_vars: HashMap::new(),
                    error: None,
                }
            }
            Err(e) => ChainResult {
                action_index: self.results.len(),
                success: false,
                status_code: None,
                response_time_ms: start.elapsed().as_millis() as u64,
                extracted_vars: HashMap::new(),
                error: Some(e.to_string()),
            }
        }
    }

    async fn execute_extract(&mut self, rule: ExtractRule) -> ChainResult {
        let source = match rule.from {
            ExtractionSource::ResponseBody => {
                self.variables.get("_last_body").cloned().unwrap_or_default()
            }
            ExtractionSource::ResponseStatus => {
                self.variables.get("_last_status").cloned().unwrap_or_default()
            }
            ExtractionSource::ResponseHeader(name) => {
                self.variables.get(&format!("_header_{}", name)).cloned().unwrap_or_default()
            }
            ExtractionSource::Cookie(name) => {
                self.variables.get(&format!("_cookie_{}", name)).cloned().unwrap_or_default()
            }
        };

        let pattern = self.interpolate_string(&rule.pattern);
        
        let value = match regex::Regex::new(&pattern) { Ok(re) => {
            if let Some(caps) = re.captures(&source) {
                if let Some(group) = rule.group {
                    caps.get(group).map(|m| m.as_str().to_string())
                } else {
                    caps.get(0).map(|m| m.as_str().to_string())
                }
            } else {
                None
            }
        } _ => {
            if source.contains(&pattern) {
                Some(pattern)
            } else {
                None
            }
        }};

        if let Some(val) = value {
            self.variables.insert(rule.field.clone(), val.clone());
            ChainResult {
                action_index: self.results.len(),
                success: true,
                status_code: None,
                response_time_ms: 0,
                extracted_vars: vec![(rule.field, val)].into_iter().collect(),
                error: None,
            }
        } else {
            ChainResult {
                action_index: self.results.len(),
                success: false,
                status_code: None,
                response_time_ms: 0,
                extracted_vars: HashMap::new(),
                error: Some("Pattern not found".to_string()),
            }
        }
    }

    async fn check_condition(&self, check: &ConditionCheck) -> bool {
        match check {
            ConditionCheck::StatusCode(code) => {
                self.variables.get("_last_status")
                    .map(|s| s == &code.to_string())
                    .unwrap_or(false)
            }
            ConditionCheck::StatusCodeRange(start, end) => {
                self.variables.get("_last_status")
                    .and_then(|s| s.parse::<u16>().ok())
                    .map(|s| s >= *start && s <= *end)
                    .unwrap_or(false)
            }
            ConditionCheck::Contains(needle) => {
                self.variables.get("_last_body")
                    .map(|body| body.contains(needle))
                    .unwrap_or(false)
            }
            ConditionCheck::RegexMatch(pattern) => {
                if let Some(body) = self.variables.get("_last_body") {
                    regex::Regex::new(pattern)
                        .map(|re| re.is_match(body))
                        .unwrap_or(false)
                } else {
                    false
                }
            }
            ConditionCheck::VariableExists(var) => {
                self.variables.contains_key(var)
            }
            ConditionCheck::VariableEquals(var, value) => {
                self.variables.get(var)
                    .map(|v| v == value)
                    .unwrap_or(false)
            }
        }
    }

    fn interpolate_string(&self, input: &str) -> String {
        let mut result = input.to_string();
        
        for (key, value) in &self.variables {
            let placeholder = format!("${{{}}}", key);
            result = result.replace(&placeholder, value);
        }
        
        result
    }

    pub fn set_variable(&mut self, key: &str, value: &str) {
        self.variables.insert(key.to_string(), value.to_string());
    }

    pub fn get_variable(&self, key: &str) -> Option<&String> {
        self.variables.get(key)
    }
}

pub struct AutoExploiter {
    client: Client,
}

impl AutoExploiter {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    pub async fn try_ssrf_exploitation(&self, _detected_var: &str) -> Option<Vec<ChainAction>> {
        let actions = vec![
            ChainAction::ExtractVar(ExtractRule {
                from: ExtractionSource::ResponseBody,
                field: "internal_ip".to_string(),
                pattern: r"\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b".to_string(),
                group: Some(0),
            }),
            ChainAction::Request(RequestTemplate {
                method: "GET".to_string(),
                url: "http://${internal_ip}/admin".to_string(),
                headers: HashMap::new(),
                body: None,
                follow_redirects: true,
            }),
        ];
        
        Some(actions)
    }

    pub async fn try_sqli_exploitation(&self, injection_point: &str) -> Option<Vec<ChainAction>> {
        let _actions = vec![
            ChainAction::Request(RequestTemplate {
                method: "GET".to_string(),
                url: format!("{}${{}}", injection_point),
                headers: HashMap::new(),
                body: None,
                follow_redirects: false,
            }),
        ];
        
        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainedFuzzResult {
    pub original_result: crate::fuzzer::FuzzResult,
    pub follow_up_results: Vec<ChainResult>,
    pub exploitation_successful: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_interpolation() {
        let mut executor = ChainExecutor::new(Client::new());
        executor.set_variable("test", "value");
        
        let result = executor.interpolate_string("${test}_suffix");
        assert_eq!(result, "value_suffix");
    }
}
