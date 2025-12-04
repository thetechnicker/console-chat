//! This module [`encryption`] is a wrapper Module for all cryptographic code
use crate::network::error::NetworkError;
use alkali::asymmetric::cipher;
use alkali::mem;
use alkali::symmetric::cipher as symetric_cipher;
use base64::{Engine as _, engine::general_purpose};

pub type SymetricKey = symetric_cipher::Key<mem::FullAccess>;
pub type EncryptedMessageBase64 = (String, cipher::Nonce);
pub type EncryptedMessage = (Vec<u8>, cipher::Nonce);
pub type Nonce = cipher::Nonce;
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

pub fn get_new_symetric_key() -> Result<SymetricKey, NetworkError> {
    Ok(symetric_cipher::Key::generate()?)
}
pub fn encrypt_asym(
    text: &[u8],
    sender_keypair: &KeyPair,
    receiver_key: PublicKey,
) -> Result<EncryptedMessage, NetworkError> {
    let mut ciphertext = vec![0u8; text.len() + cipher::MAC_LENGTH];
    let (_, nonce) = sender_keypair.encrypt(text, &receiver_key, None, &mut ciphertext)?;
    Ok((ciphertext, nonce))
}

pub fn decrypt_asym(
    msg: EncryptedMessage,
    receiver_keypair: &KeyPair,
    sender_key: PublicKey,
) -> Result<Vec<u8>, NetworkError> {
    let ciphertext = msg.0;
    let nonce = msg.1;
    let mut plaintext = vec![0u8; ciphertext.len() - cipher::MAC_LENGTH];
    receiver_keypair.decrypt(&ciphertext, &sender_key, &nonce, &mut plaintext)?;
    Ok(plaintext)
}

#[allow(dead_code)]
pub fn encrypt_asym_base64(
    text: &str,
    sender_keypair: &KeyPair,
    receiver_key: PublicKey,
) -> Result<EncryptedMessageBase64, NetworkError> {
    let (ciphertext, nonce) = encrypt_asym(text.as_bytes(), sender_keypair, receiver_key)?;
    Ok((to_base64(ciphertext.as_slice()), nonce))
}

#[allow(dead_code)]
pub fn decrypt_asym_base64(
    msg: EncryptedMessageBase64,
    receiver_keypair: &KeyPair,
    sender_key: PublicKey,
) -> Result<String, NetworkError> {
    let text = msg.0;
    let nonce = msg.1;
    let ciphertext = from_base64(&text)?;
    let plaintext = decrypt_asym((ciphertext, nonce), receiver_keypair, sender_key)?;
    let string = str::from_utf8(&plaintext)?;
    Ok(String::from(string))
}

pub fn encrypt_base64(
    text: &str,
    key: &SymetricKey,
) -> Result<EncryptedMessageBase64, NetworkError> {
    let mut ciphertext = vec![0u8; text.len() + symetric_cipher::MAC_LENGTH];
    let (_, nonce) = symetric_cipher::encrypt(text.as_bytes(), key, None, &mut ciphertext)?;
    Ok((to_base64(ciphertext.as_slice()), nonce))
}

pub fn decrypt_base64(
    msg: EncryptedMessageBase64,
    key: &SymetricKey,
) -> Result<String, NetworkError> {
    let text = msg.0;
    let nonce = msg.1;
    let ciphertext = from_base64(&text)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asymmetric_key_pair_generation() {
        let keypair_result = get_asym_key_pair();
        assert!(keypair_result.is_ok());

        let keypair = keypair_result.unwrap();
        assert!(keypair.public_key().len() > 0); // Check that the public key is generated
    }

    #[test]
    fn test_symmetric_key_generation() {
        let symmetric_key_result = get_new_symetric_key();
        assert!(symmetric_key_result.is_ok());

        let symmetric_key = symmetric_key_result.unwrap();
        assert!(symmetric_key.len() > 0); // Check that the symmetric key is generated
    }

    #[test]
    fn test_asymmetric_encryption_decryption() {
        let sender_keypair = get_asym_key_pair().unwrap();
        let receiver_keypair = get_asym_key_pair().unwrap();

        let plaintext = b"Secret Message";
        let encrypted_message =
            encrypt_asym(plaintext, &sender_keypair, receiver_keypair.public_key()).unwrap();

        let decrypted_message = decrypt_asym(
            encrypted_message,
            &receiver_keypair,
            sender_keypair.public_key(),
        )
        .unwrap();

        assert_eq!(plaintext.to_vec(), decrypted_message);
    }

    #[test]
    fn test_asymmetric_encryption_decryption_error_handling() {
        let sender_keypair = get_asym_key_pair().unwrap();
        let receiver_keypair = KeyPair(cipher::Keypair::generate().unwrap()); // Create a different keypair
        let invalid_receiver_keypair = KeyPair(cipher::Keypair::generate().unwrap()); // Create a different keypair

        let plaintext = b"Secret Message";
        let encrypted_message =
            encrypt_asym(plaintext, &sender_keypair, receiver_keypair.public_key()).unwrap();

        // Attempt decryption with the wrong keypair
        let decryption_result = decrypt_asym(
            encrypted_message,
            &invalid_receiver_keypair,
            sender_keypair.public_key(),
        );
        assert!(decryption_result.is_err());
    }

    #[test]
    fn test_symmetric_encryption_decryption() {
        let symmetric_key = get_new_symetric_key().unwrap();
        let message = "Hello, Symmetric Encryption!";

        let encrypted_message = encrypt_base64(message, &symmetric_key).unwrap();
        let decrypted_message = decrypt_base64(encrypted_message, &symmetric_key).unwrap();

        assert_eq!(message, decrypted_message);
    }

    #[test]
    fn test_base64_conversion() {
        let original_data = b"Test Data";
        let base64_encoded = to_base64(original_data);
        let decoded_data = from_base64(&base64_encoded).unwrap();

        assert_eq!(original_data.to_vec(), decoded_data);
    }

    #[test]
    fn test_base64_decoding_error_handling() {
        let bad_base64 = "Invalid Base64 String!";
        let decoding_result = from_base64(bad_base64);
        assert!(decoding_result.is_err());
    }
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
            *b ^= key.get(i % key_len).unwrap_or(&0xff_u8);
        }
        general_purpose::STANDARD.encode(bytes)
    }

    pub fn dummy_decrypt_bytes(s: &str, key: &[u8]) -> Result<String, NetworkError> {
        let mut bytes = general_purpose::STANDARD.decode(s)?;
        let key_len = key.len();
        // TODO: Replace with actual decryption
        for (i, b) in bytes.iter_mut().enumerate() {
            *b ^= key.get(i % key_len).unwrap_or(&0xff_u8);
        }
        let string = str::from_utf8(&bytes)?;
        Ok(String::from(string))
    }
}
