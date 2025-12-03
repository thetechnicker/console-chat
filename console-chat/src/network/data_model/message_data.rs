use crate::network::Result;
use crate::network::encryption::*;
use crate::network::error::NetworkError;
use serde::{self, Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct EncryptedKeyMessageData {
    check_str: String,
    nonce: HexVec,
    key: HexVec,
    public_key: HexVec,
    key_nonce: HexVec,
}

impl EncryptedKeyMessageData {
    pub fn new(
        check_str: &str,
        sym_key: &SymetricKey,
        key_pair: &KeyPair,
        receiver_key: PublicKey,
    ) -> Result<Self> {
        let (encrypted_check_str, nonce) = encrypt_base64(check_str, sym_key)?;
        let (key, key_nonce) = encrypt_asym(sym_key.as_slice(), key_pair, receiver_key.clone())?;
        Ok(Self {
            nonce: nonce.into(),
            key: key.into(),
            key_nonce: key_nonce.into(),
            public_key: key_pair.public_key().into(),
            check_str: encrypted_check_str,
        })
    }

    pub fn get_key(self, check: &str, key_pair: &KeyPair) -> Result<SymetricKey> {
        let public_key: PublicKey = self.public_key.into();
        let sym_key_raw = decrypt_asym(
            (self.key.deref().clone(), self.key_nonce.into()),
            key_pair,
            public_key.clone(),
        )?;
        let mut sym_key = SymetricKey::new_empty()?;
        sym_key.copy_from_slice(&sym_key_raw);
        let to_check = decrypt_base64((self.check_str, self.nonce.into()), &sym_key)?;
        if to_check == check {
            Ok(sym_key)
        } else {
            Err(NetworkError::BadKeyVerification)
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EncryptedMessageData {
    nonce: HexVec,
}

impl EncryptedMessageData {
    pub fn new(nonce: Nonce) -> Self {
        Self {
            nonce: nonce.into(),
        }
    }

    pub fn decode(self, msg: &str, key: &SymetricKey) -> Result<String> {
        Ok(decrypt_base64((msg.to_string(), self.nonce.into()), key)?.to_owned())
    }
}

#[derive(Debug, PartialEq)]
pub struct HexVec(Vec<u8>);

impl Into<Nonce> for HexVec {
    fn into(self) -> Nonce {
        let mut nonce = Nonce::default();
        nonce.copy_from_slice(self.as_slice());
        nonce
    }
}
impl Into<PublicKey> for HexVec {
    fn into(self) -> PublicKey {
        let mut nonce = PublicKey::default();
        nonce.copy_from_slice(self.as_slice());
        nonce
    }
}

impl<T> From<T> for HexVec
where
    T: Into<Vec<u8>>,
{
    fn from(value: T) -> Self {
        Self(value.into())
    }
}

// Custom serializer for HexVec
impl std::fmt::Display for HexVec {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let hex_string: String = self.0.iter().map(|b| format!("{:02x}", b)).collect();
        write!(f, "{}", hex_string)
    }
}

// Implementing custom serialization
impl Serialize for HexVec {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let hex_strings: Vec<String> = self.0.iter().map(|b| format!("{:02x}", b)).collect();
        serializer.serialize_str(&hex_strings.join(","))
    }
}
use std::ops::{Deref, DerefMut};
impl Deref for HexVec {
    type Target = Vec<u8>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for HexVec {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

// Implementing custom deserialization
impl<'de> Deserialize<'de> for HexVec {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let hex_string: String = String::deserialize(deserializer)?;
        let vec: Vec<u8> = hex_string
            .split(',')
            .filter_map(|s| u8::from_str_radix(s.trim(), 16).ok())
            .collect();
        Ok(HexVec(vec))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use color_eyre::Result;

    #[test]
    fn test() -> Result<()> {
        let pub_key = get_asym_key_pair()?;
        let asym_key = get_asym_key_pair()?;
        let sym_key = get_new_symetric_key()?;
        let msg_data =
            EncryptedKeyMessageData::new("123", &sym_key, &asym_key, pub_key.public_key())?;
        let json = serde_json::json!(msg_data);
        let msg_data2: EncryptedKeyMessageData = serde_json::from_value(json.clone())?;
        let content = format!(
            "{:#?}\n---\n{:#?}\n---\n{}",
            msg_data,
            json,
            serde_json::to_string_pretty(&msg_data)?
        );
        println!("{content}");
        assert_eq!(msg_data, msg_data, "{msg_data:#?}\n{msg_data2:#?}");
        let key_res = msg_data2.get_key("123", &pub_key)?;
        assert_eq!(sym_key, key_res);
        Ok(())
    }
}
