use crate::theme::install::ThemeInstallReport;

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
        for theme_result in report.loaded_themes {
            match theme_result {
                Ok(theme) => self.theme_manager.register_theme(theme),
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

        self.update_settings_theme_selector();
    }
}
