use crate::components::ui_utils::vim::VimMode;
use ratatui::style::{Color, Modifier, Style};
use serde::{Deserialize, Serialize};

pub const LIGHT_GRAY: Color = Color::Rgb(192, 192, 192);

/// A single 4-color button palette: text, background, shadow, highlight.
#[derive(Clone, Debug, Serialize, Deserialize, Copy)]
pub struct ButtonPalette {
    pub text: Color,
    pub background: Color,
    pub shadow: Color,
    pub highlight: Color,
}
impl Default for ButtonPalette {
    fn default() -> Self {
        Self {
            text: Color::White,
            background: Color::Black,
            shadow: Color::DarkGray,
            highlight: LIGHT_GRAY,
        }
    }
}

/// Four button states: Active, Normal, Pressed.
#[derive(Clone, Debug, Serialize, Deserialize, Copy)]
pub struct ButtonStatePalettes {
    pub active: ButtonPalette,
    pub normal: ButtonPalette,
    pub pressed: ButtonPalette,
}

impl Default for ButtonStatePalettes {
    fn default() -> Self {
        Self {
            active: ButtonPalette {
                text: Color::White,
                background: Color::Blue,
                shadow: Color::DarkGray,
                highlight: Color::LightBlue,
            },
            normal: ButtonPalette {
                text: Color::Black,
                background: Color::Gray,
                shadow: Color::DarkGray,
                highlight: LIGHT_GRAY,
            },
            pressed: ButtonPalette {
                text: Color::White,
                background: Color::DarkGray,
                shadow: Color::Black,
                highlight: Color::Gray,
            },
        }
    }
}

/// Semantic kinds of buttons.
#[derive(Clone, Debug, Serialize, Deserialize, Copy)]
pub struct SemanticButtons {
    pub accepting: ButtonStatePalettes,
    pub mid_accept: ButtonStatePalettes,
    pub normal: ButtonStatePalettes,
    pub denying: ButtonStatePalettes,
}

/// Surrounding or page-level colors.
#[derive(Clone, Debug, Serialize, Deserialize, Copy)]
pub struct PageColors {
    pub background: Color,
    pub foreground: Color,
    pub border: Color,
    pub muted: Color,
}

impl Default for PageColors {
    fn default() -> Self {
        Self {
            background: Color::Black,
            foreground: Color::White,
            border: Color::DarkGray,
            muted: Color::Gray,
        }
    }
}

/// Vi-like input palette and cursor color mapping.
#[derive(Clone, Debug, Serialize, Deserialize, Copy)]
pub struct ViModePalettes {
    pub normal: Color,
    pub insert: Color,
    pub visual: Color,
    pub operator: Color,
}

impl Default for ViModePalettes {
    fn default() -> Self {
        Self {
            normal: Color::Reset,
            insert: Color::LightBlue,
            visual: Color::LightYellow,
            operator: Color::LightGreen,
        }
    }
}
/// Top-level theme containing everything.
#[derive(Clone, Debug, Serialize, Deserialize, Copy)]
pub struct Theme {
    pub buttons: SemanticButtons,
    pub page: PageColors,
    pub vi: ViModePalettes,
}

//fn parse_hex(s: &str) -> Option<(u8, u8, u8)> {
//    let s = s.strip_prefix('#').unwrap_or(s);
//    if s.len() != 6 {
//        return None;
//    }
//    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
//    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
//    let b = u8::from_str_radix(&s[4..6], 16).ok()?;
//    Some((r, g, b))
//}

/// Implement the cursor_style function you showed, using Theme.vi.
impl ViModePalettes {
    pub fn cursor_style_for_mode(&self, mode: &VimMode) -> Style {
        let color = match mode {
            VimMode::Normal => self.normal,
            VimMode::Insert => self.insert,
            VimMode::Visual => self.visual,
            VimMode::Operator(_) => self.operator,
        };
        Style::default().fg(color).add_modifier(Modifier::REVERSED)
    }
}

///// Helper: produce a ratatui Style for a given ButtonPalette (for a particular state)
//pub fn style_for_button(p: &ButtonPalette) -> Style {
//    // Use text as foreground and background as background; highlight/shadow not used directly in Style
//    let fg = p.text;
//    let bg = p.background;
//    Style::default().fg(fg).bg(bg)
//}

/// Example default theme
impl Default for Theme {
    fn default() -> Self {
        Self {
            buttons: SemanticButtons {
                accepting: ButtonStatePalettes {
                    active: ButtonPalette {
                        text: Color::White,
                        background: Color::LightGreen,
                        shadow: Color::Green,
                        highlight: Color::White,
                    },
                    normal: ButtonPalette {
                        text: Color::Black,
                        background: Color::Green,
                        shadow: Color::DarkGray,
                        highlight: Color::LightGreen,
                    },
                    pressed: ButtonPalette {
                        text: Color::White,
                        background: Color::Rgb(0, 128, 0), // #008000
                        shadow: Color::Black,
                        highlight: Color::Green,
                    },
                },
                mid_accept: ButtonStatePalettes {
                    active: ButtonPalette {
                        text: Color::White,
                        background: Color::LightBlue,
                        shadow: Color::Gray,
                        highlight: Color::White,
                    },
                    normal: ButtonPalette {
                        text: Color::Black,
                        background: Color::Blue,
                        shadow: Color::DarkGray,
                        highlight: Color::LightBlue,
                    },
                    pressed: ButtonPalette {
                        text: Color::White,
                        background: Color::Rgb(184, 134, 11), // #B8860B
                        shadow: Color::Black,
                        highlight: Color::Yellow,
                    },
                },
                normal: ButtonStatePalettes {
                    active: ButtonPalette {
                        text: Color::White,
                        background: Color::Blue,
                        shadow: Color::DarkGray,
                        highlight: Color::LightBlue,
                    },
                    normal: ButtonPalette {
                        text: Color::Black,
                        background: Color::Gray,
                        shadow: Color::DarkGray,
                        highlight: Color::Rgb(192, 192, 192), // #C0C0C0
                    },
                    pressed: ButtonPalette {
                        text: Color::White,
                        background: Color::DarkGray,
                        shadow: Color::Black,
                        highlight: Color::Gray,
                    },
                },
                denying: ButtonStatePalettes {
                    active: ButtonPalette {
                        text: Color::White,
                        background: Color::LightRed,
                        shadow: Color::Red,
                        highlight: Color::White,
                    },
                    normal: ButtonPalette {
                        text: Color::Black,
                        background: Color::Red,
                        shadow: Color::DarkGray,
                        highlight: Color::LightRed,
                    },
                    pressed: ButtonPalette {
                        text: Color::White,
                        background: Color::Rgb(128, 0, 0), // #800000
                        shadow: Color::Black,
                        highlight: Color::Red,
                    },
                },
            },
            page: PageColors {
                background: Color::Rgb(68, 68, 68), // #444444
                foreground: Color::Black,
                border: Color::Rgb(51, 51, 51), // #333333
                muted: Color::Rgb(85, 85, 85),  // #555555
            },
            vi: ViModePalettes {
                normal: Color::Reset,
                insert: Color::LightBlue,
                visual: Color::LightYellow,
                operator: Color::LightGreen,
            },
        }
    }
}
