use crate::config::EggsecConfig;
use crate::error::Result;
use crate::types::ScanProfile;
use serde::{Deserialize, Serialize};

use super::context::PipelineContext;
use super::stage::Stage;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineSession {
    pub target: String,
    pub profile: ScanProfile,
    pub completed_stages: Vec<Stage>,
    pub remaining_stages: Vec<Stage>,
    pub context: PipelineContext,
    pub spoof_config: crate::scanner::spoof::SpoofConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub concurrency: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub concurrent_stages: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<EggsecConfig>,
}

pub async fn save(path: &str, session: &PipelineSession) -> Result<()> {
    use tokio::io::AsyncWriteExt;
    let json = serde_json::to_string_pretty(session)?;
    let mut options = tokio::fs::OpenOptions::new();
    options.create(true).truncate(true).write(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options.open(path).await?;
    file.write_all(json.as_bytes()).await?;
    Ok(())
}

pub async fn load(path: &str) -> Result<PipelineSession> {
    let json = tokio::fs::read_to_string(path).await?;
    let session: PipelineSession = serde_json::from_str(&json)?;
    Ok(session)
}

/// Directory that holds discoverable scan checkpoints.
///
/// Deliberately distinct from the TUI's own `sessions/` directory, which stores
/// `SessionState` (bookmarks, theme, last tab). The two schemas are unrelated,
/// so pointing both at one directory would make every checkpoint fail to
/// deserialize as UI state and vice versa.
///
/// Resolution order: `EGGSEC_SESSION_DIR`, then the platform data directory.
pub fn default_session_dir() -> std::path::PathBuf {
    if let Some(dir) = std::env::var_os("EGGSEC_SESSION_DIR") {
        if !dir.is_empty() {
            return std::path::PathBuf::from(dir);
        }
    }
    directories::ProjectDirs::from("com", "eggsec", "eggsec")
        .map(|dirs| dirs.data_dir().join("scan-sessions"))
        .unwrap_or_else(|| {
            let home = std::env::var_os("HOME").map(std::path::PathBuf::from);
            if let Some(home) = home {
                home.join(".eggsec").join("scan-sessions")
            } else {
                std::path::PathBuf::from("/tmp/eggsec-scan-sessions")
            }
        })
}

/// A checkpoint reduced to what a picker needs to present it.
///
/// This is a projection of `PipelineSession`, not a second stored format: the
/// full session is still on disk and is what resume loads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionEntry {
    pub path: std::path::PathBuf,
    pub target: String,
    pub completed_stages: usize,
    pub remaining_stages: usize,
    /// Modification time as seconds since the Unix epoch, for age display.
    pub modified_epoch_secs: u64,
}

impl SessionEntry {
    /// One-line summary for a list row.
    pub fn label(&self) -> String {
        format!(
            "{}  ({}/{} stages)",
            self.target,
            self.completed_stages,
            self.completed_stages + self.remaining_stages
        )
    }
}

