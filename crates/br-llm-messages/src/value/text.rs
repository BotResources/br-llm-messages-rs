use serde::{Deserialize, Serialize};

use crate::error::{Error, ErrorCode};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct PlainText(String);

impl PlainText {
    pub fn new(value: impl Into<String>) -> Result<Self, Error> {
        let value = value.into();
        if value.chars().all(char::is_whitespace) {
            return Err(Error::new(ErrorCode::InvalidContent));
        }
        Ok(PlainText(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }

    pub fn scalar_count(&self) -> u64 {
        self.0.chars().count() as u64
    }
}

impl TryFrom<String> for PlainText {
    type Error = Error;

    fn try_from(value: String) -> Result<Self, Error> {
        PlainText::new(value)
    }
}

impl AsRef<str> for PlainText {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
