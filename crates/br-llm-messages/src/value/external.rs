use serde::{Deserialize, Serialize};

use crate::error::{Error, ErrorCode};

macro_rules! external_string {
    ($name:ident, $accepts:expr) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(try_from = "String")]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, Error> {
                let value = value.into();
                let accepts: fn(&str) -> bool = $accepts;
                if !accepts(&value) {
                    return Err(Error::new(ErrorCode::InvalidContent));
                }
                Ok($name(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl TryFrom<String> for $name {
            type Error = Error;

            fn try_from(value: String) -> Result<Self, Error> {
                $name::new(value)
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }
    };
}

external_string!(ExternalId, |value| !value.is_empty());
external_string!(ModelName, |value| !value.is_empty());
external_string!(ProviderOperation, |value| !value.is_empty());
external_string!(ItemRef, |value| !value.is_empty());
