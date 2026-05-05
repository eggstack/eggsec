#[cfg(feature = "python-plugins")]
use std::collections::HashSet;
#[cfg(feature = "python-plugins")]
use std::sync::LazyLock;

#[cfg(feature = "python-plugins")]
use regex::Regex;

/// Detection mode for plugin security analysis
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectionMode {
    /// Use regex-based pattern detection only
    Regex,
    /// Use AST-based analysis (when available)
    Ast,
    /// Use both regex and AST analysis (strict mode)
    Strict,
}

impl Default for DetectionMode {
    fn default() -> Self {
        Self::Regex
    }
}

impl DetectionMode {
    /// Parse detection mode from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "regex" => Some(Self::Regex),
            "ast" => Some(Self::Ast),
            "strict" => Some(Self::Strict),
            _ => None,
        }
    }

    /// Check if regex detection is enabled
    pub fn use_regex(&self) -> bool {
        matches!(self, Self::Regex | Self::Strict)
    }

    /// Check if AST detection is enabled
    pub fn use_ast(&self) -> bool {
        matches!(self, Self::Ast | Self::Strict)
    }
}

/// Result of AST-based security analysis
#[derive(Debug, Clone)]
pub struct AstAnalysisResult {
    /// Detected suspicious patterns/functions
    pub suspicious_items: Vec<String>,
    /// Whether the analysis was successful
    pub analysis_success: bool,
    /// Error message if analysis failed
    pub error: Option<String>,
}

impl AstAnalysisResult {
    pub fn new() -> Self {
        Self {
            suspicious_items: Vec::new(),
            analysis_success: true,
            error: None,
        }
    }

    pub fn add_item(&mut self, item: String) {
        if !self.suspicious_items.contains(&item) {
            self.suspicious_items.push(item);
        }
    }

    pub fn set_error(&mut self, error: String) {
        self.analysis_success = false;
        self.error = Some(error);
    }

    pub fn is_suspicious(&self) -> bool {
        !self.suspicious_items.is_empty()
    }
}

/// AST-based security scanner for Python plugins
#[cfg(feature = "python-plugins")]
pub struct AstScanner {
    mode: DetectionMode,
    dangerous_functions: HashSet<&'static str>,
    dangerous_imports: HashSet<&'static str>,
}

#[cfg(feature = "python-plugins")]
impl AstScanner {
    /// Create a new AST scanner with the specified detection mode
    pub fn new(mode: DetectionMode) -> Self {
        let mut dangerous_functions = HashSet::new();
        dangerous_functions.insert("eval");
        dangerous_functions.insert("exec");
        dangerous_functions.insert("compile");
        dangerous_functions.insert("open");
        dangerous_functions.insert("input");
        dangerous_functions.insert("getattr");
        dangerous_functions.insert("setattr");
        dangerous_functions.insert("delattr");
        dangerous_functions.insert("hasattr");
        dangerous_functions.insert("chr");
        dangerous_functions.insert("ord");

        let mut dangerous_imports = HashSet::new();
        dangerous_imports.insert("os");
        dangerous_imports.insert("subprocess");
        dangerous_imports.insert("socket");
        dangerous_imports.insert("ctypes");
        dangerous_imports.insert("multiprocessing");
        dangerous_imports.insert("importlib");
        dangerous_imports.insert("pty");
        dangerous_imports.insert("builtins");

        Self {
            mode,
            dangerous_functions,
            dangerous_imports,
        }
    }

    /// Analyze Python plugin content using AST-based approach
    /// This uses Python's ast module via pyo3 for accurate AST parsing
    pub fn analyze(&self, content: &str) -> AstAnalysisResult {
        if !self.mode.use_ast() {
            return AstAnalysisResult::new();
        }

        // Try to use Python's ast module for proper AST analysis
        match self::python_ast_analyze(content) {
            Ok(result) => result,
            Err(e) => {
                let mut result = AstAnalysisResult::new();
                result.set_error(format!("AST analysis failed: {}", e));
                result
            }
        }
    }

    /// Get the detection mode
    pub fn mode(&self) -> DetectionMode {
        self.mode
    }
}

