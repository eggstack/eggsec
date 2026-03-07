use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Stdio;
use std::time::Instant;
use tokio::process::Command;

const ALLOWED_COMMANDS: &[&str] = &["slapper"];
const MAX_OUTPUT_SIZE: usize = 10 * 1024 * 1024; // 10MB
const MAX_ARGS: usize = 50;
const MAX_ARG_LENGTH: usize = 1000;

const FORBIDDEN_PATTERNS: &[&str] = &[
    "../",
    "..\\",
    "/etc/",
    "/root/",
    "/proc/",
    "/sys/",
    "~/.ssh/",
    "~/.aws/",
    ".pem",
    ".key",
    "--config",
    "--config-file",
    "--credentials",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CommandMessage {
    #[serde(rename = "execute")]
    Execute {
        id: String,
        command: Vec<String>,
        timeout: Option<u64>,
        env: Option<HashMap<String, String>>,
    },
    #[serde(rename = "register")]
    Register {
        id: String,
        hostname: String,
        capabilities: Vec<String>,
    },
    #[serde(rename = "heartbeat")]
    Heartbeat { id: String, status: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMessage {
    pub id: String,
    #[serde(rename = "type")]
    pub msg_type: String,
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
    #[serde(rename = "duration_ms")]
    pub duration_ms: Option<u64>,
    pub hostname: Option<String>,
    pub capabilities: Option<Vec<String>>,
}

impl ResponseMessage {
    pub fn success(id: String, output: String, duration_ms: u64) -> Self {
        Self {
            id,
            msg_type: "response".to_string(),
            success: true,
            output: Some(output),
            error: None,
            duration_ms: Some(duration_ms),
            hostname: None,
            capabilities: None,
        }
    }

    pub fn error(id: String, error: String, duration_ms: Option<u64>) -> Self {
        Self {
            id,
            msg_type: "response".to_string(),
            success: false,
            output: None,
            error: Some(error),
            duration_ms,
            hostname: None,
            capabilities: None,
        }
    }

    pub fn registration(id: String, hostname: String, capabilities: Vec<String>) -> Self {
        Self {
            id,
            msg_type: "registered".to_string(),
            success: true,
            output: None,
            error: None,
            duration_ms: None,
            hostname: Some(hostname),
            capabilities: Some(capabilities),
        }
    }
}

pub struct CommandExecutor;

impl CommandExecutor {
    pub async fn execute(
        command: Vec<String>,
        timeout_secs: Option<u64>,
        env: Option<HashMap<String, String>>,
    ) -> Result<(String, u64), String> {
        if command.is_empty() {
            return Err("No command provided".to_string());
        }

        // Validate argument count
        if command.len() > MAX_ARGS + 1 {
            return Err(format!("Too many arguments (max {})", MAX_ARGS));
        }

        let program = &command[0];
        
        // Security: Only allow specific executables
        if !ALLOWED_COMMANDS.iter().any(|&cmd| cmd == program) {
            return Err(format!(
                "Command '{}' not allowed. Only {} commands are permitted.",
                program,
                ALLOWED_COMMANDS.join(", ")
            ));
        }

        // Validate arguments
        for arg in &command[1..] {
            if arg.len() > MAX_ARG_LENGTH {
                return Err(format!("Argument too long (max {} chars)", MAX_ARG_LENGTH));
            }
            
            let arg_lower = arg.to_lowercase();
            for pattern in FORBIDDEN_PATTERNS {
                if arg_lower.contains(&pattern.to_lowercase()) {
                    return Err(format!("Argument contains forbidden pattern: {}", pattern));
                }
            }
        }

        // Security: Do not allow custom environment variables
        if env.is_some() {
            return Err("Custom environment variables are not allowed".to_string());
        }

        let args = &command[1..];

        let start = Instant::now();

        let mut cmd = Command::new(program);
        cmd.args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(env_vars) = env {
            for (key, value) in env_vars {
                cmd.env(&key, &value);
            }
        }

        if let Some(timeout) = timeout_secs {
            let result = tokio::time::timeout(
                std::time::Duration::from_secs(timeout),
                cmd.output(),
            )
            .await;

            match result {
                Ok(Ok(output)) => {
                    let duration_ms = start.elapsed().as_millis() as u64;
                    let output_str = Self::format_output(&output);
                    Ok((output_str, duration_ms))
                }
                Ok(Err(e)) => {
                    let _duration_ms = start.elapsed().as_millis() as u64;
                    Err(format!("Command execution failed: {}", e))
                }
                Err(_) => {
                    let _duration_ms = start.elapsed().as_millis() as u64;
                    Err(format!("Command timed out after {} seconds", timeout))
                }
            }
        } else {
            match cmd.output().await {
                Ok(output) => {
                    let duration_ms = start.elapsed().as_millis() as u64;
                    let output_str = Self::format_output(&output);
                    Ok((output_str, duration_ms))
                }
                Err(e) => {
                    let _duration_ms = start.elapsed().as_millis() as u64;
                    Err(format!("Command execution failed: {}", e))
                }
            }
        }
    }

    fn format_output(output: &std::process::Output) -> String {
        let mut result = String::new();

        if !output.stdout.is_empty() {
            result.push_str(&String::from_utf8_lossy(&output.stdout));
        }

        if !output.stderr.is_empty() {
            if !result.is_empty() {
                result.push_str("\n--- stderr ---\n");
            }
            result.push_str(&String::from_utf8_lossy(&output.stderr));
        }

        if result.is_empty() {
            result.push_str("(no output)");
        }

        // Limit output size to prevent memory issues
        if result.len() > MAX_OUTPUT_SIZE {
            result.truncate(MAX_OUTPUT_SIZE);
            result.push_str(&format!("\n\n[Output truncated at {} bytes]", MAX_OUTPUT_SIZE));
        }

        result
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteResult {
    pub hostname: String,
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub duration_ms: u64,
}

impl RemoteResult {
    pub fn new(hostname: String, success: bool, output: String, error: Option<String>, duration_ms: u64) -> Self {
        Self {
            hostname,
            success,
            output,
            error,
            duration_ms,
        }
    }
}

pub fn generate_psk() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
    hex::encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_psk_length() {
        let psk = generate_psk();
        assert_eq!(psk.len(), 64);
    }

    #[test]
    fn test_generate_psk_unique() {
        let psk1 = generate_psk();
        let psk2 = generate_psk();
        assert_ne!(psk1, psk2);
    }
}
