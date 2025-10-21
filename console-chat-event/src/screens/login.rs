use crate::DEFAULT_BORDER;
use crate::event::{AppEvent, EventSender};
use crate::screens::Screen;
use crate::widgets;
use crate::widgets::Widget;
use ratatui::crossterm::event::{KeyCode, KeyEventKind};
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
    pub event_sender: EventSender,
    pub user_input: widgets::InputWidget,
    pub pwd_input: widgets::InputWidget,
    pub skip_button: widgets::Button,
    pub ok_button: widgets::Button,
    pub cancel_button: widgets::Button,
}

impl LoginScreen {
    pub fn new(event_sender: EventSender) -> Self {
        Self {
            tab_index: 0,
            max_tab: 6,
            event_sender: event_sender.clone(),
            user_input: widgets::InputWidget::new("Username"),
            pwd_input: widgets::InputWidget::new("Password").password(),
            skip_button: widgets::Button::new(
                "Join as Anonym",
                event_sender.clone(),
                AppEvent::ButtonPress("LOGIN".to_string()),
            )
            .theme(widgets::GREEN),
            ok_button: widgets::Button::new(
                "OK",
                event_sender.clone(),
                AppEvent::ButtonPress("LOGIN".to_string()),
            )
            .theme(widgets::BLUE),
            cancel_button: widgets::Button::new("CANCEL", event_sender.clone(), AppEvent::Quit)
                .theme(widgets::RED),
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
            1 => Some(&mut self.user_input as &mut dyn Widget),
            2 => Some(&mut self.pwd_input as &mut dyn Widget),
            3 => Some(&mut self.ok_button as &mut dyn Widget),
            4 => Some(&mut self.cancel_button as &mut dyn Widget),
            5 => Some(&mut self.skip_button as &mut dyn Widget),
            _ => None,
        }
    }
    pub fn current_widget(&mut self) -> Option<&mut dyn Widget> {
        self.widget_at(self.tab_index)
    }

    pub fn get_login_data(&self) -> serde_json::Value {
        serde_json::json!({
            "username":self.user_input.get_content(),
            "password":self.pwd_input.get_content(),
        })
    }
}

impl Screen for LoginScreen {
    fn handle_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::KeyEvent(key_event) => match key_event.code {
                KeyCode::Tab if key_event.kind == KeyEventKind::Press => {
                    self.send_current_widget_event(AppEvent::NoFocus);
                    self.tab_index = (self.tab_index.wrapping_add(1)) % self.max_tab;
                    self.send_current_widget_event(AppEvent::Focus);
                }
                KeyCode::BackTab if key_event.kind == KeyEventKind::Press => {
                    self.send_current_widget_event(AppEvent::NoFocus);
                    self.tab_index = (self.tab_index.wrapping_sub(1)) % self.max_tab;
                    self.send_current_widget_event(AppEvent::Focus);
                }
                KeyCode::Esc => {
                    self.send_all_widgets_event(AppEvent::NoFocus);
                    self.tab_index = 0;
                }
                _ => {
                    self.send_current_widget_event(AppEvent::KeyEvent(key_event));
                }
            },
            _ => {}
        };
    }
}

impl UiWidget for &LoginScreen {
    fn render(self, area: Rect, buf: &mut Buffer) {
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

        // User Input
        self.user_input.draw(user_input, buf);

        // Password Input
        self.pwd_input.draw(pwd_input, buf);

        // Buttons
        let x = 50;
        let [ok_area, cancel_area] =
            Layout::horizontal([Constraint::Percentage(x), Constraint::Percentage(x)])
                .areas(buttons);
        self.ok_button.draw(ok_area, buf);
        self.cancel_button.draw(cancel_area, buf);
        self.skip_button.draw(idk, buf);
    }
}
