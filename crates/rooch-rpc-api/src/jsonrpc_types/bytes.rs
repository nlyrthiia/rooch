// Copyright (c) RoochNetwork
// SPDX-License-Identifier: Apache-2.0

use hex::FromHexError;
use rustc_hex::{FromHex, ToHex};
use schemars::JsonSchema;
use serde::{
    de::{Error, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::fmt;
use std::str::FromStr;

/// Wrapper structure around vector of bytes.
#[derive(Debug, PartialEq, Eq, Default, Hash, Clone, JsonSchema)]
pub struct Bytes(pub Vec<u8>);

impl Bytes {
    /// Simple constructor.
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }
    /// Convert back to vector
    pub fn into_vec(self) -> Vec<u8> {
        self.0
    }
}

impl FromStr for Bytes {
    type Err = FromHexError;

    /// Convert from a hexadecimal string representation of bytes.
    fn from_str(hex_string: &str) -> Result<Self, FromHexError> {
        // Remove the "0x" prefix if it exists.
        let hex_string = if hex_string.starts_with("0x") || hex_string.starts_with("0X") {
            &hex_string[2..]
        } else {
            hex_string
        };

        // Use the `hex` crate to parse the modified hexadecimal string into bytes.
        let bytes = hex::decode(hex_string)?;

        Ok(Self(bytes))
    }
}

impl Serialize for Bytes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut serialized = "0x".to_owned();
        serialized.push_str(self.0.to_hex().as_ref());
        serializer.serialize_str(serialized.as_ref())
    }
}

impl<'a> Deserialize<'a> for Bytes {
    fn deserialize<D>(deserializer: D) -> Result<Bytes, D::Error>
    where
        D: Deserializer<'a>,
    {
        deserializer.deserialize_any(BytesVisitor)
    }
}

struct BytesVisitor;

impl<'a> Visitor<'a> for BytesVisitor {
    type Value = Bytes;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a 0x-prefixed, hex-encoded vector of bytes")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        if value.len() >= 2 && value.starts_with("0x") && value.len() & 1 == 0 {
            Ok(Bytes::new(FromHex::from_hex(&value[2..]).map_err(|e| {
                Error::custom(format!("Invalid hex: {}", e))
            })?))
        } else {
            Err(Error::custom(
                "Invalid bytes format. Expected a 0x-prefixed hex string with even length",
            ))
        }
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_str(value.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustc_hex::FromHex;
    use serde_json;

    #[test]
    fn test_bytes_serialize() {
        let bytes = Bytes("0123456789abcdef".from_hex().unwrap());
        let serialized = serde_json::to_string(&bytes).unwrap();
        assert_eq!(serialized, r#""0x0123456789abcdef""#);
    }

    #[test]
    fn test_bytes_deserialize() {
        let bytes0: Result<Bytes, serde_json::Error> = serde_json::from_str(r#""∀∂""#);
        let bytes1: Result<Bytes, serde_json::Error> = serde_json::from_str(r#""""#);
        let bytes2: Result<Bytes, serde_json::Error> = serde_json::from_str(r#""0x123""#);
        let bytes3: Result<Bytes, serde_json::Error> = serde_json::from_str(r#""0xgg""#);

        let bytes4: Bytes = serde_json::from_str(r#""0x""#).unwrap();
        let bytes5: Bytes = serde_json::from_str(r#""0x12""#).unwrap();
        let bytes6: Bytes = serde_json::from_str(r#""0x0123""#).unwrap();

        assert!(bytes0.is_err());
        assert!(bytes1.is_err());
        assert!(bytes2.is_err());
        assert!(bytes3.is_err());
        assert_eq!(bytes4, Bytes(vec![]));
        assert_eq!(bytes5, Bytes(vec![0x12]));
        assert_eq!(bytes6, Bytes(vec![0x1, 0x23]));
    }
}
