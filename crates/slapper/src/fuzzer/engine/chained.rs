use crate::cli::FuzzArgs;
use crate::error::Result;
use crate::fuzzer::chain::ExtractRule;
use crate::fuzzer::chain::ExtractionSource;
use crate::fuzzer::engine::types::FuzzSession;
use crate::fuzzer::FuzzEngine;
use regex::Regex;
use reqwest::Client;
use rustc_hash::FxHashMap;

#[derive(Clone)]
pub struct FuzzChainStep {
    pub args: FuzzArgs,
    pub extract_rules: Vec<ExtractRule>,
}

impl std::fmt::Debug for FuzzChainStep {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FuzzChainStep")
            .field("args", &"<FuzzArgs>")
            .field("extract_rules", &self.extract_rules)
            .finish()
    }
}

#[derive(Clone)]
pub struct ChainedFuzzInput {
    pub initial_args: FuzzArgs,
    pub steps: Vec<FuzzChainStep>,
}

impl std::fmt::Debug for ChainedFuzzInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChainedFuzzInput")
            .field("initial_args", &"<FuzzArgs>")
            .field("steps", &self.steps)
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct ChainedFuzzOutput {
    pub step_results: Vec<StepResults>,
    pub final_variables: FxHashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct StepResults {
    pub step_index: usize,
    pub session: FuzzSession,
}

pub struct StatefulFuzzer {
    _client: Client,
    variables: FxHashMap<String, String>,
}

impl StatefulFuzzer {
    #[allow(dead_code)]
    pub fn new(client: Client) -> Self {
        Self {
            _client: client,
            variables: FxHashMap::default(),
        }
    }

    pub async fn run_chain(&mut self, chain: ChainedFuzzInput) -> Result<ChainedFuzzOutput> {
        let mut step_results = Vec::new();
        self.variables.clear();

        let mut engine = FuzzEngine::new(chain.initial_args.clone())?;
        let session = engine.run_return_session().await?;

        self.extract_variables_from_session(&session);
        step_results.push(StepResults {
            step_index: 0,
            session,
        });

        for (idx, step) in chain.steps.into_iter().enumerate() {
            let modified_args = self.apply_variables_to_args(step.args);
            let mut engine = FuzzEngine::new(modified_args)?;
            let mut session = engine.run_return_session().await?;

            for rule in step.extract_rules {
                self.apply_extract_rule(&mut session, &rule);
            }
            self.extract_variables_from_session(&session);

            step_results.push(StepResults {
                step_index: idx + 1,
                session,
            });
        }

        Ok(ChainedFuzzOutput {
            step_results,
            final_variables: self.variables.clone(),
        })
    }

    fn extract_variables_from_session(&mut self, session: &FuzzSession) {
        if let Some(result) = session.results.last() {
            self.variables
                .insert("_last_status".to_string(), result.status_code.to_string());
        }

        for result in &session.results {
            if !result.leaks_found.is_empty() {
                if let Some(ref leak) = result.leaks_found.first() {
                    self.variables
                        .insert("_last_leak".to_string(), leak.to_string());
                }
            }
        }
    }

    fn apply_extract_rule(&mut self, session: &mut FuzzSession, rule: &ExtractRule) {
        let source = match &rule.from {
            ExtractionSource::ResponseBody => session
                .results
                .iter()
                .rev()
                .find_map(|r| r.response_body.clone())
                .unwrap_or_default(),
            ExtractionSource::ResponseStatus => session
                .results
                .last()
                .map(|r| r.status_code.to_string())
                .unwrap_or_default(),
            ExtractionSource::ResponseHeader(_) | ExtractionSource::Cookie(_) => {
                String::new()
            }
        };

        if source.is_empty() {
            return;
        }

        let value = if let Ok(re) = Regex::new(&rule.pattern) {
            re.captures(&source).and_then(|caps| {
                if let Some(group) = rule.group {
                    caps.get(group).map(|m| m.as_str().to_string())
                } else {
                    caps.get(0).map(|m| m.as_str().to_string())
                }
            })
        } else if source.contains(&rule.pattern) {
            Some(rule.pattern.clone())
        } else {
            None
        };

        if let Some(value) = value {
            self.variables.insert(rule.field.clone(), value);
        }
    }

    fn apply_variables_to_args(&self, mut args: FuzzArgs) -> FuzzArgs {
        if args.param.is_none() {
            if let Some(idx) = args.url.find("${") {
                let start = idx + 2;
                let end = args.url[start..]
                    .find('}')
                    .map(|i| start + i)
                    .unwrap_or(idx);
                let var_name = &args.url[start..end];
                if let Some(value) = self.variables.get(var_name) {
                    let placeholder = format!("${{{}}}", var_name);
                    args.url = args.url.replace(&placeholder, value);
                }
            }
        } else if let Some(ref param) = args.param {
            if let Some(value) = self.variables.get(param) {
                if args.url.contains(&format!("${{{}}}", param)) {
                    args.url = args.url.replace(&format!("${{{}}}", param), value);
                }
            }
        }

        args
    }

    pub fn set_variable(&mut self, key: &str, value: &str) {
        self.variables.insert(key.to_string(), value.to_string());
    }

    pub fn get_variable(&self, key: &str) -> Option<&String> {
        self.variables.get(key)
    }

    pub fn get_all_variables(&self) -> &FxHashMap<String, String> {
        &self.variables
    }
}

impl FuzzArgs {
    pub fn with_variable(mut self, key: &str, value: &str) -> Self {
        if self.url.contains(&format!("${{{}}}", key)) {
            self.url = self.url.replace(&format!("${{{}}}", key), value);
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{CommonHttpArgs, FuzzMode};

    fn make_test_args(url: &str) -> FuzzArgs {
        FuzzArgs {
            url: url.to_string(),
            payload_type: "sqli".to_string(),
            common: CommonHttpArgs::default(),
            method: "GET".to_string(),
            param: None,
            concurrency: 10,
            timeout: 5,
            verbose: false,
            quiet: false,
            json: false,
            output: None,
            mutate: false,
            mutation_count: 5,
            grammar_fuzz: false,
            grammar_type: None,
            session: false,
            diffing: false,
            capture_baseline: false,
            mode: FuzzMode::Sequential,
            target: None,
            graphql_introspection: false,
            graphql_depth_bypass: false,
            graphql_alias_overload: false,
            jwt_token: None,
            oauth_client_id: None,
            oauth_client_secret: None,
            oauth_redirect: false,
            oauth_scope: false,
            oauth_state: false,
            oauth_grant: false,
            oauth_issuer: None,
            idor_base_id: None,
            idor_user_ids: None,
            ssti_param: None,
            adaptive_rate: false,
            enhanced_redos: false,
            waf_fingerprint: false,
            chaining: false,
            chain_file: None,
            format: None,
            schema: None,
            discover_only: false,
            auto_discover_schema: false,
            calibrate: false,
            fc: None,
            fs: None,
            fw: None,
            fl: None,
            ft: None,
            fr: None,
        }
    }

    #[test]
    fn test_fuzz_args_with_variable() {
        let args = make_test_args("http://example.com/api/${user_id}");
        let modified = args.with_variable("user_id", "12345");
        assert!(modified.url.contains("12345"));
        assert!(!modified.url.contains("${user_id}"));
    }

    #[test]
    fn test_fuzz_args_with_variable_no_match() {
        let args = make_test_args("http://example.com/api/resource");
        let modified = args.with_variable("user_id", "12345");
        assert_eq!(modified.url, "http://example.com/api/resource");
    }

    #[test]
    fn test_stateful_fuzzer_variable_interpolation() {
        let client = Client::new();
        let mut fuzzer = StatefulFuzzer::new(client);

        fuzzer.set_variable("user_id", "12345");
        let args = make_test_args("http://example.com/api/${user_id}");
        let modified = fuzzer.apply_variables_to_args(args);
        assert!(modified.url.contains("12345"));
    }

    #[test]
    fn test_stateful_fuzzer_get_variable() {
        let client = Client::new();
        let mut fuzzer = StatefulFuzzer::new(client);

        fuzzer.set_variable("test_key", "test_value");
        assert_eq!(
            fuzzer.get_variable("test_key"),
            Some(&"test_value".to_string())
        );
        assert_eq!(fuzzer.get_variable("nonexistent"), None);
    }

    #[test]
    fn test_stateful_fuzzer_get_all_variables() {
        let client = Client::new();
        let mut fuzzer = StatefulFuzzer::new(client);

        fuzzer.set_variable("key1", "value1");
        fuzzer.set_variable("key2", "value2");

        let vars = fuzzer.get_all_variables();
        assert_eq!(vars.len(), 2);
        assert_eq!(vars.get("key1"), Some(&"value1".to_string()));
        assert_eq!(vars.get("key2"), Some(&"value2".to_string()));
    }

    #[test]
    fn test_apply_extract_rule_from_response_body_sets_variable() {
        let client = Client::new();
        let mut fuzzer = StatefulFuzzer::new(client);
        let mut session = FuzzSession {
            target_url: "http://example.com".to_string(),
            mode: "Sequential".to_string(),
            payload_type: "sqli".to_string(),
            total_payloads: 1,
            successful_requests: 1,
            failed_requests: 0,
            waf_bypasses: 0,
            potential_leaks: 0,
            time_anomalies: 0,
            redos_suspected: 0,
            duration_ms: 1,
            total_requests: 1,
            findings: 0,
            results: vec![crate::fuzzer::FuzzResult {
                payload: crate::fuzzer::Payload {
                    payload_type: crate::fuzzer::PayloadType::Sqli,
                    payload: "x".to_string(),
                    description: "d".to_string(),
                    severity: crate::fuzzer::Severity::Low,
                    tags: vec![],
                },
                status_code: 200,
                response_time_ms: 10,
                response_length: Some(10),
                response_body: Some("token=abc123".to_string()),
                is_waf_blocked: false,
                is_anomaly: false,
                is_redos_suspected: false,
                leaks_found: vec![],
                error: None,
                owasp_category: None,
                detected_severity: crate::fuzzer::Severity::Low,
            }],
            owasp_summary: crate::fuzzer::OwaspSummary {
                a01_broken_access_control: 0,
                a02_cryptographic_failures: 0,
                a03_injection: 0,
                a04_insecure_design: 0,
                a05_security_misconfiguration: 0,
                a06_vulnerable_components: 0,
                a07_auth_failures: 0,
                a08_software_integrity: 0,
                a09_logging_failures: 0,
                a10_ssrf: 0,
            },
            baseline: None,
        };

        let rule = ExtractRule {
            from: ExtractionSource::ResponseBody,
            field: "extracted_token".to_string(),
            pattern: r"token=([a-z0-9]+)".to_string(),
            group: Some(1),
        };

        fuzzer.apply_extract_rule(&mut session, &rule);
        assert_eq!(
            fuzzer.get_variable("extracted_token"),
            Some(&"abc123".to_string())
        );
    }
}
