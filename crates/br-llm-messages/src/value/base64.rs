use serde::{Deserialize, Serialize};

use crate::error::{Error, ErrorCode};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct Base64Data(String);

impl Base64Data {
    pub fn new(value: impl Into<String>) -> Result<Self, Error> {
        let value = value.into();
        if !is_standard_padded(value.as_bytes()) {
            return Err(Error::new(ErrorCode::InvalidContent));
        }
        Ok(Base64Data(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn is_standard_padded(bytes: &[u8]) -> bool {
    if bytes.is_empty() || !bytes.len().is_multiple_of(4) {
        return false;
    }
    let padding = bytes.iter().rev().take_while(|b| **b == b'=').count();
    padding <= 2
        && bytes
            .iter()
            .rev()
            .skip(padding)
            .all(|b| b.is_ascii_alphanumeric() || *b == b'+' || *b == b'/')
}

impl TryFrom<String> for Base64Data {
    type Error = Error;

    fn try_from(value: String) -> Result<Self, Error> {
        Base64Data::new(value)
    }
}

impl AsRef<str> for Base64Data {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
