use rand::random;
use ratatui::style::Color;
use serde::{self, Deserialize, Deserializer, Serialize, Serializer};
use std::ops::Deref;
use std::time::Duration;

fn de_duration_from_secs<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let secs = u64::deserialize(deserializer)?; // reads JSON integer
    Ok(Duration::from_secs(secs))
}
fn ser_duration_as_secs<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_u64(duration.as_secs())
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Token {
    pub token: String,
    #[serde(
        deserialize_with = "de_duration_from_secs",
        serialize_with = "ser_duration_as_secs"
    )]
    pub ttl: Duration,
    pub is_new: bool,
}

impl Deref for Token {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.token
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BetterUser {
    pub id: Option<usize>,
    pub username: String,
    // Will always be None
    //pub password_hash: Option<String>,
    pub private: bool,
    pub public_data_id: Option<usize>,
    pub public_data: PublicUser,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublicUser {
    pub display_name: String,
    #[serde(default = "random_color")]
    pub color: Option<String>,
}

impl Default for PublicUser {
    fn default() -> Self {
        Self {
            display_name: "".to_string(),
            color: Some(Color::Gray.to_string()),
        }
    }
}

fn random_color() -> Option<String> {
    let [r, g, b] = random::<[u8; 3]>();
    Some(format!("#{:02x}{:02x}{:02x}", r, g, b))
}
