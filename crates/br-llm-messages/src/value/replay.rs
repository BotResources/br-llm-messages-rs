use serde::{Deserialize, Serialize};

use super::opaque;
use crate::error::{Error, ErrorCode};
use crate::opaque_json::OpaqueJson;

const MAX_FORMAT_LEN: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct ReplayFormat(String);

impl ReplayFormat {
    pub fn new(value: impl Into<String>) -> Result<Self, Error> {
        let value = value.into();
        if !is_valid(&value) {
            return Err(Error::new(ErrorCode::InvalidContent));
        }
        Ok(ReplayFormat(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn is_valid(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.is_empty() || bytes.len() > MAX_FORMAT_LEN || !bytes[0].is_ascii_lowercase() {
        return false;
    }
    bytes[1..]
        .iter()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(*b, b'_' | b'.' | b'-'))
}

impl TryFrom<String> for ReplayFormat {
    type Error = Error;

    fn try_from(value: String) -> Result<Self, Error> {
        ReplayFormat::new(value)
    }
}

impl AsRef<str> for ReplayFormat {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[non_exhaustive]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ReplayPayload {
    #[non_exhaustive]
    Text { value: String },
    #[non_exhaustive]
    Json {
        #[serde(deserialize_with = "super::opaque::deserialize")]
        value: OpaqueJson,
    },
}

impl std::fmt::Debug for ReplayPayload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReplayPayload::Text { .. } => f.write_str("ReplayPayload::Text(<redacted>)"),
            ReplayPayload::Json { .. } => f.write_str("ReplayPayload::Json(<redacted>)"),
        }
    }
}

impl ReplayPayload {
    pub fn text(value: impl Into<String>) -> Self {
        ReplayPayload::Text {
            value: value.into(),
        }
    }

    pub fn json(value: OpaqueJson) -> Result<Self, Error> {
        opaque::check(&value).map_err(|error| error.at("value"))?;
        Ok(ReplayPayload::Json { value })
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayMaterial {
    format: ReplayFormat,
    payload: ReplayPayload,
}

impl ReplayMaterial {
    pub fn new(format: ReplayFormat, payload: ReplayPayload) -> Self {
        ReplayMaterial { format, payload }
    }

    pub fn format(&self) -> &ReplayFormat {
        &self.format
    }

    pub fn payload(&self) -> &ReplayPayload {
        &self.payload
    }

    pub(crate) fn text_mut(&mut self) -> Option<&mut String> {
        match &mut self.payload {
            ReplayPayload::Text { value } => Some(value),
            _ => None,
        }
    }
}
