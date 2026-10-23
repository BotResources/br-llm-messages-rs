use serde::{Deserialize, Serialize};

use crate::error::{Error, ErrorCode};

const MAX_SCALAR_VALUES: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct Author(String);

impl Author {
    pub fn new(value: impl Into<String>) -> Result<Self, Error> {
        let value = value.into();
        let has_visible = value.chars().any(|c| !c.is_whitespace());
        if !has_visible || value.chars().count() > MAX_SCALAR_VALUES {
            return Err(Error::new(ErrorCode::InvalidAuthor));
        }
        Ok(Author(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Author {
    type Error = Error;

    fn try_from(value: String) -> Result<Self, Error> {
        Author::new(value)
    }
}

impl AsRef<str> for Author {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
