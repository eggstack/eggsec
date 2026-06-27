use crate::tabs::Tab;
use crate::App;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

/// Session state persistence.
///
/// # Stability Guarantees
///
/// - `current_tab_id` (stable ID string) is the **authoritative** tab identity for new sessions.
/// - `legacy_current_tab` stores the **visible index** for backward compatibility. When restoring
///   from legacy data, `Tab::from_index()` interprets the value as a visible index into `Tab::all()`.
/// - `bookmarks` stores stable IDs and is the **authoritative** bookmark identity for new sessions.
/// - `legacy_bookmarks` stores visible indexes for backward compatibility.
///
/// # Migration Path
///
/// Old numeric session files may contain enum discriminants written as `tab as usize`.
/// When restoring, we interpret `legacy_current_tab` as a visible index (not discriminant)
/// because `Tab::from_index()` maps directly to `Tab::all()[index]`. If tabs are reordered in
/// future versions, the stable ID (`current_tab_id`) remains correct regardless of tab ordering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub current_tab_id: Option<String>,
    #[serde(default)]
    pub bookmarks: Vec<String>,
    pub theme_name: String,
    #[serde(default)]
    pub legacy_current_tab: Option<usize>,
    #[serde(default)]
    pub legacy_bookmarks: Vec<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    pub auto_save_interval_secs: u64,
    pub session_dir: PathBuf,
    pub max_sessions: usize,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            auto_save_interval_secs: 30,
            session_dir: Self::default_session_dir(),
            max_sessions: 10,
        }
    }
}

impl SessionConfig {
    fn default_session_dir() -> PathBuf {
        directories::ProjectDirs::from("com", "eggsec", "eggsec")
            .map(|dirs| dirs.data_dir().join("sessions"))
            .unwrap_or_else(|| {
                // Expand ~ to a real home directory so this path works as a
                // fallback, even on platforms where `~` is treated literally.
                let home = std::env::var_os("HOME").map(PathBuf::from);
                if let Some(home) = home {
                    home.join(".eggsec").join("sessions")
                } else {
                    PathBuf::from("/tmp/eggsec-sessions")
                }
            })
    }

    pub fn with_auto_save_interval(mut self, interval_secs: u64) -> Self {
        self.auto_save_interval_secs = interval_secs;
        self
    }
}

#[derive(Default)]
pub struct SessionManager {
    pub config: SessionConfig,
}

impl SessionManager {
    pub fn new(config: SessionConfig) -> Self {
        Self { config }
    }

    pub fn save_session(&self, app: &App) -> anyhow::Result<PathBuf> {
        let state = self.capture_state(app);
        let filename = format!(
            "session_{}.json",
            chrono::Local::now().format("%Y%m%d_%H%M%S")
        );
        let path = self.config.session_dir.join(&filename);

        fs::create_dir_all(&self.config.session_dir)?;
        let json = serde_json::to_string_pretty(&state)?;
        let tmp_path = path.with_extension("json.tmp");
        fs::write(&tmp_path, &json)?;
        fs::rename(&tmp_path, &path)?;

        self.cleanup_orphaned_tmp_files();
        self.cleanup_old_sessions()?;

        Ok(path)
    }

    pub fn save_quick(&self, app: &App) -> anyhow::Result<PathBuf> {
        let path = self.config.session_dir.join("quick_save.json");
        fs::create_dir_all(&self.config.session_dir)?;
        let state = self.capture_state(app);
        let json = serde_json::to_string_pretty(&state)?;
        // .json.tmp is an unusual double-extension but is the conventional
        // atomic-write temp pattern (write to `.tmp`, then rename over dest).
        let tmp_path = path.with_extension("json.tmp");
        fs::write(&tmp_path, &json)?;
        fs::rename(&tmp_path, &path)?;
        self.cleanup_orphaned_tmp_files();
        Ok(path)
    }

    pub fn load_quick(&self) -> anyhow::Result<Option<SessionState>> {
        let path = self.config.session_dir.join("quick_save.json");
        if !path.exists() {
            return Ok(None);
        }
        match fs::read_to_string(&path)
            .and_then(|s| serde_json::from_str::<SessionState>(&s).map_err(|e| e.into()))
        {
            Ok(state) => Ok(Some(state)),
            Err(e) => {
                tracing::warn!(
                    path = %path.display(),
                    error = %e,
                    "quick_save.json is corrupt; quarantining"
                );
                let quarantine = path.with_extension("json.bad");
                if let Err(e) = fs::rename(&path, &quarantine) {
                    tracing::warn!(
                        path = %path.display(),
                        error = %e,
                        "failed to quarantine corrupt quick_save.json"
                    );
                }
                Ok(None)
            }
        }
    }

