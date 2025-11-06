use crate::DEFAULT_BORDER;
use crate::event::{AppEvent, AppEventSender};
use crate::screens::{CursorPos, Screen};
use crate::widgets;
use crate::widgets::Widget;
use crossterm::event::{KeyCode, KeyEventKind};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    widgets::{Block, Widget as UiWidget},
};
use serde_json;

#[derive(Debug)]
pub struct LoginScreen {
    pub tab_index: usize,
    pub max_tab: usize,
    pub event_sender: AppEventSender,
    pub user_input: widgets::InputWidget,
    pub pwd_input: widgets::InputWidget,
    pub skip_button: widgets::Button,
    pub ok_button: widgets::Button,
    pub cancel_button: widgets::Button,
}

impl LoginScreen {
    pub fn new(event_sender: AppEventSender) -> Self {
        Self {
            tab_index: 0,
            max_tab: 6,
            event_sender: event_sender.clone(),
            user_input: widgets::InputWidget::new(
                "Username",
                "USERNAME",
                event_sender.clone().into(),
            ),
            pwd_input: widgets::InputWidget::new("Password", "LOGIN", event_sender.clone().into())
                .password(),
            skip_button: widgets::Button::new("Anonym", event_sender.clone(), "LOGIN_ANONYM")
                .theme(widgets::GREEN),
            ok_button: widgets::Button::new("Login", event_sender.clone(), "LOGIN")
                .theme(widgets::BLUE),
            cancel_button: widgets::Button::new("Exit", event_sender.clone(), "QUIT")
                .theme(widgets::RED),
        }
    }
    pub fn send_current_widget_event(&mut self, event: AppEvent) {
        if let Some(elem) = self.current_widget_mut() {
            elem.handle_event(event)
        }
    }
    pub fn send_all_widgets_event(&mut self, event: AppEvent) {
        for i in 0..self.max_tab {
            if let Some(elem) = self.widget_at_mut(i) {
                elem.handle_event(event.clone());
            }
        }
    }

    pub fn current_widget(&self) -> Option<&dyn Widget> {
        self.widget_at(self.tab_index)
    }
    pub fn widget_at(&self, index: usize) -> Option<&dyn Widget> {
        match index {
            1 => Some(&self.user_input as &dyn Widget),
            2 => Some(&self.pwd_input as &dyn Widget),
            3 => Some(&self.ok_button as &dyn Widget),
            4 => Some(&self.skip_button as &dyn Widget),
            5 => Some(&self.cancel_button as &dyn Widget),
            _ => None,
        }
    }
    pub fn widget_at_mut(&mut self, index: usize) -> Option<&mut dyn Widget> {
        match index {
            1 => Some(&mut self.user_input as &mut dyn Widget),
            2 => Some(&mut self.pwd_input as &mut dyn Widget),
            3 => Some(&mut self.ok_button as &mut dyn Widget),
            4 => Some(&mut self.skip_button as &mut dyn Widget),
            5 => Some(&mut self.cancel_button as &mut dyn Widget),
            _ => None,
        }
    }
    pub fn current_widget_mut(&mut self) -> Option<&mut dyn Widget> {
        self.widget_at_mut(self.tab_index)
    }
    fn incr_tab(&mut self) {
        self.send_current_widget_event(AppEvent::NoFocus);
        self.tab_index = (self.tab_index.wrapping_add(1)) % self.max_tab;
        self.send_current_widget_event(AppEvent::Focus);
    }
    fn decr_tab(&mut self) {
        self.send_current_widget_event(AppEvent::NoFocus);
        self.tab_index = if self.tab_index == 0 {
            self.max_tab - 1
        } else {
            self.tab_index.wrapping_sub(1)
        } % self.max_tab;
        self.send_current_widget_event(AppEvent::Focus);
    }
}

impl Screen for LoginScreen {
    fn handle_event(&mut self, event: AppEvent) -> bool {
        match event {
            AppEvent::Clear(hard) => {
                self.tab_index = 0;
                for i in 0..self.max_tab {
                    if let Some(w) = self.widget_at_mut(i) {
                        w.handle_event(AppEvent::Clear(hard));
                    }
                }
            }
            AppEvent::OnWidgetEnter(str, _) if str == "USERNAME" => {
                self.incr_tab();
                return true;
            }
            AppEvent::KeyEvent(key_event) => match key_event.code {
                KeyCode::Tab if key_event.kind == KeyEventKind::Press => {
                    self.incr_tab();
                }
                KeyCode::BackTab if key_event.kind == KeyEventKind::Press => {
                    self.decr_tab();
                }
                KeyCode::Esc => {
                    self.send_all_widgets_event(AppEvent::NoFocus);
                    self.tab_index = 0;
                }
                _ => {
                    self.send_current_widget_event(AppEvent::KeyEvent(key_event));
                }
            },
            _ => {
                return false;
            }
        };
        true
    }

    /*
    }
    impl UiWidget for &LoginScreen {
        fn render(self, area: Rect, buf: &mut Buffer) {
        */
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
        self.user_input.draw(user_input, buf, &mut u_x);

        // Password Input
        self.pwd_input.draw(pwd_input, buf, &mut p_x);

        // Buttons
        let x = 50;
        let [ok_area, anonym_area] =
            Layout::horizontal([Constraint::Percentage(x), Constraint::Percentage(x)])
                .areas(buttons);
        self.ok_button.draw(ok_area, buf, &mut None);
        self.skip_button.draw(anonym_area, buf, &mut None);
        self.cancel_button.draw(idk, buf, &mut None);

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

    fn get_data(&self) -> serde_json::Value {
        serde_json::json!({
            "username":self.user_input.get_content(),
            "password":self.pwd_input.get_content(),
        })
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
    fn test_render_home() {
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
}
