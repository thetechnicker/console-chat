use crate::action::ActionSubsetWrapper;
use crate::action::Result;
use crate::action::SelectionEvent;
use crate::components::EventWidget;
use crate::components::theme::SelectPalettes;
use crate::components::ui_utils::ContentType;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Paragraph};
use strum::FromRepr;
use tracing::warn;

#[derive(Debug, Default, Clone, Copy, FromRepr, PartialEq)]
pub enum SelectState {
    #[default]
    Normal,
    Selecting(usize),
    Selected(usize),
}

#[derive(Debug)]
pub struct SelectWidget<T>
where
    T: std::fmt::Display + std::fmt::Debug + Clone,
{
    title: String,
    options: Box<[T]>,
    state: SelectState,
    active: bool,
    _theme: SelectPalettes,
}

impl<T> SelectWidget<T>
where
    T: std::fmt::Display + std::fmt::Debug + Clone,
{
    pub fn new<I>(title: impl Into<String>, options: I, theme: SelectPalettes) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        let options: Box<[T]> = options.into_iter().map(|s| s.into()).collect();

        // Ensure we have at least one option to prevent index issues
        assert!(
            !options.is_empty(),
            "SelectWidget requires at least one option"
        );

        Self {
            title: title.into(),
            options,
            state: SelectState::Normal,
            active: false,
            _theme: theme,
        }
    }

    /// Get the next index with wrapping
    fn next_index(&self, current: usize) -> usize {
        if self.options.is_empty() {
            0
        } else {
            (current + 1) % self.options.len()
        }
    }

    /// Get the previous index with wrapping (handles underflow)
    fn prev_index(&self, current: usize) -> usize {
        if self.options.is_empty() {
            0
        } else {
            (current + self.options.len() - 1) % self.options.len()
        }
    }

    /// Ensure index is within bounds
    fn clamp_index(&self, index: usize) -> usize {
        if self.options.is_empty() {
            0
        } else {
            index % self.options.len()
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Option<SelectionEvent> {
        match key.code {
            KeyCode::Enter => match self.state {
                SelectState::Normal => {
                    self.state = SelectState::Selecting(0);
                }
                SelectState::Selected(i) => {
                    let safe_index = self.clamp_index(i);
                    self.state = SelectState::Selecting(safe_index);
                }
                SelectState::Selecting(i) => {
                    let safe_index = self.clamp_index(i);
                    self.state = SelectState::Selected(safe_index);
                }
            },
            KeyCode::Char('j') => match self.state {
                SelectState::Selected(_) | SelectState::Normal => {
                    return Some(SelectionEvent::Down);
                }
                SelectState::Selecting(i) => {
                    let next = self.next_index(i);
                    self.state = SelectState::Selecting(next);
                }
            },
            KeyCode::Char('k') => match self.state {
                SelectState::Selected(_) | SelectState::Normal => {
                    return Some(SelectionEvent::Up);
                }
                SelectState::Selecting(i) => {
                    let prev = self.prev_index(i);
                    self.state = SelectState::Selecting(prev);
                }
            },
            _ => {}
        }
        None
    }

    fn render_normal(&self, area: Rect, buf: &mut Buffer) {
        let center = Layout::vertical([Constraint::Length(3), Constraint::Fill(1)]).split(area)[0];
        let title = Line::from(self.title.clone()).left_aligned().bold();
        let style = if self.active {
            Style::new().gray().reversed()
        } else {
            Style::default()
        };
        Paragraph::new(title)
            .block(Block::bordered())
            .style(style)
            .render(center, buf);
    }

    fn render_selected(&self, area: Rect, buf: &mut Buffer, i: usize) {
        let center = Layout::vertical([Constraint::Length(3), Constraint::Fill(1)]).split(area)[0];
        let title = Line::from(self.title.clone()).left_aligned().bold();

        // Clamp index to prevent out of bounds access
        let safe_index = self.clamp_index(i);
        let value = Line::from(self.options[safe_index].to_string())
            .left_aligned()
            .bold();

        Paragraph::new(value)
            .block(Block::bordered().title_top(title))
            .render(center, buf);
    }

    fn render_selecting(&self, area: Rect, buf: &mut Buffer, selected: usize) {
        // Clamp selected index at the start to ensure it's valid
        let selected = self.clamp_index(selected);
        let top = Layout::vertical([Constraint::Length(3), Constraint::Fill(1)]).split(area)[0];

        let title = Line::from(self.title.clone()).left_aligned().bold();

        let actual_len = 4; // the place of the top
        let start = selected
            .saturating_sub(actual_len / 2)
            .min(self.options.len().saturating_sub(actual_len));
        let end = self.options.len().min(start + actual_len);

        // This should never happen now due to clamping, but keep the warning
        if selected >= self.options.len() {
            warn!(
                selected = selected,
                total_options = self.options.len(),
                "Selected index out of bounds after clamping!"
            );
            return;
        }

        let style = Style::new().gray().reversed();

        let values: Vec<_> = (&self.options[start..end])
            .iter()
            .enumerate()
            .map(|(i, str)| {
                let actual_index = start + i; // Convert relative index to absolute
                let is_selected = actual_index == selected;

                if is_selected {
                    Line::from(str.to_string()).yellow().reversed()
                } else {
                    Line::from(str.to_string()).style(style)
                }
            })
            .collect();

        let max_width = values
            .iter()
            .map(|line| line.width() as u16)
            .max()
            .unwrap_or(top.width);

        if values.is_empty() {
            warn!("No values to render!");
            return;
        }

        Block::bordered()
            .title_top(title)
            .style(style)
            .render(top, buf);

        let mut area = Rect::new(top.x, top.y + 1, max_width + 2, 1);
        for (i, text) in values.iter().enumerate() {
            area = Rect::new(top.x, i as u16 + top.y + 1, max_width + 2, 1);
            buf.set_style(area, Style::reset());
            buf.set_string(
                area.x,
                area.y,
                format!(
                    "│{}{}",
                    " ".repeat(max_width as usize),
                    match i {
                        0 => " ",
                        1 => "┌",
                        _ => "│",
                    }
                ),
                text.style,
            );
            buf.set_line(area.x + 1, area.y, text, max_width);
        }
        buf.set_string(
            area.x,
            area.y + 1,
            format!("└{}┘", "─".repeat(max_width as usize)),
            style,
        );
    }
}

impl<T> Widget for &SelectWidget<T>
where
    T: std::fmt::Display + std::fmt::Debug + Clone,
{
    fn render(self, area: Rect, buf: &mut Buffer) {
        match self.state {
            SelectState::Normal => self.render_normal(area, buf),
            SelectState::Selected(i) => self.render_selected(area, buf, i),
            SelectState::Selecting(i) => self.render_selecting(area, buf, i),
        }
    }
}

impl<T> EventWidget for SelectWidget<T>
where
    T: std::fmt::Display + std::fmt::Debug + Clone,
{
    fn handle_event(&mut self, key: KeyEvent) -> Result<Option<ActionSubsetWrapper>> {
        Ok(self.handle_key(key).map(|e| e.into()))
    }

    fn draw(&self, area: Rect, buf: &mut Buffer) {
        self.render(area, buf)
    }

    fn select(&mut self) {
        self.active = true;
    }
    fn deselect(&mut self) {
        self.active = false;
    }

    fn get_content(&self) -> ContentType {
        match self.state {
            SelectState::Selected(i) | SelectState::Selecting(i) => ContentType::Index(i),
            SelectState::Normal => ContentType::None,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_creation() {
        let _select_widget =
            SelectWidget::new("test_select", ["test", "option"], SelectPalettes::default());
    }

    #[test]
    #[should_panic(expected = "SelectWidget requires at least one option")]
    fn test_empty_options_panics() {
        let empty: Vec<String> = vec![];
        let _select_widget = SelectWidget::new("test_select", empty, SelectPalettes::default());
    }

    #[test]
    fn test_index_wrapping() {
        let widget = SelectWidget::new("test", ["a", "b", "c"], SelectPalettes::default());

        // Test next wrapping
        assert_eq!(widget.next_index(0), 1);
        assert_eq!(widget.next_index(1), 2);
        assert_eq!(widget.next_index(2), 0); // wraps to beginning

        // Test prev wrapping
        assert_eq!(widget.prev_index(0), 2); // wraps to end
        assert_eq!(widget.prev_index(1), 0);
        assert_eq!(widget.prev_index(2), 1);
    }

    #[test]
    fn test_clamp_index() {
        let widget = SelectWidget::new("test", ["a", "b", "c"], SelectPalettes::default());

        assert_eq!(widget.clamp_index(0), 0);
        assert_eq!(widget.clamp_index(2), 2);
        assert_eq!(widget.clamp_index(3), 0); // 3 % 3 = 0
        assert_eq!(widget.clamp_index(5), 2); // 5 % 3 = 2
    }
}
