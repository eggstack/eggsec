use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeakMatch {
    pub pattern: String,
    pub category: LeakCategory,
    pub severity: LeakSeverity,
    pub context: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum LeakCategory {
    DatabaseError,
    StackTrace,
    FilePath,
    SensitiveData,
    DebugInfo,
    Configuration,
    Credentials,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum LeakSeverity {
    Critical,
    High,
    Medium,
    Low,
}

impl std::fmt::Display for LeakCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LeakCategory::DatabaseError => write!(f, "Database Error"),
            LeakCategory::StackTrace => write!(f, "Stack Trace"),
            LeakCategory::FilePath => write!(f, "File Path"),
            LeakCategory::SensitiveData => write!(f, "Sensitive Data"),
            LeakCategory::DebugInfo => write!(f, "Debug Info"),
            LeakCategory::Configuration => write!(f, "Configuration"),
            LeakCategory::Credentials => write!(f, "Credentials"),
        }
    }
}

static PATTERNS: Lazy<Vec<(String, LeakCategory, LeakSeverity)>> = Lazy::new(get_all_patterns);
static MATCHER: Lazy<AhoCorasick> = Lazy::new(|| {
    let pattern_strings: Vec<&str> = PATTERNS.iter().map(|(p, _, _)| p.as_str()).collect();
    AhoCorasickBuilder::new()
        .match_kind(MatchKind::LeftmostLongest)
        .build(&pattern_strings)
        .expect("Failed to create Aho-Corasick matcher")
});

#[derive(Clone)]
pub struct PatternMatcher;

impl PatternMatcher {
    pub fn new() -> Self {
        Self
    }

    pub fn scan(&self, text: &str) -> Vec<LeakMatch> {
        let mut matches = Vec::with_capacity(8);

        for mat in MATCHER.find_iter(text) {
            let pattern_idx = mat.pattern().as_usize();
            if let Some((pattern, category, severity)) = PATTERNS.get(pattern_idx) {
                let start = mat.start().saturating_sub(50);
                let end = (mat.end() + 50).min(text.len());
                let context = text.get(start..end).map(|s| s.to_string());

                matches.push(LeakMatch {
                    pattern: pattern.clone(),
                    category: *category,
                    severity: *severity,
                    context,
                });
            }
        }

        matches.sort_by(|a, b| b.severity.ordinal().cmp(&a.severity.ordinal()));

        matches
    }
}

impl Default for PatternMatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl LeakSeverity {
    fn ordinal(&self) -> u8 {
        match self {
            LeakSeverity::Critical => 4,
            LeakSeverity::High => 3,
            LeakSeverity::Medium => 2,
            LeakSeverity::Low => 1,
        }
    }
}

fn get_all_patterns() -> Vec<(String, LeakCategory, LeakSeverity)> {
    let mut patterns = Vec::new();

    let db_patterns = get_database_patterns();
    for (pattern, severity) in db_patterns {
        patterns.push((pattern, LeakCategory::DatabaseError, severity));
    }

    let stack_patterns = get_stack_trace_patterns();
    for (pattern, severity) in stack_patterns {
        patterns.push((pattern, LeakCategory::StackTrace, severity));
    }

    let file_patterns = get_file_path_patterns();
    for (pattern, severity) in file_patterns {
        patterns.push((pattern, LeakCategory::FilePath, severity));
    }

    let sensitive_patterns = get_sensitive_patterns();
    for (pattern, severity) in sensitive_patterns {
        patterns.push((pattern, LeakCategory::SensitiveData, severity));
    }

    patterns
}

fn get_database_patterns() -> Vec<(String, LeakSeverity)> {
    vec![
        ("SQL syntax".to_string(), LeakSeverity::High),
        ("mysql_fetch".to_string(), LeakSeverity::High),
        ("ORA-".to_string(), LeakSeverity::High),
        ("PLS-".to_string(), LeakSeverity::High),
        (
            "Unclosed quotation mark".to_string(),
            LeakSeverity::Critical,
        ),
        (
            "quoted string not properly terminated".to_string(),
            LeakSeverity::Critical,
        ),
        (
            "You have an error in your SQL syntax".to_string(),
            LeakSeverity::Critical,
        ),
        ("Warning: mysql_".to_string(), LeakSeverity::High),
        ("PostgreSQL query failed".to_string(), LeakSeverity::High),
        ("pg_query()".to_string(), LeakSeverity::Medium),
        ("pg_exec()".to_string(), LeakSeverity::Medium),
        ("SQLSTATE[".to_string(), LeakSeverity::High),
        ("PDO::SQLSTATE".to_string(), LeakSeverity::High),
        ("Microsoft OLE DB Provider".to_string(), LeakSeverity::High),
        (
            "ODBC Microsoft Access Driver".to_string(),
            LeakSeverity::Medium,
        ),
        (
            "Syntax error in string in query expression".to_string(),
            LeakSeverity::Critical,
        ),
        (
            "Data type mismatch in criteria expression".to_string(),
            LeakSeverity::High,
        ),
    ]
}

