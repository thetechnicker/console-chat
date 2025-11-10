use crate::network::messages;
use crate::widgets::{Widget, get_inverse};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    layout::{Constraint, Layout},
    style::{Color, Style},
    widgets::{Block, BorderType, Paragraph, Widget as ratatuiWidget, Wrap},
};
use std::str::FromStr;

const DEFAULT_USER: (&str, Color) = ("System", Color::Gray);
const BROKEN_COLOR: Color = Color::Red;
const MIN_WIDTH_P: usize = 30;

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
        let mut current_height = 0;
        let mut messages_with_size = Vec::new();
        for msg in self.messages.iter().rev() {
            let len = msg.base.text.len();
            let width = (area.width as usize - 2)
                .min(len)
                .max((area.width as usize * MIN_WIDTH_P) / 100);
            let height = (len as f32 / width as f32).ceil() as usize + 2;
            if current_height + height > area.height as usize {
                break;
            }
            current_height += height;
            messages_with_size.push((msg, width as u16 + 2, height as u16));
        }

        let mut big = area;
        for (msg, width, height) in messages_with_size.iter().rev() {
            let width = *width;
            let height = *height;
            let [a, b] =
                Layout::vertical([Constraint::Max(height), Constraint::Fill(1)]).areas(big);
            big = b;
            let [msg_a, _] =
                Layout::horizontal([Constraint::Max(width), Constraint::Fill(1)]).areas(a);
            let text = msg.base.text.clone();
            let (user, color) = msg.user.as_ref().map_or(DEFAULT_USER, |u| {
                (
                    &u.display_name,
                    u.color.as_ref().map_or(BROKEN_COLOR, |c| {
                        Color::from_str(&c).unwrap_or(BROKEN_COLOR)
                    }),
                )
            });
            buf.set_style(a, Style::new().bg(color).fg(get_inverse(color)));
            let msg_paragraf = Paragraph::new(text)
                .wrap(Wrap { trim: false })
                .block(
                    Block::bordered()
                        .border_type(BorderType::Rounded)
                        .title(user),
                )
               // .style(Style::new().bg(color))
                ;
            msg_paragraf.render(msg_a, buf);
        }
    }
}
