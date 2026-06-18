use crate::app::tab_error::TabError;
use crate::components::ScrollableText;
use crate::tabs::{AppState, TabInput, TabRender, TabState};
use crate::tc;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use serde::Serialize;
use std::collections::VecDeque;

const DEFAULT_HISTORY_LIMIT: usize = 100;

#[derive(Debug, Clone, Serialize)]
pub struct HistoryEntry {
    pub id: usize,
    pub timestamp: String,
    pub scan_type: String,
    pub target: String,
    pub summary: String,
    pub details: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryFocusArea {
    List,
    Details,
}

#[derive(Clone)]
pub struct HistoryTab {
    pub entries: VecDeque<HistoryEntry>,
    pub selected: Option<usize>,
    pub results_view: ScrollableText,
    pub next_id: usize,
    pub focus_area: HistoryFocusArea,
    pub scroll_offset: usize,
    pub visible_rows: usize,
    pub error: Option<TabError>,
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
        match serde_json::to_string_pretty(&export_data) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("Failed to serialize history export: {}", e);
                String::new()
            }
        }
    }
}

impl HistoryTab {
    pub fn new() -> Self {
        Self {
            entries: VecDeque::new(),
            selected: None,
            results_view: ScrollableText::new("Details"),
            next_id: 1,
            focus_area: HistoryFocusArea::List,
            scroll_offset: 0,
            visible_rows: 20,
            error: None,
        }
    }

    fn calc_visible_rows(&self, area: Rect) -> usize {
        let header_lines = 3;
        area.height.saturating_sub(header_lines).max(1).into()
    }

