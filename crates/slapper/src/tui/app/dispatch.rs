use crate::tui::tabs::{AppState, TabInput, TabState};

pub struct TabDispatcher<'a>(&'a mut dyn TabInput);

impl<'a> TabDispatcher<'a> {
    pub fn handle_enter(&mut self) {
        self.0.handle_enter();
    }
}

impl<'a> TabDispatcher<'a> {
    pub fn new(tab_input: &'a mut dyn TabInput) -> Self {
        Self(tab_input)
    }

    pub fn handle_focus_next(&mut self) {
        self.0.handle_focus_next();
    }

    pub fn handle_focus_prev(&mut self) {
        self.0.handle_focus_prev();
    }

    pub fn handle_char(&mut self, c: char) {
        self.0.handle_char(c);
    }

    pub fn handle_backspace(&mut self) {
        self.0.handle_backspace();
    }

    pub fn handle_escape(&mut self) {
        self.0.handle_escape();
    }

    pub fn handle_up(&mut self) {
        self.0.handle_up();
    }

    pub fn handle_down(&mut self) {
        self.0.handle_down();
    }

    pub fn handle_left(&mut self) -> bool {
        self.0.handle_left()
    }

    pub fn handle_right(&mut self) -> bool {
        self.0.handle_right()
    }

    pub fn handle_word_forward(&mut self) {
        self.0.handle_word_forward();
    }

    pub fn handle_word_backward(&mut self) {
        self.0.handle_word_backward();
    }

    pub fn handle_home(&mut self) {
        self.0.handle_home();
    }

    pub fn handle_end(&mut self) {
        self.0.handle_end();
    }

    pub fn handle_top(&mut self) {
        self.0.handle_top();
    }

    pub fn handle_bottom(&mut self) {
        self.0.handle_bottom();
    }

    pub fn handle_autocomplete(&mut self) -> bool {
        self.0.handle_autocomplete()
    }

    pub fn is_input_focused(&self) -> bool {
        self.0.is_input_focused()
    }

    pub fn is_at_left_edge(&self) -> bool {
        self.0.is_at_left_edge()
    }

    pub fn is_at_right_edge(&mut self) -> bool {
        self.0.is_at_right_edge()
    }

    pub fn stop(&mut self) {
        self.0.stop();
    }

    pub fn page_up(&mut self, page_size: usize) {
        self.0.page_up(page_size);
    }

    pub fn page_down(&mut self, page_size: usize) {
        self.0.page_down(page_size);
    }

    pub fn handle_paste(&mut self, text: &str) {
        self.0.handle_paste(text);
    }

    pub fn reset(&mut self) {
        TabInput::reset(self.0);
    }

    pub fn is_running(&self) -> bool {
        TabState::state(self.0) == AppState::Running
    }
}
