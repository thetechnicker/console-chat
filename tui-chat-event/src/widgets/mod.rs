use crate::event::WidgetEvent;
//use ratatui::widgets as ratatui_widgets;
//use ratatui::crossterm::event::KeyEvent;
use std::fmt::Debug;

pub trait Widget: Debug {
    fn handle_event(&mut self, event: WidgetEvent);
}

pub mod input;
pub use input::*;
