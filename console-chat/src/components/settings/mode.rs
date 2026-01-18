use crate::app::Mode;
use ratatui::{
    prelude::*,
    style::{Stylize, palette::tailwind},
    widgets::*,
};

impl Mode {
    /// Get the previous tab, if there is no previous tab return the current tab.
    pub fn previous(self) -> Self {
        let current_index: usize = self as usize;
        let previous_index = current_index.saturating_sub(1);
        Self::from_repr(previous_index).unwrap_or(self)
    }

    /// Get the next tab, if there is no next tab return the current tab.
    pub fn next(self) -> Self {
        let current_index = self as usize;
        let next_index = current_index.saturating_add(1);
        Self::from_repr(next_index).unwrap_or(self)
    }

    pub fn title(self) -> Line<'static> {
        format!("  {self}  ")
            .fg(tailwind::SLATE.c200)
            .bg(self.palette().c900)
            .into()
    }

    /// A block surrounding the tab's content
    pub fn block(self) -> Block<'static> {
        Block::bordered()
            .border_set(symbols::border::PROPORTIONAL_TALL)
            .padding(Padding::horizontal(1))
            .border_style(self.palette().c700)
    }
    pub const fn palette(self) -> tailwind::Palette {
        match self {
            Mode::Home => tailwind::BLUE,
            Mode::Global => tailwind::AMBER,
            Mode::Chat => tailwind::ROSE,
            Mode::Join => tailwind::PURPLE,
            Mode::Login => tailwind::RED,
            Mode::Settings => tailwind::SKY,
            Mode::Insert => tailwind::STONE,
        }
    }
    pub fn render(self, area: Rect, buf: &mut Buffer) -> Rect {
        let block = self.block();
        let inner = block.inner(area);
        block.render(area, buf);
        inner
    }
}
