use crate::event::{AppEvent, EventSender};
use crate::widgets::Widget;
use crossterm::event::KeyEventKind;
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
    label: String,
    theme: Theme,
    event_sender: EventSender,
    on_press: AppEvent,
}

impl Button {
    pub fn new(label: &str, event_sender: EventSender, on_press: AppEvent) -> Self {
        Self {
            state: ButtonState::Normal,
            label: String::from(label),
            theme: BLUE,
            event_sender,
            on_press,
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
}

impl Widget for Button {
    fn handle_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::Clear(_) => self.state = ButtonState::Normal,
            AppEvent::NoFocus => self.state = ButtonState::Normal,
            AppEvent::Focus => self.state = ButtonState::Selected,
            AppEvent::KeyEvent(key) => match key.kind {
                KeyEventKind::Press => {
                    if self.state == ButtonState::Selected {
                        self.state = ButtonState::Active;
                        self.event_sender.send(self.on_press.clone().into());
                    }
                }
                KeyEventKind::Release => {
                    if self.state == ButtonState::Active {
                        self.state = ButtonState::Selected
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }
    fn draw(&self, area: Rect, buf: &mut Buffer, _: &mut Option<u16>) {
        let (background, text, shadow, _highlight) = self.colors();
        let block = Block::bordered()
            .border_type(ratatui::widgets::BorderType::Rounded)
            .style(Style::new().fg(shadow));
        let inner = block.inner(area);
        block.render(area, buf);
        buf.set_style(inner, Style::new().bg(background).fg(text));
        /*
                // render top line if there's enough space
                if area.height > 2 {
                    buf.set_string(
                        area.x,
                        area.y,
                        "▔".repeat(area.width as usize),
                        Style::new().fg(highlight).bg(background),
                    );
                }
                // render bottom line if there's enough space
                if area.height > 1 {
                    buf.set_string(
                        area.x,
                        area.y + area.height - 1,
                        "▁".repeat(area.width as usize),
                        Style::new().fg(shadow).bg(background),
                    );
                }
        */
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
