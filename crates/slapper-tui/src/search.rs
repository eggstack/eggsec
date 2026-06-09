use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Paragraph, Row, Table},
    Frame,
};

use crate::tc;

use crate::components::centered_rect;
use crate::App;
use slapper::utils::preserve_all;

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub tab: String,
    pub title: String,
    pub content: String,
    pub line: usize,
}

pub struct GlobalSearch {
    pub results: Vec<SearchResult>,
    pub selected: usize,
    pub active_tab: Option<String>,
}

impl GlobalSearch {
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
            selected: 0,
            active_tab: None,
        }
    }

    pub fn search_from_strings(&mut self, query: &str, data: &[(&str, String)]) {
        self.results.clear();
        self.active_tab = None;

        if query.is_empty() {
            return;
        }

        let query_lower = query.to_lowercase();

        for (tab_name, content) in data {
            if !content.is_empty() && content.to_lowercase().contains(&query_lower) {
                self.results.push(SearchResult {
                    tab: tab_name.to_string(),
                    title: "Target".to_string(),
                    content: content.clone(),
                    line: 1,
                });
            }
        }

        if let Some(first) = self.results.first() {
            self.active_tab = Some(first.tab.clone());
        }
    }

    pub fn is_empty(&self) -> bool {
        self.results.is_empty()
    }

    pub fn len(&self) -> usize {
        self.results.len()
    }
}

impl Default for GlobalSearch {
    fn default() -> Self {
        Self::new()
    }
}

pub fn draw_search_results(f: &mut Frame, app: &App) {
    let search = match &app.search.global_search {
        Some(s) => s,
        None => return,
    };

    let area = f.area();
    let width = 80u16;
    let height = 20u16;

    let search_area = centered_rect(width, height, area);

    if search_area.width < 4 || search_area.height < 4 {
        return;
    }

    f.render_widget(
        Block::default()
            .title("Search Results")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(tc!(accent))),
        search_area,
    );

    let inner = Rect {
        x: search_area.x + 1,
        y: search_area.y + 1,
        width: search_area.width.saturating_sub(2).max(1),
        height: search_area.height.saturating_sub(2).max(1),
    };

    if search.is_empty() {
        let msg = if app.search.query.is_empty() {
            "Press Enter to search..."
        } else {
            "No results found"
        };
        f.render_widget(
            Paragraph::new(msg).style(Style::default().fg(tc!(text_dim))),
            inner,
        );
        return;
    }

    let visible_rows = (inner.height as usize).saturating_sub(1).min(search.len());
    let start = search.selected.saturating_sub(visible_rows / 2);
    let start = start.min(search.len().saturating_sub(visible_rows));
    let end = (start + visible_rows).min(search.len());

    let content_max_chars = (inner.width.saturating_sub(30) as usize).max(10);

    let rows: Vec<Row> = search.results[start..end]
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let style = if start + i == search.selected {
                Style::default()
                    .fg(tc!(warning))
                    .add_modifier(Modifier::REVERSED)
            } else {
                Style::default().fg(tc!(text))
            };
            Row::new(vec![
                Span::raw(&r.tab),
                Span::raw(&r.title),
                Span::raw(preserve_all(&r.content, content_max_chars)),
            ])
            .style(style)
        })
        .collect();

    let third_col_min = inner.width.saturating_sub(20).max(10);
    let widths = [
        Constraint::Length(10),
        Constraint::Length(10),
        Constraint::Min(third_col_min),
    ];
    let table = Table::new(rows, widths).block(Block::default().borders(Borders::NONE));

    f.render_widget(table, inner);
}
