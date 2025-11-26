use crate::utils;
use crate::widgets::{self, Widget};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
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
    on_enter_id: Option<String>,
    clear_on_enter: bool,
    use_clipboard: bool,
}

impl InputWidget {
    pub fn new(titel: &str, on_enter: &str) -> Self {
        Self {
            titel: String::from(titel),
            input_type: InputType::Text,
            input_mode: InputMode::default(),
            input: Input::default(),
            on_enter_id: Some(on_enter.to_uppercase().to_owned()),
            clear_on_enter: false,
            use_clipboard: false,
        }
    }
    pub fn with_clippboard(mut self, clipboard: bool) -> Self {
        self.use_clipboard = clipboard;
        self
    }

    pub fn clear_on_enter(mut self, clear: bool) -> Self {
        self.clear_on_enter = clear;
        self
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
    fn clear(&mut self, hard: bool) {
        self.stop_editing();
        if hard {
            self.input.reset();
        }
    }

    fn focus(&mut self) {
        self.start_editing();
    }
    fn unfocus(&mut self) {
        self.stop_editing();
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) -> Option<widgets::WidgetEvent> {
        if KeyCode::Enter == key_event.code && key_event.is_press() {
            if let Some(event_id) = self.on_enter_id.as_ref() {
                let content = self.get_content().clone();
                if self.clear_on_enter {
                    self.input.reset();
                }
                return Some(widgets::WidgetEvent::Input((
                    event_id.to_string(),
                    Some(content),
                )));
            }
        }
        if self.use_clipboard
            && KeyCode::Char('v') == key_event.code
            && key_event.is_press()
            && key_event.modifiers.contains(KeyModifiers::CONTROL)
        {
            if let Some(content) = utils::get_clipboard_content() {
                let mut value = self.input.value().to_string();
                let cursor = self.input.cursor();
                value.insert_str(cursor, &content);
                self.input = Input::new(value).with_cursor(cursor);
                return None;
            }
        }
        self.input.handle_event(&Event::Key(key_event));
        None
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
