use crate::event::WidgetEvent;
//use ratatui::widgets as ratatui_widgets;
//use ratatui::crossterm::event::KeyEvent;
use ratatui::{buffer::Buffer, layout::Rect};
use std::fmt::Debug;

pub trait Widget: Debug {
    fn handle_event(&mut self, event: WidgetEvent);
    fn draw(&self, area: Rect, buf: &mut Buffer);
}

pub mod input;
pub use input::*;
pub mod button;
pub use button::*;
