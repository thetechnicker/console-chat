use super::EventWidget;
use super::theme;
use crate::action::ActionSubsetWrapper;
use crate::action::ButtonEvent;
use crate::error::Result;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::Line,
    widgets::{Block, Widget},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonState {
    #[default]
    Normal,
    Selected,
    Active,
}

pub const fn colors_from_state(
    state: ButtonState,
    theme: theme::ButtonStatePalettes,
) -> (Color, Color, Color, Color) {
    let used_theme = match state {
        ButtonState::Normal => theme.normal,
        ButtonState::Selected => theme.active,
        ButtonState::Active => theme.pressed,
    };
    (
        used_theme.background,
        used_theme.text,
        used_theme.shadow,
        used_theme.highlight,
    )
}

#[derive(Debug, Default)]
pub struct Button {
    state: ButtonState,
    theme: theme::ButtonStatePalettes,
    label: String,
    sub_titel: String,
    action: Option<ButtonEvent>,
}

impl Button {
    pub fn new(
        label: impl Into<String>,
        sub_titel: impl Into<String>,
        theme: theme::ButtonStatePalettes,
        action: ButtonEvent,
    ) -> Self {
        Self {
            state: ButtonState::Normal,
            theme,
            label: label.into(),
            sub_titel: sub_titel.into(),
            action: Some(action),
        }
    }

    pub fn is_active(&self) -> bool {
        self.state == ButtonState::Active
    }

    pub fn trigger(&self) -> Option<ButtonEvent> {
        self.action.clone()
    }

    pub fn set_state(&mut self, state: ButtonState) {
        self.state = state;
    }

    pub fn draw_button(&self, area: Rect, buf: &mut Buffer) {
        let (background, text, shadow, _highlight) = colors_from_state(self.state, self.theme);
        let block = Block::bordered()
            .title_bottom(self.sub_titel.clone())
            .border_type(ratatui::widgets::BorderType::Rounded)
            .style(Style::new().fg(shadow));
        let inner = block.inner(area);
        block.render(area, buf);
        buf.set_style(inner, Style::new().bg(background).fg(text));

        let line: Line<'_> = self.label.clone().into();
        // render label centered
        buf.set_line(
            area.x + (area.width.saturating_sub(line.width() as u16)) / 2,
            area.y + (area.height.saturating_sub(1)) / 2,
            &line,
            area.width,
        );
    }
}

use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;

impl EventWidget for Button {
    fn handle_event(&mut self, key: KeyEvent) -> Result<Option<ActionSubsetWrapper>> {
        if key.code == KeyCode::Enter {
            match key.kind {
                KeyEventKind::Repeat => {}
                KeyEventKind::Press => {
                    self.set_state(ButtonState::Active);
                    return Ok(self.action.as_ref().map(|a| a.into()));
                }
                KeyEventKind::Release => self.set_state(ButtonState::Selected),
            }
        }
        Ok(None)
    }
}
