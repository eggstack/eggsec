mod history;

use crate::tui::tabs::HistoryTab;
use parking_lot::Mutex;
use std::sync::Arc;

pub type SharedHistory = Arc<Mutex<HistoryTab>>;

pub fn create_shared_history() -> SharedHistory {
    Arc::new(Mutex::new(HistoryTab::new()))
}
