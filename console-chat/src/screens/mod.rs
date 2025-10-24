use crate::event::AppEvent;
use ratatui::{buffer::Buffer, layout::Rect};
use std::fmt::Debug;

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub enum CurrentScreen {
    #[default]
    Login,
    Chat,
    Home,
}

#[derive(Default, Debug)]
pub struct CursorPos {
    pub x: u16,
    pub y: u16,
}

pub trait Screen: Debug //where
//    for<'a> &'a Self: Widget,
{
    fn handle_event(&mut self, event: AppEvent);
    fn draw(&self, area: Rect, buf: &mut Buffer) -> Option<CursorPos>;
    fn get_data(&self) -> serde_json::Value {
        serde_json::Value::Null
    }
}

pub mod chat;
pub use chat::*;
pub mod login;
pub use login::*;
pub mod home;
pub use home::*;
