use crossterm::event::KeyEvent;
use ratatui::{buffer::Buffer, layout::Rect};
use std::fmt::Debug;

pub mod input;
pub use input::*;
pub mod button;
pub use button::*;
pub mod message_widget;
pub use message_widget::*;
pub mod color;
pub mod widget_hirarchie;
pub use color::*;

pub enum WidgetEvent {
    Input((String, Option<String>)),
    Button(String),
}

pub trait Widget: Debug {
    fn focus(&mut self) {}
    fn unfocus(&mut self) {}

    fn handle_key_event(&mut self, _: KeyEvent) -> Option<WidgetEvent> {
        None
    }

    fn clear(&mut self, hard: bool);

    fn draw(&self, area: Rect, buf: &mut Buffer, ret: &mut Option<u16>);

    fn into_widget(&self) -> &dyn Widget
    where
        Self: Sized,
    {
        self as &dyn Widget
    }
}
