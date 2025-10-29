use crate::network::error::ApiError;
use base64::{Engine as _, engine::general_purpose};

// TODO: Replace with random generated, between client syncronised key
pub const KEY: [u8; 20] = [
    0x12, 0x11, 0xab, 0x12, 0xff, 0x12, 0x11, 0xab, 0x12, 0xff, 0x12, 0x11, 0xab, 0x12, 0xff, 0x12,
    0x11, 0xab, 0x12, 0xff,
];

pub fn encrypt_text(text: &str, key: &[u8]) -> String {
    let bytes: &[u8] = text.as_bytes();
    let mut bytes: Vec<u8> = Vec::from(bytes);
    let key_len = key.len();
    // TODO: Replace with actual encryption
    for (i, b) in bytes.iter_mut().enumerate() {
        *b = *b ^ key.get(i % key_len).unwrap_or(&(0xff as u8));
    }
    general_purpose::STANDARD.encode(bytes)
}

pub fn decrypt_bytes(s: &str, key: &[u8]) -> Result<String, ApiError> {
    let mut bytes = general_purpose::STANDARD.decode(s)?;
    let key_len = key.len();
    // TODO: Replace with actual decryption
    for (i, b) in bytes.iter_mut().enumerate() {
        *b = *b ^ key.get(i % key_len).unwrap_or(&(0xff as u8));
    }
    let string = str::from_utf8(&bytes)?;
    Ok(String::from(string))
}
