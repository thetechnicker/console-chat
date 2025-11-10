use crate::DEFAULT_BORDER;
use crate::event::{AppEvent, AppEventSender};
use crate::screens::{self, CursorPos, Screen};
use crate::widgets;
use crate::widgets::Widget;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    widgets::{Block, Widget as UiWidget},
};
use serde_json;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug)]
pub struct HomeScreen {
    event_sender: AppEventSender,
    mode: screens::InputMode,
    room_input: Rc<RefCell<widgets::InputWidget>>,
    join_button: Rc<RefCell<widgets::Button>>,
    logout_button: Rc<RefCell<widgets::Button>>,
    exit_button: Rc<RefCell<widgets::Button>>,

    widget_hirarchie: screens::WidgetElement,
    buttons: screens::WidgetElement,

    x: usize,
    y: usize,
}

impl HomeScreen {
    pub fn new(event_sender: AppEventSender) -> Self {
        let room_input = Rc::new(RefCell::new(widgets::InputWidget::new("Room", "JOIN")));
        let join_button = Rc::new(RefCell::new(
            widgets::Button::new("Join Room", 'o', "JOIN").theme(widgets::GREEN),
        ));
        let logout_button = Rc::new(RefCell::new(
            widgets::Button::new("Logout", 'u', "LOGOUT").theme(widgets::BLUE),
        ));
        let exit_button = Rc::new(RefCell::new(
            widgets::Button::new("Exit", 'q', "QUIT").theme(widgets::RED),
        ));

        let buttons = screens::WidgetElement::Collection(Rc::new([
            screens::WidgetElement::Item(join_button.clone()),
            screens::WidgetElement::Item(logout_button.clone()),
            screens::WidgetElement::Item(exit_button.clone()),
        ]));

        let widget_hirarchie = screens::WidgetElement::Collection(Rc::new([
            screens::WidgetElement::Item(room_input.clone()),
            screens::WidgetElement::Collection(Rc::new([
                screens::WidgetElement::Item(join_button.clone()),
                screens::WidgetElement::Item(logout_button.clone()),
            ])),
            screens::WidgetElement::Item(exit_button.clone()),
        ]));

        Self {
            mode: screens::InputMode::default(),
            event_sender,
            x: 0,
            y: 0,
            room_input,
            logout_button,
            join_button,
            exit_button,
            widget_hirarchie,
            buttons,
        }
    }
}

impl Screen for HomeScreen {
    fn get_data(&self) -> serde_json::Value {
        serde_json::json!(self.room_input.borrow().get_content())
    }

    fn set_mode(&mut self, mode: screens::InputMode) {
        self.mode = mode;
    }

    fn get_mode(&self) -> screens::InputMode {
        self.mode
    }

    fn get_widget_hirarchie(&self) -> screens::WidgetElement {
        self.widget_hirarchie.clone()
    }

    fn get_buttons(&self) -> Option<screens::WidgetElement> {
        Some(self.buttons.clone())
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

    fn handle_widget_event(&mut self, command: String, _: Option<String>) {
        match command.to_uppercase().as_str() {
            "QUIT" => self.event_sender.send(AppEvent::Quit),
            "JOIN" | "LOGOUT" => self
                .event_sender
                .send(AppEvent::OnWidgetEnter(command.clone(), None)),
            _ => {}
        }
    }
    fn draw(&self, area: Rect, buf: &mut Buffer) -> Option<CursorPos> {
        // MAIN
        let login_block = Block::bordered().border_type(DEFAULT_BORDER);
        let login_inner = login_block.inner(area);

        login_block.render(area, buf);
        let [_, input_area, _] = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Percentage(70),
            Constraint::Fill(1),
        ])
        .areas(login_inner);

        // Input
        let [_, user_input, _, buttons1, buttons2, _] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Max(3),
            Constraint::Max(3),
            Constraint::Max(3),
            Constraint::Max(3),
            Constraint::Fill(1),
        ])
        .areas(input_area);

        // User Input
        let mut u_x: Option<u16> = None;
        self.room_input.borrow().draw(user_input, buf, &mut u_x);

        // Buttons
        let x = 50;
        let [join_area, logout_area] =
            Layout::horizontal([Constraint::Percentage(x), Constraint::Percentage(x)])
                .areas(buttons1);
        self.join_button.borrow().draw(join_area, buf, &mut None);
        self.logout_button
            .borrow()
            .draw(logout_area, buf, &mut None);
        self.exit_button.borrow().draw(buttons2, buf, &mut None);

        return if self.mode == screens::InputMode::Editing {
            if let Some(x) = u_x {
                Some(CursorPos {
                    x: x + user_input.x,
                    y: user_input.y + 1_u16,
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
    use super::HomeScreen;
    use crate::event::test_utils::dummy_event_sender;
    use insta::assert_snapshot;
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn test_render_home() {
        let chat_screen = HomeScreen::new(dummy_event_sender().0.into());
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
