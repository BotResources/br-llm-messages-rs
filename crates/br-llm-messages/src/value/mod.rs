macro_rules! nonempty_string_newtype {
    ($name:ident, $field:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, $crate::error::MessageError> {
                let value = value.into();
                if value.is_empty() {
                    return Err($crate::error::MessageError::Blank { field: $field });
                }
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl std::convert::TryFrom<String> for $name {
            type Error = $crate::error::MessageError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl std::convert::From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}

mod base64;
mod identifiers;
mod mime;
mod signature;
mod text;

pub use base64::Base64Data;
pub use identifiers::{Author, ModelId, ToolCallId, ToolName, TurnId};
pub use mime::ImageMime;
pub use signature::Signature;
pub use text::Text;
