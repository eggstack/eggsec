use crate::app::notifications::NotificationSeverity;
use crate::app::state::ThemeLoadReason;
use crate::theme::install::ThemeInstallReport;

impl super::App {
    pub fn spawn_theme_loader_with_reason(&mut self, reason: ThemeLoadReason) {
        if self.theme_load.is_running() {
            if reason == ThemeLoadReason::ManualReload {
                self.overlay.notification = Some(crate::app::notifications::Notification::new(
                    "Theme reload already in progress...".to_string(),
                    NotificationSeverity::Warning,
                ));
                self.needs_redraw = true;
            }
            return;
        }

        self.theme_load.reason = reason;
        let (tx, rx) = std::sync::mpsc::channel();
        self.theme_load.rx = Some(rx);
        self.theme_load.handle = Some(std::thread::spawn(move || {
            let report = crate::theme::install::load_and_install_themes();
            if let Err(err) = tx.send(report) {
                tracing::warn!(?err, "failed to send theme loading report");
            }
        }));

        if reason == ThemeLoadReason::ManualReload {
            self.overlay.notification = Some(crate::app::notifications::Notification::new(
                "Loading themes...".to_string(),
                NotificationSeverity::Info,
            ));
            self.needs_redraw = true;
        }
    }

    pub fn spawn_theme_loader(&mut self) {
        self.spawn_theme_loader_with_reason(ThemeLoadReason::Startup);
    }

    pub(crate) fn join_theme_loader_handle(&mut self) {
        if let Some(handle) = self.theme_load.handle.take() {
            if let Err(err) = handle.join() {
                tracing::warn!(?err, "theme loading thread panicked");
            }
        }
    }

    pub fn handle_theme_install_report(&mut self, report: ThemeInstallReport) {
        tracing::info!(
            "Theme loading complete: {} installed, {} loaded, {} skipped, {} errors",
            report.installed,
            report.loaded,
            report.skipped_existing,
            report.errors.len()
        );

        // Collect invalid theme file stems for metadata tracking.
        let mut invalid_stems: Vec<String> = Vec::new();

        for record in report.loaded_themes {
            match record.result {
                Ok(theme) => {
                    let source = record.source;
                    let theme_id = theme.name.clone();
                    self.theme_manager.register_theme_with_source(theme, source);
                    // Track contrast warnings on the registered theme.
                    if !record.contrast_warnings.is_empty() {
                        self.theme_manager.mark_theme_fallback_adjusted(&theme_id);
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to load theme '{}': {}", record.file_stem, e);
                    // Register invalid theme metadata so Settings can display it.
                    self.theme_manager.register_theme_invalid(
                        &record.file_stem,
                        record.source,
                        format!("{e}"),
                    );
                    invalid_stems.push(record.file_stem);
                }
            }
        }

        if self.theme_load.changed_by_user {
            self.theme_load.deferred_theme_name = None;
        } else if let Some(theme_name) = self.theme_load.deferred_theme_name.take() {
            if self.theme_manager.set_theme(&theme_name) {
                crate::theme::sync_theme_to_thread_local(self.theme_manager.current());
                tracing::info!(theme = %theme_name, "restored deferred theme after theme load");
            } else {
                tracing::warn!(
                    theme = %theme_name,
                    "deferred theme still unavailable after theme load"
                );
            }
        }

        // Surface installation failures to the user. Without this, a broken
        // archive or permission error leaves the user staring at the 3 built-in
        // themes with no idea why their favorite Halloy theme didn't appear.
        if !report.errors.is_empty() {
            let message = if report.installed == 0 && report.loaded == 0 {
                format!(
                    "Theme installation failed: {} error(s). Check logs.",
                    report.errors.len()
                )
            } else {
                format!(
                    "Theme installation: {} installed, {} errors. Some themes may be missing.",
                    report.installed,
                    report.errors.len()
                )
            };
            self.overlay.notification = Some(crate::app::notifications::Notification::new(
                message,
                NotificationSeverity::Warning,
            ));
            self.needs_redraw = true;
        } else if self.theme_load.reason == ThemeLoadReason::ManualReload {
            // Show feedback for manual reload with detailed counts.
            let new_themes = report.installed;
            let loaded = report.loaded;
            let errors = report.errors.len();
            let invalid = self.theme_manager.invalid_count();
            let mut parts = Vec::new();
            if new_themes > 0 {
                parts.push(format!("{} new", new_themes));
            }
            parts.push(format!("{} loaded", loaded));
            if invalid > 0 {
                parts.push(format!("{} invalid", invalid));
            }
            if errors > 0 {
                parts.push(format!("{} error(s)", errors));
            }
            let message = format!("Themes reloaded: {}.", parts.join(", "));
            self.overlay.notification = Some(crate::app::notifications::Notification::new(
                message,
                if errors > 0 {
                    NotificationSeverity::Warning
                } else {
                    NotificationSeverity::Info
                },
            ));
            self.needs_redraw = true;
        }

        self.update_settings_theme_selector();

        // Sync theme metadata to Settings tab for the detail pane.
        let dir_path = report
            .theme_dir
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "~/.config/eggsec/themes".to_string());

        // Compute contrast warnings for all loaded themes.
        let mut contrast_cache = rustc_hash::FxHashMap::default();
        for info in self.theme_manager.theme_info_list() {
            if info.status == crate::theme::manager::ThemeLoadStatus::Loaded {
                let warnings = self.theme_manager.validate_contrast(&info.id);
                if !warnings.is_empty() {
                    contrast_cache.insert(info.id.clone(), warnings);
                }
            }
        }

        // Resolve the currently selected theme's colors for preview.
        let selected_id = self
            .tabs
            .settings
            .theme_selector
            .selected_value()
            .map(|s| s.to_string())
            .unwrap_or_else(|| self.theme_manager.current_name().to_string());
        let resolved_theme_colors = self
            .theme_manager
            .get_theme(&selected_id)
            .map(|t| t.colors.clone());

        self.tabs.settings.update_theme_metadata(
            self.theme_manager.theme_info_list(),
            self.theme_manager.invalid_count(),
            dir_path,
            contrast_cache,
            resolved_theme_colors,
        );
    }
}
