use super::theme::*;
use crate::action::Action;
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

pub const fn colors_from_state(state: ButtonState, theme: Theme) -> (Color, Color, Color, Color) {
    match state {
        ButtonState::Normal => (theme.background, theme.text, theme.shadow, theme.highlight),
        ButtonState::Selected => (theme.highlight, theme.text, theme.shadow, theme.highlight),
        ButtonState::Active => (theme.background, theme.text, theme.highlight, theme.shadow),
    }
}

#[derive(Debug, Default)]
pub struct Button {
    state: ButtonState,
    theme: Theme,
    label: String,
    sub_titel: String,
    action: Option<Action>,
}

impl Button {
    pub fn new(
        label: impl Into<String>,
        sub_titel: impl Into<String>,
        theme: Theme,
        action: Action,
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
    /*
    pub fn is_selected(&self) -> bool {
        self.state == ButtonState::Selected
    }
    pub fn is_normal(&self) -> bool {
        self.state == ButtonState::Normal
    }
    */

    pub fn trigger(&self) -> Option<Action> {
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