    pub fn load_latest_session(&self) -> anyhow::Result<Option<SessionState>> {
        let entries = fs::read_dir(&self.config.session_dir)?;
        let mut sessions: Vec<_> = entries
            .filter_map(|e| match e {
                Ok(entry) => Some(entry),
                Err(e) => {
                    tracing::warn!("Skipping unreadable directory entry: {:?}", e);
                    None
                }
            })
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
            // Exclude quick_save.json from the snapshot rotation sort; it is
            // an alias for the most-recent state, not a candidate snapshot.
            .filter(|e| {
                e.path()
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy() != "quick_save.json")
            })
            .collect();

        sessions.sort_by_key(|e| e.path());

        // Try the newest snapshot first; on parse failure, quarantine the
        // file and fall through to the next candidate so the user's
        // bookmarks, theme, and last-tab are not lost entirely.
        for entry in sessions.iter().rev() {
            match fs::read_to_string(entry.path())
                .and_then(|s| serde_json::from_str::<SessionState>(&s).map_err(|e| e.into()))
            {
                Ok(state) => return Ok(Some(state)),
                Err(e) => {
                    tracing::warn!(
                        path = %entry.path().display(),
                        error = %e,
                        "session file is corrupt; quarantining and trying next"
                    );
                    let quarantine = entry.path().with_extension("json.bad");
                    if let Err(e) = fs::rename(entry.path(), &quarantine) {
                        tracing::warn!(
                            path = %entry.path().display(),
                            error = %e,
                            "failed to quarantine corrupt session file"
                        );
                    }
                }
            }
        }

        // If no snapshots were valid, fall back to quick_save.json.
        self.load_quick()
    }

    fn cleanup_orphaned_tmp_files(&self) {
        let entries = match fs::read_dir(&self.config.session_dir) {
            Ok(e) => e,
            Err(_) => return,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path
                .file_name()
                .is_some_and(|n| n.to_string_lossy().ends_with(".json.tmp"))
            {
                let modified = match entry.metadata().and_then(|m| m.modified()) {
                    Ok(m) => m,
                    Err(e) => {
                        tracing::debug!(
                            path = %path.display(),
                            error = %e,
                            "failed to read metadata for temp session file"
                        );
                        continue;
                    }
                };
                let elapsed = match modified.elapsed() {
                    Ok(d) => d,
                    Err(e) => {
                        tracing::debug!(
                            path = %path.display(),
                            error = %e,
                            "failed to compute elapsed time for temp session file, skipping cleanup"
                        );
                        continue;
                    }
                };
                if elapsed > Duration::from_secs(3600) {
                    if let Err(e) = fs::remove_file(&path) {
                        tracing::debug!(
                            path = %path.display(),
                            error = %e,
                            "failed to remove orphaned temp session file"
                        );
                    }
                }
            }
        }
    }

    fn capture_state(&self, app: &App) -> SessionState {
        let current_tab_visible = app.current_tab.visible_index();
        SessionState {
            current_tab_id: Some(app.current_tab.stable_id().to_string()),
            bookmarks: app.get_bookmarked_tab_ids(),
            theme_name: app.theme_manager.current().name.to_string(),
            legacy_current_tab: current_tab_visible,
            legacy_bookmarks: app
                .get_bookmarked_tab_ids()
                .iter()
                .filter_map(|id| Tab::from_stable_id(id).and_then(|t| t.visible_index()))
                .collect(),
        }
    }

    fn cleanup_old_sessions(&self) -> anyhow::Result<()> {
        let entries = fs::read_dir(&self.config.session_dir)?;
        let mut sessions: Vec<_> = entries
            .filter_map(|e| match e {
                Ok(entry) => Some(entry),
                Err(e) => {
                    tracing::warn!("Skipping unreadable directory entry: {:?}", e);
                    None
                }
            })
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
            .filter(|e| {
                e.path()
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy() != "quick_save.json")
            })
            .collect();

        sessions.sort_by_key(|e| e.path());

        while sessions.len() > self.config.max_sessions {
            if let Some(oldest) = sessions.first() {
                if let Err(e) = fs::remove_file(oldest.path()) {
                    tracing::warn!("Failed to cleanup old session {:?}: {:?}", oldest.path(), e);
                    break;
                }
                sessions.remove(0);
            }
        }

        Ok(())
    }

    pub fn session_dir(&self) -> &PathBuf {
        &self.config.session_dir
    }

    pub fn auto_save_interval(&self) -> u64 {
        self.config.auto_save_interval_secs.max(1)
    }

    pub fn config(&self) -> &SessionConfig {
        &self.config
    }
}
