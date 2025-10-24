use crate::DEFAULT_BORDER;
use crate::event::{AppEvent, AppEventSender};
use crate::network::{self, user};
use crate::screens::{CurrentScreen, CursorPos, Screen};
use crate::widgets;
use crate::widgets::Widget;
use crossterm::event::{KeyCode, KeyEventKind};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{
        Color, Modifier, Style, Stylize,
        palette::tailwind::{BLUE, /*GREEN,*/ SLATE},
    },
    symbols,
    text::{Line, Span},
    widgets::{
        Block, Borders, HighlightSpacing, List, ListItem, ListState, StatefulWidget,
        Widget as UiWidget,
    },
};

use std::cell::RefCell;

const HEADER_STYLE: Style = Style::new().fg(SLATE.c100).bg(BLUE.c800);
const NORMAL_ROW_BG: Color = SLATE.c950;
const ALT_ROW_BG_COLOR: Color = SLATE.c900;
const SELECTED_STYLE: Style = Style::new().bg(SLATE.c800).add_modifier(Modifier::BOLD);

//const TEXT_FG_COLOR: Color = SLATE.c200;
//const SELF_TEXT_FG_COLOR: Color = GREEN.c500;

#[derive(Debug)]
pub struct ChatScreen {
    pub tab_index: usize,
    pub max_tab: usize,
    pub event_sender: AppEventSender,
    pub input: widgets::InputWidget,
    pub items: Vec<user::ServerMessage>,
    pub state: RefCell<ListState>,
}

impl ChatScreen {
    pub fn new(event_sender: AppEventSender) -> Self {
        Self {
            tab_index: 0,
            max_tab: 2,
            event_sender,
            input: widgets::InputWidget::default(),
            items: Vec::new(),
            state: RefCell::new(ListState::default()),
        }
    }
    pub fn send_current_widget_event(&mut self, event: AppEvent) {
        if let Some(elem) = self.current_widget() {
            elem.handle_event(event)
        }
    }
    pub fn send_all_widgets_event(&mut self, event: AppEvent) {
        for i in 0..self.max_tab {
            if let Some(elem) = self.widget_at(i) {
                elem.handle_event(event.clone());
            }
        }
    }

    pub fn widget_at(&mut self, index: usize) -> Option<&mut dyn Widget> {
        match index {
            1 => Some(&mut self.input as &mut dyn Widget),
            _ => None,
        }
    }
    pub fn current_widget(&mut self) -> Option<&mut dyn Widget> {
        match self.tab_index {
            1 => Some(&mut self.input as &mut dyn Widget),
            _ => None,
        }
    }
}

impl Screen for ChatScreen {
    fn handle_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::Clear(hard) => {
                self.tab_index = 0;
                for i in 0..self.max_tab {
                    if let Some(w) = self.widget_at(i) {
                        w.handle_event(AppEvent::Clear(hard));
                    }
                }
            }
            AppEvent::NetworkEvent(network::NetworkEvent::Message(msg)) => self.items.push(msg),
            AppEvent::KeyEvent(key_event) => {
                match key_event.code {
                    KeyCode::Tab if key_event.kind == KeyEventKind::Press => {
                        self.send_current_widget_event(AppEvent::NoFocus);
                        self.tab_index = (self.tab_index + 1) % self.max_tab;
                        self.send_current_widget_event(AppEvent::Focus);
                    }
                    KeyCode::BackTab if key_event.kind == KeyEventKind::Press => {
                        self.send_current_widget_event(AppEvent::NoFocus);
                        self.tab_index = (self.tab_index - 1) % self.max_tab;
                        self.send_current_widget_event(AppEvent::Focus);
                    }
                    KeyCode::Esc => {
                        self.send_all_widgets_event(AppEvent::NoFocus);
                        self.tab_index = 0;
                    }
                    KeyCode::Enter if self.tab_index == 1 => {
                        let input = self.input.get_content();
                        if !input.is_empty() {
                            let msg = input.clone();
                            self.event_sender
                                .send(AppEvent::SendMessage(String::from(msg.trim().to_owned())));
                            self.send_current_widget_event(AppEvent::Clear(true));
                            self.send_current_widget_event(AppEvent::Focus);
                        }
                    }
                    KeyCode::Char('q' | 'Q') if self.tab_index == 0 => {
                        self.event_sender
                            .send(AppEvent::SwitchScreen(CurrentScreen::Home));
                    }

                    _ => {}
                }
                self.send_current_widget_event(AppEvent::KeyEvent(key_event));
            }
            _ => {}
        };
    }
    /*
    }

    impl UiWidget for &ChatScreen {
        fn render(self, area: Rect, buf: &mut Buffer) {
        */
    fn draw(&self, area: Rect, buf: &mut Buffer) -> Option<CursorPos> {
        // MAIN
        let chat_block = Block::bordered().border_type(DEFAULT_BORDER);
        let chat_inner = chat_block.inner(area);
        chat_block.render(area, buf);

        let [chat, input] =
            Layout::vertical([Constraint::Min(10), Constraint::Max(3)]).areas(chat_inner);

        let block = Block::new()
            .title(Line::raw("TODO List").centered())
            .borders(Borders::TOP)
            .border_set(symbols::border::EMPTY)
            .border_style(HEADER_STYLE)
            .bg(NORMAL_ROW_BG);

        let items: Vec<ListItem> = self
            .items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let color = alternate_colors(i);
                ListItem::from(item).bg(color)
            })
            .collect();
        let list = List::new(items)
            .block(block)
            .highlight_style(SELECTED_STYLE)
            .highlight_symbol(">")
            .highlight_spacing(HighlightSpacing::Always);

        StatefulWidget::render(list, chat, buf, &mut self.state.borrow_mut());

        // Input
        self.input.draw(input, buf, &mut None);
        None
    }
}

impl From<&user::ServerMessage> for ListItem<'_> {
    fn from(item: &user::ServerMessage) -> Self {
        // Example: Format message with user display name if present and message text
        let default = String::from("System");
        let user_display = item
            .user
            .as_ref()
            .map(|u| &u.display_name)
            .unwrap_or(&default);
        let content = format!("{}: {}", user_display, item.base.text);

        // Create ListItem from single Spans line
        ListItem::new(Span::from(Span::raw(content)))
    }
}
const fn alternate_colors(i: usize) -> Color {
    if i % 2 == 0 {
        NORMAL_ROW_BG
    } else {
        ALT_ROW_BG_COLOR
    }
}
