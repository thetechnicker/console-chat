use crate::DEFAULT_BORDER;
use crate::event::{AppEvent, AppEventSender};
use crate::network;
use crate::screens::{CurrentScreen, CursorPos, Screen};
use crate::widgets;
use crate::widgets::Widget;
use crossterm::event::{KeyCode, KeyEventKind};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    widgets::{Block, Widget as UiWidget},
};

#[derive(Debug)]
pub struct ChatScreen {
    pub tab_index: usize,
    pub max_tab: usize,
    pub event_sender: AppEventSender,
    pub input: widgets::InputWidget,
    pub msg_list: widgets::MessageList,
}

impl ChatScreen {
    pub fn new(event_sender: AppEventSender) -> Self {
        Self {
            tab_index: 0,
            max_tab: 2,
            event_sender,
            input: widgets::InputWidget::default(),
            msg_list: widgets::MessageList::new(),
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
    fn handle_event(&mut self, event: AppEvent) -> bool {
        match event {
            AppEvent::Clear(hard) => {
                self.tab_index = 0;
                for i in 0..self.max_tab {
                    if let Some(w) = self.widget_at(i) {
                        w.handle_event(AppEvent::Clear(hard));
                    }
                }
            }
            AppEvent::NetworkEvent(network::NetworkEvent::Message(msg)) => self.msg_list.push(msg),
            AppEvent::KeyEvent(key_event) => {
                match key_event.code {
                    KeyCode::Tab if key_event.kind == KeyEventKind::Press => {
                        self.send_current_widget_event(AppEvent::NoFocus);
                        self.tab_index = (self.tab_index + 1) % self.max_tab;
                        self.send_current_widget_event(AppEvent::Focus);
                    }
                    KeyCode::BackTab if key_event.kind == KeyEventKind::Press => {
                        self.send_current_widget_event(AppEvent::NoFocus);
                        self.tab_index = if self.tab_index == 0 {
                            self.max_tab - 1
                        } else {
                            self.tab_index.wrapping_sub(1)
                        } % self.max_tab;
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
                                .send(AppEvent::SendMessage(msg.trim().to_owned()));
                            self.send_current_widget_event(AppEvent::Clear(true));
                            self.send_current_widget_event(AppEvent::Focus);
                        }
                    }
                    KeyCode::Char('q' | 'Q') if self.tab_index == 0 => {
                        self.msg_list.handle_event(AppEvent::Clear(true));
                        self.event_sender
                            .send(AppEvent::SwitchScreen(CurrentScreen::Home));
                        self.event_sender
                            .send(AppEvent::NetworkEvent(network::NetworkEvent::Leaf));
                    }
                    _ => {}
                }
                self.send_current_widget_event(AppEvent::KeyEvent(key_event));
            }
            _ => {
                return false;
            }
        };
        true
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

        self.msg_list.draw(chat, buf, &mut None);

        // Input
        self.input.draw(input, buf, &mut None);
        None
    }
}

#[cfg(test)]
mod tests {
    use super::super::Screen;
    use super::ChatScreen;
    use crate::event::test_utils::dummy_event_sender;
    use insta::assert_snapshot;
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn test_render_chat() {
        let chat_screen = ChatScreen::new(dummy_event_sender().0.into());
        let mut terminal = Terminal::new(TestBackend::new(80, 20)).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                let buf = frame.buffer_mut();
                chat_screen.draw(area, buf);
            })
            .unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[tokio::test]
    async fn test_tab_switch_increments_index() {
        use crate::event as crate_event;
        use crossterm::event;

        let (send, _) = dummy_event_sender();
        let mut chat_screen = ChatScreen::new(send.into());

        chat_screen.handle_event(crate_event::AppEvent::KeyEvent(event::KeyEvent::new(
            event::KeyCode::Tab,
            event::KeyModifiers::NONE,
        )));
        assert_eq!(chat_screen.tab_index, 1);

        chat_screen.handle_event(crate_event::AppEvent::KeyEvent(event::KeyEvent::new(
            event::KeyCode::Esc,
            event::KeyModifiers::NONE,
        )));
        assert_eq!(chat_screen.tab_index, 0);

        chat_screen.handle_event(crate_event::AppEvent::KeyEvent(event::KeyEvent::new(
            event::KeyCode::BackTab,
            event::KeyModifiers::NONE,
        )));
        assert_eq!(chat_screen.tab_index, chat_screen.max_tab - 1);

        chat_screen.handle_event(crate_event::AppEvent::KeyEvent(event::KeyEvent::new(
            event::KeyCode::Esc,
            event::KeyModifiers::NONE,
        )));
        assert_eq!(chat_screen.tab_index, 0);

        let magic_test_amount = 10;
        for _ in 0..magic_test_amount {
            chat_screen.handle_event(crate_event::AppEvent::KeyEvent(event::KeyEvent::new(
                event::KeyCode::Tab,
                event::KeyModifiers::NONE,
            )));
        }
        assert_eq!(
            chat_screen.tab_index,
            magic_test_amount % chat_screen.max_tab
        );

        chat_screen.handle_event(crate_event::AppEvent::KeyEvent(event::KeyEvent::new(
            event::KeyCode::Esc,
            event::KeyModifiers::NONE,
        )));
        assert_eq!(chat_screen.tab_index, 0);

        chat_screen.handle_event(crate_event::AppEvent::KeyEvent(event::KeyEvent::new(
            event::KeyCode::BackTab,
            event::KeyModifiers::NONE,
        )));
        chat_screen.handle_event(crate_event::AppEvent::KeyEvent(event::KeyEvent::new(
            event::KeyCode::BackTab,
            event::KeyModifiers::NONE,
        )));
        assert_eq!(chat_screen.tab_index, chat_screen.max_tab - 2);
    }

