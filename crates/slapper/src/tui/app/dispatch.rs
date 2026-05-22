use crate::tui::tabs::HistoryTab;
use crate::tui::tabs::{AppState, TabInput, TabState};
use parking_lot::MutexGuard;

pub enum TabDispatcher<'a> {
    Standard(&'a mut dyn TabInput),
    LockedHistory(MutexGuard<'a, HistoryTab>),
}

impl<'a> TabDispatcher<'a> {
    pub fn new(tab_input: &'a mut dyn TabInput) -> Self {
        Self::Standard(tab_input)
    }

    pub fn new_locked(history: MutexGuard<'a, HistoryTab>) -> Self {
        Self::LockedHistory(history)
    }

    pub fn handle_focus_next(&mut self) {
        match self {
            Self::Standard(t) => t.handle_focus_next(),
            Self::LockedHistory(h) => h.handle_focus_next(),
        }
    }

    pub fn handle_focus_prev(&mut self) {
        match self {
            Self::Standard(t) => t.handle_focus_prev(),
            Self::LockedHistory(h) => h.handle_focus_prev(),
        }
    }

    pub fn handle_char(&mut self, c: char) {
        match self {
            Self::Standard(t) => t.handle_char(c),
            Self::LockedHistory(h) => h.handle_char(c),
        }
    }

    pub fn handle_backspace(&mut self) {
        match self {
            Self::Standard(t) => t.handle_backspace(),
            Self::LockedHistory(h) => h.handle_backspace(),
        }
    }

    pub fn handle_delete(&mut self) {
        match self {
            Self::Standard(t) => t.handle_delete(),
            Self::LockedHistory(h) => h.handle_delete(),
        }
    }

    pub fn handle_enter(&mut self) {
        match self {
            Self::Standard(t) => t.handle_enter(),
            Self::LockedHistory(h) => h.handle_enter(),
        }
    }

    pub fn handle_escape(&mut self) {
        match self {
            Self::Standard(t) => t.handle_escape(),
            Self::LockedHistory(h) => h.handle_escape(),
        }
    }

    pub fn handle_up(&mut self) {
        match self {
            Self::Standard(t) => t.handle_up(),
            Self::LockedHistory(h) => h.handle_up(),
        }
    }

    pub fn handle_down(&mut self) {
        match self {
            Self::Standard(t) => t.handle_down(),
            Self::LockedHistory(h) => h.handle_down(),
        }
    }

    pub fn handle_left(&mut self) -> bool {
        match self {
            Self::Standard(t) => t.handle_left(),
            Self::LockedHistory(h) => h.handle_left(),
        }
    }

    pub fn handle_right(&mut self) -> bool {
        match self {
            Self::Standard(t) => t.handle_right(),
            Self::LockedHistory(h) => h.handle_right(),
        }
    }

    pub fn handle_word_forward(&mut self) {
        match self {
            Self::Standard(t) => t.handle_word_forward(),
            Self::LockedHistory(h) => h.handle_word_forward(),
        }
    }

    pub fn handle_word_backward(&mut self) {
        match self {
            Self::Standard(t) => t.handle_word_backward(),
            Self::LockedHistory(h) => h.handle_word_backward(),
        }
    }

    pub fn handle_home(&mut self) {
        match self {
            Self::Standard(t) => t.handle_home(),
            Self::LockedHistory(h) => h.handle_home(),
        }
    }

    pub fn handle_end(&mut self) {
        match self {
            Self::Standard(t) => t.handle_end(),
            Self::LockedHistory(h) => h.handle_end(),
        }
    }

    pub fn handle_top(&mut self) {
        match self {
            Self::Standard(t) => t.handle_top(),
            Self::LockedHistory(h) => h.handle_top(),
        }
    }

    pub fn handle_bottom(&mut self) {
        match self {
            Self::Standard(t) => t.handle_bottom(),
            Self::LockedHistory(h) => h.handle_bottom(),
        }
    }

    pub fn handle_paste(&mut self, text: &str) {
        match self {
            Self::Standard(t) => t.handle_paste(text),
            Self::LockedHistory(h) => h.handle_paste(text),
        }
    }

    pub fn handle_copy(&mut self) -> Option<String> {
        match self {
            Self::Standard(t) => t.handle_copy(),
            Self::LockedHistory(h) => h.handle_copy(),
        }
    }

    pub fn handle_autocomplete(&mut self) -> bool {
        match self {
            Self::Standard(t) => t.handle_autocomplete(),
            Self::LockedHistory(h) => h.handle_autocomplete(),
        }
    }

    pub fn is_input_focused(&self) -> bool {
        match self {
            Self::Standard(t) => t.is_input_focused(),
            Self::LockedHistory(h) => h.is_input_focused(),
        }
    }

    pub fn is_at_left_edge(&self) -> bool {
        match self {
            Self::Standard(t) => t.is_at_left_edge(),
            Self::LockedHistory(h) => h.is_at_left_edge(),
        }
    }

    pub fn is_at_right_edge(&self) -> bool {
        match self {
            Self::Standard(t) => t.is_at_right_edge(),
            Self::LockedHistory(h) => h.is_at_right_edge(),
        }
    }

    pub fn page_up(&mut self, page_size: usize) {
        match self {
            Self::Standard(t) => t.page_up(page_size),
            Self::LockedHistory(h) => h.page_up(page_size),
        }
    }

    pub fn page_down(&mut self, page_size: usize) {
        match self {
            Self::Standard(t) => t.page_down(page_size),
            Self::LockedHistory(h) => h.page_down(page_size),
        }
    }

    pub fn reset(&mut self) {
        match self {
            Self::Standard(t) => t.reset(),
            Self::LockedHistory(h) => h.reset(),
        }
    }

    pub fn is_running(&self) -> bool {
        match self {
            Self::Standard(t) => TabState::state(*t) == AppState::Running,
            Self::LockedHistory(h) => TabState::state(&**h) == AppState::Running,
        }
    }
}