    fn ensure_visible(&mut self) {
        if let Some(idx) = self.selected {
            if idx < self.scroll_offset {
                self.scroll_offset = idx;
            } else if idx >= self.scroll_offset + self.visible_rows {
                self.scroll_offset = idx.saturating_sub(self.visible_rows.saturating_sub(1));
            }
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
        if self.entries.len() > DEFAULT_HISTORY_LIMIT {
            self.entries.pop_back();
        }
        if self.selected.is_none() {
            self.selected = Some(0);
            self.update_results_view();
        }
    }

    pub fn select_next(&mut self) {
        if let Some(idx) = self.selected {
            if idx < self.entries.len().saturating_sub(1) {
                self.selected = Some(idx + 1);
                self.ensure_visible();
                self.update_results_view();
            }
        } else if !self.entries.is_empty() {
            self.selected = Some(0);
            self.update_results_view();
        }
    }

    pub fn select_prev(&mut self) {
        if let Some(idx) = self.selected {
            if idx > 0 {
                self.selected = Some(idx - 1);
                self.ensure_visible();
                self.update_results_view();
            }
        }
    }

    pub fn delete_selected(&mut self) {
        if let Some(idx) = self.selected {
            if idx < self.entries.len() {
                self.entries.remove(idx);
            }
            if self.entries.is_empty() {
                self.selected = None;
                self.results_view.clear();
            } else {
                self.selected = Some(idx.min(self.entries.len() - 1));
                self.update_results_view();
            }
        }
    }

    pub fn clear_all(&mut self) {
        self.entries.clear();
        self.selected = None;
        self.results_view.clear();
        self.scroll_offset = 0;
    }

    pub fn get_selected_entry(&self) -> Option<&HistoryEntry> {
        self.selected.and_then(|idx| self.entries.get(idx))
    }

    pub fn filter_by_scan_type(&self, scan_type: &str) -> Vec<&HistoryEntry> {
        if scan_type.is_empty() {
            return self.entries.iter().collect();
        }
        let scan_type_lower = scan_type.to_lowercase();
        let entry_lowers: Vec<_> = self
            .entries
            .iter()
            .map(|e| e.scan_type.to_lowercase())
            .collect();
        self.entries
            .iter()
            .zip(entry_lowers.iter())
            .filter(|(_, scan_type_lower_e)| scan_type_lower_e.contains(&scan_type_lower))
            .map(|(e, _)| e)
            .collect()
    }

    pub fn filter_by_target(&self, target: &str) -> Vec<&HistoryEntry> {
        if target.is_empty() {
            return self.entries.iter().collect();
        }
        let target_lower = target.to_lowercase();
        let entry_lowers: Vec<_> = self
            .entries
            .iter()
            .map(|e| e.target.to_lowercase())
            .collect();
        self.entries
            .iter()
            .zip(entry_lowers.iter())
            .filter(|(_, target_lower_e)| target_lower_e.contains(&target_lower))
            .map(|(e, _)| e)
            .collect()
    }

    pub fn search(&self, query: &str) -> Vec<&HistoryEntry> {
        if query.is_empty() {
            return self.entries.iter().collect();
        }
        let query_lower = query.to_lowercase();

        let entry_lowers: Vec<_> = self
            .entries
            .iter()
            .map(|e| {
                (
                    e.target.to_lowercase(),
                    e.scan_type.to_lowercase(),
                    e.summary.to_lowercase(),
                    e.details
                        .iter()
                        .map(|d| d.to_lowercase())
                        .collect::<Vec<String>>(),
                )
            })
            .collect();

        self.entries
            .iter()
            .zip(entry_lowers.iter())
            .filter(
                |(_, (target_lower, scan_type_lower, summary_lower, details_lower))| {
                    target_lower.contains(&query_lower)
                        || scan_type_lower.contains(&query_lower)
                        || summary_lower.contains(&query_lower)
                        || details_lower.iter().any(|d| d.contains(&query_lower))
                },
            )
            .map(|(e, _)| e)
            .collect()
    }

    pub fn get_scan_types(&self) -> Vec<String> {
        let mut types: Vec<String> = self.entries.iter().map(|e| e.scan_type.clone()).collect();
        types.sort();
        types.dedup();
        types
    }

    pub fn update_results_view(&mut self) {
        self.results_view.clear();

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
            self.results_view.add_line(Line::from(vec![
                Span::styled("Type: ", Style::default().fg(tc!(accent))),
                Span::raw(scan_type),
            ]));
            self.results_view.add_line(Line::from(vec![
                Span::styled("Time: ", Style::default().fg(tc!(accent))),
                Span::raw(timestamp),
            ]));
            self.results_view.add_line(Line::from(vec![
                Span::styled("Target: ", Style::default().fg(tc!(accent))),
                Span::raw(target),
            ]));
            self.results_view.add_line(Line::from(vec![
                Span::styled("Summary: ", Style::default().fg(tc!(accent))),
                Span::raw(summary),
            ]));
            self.results_view.add_line(Line::from(""));

            if !details.is_empty() {
                self.results_view.add_line(Line::from(Span::styled(
                    "Details:",
                    Style::default().fg(tc!(info)),
                )));
                for detail in &details {
                    self.results_view.add_text(detail, None);
                }
            }
        }
    }

    pub fn scroll_details_up(&mut self) {
        self.results_view.scroll_up(1);
    }

    pub fn scroll_details_down(&mut self) {
        self.results_view.scroll_down(1);
    }
}

impl Default for HistoryTab {
    fn default() -> Self {
        Self::new()
    }
}

impl HistoryTab {
    pub fn stop(&mut self) {}
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
        self.error = None;
        self.next_id = 1;
        self.visible_rows = 20;
        self.focus_area = HistoryFocusArea::List;
    }

    fn set_error(&mut self, error: TabError) {
        self.error = Some(error);
    }
}

