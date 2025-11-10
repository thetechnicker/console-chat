use crate::widgets::{self, Widget};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::Line,
    widgets::{Block, Widget as w},
};

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    text: Color,
    background: Color,
    highlight: Color,
    shadow: Color,
}

pub const BLUE: Theme = Theme {
    text: Color::Rgb(16, 24, 48),
    background: Color::Rgb(48, 72, 144),
    highlight: Color::Rgb(64, 96, 192),
    shadow: Color::Rgb(32, 48, 96),
};

pub const RED: Theme = Theme {
    text: Color::Rgb(48, 16, 16),
    background: Color::Rgb(144, 48, 48),
    highlight: Color::Rgb(192, 64, 64),
    shadow: Color::Rgb(96, 32, 32),
};

pub const GREEN: Theme = Theme {
    text: Color::Rgb(16, 48, 16),
    background: Color::Rgb(48, 144, 48),
    highlight: Color::Rgb(64, 192, 64),
    shadow: Color::Rgb(32, 96, 32),
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonState {
    Normal,
    Selected,
    Active,
}

#[derive(Debug, Clone)]
pub struct Button {
    state: ButtonState,
    prev_state: ButtonState,
    label: String,
    theme: Theme,
    key_id: char,
    on_press_id: String,
}

impl Button {
    pub fn new(label: &str, key_id: char, on_press: &str) -> Self {
        Self {
            state: ButtonState::Normal,
            prev_state: ButtonState::Normal,
            label: String::from(label),
            theme: BLUE,
            key_id,
            on_press_id: on_press.to_uppercase().to_string(),
        }
    }
    pub fn is_pressed(&self) -> bool {
        self.state == ButtonState::Active
    }
    pub const fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }
    const fn colors(&self) -> (Color, Color, Color, Color) {
        let theme = self.theme;
        match self.state {
            ButtonState::Normal => (theme.background, theme.text, theme.shadow, theme.highlight),
            ButtonState::Selected => (theme.highlight, theme.text, theme.shadow, theme.highlight),
            ButtonState::Active => (theme.background, theme.text, theme.highlight, theme.shadow),
        }
    }

    fn handle_press(&mut self, event: KeyEvent) -> Option<widgets::WidgetEvent> {
        match event.kind {
            KeyEventKind::Press => {
                if self.state == ButtonState::Selected
                    || event.code == KeyCode::Char(self.key_id.clone())
                {
                    self.prev_state = self.state;
                    self.state = ButtonState::Active;
                    return Some(widgets::WidgetEvent::Button(self.on_press_id.clone()));
                }
                None
            }
            KeyEventKind::Release => {
                if self.state == ButtonState::Active {
                    self.state = self.prev_state;
                    return None;
                }
                None
            }
            _ => None,
        }
    }
}

impl Widget for Button {
    fn clear(&mut self, _: bool) {
        self.state = ButtonState::Normal;
    }

    fn handle_key_event(&mut self, event: KeyEvent) -> Option<widgets::WidgetEvent> {
        match event.code {
            KeyCode::Enter => self.handle_press(event),
            KeyCode::Char(' ') => self.handle_press(event),
            KeyCode::Char(c) if c == self.key_id => self.handle_press(event),
            _ => None,
        }
    }

    fn focus(&mut self) {
        self.state = ButtonState::Selected;
    }
    fn unfocus(&mut self) {
        self.state = ButtonState::Normal;
    }

    fn draw(&self, area: Rect, buf: &mut Buffer, _: &mut Option<u16>) {
        let (background, text, shadow, _highlight) = self.colors();
        let block = Block::bordered()
            .title_bottom(self.key_id.to_string())
            .border_type(ratatui::widgets::BorderType::Rounded)
            .style(Style::new().fg(shadow));
        let inner = block.inner(area);
        block.render(area, buf);
        buf.set_style(inner, Style::new().bg(background).fg(text));

        let line: Line<'_> = String::from(&self.label).into();
        // render label centered
        buf.set_line(
            area.x + (area.width.saturating_sub(line.width() as u16)) / 2,
            area.y + (area.height.saturating_sub(1)) / 2,
            &line,
            area.width,
        );
    }
}