/// List every readable checkpoint in `dir`, newest first.
///
/// Synchronous by design: the TUI calls this from its tab-entry path, which
/// runs on the event loop, and blocking a runtime worker thread there is a
/// hazard. A checkpoint directory holds a handful of small JSON files, so the
/// read is not worth a thread hop.
///
/// A checkpoint that cannot be read or parsed is skipped with a
/// `tracing::warn!` rather than aborting the listing: one corrupt file must not
/// hide every other resumable session, and a silently-empty list would read as
/// "nothing to resume" instead of "something is wrong".
pub fn list_sessions(dir: &std::path::Path) -> Vec<SessionEntry> {
    let mut entries = Vec::new();

    let dir_entries = match std::fs::read_dir(dir) {
        Ok(read) => read,
        Err(e) => {
            // A missing directory is the normal "no sessions yet" case.
            if e.kind() != std::io::ErrorKind::NotFound {
                tracing::warn!(
                    path = %dir.display(),
                    error = %e,
                    "could not read the checkpoint directory; the resume list is empty"
                );
            }
            return entries;
        }
    };

    for entry in dir_entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                tracing::warn!(
                    path = %dir.display(),
                    error = %e,
                    "skipping unreadable directory entry while listing checkpoints"
                );
                continue;
            }
        };
        let path = entry.path();
        if path.extension().is_none_or(|ext| ext != "json") {
            continue;
        }
        match load_blocking(path.to_string_lossy().as_ref()) {
            Ok(session) => {
                let modified_epoch_secs = entry
                    .metadata()
                    .ok()
                    .and_then(|meta| meta.modified().ok())
                    .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                entries.push(SessionEntry {
                    path,
                    target: session.target,
                    completed_stages: session.completed_stages.len(),
                    remaining_stages: session.remaining_stages.len(),
                    modified_epoch_secs,
                });
            }
            Err(e) => {
                tracing::warn!(
                    path = %path.display(),
                    error = %e,
                    "skipping unreadable checkpoint; other sessions are still listed"
                );
            }
        }
    }

    // Newest first.
    entries.sort_by_key(|e| std::cmp::Reverse(e.modified_epoch_secs));
    entries
}

/// `load` without a runtime, for callers that are not on one.
fn load_blocking(path: &str) -> Result<PipelineSession> {
    let json = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&json)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir_for(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("eggsec-session-test-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    fn write_session(path: &std::path::Path, target: &str) {
        let session = PipelineSession {
            target: target.to_string(),
            profile: ScanProfile::Quick,
            completed_stages: vec![Stage::PortScan],
            remaining_stages: vec![Stage::Fingerprint, Stage::EndpointScan],
            context: PipelineContext::new("https://example.test"),
            spoof_config: crate::scanner::spoof::SpoofConfig::default(),
            concurrency: None,
            concurrent_stages: None,
            config: None,
        };
        std::fs::write(path, serde_json::to_string_pretty(&session).unwrap()).unwrap();
    }

    #[test]
    fn lists_checkpoints_with_stage_counts() {
        let dir = dir_for("list");
        write_session(&dir.join("a.json"), "example.test");
        std::fs::write(dir.join("b.json"), "{ not json").unwrap();

        let listed = list_sessions(&dir);
        assert_eq!(listed.len(), 1, "a corrupt file must not hide the rest");
        assert_eq!(listed[0].target, "example.test");
        assert_eq!(listed[0].completed_stages, 1);
        assert_eq!(listed[0].remaining_stages, 2);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn missing_directory_lists_empty_without_error() {
        let missing = std::env::temp_dir().join("eggsec-session-test-absent-dir-xyz");
        assert!(list_sessions(&missing).is_empty());
    }

    #[test]
    fn non_json_files_are_ignored() {
        let dir = dir_for("nonjson");
        write_session(&dir.join("real.json"), "example.test");
        std::fs::write(dir.join("notes.txt"), "ignored").unwrap();
        std::fs::write(dir.join("real.json.tmp"), "partial").unwrap();

        let listed = list_sessions(&dir);
        assert_eq!(listed.len(), 1);
        assert!(listed[0].label().contains("example.test"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn env_override_wins() {
        // Guard the precedence order without mutating global state for other
        // tests: assert the helper reads the variable when it is set.
        let dir = dir_for("env");
        let previous = std::env::var_os("EGGSEC_SESSION_DIR");
        // SAFETY: single-threaded test body; no other test reads this variable.
        unsafe { std::env::set_var("EGGSEC_SESSION_DIR", &dir) };
        assert_eq!(default_session_dir(), dir);
        match previous {
            Some(v) => unsafe { std::env::set_var("EGGSEC_SESSION_DIR", v) },
            None => unsafe { std::env::remove_var("EGGSEC_SESSION_DIR") },
        }
        std::fs::remove_dir_all(&dir).ok();
    }
}
