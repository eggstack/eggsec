use super::input_field::InputField;

pub struct InputGroup {
    pub fields: Vec<InputField>,
    pub focused: Option<usize>,
}

impl InputGroup {
    pub fn new() -> Self {
        Self {
            fields: Vec::new(),
            focused: None,
        }
    }

    pub fn add(mut self, field: InputField) -> Self {
        assert!(
            !self.has_label(&field.label),
            "duplicate input field label in group: {}",
            field.label
        );
        self.fields.push(field);
        self
    }

    fn label_key(label: &str) -> String {
        label.trim().to_lowercase()
    }

    pub fn has_label(&self, label: &str) -> bool {
        let label = Self::label_key(label);
        self.fields
            .iter()
            .any(|field| Self::label_key(&field.label) == label)
    }

    pub fn field_value(&self, label: &str) -> Option<&str> {
        let label = Self::label_key(label);
        self.fields
            .iter()
            .find(|field| Self::label_key(&field.label) == label)
            .map(|field| field.value.as_str())
    }

    pub fn set_field_value(&mut self, label: &str, value: impl Into<String>) -> bool {
        let label = Self::label_key(label);
        let Some(field) = self
            .fields
            .iter_mut()
            .find(|field| Self::label_key(&field.label) == label)
        else {
            return false;
        };
        let value = value.into();
        field.cursor_pos = value.len();
        field.value = value;
        true
    }

    /// Return the current focused index if it is valid, or clear stale state and return None.
    pub(crate) fn valid_focused_index(&mut self) -> Option<usize> {
        match self.focused {
            Some(idx) if idx < self.fields.len() => Some(idx),
            Some(_) => {
                self.focused = None;
                None
            }
            None => None,
        }
    }

    /// Read-only version of valid_focused_index that does not mutate self.
    pub(crate) fn valid_focused_index_ref(&self) -> Option<usize> {
        self.focused.filter(|&idx| idx < self.fields.len())
    }

    pub fn focus_next(&mut self) {
        if self.fields.is_empty() {
            return;
        }
        if let Some(idx) = self.valid_focused_index() {
            self.fields[idx].focused = false;
            let next = (idx + 1) % self.fields.len();
            self.fields[next].focused = true;
            self.focused = Some(next);
        } else if !self.fields.is_empty() {
            self.fields[0].focused = true;
            self.focused = Some(0);
        }
    }

    pub fn focus_prev(&mut self) {
        if self.fields.is_empty() {
            return;
        }
        if let Some(idx) = self.valid_focused_index() {
            self.fields[idx].focused = false;
            let prev = if idx == 0 {
                self.fields.len() - 1
            } else {
                idx - 1
            };
            self.fields[prev].focused = true;
            self.focused = Some(prev);
        } else if !self.fields.is_empty() {
            let last = self.fields.len() - 1;
            self.fields[last].focused = true;
            self.focused = Some(last);
        }
    }

    pub fn focus(&mut self, idx: usize) {
        if idx < self.fields.len() {
            if let Some(current) = self.focused {
                if current < self.fields.len() {
                    self.fields[current].focused = false;
                }
            }
            self.fields[idx].focused = true;
            self.focused = Some(idx);
        }
    }

    pub fn blur(&mut self) {
        if let Some(idx) = self.focused {
            if idx < self.fields.len() {
                self.fields[idx].focused = false;
            }
        }
        self.focused = None;
    }

    pub fn insert(&mut self, c: char) {
        if let Some(idx) = self.valid_focused_index() {
            self.fields[idx].insert(c);
        }
    }

    pub fn paste(&mut self, text: &str) {
        if let Some(idx) = self.valid_focused_index() {
            self.fields[idx].paste(text);
        }
    }

    pub fn backspace(&mut self) {
        if let Some(idx) = self.valid_focused_index() {
            self.fields[idx].backspace();
        }
    }

    pub fn delete(&mut self) {
        if let Some(idx) = self.valid_focused_index() {
            self.fields[idx].delete();
        }
    }

    pub fn handle_autocomplete(&mut self) -> bool {
        if let Some(idx) = self.valid_focused_index() {
            let suggestions = self.fields[idx].get_autocomplete_suggestions();
            if let Some(first) = suggestions.first() {
                self.fields[idx].apply_autocomplete(first);
                return true;
            }
        }
        false
    }

    pub fn move_left(&mut self) -> bool {
        if let Some(idx) = self.valid_focused_index() {
            self.fields[idx].move_left()
        } else {
            false
        }
    }

    pub fn move_right(&mut self) -> bool {
        if let Some(idx) = self.valid_focused_index() {
            self.fields[idx].move_right()
        } else {
            false
        }
    }

    pub fn move_word_forward(&mut self) {
        if let Some(idx) = self.valid_focused_index() {
            self.fields[idx].move_word_forward();
        }
    }

    pub fn move_word_backward(&mut self) {
        if let Some(idx) = self.valid_focused_index() {
            self.fields[idx].move_word_backward();
        }
    }

    pub fn move_home(&mut self) {
        if let Some(idx) = self.valid_focused_index() {
            self.fields[idx].move_home();
        }
    }

    pub fn move_end(&mut self) {
        if let Some(idx) = self.valid_focused_index() {
            self.fields[idx].move_end();
        }
    }

