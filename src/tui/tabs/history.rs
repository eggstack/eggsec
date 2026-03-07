use crate::tui::components::ScrollableText;
use crate::tui::tabs::{AppState, TabInput, TabRender, TabState};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use serde::Serialize;
use std::collections::VecDeque;

#[derive(Debug, Clone, Serialize)]
pub struct HistoryEntry {
    pub id: usize,
    pub timestamp: String,
    pub scan_type: String,
    pub target: String,
    pub summary: String,
    pub details: Vec<String>,
}

#[derive(Clone)]
pub struct HistoryTab {
    pub entries: VecDeque<HistoryEntry>,
    pub selected: Option<usize>,
    pub details_view: ScrollableText,
    pub next_id: usize,
    pub details_focused: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct HistoryExport {
    pub entries: Vec<HistoryEntry>,
}

impl HistoryTab {
    pub fn export(&self) -> String {
        let export_data = HistoryExport {
            entries: self.entries.iter().cloned().collect(),
        };
        serde_json::to_string_pretty(&export_data).unwrap_or_default()
    }
}

impl HistoryTab {
    pub fn new() -> Self {
        Self {
            entries: VecDeque::new(),
            selected: None,
            details_view: ScrollableText::new("Details"),
            next_id: 1,
            details_focused: false,
        }
    }

    pub fn add_entry(
        &mut self,
        scan_type: String,
        target: String,
        summary: String,
        details: Vec<String>,
    ) {
        let entry = HistoryEntry {
            id: self.next_id,
            timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            scan_type,
            target,
            summary,
            details,
        };
        self.next_id += 1;
        self.entries.push_front(entry);
        if self.entries.len() > 100 {
            self.entries.pop_back();
        }
        if self.selected.is_none() {
            self.selected = Some(0);
            self.update_details_view();
        }
    }

    pub fn select_next(&mut self) {
        if let Some(idx) = self.selected {
            if idx < self.entries.len().saturating_sub(1) {
                self.selected = Some(idx + 1);
                self.update_details_view();
            }
        } else if !self.entries.is_empty() {
            self.selected = Some(0);
            self.update_details_view();
        }
    }

    pub fn select_prev(&mut self) {
        if let Some(idx) = self.selected {
            if idx > 0 {
                self.selected = Some(idx - 1);
                self.update_details_view();
            }
        }
    }

    pub fn delete_selected(&mut self) {
        if let Some(idx) = self.selected {
            self.entries.remove(idx);
            if self.entries.is_empty() {
                self.selected = None;
                self.details_view.clear();
            } else {
                self.selected = Some(idx.min(self.entries.len() - 1));
                self.update_details_view();
            }
        }
    }

    pub fn clear_all(&mut self) {
        self.entries.clear();
        self.selected = None;
        self.details_view.clear();
    }

    pub fn get_selected_entry(&self) -> Option<&HistoryEntry> {
        self.selected.and_then(|idx| self.entries.get(idx))
    }

    pub fn filter_by_scan_type(&self, scan_type: &str) -> Vec<&HistoryEntry> {
        if scan_type.is_empty() {
            return self.entries.iter().collect();
        }
        self.entries
            .iter()
            .filter(|e| {
                e.scan_type
                    .to_lowercase()
                    .contains(&scan_type.to_lowercase())
            })
            .collect()
    }

    pub fn filter_by_target(&self, target: &str) -> Vec<&HistoryEntry> {
        if target.is_empty() {
            return self.entries.iter().collect();
        }
        self.entries
            .iter()
            .filter(|e| e.target.to_lowercase().contains(&target.to_lowercase()))
            .collect()
    }

    pub fn search(&self, query: &str) -> Vec<&HistoryEntry> {
        if query.is_empty() {
            return self.entries.iter().collect();
        }
        let query_lower = query.to_lowercase();
        self.entries
            .iter()
            .filter(|e| {
                e.target.to_lowercase().contains(&query_lower)
                    || e.scan_type.to_lowercase().contains(&query_lower)
                    || e.summary.to_lowercase().contains(&query_lower)
                    || e.details
                        .iter()
                        .any(|d| d.to_lowercase().contains(&query_lower))
            })
            .collect()
    }

    pub fn get_scan_types(&self) -> Vec<String> {
        let mut types: Vec<String> = self.entries.iter().map(|e| e.scan_type.clone()).collect();
        types.sort();
        types.dedup();
        types
    }

    pub fn update_details_view(&mut self) {
        self.details_view.clear();

        let entry_data = self.selected.and_then(|idx| {
            self.entries.get(idx).map(|e| {
                (
                    e.scan_type.clone(),
                    e.timestamp.clone(),
                    e.target.clone(),
                    e.summary.clone(),
                    e.details.clone(),
                )
            })
        });

        if let Some((scan_type, timestamp, target, summary, details)) = entry_data {
            self.details_view.add_line(Line::from(vec![
                Span::styled("Type: ", Style::default().fg(Color::Yellow)),
                Span::raw(scan_type),
            ]));
            self.details_view.add_line(Line::from(vec![
                Span::styled("Time: ", Style::default().fg(Color::Yellow)),
                Span::raw(timestamp),
            ]));
            self.details_view.add_line(Line::from(vec![
                Span::styled("Target: ", Style::default().fg(Color::Yellow)),
                Span::raw(target),
            ]));
            self.details_view.add_line(Line::from(vec![
                Span::styled("Summary: ", Style::default().fg(Color::Yellow)),
                Span::raw(summary),
            ]));
            self.details_view.add_line(Line::from(""));

            if !details.is_empty() {
                self.details_view.add_line(Line::from(Span::styled(
                    "Details:",
                    Style::default().fg(Color::Cyan),
                )));
                for detail in &details {
                    self.details_view.add_text(detail, None);
                }
            }
        }
    }

