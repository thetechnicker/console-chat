use crate::DEFAULT_BORDER;
use crate::event::{AppEvent, EventSender};
use crate::screens::{CursorPos, Screen};
use crate::widgets;
use crate::widgets::Widget;
use crossterm::event::{KeyCode, KeyEventKind};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    widgets::{Block, Paragraph, Widget as UiWidget},
};

#[derive(Debug)]
pub struct ChatScreen {
    pub tab_index: usize,
    pub max_tab: usize,
    pub event_sender: EventSender,
    pub input: widgets::InputWidget,
}

impl ChatScreen {
    pub fn new(event_sender: EventSender) -> Self {
        Self {
            tab_index: 0,
            max_tab: 2,
            event_sender,
            input: widgets::InputWidget::default(),
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

        let x = Paragraph::new(format!("{:?}\n{}", self.input, self.tab_index));
        x.render(chat, buf);

        // Input
        self.input.draw(input, buf, &mut None);
        None
    }
}