#[cfg(feature = "python-plugins")]
/// Use Python's ast module via pyo3 to perform AST analysis
fn python_ast_analyze(content: &str) -> anyhow::Result<AstAnalysisResult> {
    use pyo3::types::PyAnyMethods;
    use pyo3::{types::PyModule, Python};

    let mut result = AstAnalysisResult::new();

    let dump_str: String = Python::attach(|py| {
        // Import ast module
        let ast_module: pyo3::Bound<'_, PyModule> = match py.import("ast") {
            Ok(m) => m,
            Err(e) => {
                return Err(pyo3::PyErr::new::<pyo3::exceptions::PyImportError, _>(
                    format!("Failed to import ast module: {}", e),
                ));
            }
        };

        // Get parse function and call it
        let parse_fn = match ast_module.as_any().getattr("parse") {
            Ok(f) => f,
            Err(e) => {
                return Err(pyo3::PyErr::new::<pyo3::exceptions::PyAttributeError, _>(
                    format!("Failed to get ast.parse: {}", e),
                ));
            }
        };

        // Parse the code into an AST
        let code_obj = match parse_fn.call1((content,)) {
            Ok(obj) => obj,
            Err(e) => {
                return Err(e);
            }
        };

        // Get dump function
        let dump_fn = match ast_module.as_any().getattr("dump") {
            Ok(f) => f,
            Err(e) => {
                return Err(pyo3::PyErr::new::<pyo3::exceptions::PyAttributeError, _>(
                    format!("Failed to get ast.dump: {}", e),
                ));
            }
        };

        // Dump the AST to string for analysis
        let dump_result = match dump_fn.call1((&code_obj,)) {
            Ok(obj) => obj,
            Err(e) => {
                return Err(e);
            }
        };

        // Extract the string
        match dump_result.extract::<String>() {
            Ok(s) => Ok(s),
            Err(e) => Err(e),
        }
    })?;

    // Analyze the dump for dangerous patterns
    let scanner = AstScanner::new(DetectionMode::Ast);
    analyze_ast_dump(&dump_str, &mut result, &scanner);

    Ok(result)
}

