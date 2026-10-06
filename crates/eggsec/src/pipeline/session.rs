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
    /// True when this checkpoint is the run's final save rather than a mid-run
    /// one.
    ///
    /// Without it a clean finish and a run that died on its last stage are
    /// indistinguishable: both end with `remaining_stages` empty. `Default` so
    /// checkpoints written before this field existed still load, and such a
    /// file is read as mid-run — the safe direction, since it is offered as
    /// resumable rather than declared complete.
    #[serde(default)]
    pub finalized: bool,
    /// Stages attempted and failed. Recorded only when `finalized`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub failed_stages: Vec<Stage>,
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

/// Create the store directory if needed, returning it.
///
/// `save()` opens the file but does not create parents, so a caller choosing a
/// store location must materialise the directory first.
pub fn ensure_store_dir() -> Result<std::path::PathBuf> {
    let dir = default_session_dir();
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// A discoverable checkpoint path inside the store for `target`.
///
/// The filename carries a sanitized target plus a UTC timestamp so repeated
/// scans of the same host do not overwrite each other and so a row is
/// identifiable without opening it. Path separators and control characters are
/// stripped: a target is operator-supplied, and an unsanitized one would write
/// outside the store.
pub fn store_path_for(target: &str, epoch_secs: u64) -> std::path::PathBuf {
    let sanitized: String = crate::utils::sanitize_for_logging(target)
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .take(48)
        .collect();
    // Trim leading/trailing separators *and* a leading dot: a checkpoint must
    // be a visible file in a plain directory listing, or an operator auditing
    // the store would not know it exists. `.` itself is dropped for the same
    // reason.
    let sanitized = sanitized
        .trim_matches(|c: char| c == '-' || c == '.')
        .to_string();
    let sanitized = if sanitized.is_empty() {
        "scan".to_string()
    } else {
        sanitized
    };
    default_session_dir().join(format!("{sanitized}-{epoch_secs}.session.json"))
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
    /// True when the run finished and this is its last checkpoint.
    pub finalized: bool,
    /// Stages attempted and failed, recorded only when `finalized`.
    pub failed_stages: usize,
}

impl SessionEntry {
    /// Whether resuming this checkpoint would do any work.
    ///
    /// A checkpoint exists so an *interrupted* scan can be continued. Once
    /// every stage has completed there is nothing left to resume, and offering
    /// such a row as resumable would start a no-op run the operator reads as a
    /// real result.
    pub fn is_resumable(&self) -> bool {
        // Resuming re-runs `remaining_stages`; when none are left the resume is
        // a no-op, so there is nothing to offer regardless of how the run ended.
        self.remaining_stages > 0
    }

    /// One-line summary for a list row.
    pub fn label(&self) -> String {
        if self.is_resumable() {
            format!(
                "{}  ({}/{} stages, {} left)",
                self.target,
                self.completed_stages,
                self.completed_stages + self.remaining_stages,
                self.remaining_stages
            )
        } else if !self.finalized {
            // No stages left *and* no final save: the run stopped before it
            // could record an outcome (a hard error aborts `run()` before the
            // final checkpoint). Nothing is queued to resume, so this is not
            // resumable — but it is also not a finished assessment, and calling
            // it "complete" would read as a clean result for a broken scan.
            format!(
                "{}  (stopped early after {} stage(s) — re-run to continue)",
                self.target, self.completed_stages
            )
        } else if self.finalized && self.failed_stages > 0 {
            // Not "complete": the run ended, but stages failed and resume
            // cannot retry them, so the operator has to re-run the scan. Saying
            // "complete" here would read as a clean assessment.
            format!(
                "{}  ({} stages, {} failed — re-run to retry)",
                self.target, self.completed_stages, self.failed_stages
            )
        } else {
            format!(
                "{}  (complete, {} stages)",
                self.target, self.completed_stages
            )
        }
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
                    finalized: session.finalized,
                    failed_stages: session.failed_stages.len(),
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
            finalized: false,
            failed_stages: Vec::new(),
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
    fn store_path_is_sanitized_and_unique_per_call() {
        // A target with a path separator must not escape the store: an
        // operator-supplied target is untrusted input for a filename. The
        // property that matters is that the result stays one component inside
        // the store — `..` surviving as literal text is harmless precisely
        // because the separators around it are gone.
        let escaped = store_path_for("../../etc/passwd", 1_700_000_000);
        let name = escaped.file_name().unwrap().to_string_lossy().to_string();
        assert!(!name.contains('/') && !name.contains('\\'), "{name}");
        assert!(!name.starts_with('.'), "must not be a hidden file: {name}");
        // Repeated scans of the same host must not overwrite each other.
        let a = store_path_for("example.com", 1_700_000_000);
        let b = store_path_for("example.com", 1_700_000_001);
        assert_ne!(a, b);
        assert!(a.to_string_lossy().ends_with(".session.json"));

        // A target that sanitizes to nothing must still produce a usable name.
        let empty = store_path_for("///", 1_700_000_000);
        assert!(empty.file_name().unwrap().to_string_lossy().len() > 5);
    }

    #[test]
    fn completed_checkpoint_is_not_resumable() {
        let dir = dir_for("complete");
        let session = PipelineSession {
            target: "example.test".into(),
            profile: ScanProfile::Quick,
            completed_stages: vec![Stage::PortScan, Stage::Fingerprint],
            remaining_stages: vec![],
            context: PipelineContext::new("https://example.test"),
            spoof_config: crate::scanner::spoof::SpoofConfig::default(),
            concurrency: None,
            concurrent_stages: None,
            config: None,
            finalized: true,
            failed_stages: Vec::new(),
        };
        std::fs::write(
            dir.join("done.json"),
            serde_json::to_string_pretty(&session).unwrap(),
        )
        .unwrap();

        let listed = list_sessions(&dir);
        assert_eq!(listed.len(), 1);
        assert!(
            !listed[0].is_resumable(),
            "a finished scan has nothing to resume"
        );
        assert!(
            listed[0].label().contains("complete"),
            "label must say it is done, got {:?}",
            listed[0].label()
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn interrupted_run_is_not_labelled_complete() {
        // A hard error aborts `run()` before the final checkpoint, so the file
        // on disk has no outcome recorded. "complete" would read as a clean
        // assessment for a scan that died.
        let entry = SessionEntry {
            path: std::path::PathBuf::from("/tmp/x.session.json"),
            target: "example.test".into(),
            completed_stages: 1,
            remaining_stages: 0,
            modified_epoch_secs: 0,
            finalized: false,
            failed_stages: 0,
        };
        let label = entry.label();
        assert!(!label.contains("complete"), "got {label:?}");
        assert!(label.contains("stopped early"), "got {label:?}");
        assert!(!entry.is_resumable());
    }

    /// Both env-var assertions live in one test on purpose: `EGGSEC_SESSION_DIR`
    /// is process-global, so two tests touching it in parallel would race and
    /// each would observe the other's directory.
    #[test]
    fn env_override_drives_dir_and_store_path() {
        let dir = dir_for("env");
        let previous = std::env::var_os("EGGSEC_SESSION_DIR");
        // SAFETY: single-threaded test body; no other test reads this variable.
        unsafe { std::env::set_var("EGGSEC_SESSION_DIR", &dir) };

        assert_eq!(default_session_dir(), dir);
        let path = store_path_for("example.com", 1_700_000_000);
        assert_eq!(path.parent().unwrap(), dir.as_path());

        match previous {
            Some(v) => unsafe { std::env::set_var("EGGSEC_SESSION_DIR", v) },
            None => unsafe { std::env::remove_var("EGGSEC_SESSION_DIR") },
        }
        std::fs::remove_dir_all(&dir).ok();
    }
}
