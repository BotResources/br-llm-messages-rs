use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

use crate::error::Error;

macro_rules! typed_id {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $name(Uuid);

        impl $name {
            pub fn from_uuid(uuid: Uuid) -> Result<Self, Error> {
                super::check(uuid).map(Self)
            }

            pub fn parse(text: &str) -> Result<Self, Error> {
                super::parse(text).map(Self)
            }

            pub fn as_uuid(&self) -> Uuid {
                self.0
            }
        }

        impl AsRef<Uuid> for $name {
            fn as_ref(&self) -> &Uuid {
                &self.0
            }
        }

        impl TryFrom<Uuid> for $name {
            type Error = Error;

            fn try_from(uuid: Uuid) -> Result<Self, Error> {
                Self::from_uuid(uuid)
            }
        }

        impl FromStr for $name {
            type Err = Error;

            fn from_str(text: &str) -> Result<Self, Error> {
                Self::parse(text)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.0.hyphenated(), f)
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.collect_str(&self.0.hyphenated())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let text = String::deserialize(deserializer)?;
                Self::parse(&text).map_err(serde::de::Error::custom)
            }
        }
    };
}

macro_rules! mintable_id {
    ($name:ident) => {
        #[cfg(feature = "mint")]
        impl $name {
            pub fn mint() -> Self {
                Self(Uuid::now_v7())
            }
        }
    };
}

typed_id!(ConversationId);
typed_id!(TurnId);
typed_id!(EntryId);
typed_id!(BlockId);
typed_id!(CitationId);
mintable_id!(ConversationId);
mintable_id!(TurnId);
mintable_id!(EntryId);
mintable_id!(BlockId);
mintable_id!(CitationId);
typed_id!(HumanId);
typed_id!(AgentId);
typed_id!(RequestId);
typed_id!(TaskId);
