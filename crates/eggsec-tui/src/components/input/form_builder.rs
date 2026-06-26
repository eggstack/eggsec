use super::input_field::InputField;
use crate::components::selector::{Checkbox, DropdownInfo, RadioGroup, Selector};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Borders},
    Frame,
};

#[derive(Clone, Debug)]
pub enum FieldVariant {
    Input(InputField),
    Checkbox(Checkbox),
    Selector(Selector),
    RadioGroup(RadioGroup),
}

pub struct FormBuilder {
    title: String,
    fields: Vec<FieldVariant>,
    row_height: u16,
}

impl FormBuilder {
    pub fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
            fields: Vec::new(),
            row_height: 3,
        }
    }

    pub fn add_input(mut self, field: InputField) -> Self {
        self.fields.push(FieldVariant::Input(field));
        self
    }

    pub fn add_checkbox(mut self, cb: Checkbox) -> Self {
        self.fields.push(FieldVariant::Checkbox(cb));
        self
    }

    pub fn add_selector(mut self, sel: Selector) -> Self {
        self.fields.push(FieldVariant::Selector(sel));
        self
    }

    pub fn add_radio(mut self, rg: RadioGroup) -> Self {
        self.fields.push(FieldVariant::RadioGroup(rg));
        self
    }

    pub fn row_height(mut self, height: u16) -> Self {
        self.row_height = height;
        self
    }

    fn calculate_constraints(&self) -> Vec<Constraint> {
        self.fields
            .iter()
            .map(|field| match field {
                FieldVariant::Input(_) => Constraint::Length(self.row_height),
                FieldVariant::Checkbox(_) => Constraint::Length(2),
                FieldVariant::Selector(_) => Constraint::Length(3),
                FieldVariant::RadioGroup(_) => Constraint::Length(2),
            })
            .collect()
    }

    pub fn collect_dropdowns(&self, area: Rect, viewport_height: u16) -> Vec<DropdownInfo> {
        let theme = crate::theme::legacy::current_theme();
        self.collect_dropdowns_with_theme(area, viewport_height, &theme)
    }

    pub fn collect_dropdowns_with_theme(
        &self,
        area: Rect,
        viewport_height: u16,
        theme: &crate::theme::Theme,
    ) -> Vec<DropdownInfo> {
        self.fields
            .iter()
            .enumerate()
            .filter_map(|(i, field)| {
                if let FieldVariant::Selector(sel) = field {
                    let constraints = self.calculate_constraints();
                    let block = Block::default()
                        .title(self.title.as_str())
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(theme.colors.border));
                    let inner = block.inner(area);
                    let mut anchor_y = inner.y;
                    for j in 0..i {
                        let h = match constraints.get(j) {
                            Some(Constraint::Length(h)) => *h,
                            _ => 3,
                        };
                        anchor_y = anchor_y.saturating_add(h);
                    }
                    let chunk_height = match constraints.get(i) {
                        Some(Constraint::Length(h)) => *h,
                        _ => 3,
                    };
                    let anchor = Rect {
                        x: inner.x,
                        y: anchor_y,
                        width: inner.width,
                        height: chunk_height,
                    };
                    sel.dropdown_info(anchor, viewport_height)
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn render(&self, f: &mut Frame, area: Rect, insert_mode: bool) {
        let theme = crate::theme::legacy::current_theme();
        self.render_with_theme(f, area, insert_mode, &theme);
    }

    pub fn render_with_theme(
        &self,
        f: &mut Frame,
        area: Rect,
        insert_mode: bool,
        theme: &crate::theme::Theme,
    ) {
        let block = Block::default()
            .title(self.title.as_str())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.colors.border));

        let inner = block.inner(area);
        f.render_widget(block, area);

        let constraints = self.calculate_constraints();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(inner);

        for (i, field) in self.fields.iter().enumerate() {
            if let Some(chunk) = chunks.get(i) {
                match field {
                    FieldVariant::Input(input) => {
                        input.render_with_theme(f, *chunk, insert_mode, theme)
                    }
                    FieldVariant::Checkbox(cb) => {
                        cb.render_with_theme(cb.focused, f, *chunk, theme)
                    }
                    FieldVariant::Selector(sel) => sel.render_with_theme(f, *chunk, theme),
                    FieldVariant::RadioGroup(rg) => rg.render_with_theme(f, *chunk, theme),
                }
            }
        }
    }
}
