use regex::{Regex, RegexBuilder};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::sync::LazyLock;
use std::time::{Duration, Instant};
use tokio::task;

static KNOWN_VULNERABLE_PATTERNS: LazyLock<Vec<String>> = LazyLock::new(|| {
    vec![
        "(.+)+".to_string(),
        "(.*)*".to_string(),
        "(a+)+".to_string(),
        "(a*)a*".to_string(),
        "([a-zA-Z]+)*".to_string(),
        "(x+x+)+y".to_string(),
        "(x?x?)+".to_string(),
        "(a{1,100})+".to_string(),
        "(a|a|a)+".to_string(),
        "(.*a){10}".to_string(),
        "(\\d+\\.?\\d*)+".to_string(),
        "(\\w+\\s?)*".to_string(),
        "(a+)+b".to_string(),
        "(?:a|a)+".to_string(),
        "^(.*?)*$".to_string(),
    ]
});

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReDosResult {
    pub pattern: String,
    pub is_vulnerable: bool,
    pub is_match: bool,
    pub execution_time_ms: u64,
    pub iterations: u64,
    pub catastrophic_backtracking: bool,
    pub sample_match: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RegexExecutor {
    timeout: Duration,
    max_iterations: u64,
    test_strings: Vec<String>,
}

impl Default for RegexExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl RegexExecutor {
    pub fn new() -> Self {
        Self {
            timeout: Duration::from_millis(1000),
            max_iterations: 100000,
            test_strings: Self::default_test_strings(),
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_max_iterations(mut self, max: u64) -> Self {
        self.max_iterations = max;
        self
    }

    pub fn with_test_strings(mut self, strings: Vec<String>) -> Self {
        self.test_strings = strings;
        self
    }

    fn default_test_strings() -> Vec<String> {
        vec![
            "a".repeat(10),
            "a".repeat(100),
            "a".repeat(1000),
            "aaaaaaaaaa".to_string(),
            "ababababab".to_string(),
            "a".repeat(5000),
            "abcdefghij".to_string(),
            "1".repeat(100),
            "@".repeat(100),
            "".to_string(),
        ]
    }

    pub fn check_pattern(&self, pattern: &str) -> ReDosResult {
        let start = Instant::now();

        let regex = match RegexBuilder::new(pattern).size_limit(100_000).build() {
            Ok(r) => r,
            Err(e) => {
                return ReDosResult {
                    pattern: pattern.to_string(),
                    is_vulnerable: false,
                    is_match: false,
                    execution_time_ms: start.elapsed().as_millis() as u64,
                    iterations: 0,
                    catastrophic_backtracking: false,
                    sample_match: Some(format!("Invalid regex: {}", e)),
                };
            }
        };

        let iterations = Arc::new(AtomicU64::new(0));
        let timeout_flag = Arc::new(AtomicU64::new(0));

        let mut matched_any = false;

        for test_string in &self.test_strings {
            if start.elapsed() > self.timeout {
                timeout_flag.store(1, Ordering::SeqCst);
                break;
            }

            let iterations_clone = iterations.clone();
            let timeout_clone = timeout_flag.clone();

            let result =
                self.execute_with_limit(&regex, test_string, iterations_clone, timeout_clone);

            if result.is_match {
                matched_any = true;
            }

            if timeout_flag.load(Ordering::SeqCst) == 1 {
                break;
            }
        }

        let elapsed = start.elapsed().as_millis() as u64;
        let iter_count = iterations.load(Ordering::SeqCst);
        let is_vulnerable =
            timeout_flag.load(Ordering::SeqCst) == 1 || iter_count > self.max_iterations / 10;

        ReDosResult {
            pattern: pattern.to_string(),
            is_vulnerable,
            is_match: matched_any,
            execution_time_ms: elapsed,
            iterations: iter_count,
            catastrophic_backtracking: is_vulnerable,
            sample_match: if matched_any {
                Some("matched".to_string())
            } else {
                None
            },
        }
    }

    fn execute_with_limit(
        &self,
        regex: &Regex,
        input: &str,
        iterations: Arc<AtomicU64>,
        timeout_flag: Arc<AtomicU64>,
    ) -> RegexMatchResult {
        let mut last_len = 0;

        for _ in 0.. {
            if iterations.fetch_add(1, Ordering::SeqCst) > self.max_iterations {
                timeout_flag.store(1, Ordering::SeqCst);
                break;
            }

            if timeout_flag.load(Ordering::SeqCst) == 1 {
                break;
            }

            match regex.find(input) {
                Some(m) => {
                    if m.len() == last_len {
                        break;
                    }
                    last_len = m.len();
                }
                None => break,
            }
        }

        RegexMatchResult {
            is_match: last_len > 0,
            match_length: last_len,
        }
    }

    pub fn check_patterns(&self, patterns: &[String]) -> Vec<ReDosResult> {
        patterns.iter().map(|p| self.check_pattern(p)).collect()
    }

    pub async fn check_pattern_async(&self, pattern: &str) -> ReDosResult {
        let executor = self.clone();
        let pattern_owned = pattern.to_string();

        let result = task::spawn_blocking(move || executor.check_pattern(&pattern_owned)).await;

        match result {
            Ok(r) => r,
            Err(_) => ReDosResult {
                pattern: pattern.to_string(),
                is_vulnerable: false,
                is_match: false,
                execution_time_ms: 0,
                iterations: 0,
                catastrophic_backtracking: false,
                sample_match: Some("Task failed".to_string()),
            },
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct RegexMatchResult {
    is_match: bool,
    match_length: usize,
}

pub struct ReDosDetector {
    executor: RegexExecutor,
    known_vulnerable_patterns: Vec<String>,
}

impl Default for ReDosDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl ReDosDetector {
    pub fn new() -> Self {
        Self {
            executor: RegexExecutor::new(),
            known_vulnerable_patterns: KNOWN_VULNERABLE_PATTERNS.clone(),
        }
    }

    pub fn with_executor(mut self, executor: RegexExecutor) -> Self {
        self.executor = executor;
        self
    }

    pub fn detect(&self, pattern: &str) -> ReDosResult {
        if self
            .known_vulnerable_patterns
            .iter()
            .any(|p| pattern.contains(p))
        {
            return ReDosResult {
                pattern: pattern.to_string(),
                is_vulnerable: true,
                is_match: false,
                execution_time_ms: 0,
                iterations: 0,
                catastrophic_backtracking: true,
                sample_match: Some("Known vulnerable pattern".to_string()),
            };
        }

        self.executor.check_pattern(pattern)
    }

    pub fn add_known_pattern(&mut self, pattern: &str) {
        self.known_vulnerable_patterns.push(pattern.to_string());
    }
}

pub struct PayloadReDosChecker {
    detector: ReDosDetector,
    vulnerable_payloads: FxHashMap<String, Vec<String>>,
}

impl Default for PayloadReDosChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl PayloadReDosChecker {
    pub fn new() -> Self {
        Self {
            detector: ReDosDetector::new(),
            vulnerable_payloads: FxHashMap::default(),
        }
    }

    pub fn check_payload_patterns(&self, description: &str) -> Vec<ReDosResult> {
        let patterns = self.extract_regex_patterns(description);
        patterns.iter().map(|p| self.detector.detect(p)).collect()
    }

    fn extract_regex_patterns(&self, text: &str) -> Vec<String> {
        let mut patterns = Vec::new();
        let mut in_bracket = false;
        let mut current = String::new();

        for c in text.chars() {
            match c {
                '/' if !in_bracket => {
                    in_bracket = true;
                    current.clear();
                }
                '/' if in_bracket => {
                    in_bracket = false;
                    if !current.is_empty() {
                        patterns.push(current.clone());
                    }
                }
                c if in_bracket => current.push(c),
                _ => {}
            }
        }

        if patterns.is_empty() && (text.contains("regex") || text.contains("pattern")) {
            patterns.push(text.to_string());
        }

        patterns
    }

    pub fn add_vulnerable_payload(&mut self, payload: &str, patterns: Vec<String>) {
        self.vulnerable_payloads
            .insert(payload.to_string(), patterns);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regex_executor() {
        let executor = RegexExecutor::new();

        let result = executor.check_pattern(r"a+");
        assert!(!result.is_vulnerable);
        assert!(result.is_match);
    }

    #[test]
    fn test_redos_detector() {
        let detector = ReDosDetector::new();

        let result = detector.detect(r"a+");
        assert!(!result.is_vulnerable);

        let result = detector.detect(r"(.+)+");
        assert!(result.is_vulnerable);
    }
}