    pub fn get_focused_value(&self) -> Option<String> {
        self.valid_focused_index_ref()
            .map(|idx| self.fields[idx].get_value())
    }

    pub fn is_focused(&self) -> bool {
        self.valid_focused_index_ref().is_some()
    }

    pub fn is_at_left_edge(&self) -> bool {
        if let Some(idx) = self.valid_focused_index_ref() {
            self.fields[idx].is_at_left_edge()
        } else {
            true
        }
    }

    pub fn is_at_right_edge(&self) -> bool {
        if let Some(idx) = self.valid_focused_index_ref() {
            self.fields[idx].is_at_right_edge()
        } else {
            true
        }
    }

    /// Synchronize per-field `focused` flags with the given index.
    /// `None` blurs all fields; `Some(i)` focuses field `i` (if in bounds).
    pub fn set_focus_for_index(&mut self, idx: Option<usize>) {
        let idx = idx.filter(|idx| *idx < self.fields.len());
        for (i, field) in self.fields.iter_mut().enumerate() {
            field.focused = Some(i) == idx;
        }
        self.focused = idx;
    }

    /// Clear every field's value and cursor position.
    pub fn clear_all_fields(&mut self) {
        for field in &mut self.fields {
            field.clear();
        }
    }

    pub fn can_move_left(&self) -> bool {
        if let Some(idx) = self.valid_focused_index_ref() {
            self.fields[idx].cursor_pos > 0
        } else {
            false
        }
    }

    pub fn can_move_right(&self) -> bool {
        if let Some(idx) = self.valid_focused_index_ref() {
            self.fields[idx].cursor_pos < self.fields[idx].value.len()
        } else {
            false
        }
    }

    pub fn duplicate_label_names(&self) -> Vec<String> {
        let mut seen = Vec::new();
        let mut duplicate_keys = Vec::new();
        let mut duplicates = Vec::new();
        for field in &self.fields {
            let key = Self::label_key(&field.label);
            if seen.contains(&key) && !duplicate_keys.contains(&key) {
                duplicate_keys.push(key.clone());
                duplicates.push(field.label.clone());
            }
            if !seen.contains(&key) {
                seen.push(key);
            }
        }
        duplicates
    }

    pub fn focus_state_is_consistent(&self) -> bool {
        let focused_flags: Vec<usize> = self
            .fields
            .iter()
            .enumerate()
            .filter_map(|(idx, field)| field.focused.then_some(idx))
            .collect();

        match self.valid_focused_index_ref() {
            Some(idx) => focused_flags == [idx],
            None => focused_flags.is_empty(),
        }
    }
}

impl Default for InputGroup {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_focus_insert_does_not_panic() {
        let mut group = InputGroup::new().add(InputField::new("Field 1"));
        group.focused = Some(99);
        group.insert('a');
        assert!(group.focused.is_none());
    }

    #[test]
    fn stale_focus_blur_does_not_panic() {
        let mut group = InputGroup::new().add(InputField::new("Field 1"));
        group.focused = Some(99);
        group.blur();
        assert!(group.focused.is_none());
    }

    #[test]
    fn stale_focus_move_left_does_not_panic() {
        let mut group = InputGroup::new().add(InputField::new("Field 1"));
        group.focused = Some(99);
        assert!(!group.move_left());
        assert!(group.focused.is_none());
    }

    #[test]
    fn stale_focus_get_focused_value_returns_none() {
        let mut group = InputGroup::new().add(InputField::new("Field 1"));
        group.focused = Some(99);
        assert!(group.get_focused_value().is_none());
    }

    #[test]
    fn focus_next_recovers_from_stale_focus() {
        let mut group = InputGroup::new()
            .add(InputField::new("Field 1"))
            .add(InputField::new("Field 2"));
        group.focused = Some(99);
        group.focus_next();
        assert_eq!(group.focused, Some(0));
        assert!(group.fields[0].focused);
    }

    #[test]
    fn focus_prev_recovers_from_stale_focus() {
        let mut group = InputGroup::new()
            .add(InputField::new("Field 1"))
            .add(InputField::new("Field 2"));
        group.focused = Some(99);
        group.focus_prev();
        assert_eq!(group.focused, Some(1));
        assert!(group.fields[1].focused);
    }

    #[test]
    fn duplicate_label_names_reports_each_duplicate_once() {
        let mut group = InputGroup::new()
            .add(InputField::new("Target"))
            .add(InputField::new("Port"));
        group.fields.push(InputField::new(" target "));
        group.fields.push(InputField::new("PORT"));
        group.fields.push(InputField::new("Port"));
        assert_eq!(group.duplicate_label_names(), vec![" target ", "PORT"]);
    }

    #[test]
    #[should_panic(expected = "duplicate input field label in group: Target")]
    fn add_rejects_duplicate_labels() {
        let _group = InputGroup::new()
            .add(InputField::new("Target"))
            .add(InputField::new("Target"));
    }

    #[test]
    fn set_field_value_updates_by_label_and_cursor() {
        let mut group = InputGroup::new()
            .add(InputField::new("Target"))
            .add(InputField::new("Timeout"));
        assert!(group.set_field_value(" timeout ", "\u{e9}x"));
        assert_eq!(group.field_value("TIMEOUT"), Some("\u{e9}x"));
        assert_eq!(group.fields[1].cursor_pos, "\u{e9}x".len());
        assert!(!group.set_field_value("missing", "value"));
    }
}