    pub fn scroll_details_up(&mut self) {
        self.details_view.scroll_up(1);
    }

    pub fn scroll_details_down(&mut self) {
        self.details_view.scroll_down(1);
    }

    pub fn page_up(&mut self, page_size: usize) {
        self.details_view.page_up(page_size);
    }

    pub fn page_down(&mut self, page_size: usize) {
        self.details_view.page_down(page_size);
    }
}

impl Default for HistoryTab {
    fn default() -> Self {
        Self::new()
    }
}

impl TabState for HistoryTab {
    fn state(&self) -> AppState {
        AppState::Idle
    }

    fn progress(&self) -> f64 {
        0.0
    }

    fn reset(&mut self) {
        self.clear_all();
    }
}

impl TabRender for HistoryTab {
    fn render(&self, f: &mut Frame, area: Rect, _insert_mode: bool) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        let list_area = chunks[0];
        let details_area = chunks[1];

        if self.entries.is_empty() {
            let empty =
                Paragraph::new("No history entries yet.\n\nRun a scan to see results here.")
                    .block(Block::default().borders(Borders::ALL).title("History"))
                    .style(Style::default().fg(Color::DarkGray));
            f.render_widget(empty, list_area);

            let placeholder = Paragraph::new("Select an entry to view details")
                .block(Block::default().borders(Borders::ALL).title("Details"))
                .style(Style::default().fg(Color::DarkGray));
            f.render_widget(placeholder, details_area);
            return;
        }

        let mut list_lines = vec![
            Line::from(vec![
                Span::styled(
                    format!("{:<20}", "TIME"),
                    Style::default().fg(Color::Yellow),
                ),
                Span::styled(
                    format!("{:<10}", "TYPE"),
                    Style::default().fg(Color::Yellow),
                ),
                Span::styled("TARGET", Style::default().fg(Color::Yellow)),
            ]),
            Line::from(Span::styled(
                "─".repeat(60),
                Style::default().fg(Color::DarkGray),
            )),
        ];

        for (idx, entry) in self.entries.iter().enumerate() {
            let is_selected = Some(idx) == self.selected;
            let style = if is_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let time_short = if entry.timestamp.len() > 16 {
                &entry.timestamp[5..16]
            } else {
                &entry.timestamp
            };

            let target_display = if entry.target.len() > 30 {
                format!("{}...", &entry.target[..27])
            } else {
                entry.target.clone()
            };

            let scan_type_short = match entry.scan_type.as_str() {
                "LoadTest" => "Load",
                "PortScan" => "Port",
                "EndpointScan" => "Endpoint",
                "Fingerprint" => "Finger",
                "Fuzz" => "Fuzz",
                "WAF" => "WAF",
                "Pipeline" => "Pipeline",
                other => other,
            };

            list_lines.push(Line::from(vec![
                Span::styled(format!("{:<20}", time_short), style),
                Span::styled(format!("{:<10}", scan_type_short), style),
                Span::styled(target_display, style),
            ]));
        }

        let list = Paragraph::new(list_lines)
            .block(Block::default().borders(Borders::ALL).title("History"));
        f.render_widget(list, list_area);

        if self.details_view.len() > 0 {
            self.details_view.render(f, details_area);
        } else {
            let placeholder = Paragraph::new("Select an entry to view details")
                .block(Block::default().borders(Borders::ALL).title("Details"))
                .style(Style::default().fg(Color::DarkGray));
            f.render_widget(placeholder, details_area);
        }
    }
}

impl TabInput for HistoryTab {
    fn handle_focus_next(&mut self) {
        self.details_focused = !self.details_focused;
    }
    fn handle_focus_prev(&mut self) {
        self.details_focused = !self.details_focused;
    }

    fn handle_char(&mut self, _c: char) {}
    fn handle_backspace(&mut self) {}

    fn handle_enter(&mut self) {}

    fn handle_escape(&mut self) {
        self.details_focused = false;
    }

    fn handle_up(&mut self) {
        self.select_prev();
    }

    fn handle_down(&mut self) {
        self.select_next();
    }

    fn handle_left(&mut self) -> bool {
        if self.details_focused {
            self.details_view.scroll_left(5);
            true
        } else {
            false
        }
    }

    fn handle_right(&mut self) -> bool {
        if self.details_focused {
            self.details_view.scroll_right(5);
            true
        } else {
            false
        }
    }

    fn handle_home(&mut self) {
        if self.details_focused {
            self.details_view.scroll_to_top();
        } else {
            self.selected = Some(0);
        }
    }

    fn handle_end(&mut self) {
        if self.details_focused {
            self.details_view.scroll_to_bottom();
        } else {
            self.selected = Some(self.entries.len().saturating_sub(1));
        }
    }

    fn handle_top(&mut self) {
        self.selected = Some(0);
    }

    fn handle_bottom(&mut self) {
        self.selected = Some(self.entries.len().saturating_sub(1));
    }

    fn is_input_focused(&self) -> bool {
        self.details_focused
    }
}
