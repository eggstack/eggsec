use crate::tui::tabs::TabInput;

pub struct TabDispatcher<'a>(&'a mut dyn TabInput);

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

    #[allow(dead_code)]
    pub fn handle_char(&mut self, c: char) {
        self.0.handle_char(c);
    }

    #[allow(dead_code)]
    pub fn handle_backspace(&mut self) {
        self.0.handle_backspace();
    }

    #[allow(dead_code)]
    pub fn handle_escape(&mut self) {
        self.0.handle_escape();
    }

    pub fn handle_up(&mut self) {
        self.0.handle_up();
    }

    pub fn handle_down(&mut self) {
        self.0.handle_down();
    }

    #[allow(dead_code)]
    pub fn handle_left(&mut self) -> bool {
        self.0.handle_left()
    }

    #[allow(dead_code)]
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

    #[allow(dead_code)]
    pub fn is_input_focused(&self) -> bool {
        self.0.is_input_focused()
    }

    #[allow(dead_code)]
    pub fn is_at_left_edge(&self) -> bool {
        self.0.is_at_left_edge()
    }

    #[allow(dead_code)]
    pub fn is_at_right_edge(&self) -> bool {
        self.0.is_at_right_edge()
    }
}
