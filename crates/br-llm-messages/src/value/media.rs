use serde::{Deserialize, Serialize};

use super::base64::Base64Data;
use crate::error::{Error, ErrorCode};

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub enum MediaType {
    #[serde(rename = "image/jpeg")]
    Jpeg,
    #[serde(rename = "image/png")]
    Png,
    #[serde(rename = "image/gif")]
    Gif,
    #[serde(rename = "image/webp")]
    Webp,
}

impl MediaType {
    pub fn as_str(self) -> &'static str {
        match self {
            MediaType::Jpeg => "image/jpeg",
            MediaType::Png => "image/png",
            MediaType::Gif => "image/gif",
            MediaType::Webp => "image/webp",
        }
    }
}

impl std::str::FromStr for MediaType {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self, Error> {
        [
            MediaType::Jpeg,
            MediaType::Png,
            MediaType::Gif,
            MediaType::Webp,
        ]
        .into_iter()
        .find(|media| media.as_str() == value)
        .ok_or_else(|| Error::new(ErrorCode::UnsupportedVariant))
    }
}

impl TryFrom<String> for MediaType {
    type Error = Error;

    fn try_from(value: String) -> Result<Self, Error> {
        value.parse()
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ImageSource {
    #[non_exhaustive]
    Inline {
        media_type: MediaType,
        data: Base64Data,
    },
}

impl ImageSource {
    pub fn inline(media_type: MediaType, data: Base64Data) -> Self {
        ImageSource::Inline { media_type, data }
    }
}