fn get_stack_trace_patterns() -> Vec<(String, LeakSeverity)> {
    vec![
        ("at java.".to_string(), LeakSeverity::Medium),
        ("at org.".to_string(), LeakSeverity::Medium),
        ("at com.".to_string(), LeakSeverity::Medium),
        ("at net.".to_string(), LeakSeverity::Medium),
        (
            "Traceback (most recent call last)".to_string(),
            LeakSeverity::Medium,
        ),
        ("File \"/".to_string(), LeakSeverity::Medium),
        ("line ".to_string(), LeakSeverity::Low),
        ("PHP Fatal error".to_string(), LeakSeverity::High),
        ("PHP Warning".to_string(), LeakSeverity::Medium),
        ("PHP Notice".to_string(), LeakSeverity::Low),
        ("PHP Stack trace".to_string(), LeakSeverity::High),
        ("#0 ".to_string(), LeakSeverity::Low),
        ("Stack trace:".to_string(), LeakSeverity::Medium),
        ("at System.".to_string(), LeakSeverity::Medium),
        ("at Microsoft.".to_string(), LeakSeverity::Medium),
        ("   at ".to_string(), LeakSeverity::Low),
        ("Error: ".to_string(), LeakSeverity::Low),
        ("Exception: ".to_string(), LeakSeverity::Medium),
        ("Uncaught exception".to_string(), LeakSeverity::High),
    ]
}

fn get_file_path_patterns() -> Vec<(String, LeakSeverity)> {
    vec![
        ("/etc/passwd".to_string(), LeakSeverity::Critical),
        ("/etc/shadow".to_string(), LeakSeverity::Critical),
        ("/etc/hosts".to_string(), LeakSeverity::Medium),
        ("/var/log/".to_string(), LeakSeverity::Medium),
        ("/home/".to_string(), LeakSeverity::Medium),
        ("/root/".to_string(), LeakSeverity::High),
        ("/usr/local/".to_string(), LeakSeverity::Low),
        ("C:\\Windows".to_string(), LeakSeverity::High),
        ("C:\\Users".to_string(), LeakSeverity::Medium),
        ("C:\\inetpub".to_string(), LeakSeverity::High),
        ("/var/www/".to_string(), LeakSeverity::High),
        ("/app/".to_string(), LeakSeverity::Medium),
        (".env".to_string(), LeakSeverity::Critical),
        ("config.php".to_string(), LeakSeverity::High),
        ("wp-config.php".to_string(), LeakSeverity::Critical),
        ("database.yml".to_string(), LeakSeverity::Critical),
        ("settings.py".to_string(), LeakSeverity::High),
        ("application.properties".to_string(), LeakSeverity::High),
    ]
}

fn get_sensitive_patterns() -> Vec<(String, LeakSeverity)> {
    vec![
        ("password".to_string(), LeakSeverity::Critical),
        ("passwd".to_string(), LeakSeverity::Critical),
        ("api_key".to_string(), LeakSeverity::Critical),
        ("apikey".to_string(), LeakSeverity::Critical),
        ("api-key".to_string(), LeakSeverity::Critical),
        ("secret_key".to_string(), LeakSeverity::Critical),
        ("secretkey".to_string(), LeakSeverity::Critical),
        ("secret-key".to_string(), LeakSeverity::Critical),
        ("access_token".to_string(), LeakSeverity::Critical),
        ("accesstoken".to_string(), LeakSeverity::Critical),
        ("access-token".to_string(), LeakSeverity::Critical),
        ("refresh_token".to_string(), LeakSeverity::Critical),
        ("auth_token".to_string(), LeakSeverity::Critical),
        ("authtoken".to_string(), LeakSeverity::Critical),
        ("private_key".to_string(), LeakSeverity::Critical),
        ("privatekey".to_string(), LeakSeverity::Critical),
        ("private-key".to_string(), LeakSeverity::Critical),
        (
            "-----BEGIN RSA PRIVATE KEY-----".to_string(),
            LeakSeverity::Critical,
        ),
        (
            "-----BEGIN PRIVATE KEY-----".to_string(),
            LeakSeverity::Critical,
        ),
        (
            "-----BEGIN OPENSSH PRIVATE KEY-----".to_string(),
            LeakSeverity::Critical,
        ),
        ("aws_access_key_id".to_string(), LeakSeverity::Critical),
        ("aws_secret_access_key".to_string(), LeakSeverity::Critical),
        ("AWS_ACCESS_KEY".to_string(), LeakSeverity::Critical),
        ("AWS_SECRET_KEY".to_string(), LeakSeverity::Critical),
        ("connection string".to_string(), LeakSeverity::High),
        ("jdbc:".to_string(), LeakSeverity::High),
        ("mysql://".to_string(), LeakSeverity::High),
        ("postgres://".to_string(), LeakSeverity::High),
        ("mongodb://".to_string(), LeakSeverity::High),
        ("redis://".to_string(), LeakSeverity::High),
    ]
}
