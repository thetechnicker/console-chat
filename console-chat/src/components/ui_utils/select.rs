use crate::action::ActionSubsetWrapper;
use crate::action::Result;
use crate::components::EventWidget;
use crate::components::theme::ViModePalettes;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use ratatui::widgets::{Block, BorderType, Paragraph};
use strum::FromRepr;

#[derive(Debug, Default, Clone, Copy, FromRepr, PartialEq)]
pub enum SelectState {
    #[default]
    Normal,
    Selecting(usize),
    Selected(usize),
}

#[derive(Debug)]
pub struct SelectWidget {
    title: String,
    options: Box<[&'static str]>,
    state: SelectState,
    _theme: ViModePalettes,
}

impl SelectWidget {
    pub fn new(
        title: impl Into<String>,
        options: impl Into<Box<[&'static str]>>,
        theme: ViModePalettes,
    ) -> Self {
        Self {
            title: title.into(),
            options: options.into(),
            state: SelectState::Normal,
            _theme: theme,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Enter => match self.state {
                SelectState::Normal => self.state = SelectState::Selecting(0),
                SelectState::Selected(i) => self.state = SelectState::Selecting(i),
                SelectState::Selecting(i) => self.state = SelectState::Selected(i),
            },
            KeyCode::Char('j') => match self.state {
                SelectState::Selected(_) | SelectState::Normal => return false,
                SelectState::Selecting(i) => self.state = SelectState::Selecting(i + 1),
            },
            KeyCode::Char('k') => match self.state {
                SelectState::Selected(_) | SelectState::Normal => return false,
                SelectState::Selecting(i) => self.state = SelectState::Selecting(i - 1),
            },
            _ => return false,
        }
        true
    }

    fn render_normal(&self, area: Rect, buf: &mut Buffer) {
        let center = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(3),
            Constraint::Fill(1),
        ])
        .split(area)[1];
        let title = Line::from(self.title.clone()).left_aligned().bold();
        Paragraph::new(title)
            .block(Block::bordered())
            .render(center, buf);
    }
    fn render_selected(&self, area: Rect, buf: &mut Buffer, i: usize) {
        let center = Layout::vertical([Constraint::Length(3), Constraint::Fill(1)]).split(area)[1];
        let title = Line::from(self.title.clone()).left_aligned().bold();
        let value = Line::from(self.options[i]).left_aligned().bold();
        Paragraph::new(value)
            .block(Block::bordered().title_top(title))
            .render(center, buf);
    }
    fn render_selecting(&self, area: Rect, buf: &mut Buffer, selected: usize) {
        let [top, overflow] =
            Layout::vertical([Constraint::Length(3), Constraint::Fill(1)]).areas(area);
        let title = Line::from(self.title.clone()).left_aligned().bold();
        let len = (overflow.height - overflow.y) as usize;
        let start = 0.max(selected - (len / 2));
        let end = self.options.len().min(start + len as usize);

        let values: Vec<_> = (&self.options[start..end])
            .iter()
            .enumerate()
            .map(|(i, str)| {
                if i == selected {
                    Line::from(str.to_owned()).yellow().reversed()
                } else {
                    Line::from(str.to_owned())
                }
            })
            .collect();
        let (first, other) = values.split_at(1);

        Paragraph::new(first[0].clone())
            .block(
                Block::bordered()
                    .border_type(BorderType::Thick)
                    .title_top(title),
            )
            .render(top, buf);

        let text = Text::from(other.to_vec());
        Paragraph::new(text)
            .block(Block::bordered().border_type(BorderType::Plain))
            .render(overflow, buf);
    }
}

impl Widget for &SelectWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        //let area = Rect::new(area.x, area.y, area.width, area.height.min(6));
        match self.state {
            SelectState::Normal => self.render_normal(area, buf),
            SelectState::Selected(i) => self.render_selected(area, buf, i),
            SelectState::Selecting(i) => self.render_selecting(area, buf, i),
        }
    }
}
impl EventWidget for SelectWidget {
    fn handle_event(&mut self, key: KeyEvent) -> Result<Option<ActionSubsetWrapper>> {
        let _ = self.handle_key(key);
        Ok(None)
    }
    fn draw(&self, area: Rect, buf: &mut Buffer) {
        self.render(area, buf)
    }

    fn select(&mut self) {}
    fn deselect(&mut self) {}
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_creation() {
        let _select_widget =
            SelectWidget::new("test_select", ["test", "option"], ViModePalettes::default());
    }
}
