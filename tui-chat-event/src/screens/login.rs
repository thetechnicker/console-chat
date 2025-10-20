use crate::DEFAULT_BORDER;
use crate::event::{AppEvent, Event, EventSender, WidgetEvent};
use crate::screens::CurrentScreen;
use crate::screens::Screen;
use crate::widgets;
use crate::widgets::Widget;
use ratatui::crossterm::event::{KeyCode, KeyEventKind};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Rect},
    widgets::{Block, BorderType, Widget as UiWidget},
};

#[derive(Debug)]
pub struct LoginScreen {
    pub tab_index: usize,
    pub max_tab: usize,
    pub event_sender: EventSender,
    pub user_input: widgets::InputWidget,
    pub pwd_input: widgets::InputWidget,
    pub ok_button: widgets::Button,
    pub cancel_button: widgets::Button,
}

impl LoginScreen {
    pub fn new(event_sender: EventSender) -> Self {
        Self {
            tab_index: 0,
            max_tab: 5,
            event_sender,
            user_input: widgets::InputWidget::new("Username"),
            pwd_input: widgets::InputWidget::new("Password").password(),
            ok_button: widgets::Button::new("OK").theme(widgets::BLUE),
            cancel_button: widgets::Button::new("CANCEL").theme(widgets::RED),
        }
    }
    pub fn send_current_widget_event(&mut self, event: WidgetEvent) {
        if let Some(elem) = self.current_widget() {
            elem.handle_event(event)
        }
    }
    pub fn send_all_widgets_event(&mut self, event: WidgetEvent) {
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
            _ => None,
        }
    }
    pub fn current_widget(&mut self) -> Option<&mut dyn Widget> {
        self.widget_at(self.tab_index)
    }
}

impl Screen for LoginScreen {
    fn handle_event(&mut self, event: WidgetEvent) {
        match event {
            WidgetEvent::KeyEvent(key_event) => match key_event.code {
                KeyCode::Tab if key_event.kind == KeyEventKind::Press => {
                    self.send_current_widget_event(WidgetEvent::NoFocus);
                    self.tab_index = (self.tab_index + 1) % self.max_tab;
                    self.send_current_widget_event(WidgetEvent::Focus);
                }
                KeyCode::BackTab if key_event.kind == KeyEventKind::Press => {
                    self.send_current_widget_event(WidgetEvent::NoFocus);
                    self.tab_index = (self.tab_index - 1) % self.max_tab;
                    self.send_current_widget_event(WidgetEvent::Focus);
                }
                KeyCode::Esc => {
                    self.send_all_widgets_event(WidgetEvent::NoFocus);
                    self.tab_index = 0;
                }
                _ => {
                    self.send_current_widget_event(WidgetEvent::KeyEvent(key_event));
                    if self.ok_button.is_pressed() {
                        self.event_sender
                            .send(Event::App(AppEvent::SwitchScreen(CurrentScreen::Chat)))
                    }
                    if self.cancel_button.is_pressed() {
                        self.event_sender.send(Event::App(AppEvent::Quit))
                    }
                }
            },
            _ => {}
        };
    }
}

impl UiWidget for &LoginScreen {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let outer_block = Block::bordered()
            .border_type(BorderType::Double)
            .title("TUI-CHAT")
            .title_alignment(Alignment::Center);
        let inner = outer_block.inner(area);

        outer_block.render(area, buf);
        let [left, main, right] = Layout::horizontal([
            Constraint::Percentage(20),
            Constraint::Percentage(60),
            Constraint::Percentage(20),
        ])
        .areas(inner);

        // LEFT

        let left_block = Block::bordered().border_type(DEFAULT_BORDER);
        let _left_inner = left_block.inner(left);
        left_block.render(left, buf);

        // RIGHT

        let right_block = Block::bordered().border_type(DEFAULT_BORDER);
        let _right_inner = right_block.inner(right);
        right_block.render(right, buf);

        // MAIN
        let login_block = Block::bordered().border_type(DEFAULT_BORDER);
        let login_inner = login_block.inner(main);

        login_block.render(main, buf);
        let [_, input_area, _] = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Percentage(60),
            Constraint::Fill(1),
        ])
        .areas(login_inner);

        // Input
        let [_, user_input, pwd_input, buttons, _] = Layout::vertical([
            Constraint::Fill(1),
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
        let [ok_area, cancel_area] =
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                .areas(buttons);
        self.ok_button.draw(ok_area, buf);
        self.cancel_button.draw(cancel_area, buf);
    }
}
