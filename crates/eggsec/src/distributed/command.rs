use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use std::time::Instant;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::Command;

const ALLOWED_COMMANDS: &[&str] = &["eggsec"];
const MAX_OUTPUT_SIZE: usize = 10 * 1024 * 1024; // 10MB
const MAX_ARGS: usize = 50;
const MAX_ARG_LENGTH: usize = 1000;

/// Upper bound for a single remote command. A peer that supplies no timeout
/// (`timeout: None` on the wire) gets this default instead of an unbounded
/// child, and an explicit value is clamped into `1..=MAX_COMMAND_TIMEOUT_SECS`.
const MAX_COMMAND_TIMEOUT_SECS: u64 = 300;

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
        #[serde(default)]
        env: Option<FxHashMap<String, String>>,
    },
    #[serde(rename = "register")]
    Register {
        id: String,
        hostname: String,
        capabilities: Vec<String>,
    },
    #[serde(rename = "heartbeat")]
    Heartbeat { id: String, status: String },
    #[serde(rename = "result")]
    Result {
        id: String,
        result: crate::distributed::TaskResult,
    },
    #[serde(rename = "request_tasks")]
    RequestTasks {
        id: String,
        worker_id: String,
        max_tasks: usize,
    },
    #[serde(rename = "assign_tasks")]
    AssignTasks {
        id: String,
        tasks: Vec<crate::distributed::queue::Task>,
    },
    #[serde(rename = "enqueue_task")]
    EnqueueTask {
        id: String,
        task: crate::distributed::queue::Task,
    },
    #[serde(rename = "status_request")]
    StatusRequest { id: String },
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
        env: Option<FxHashMap<String, String>>,
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

        // Security: The `env` field is intentionally rejected even though it's accepted
        // by the protocol. This is a deliberate security measure - custom environment
        // variables could be used to inject malicious values into command execution
        // (e.g., PATH manipulation, LD_PRELOAD, etc.). The field is kept in the protocol
        // definition for backward compatibility but is reserved for future use. When
        // environment variable support is eventually needed, it must be implemented with
        // strict allowlist validation (e.g., only known-safe variables like LANG, TZ).
        if env.is_some() {
            return Err("Custom environment variables are not allowed".to_string());
        }

        let args = &command[1..];

        let start = Instant::now();

        let mut cmd = Command::new(program);
        cmd.args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            // `kill_on_drop` is what makes the timeout below actually reap the
            // child: dropping the `output()` future only *detaches* a
            // kill_on_drop=false process, which leaks an orphan per timeout.
            .kill_on_drop(true);

        // A peer-supplied `timeout: None` must not remove the bound entirely.
        let timeout_secs = timeout_secs
            .unwrap_or(MAX_COMMAND_TIMEOUT_SECS)
            .clamp(1, MAX_COMMAND_TIMEOUT_SECS);

        match tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            Self::collect_output(&mut cmd),
        )
        .await
        {
            Ok(Ok(output)) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                let output_str = Self::format_output(&output.0, &output.1);
                Ok((output_str, duration_ms))
            }
            Ok(Err(e)) => {
                let _duration_ms = start.elapsed().as_millis() as u64;
                Err(format!("Command execution failed: {}", e))
            }
            Err(_) => {
                let _duration_ms = start.elapsed().as_millis() as u64;
                Err(format!("Command timed out after {} seconds", timeout_secs))
            }
        }
    }

    /// Spawn the child and drain stdout/stderr concurrently, stopping each
    /// stream at [`MAX_OUTPUT_SIZE`] so a chatty child cannot force an
    /// unbounded allocation before truncation is applied.
    ///
    /// A non-zero exit is *not* an error: this mirrors `Command::output()`,
    /// which also returns `Ok` for a failed exit, and the operator still wants
    /// the captured output when `eggsec` exits non-zero.
    async fn collect_output(cmd: &mut Command) -> std::io::Result<(Vec<u8>, Vec<u8>)> {
        let mut child = cmd.spawn()?;
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let (out, err) = tokio::join!(Self::read_capped(stdout), Self::read_capped(stderr));
        child.wait().await?;
        Ok((out, err))
    }

    async fn read_capped<R>(reader: Option<R>) -> Vec<u8>
    where
        R: AsyncRead + Unpin,
    {
        let Some(mut reader) = reader else {
            return Vec::new();
        };
        let mut buf = Vec::new();
        let mut chunk = vec![0u8; 64 * 1024];
        loop {
            match reader.read(&mut chunk).await {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let remaining = MAX_OUTPUT_SIZE.saturating_sub(buf.len());
                    if remaining == 0 {
                        break;
                    }
                    buf.extend_from_slice(&chunk[..n.min(remaining)]);
                }
            }
        }
        buf
    }

    fn format_output(stdout: &[u8], stderr: &[u8]) -> String {
        let mut result = String::new();

        if !stdout.is_empty() {
            result.push_str(&String::from_utf8_lossy(stdout));
        }

        if !stderr.is_empty() {
            if !result.is_empty() {
                result.push_str("\n--- stderr ---\n");
            }
            result.push_str(&String::from_utf8_lossy(stderr));
        }

        if result.is_empty() {
            result.push_str("(no output)");
        }

        // Limit output size to prevent memory issues
        if result.len() > MAX_OUTPUT_SIZE {
            // Floor to a char boundary first: `String::truncate` panics when the
            // cut lands mid-character, and `from_utf8_lossy` emits multi-byte
            // `U+FFFD` for invalid input.
            let mut cut = MAX_OUTPUT_SIZE;
            while !result.is_char_boundary(cut) {
                cut -= 1;
            }
            result.truncate(cut);
            result.push_str(&format!(
                "\n\n[Output truncated at {} bytes]",
                MAX_OUTPUT_SIZE
            ));
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
    pub fn new(
        hostname: String,
        success: bool,
        output: String,
        error: Option<String>,
        duration_ms: u64,
    ) -> Self {
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
    // OsRng (getrandom) rather than a fork-reproducible PRNG: PSKs are
    // long-lived key material and must not repeat across process forks.
    use rand::rngs::OsRng;
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
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

    #[test]
    fn format_output_truncates_on_char_boundary() {
        // MAX_OUTPUT_SIZE - 2 ASCII bytes then a 4-byte emoji: a naive
        // `truncate(MAX_OUTPUT_SIZE)` lands mid-character and panics.
        let mut stdout = vec![b'a'; MAX_OUTPUT_SIZE - 2];
        stdout.extend_from_slice("😀".as_bytes());
        stdout.extend_from_slice(&[b'b'; 64]);

        let out = CommandExecutor::format_output(&stdout, b"");
        assert!(out.len() > MAX_OUTPUT_SIZE, "truncation marker expected");
        assert!(out.starts_with(&"a".repeat(64)));
    }

    #[test]
    fn format_output_truncates_invalid_utf8_on_boundary() {
        // from_utf8_lossy expands each invalid byte into a 3-byte U+FFFD, so
        // the cut can land mid-character there too.
        let stdout = vec![0xffu8; MAX_OUTPUT_SIZE + 128];
        let out = CommandExecutor::format_output(&stdout, b"");
        assert!(out.contains("[Output truncated at"));
    }

    #[test]
    fn format_output_keeps_short_output_intact() {
        let out = CommandExecutor::format_output(b"stdout bytes", b"stderr bytes");
        assert_eq!(out, "stdout bytes\n--- stderr ---\nstderr bytes");
    }

    #[test]
    fn format_output_empty_reports_placeholder() {
        let out = CommandExecutor::format_output(b"", b"");
        assert_eq!(out, "(no output)");
    }
}
