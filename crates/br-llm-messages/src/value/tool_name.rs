use serde::{Deserialize, Serialize};

use crate::error::{Error, ErrorCode};

const MAX_LEN: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct ToolName(String);

impl ToolName {
    pub fn new(value: impl Into<String>) -> Result<Self, Error> {
        let value = value.into();
        if !is_valid(&value) {
            return Err(Error::new(ErrorCode::InvalidContent));
        }
        Ok(ToolName(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn is_valid(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() > MAX_LEN {
        return false;
    }
    let Some((first, rest)) = bytes.split_first() else {
        return false;
    };
    (first.is_ascii_alphabetic() || *first == b'_')
        && rest
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || *b == b'_' || *b == b'-')
}

impl TryFrom<String> for ToolName {
    type Error = Error;

    fn try_from(value: String) -> Result<Self, Error> {
        ToolName::new(value)
    }
}

impl AsRef<str> for ToolName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
