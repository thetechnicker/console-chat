use ratatui::style::Color;

pub fn get_inverse(color: Color) -> Color {
    match color {
        Color::Reset => color,
        Color::Black => Color::White,
        Color::Red => Color::Cyan,
        Color::Green => Color::Magenta,
        Color::Yellow => Color::Blue,
        Color::Blue => Color::Yellow,
        Color::Magenta => Color::Green,
        Color::Cyan => Color::Red,
        Color::Gray => Color::DarkGray,
        Color::DarkGray => Color::Gray,
        Color::LightRed => Color::LightCyan,
        Color::LightGreen => Color::LightMagenta,
        Color::LightYellow => Color::LightBlue,
        Color::LightBlue => Color::LightYellow,
        Color::LightMagenta => Color::LightGreen,
        Color::LightCyan => Color::LightRed,
        Color::White => Color::Black,
        Color::Rgb(r, g, b) => get_rgb_inverse(r, g, b),
        Color::Indexed(i) => get_indexed_inverse(i),
    }
}

fn get_rgb_inverse(r: u8, g: u8, b: u8) -> Color {
    use palette::{FromColor, Hsl, Srgb};
    let r_f = r as f32 / 255.0;
    let g_f = g as f32 / 255.0;
    let b_f = b as f32 / 255.0;
    let hsl: Hsl = Hsl::from_color(Srgb::new(r_f, g_f, b_f)).into_format::<f32>();
    let new_hue = (hsl.hue.into_degrees() + 180.0) % 360.0;
    let complementary_hsl = Hsl::new(
        palette::RgbHue::from_degrees(new_hue),
        hsl.saturation,
        hsl.lightness,
    );
    let rgb: Srgb = Srgb::from_color(complementary_hsl);
    Color::Rgb(
        (rgb.red * 255.0).round() as u8,
        (rgb.green * 255.0).round() as u8,
        (rgb.blue * 255.0).round() as u8,
    )
}

fn get_indexed_inverse(i: u8) -> Color {
    match i {
        0..=15 => {
            // Standard and bright ANSI colors: invert based on known complements
            match i {
                0 => Color::Indexed(15), // Black -> White
                1 => Color::Indexed(14), // Red -> Light Cyan
                2 => Color::Indexed(13), // Green -> Light Magenta
                3 => Color::Indexed(12), // Yellow -> Light Blue
                4 => Color::Indexed(11), // Blue -> Light Yellow
                5 => Color::Indexed(10), // Magenta -> Light Green
                6 => Color::Indexed(9),  // Cyan -> Light Red
                7 => Color::Indexed(8),  // Gray -> Dark Gray
                8 => Color::Indexed(7),  // Dark Gray -> Gray
                9 => Color::Indexed(6),  // Light Red -> Cyan
                10 => Color::Indexed(5), // Light Green -> Magenta
                11 => Color::Indexed(4), // Light Yellow -> Blue
                12 => Color::Indexed(3), // Light Blue -> Yellow
                13 => Color::Indexed(2), // Light Magenta -> Green
                14 => Color::Indexed(1), // Light Cyan -> Red
                15 => Color::Indexed(0), // White -> Black
                _ => Color::Indexed(i),
            }
        }
        16..=231 => {
            // 6x6x6 cube
            let c = i - 16;
            let r = c / 36;
            let g = (c % 36) / 6;
            let b = c % 6;
            // invert each RGB component in 0-5 range
            let r_inv = 5 - r;
            let g_inv = 5 - g;
            let b_inv = 5 - b;
            Color::Indexed(16 + (r_inv * 36) + (g_inv * 6) + b_inv)
        }
        232..=255 => {
            // Grayscale ramp; invert around midpoint (232+255=487)
            let inv = 487u16 - i as u16;
            Color::Indexed(inv as u8)
        } //_ => Color::Indexed(i), // fallback
    }
}
