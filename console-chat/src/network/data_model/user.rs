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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_random_color() {
        let color_option = random_color();
        assert!(color_option.is_some());
        let color_str = color_option.unwrap();
        let color = color_str.parse::<Color>();
        assert!(color.is_ok());
    }

    #[test]
    fn test_token_serialization() {
        let token = Token {
            token: "my_secret_token".to_string(),
            ttl: Duration::from_secs(3600),
            is_new: true,
        };

        // Serialize Token to JSON
        let serialized = serde_json::to_string(&token).unwrap();
        let expected_json = r#"{"token":"my_secret_token","ttl":3600,"is_new":true}"#;
        assert_eq!(serialized, expected_json);

        // Deserialize JSON back to Token
        let deserialized: Token = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.token, token.token);
        assert_eq!(deserialized.ttl, token.ttl);
        assert_eq!(deserialized.is_new, token.is_new);
    }

    #[test]
    fn test_public_user_default() {
        let user = PublicUser::default();
        assert_eq!(user.display_name, "");
        assert_eq!(user.color, Some(Color::Gray.to_string()));
    }

    #[test]
    fn test_better_user_serialization() {
        let public_data = PublicUser {
            display_name: "JohnDoe".to_string(),
            color: Some("blue".to_string()),
        };

        let better_user = BetterUser {
            id: Some(1),
            username: "johndoe123".to_string(),
            private: false,
            public_data_id: None,
            public_data,
        };

        // Serialize BetterUser to JSON
        let serialized = serde_json::to_string(&better_user).unwrap();
        let expected_json = r#"{"id":1,"username":"johndoe123","private":false,"public_data_id":null,"public_data":{"display_name":"JohnDoe","color":"blue"}}"#;
        assert_eq!(serialized, expected_json);

        // Deserialize JSON back to BetterUser
        let deserialized: BetterUser = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.id, better_user.id);
        assert_eq!(deserialized.username, better_user.username);
        assert_eq!(deserialized.private, better_user.private);
        assert_eq!(deserialized.public_data_id, better_user.public_data_id);
        assert_eq!(
            deserialized.public_data.display_name,
            better_user.public_data.display_name
        );
        assert_eq!(
            deserialized.public_data.color,
            better_user.public_data.color
        );
    }

    #[test]
    fn test_token_deref() {
        let token = Token {
            token: "my_secret_token".to_string(),
            ttl: Duration::from_secs(3600),
            is_new: true,
        };

        // Deref to String
        let token_as_string: &String = &token;
        assert_eq!(token_as_string, "my_secret_token");
    }
}
