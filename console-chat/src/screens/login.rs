use crate::DEFAULT_BORDER;
use crate::event::AppEvent;
use crate::event::AppEventSender;
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
pub struct LoginScreen {
    event_sender: AppEventSender,
    mode: screens::InputMode,

    user_input: Rc<RefCell<widgets::InputWidget>>,
    pwd_input: Rc<RefCell<widgets::InputWidget>>,
    join_anonym_button: Rc<RefCell<widgets::Button>>,
    join_button: Rc<RefCell<widgets::Button>>,
    exit_button: Rc<RefCell<widgets::Button>>,

    widget_hirarchie: screens::WidgetElement,
    buttons: screens::WidgetElement,

    x: usize,
    y: usize,
}

impl LoginScreen {
    pub fn new(event_sender: AppEventSender) -> Self {
        let user_input = Rc::new(RefCell::new(widgets::InputWidget::new(
            "Username", "USERNAME",
        )));

        {
            user_input.borrow_mut().focus();
        }

        let pwd_input = Rc::new(RefCell::new(
            widgets::InputWidget::new("Password", "LOGIN").password(),
        ));
        let join_button = Rc::new(RefCell::new(
            widgets::Button::new("Login", 'i', "LOGIN").theme(widgets::BLUE),
        ));
        let join_anonym_button = Rc::new(RefCell::new(
            widgets::Button::new("Anonym", 'a', "LOGIN_ANONYM").theme(widgets::GREEN),
        ));
        let exit_button = Rc::new(RefCell::new(
            widgets::Button::new("Exit", 'q', "QUIT").theme(widgets::RED),
        ));

        let buttons = screens::WidgetElement::Collection(Rc::new([
            screens::WidgetElement::Item(join_button.clone()),
            screens::WidgetElement::Item(join_anonym_button.clone()),
            screens::WidgetElement::Item(exit_button.clone()),
        ]));

        let widget_hirarchie = screens::WidgetElement::Collection(Rc::new([
            screens::WidgetElement::Item(user_input.clone()),
            screens::WidgetElement::Item(pwd_input.clone()),
            screens::WidgetElement::Collection(Rc::new([
                screens::WidgetElement::Item(join_button.clone()),
                screens::WidgetElement::Item(join_anonym_button.clone()),
            ])),
            screens::WidgetElement::Item(exit_button.clone()),
        ]));

        Self {
            x: 0,
            y: 0,
            event_sender: event_sender,
            mode: screens::InputMode::default(),
            user_input,
            pwd_input,
            join_button,
            join_anonym_button,
            exit_button,
            widget_hirarchie,
            buttons,
        }
    }
}

impl Screen for LoginScreen {
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
            "LOGIN_ANONYM" | "LOGIN" => self
                .event_sender
                .send(AppEvent::OnWidgetEnter(command.clone(), None)),
            "USERNAME" => {
                self.unfocus();
                crate::utils::increment_wrapping(&mut self.y, self.widget_hirarchie.num_rows());
                self.focus();
            }
            _ => {}
        }
    }

    fn get_mode(&self) -> screens::InputMode {
        self.mode
    }

    fn set_mode(&mut self, mode: screens::InputMode) {
        self.mode = mode;
    }

    fn get_data(&self) -> serde_json::Value {
        serde_json::json!({
            "username":self.user_input.borrow().get_content(),
            "password":self.pwd_input.borrow().get_content(),
        })
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
        let [_, user_input, pwd_input, buttons, idk, _] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Max(3),
            Constraint::Max(3),
            Constraint::Max(3),
            Constraint::Max(3),
            Constraint::Fill(1),
        ])
        .areas(input_area);

        let mut u_x: Option<u16> = None;
        let mut p_x: Option<u16> = None;

        // User Input
        self.user_input.borrow().draw(user_input, buf, &mut u_x);

        // Password Input
        self.pwd_input.borrow().draw(pwd_input, buf, &mut p_x);

        // Buttons
        let x = 50;
        let [ok_area, anonym_area] =
            Layout::horizontal([Constraint::Percentage(x), Constraint::Percentage(x)])
                .areas(buttons);
        self.join_button.borrow().draw(ok_area, buf, &mut None);
        self.join_anonym_button
            .borrow()
            .draw(anonym_area, buf, &mut None);
        self.exit_button.borrow().draw(idk, buf, &mut None);

        return if self.mode == screens::InputMode::Editing {
            if let Some(x) = u_x {
                Some(CursorPos {
                    x: x + user_input.x,
                    y: user_input.y + 1_u16,
                })
            } else {
                p_x.map(|x| CursorPos {
                    x: x + pwd_input.x,
                    y: pwd_input.y + 1_u16,
                })
            }
        } else {
            None
        };
    }
}
#[cfg(test)]
mod tests {
    use super::super::Screen;
    use super::LoginScreen;
    use crate::event::test_utils::dummy_event_sender;
    use insta::assert_snapshot;
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn test_render_login() {
        let chat_screen = LoginScreen::new(dummy_event_sender().0.into());
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
