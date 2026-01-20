use crate::action::ButtonEvent;
use crate::action::DialogEvent;
use crate::action::VimEvent;
use crate::components::ui_utils::button::Button;
use crate::components::ui_utils::button::ButtonState;
use crate::components::ui_utils::render_nice_bg;
use crate::components::ui_utils::theme::Theme;
use crate::components::ui_utils::vim::VimWidget;
use crate::error::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Widget};

#[derive(Debug)]
pub struct Dialog<'a> {
    title: String,
    inputs: Vec<VimWidget<'a>>,
    theme: Theme,
    row: usize,
    button: bool,
    ok: Button,
    cancel: Button,
}

impl Dialog<'_> {
    pub fn new(title: impl Into<String>, theme: Theme) -> Self {
        let mut this = Self {
            title: title.into(),
            inputs: Vec::new(),
            theme,
            ok: Button::new("Ok", "", theme.buttons.accepting, ButtonEvent::Ok),
            cancel: Button::new("Cancel", "", theme.buttons.denying, ButtonEvent::Cancel),
            button: false,
            row: 0,
        };
        this.select_current_selection();
        this
    }

    fn deselect_current_selection(&mut self) {
        if self.row < self.inputs.len() {
            self.inputs[self.row].deselect();
        } else {
            if self.button {
                self.cancel.set_state(ButtonState::Normal);
            } else {
                self.ok.set_state(ButtonState::Normal);
            }
        }
    }
    fn select_current_selection(&mut self) {
        if self.row < self.inputs.len() {
            self.inputs[self.row].select();
        } else {
            if self.button {
                self.cancel.set_state(ButtonState::Selected);
            } else {
                self.ok.set_state(ButtonState::Selected);
            }
        }
    }

    fn up(&mut self) {
        self.deselect_current_selection();
        if self.row < self.inputs.len() {
            self.row = self.row.saturating_add(1);
        }
        self.select_current_selection();
    }
    fn down(&mut self) {
        self.deselect_current_selection();
        self.row = self.row.saturating_sub(1);
        self.select_current_selection();
    }

    fn lr(&mut self) {
        self.deselect_current_selection();
        self.button = !self.button;
        self.select_current_selection();
    }

    pub fn handle_event(&mut self, key: KeyEvent) -> Result<Option<DialogEvent>> {
        if self.row < self.inputs.len() {
            if let Some(vim_event) = self.inputs[self.row].handle_event(key)? {
                match vim_event {
                    VimEvent::Up => self.down(),
                    VimEvent::Down => self.up(),
                    VimEvent::Enter(_) => self.up(),
                    VimEvent::Insert => return Ok(Some(DialogEvent::Insert)),
                    VimEvent::Normal => return Ok(Some(DialogEvent::Normal)),
                    _ => {}
                }
            }
        } else {
            match key.code {
                KeyCode::Char('j') => self.down(),
                KeyCode::Char('k') => self.up(),
                KeyCode::Char('h') | KeyCode::Char('l') => self.lr(),
                KeyCode::Enter => {
                    //self.active = false;
                    let button = if self.button {
                        &mut self.cancel
                    } else {
                        &mut self.ok
                    };
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

    pub fn get_data(&self) -> Vec<String> {
        self.inputs
            .iter()
            .map(|input| input.lines()[0].clone())
            .collect()
    }
}

impl<'a> Widget for &Dialog<'a> {
    fn render(self, area: ratatui::layout::Rect, buf: &mut ratatui::buffer::Buffer) {
        let inner_area = render_nice_bg(area, self.theme.page, buf);

        // Layout for inputs and buttons
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints(
                // One for each input followed by one for the buttons
                self.inputs
                    .iter()
                    .map(|_| Constraint::Min(1))
                    .chain(Some(Constraint::Min(3)))
                    .collect::<Vec<_>>(),
            )
            .split(inner_area);

        // Render each input widget
        for (i, input) in self.inputs.iter().enumerate() {
            input.render(chunks[i], buf);
        }

        // Create a block for the buttons
        let button_area = chunks.last().unwrap();

        // Render buttons
        let button_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(50), Constraint::Percentage(50)]) // Two buttons side by side
            .split(*button_area);

        self.ok.draw_button(button_layout[0], buf);
        self.cancel.draw_button(button_layout[1], buf);
    }
}
