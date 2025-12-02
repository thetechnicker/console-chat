//! This module [`encryption`] is a wrapper Module for all cryptographic code
use crate::network::error::NetworkError;
use alkali::asymmetric::cipher;
use alkali::mem;
use alkali::symmetric::cipher as symetric_cipher;
use base64::{Engine as _, engine::general_purpose};

pub type SymetricKey = symetric_cipher::Key<mem::FullAccess>;
pub type EncryptedMessage = (String, cipher::Nonce);
pub type Nonce = cipher::Nonce;
//pub type PrivateKey = cipher::PrivateKey<mem::FullAccess>;
pub type PublicKey = cipher::PublicKey;

pub struct KeyPair(cipher::Keypair);

impl KeyPair {
    pub fn encrypt(
        &self,
        message: &[u8],
        receiver: &PublicKey,
        nonce: Option<&Nonce>,
        output: &mut [u8],
    ) -> Result<(usize, Nonce), alkali::AlkaliError> {
        self.0.encrypt(message, receiver, nonce, output)
    }
    pub fn decrypt(
        &self,
        ciphertext: &[u8],
        sender: &PublicKey,
        nonce: &Nonce,
        output: &mut [u8],
    ) -> Result<usize, alkali::AlkaliError> {
        self.0.decrypt(ciphertext, sender, nonce, output)
    }
    pub fn public_key(&self) -> PublicKey {
        self.0.public_key
    }
}

impl std::fmt::Debug for KeyPair {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "KeyPair(*****************)")
    }
}

pub fn get_asym_key_pair() -> Result<KeyPair, NetworkError> {
    Ok(KeyPair(cipher::Keypair::generate()?))
}

//pub fn get_new_keys() -> Result<(SymetricKey, KeyPair), NetworkError> {
//    Ok((
//        symetric_cipher::Key::generate()?,
//        KeyPair(cipher::Keypair::generate()?),
//    ))
//}

pub fn get_new_symetric_key() -> Result<SymetricKey, NetworkError> {
    Ok(symetric_cipher::Key::generate()?)
}

pub fn encrypt_asym(
    text: &str,
    sender_keypair: &KeyPair,
    receiver_key: PublicKey,
) -> Result<EncryptedMessage, NetworkError> {
    let mut ciphertext = vec![0u8; text.as_bytes().len() + cipher::MAC_LENGTH];
    let (_, nonce) =
        sender_keypair.encrypt(text.as_bytes(), &receiver_key, None, &mut ciphertext)?;
    Ok((general_purpose::STANDARD.encode(ciphertext), nonce))
}

pub fn decrypt_asym(
    msg: EncryptedMessage,
    receiver_keypair: &KeyPair,
    sender_key: PublicKey,
) -> Result<String, NetworkError> {
    let text = msg.0;
    let nonce = msg.1;
    let ciphertext = general_purpose::STANDARD.decode(text)?;
    let mut plaintext = vec![0u8; ciphertext.len() - cipher::MAC_LENGTH];
    receiver_keypair.decrypt(&ciphertext, &sender_key, &nonce, &mut plaintext)?;
    let string = str::from_utf8(&plaintext)?;
    Ok(String::from(string))
}

pub fn encrypt(text: &str, key: &SymetricKey) -> Result<EncryptedMessage, NetworkError> {
    let mut ciphertext = vec![0u8; text.as_bytes().len() + symetric_cipher::MAC_LENGTH];
    let (_, nonce) = symetric_cipher::encrypt(text.as_bytes(), key, None, &mut ciphertext)?;
    Ok((general_purpose::STANDARD.encode(ciphertext), nonce))
}

pub fn decrypt(msg: EncryptedMessage, key: &SymetricKey) -> Result<String, NetworkError> {
    let text = msg.0;
    let nonce = msg.1;
    let ciphertext = general_purpose::STANDARD.decode(text)?;
    let mut plaintext = vec![0u8; ciphertext.len() - cipher::MAC_LENGTH];
    symetric_cipher::decrypt(&ciphertext, key, &nonce, &mut plaintext)?;
    let string = str::from_utf8(&plaintext)?;
    Ok(String::from(string))
}

pub fn to_base64(arg: &[u8]) -> String {
    general_purpose::STANDARD.encode(arg)
}

pub fn from_base64(arg: &str) -> Result<Vec<u8>, NetworkError> {
    Ok(general_purpose::STANDARD.decode(arg)?)
}

#[allow(dead_code)]
mod dummy_crypto {
    use super::*;
    pub const KEY_LENGTH: usize = 64;

    pub type KeyType = [u8; KEY_LENGTH];

    const fn create_key() -> [u8; KEY_LENGTH] {
        let mut arr = [0u8; KEY_LENGTH];
        let mut i = 0;
        while i < KEY_LENGTH {
            arr[i] = (i * 37 % 256) as u8; // example pseudo-random formula
            i += 1;
        }
        arr
    }
    pub const DUMMY_KEY: [u8; KEY_LENGTH] = create_key();

    pub fn dummy_encrypt_text(text: &str, key: &[u8]) -> String {
        let bytes: &[u8] = text.as_bytes();
        let mut bytes: Vec<u8> = Vec::from(bytes);
        let key_len = key.len();
        // TODO: Replace with actual encryption
        for (i, b) in bytes.iter_mut().enumerate() {
            *b = *b ^ key.get(i % key_len).unwrap_or(&(0xff as u8));
        }
        general_purpose::STANDARD.encode(bytes)
    }

    pub fn dummy_decrypt_bytes(s: &str, key: &[u8]) -> Result<String, NetworkError> {
        let mut bytes = general_purpose::STANDARD.decode(s)?;
        let key_len = key.len();
        // TODO: Replace with actual decryption
        for (i, b) in bytes.iter_mut().enumerate() {
            *b = *b ^ key.get(i % key_len).unwrap_or(&(0xff as u8));
        }
        let string = str::from_utf8(&bytes)?;
        Ok(String::from(string))
    }
}