    #[tokio::test]
    async fn test_typing_characters_updates_input() {
        use crate::event as crate_event;
        use crossterm::event;

        let (send, _) = dummy_event_sender();
        let mut chat_screen = ChatScreen::new(send.into());

        chat_screen.handle_event(crate_event::AppEvent::KeyEvent(event::KeyEvent::new(
            event::KeyCode::Tab,
            event::KeyModifiers::NONE,
        )));

        for c in ['t', 'e', 's', 't'] {
            chat_screen.handle_event(crate_event::AppEvent::KeyEvent(event::KeyEvent::new(
                event::KeyCode::Char(c),
                event::KeyModifiers::NONE,
            )));
        }

        assert_eq!(chat_screen.input.get_content(), "test");
    }

    #[tokio::test]
    async fn test_enter_sends_message_event() {
        use crate::event as crate_event;
        use crossterm::event;

        let (send, mut resv) = dummy_event_sender();
        let mut chat_screen = ChatScreen::new(send.clone().into());

        chat_screen.handle_event(crate_event::AppEvent::KeyEvent(event::KeyEvent::new(
            event::KeyCode::Tab,
            event::KeyModifiers::NONE,
        )));

        for c in ['t', 'e', 's', 't'] {
            chat_screen.handle_event(crate_event::AppEvent::KeyEvent(event::KeyEvent::new(
                event::KeyCode::Char(c),
                event::KeyModifiers::NONE,
            )));
        }

        chat_screen.handle_event(crate_event::AppEvent::KeyEvent(event::KeyEvent::new(
            event::KeyCode::Enter,
            event::KeyModifiers::NONE,
        )));

        let res = resv.next().await;
        assert!(res.is_ok());
        let evt = res.unwrap();
        if let crate_event::Event::App(crate_event::AppEvent::SendMessage(msg)) = evt {
            assert_eq!(msg, "test");
        } else {
            panic!("expected SendMessage event");
        }
    }
}
