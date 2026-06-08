use std::time::Duration;

use crate::workers;

impl super::App {
    pub fn has_active_task(&self) -> bool {
        self.task_handle.is_some()
            || self.task_tab.is_some()
            || self.progress_rx.is_some()
            || self.result_rx.is_some()
    }

    fn stop_tab_state(&mut self, tab: super::tabs::Tab) {
        let mut tab = tab;
        tab.as_tab_input(self).stop();
    }

    fn clear_task_runtime(&mut self) {
        if let Some(handle) = self.task_handle.take() {
            handle.abort();
        }
        if let Some(abort) = self.task_inner_abort.take() {
            abort.abort();
        }
        self.task_tab = None;
        if let Some(rx) = self.progress_rx.take() {
            drop(rx);
        }
        if let Some(rx) = self.result_rx.take() {
            drop(rx);
        }
    }

    pub fn stop(&mut self) {
        let tab = self.task_tab.unwrap_or(self.current_tab);
        self.stop_tab_state(tab);
        self.clear_task_runtime();
    }

    pub fn stop_with_message(&mut self, message: &str) {
        let tab = self.task_tab.unwrap_or(self.current_tab);
        self.stop_tab_state(tab);
        self.clear_task_runtime();

        // Reuse current tab-targeted error plumbing by temporarily scoping task_tab.
        self.task_tab = Some(tab);
        self.set_error_for_current_tab(crate::app::tab_error::TabError::Target(
            message.to_string(),
        ));
        self.task_tab = None;
    }

    pub(crate) fn spawn_task(&mut self, config: Option<workers::TaskConfig>) {
        if let Some(config) = config {
            if self.has_active_task() {
                tracing::warn!(
                    "A task is already running. Aborting previous task before starting new one."
                );
                self.clear_task_runtime();
            }

            let (progress_tx, progress_rx) = tokio::sync::mpsc::channel(100);
            let (result_tx, result_rx) = tokio::sync::mpsc::channel(1);

            let runner = workers::TaskRunner::new(config, progress_tx, result_tx.clone());
            let error_tx = result_tx.clone();

            self.progress_rx = Some(progress_rx);
            self.result_rx = Some(result_rx);

            self.task_tab = Some(self.current_tab);

            let inner_handle = tokio::spawn(async move { runner.run().await });

            let inner_abort = inner_handle.abort_handle();
            let handle_to_abort = inner_abort.clone();
            self.task_inner_abort = Some(inner_abort);

            let handle = tokio::spawn(async move {
                match tokio::time::timeout(Duration::from_secs(300), inner_handle).await {
                    Ok(Ok(Ok(()))) => {}
                    Ok(Ok(Err(e))) => {
                        let friendly_error = super::make_friendly_error(&e);
                        tracing::error!("Task failed: {}", friendly_error);
                        if let Err(e) = error_tx
                            .send(workers::TaskResult::Error(friendly_error))
                            .await
                        {
                            tracing::warn!("Failed to send task error result: {:?}", e);
                        }
                    }
                    Ok(Err(join_error)) => {
                        if join_error.is_cancelled() {
                            tracing::error!("Task was cancelled");
                            if let Err(e) = error_tx
                                .send(workers::TaskResult::Error("Task was cancelled".to_string()))
                                .await
                            {
                                tracing::warn!("Failed to send task error result: {:?}", e);
                            }
                        } else if join_error.is_panic() {
                            tracing::error!("Task panicked");
                            if let Err(e) = error_tx
                                .send(workers::TaskResult::Error("Task panicked".to_string()))
                                .await
                            {
                                tracing::warn!("Failed to send task error result: {:?}", e);
                            }
                        } else {
                            tracing::error!("Task failed: {}", join_error);
                            if let Err(e) = error_tx
                                .send(workers::TaskResult::Error("Task failed".to_string()))
                                .await
                            {
                                tracing::warn!("Failed to send task error result: {:?}", e);
                            }
                        }
                    }
                    Err(_) => {
                        tracing::error!("Task timed out after 300s - aborting task");
                        handle_to_abort.abort();
                        if let Err(e) = error_tx
                            .send(workers::TaskResult::Error(
                                "Task timed out after 300 seconds".to_string(),
                            ))
                            .await
                        {
                            tracing::warn!("Failed to send task error result: {:?}", e);
                        }
                    }
                }
            });
            self.task_handle = Some(handle);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::app::{create_shared_history, App};
    use crate::tabs::{AppState, Tab};

    fn create_test_app() -> App {
        App::new_for_testing(create_shared_history())
    }

    #[test]
    fn stop_with_message_targets_task_tab_when_current_tab_differs() {
        let mut app = create_test_app();
        app.current_tab = Tab::Dashboard;
        app.task_tab = Some(Tab::Recon);
        app.recon.state = AppState::Running;

        app.stop_with_message("Interrupted by user");

        assert!(matches!(app.recon.state, AppState::Error(ref m) if m == "Interrupted by user"));
        assert!(app.task_tab.is_none());
        assert!(!app.has_active_task());
    }

    #[test]
    fn stop_targets_task_tab_state_when_current_tab_differs() {
        let mut app = create_test_app();
        app.current_tab = Tab::Dashboard;
        app.task_tab = Some(Tab::Recon);
        app.recon.state = AppState::Running;

        app.stop();

        assert!(matches!(app.recon.state, AppState::Idle));
        assert!(app.task_tab.is_none());
        assert!(!app.has_active_task());
    }
}
