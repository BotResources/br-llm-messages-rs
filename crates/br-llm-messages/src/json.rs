use std::collections::HashSet;

use serde::de::{DeserializeOwned, Error as _, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};

use crate::error::{Error, ErrorCode};

pub(crate) fn encode<T: Serialize>(value: &T) -> Result<String, Error> {
    serde_json::to_string(value).map_err(|_| Error::new(ErrorCode::InvalidContent))
}

pub(crate) fn encode_value<T: Serialize>(value: &T) -> Result<serde_json::Value, Error> {
    serde_json::to_value(value).map_err(|_| Error::new(ErrorCode::InvalidContent))
}

pub(crate) fn decode<T: DeserializeOwned>(text: &str) -> Result<T, Error> {
    serde_json::from_str::<DistinctKeys>(text).map_err(|error| Error::from_json_error(&error))?;
    serde_json::from_str(text).map_err(|error| Error::from_json_error(&error))
}

pub(crate) fn decode_value<T: DeserializeOwned>(value: serde_json::Value) -> Result<T, Error> {
    decode_ref(&value)
}

pub(crate) fn decode_ref<T: DeserializeOwned>(value: &serde_json::Value) -> Result<T, Error> {
    let text = serde_json::to_string(value).map_err(|_| Error::new(ErrorCode::InvalidContent))?;
    serde_json::from_str(&text).map_err(|error| Error::from_json_error(&error))
}

struct DistinctKeys;

impl<'de> Deserialize<'de> for DistinctKeys {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(DistinctKeysVisitor)
    }
}

struct DistinctKeysVisitor;

impl<'de> Visitor<'de> for DistinctKeysVisitor {
    type Value = DistinctKeys;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("any JSON value")
    }

    fn visit_bool<E>(self, _: bool) -> Result<DistinctKeys, E> {
        Ok(DistinctKeys)
    }

    fn visit_i64<E>(self, _: i64) -> Result<DistinctKeys, E> {
        Ok(DistinctKeys)
    }

    fn visit_u64<E>(self, _: u64) -> Result<DistinctKeys, E> {
        Ok(DistinctKeys)
    }

    fn visit_f64<E>(self, _: f64) -> Result<DistinctKeys, E> {
        Ok(DistinctKeys)
    }

    fn visit_str<E>(self, _: &str) -> Result<DistinctKeys, E> {
        Ok(DistinctKeys)
    }

    fn visit_unit<E>(self) -> Result<DistinctKeys, E> {
        Ok(DistinctKeys)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<DistinctKeys, A::Error> {
        while seq.next_element::<DistinctKeys>()?.is_some() {}
        Ok(DistinctKeys)
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<DistinctKeys, A::Error> {
        let mut seen: HashSet<String> = HashSet::new();
        while let Some(key) = map.next_key::<String>()? {
            if !seen.insert(key) {
                return Err(A::Error::custom(Error::new(ErrorCode::MalformedJson)));
            }
            map.next_value::<DistinctKeys>()?;
        }
        Ok(DistinctKeys)
    }
}
