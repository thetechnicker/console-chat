use crate::event::AppEvent;
use crate::network::messages;
use crate::widgets::Widget;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    layout::{Constraint, Layout},
    style::{Color, Style},
    //   symbols,
    //  text::{Line, Span},
    widgets::{
        Block,
        BorderType,
        //Borders,
        //HighlightSpacing,
        //List,
        //ListItem,
        //ListState,
        Paragraph,
        Widget as ratatuiWidget,
        //StatefulWidget,
    },
};
use std::str::FromStr;
//use std::cell::RefCell;

//const HEADER_STYLE: Style = Style::new().fg(SLATE.c100).bg(BLUE.c800);
//const NORMAL_ROW_BG: Color = SLATE.c950;
//const ALT_ROW_BG_COLOR: Color = SLATE.c900;
//const SELECTED_STYLE: Style = Style::new().bg(SLATE.c800).add_modifier(Modifier::BOLD);

const DEFAULT_USER: (&str, Color) = ("System", Color::Gray);
const BROKEN_COLOR: Color = Color::Red;

#[derive(Debug)]
pub struct MessageList {
    messages: Vec<messages::ServerMessage>,
    //state: RefCell<ListState>,
}

impl MessageList {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            //state: RefCell::new(ListState::default()),
        }
    }

    pub fn push(&mut self, msg: messages::ServerMessage) {
        self.messages.push(msg);
    }
}

impl Widget for MessageList {
    fn handle_event(&mut self, app_event: AppEvent) {
        match app_event {
            AppEvent::Clear(hard) => {
                if hard {
                    self.messages.clear();
                }
            }
            _ => {}
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
