use crate::event::AppEvent;
use crate::event::AppEventSender;
use crate::widgets::Widget;
use crossterm::event::{Event, KeyCode};
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
    event_sender: AppEventSender,
    on_enter_id: Option<String>,
    //placeholder: Option<String>,
}

/*
impl Default for InputWidget {
    fn default() -> Self {
        Self {
            titel: String::from("Input"),
            input_type: InputType::Text,
            input_mode: InputMode::default(),
            input: Input::default(),
            event_sender: None,
            on_enter_id: None,
            //       placeholder: None,
        }
    }
}
*/

impl InputWidget {
    pub fn new(titel: &str, on_enter: &str, event_sender: AppEventSender) -> Self {
        Self {
            titel: String::from(titel),
            input_type: InputType::Text,
            input_mode: InputMode::default(),
            input: Input::default(),
            on_enter_id: Some(on_enter.to_uppercase().to_owned()),
            event_sender: event_sender,
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
            AppEvent::Clear(hard) => {
                self.stop_editing();
                if hard {
                    self.input.reset();
                }
            }
            AppEvent::NoFocus => self.stop_editing(),
            AppEvent::Focus => self.start_editing(),
            AppEvent::KeyEvent(key) => match self.input_mode {
                InputMode::Normal => {}
                InputMode::Editing => {
                    if let KeyCode::Enter = key.code {
                        if let Some(event_id) = self.on_enter_id.as_ref() {
                            self.event_sender.send(AppEvent::OnWidgetEnter(
                                event_id.to_string(),
                                Some(self.get_content()),
                            ));
                        }
                    }
                    self.input.handle_event(&Event::Key(key));
                }
            },
            _ => {}
        }
    }

    fn draw(&self, area: Rect, buf: &mut Buffer, ret: &mut Option<u16>) {
        let style = match self.input_mode {
            InputMode::Normal => Style::default(),
            InputMode::Editing => Color::Yellow.into(),
        };
        let width = area.width.max(3) - 3;
        let scroll = self.input.visual_scroll(width as usize);
        let value = self.input.value();
        let [content, title] = if !value.is_empty() {
            match self.input_type {
                InputType::Password => [
                    "*".repeat(self.input.value().len()).to_string(),
                    self.titel.clone(),
                ],
                _ => [self.input.value().to_string(), self.titel.clone()],
            }
        } else {
            [self.titel.clone(), String::from("")]
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

        if self.input_mode == InputMode::Editing {
            *ret = Some((self.input.visual_cursor().max(scroll) - scroll + 1) as u16);
        }
    }
}