impl TabRender for HistoryTab {
    fn render(&self, f: &mut Frame, area: Rect, _insert_mode: bool) {
        if let Some(ref err) = self.error {
            let error_text = Paragraph::new(format!("Error: {}", err.message()))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("History - Error"),
                )
                .style(Style::default().fg(tc!(error)));
            f.render_widget(error_text, area);
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        let list_area = chunks.first().copied().unwrap_or(area);
        let details_area = chunks.get(1).copied().unwrap_or(area);

        if self.entries.is_empty() {
            let empty =
                Paragraph::new("No history entries yet.\n\nRun a scan to see results here.")
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("History")
                            .border_style(Style::default().fg(
                                if self.focus_area == HistoryFocusArea::List {
                                    tc!(border_focused)
                                } else {
                                    tc!(border)
                                },
                            )),
                    )
                    .style(Style::default().fg(tc!(text_dim)));
            f.render_widget(empty, list_area);

            let placeholder = Paragraph::new("Select an entry to view details")
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Details")
                        .border_style(Style::default().fg(
                            if self.focus_area == HistoryFocusArea::Details {
                                tc!(border_focused)
                            } else {
                                tc!(border)
                            },
                        )),
                )
                .style(Style::default().fg(tc!(text_dim)));
            f.render_widget(placeholder, details_area);
            return;
        }

        let mut list_lines = vec![
            Line::from(vec![
                Span::styled(format!("{:<20}", "TIME"), Style::default().fg(tc!(accent))),
                Span::styled(format!("{:<10}", "TYPE"), Style::default().fg(tc!(accent))),
                Span::styled("TARGET", Style::default().fg(tc!(accent))),
            ]),
            Line::from(Span::styled(
                "─".repeat(60),
                Style::default().fg(tc!(text_dim)),
            )),
        ];

        let visible_rows = self.calc_visible_rows(list_area);

        for (display_idx, entry) in self
            .entries
            .iter()
            .enumerate()
            .skip(self.scroll_offset)
            .take(visible_rows)
        {
            let real_idx = self.scroll_offset + display_idx;
            let is_selected = Some(real_idx) == self.selected;
            let style = if is_selected {
                Style::default().fg(tc!(selected_text)).bg(tc!(selected))
            } else {
                Style::default()
            };

            let time_short = entry.timestamp.get(5..16).unwrap_or(&entry.timestamp);

            let target_display = if entry.target.len() > 30 {
                let truncate_pos = entry
                    .target
                    .char_indices()
                    .take_while(|(i, _)| *i < 27)
                    .last()
                    .map(|(i, c)| i + c.len_utf8())
                    .unwrap_or(27);
                format!("{}...", &entry.target[..truncate_pos])
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

        let list = Paragraph::new(list_lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title("History")
                .border_style(
                    Style::default().fg(if self.focus_area == HistoryFocusArea::List {
                        tc!(border_focused)
                    } else {
                        tc!(border)
                    }),
                ),
        );
        f.render_widget(list, list_area);

        if !self.results_view.is_empty() {
            self.results_view.render(f, details_area, None);
        } else {
            let placeholder = Paragraph::new("Select an entry to view details")
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Details")
                        .border_style(Style::default().fg(
                            if self.focus_area == HistoryFocusArea::Details {
                                tc!(border_focused)
                            } else {
                                tc!(border)
                            },
                        )),
                )
                .style(Style::default().fg(tc!(text_dim)));
            f.render_widget(placeholder, details_area);
        }
    }
}

impl TabInput for HistoryTab {
    fn handle_focus_next(&mut self) {
        if self.is_running() {
            return;
        }
        self.focus_area = match self.focus_area {
            HistoryFocusArea::List => HistoryFocusArea::Details,
            HistoryFocusArea::Details => HistoryFocusArea::List,
        };
    }
    fn handle_focus_prev(&mut self) {
        if self.is_running() {
            return;
        }
        self.focus_area = match self.focus_area {
            HistoryFocusArea::List => HistoryFocusArea::Details,
            HistoryFocusArea::Details => HistoryFocusArea::List,
        };
    }

    fn handle_char(&mut self, c: char) {
        if self.is_running() {
            return;
        }
        match c {
            'd' => self.delete_selected(),
            'C' => self.clear_all(),
            _ => {}
        }
    }
    fn handle_backspace(&mut self) {
        if self.is_running() {
            return;
        }
    }

    fn handle_paste(&mut self, _text: &str) {
        if self.is_running() {
            return;
        }
    }

    fn handle_copy(&mut self) -> Option<String> {
        if self.is_running() {
            return None;
        }
        if self.focus_area == HistoryFocusArea::Details {
            Some(self.results_view.get_content())
        } else if let Some(idx) = self.selected {
            if let Some(entry) = self.entries.get(idx) {
                return Some(format!(
                    "ID: {}\nTimestamp: {}\nType: {}\nTarget: {}\nSummary: {}\nDetails:\n{}",
                    entry.id,
                    entry.timestamp,
                    entry.scan_type,
                    entry.target,
                    entry.summary,
                    entry.details.join("\n")
                ));
            }
            None
        } else {
            None
        }
    }

