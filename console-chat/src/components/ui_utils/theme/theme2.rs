use ratatui::style::Style;
use ratatui::widgets::BorderType;
use serde::{Deserialize, Serialize, de::Deserializer, ser::Serializer};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BorderTypeDef(BorderType);

impl From<BorderType> for BorderTypeDef {
    fn from(e: BorderType) -> Self {
        Self(e)
    }
}
impl From<BorderTypeDef> for BorderType {
    fn from(w: BorderTypeDef) -> Self {
        w.0
    }
}

// Serialize as string via Display
impl Serialize for BorderTypeDef {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

// Deserialize from string via FromStr
impl<'de> Deserialize<'de> for BorderTypeDef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        BorderType::from_str(&s)
            .map(BorderTypeDef)
            .map_err(serde::de::Error::custom)
    }
}

// Optional: convenience access
impl BorderTypeDef {
    pub fn into_inner(self) -> BorderType {
        self.0
    }
    pub fn as_ref(&self) -> &BorderType {
        &self.0
    }
}

impl std::ops::Deref for BorderTypeDef {
    type Target = BorderType;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for BorderTypeDef {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub struct ItemStyle {
    text: Style,
    block: Style,
    border: Option<BorderTypeDef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub struct Theme {
    default: ItemStyle,
    highlight: ItemStyle,
    warning: ItemStyle,
    error: ItemStyle,
}
