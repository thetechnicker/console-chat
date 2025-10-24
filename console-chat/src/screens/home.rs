use crate::DEFAULT_BORDER;
use crate::event::{AppEvent, EventSender};
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
pub struct HomeScreen {
    pub tab_index: usize,
    pub max_tab: usize,
    pub event_sender: EventSender,
    pub room_input: widgets::InputWidget,
    pub join_button: widgets::Button,
    pub logout_button: widgets::Button,
    pub exit_button: widgets::Button,
}

impl HomeScreen {
    pub fn new(event_sender: EventSender) -> Self {
        Self {
            tab_index: 0,
            max_tab: 5,
            event_sender: event_sender.clone(),
            room_input: widgets::InputWidget::new("Room"),
            join_button: widgets::Button::new(
                "Join Room",
                event_sender.clone(),
                AppEvent::ButtonPress("JOIN".to_string()),
            )
            .theme(widgets::GREEN),
            logout_button: widgets::Button::new(
                "Logout",
                event_sender.clone(),
                AppEvent::ButtonPress("LOGOUT".to_string()),
            )
            .theme(widgets::BLUE),
            exit_button: widgets::Button::new("Exit", event_sender.clone(), AppEvent::Quit)
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
            1 => Some(&mut self.room_input as &mut dyn Widget),
            2 => Some(&mut self.join_button as &mut dyn Widget),
            3 => Some(&mut self.logout_button as &mut dyn Widget),
            4 => Some(&mut self.exit_button as &mut dyn Widget),
            _ => None,
        }
    }
    pub fn current_widget(&mut self) -> Option<&mut dyn Widget> {
        self.widget_at(self.tab_index)
    }
}

impl Screen for HomeScreen {
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

    fn get_data(&self) -> serde_json::Value {
        serde_json::json!(self.room_input.get_content())
    }

    /*
    }

    impl UiWidget for &HomeScreen {
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
        self.room_input.draw(user_input, buf, &mut None);

        // Buttons
        let x = 50;
        let [join_area, logout_area] =
            Layout::horizontal([Constraint::Percentage(x), Constraint::Percentage(x)])
                .areas(buttons1);
        self.join_button.draw(join_area, buf, &mut None);
        self.logout_button.draw(logout_area, buf, &mut None);
        self.exit_button.draw(buttons2, buf, &mut None);
        None
    }
}
