use ratatui::{
    prelude::*,
    style::{Stylize, palette::tailwind},
    widgets::*,
};
use strum::{Display, EnumIter, FromRepr};

#[derive(Default, Clone, Copy, Display, FromRepr, PartialEq, EnumIter)]
pub enum Chategory {
    #[default]
    #[strum(to_string = "Basics")]
    Basic,
    #[strum(to_string = "Network")]
    Network,
    #[strum(to_string = "Design")]
    Desing,
    #[strum(to_string = "Shortcuts")]
    Shortcuts,
    #[strum(to_string = "Settings File")]
    File,
}

impl Chategory {
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

    /// Return tab's name as a styled `Line`
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
            Self::Basic => tailwind::BLUE,
            Self::Desing => tailwind::AMBER,
            Self::Shortcuts => tailwind::ROSE,
            Self::File => tailwind::SKY,
            Self::Network => tailwind::PURPLE,
        }
    }
    pub fn render(self, area: Rect, buf: &mut Buffer) -> Rect {
        let block = self.block();
        let inner = block.inner(area);
        block.render(area, buf);
        inner
    }
}
