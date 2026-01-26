use crate::action::ActionSubsetWrapper;
use crate::action::ButtonEvent;
use crate::action::DialogEvent;
use crate::action::SelectionEvent;
use crate::action::VimEvent;
use crate::components::ui_utils::EventWidget;
use crate::components::ui_utils::button::Button;
use crate::components::ui_utils::button::ButtonState;
use crate::components::ui_utils::render_nice_bg;
use crate::components::ui_utils::select::SelectWidget;
use crate::components::ui_utils::theme::Theme;
use crate::components::ui_utils::vim::VimType;
use crate::components::ui_utils::vim::VimWidget;
use crate::error::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::widgets::{Paragraph, Widget};
use std::collections::VecDeque;

#[derive(Debug)]
pub struct Dialog {
    title: String,
    inputs: VecDeque<Box<dyn EventWidget>>,
    labels: VecDeque<String>,
    theme: Theme,
    row: usize,
    button: bool,
    ok: Button,
    cancel: Button,
    size: usize,
}

impl Dialog {
    pub fn new(title: impl Into<String>, theme: Theme) -> Self {
        Self {
            title: title.into(),
            inputs: VecDeque::new(),
            labels: VecDeque::new(),
            theme,
            ok: Button::new("Ok", "", theme.buttons.accepting, ButtonEvent::Ok),
            cancel: Button::new("Cancel", "", theme.buttons.denying, ButtonEvent::Cancel),
            button: false,
            row: 0,
            // title + buttons + border
            size: 1 + 3 + 2,
        }
    }

    fn add_input_inner(&mut self, widget: Box<dyn EventWidget>, label: String, front: bool) {
        let is_first = self.inputs.is_empty();
        if front {
            self.inputs.push_front(widget);
            self.labels.push_front(label);
        } else {
            self.inputs.push_back(widget);
            self.labels.push_back(label);
        }
        if is_first {
            self.select_current_selection();
        }
        self.size += 3;
    }

    pub fn add_input(mut self, label: &str) -> Self {
        self.add_input_inner(
            Box::new(VimWidget::new(label, VimType::SingleLine, self.theme.vi)),
            label.to_string(),
            true,
        );
        self
    }

    pub fn add_password(mut self, label: &str) -> Self {
        self.add_input_inner(
            Box::new(VimWidget::new(label, VimType::SingleLine, self.theme.vi).password()),
            label.to_string(),
            true,
        );
        self
    }

    pub fn add_select<'a, I, T>(mut self, label: &str, options: I) -> Dialog
    where
        I: IntoIterator<Item = T>,
        T: std::fmt::Debug + std::fmt::Display + Clone + 'static + serde::Serialize,
    {
        let select = SelectWidget::new(label, options, self.theme.select);
        self.add_input_inner(Box::new(select), label.to_string(), false);
        self
    }

    fn get_button(&mut self) -> &mut Button {
        if self.button {
            &mut self.cancel
        } else {
            &mut self.ok
        }
    }

    fn deselect_current_selection(&mut self) {
        if self.row < self.inputs.len() {
            self.inputs[self.row].deselect();
        } else {
            self.get_button().set_state(ButtonState::Normal);
        }
    }

    fn select_current_selection(&mut self) {
        if self.row < self.inputs.len() {
            self.inputs[self.row].select();
        } else {
            self.get_button().set_state(ButtonState::Selected);
        }
    }

    fn up(&mut self) {
        self.deselect_current_selection();
        self.row = self.row.saturating_sub(1);
        self.select_current_selection();
    }

    fn down(&mut self) {
        self.deselect_current_selection();
        if self.row < self.inputs.len() {
            self.row = self.row.saturating_add(1);
        }
        self.select_current_selection();
    }

    fn lr(&mut self) {
        self.deselect_current_selection();
        self.button = !self.button;
        self.select_current_selection();
    }

    pub fn handle_event(&mut self, key: KeyEvent) -> Result<Option<DialogEvent>> {
        if self.row < self.inputs.len() {
            if let Some(event) = self.inputs[self.row].handle_event(key)? {
                match event {
                    ActionSubsetWrapper::VimEvent(vim_event) => match vim_event {
                        VimEvent::Down => self.down(),
                        VimEvent::Up => self.up(),
                        VimEvent::Enter(_) => {
                            self.down();
                            return Ok(Some(DialogEvent::Normal));
                        }
                        VimEvent::Insert => return Ok(Some(DialogEvent::Insert)),
                        VimEvent::Normal => return Ok(Some(DialogEvent::Normal)),
                        _ => {}
                    },
                    ActionSubsetWrapper::SelectionEvent(select_event) => match select_event {
                        SelectionEvent::Down => self.down(),
                        SelectionEvent::Up => self.up(),
                    },
                    _ => {}
                }
            }
        } else {
            match key.code {
                KeyCode::Esc => return Ok(Some(DialogEvent::Cancel)),
                KeyCode::Char('j') => self.down(),
                KeyCode::Char('k') => self.up(),
                KeyCode::Char('h') | KeyCode::Char('l') => self.lr(),
                KeyCode::Enter => {
                    let button = self.get_button();
                    button.set_state(ButtonState::Active);
                    if let Some(button_event) = button.trigger() {
                        match button_event {
                            ButtonEvent::Ok => return Ok(Some(DialogEvent::Ok(self.get_data()))),
                            ButtonEvent::Cancel => return Ok(Some(DialogEvent::Cancel)),
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(None)
    }

    pub fn get_data(&self) -> serde_json::Value {
        serde_json::Value::Object(serde_json::Map::from(
            self.labels
                .iter()
                .zip(self.inputs.iter())
                .map(|(label, input)| (label.to_lowercase(), input.get_content()))
                .collect(),
        ))
    }
}

impl Widget for &Dialog {
    fn render(self, area: ratatui::layout::Rect, buf: &mut ratatui::buffer::Buffer) {
        let inner_horizontal = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Percentage(40),
            Constraint::Fill(1),
        ])
        .split(area)[1];

        let inner_vertical = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(self.size as u16),
            Constraint::Fill(1),
        ])
        .split(inner_horizontal)[1];

        let inner_area = render_nice_bg(inner_vertical, self.theme.page, buf);

        // Layout for inputs and buttons
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                vec![
                    vec![Constraint::Length(1)],
                    self.inputs
                        .iter()
                        .map(|_| Constraint::Length(3))
                        .chain(Some(Constraint::Length(3)))
                        .collect::<Vec<_>>(),
                ]
                .concat(),
            )
            .split(inner_area);

        // Create a block for the buttons
        let button_area = chunks[chunks.len() - 1];

        // Render buttons
        let button_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(button_area);

        Paragraph::new(self.title.as_str())
            .centered()
            .render(chunks[0], buf);

        self.ok.draw_button(button_layout[0], buf);
        self.cancel.draw_button(button_layout[1], buf);

        // Render each input widget
        for (i, input) in self.inputs.iter().enumerate() {
            input.draw(chunks[i + 1], buf);
        }
    }
}
