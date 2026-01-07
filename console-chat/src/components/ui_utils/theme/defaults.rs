#![allow(dead_code)]
use super::theme::Theme;
use ratatui::style::Color;

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

pub const GRAY: Theme = Theme {
    text: Color::Rgb(48, 48, 48),          // Dark but readable text
    background: Color::Rgb(128, 128, 128), // Medium-gray background, similar brightness to others
    highlight: Color::Rgb(160, 160, 160),  // Slightly lighter for highlight
    shadow: Color::Rgb(96, 96, 96),        // Darker shadow tone
};

pub const DARK_GRAY: Theme = Theme {
    text: Color::Rgb(200, 200, 200), // Light text for contrast on dark background
    background: Color::Rgb(48, 48, 48), // Dark gray background
    highlight: Color::Rgb(72, 72, 72), // Slightly lighter for subtle highlights
    shadow: Color::Rgb(24, 24, 24),  // Very dark shadow tone
};

pub const CYAN: Theme = Theme {
    text: Color::Rgb(6, 58, 58),
    background: Color::Rgb(32, 160, 160),
    highlight: Color::Rgb(64, 192, 192),
    shadow: Color::Rgb(8, 40, 40),
};

pub const MAGENTA: Theme = Theme {
    text: Color::Rgb(58, 6, 48),
    background: Color::Rgb(200, 32, 160),
    highlight: Color::Rgb(224, 80, 192),
    shadow: Color::Rgb(48, 8, 40),
};

pub const ORANGE: Theme = Theme {
    text: Color::Rgb(64, 32, 0),
    background: Color::Rgb(224, 112, 32),
    highlight: Color::Rgb(240, 160, 64),
    shadow: Color::Rgb(96, 48, 8),
};

pub const PURPLE: Theme = Theme {
    text: Color::Rgb(48, 16, 72),
    background: Color::Rgb(112, 64, 192),
    highlight: Color::Rgb(152, 112, 224),
    shadow: Color::Rgb(40, 16, 56),
};

pub const TEAL: Theme = Theme {
    text: Color::Rgb(8, 56, 48),
    background: Color::Rgb(48, 200, 168),
    highlight: Color::Rgb(96, 224, 200),
    shadow: Color::Rgb(16, 40, 32),
};

pub const MONOCHROME: Theme = Theme {
    text: Color::Rgb(220, 220, 220),
    background: Color::Rgb(34, 34, 34),
    highlight: Color::Rgb(100, 100, 100),
    shadow: Color::Rgb(16, 16, 16),
};
