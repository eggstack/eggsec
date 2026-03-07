mod history;

use crate::tui::tabs::HistoryTab;
use std::sync::{Arc, Mutex};

pub type SharedHistory = Arc<Mutex<HistoryTab>>;

pub fn create_shared_history() -> SharedHistory {
    Arc::new(Mutex::new(HistoryTab::new()))
}
