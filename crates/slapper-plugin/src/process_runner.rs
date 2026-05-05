use anyhow::Result;
use std::path::Path;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

pub struct ProcessPluginRunner {
    timeout: Duration,
    isolation_level: IsolationLevel,
}

#[derive(Debug, Clone, Default)]
pub enum IsolationLevel {
    #[default]
    InProcess,
    Process,
    Sandboxed,
}

#[derive(Debug, Clone)]
pub struct PluginResult {
    pub success: bool,
    pub output: String,
    pub exit_code: Option<i32>,
}

impl ProcessPluginRunner {
    pub fn new(timeout: Duration, isolation_level: IsolationLevel) -> Self {
        Self {
            timeout,
            isolation_level,
        }
    }

    pub async fn run_plugin(&self, path: &Path, target: &str) -> Result<PluginResult> {
        match self.isolation_level {
            IsolationLevel::InProcess => {
                anyhow::bail!("InProcess isolation not supported for ProcessPluginRunner")
            }
            IsolationLevel::Process => self.run_in_process(path, target).await,
            IsolationLevel::Sandboxed => self.run_sandboxed(path, target).await,
        }
    }

    async fn run_in_process(&self, path: &Path, target: &str) -> Result<PluginResult> {
        let mut cmd = Command::new("python3");
        cmd.arg(path).arg(target).kill_on_drop(true);

        let result = timeout(self.timeout, cmd.output()).await;

        match result {
            Ok(Ok(output)) => {
                let exit_code = output.status.code();
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();

                Ok(PluginResult {
                    success: output.status.success(),
                    output: if stderr.is_empty() {
                        stdout
                    } else {
                        format!("{}\n{}", stdout, stderr)
                    },
                    exit_code,
                })
            }
            Ok(Err(e)) => anyhow::bail!("Process error: {}", e),
            Err(_) => anyhow::bail!("Plugin timed out after {:?}", self.timeout),
        }
    }

    async fn run_sandboxed(&self, path: &Path, target: &str) -> Result<PluginResult> {
        let mut cmd = Command::new("python3");
        cmd.arg("-u")
            .arg(path)
            .arg(target)
            .kill_on_drop(true)
            .env("PYTHONPATH", "")
            .env("HOME", "/tmp");

        let result = timeout(self.timeout, cmd.output()).await;

        match result {
            Ok(Ok(output)) => {
                let exit_code = output.status.code();
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();

                Ok(PluginResult {
                    success: output.status.success(),
                    output: if stderr.is_empty() {
                        stdout
                    } else {
                        format!("{}\n{}", stdout, stderr)
                    },
                    exit_code,
                })
            }
            Ok(Err(e)) => anyhow::bail!("Process error: {}", e),
            Err(_) => anyhow::bail!("Plugin timed out after {:?}", self.timeout),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_plugin_runner_creation() {
        let runner = ProcessPluginRunner::new(Duration::from_secs(30), IsolationLevel::Process);
        assert_eq!(runner.timeout, Duration::from_secs(30));
    }

    #[test]
    fn test_isolation_level_variants() {
        let default_isolation = IsolationLevel::default();
        assert!(matches!(default_isolation, IsolationLevel::InProcess));
    }
}
