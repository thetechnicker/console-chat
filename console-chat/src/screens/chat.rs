use crate::DEFAULT_BORDER;
use crate::event::AppEventSender;
use crate::network;
use crate::screens::{self, CursorPos, Screen};
use crate::widgets;
use crate::widgets::Widget;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    widgets::{Block, Widget as UiWidget},
};
use std::cell::RefCell;
use std::rc::Rc;
use tracing::debug;

#[derive(Debug)]
pub struct ChatScreen {
    mode: screens::InputMode,
    event_sender: AppEventSender,
    input: Rc<RefCell<widgets::InputWidget>>,
    msg_list: Rc<RefCell<widgets::MessageList>>,
    widget_hirarchie: screens::WidgetElement,
    x: usize,
    y: usize,
}

impl ChatScreen {
    pub fn new(event_sender: AppEventSender) -> Self {
        let input = Rc::new(RefCell::new(
            widgets::InputWidget::new("Input", "SEND_MSG", event_sender.clone()).clear_on_enter(),
        ));
        let msg_list = Rc::new(RefCell::new(widgets::MessageList::new()));
        let widget_hirarchie = screens::WidgetElement::Collection(Rc::new([
            screens::WidgetElement::Item(input.clone()),
            screens::WidgetElement::Item(msg_list.clone()),
        ]));
        Self {
            x: 0,
            y: 0,
            mode: screens::InputMode::default(),
            event_sender,
            input,
            msg_list,
            widget_hirarchie,
        }
    }
    fn focus(&self) {
        debug!(
            "Focus ({}, {}) {:#?}",
            self.x, self.y, self.widget_hirarchie
        );
        match self.widget_hirarchie.get_item(self.y, self.x) {
            None => panic!(),
            Some(item) => item.borrow_mut().focus(),
        };
    }
    fn unfocus(&self) {
        debug!(
            "UnFocus ({}, {}) {:#?}",
            self.x, self.y, self.widget_hirarchie
        );
        match self.widget_hirarchie.get_item(self.y, self.x) {
            None => panic!(),
            Some(item) => item.borrow_mut().unfocus(),
        };
    }
}

impl Screen for ChatScreen {
    fn handle_network_event(&mut self, event: network::NetworkEvent) -> bool {
        match event {
            network::NetworkEvent::Message(msg) => {
                self.msg_list.borrow_mut().push(msg);
                true
            }
            _ => false,
        }
    }

    fn clear(&mut self, hard: bool) {
        self.msg_list.borrow_mut().clear(hard);
        self.input.borrow_mut().clear(hard);
        self.mode = screens::InputMode::default();
        self.focus();
    }

    fn set_mode(&mut self, mode: screens::InputMode) {
        self.mode = mode;
    }

    fn get_mode(&self) -> screens::InputMode {
        self.mode
    }

    fn normal_mode(&mut self, event: KeyEvent) -> bool {
        match event.code {
            KeyCode::Char('h') if event.is_press() || event.is_repeat() => {
                self.unfocus();
                crate::utils::decrement_wrapping(
                    &mut self.x,
                    self.widget_hirarchie.num_col(self.y),
                );
                self.focus();
                return true;
            }
            KeyCode::Char('l') if event.is_press() || event.is_repeat() => {
                self.unfocus();
                crate::utils::increment_wrapping(
                    &mut self.x,
                    self.widget_hirarchie.num_col(self.y),
                );
                self.focus();
                return true;
            }
            KeyCode::Char('j') if event.is_press() || event.is_repeat() => {
                self.unfocus();
                crate::utils::increment_wrapping(&mut self.y, self.widget_hirarchie.num_rows());
                self.focus();
                return true;
            }
            KeyCode::Char('k') if event.is_press() || event.is_repeat() => {
                self.unfocus();
                crate::utils::decrement_wrapping(&mut self.y, self.widget_hirarchie.num_rows());
                self.focus();
                return true;
            }
            KeyCode::Char('i') if event.is_press() || event.is_repeat() => {
                self.mode = screens::InputMode::Editing;
                return true;
            }
            KeyCode::Char('i') => {
                self.event_sender.send(crate::event::AppEvent::SwitchScreen(
                    screens::CurrentScreen::Home,
                ));
                return true;
            }
            _ => {}
        }
        false
    }

    fn edit_mode(&mut self, event: KeyEvent) -> bool {
        let item = match self.widget_hirarchie.get_item(self.y, self.x) {
            None => panic!(),
            Some(item) => item,
        };
        item.borrow_mut().handle_key_event(event)
    }

    fn draw(&self, area: Rect, buf: &mut Buffer) -> Option<CursorPos> {
        // MAIN
        let chat_block = Block::bordered().border_type(DEFAULT_BORDER);
        let chat_inner = chat_block.inner(area);
        chat_block.render(area, buf);

        let [chat, input] =
            Layout::vertical([Constraint::Min(10), Constraint::Max(3)]).areas(chat_inner);

        self.msg_list.borrow().draw(chat, buf, &mut None);

        // Input
        let mut x: Option<u16> = None;
        self.input.borrow().draw(input, buf, &mut x);
        return if self.mode == screens::InputMode::Editing {
            if let Some(x) = x {
                Some(CursorPos {
                    x: x + input.x,
                    y: input.y + 1_u16,
                })
            } else {
                None
            }
        } else {
            None
        };
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
}