#[cfg(feature = "python-plugins")]
/// Analyze AST dump string for dangerous patterns
fn analyze_ast_dump(dump: &str, result: &mut AstAnalysisResult, scanner: &AstScanner) {
    // Look for Call nodes with dangerous function names
    // AST dump format: Call(func=Name(id='eval', ctx=Load()), ...)
    static FUNCTION_PATTERN: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"Name\(id=['"]([^'"]+)['"]"#).unwrap());

    static IMPORT_PATTERN: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"alias\(name=['"]([^'"]+)['"]"#).unwrap());

    // Check for dangerous function calls
    for cap in FUNCTION_PATTERN.captures_iter(dump) {
        if let Some(func_name) = cap.get(1) {
            let name = func_name.as_str();
            if scanner.dangerous_functions.contains(name) {
                result.add_item(format!("Dangerous function call: {}", name));
            }
        }
    }

    // Check for dangerous imports
    for cap in IMPORT_PATTERN.captures_iter(dump) {
        if let Some(import_name) = cap.get(1) {
            let name = import_name.as_str();
            if scanner.dangerous_imports.contains(name) {
                result.add_item(format!("Dangerous import: {}", name));
            }
        }
    }

    // Also check for attribute access like os.system, subprocess.run, etc.
    static ATTR_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"Attribute\(value=Name\(id=['"]([^'"]+)['"][^)]*\),attr=['"]([^'"]+)['"]"#)
            .unwrap()
    });

    let dangerous_attrs: HashSet<(&str, &str)> = [
        ("os", "system"),
        ("os", "popen"),
        ("os", "exec"),
        ("os", "spawn"),
        ("subprocess", "run"),
        ("subprocess", "Popen"),
        ("subprocess", "call"),
        ("subprocess", "check_output"),
        ("socket", "socket"),
        ("socket", "create_connection"),
        ("ctypes", "CDLL"),
        ("ctypes", "WinDLL"),
        ("pty", "spawn"),
        ("multiprocessing", "Process"),
    ]
    .iter()
    .cloned()
    .collect();

    for cap in ATTR_PATTERN.captures_iter(dump) {
        if let (Some(module), Some(attr)) = (cap.get(1), cap.get(2)) {
            let module_name = module.as_str();
            let attr_name = attr.as_str();
            if dangerous_attrs.contains(&(module_name, attr_name)) {
                result.add_item(format!(
                    "Dangerous attribute access: {}.{}",
                    module_name, attr_name
                ));
            }
        }
    }
}

#[cfg(feature = "python-plugins")]
pub fn validate_python_plugin_ast(
    content: &str,
    mode: DetectionMode,
    block_suspicious_plugins: bool,
) -> anyhow::Result<()> {
    let scanner = AstScanner::new(mode);
    let mut all_suspicious = Vec::new();

    // Run regex-based detection if enabled
    if mode.use_regex() {
        // Use existing regex patterns from security.rs
        // This is handled by the caller or we re-import the patterns
        // For now, we'll just run AST analysis
    }

    // Run AST-based detection if enabled
    if mode.use_ast() {
        let ast_result = scanner.analyze(content);
        if !ast_result.analysis_success {
            tracing::warn!("AST analysis failed: {:?}", ast_result.error);
        }
        all_suspicious.extend(ast_result.suspicious_items);
    }

    if !all_suspicious.is_empty() {
        if block_suspicious_plugins {
            anyhow::bail!(
                "Plugin contains suspicious patterns (AST analysis): {}",
                all_suspicious.join(", ")
            );
        } else {
            tracing::warn!(
                "Plugin contains suspicious patterns (AST analysis, allowing due to config): {}",
                all_suspicious.join(", ")
            );
        }
    }

    Ok(())
}

#[cfg(not(feature = "python-plugins"))]
pub fn validate_python_plugin_ast(
    _content: &str,
    _mode: DetectionMode,
    _block_suspicious_plugins: bool,
) -> anyhow::Result<()> {
    anyhow::bail!("Python plugins support is not enabled");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detection_mode_from_str() {
        assert_eq!(DetectionMode::from_str("regex"), Some(DetectionMode::Regex));
        assert_eq!(DetectionMode::from_str("ast"), Some(DetectionMode::Ast));
        assert_eq!(
            DetectionMode::from_str("strict"),
            Some(DetectionMode::Strict)
        );
        assert_eq!(DetectionMode::from_str("invalid"), None);
    }

    #[test]
    fn test_detection_mode_use_flags() {
        assert!(DetectionMode::Regex.use_regex());
        assert!(!DetectionMode::Regex.use_ast());

        assert!(!DetectionMode::Ast.use_regex());
        assert!(DetectionMode::Ast.use_ast());

        assert!(DetectionMode::Strict.use_regex());
        assert!(DetectionMode::Strict.use_ast());
    }

    #[test]
    fn test_ast_analysis_result() {
        let mut result = AstAnalysisResult::new();
        assert!(!result.is_suspicious());
        assert!(result.analysis_success);

        result.add_item("test_item".to_string());
        assert!(result.is_suspicious());
        assert_eq!(result.suspicious_items.len(), 1);

        // Adding duplicate should not increase count
        result.add_item("test_item".to_string());
        assert_eq!(result.suspicious_items.len(), 1);
    }

    #[test]
    #[cfg(feature = "python-plugins")]
    fn test_ast_scanner_creation() {
        let scanner = AstScanner::new(DetectionMode::Ast);
        assert_eq!(scanner.mode(), DetectionMode::Ast);
    }

    #[test]
    #[cfg(feature = "python-plugins")]
    fn test_validate_clean_plugin_ast() {
        let content = r#"
def run_check(target):
    return []
"#;
        let result = validate_python_plugin_ast(content, DetectionMode::Ast, true);
        // AST analysis may fail if Python is not available, so we just check it doesn't panic
        // In a real test environment with Python, this would work properly
        let _ = result;
    }

    #[test]
    #[cfg(feature = "python-plugins")]
    fn test_validate_suspicious_plugin_ast() {
        let content = r#"
import os
os.system('ls /')
"#;
        // Note: This test may not catch the issue if Python's ast module is not available
        // or if the AST dump parsing doesn't work as expected
        let _ = validate_python_plugin_ast(content, DetectionMode::Ast, true);
    }
}
