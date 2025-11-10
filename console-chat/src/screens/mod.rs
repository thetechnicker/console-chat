use crate::event::AppEvent;
use crate::network::NetworkEvent;
use crate::widgets::WidgetEvent;
pub use crate::widgets::widget_hirarchie::*;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use ratatui::{buffer::Buffer, layout::Rect};
use std::fmt::Debug;
use tracing::debug;

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
    fn draw(&self, area: Rect, buf: &mut Buffer) -> Option<CursorPos>;
    fn get_widget_hirarchie(&self) -> WidgetElement;
    fn get_buttons(&self) -> Option<WidgetElement>;
    fn get_index(&self) -> (usize, usize);
    fn get_index_mut(&mut self) -> (&mut usize, &mut usize);
    fn set_index(&mut self, x: usize, y: usize);
    fn get_mode(&self) -> InputMode;
    fn set_mode(&mut self, mode: InputMode);
    fn handle_widget_event(&mut self, command: String, _: Option<String>);
    fn handle_network_event(&mut self, _: NetworkEvent) -> bool {
        false
    }
    fn select_mode(&mut self, _: KeyEvent) -> bool {
        false
    }

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

    fn normal_mode(&mut self, event: KeyEvent) -> bool {
        let widget_hirarchie = self.get_widget_hirarchie();
        let buttons = self.get_buttons();

        // Handling Navigation Keys
        self.unfocus();
        {
            let (x, y) = self.get_index_mut();
            match event.code {
                KeyCode::Char('h') if event.is_press() || event.is_repeat() => {
                    crate::utils::decrement_wrapping(x, widget_hirarchie.num_col(*y));
                }
                KeyCode::Char('l') if event.is_press() || event.is_repeat() => {
                    crate::utils::increment_wrapping(x, widget_hirarchie.num_col(*y));
                }
                KeyCode::Char('j') if event.is_press() || event.is_repeat() => {
                    crate::utils::increment_wrapping(y, widget_hirarchie.num_rows());
                }
                KeyCode::Char('k') if event.is_press() || event.is_repeat() => {
                    crate::utils::decrement_wrapping(y, widget_hirarchie.num_rows());
                }
                _ => {}
            }
        }
        self.focus();

        match event.code {
            KeyCode::Char('i') if event.is_press() || event.is_repeat() => {
                self.set_mode(InputMode::Editing);
                return true;
            }
            _ => {
                if let Some(buttons) = buttons {
                    for button in buttons.iter() {
                        let mut command: Option<String> = None;
                        if let Some(WidgetEvent::Button(event)) =
                            button.borrow_mut().handle_key_event(event.clone())
                        {
                            command = Some(event.clone());
                        }
                        if let Some(event) = command {
                            self.handle_widget_event(event, None);
                            return true;
                        }
                    }
                }
                return false;
            }
        }
    }

    fn edit_mode(&mut self, event: KeyEvent) -> bool {
        let (x, y) = self.get_index();
        let item = match self.get_widget_hirarchie().get_item(y, x) {
            None => panic!(),
            Some(item) => item,
        };
        let w_event = item.borrow_mut().handle_key_event(event);
        if let Some(w_event) = w_event {
            match w_event {
                WidgetEvent::Button(command) => self.handle_widget_event(command, None),
                WidgetEvent::Input((command, data)) => self.handle_widget_event(command, data),
            }
        }
        true
    }

    fn get_data(&self) -> serde_json::Value {
        serde_json::Value::Null
    }

    fn focus(&self) {
        let (x, y) = self.get_index();
        debug!("Focus ({}, {}) ", x, y);
        match self.get_widget_hirarchie().get_item(y, x) {
            None => panic!(),
            Some(item) => item.borrow_mut().focus(),
        };
    }
    fn unfocus(&self) {
        let (x, y) = self.get_index();
        debug!("UnFocus ({}, {}) ", x, y);
        match self.get_widget_hirarchie().get_item(y, x) {
            None => panic!(),
            Some(item) => item.borrow_mut().unfocus(),
        };
    }
    fn clear(&mut self, hard: bool) {
        for w in self.get_widget_hirarchie().iter() {
            w.borrow_mut().clear(hard);
        }
        self.set_mode(InputMode::Normal);
        self.focus();
    }
}

pub mod login;
pub use login::*;
pub mod home;
pub use home::*;
pub mod chat;
pub use chat::*;
/*
*/
