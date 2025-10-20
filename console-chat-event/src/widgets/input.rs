use crate::event::AppEvent;
use crate::widgets::Widget;
use ratatui::crossterm::event::Event;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, BorderType, Paragraph, Widget as UiWidget},
};

use tui_input::Input;
use tui_input::backend::crossterm::EventHandler;
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    #[default]
    Normal,
    Editing,
}

#[derive(Debug)]
pub enum InputType {
    Text,
    Password,
}

#[derive(Debug)]
pub struct InputWidget {
    titel: String,
    input_type: InputType,
    input_mode: InputMode,
    input: Input,
    //placeholder: Option<String>,
}

impl Default for InputWidget {
    fn default() -> Self {
        Self {
            titel: String::from("Input"),
            input_type: InputType::Text,
            input_mode: InputMode::default(),
            input: Input::default(),
            //       placeholder: None,
        }
    }
}

impl InputWidget {
    pub fn new(titel: &str) -> Self {
        Self {
            titel: String::from(titel),
            input_type: InputType::Text,
            input_mode: InputMode::default(),
            input: Input::default(),
            //placeholder: None,
        }
    }

    pub fn password(mut self) -> Self {
        self.input_type = InputType::Password;
        self
    }

    pub fn get_content(&self) -> String {
        String::from(self.input.value())
    }

    fn start_editing(&mut self) {
        self.input_mode = InputMode::Editing
    }

    fn stop_editing(&mut self) {
        self.input_mode = InputMode::Normal
    }
}

impl Widget for InputWidget {
    fn handle_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::NoFocus => self.stop_editing(),
            AppEvent::Focus => self.start_editing(),
            AppEvent::KeyEvent(key) => match self.input_mode {
                InputMode::Normal => match key.code {
                    _ => {}
                },
                InputMode::Editing => match key.code {
                    _ => {
                        self.input.handle_event(&Event::Key(key));
                    }
                },
            },
            _ => {}
        }
    }

    fn draw(&self, area: Rect, buf: &mut Buffer) {
        let style = match self.input_mode {
            InputMode::Normal => Style::default(),
            InputMode::Editing => Color::Yellow.into(),
        };
        let width = area.width.max(3) - 3;
        let scroll = self.input.visual_scroll(width as usize);
        let value = self.input.value();
        let [content, title] = if value.len() > 0 {
            match self.input_type {
                InputType::Password => [
                    format!("{}", "*".repeat(self.input.value().len())),
                    self.titel.clone(),
                ],
                _ => [format!("{}", self.input.value()), self.titel.clone()],
            }
        } else {
            [String::from(self.titel.clone()), String::from("")]
        };

        let input_elem = Paragraph::new(content)
            .style(style)
            .scroll((0, scroll as u16))
            .block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .title(title),
            );
        input_elem.render(area, buf);
    }
}
