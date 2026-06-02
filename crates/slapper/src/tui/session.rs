use crate::tui::tabs::Tab;
use crate::tui::App;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

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
        directories::ProjectDirs::from("com", "slapper", "slapper")
            .map(|dirs| dirs.data_dir().join("sessions"))
            .unwrap_or_else(|| PathBuf::from("~/.slapper/sessions"))
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

        self.cleanup_old_sessions()?;

        Ok(path)
    }

    pub fn save_quick(&self, app: &App) -> anyhow::Result<PathBuf> {
        let path = self.config.session_dir.join("quick_save.json");
        fs::create_dir_all(&self.config.session_dir)?;
        let state = self.capture_state(app);
        let json = serde_json::to_string_pretty(&state)?;
        let tmp_path = path.with_extension("json.tmp");
        fs::write(&tmp_path, &json)?;
        fs::rename(&tmp_path, &path)?;
        Ok(path)
    }

    pub fn load_quick(&self) -> anyhow::Result<Option<SessionState>> {
        let path = self.config.session_dir.join("quick_save.json");
        if !path.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(&path)?;
        let state: SessionState = serde_json::from_str(&content)?;
        Ok(Some(state))
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
            .collect();

        sessions.sort_by_key(|e| e.path());

        if let Some(latest) = sessions.last() {
            let content = fs::read_to_string(latest.path())?;
            let state: SessionState = serde_json::from_str(&content)?;
            Ok(Some(state))
        } else {
            Ok(None)
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
                    .is_some_and(|name| name != "quick_save.json")
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
        self.config.auto_save_interval_secs
    }

    pub fn config(&self) -> &SessionConfig {
        &self.config
    }
}
