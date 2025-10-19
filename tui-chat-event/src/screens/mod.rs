use crate::event::WidgetEvent;
use ratatui::layout::Rect;
use std::fmt::Debug;

#[derive(Default, Debug, Clone)]
pub enum CurrentScreen {
    #[default]
    Login,
    Chat,
}

pub struct ScreenState {
    pub title: String,
    pub hint_area: Rect,
}

pub trait Screen: Debug {
    fn handle_event(&mut self, event: WidgetEvent);
    //fn render(&self, area: Rect, buf: &mut Buffer);
}

pub mod chat;
pub use chat::*;
pub mod login;
pub use login::*;