    fn handle_word_forward(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == HistoryFocusArea::Details {
            self.results_view.scroll_right(5);
        }
    }

    fn handle_word_backward(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == HistoryFocusArea::Details {
            self.results_view.scroll_left(5);
        }
    }

    fn handle_home(&mut self) {
        if self.is_running() {
            return;
        }
        match self.focus_area {
            HistoryFocusArea::List => {
                if !self.entries.is_empty() {
                    self.selected = Some(0);
                    self.scroll_offset = 0;
                    self.update_results_view();
                }
            }
            HistoryFocusArea::Details => self.results_view.scroll_to_top(),
        }
    }

    fn handle_end(&mut self) {
        if self.is_running() {
            return;
        }
        match self.focus_area {
            HistoryFocusArea::List => {
                if !self.entries.is_empty() {
                    let last = self.entries.len() - 1;
                    self.selected = Some(last);
                    self.scroll_offset = if last >= self.visible_rows.saturating_sub(1) {
                        last.saturating_sub(self.visible_rows.saturating_sub(1))
                    } else {
                        0
                    };
                    self.update_results_view();
                }
            }
            HistoryFocusArea::Details => self.results_view.scroll_to_bottom(),
        }
    }

    fn handle_top(&mut self) {
        if self.is_running() {
            return;
        }
        self.handle_home();
    }

    fn handle_bottom(&mut self) {
        if self.is_running() {
            return;
        }
        self.handle_end();
    }

    fn handle_enter(&mut self) {
        if self.is_running() {
            return;
        }
    }

    fn handle_escape(&mut self) {
        if self.is_running() {
            self.stop();
            return;
        }
        self.focus_area = HistoryFocusArea::List;
    }

    fn handle_up(&mut self) {
        if self.is_running() {
            return;
        }
        match self.focus_area {
            HistoryFocusArea::List => self.select_prev(),
            HistoryFocusArea::Details => self.results_view.scroll_up(1),
        }
    }

    fn handle_down(&mut self) {
        if self.is_running() {
            return;
        }
        match self.focus_area {
            HistoryFocusArea::List => self.select_next(),
            HistoryFocusArea::Details => self.results_view.scroll_down(1),
        }
    }

    fn handle_left(&mut self) -> bool {
        if self.is_running() {
            return false;
        }
        if self.focus_area == HistoryFocusArea::Details {
            self.results_view.scroll_left(5);
            true
        } else {
            false
        }
    }

    fn handle_right(&mut self) -> bool {
        if self.is_running() {
            return false;
        }
        if self.focus_area == HistoryFocusArea::Details {
            self.results_view.scroll_right(5);
            true
        } else {
            false
        }
    }

    fn is_at_left_edge(&self) -> bool {
        if self.focus_area == HistoryFocusArea::Details {
            if self.results_view.is_empty() {
                return true;
            }
            self.results_view.is_at_left_edge()
        } else {
            true
        }
    }

    fn is_at_right_edge(&self) -> bool {
        if self.focus_area == HistoryFocusArea::Details {
            if self.results_view.is_empty() {
                return true;
            }
            self.results_view.is_at_right_edge()
        } else {
            true
        }
    }

    fn is_input_focused(&self) -> bool {
        self.focus_area == HistoryFocusArea::Details
    }

    fn page_up(&mut self, page_size: usize) {
        if self.is_running() {
            return;
        }
        match self.focus_area {
            HistoryFocusArea::List => {
                self.scroll_offset = self.scroll_offset.saturating_sub(page_size);
            }
            HistoryFocusArea::Details => self.results_view.page_up(page_size),
        }
    }

    fn page_down(&mut self, page_size: usize) {
        if self.is_running() {
            return;
        }
        match self.focus_area {
            HistoryFocusArea::List => {
                let max_offset = self.entries.len().saturating_sub(self.visible_rows);
                self.scroll_offset = (self.scroll_offset + page_size).min(max_offset);
            }
            HistoryFocusArea::Details => self.results_view.page_down(page_size),
        }
    }
}
