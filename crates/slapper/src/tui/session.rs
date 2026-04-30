use crate::tui::tabs::Tab;
use crate::tui::App;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

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
        fs::write(&path, json)?;

        self.cleanup_old_sessions()?;

        Ok(path)
    }

    pub fn save_quick(&self, app: &App) -> anyhow::Result<PathBuf> {
        let path = self.config.session_dir.join("quick_save.json");
        fs::create_dir_all(&self.config.session_dir)?;
        let state = self.capture_state(app);
        let json = serde_json::to_string_pretty(&state)?;
        fs::write(&path, json)?;
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
            .filter_map(|e| e.ok())
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

    pub fn restore_session(&self, app: &mut App, state: &SessionState) {
        let tab_to_restore = state.current_tab_id.as_ref()
            .and_then(|id| Tab::from_stable_id(id))
            .or_else(|| {
                state.legacy_current_tab.and_then(|idx| Tab::from_index(idx))
            })
            .unwrap_or(Tab::Recon);
        app.current_tab = tab_to_restore;

        for bookmark_id in &state.bookmarks {
            if let Some(tab) = Tab::from_stable_id(bookmark_id) {
                if let Some(idx) = tab.visible_index() {
                    app.bookmarks.insert(idx);
                }
            }
        }

        for &idx in &state.legacy_bookmarks {
            if let Some(tab) = Tab::from_index(idx) {
                if let Some(visible_idx) = tab.visible_index() {
                    app.bookmarks.insert(visible_idx);
                }
            }
        }
    }

    fn capture_state(&self, app: &App) -> SessionState {
        SessionState {
            current_tab_id: Some(app.current_tab.stable_id().to_string()),
            bookmarks: app.get_bookmarked_tabs().iter().filter_map(|&idx| {
                Tab::from_index(idx).map(|t| t.stable_id().to_string())
            }).collect(),
            theme_name: "dark".to_string(),
            legacy_current_tab: Some(app.current_tab as usize),
            legacy_bookmarks: app.get_bookmarked_tabs(),
        }
    }

    fn cleanup_old_sessions(&self) -> anyhow::Result<()> {
        let entries = fs::read_dir(&self.config.session_dir)?;
        let mut sessions: Vec<_> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
            .collect();

        sessions.sort_by_key(|e| e.path());

        while sessions.len() > self.config.max_sessions {
            if let Some(oldest) = sessions.first() {
                let _ = fs::remove_file(oldest.path());
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