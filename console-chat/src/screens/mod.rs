use crate::event::AppEvent;
use crate::network::NetworkEvent;
pub use crate::widgets::widget_hirarchie::*;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use ratatui::{buffer::Buffer, layout::Rect};
use std::fmt::Debug;

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub enum CurrentScreen {
    #[default]
    Login,
    Chat,
    Home,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    #[default]
    Normal,
    Editing,
    Select,
}

#[derive(Default, Debug)]
pub struct CursorPos {
    pub x: u16,
    pub y: u16,
}

pub trait Screen: Debug {
    fn handle_event(&mut self, event: AppEvent) -> bool {
        match event {
            AppEvent::KeyEvent(key_event) => {
                if let KeyCode::Esc = key_event.code {
                    self.set_mode(InputMode::Normal);
                    return true;
                }
                match self.get_mode() {
                    InputMode::Normal => self.normal_mode(key_event),
                    InputMode::Editing => self.edit_mode(key_event),
                    InputMode::Select => self.select_mode(key_event),
                }
            }
            AppEvent::NetworkEvent(network_event) => self.handle_network_event(network_event),
            AppEvent::Clear(hard) => {
                self.clear(hard);
                true
            }
            _ => false,
        }
    }

    fn clear(&mut self, hard: bool);

    fn normal_mode(&mut self, event: KeyEvent) -> bool;

    fn edit_mode(&mut self, _: KeyEvent) -> bool {
        false
    }
    fn select_mode(&mut self, _: KeyEvent) -> bool {
        false
    }
    fn handle_network_event(&mut self, _: NetworkEvent) -> bool {
        false
    }

    fn get_mode(&self) -> InputMode;
    fn set_mode(&mut self, mode: InputMode);

    fn get_data(&self) -> serde_json::Value {
        serde_json::Value::Null
    }

    fn draw(&self, area: Rect, buf: &mut Buffer) -> Option<CursorPos>;
}

pub mod login;
pub use login::*;
pub mod home;
pub use home::*;
pub mod chat;
pub use chat::*;
