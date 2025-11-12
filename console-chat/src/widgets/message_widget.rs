use crate::network::messages;
use crate::widgets::{Widget, get_inverse};
use ratatui::{
    buffer::{Buffer, Cell},
    layout::Rect,
    layout::{Constraint, Layout},
    style::{Color, Style},
    widgets::{Block, BorderType, Paragraph, Widget as ratatuiWidget, Wrap},
};
use std::cell::RefCell;
use std::str::FromStr;
//use tracing::debug;

const DEFAULT_USER: (&str, Color) = ("System", Color::Gray);
const BROKEN_COLOR: Color = Color::Red;
const MIN_WIDTH_P: usize = 30;

#[derive(Debug)]
pub struct MessageList {
    messages: Vec<messages::ServerMessage>,
    line: u16,
    len: RefCell<usize>,
}

impl MessageList {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            line: 0,
            len: RefCell::new(0),
        }
    }

    pub fn push(&mut self, msg: messages::ServerMessage) {
        self.messages.push(msg);
    }

    pub fn set_line(&mut self, line: u16) {
        self.line = line.into();
    }

    pub fn get_line(&self) -> u16 {
        self.line
    }
    pub fn get_line_mut(&mut self) -> &mut u16 {
        &mut self.line
    }
}

impl Widget for MessageList {
    fn get_len(&self) -> usize {
        self.len.borrow().clone()
    }

    fn clear(&mut self, hard: bool) {
        if hard {
            self.messages.clear();
        }
    }

    fn draw(&self, area: Rect, buf: &mut Buffer, _: &mut Option<u16>) {
        if self.messages.len() == 0 {
            return;
        }
        let mut current_height = 0;
        let mut messages_with_size = Vec::new();
        for msg in self.messages.iter() {
            let len = msg.base.text.len();
            let width = (area.width as usize - 2)
                .min(len)
                .max((area.width as usize * MIN_WIDTH_P) / 100);
            let height = (len as f32 / width as f32).ceil() as usize + 2;
            current_height += height;
            messages_with_size.push((msg, width as u16 + 2, height as u16));
        }
        *self.len.borrow_mut() = current_height;
        let total_area = Rect::new(area.x, area.y, area.width, current_height as u16);
        let mut buf2 = Buffer::empty(total_area);

        let mut big = total_area;
        for (msg, width, height) in messages_with_size.iter() {
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
            buf2.set_style(a, Style::new().bg(color).fg(get_inverse(color)));
            let msg_paragraf = Paragraph::new(text).wrap(Wrap { trim: false }).block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .title(user),
            );
            msg_paragraf.render(msg_a, &mut buf2);
        }
        //This is bad but needed
        {
            let mut x = buf2.content.clone();
            let length = area.area() as usize;
            let total_length = total_area.area() as usize;
            if length != total_length {
                if x.len() > length {
                    //let line = self.get_len() as u16 - self.line;
                    //let row = (line * area.width) as usize;
                    //debug!(
                    //    "Message Widget: {}, {}, {},{}",
                    //    total_length, length, line, row
                    //);
                    //let end = total_length - row;
                    //let start = end - length;
                    //debug!("range: {}, {}", start, end);
                    //x.drain(end..);
                    let start = total_length - length;
                    x.drain(..start);
                    //debug!("New length: {}, Wanted Length {}", x.len(), length);
                } else {
                    x.resize(length, Cell::EMPTY);
                }
                buf2.content = x;
                buf2.area = area;
            }
        }
        buf.merge(&buf2);
    }
}
