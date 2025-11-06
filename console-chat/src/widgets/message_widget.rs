use crate::network::messages;
use crate::widgets::Widget;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    layout::{Constraint, Layout},
    style::{Color, Style},
    widgets::{Block, BorderType, Paragraph, Widget as ratatuiWidget},
};
use std::str::FromStr;

const DEFAULT_USER: (&str, Color) = ("System", Color::Gray);
const BROKEN_COLOR: Color = Color::Red;

#[derive(Debug)]
pub struct MessageList {
    messages: Vec<messages::ServerMessage>,
}

impl MessageList {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }

    pub fn push(&mut self, msg: messages::ServerMessage) {
        self.messages.push(msg);
    }
}

impl Widget for MessageList {
    fn clear(&mut self, hard: bool) {
        if hard {
            self.messages.clear();
        }
    }

    fn draw(&self, area: Rect, buf: &mut Buffer, _: &mut Option<u16>) {
        let num = (area.height as f32 / 3 as f32).floor() as u32;
        let x: Vec<Constraint> = (0..num).into_iter().map(|_| Constraint::Max(3)).collect();
        let areas = Layout::vertical(x).split(area).to_vec();

        for (area, msg) in areas.iter().zip(self.messages.iter().as_ref()) {
            let text = msg.base.text.clone();
            let (user, color) = msg.user.as_ref().map_or(DEFAULT_USER, |u| {
                (
                    &u.display_name,
                    u.color.as_ref().map_or(BROKEN_COLOR, |c| {
                        Color::from_str(&c).unwrap_or(BROKEN_COLOR)
                    }),
                )
            });
            let msg_paragraf = Paragraph::new(text)
                .block(
                    Block::bordered()
                        .border_type(BorderType::Rounded)
                        .title(user),
                )
                .style(Style::new().bg(color));
            msg_paragraf.render(*area, buf);
        }
    }
}
