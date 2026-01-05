use ratatui::style::{Color, Style};
use serde::{Deserialize, Serialize};

mod defaults;
pub use defaults::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub struct Theme {
    pub text: Color,
    pub background: Color,
    pub highlight: Color,
    pub shadow: Color,
}

impl Into<Style> for Theme {
    fn into(self) -> Style {
        let style: Style = self.text.into();
        style.bg(self.background)
    }
}

impl Default for Theme {
    fn default() -> Self {
        BLUE
    }
}

pub fn colors(theme: impl Into<Theme>) -> (Color, Color, Color, Color) {
    let theme = theme.into();
    (theme.background, theme.text, theme.shadow, theme.highlight)
}

/*
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum ThemeKind {
    Blue,
    Red,
    Green,
    Gray,
    DarkGray,
    Cyan,
    Magenta,
    Orange,
    Purple,
    Teal,
    Monochrome,
    Custom(Theme),
}

impl ThemeKind {
    pub const fn as_theme(self) -> Theme {
        match self {
            ThemeKind::Blue => BLUE,
            ThemeKind::Red => RED,
            ThemeKind::Green => GREEN,
            ThemeKind::Gray => GRAY,
            ThemeKind::DarkGray => DARK_GRAY,
            ThemeKind::Cyan => CYAN,
            ThemeKind::Magenta => MAGENTA,
            ThemeKind::Orange => ORANGE,
            ThemeKind::Purple => PURPLE,
            ThemeKind::Teal => TEAL,
            ThemeKind::Monochrome => MONOCHROME,
            ThemeKind::Custom(t) => t,
        }
    }
}
impl Into<Theme> for ThemeKind {
    fn into(self) -> Theme {
        self.as_theme()
    }
}

impl Into<Style> for ThemeKind {
    fn into(self) -> Style {
        self.as_theme().into()
    }
}
*/
