use crate::DEFAULT_BORDER;
use crate::event::{AppEvent, AppEventSender};
use crate::network;
use crate::screens::{self, CursorPos, Screen};
use crate::widgets;
use crate::widgets::Widget;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    widgets::{Block, Widget as UiWidget},
};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

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
            widgets::InputWidget::new("Input", "SEND_MSG").clear_on_enter(),
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

    fn get_widget_hirarchie(&self) -> screens::WidgetElement {
        self.widget_hirarchie.clone()
    }

    fn get_buttons(&self) -> Option<screens::WidgetElement> {
        None
    }
    fn get_index_mut(&mut self) -> (&mut usize, &mut usize) {
        (&mut self.x, &mut self.y)
    }

    fn get_index(&self) -> (usize, usize) {
        (self.x, self.y)
    }
    fn set_index(&mut self, x: usize, y: usize) {
        self.x = x;
        self.y = y;
    }

    fn set_mode(&mut self, mode: screens::InputMode) {
        self.mode = mode;
    }

    fn get_mode(&self) -> screens::InputMode {
        self.mode
    }

    fn handle_widget_event(&mut self, command: String, content: Option<String>) {
        if let Some(content) = content {
            match command.to_uppercase().as_str() {
                "SEND_MSG" => self.event_sender.send(AppEvent::OnWidgetEnter(
                    command.clone(),
                    Some(Arc::new([content.clone()])),
                )),
                _ => {}
            }
        }
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
