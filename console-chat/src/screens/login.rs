use crate::DEFAULT_BORDER;
use crate::event::AppEventSender;
use crate::screens::{self, CursorPos, Screen};
use crate::widgets;
use crate::widgets::Widget;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    widgets::{Block, Widget as UiWidget},
};
use serde_json;
use std::cell::RefCell;
use std::rc::Rc;
use tracing::debug;

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
            "Username",
            "USERNAME",
            event_sender.clone().into(),
        )));

        {
            user_input.borrow_mut().focus();
        }

        let pwd_input = Rc::new(RefCell::new(
            widgets::InputWidget::new("Password", "LOGIN", event_sender.clone().into()).password(),
        ));
        let join_button = Rc::new(RefCell::new(
            widgets::Button::new("Login", event_sender.clone(), 'i', "LOGIN").theme(widgets::BLUE),
        ));
        let join_anonym_button = Rc::new(RefCell::new(
            widgets::Button::new("Anonym", event_sender.clone(), 'a', "LOGIN_ANONYM")
                .theme(widgets::GREEN),
        ));
        let exit_button = Rc::new(RefCell::new(
            widgets::Button::new("Exit", event_sender.clone(), 'q', "QUIT").theme(widgets::RED),
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
            event_sender: event_sender.clone(),
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

impl Screen for LoginScreen {
    fn clear(&mut self, _: bool) {}

    fn get_mode(&self) -> screens::InputMode {
        self.mode
    }

    fn get_data(&self) -> serde_json::Value {
        serde_json::json!({
            "username":self.user_input.borrow().get_content(),
            "password":self.pwd_input.borrow().get_content(),
        })
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
            _ => {
                for button in self.buttons.iter() {
                    if button.borrow_mut().handle_key_event(event.clone()) {
                        return true;
                    }
                }
            }
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
    /*
    #[tokio::test]
    async fn test_tab_switch_increments_index() {
        use crate::event as crate_event;
        use crossterm::event;

        let (send, _) = dummy_event_sender();
        let mut chat_screen = LoginScreen::new(send.into());

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
    */
}
