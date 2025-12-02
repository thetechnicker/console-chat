use ratatui::style::Color;
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    pub text: Color,
    pub background: Color,
    pub highlight: Color,
    pub shadow: Color,
}

impl Default for Theme {
    fn default() -> Self {
        BLUE
    }
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

pub const fn colors(theme: Theme) -> (Color, Color, Color, Color) {
    (theme.background, theme.text, theme.shadow, theme.highlight)
}

/*pub enum SerializedTheme {
    Const(String),
    Custom(Theme),
}*/

impl Serialize for Theme {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Compare self to predefined constants and serialize as string if matched
        if *self == BLUE {
            serializer.serialize_str("BLUE")
        } else if *self == RED {
            serializer.serialize_str("RED")
        } else if *self == GREEN {
            serializer.serialize_str("GREEN")
        } else if *self == GRAY {
            serializer.serialize_str("GRAY")
        } else if *self == DARK_GRAY {
            serializer.serialize_str("DARK_GRAY")
        } else {
            // Otherwise serialize full Theme normally as a struct
            #[derive(Serialize)]
            struct ThemeData<'a> {
                text: &'a Color,
                background: &'a Color,
                highlight: &'a Color,
                shadow: &'a Color,
            }
            let data = ThemeData {
                text: &self.text,
                background: &self.background,
                highlight: &self.highlight,
                shadow: &self.shadow,
            };
            data.serialize(serializer)
        }
    }
}

struct ThemeVisitor;

impl<'de> Visitor<'de> for ThemeVisitor {
    type Value = Theme;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string with predefined theme name or a Theme struct")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match v {
            "BLUE" => Ok(BLUE),
            "RED" => Ok(RED),
            "GREEN" => Ok(GREEN),
            "GRAY" => Ok(GRAY),
            "DARK_GRAY" => Ok(DARK_GRAY),
            _ => Err(de::Error::unknown_variant(
                v,
                &["BLUE", "RED", "GREEN", "GRAY", "DARK_GRAY"],
            )),
        }
    }

    fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
    where
        A: de::MapAccess<'de>,
    {
        // Deserialize full Theme struct from map
        #[derive(Deserialize)]
        struct ThemeData {
            text: Color,
            background: Color,
            highlight: Color,
            shadow: Color,
        }
        let data = ThemeData::deserialize(de::value::MapAccessDeserializer::new(map))?;
        Ok(Theme {
            text: data.text,
            background: data.background,
            highlight: data.highlight,
            shadow: data.shadow,
        })
    }
}

impl<'de> Deserialize<'de> for Theme {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(ThemeVisitor)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use color_eyre::Result;
    use std::collections::HashMap;

    #[test]
    fn example_theme() -> Result<()> {
        let map = HashMap::from([
            ("a", GREEN),
            ("b", RED),
            ("c", BLUE),
            ("d", GRAY),
            ("e", DARK_GRAY),
            (
                "f",
                Theme {
                    text: Color::Black,
                    background: Color::White,
                    highlight: Color::Yellow,
                    shadow: Color::Red,
                },
            ),
        ]);
        let content = serde_json::to_string_pretty(&map)?;
        let path = crate::config::get_data_dir();
        if !path.exists() {
            let _ = std::fs::create_dir(&path);
        }
        let res = std::fs::write(path.join("test.json"), content);
        assert!(res.is_ok(), "{res:?}");
        Ok(())
    }
}
