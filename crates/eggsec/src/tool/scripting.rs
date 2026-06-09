//! Runtime scripting engine for dynamic tool and payload generation.
//!
//! Provides script execution capabilities using Python and Ruby runtimes
//! with sandbox restrictions for security.

use rustc_hash::FxHashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

pub trait ScriptEngine: Send + Sync + std::fmt::Debug {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn execute(&self, script: &str, context: &ScriptContext) -> Result<ScriptResult, ScriptError>;
    fn validate(&self, script: &str) -> Result<(), ScriptError>;
    fn set_timeout(&mut self, seconds: u64);
    fn set_sandbox(&mut self, enabled: bool);
}

#[derive(Debug, Clone)]
pub struct ScriptContext {
    pub target: Option<String>,
    pub parameters: FxHashMap<String, serde_json::Value>,
    pub environment: FxHashMap<String, String>,
    pub working_directory: Option<PathBuf>,
}

impl Default for ScriptContext {
    fn default() -> Self {
        Self {
            target: None,
            parameters: FxHashMap::default(),
            environment: FxHashMap::default(),
            working_directory: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScriptResult {
    pub output: String,
    pub errors: Vec<String>,
    pub findings: Vec<serde_json::Value>,
    pub execution_time_ms: u64,
}

#[derive(Debug, Clone)]
pub struct ScriptError {
    pub code: String,
    pub message: String,
    pub line: Option<usize>,
}

impl std::fmt::Display for ScriptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)?;
        if let Some(line) = self.line {
            write!(f, " at line {}", line)?;
        }
        Ok(())
    }
}

impl std::error::Error for ScriptError {}

#[derive(Clone)]
pub struct ScriptEngineRegistry {
    engines: Arc<RwLock<FxHashMap<String, Arc<dyn ScriptEngine>>>>,
    execution_count: Arc<RwLock<u64>>,
    total_execution_time_ms: Arc<RwLock<u64>>,
}

impl ScriptEngineRegistry {
    pub fn new() -> Self {
        Self {
            engines: Arc::new(RwLock::new(FxHashMap::default())),
            execution_count: Arc::new(RwLock::new(0)),
            total_execution_time_ms: Arc::new(RwLock::new(0)),
        }
    }

    pub async fn register(&self, engine: Arc<dyn ScriptEngine>) {
        let mut engines = self.engines.write().await;
        engines.insert(engine.id().to_string(), engine);
    }

    pub async fn get(&self, id: &str) -> Option<Arc<dyn ScriptEngine>> {
        let engines = self.engines.read().await;
        engines.get(id).cloned()
    }

    pub async fn list_engines(&self) -> Vec<String> {
        let engines = self.engines.read().await;
        engines.keys().cloned().collect()
    }

    pub async fn execute(
        &self,
        engine_id: &str,
        script: &str,
        context: &ScriptContext,
    ) -> Result<ScriptResult, ScriptError> {
        let engines = self.engines.read().await;
        let engine = engines.get(engine_id).ok_or_else(|| ScriptError {
            code: "ENGINE_NOT_FOUND".to_string(),
            message: format!("Script engine '{}' not found", engine_id),
            line: None,
        })?;

        let start = std::time::Instant::now();
        let result = engine.execute(script, context);

        if result.is_ok() {
            let mut count = self.execution_count.write().await;
            *count += 1;
            let mut total_time = self.total_execution_time_ms.write().await;
            *total_time += start.elapsed().as_millis() as u64;
        }

        result
    }

    pub async fn stats(&self) -> ScriptEngineStats {
        let count = *self.execution_count.read().await;
        let total_time = *self.total_execution_time_ms.read().await;
        let avg_time = if count > 0 { total_time / count } else { 0 };

        ScriptEngineStats {
            total_executions: count,
            total_execution_time_ms: total_time,
            average_execution_time_ms: avg_time,
        }
    }
}

impl Default for ScriptEngineRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct ScriptEngineStats {
    pub total_executions: u64,
    pub total_execution_time_ms: u64,
    pub average_execution_time_ms: u64,
}

pub struct ScriptSandbox {
    allowed_modules: Vec<String>,
    blocked_modules: Vec<String>,
    allowed_paths: Vec<PathBuf>,
    max_memory_mb: Option<usize>,
    max_execution_seconds: Option<u64>,
}

impl ScriptSandbox {
    pub fn new() -> Self {
        Self {
            allowed_modules: vec![
                "json".to_string(),
                "re".to_string(),
                "collections".to_string(),
            ],
            blocked_modules: vec![
                "os".to_string(),
                "subprocess".to_string(),
                "socket".to_string(),
                "urllib".to_string(),
                "httplib".to_string(),
            ],
            allowed_paths: vec![],
            max_memory_mb: Some(512),
            max_execution_seconds: Some(300),
        }
    }

    pub fn allow_module(&mut self, module: &str) -> &mut Self {
        self.allowed_modules.push(module.to_string());
        self
    }

    pub fn block_module(&mut self, module: &str) -> &mut Self {
        self.blocked_modules.push(module.to_string());
        self
    }

    pub fn allow_path(&mut self, path: PathBuf) -> &mut Self {
        self.allowed_paths.push(path);
        self
    }

    pub fn set_max_memory_mb(&mut self, mb: usize) -> &mut Self {
        self.max_memory_mb = Some(mb);
        self
    }

    pub fn set_max_execution_seconds(&mut self, seconds: u64) -> &mut Self {
        self.max_execution_seconds = Some(seconds);
        self
    }

    pub fn is_allowed_module(&self, module: &str) -> bool {
        if self.blocked_modules.contains(&module.to_string()) {
            return false;
        }
        if self.allowed_modules.is_empty() {
            return true;
        }
        self.allowed_modules.contains(&module.to_string())
    }

    pub fn is_allowed_path(&self, path: &PathBuf) -> bool {
        if self.allowed_paths.is_empty() {
            return true;
        }
        self.allowed_paths
            .iter()
            .any(|allowed| path.starts_with(allowed))
    }
}

impl Default for ScriptSandbox {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_script_context_default() {
        let ctx = ScriptContext::default();
        assert!(ctx.target.is_none());
        assert!(ctx.parameters.is_empty());
    }

    #[test]
    fn test_sandbox_blocked_module() {
        let sandbox = ScriptSandbox::new();
        assert!(!sandbox.is_allowed_module("os"));
        assert!(!sandbox.is_allowed_module("subprocess"));
    }

    #[test]
    fn test_sandbox_allowed_module() {
        let mut sandbox = ScriptSandbox::new();
        sandbox.allow_module("custom_module");
        assert!(sandbox.is_allowed_module("custom_module"));
    }

    #[tokio::test]
    async fn test_registry_list_engines_empty() {
        let registry = ScriptEngineRegistry::new();
        let engines = registry.list_engines().await;
        assert!(engines.is_empty());
    }

    #[tokio::test]
    async fn test_registry_stats_initial() {
        let registry = ScriptEngineRegistry::new();
        let stats = registry.stats().await;
        assert_eq!(stats.total_executions, 0);
        assert_eq!(stats.total_execution_time_ms, 0);
    }

    #[test]
    fn test_sandbox_custom() {
        let mut sandbox = ScriptSandbox::new();
        sandbox
            .allow_module("json")
            .block_module("os")
            .set_max_memory_mb(1024)
            .set_max_execution_seconds(600);

        assert!(sandbox.is_allowed_module("json"));
        assert!(!sandbox.is_allowed_module("os"));
        assert_eq!(sandbox.max_memory_mb, Some(1024));
        assert_eq!(sandbox.max_execution_seconds, Some(600));
    }
}
