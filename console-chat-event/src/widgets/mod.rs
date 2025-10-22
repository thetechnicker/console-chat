use crate::event::AppEvent;
//use ratatui::widgets as ratatui_widgets;
//use ratatui::crossterm::event::KeyEvent;
use ratatui::{buffer::Buffer, layout::Rect};
use std::fmt::Debug;

pub trait Widget: Debug {
    fn handle_event(&mut self, event: AppEvent);
    fn draw(&self, area: Rect, buf: &mut Buffer, ret: &mut Option<u16>);
}

pub mod input;
pub use input::*;
pub mod button;
pub use button::*;
