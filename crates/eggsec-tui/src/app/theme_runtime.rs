use crate::app::notifications::NotificationSeverity;
use crate::theme::install::ThemeInstallReport;
use crate::theme::ThemeSource;

impl super::App {
    pub fn spawn_theme_loader(&mut self) {
        if self.theme_load.is_running() {
            tracing::debug!("theme loader already running");
            return;
        }

        let (tx, rx) = std::sync::mpsc::channel();
        self.theme_load.rx = Some(rx);
        self.theme_load.handle = Some(std::thread::spawn(move || {
            let report = crate::theme::install::load_and_install_themes();
            if let Err(err) = tx.send(report) {
                tracing::warn!(?err, "failed to send theme loading report");
            }
        }));
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

        let packaged_count = report.installed;
        let mut loaded_from_packaged = packaged_count;
        for theme_result in report.loaded_themes {
            match theme_result {
                Ok(theme) => {
                    let source = if loaded_from_packaged > 0 {
                        loaded_from_packaged -= 1;
                        ThemeSource::Packaged
                    } else {
                        ThemeSource::Custom
                    };
                    self.theme_manager.register_theme_with_source(theme, source);
                }
                Err(e) => tracing::warn!("Failed to load theme: {}", e),
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
        }

        self.update_settings_theme_selector();

        // Sync theme metadata to Settings tab for the detail pane.
        let dir_path = report
            .theme_dir
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "~/.config/eggsec/themes".to_string());
        self.tabs.settings.update_theme_metadata(
            self.theme_manager.theme_info_list(),
            self.theme_manager.invalid_count(),
            dir_path,
        );
    }
}
