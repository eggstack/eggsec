#![allow(dead_code)]

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

#[derive(Clone, Debug)]
pub struct ValidationResult {
    pub valid: bool,
    pub message: String,
}

pub struct InputField {
    pub label: String,
    pub value: String,
    pub focused: bool,
    pub cursor_pos: usize,
    pub width: Option<usize>,
    pub autocomplete: Option<Vec<&'static str>>,
}

impl InputField {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: String::new(),
            focused: false,
            cursor_pos: 0,
            width: None,
            autocomplete: None,
        }
    }

    pub fn with_value(mut self, value: impl Into<String>) -> Self {
        let v = value.into();
        self.cursor_pos = v.len();
        self.value = v;
        self
    }

    pub fn with_width(mut self, width: usize) -> Self {
        self.width = Some(width);
        self
    }

    pub fn with_autocomplete(mut self, completions: Vec<&'static str>) -> Self {
        self.autocomplete = Some(completions);
        self
    }

    pub fn get_autocomplete_suggestions(&self) -> Vec<String> {
        if let Some(ref completions) = self.autocomplete {
            if self.value.is_empty() {
                return completions.iter().map(|s| s.to_string()).collect();
            }
            completions
                .iter()
                .filter(|s| s.to_lowercase().starts_with(&self.value.to_lowercase()))
                .map(|s| s.to_string())
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn apply_autocomplete(&mut self, suggestion: &str) {
        self.value = suggestion.to_string();
        self.cursor_pos = self.value.len();
    }

    pub fn insert(&mut self, c: char) {
        if self.focused {
            self.value.insert(self.cursor_pos, c);
            self.cursor_pos += c.len_utf8();
        }
    }

    pub fn backspace(&mut self) {
        if self.focused && self.cursor_pos > 0 {
            self.cursor_pos -= 1;
            self.value.remove(self.cursor_pos);
        }
    }

    pub fn delete(&mut self) {
        if self.focused && self.cursor_pos < self.value.len() {
            self.value.remove(self.cursor_pos);
        }
    }

    pub fn move_left(&mut self) -> bool {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
            true
        } else {
            false
        }
    }

    pub fn move_right(&mut self) -> bool {
        if self.cursor_pos < self.value.len() {
            self.cursor_pos += 1;
            true
        } else {
            false
        }
    }

    pub fn move_home(&mut self) {
        self.cursor_pos = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor_pos = self.value.len();
    }

    pub fn clear(&mut self) {
        self.value.clear();
        self.cursor_pos = 0;
    }

    pub fn validate_url(&self) -> ValidationResult {
        if self.value.is_empty() {
            return ValidationResult {
                valid: false,
                message: "URL cannot be empty".to_string(),
            };
        }
        if !self.value.starts_with("http://") && !self.value.starts_with("https://") {
            return ValidationResult {
                valid: false,
                message: "URL must start with http:// or https://".to_string(),
            };
        }
        ValidationResult {
            valid: true,
            message: String::new(),
        }
    }

    pub fn validate_ip(&self) -> ValidationResult {
        if self.value.is_empty() {
            return ValidationResult {
                valid: false,
                message: "IP address cannot be empty".to_string(),
            };
        }
        let parts: Vec<&str> = self.value.split('.').collect();
        if parts.len() != 4 {
            return ValidationResult {
                valid: false,
                message: "Invalid IP address format (expected x.x.x.x)".to_string(),
            };
        }
        for part in parts {
            match part.parse::<u8>() {
                Ok(_) => {}
                Err(_) => {
                    return ValidationResult {
                        valid: false,
                        message: format!("Invalid octet: {}", part),
                    };
                }
            }
        }
        ValidationResult {
            valid: true,
            message: String::new(),
        }
    }

    pub fn validate_port(&self) -> ValidationResult {
        if self.value.is_empty() {
            return ValidationResult {
                valid: false,
                message: "Port cannot be empty".to_string(),
            };
        }
        match self.value.parse::<u16>() {
            Ok(port) => {
                if port == 0 {
                    return ValidationResult {
                        valid: false,
                        message: "Port must be between 1-65535".to_string(),
                    };
                }
                ValidationResult {
                    valid: true,
                    message: String::new(),
                }
            }
            Err(_) => ValidationResult {
                valid: false,
                message: "Invalid port number".to_string(),
            },
        }
    }

    pub fn validate_port_range(&self) -> ValidationResult {
        if self.value.is_empty() {
            return ValidationResult {
                valid: false,
                message: "Port range cannot be empty".to_string(),
            };
        }
        if self.value.contains('-') {
            let parts: Vec<&str> = self.value.split('-').collect();
            if parts.len() != 2 {
                return ValidationResult {
                    valid: false,
                    message: "Invalid port range format (expected: start-end)".to_string(),
                };
            }
            match (parts[0].parse::<u16>(), parts[1].parse::<u16>()) {
                (Ok(start), Ok(end)) => {
                    if start > end || start == 0 || end == 0 {
                        return ValidationResult {
                            valid: false,
                            message: "Invalid port range".to_string(),
                        };
                    }
                }
                _ => {
                    return ValidationResult {
                        valid: false,
                        message: "Invalid port numbers in range".to_string(),
                    };
                }
            }
        } else if self.value.contains(',') {
            for port_str in self.value.split(',') {
                match port_str.trim().parse::<u16>() {
                    Ok(0) => {
                        return ValidationResult {
                            valid: false,
                            message: format!("Invalid port: {}", port_str),
                        };
                    }
                    Err(_) => {
                        return ValidationResult {
                            valid: false,
                            message: format!("Invalid port: {}", port_str),
                        };
                    }
                    _ => {}
                }
            }
        }
        ValidationResult {
            valid: true,
            message: String::new(),
        }
    }

    pub fn validate_number(&self, min: u64, max: u64) -> ValidationResult {
        match self.value.parse::<u64>() {
            Ok(n) => {
                if n < min || n > max {
                    return ValidationResult {
                        valid: false,
                        message: format!("Value must be between {} and {}", min, max),
                    };
                }
                ValidationResult {
                    valid: true,
                    message: String::new(),
                }
            }
            Err(_) => ValidationResult {
                valid: false,
                message: "Invalid number".to_string(),
            },
        }
    }

    pub fn render(&self, f: &mut Frame, area: Rect, insert_mode: bool) {
        let style = if self.focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Gray)
        };

        let block = Block::default()
            .title(self.label.as_str())
            .borders(Borders::ALL)
            .border_style(style);

        let display_value = if let Some(w) = self.width {
            let available = w.saturating_sub(2);
            if self.value.len() > available {
                let start = self.cursor_pos.saturating_sub(available / 2);
                let end = (start + available).min(self.value.len());
                format!("{}...", &self.value[start..end])
            } else {
                self.value.clone()
            }
        } else {
            self.value.clone()
        };

        let paragraph = Paragraph::new(display_value).block(block);
        f.render_widget(paragraph, area);

        if self.focused && insert_mode {
            let display_cursor = if let Some(w) = self.width {
                let available = w.saturating_sub(2);
                if self.value.len() > available {
                    let start = self.cursor_pos.saturating_sub(available / 2);
                    if self.cursor_pos >= start && self.cursor_pos < start + available {
                        (self.cursor_pos - start) as u16
                    } else {
                        available as u16
                    }
                } else {
                    self.cursor_pos as u16
                }
            } else {
                self.cursor_pos as u16
            };

            let cursor_x = area.x + display_cursor + 1;
            let cursor_y = area.y + 1;
            if cursor_x < area.x + area.width {
                f.set_cursor_position((cursor_x, cursor_y));
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum InputState {
    None,
    Focused(usize),
}

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
        self.fields.push(field);
        self
    }

    pub fn focus_next(&mut self) {
        if let Some(idx) = self.focused {
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
        if let Some(idx) = self.focused {
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
                self.fields[current].focused = false;
            }
            self.fields[idx].focused = true;
            self.focused = Some(idx);
        }
    }

    pub fn blur(&mut self) {
        if let Some(idx) = self.focused {
            self.fields[idx].focused = false;
        }
        self.focused = None;
    }

    pub fn insert(&mut self, c: char) {
        if let Some(idx) = self.focused {
            self.fields[idx].insert(c);
        }
    }

    pub fn backspace(&mut self) {
        if let Some(idx) = self.focused {
            self.fields[idx].backspace();
        }
    }

    pub fn delete(&mut self) {
        if let Some(idx) = self.focused {
            self.fields[idx].delete();
        }
    }

    pub fn handle_tab(&mut self) {
        if let Some(idx) = self.focused {
            let suggestions = self.fields[idx].get_autocomplete_suggestions();
            if let Some(first) = suggestions.first() {
                self.fields[idx].apply_autocomplete(first);
            }
        }
    }

    pub fn move_left(&mut self) -> bool {
        if let Some(idx) = self.focused {
            self.fields[idx].move_left()
        } else {
            false
        }
    }

    pub fn move_right(&mut self) -> bool {
        if let Some(idx) = self.focused {
            self.fields[idx].move_right()
        } else {
            false
        }
    }

    pub fn is_focused(&self) -> bool {
        self.focused.is_some()
    }
}

impl Default for InputGroup {
    fn default() -> Self {
        Self::new()
    }
}
